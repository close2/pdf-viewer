//! Nested content: a `Do` operator, a form, an image, an inline image, and the one place a nested stream is opened.

use crate::finding::Where;
use pdf_model::content::ledger::Route as Via;
use pdf_model::content::reader::ContentReader;
use pdf_model::content::reader::LOOKAHEAD;
use pdf_model::content::reader::NestedContent;
use pdf_model::content::reader::WINDOW;
use pdf_syntax::Dictionary;
use pdf_syntax::Object;
use pdf_syntax::ObjectId;
use pdf_syntax::Stream;

use super::{
    Context, MAX_FORM_DEPTH, MAX_OBSERVATIONS, MAX_STREAMS, Route, State, Walk,
    jpeg2000_device_family,
};

impl Walk<'_> {
    /// Follows `Do` into a form or image `XObject`.
    pub(super) fn invoke(&mut self, name: &[u8], state: &State, context: &Context<'_>) {
        let Some(entry) = self.look_up("XObject", name, context, context.what) else {
            return;
        };
        let id = entry.as_reference();
        let Object::Stream(stream) = self.document.resolve(&entry) else {
            return;
        };
        let subtype = self.document.get_key(&stream.dict, "Subtype");
        let subtype = subtype.as_name().map(|name| name.as_bytes().to_vec());
        match subtype.as_deref() {
            Some(b"Form") => {
                // The page's own table is the one searched when the invoking stream is the page's
                // content or states no `/Resources` (§7.8.3), which is when the walk's resources
                // are the page's.
                let page_table = context.record.is_some_and(|record| {
                    self.survey.streams.get(record).is_some_and(|invoking| {
                        invoking.what == "the page content" || !invoking.own_resources
                    })
                });
                self.form(
                    &stream,
                    id,
                    context.page,
                    context.depth,
                    state,
                    Via::Form,
                    page_table,
                );
            }
            Some(b"Image") => self.image(&stream, id, context),
            _ => {}
        }
    }

    /// One form `XObject`: its transparency group, and then its content.
    ///
    /// ISO 32000-2 §11.6.6 makes an isolated group's own `CS` the blending colour space for
    /// what it contains; a non-isolated group, or one that states no `CS`, inherits the space of
    /// the page or group it is painted into.
    ///
    /// A form stating no `/Resources` is read against the page's dictionary — §7.8.3's NOTE 3
    /// since Errata Collection 3 names "the resource dictionary of the page on which they are
    /// used", and Table 93's `/Resources` cell says the same of a PDF 1.1 file in its own words
    /// — and not against the stream that invoked it, whatever route that was. The two are one
    /// dictionary until a form is nested inside a form with `/Resources` of its own, and there
    /// `pdf_model`'s interpreter reads the page's too (ADR 1059).
    #[expect(
        clippy::too_many_arguments,
        reason = "the walk's whole context travels together, as it does into `nested`, and naming \
                  a struct for the four call sites would hide which argument a caller changed"
    )]
    pub(super) fn form(
        &mut self,
        stream: &Stream,
        id: Option<ObjectId>,
        page: usize,
        depth: u32,
        state: &State,
        route: Via,
        through_page_table: bool,
    ) {
        let mut state = state.clone();
        let own = self.document.get_key(&stream.dict, "Resources");
        let stated_resources = own.as_dict().is_some();
        let resources = own
            .as_dict()
            .cloned()
            .unwrap_or_else(|| (*self.page_resources).clone());
        let group = self.document.get_key(&stream.dict, "Group");
        if let Some(group) = group.as_dict()
            && self.is_transparency_group(group)
        {
            self.survey.transparent.insert(page);
            let place = id.map_or_else(|| Where::page(page), Where::object);
            let stated = self.record_group_space(group, &resources, page, place, stated_resources);
            if self.document.get_key(group, "I") == Object::Boolean(true) && stated.kind.is_some() {
                state.blending = stated;
            }
        }
        self.nested(
            stream,
            id,
            &resources,
            page,
            depth,
            &state,
            "a form XObject",
            stated_resources,
            route,
            through_page_table,
        );
    }

    /// Opens one nested content stream and runs it against the resources given.
    #[expect(
        clippy::too_many_arguments,
        reason = "the walk's whole context travels together, and naming a struct for one call \
                  site would hide which of the seven a caller changed"
    )]
    pub(super) fn nested(
        &mut self,
        stream: &Stream,
        id: Option<ObjectId>,
        resources: &Dictionary,
        page: usize,
        depth: u32,
        state: &State,
        description: &'static str,
        own_resources: bool,
        route: Via,
        through_page_table: bool,
    ) {
        if depth >= MAX_FORM_DEPTH || self.opened >= MAX_STREAMS {
            self.survey.stopped_short = true;
            return;
        }
        // A stream reached twice on one page draws the same marks twice, and following it once
        // is what stops a `/Resources` cycle from becoming an unbounded walk.
        if let Some(id) = id {
            if let Some(earlier) = self.visited.get(&(page, id)) {
                // The second route to the stream is not followed again, but it is remembered:
                // a stream also reached some other way has another name for it to be changed at.
                if !through_page_table
                    && let Some(record) = earlier.and_then(|at| self.survey.streams.get_mut(at))
                {
                    record.through_page_table = false;
                }
                return;
            }
            self.visited.insert((page, id), None);
        }
        let Ok(content) = NestedContent::of(self.document, stream, description.to_owned()) else {
            return;
        };
        self.opened = self.opened.saturating_add(1);
        let place = id.map_or_else(|| Where::page(page), Where::object);
        let origin = self.open_record(
            page,
            id,
            place,
            description,
            own_resources,
            through_page_table,
        );
        if let Some(id) = id {
            self.visited.insert((page, id), origin.record);
        }
        let mut reader = content.reader();
        if let Some(ledger) = &mut self.ledger {
            // The interpreter's frame for a soft mask's group names no object — it holds the
            // group resolved rather than the reference — and the ledger is compared frame for
            // frame, so the survey's says the same (`doc/adr/2263`).
            let frame = if route == Via::SoftMask { None } else { id };
            ledger.enter(route, frame);
        }
        self.run(
            &mut reader,
            resources,
            page,
            depth.saturating_add(1),
            state,
            origin,
        );
        if let Some(ledger) = &mut self.ledger {
            ledger.leave();
        }
    }

    /// An image `XObject`: its colour space, and Annex Q.4's two transparency conditions.
    fn image(&mut self, stream: &Stream, id: Option<ObjectId>, context: &Context<'_>) {
        let place = id.map_or_else(|| Where::page(context.page), Where::object);
        if matches!(
            self.document.get_key(&stream.dict, "SMask"),
            Object::Stream(_)
        ) || self
            .document
            .get_key(&stream.dict, "SMaskInData")
            .as_integer()
            .is_some_and(|value| value > 0)
        {
            self.survey.transparent.insert(context.page);
        }
        self.image_space(&stream.dict, &place, context, "an image XObject");
        if stream.dict.get("ColorSpace").is_none()
            && let Some(family) = jpeg2000_device_family(self.document, stream)
        {
            self.push_colour(
                (family, Route::Direct),
                place,
                context,
                "a JPEG 2000 image's own data",
                None,
            );
        }
    }

    /// One image dictionary's `/ColorSpace`, recorded where it names a device space.
    ///
    /// §8.6.5.6's remapping reaches an image's colour space through the resource dictionary the
    /// image was *drawn* from, which is the one in force here rather than any the image carries.
    fn image_space(
        &mut self,
        dict: &Dictionary,
        place: &Where,
        context: &Context<'_>,
        what: &'static str,
    ) {
        let Some(stated) = dict.get("ColorSpace") else {
            return;
        };
        let stated = self.document.resolve(stated);
        self.record_space(&stated, place, context, what, None, None);
    }

    /// ISO 32000-2 §8.9.7's inline image: its colour space, and then the bytes stepped over.
    pub(super) fn inline_image(&mut self, reader: &mut ContentReader<'_>, context: &Context<'_>) {
        let bound = LOOKAHEAD.min(self.document.limits().max_stream_len);
        let mut want = WINDOW;
        loop {
            let settled = {
                let (ahead, complete) = reader.lookahead(want);
                let scanned = pdf_model::inline_image::scan(
                    self.document,
                    ahead,
                    0,
                    context.resources,
                    complete,
                );
                if complete || scanned.image.is_ok() || want >= bound {
                    Some((scanned.resume, scanned.image.ok()))
                } else {
                    None
                }
            };
            if let Some((resume, image)) = settled {
                reader.skip(resume);
                if let Some(image) = image {
                    let place = Where::page(context.page);
                    self.image_space(&image.dict, &place, context, "an inline image");
                    if self.survey.inline_images.len() < MAX_OBSERVATIONS {
                        self.survey
                            .inline_images
                            .push((context.page, image.dict.clone()));
                    }
                }
                return;
            }
            want = want.saturating_mul(2).min(bound);
        }
    }
}

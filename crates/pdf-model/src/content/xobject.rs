//! `Do`: form `XObject`s (§8.10) and the entries their dictionaries share.
//!
//! An image `XObject` is dispatched from here and drawn by [`super::image`]; a form with a
//! `/Group` is handed to [`super::transparency`].

use pdf_render::Transform;
use pdf_syntax::{Dictionary, Object};

use super::report::{ContentStream, Unsupported};
use super::run::{name_at, narrow};
use super::{GraphicsState, Interpreter};

impl Interpreter<'_> {
    /// Draws an `XObject`: a form is interpreted inline, an image is reported.
    ///
    /// §8.8.1's Table 86 states two requirements on what `Do`'s operand finds, and a file that
    /// breaks either draws nothing where the producer asked for a mark:
    ///
    /// > Paint the specified XObject . The operand name shall appear as a key in the XObject
    /// > subdictionary of the current resource dictionary (see 7.8.3, "Resource
    /// > dictionaries"). The associated value shall be a stream whose Type entry, if present,
    /// > is XObject .
    ///
    /// **Both were silent until the four-hundred-and-nineteenth session**, which is trap 5's
    /// shape: a missing *font* has said "no /Font resource named /F1" since the interpreter
    /// had fonts, and `sh` has said "/Sh0 is not in /Shading", so a page that names an
    /// undefined `XObject` was the one resource category out of the three whose absence
    /// looked exactly like a page the producer meant to leave sparse. §7.8.3 makes the file
    /// wrong rather than this reader — "[a] content stream's named resources shall be defined
    /// by a resource dictionary, which shall enumerate the named resources needed by the
    /// operators in the content stream" — and this tree's rule for a malformed file is to draw
    /// what can be drawn and say what could not.
    pub(super) fn draw_xobject(
        &mut self,
        operands: &[Object],
        resources: &Dictionary,
        state: &GraphicsState,
    ) {
        let Some(name) = name_at(operands, 0) else {
            // Table 86's operand column is `name`, so a `Do` with anything else — or with
            // nothing — is not a `Do` at all. Reported by operator rather than by resource,
            // because there is no name to say was undefined.
            self.note(Unsupported::Operator {
                operator: "Do with no name operand".to_owned(),
            });
            return;
        };
        // Looked up unresolved and resolved here, rather than through [`Interpreter::resource`],
        // because §14.7.5.2's identifier is only unique within one content stream and Table 357's
        // `/Stm` names that stream by *reference*: resolving first throws away the one thing that
        // tells a form's `/MCID 0` from the page's. The lookup is still one.
        let Some(entry) = self.resource_entry(resources, "XObject", &name) else {
            self.note_missing_resource("XObject", &name, "is not in /XObject");
            return;
        };
        let named = entry.as_reference();
        let object = self.document.resolve(&entry);
        let Some(stream) = object.as_stream().cloned() else {
            self.note_missing_resource("XObject", &name, "is not a stream");
            return;
        };

        // §8.11.3.3: a form or image XObject may carry an `/OC` entry naming a group or a
        // membership dictionary, and its visibility is that of the group "along with the
        // current visibility state in the context in which the XObject is invoked" — which
        // is what `is_hidden` already carries. §8.11.3.1 permits skipping such an object
        // entirely, because a form's state changes do not outlive it, and skipping is what
        // keeps an undrawable image inside a hidden layer from being reported as a gap.
        // Read unresolved: a group is identified by *which object* it is (§8.11.2.2).
        let group = stream.dict.get("OC").cloned();
        if let Some(oc) = &group
            && !self.shows_optional_content(oc)
        {
            // §8.9.5.4 step a), as Errata Collection 3 amends it: "[i]f the base image contains
            // an OC entry that specifies that the content is not visible, then nothing shall be
            // shown." Terminal — the `/Alternates` are **not** examined, because steps c) and d)
            // begin at "Otherwise" and are reached only by an image that states no `/OC` at all.
            // Step b) is the fall-through below, since drawing the base image is what it asks
            // for. `Interpreter::alternate_image` carries the whole amended algorithm.
            return;
        }
        if self.is_hidden() {
            return;
        }

        // What a *report* and an image's diagnostic label want is text; §7.3.5 binds the lookups
        // above, which take the name's own bytes.
        let label = String::from_utf8_lossy(name.as_bytes()).into_owned();
        let subtype = self.document.get_key(&stream.dict, "Subtype");
        let subtype = subtype
            .as_name()
            .map(|name| name.as_bytes().to_vec())
            .unwrap_or_default();

        if subtype == b"Image" {
            // §8.9.5.4 steps d) and e): a base image stating no `/OC` may be replaced by the
            // first of its `/Alternates` whose own `/OC` says it is visible, and where none is
            // identified "the base image shall be rendered".
            let alternate = match group {
                Some(_) => None,
                None => self.alternate_image(&stream.dict, &label),
            };
            let drawn = alternate.as_ref().unwrap_or(&stream);
            // An `XObject` is an object the resource dictionary hands out, so the same `Do`
            // twice is the same allocation twice and its address is the name of the image.
            self.draw_image(
                crate::image::NamedStream::allocation(drawn),
                &label,
                resources,
                state,
            );
            return;
        }
        if subtype != b"Form" {
            self.note(Unsupported::Operator {
                operator: format!("Do on /{label}"),
            });
            return;
        }

        // §8.10.4: "[t]he presence of the Ref entry shall distinguish reference XObjects from
        // other types of form XObjects", and what such a form draws depends on whether the target
        // document is here. §8.10.4.1 writes a `shall` to each of the two cases, and this reader
        // is in one or the other according to what a host supplied — never according to the file.
        // `imported_page` is where the supply is asked, once, and the proxy below is the answer
        // for every reference whose page it cannot produce (`crate::reference`, ADR 1101).
        if let Some((supplied, index)) = self.imported_page(&stream.dict, &label) {
            self.draw_imported_page(&stream.dict, &label, state, supplied, index);
            return;
        }

        // §8.10.1 makes a form "a self-contained description of any sequence of graphics
        // objects", which is §7.8.2's content stream under another name — so a form that
        // decoded only part-way is drawn to where its damage is and reported for the rest.
        // See [`Interpreter::content_stream`].
        let Some(data) = self.content_stream(&stream, &format!("a form XObject /{label} (§8.10)"))
        else {
            self.note(Unsupported::Operator {
                operator: format!("undecodable form /{label}"),
            });
            return;
        };

        // A form carries its own matrix and its own resources, or inherits the page's.
        let mut inner = state.clone();
        if let Some(matrix) = self.matrix(&stream.dict) {
            inner.transform = matrix.then(inner.transform);
        }

        // §8.10.1 lists what `Do` performs on a form XObject, and step c) is "Clips
        // according to the form dictionary's BBox entry"; Table 93 says of `/BBox` that
        //
        // > These boundaries shall be used to clip the form XObject and to determine its
        // > size for caching.
        //
        // Required of every form, not only of an annotation's appearance — which is the one
        // place this tree had it. §11.6.6 needs it too: a group's shape is the union of its
        // elements "clipped by the group XObject's bounding box".
        //
        // A form with no `/BBox` is malformed, since Table 93 makes it required. It is drawn
        // unclipped rather than refused: there is no box to honour, and the alternative
        // reading — clip to nothing — would delete content the producer plainly meant to
        // draw.
        if let Some(bbox) = self.rectangle(&stream.dict, "BBox") {
            let Some(clip) = self.rect_clip(bbox, inner.transform, inner.clip) else {
                self.note(Unsupported::LimitReached { limit: "max_clips" });
                return;
            };
            inner.clip = Some(clip);
        }

        // A form that omits `/Resources` is looked up in the **page's** dictionary, and every
        // sentence on the subject names that dictionary and no other. §7.8.3's NOTE 3, which
        // Errata Collection 3 Issue #128 put where the struck fourth bullet was, and which
        // `annotations.rs` quotes in full one construct over: resources referenced from such a
        // stream "can be inherited from the resource dictionary of the page on which they are
        // used". Table 93's `/Resources` cell says the same as a `shall` for the files that
        // omit the entry — "In a PDF whose version is 1.1 and earlier, all named resources used
        // in the form XObject shall be included in the resource dictionary of each page object
        // on which the form XObject appears" — so the *direction* of the fallback is stated
        // twice even though the fallback itself is now a choice about pre-2.0 and malformed
        // files rather than a `shall` (ADR 0255).
        //
        // The stream that *invoked* the form is the page's dictionary only until a form is
        // nested inside a form with `/Resources` of its own, and that is where the two readings
        // part: the inner form's names are the page's to define, and a name only the outer form
        // defines is one nothing defines. ADR 1059, and `tests/missing_resources.rs` holds the
        // fixture the corpus cannot (trap 8).
        //
        // **The fallback is on the entry's absence and not on a name's**: a form that states a
        // `/Resources` has stated which names it uses, so a name that dictionary omits is
        // reported by `draw_xobject` above rather than looked up a second time here. That is
        // the same choice `font` makes for `Tf`, which matters because the alternative is what
        // session 127 had to undo: a page's `/Fm0` and a form's `/Fm0` are two objects as often
        // as they are one, and reaching past the dictionary that names them is how a reader
        // draws the wrong one and says nothing (ADR 0255).
        let form_resources = self
            .document
            .get_key(&stream.dict, "Resources")
            .as_dict()
            .cloned()
            .unwrap_or_else(|| self.page_resources.as_ref().clone());

        // §8.7.2: a pattern's matrix maps pattern space to "the default coordinate system of
        // the pattern's parent content stream", and the clause says what that means here:
        //
        // > Similarly, if a pattern is used within a form XObject (see 8.10, "Form XObjects"
        // > ), the pattern matrix maps pattern space to the form's default user space (that
        // > is, the form coordinate space at the time the form is painted with the Do
        // > operator).
        //
        // Which is `inner.transform`: §8.10.1's step b) concatenates the form's `/Matrix`
        // with the CTM before its content stream runs, so the form's default user space is
        // the space that content starts in. Restored afterwards, because the *page's*
        // default space is what a pattern used on the page maps to and the two are different
        // spaces with the same name.
        let outer_base = std::mem::replace(&mut self.base, inner.transform);
        // §14.7.5.2 permits this stream to carry marked-content sequences of its own, numbered
        // from zero like the page's, and §14.7.5.4 gives it its own parent tree entry to match:
        // "[t]he tree shall contain an entry … for each content stream containing at least one
        // marked-content sequence that is a content item". So for as long as it runs, a sequence
        // that closes belongs to *this* stream. A form written as a direct object cannot be named
        // by Table 357's `/Stm`, which requires an indirect reference, and says so.
        let outer_stream = std::mem::replace(
            &mut self.stream,
            named.map_or(ContentStream::Unnameable, ContentStream::Object),
        );
        // §14.7.5.4's own route back, per stream: this form's `/StructParents` names an array of
        // its own, and reading the page's with a form's identifier is how §14.9's `/Alt` arrives
        // from somebody else's element. A form stating no `/StructParents` keeps the page's tree,
        // which is what a file using §14.7.5.2's *first* method — a `Do` inside the page's own
        // sequence — has said: the form is part of that sequence rather than a container of its
        // own.
        let outer_structure = self.enter_stream_structure(named, &stream.dict);
        self.enter_ledger_frame(super::ledger::Route::Form, named);
        match self.transparency_group(&stream.dict) {
            None => self.run(&data, &form_resources, &inner),
            Some(group) => {
                self.run_transparency_group(&group, &data, &form_resources, &inner, state);
            }
        }
        self.leave_ledger_frame();
        self.leave_stream_structure(outer_structure);
        self.stream = outer_stream;
        self.base = outer_base;
    }

    /// Reads a form dictionary's `/Matrix` (§8.10.2 Table 93).
    ///
    /// > An array of six numbers specifying the form matrix , which maps form space into
    /// > user space (see 8.3.4, "Transformation matrices").
    ///
    /// Shared by the two places that need it, because §11.6.5.1 defines a soft mask's
    /// coordinate system as this matrix concatenated with the transform in force at the
    /// `gs` — the same reading of the same entry that `Do` makes, and one worth making
    /// once.
    pub(super) fn matrix(&mut self, dict: &Dictionary) -> Option<Transform> {
        let entry = self.document.get_key(dict, "Matrix");
        let items = entry.as_array()?;
        let values: Vec<f32> = items
            .iter()
            .map(|item| self.document.resolve(item))
            .filter_map(|item| item.as_number())
            .map(narrow)
            .collect();
        (values.len() >= 6).then(|| {
            Transform::new(
                values[0], values[1], values[2], values[3], values[4], values[5],
            )
        })
    }

    /// Reads a rectangle entry as four numbers, in the order the file wrote them.
    pub(super) fn rectangle(&mut self, dict: &Dictionary, key: &str) -> Option<[f32; 4]> {
        let entry = self.document.get_key(dict, key);
        let items = entry.as_array()?;
        let values: Vec<f32> = items
            .iter()
            .map(|item| self.document.resolve(item))
            .filter_map(|item| item.as_number())
            .map(narrow)
            .collect();
        (values.len() >= 4).then(|| [values[0], values[1], values[2], values[3]])
    }
}

/// What [`Interpreter::enter_imported`] takes out of the interpreter for the span of ISO 32000-2
/// §8.10.4's imported page, and [`Interpreter::leave_imported`] puts back.
///
/// **Every field of it is keyed by, or derived from, one document**, and that is the whole reason
/// this type exists rather than three `mem::replace`s at the call site: two PDF files hand out the
/// same object numbers, so a memo carried across the swap answers a question about the target
/// document with a fact about the containing one. The failure that would produce is a page drawn
/// with another file's fonts, colours and masks, in silence — the shape `doc/traps/` records under
/// trap 1 and ADR 0115.
///
/// The destructure in [`Interpreter::leave_imported`] is exhaustive on purpose, for the reason
/// `Interpreter::checkpoint` gives: a field added here stops the crate compiling until somebody
/// has decided what an imported page does with it.
struct ImportedFrame<'a> {
    document: &'a pdf_syntax::Document,
    across: Option<&'a super::FontCache>,
    fonts: std::collections::BTreeMap<super::FontKey, Option<super::Font>>,
    icc_spaces: std::collections::BTreeMap<pdf_syntax::ObjectId, super::ColourSpace>,
    shadings: crate::shading::Cache,
    resource_tables:
        std::cell::RefCell<std::collections::BTreeMap<pdf_syntax::ObjectId, Dictionary>>,
    image_masks: crate::image::MaskCache,
    image_rasters: crate::image::RasterCache,
    structure: std::sync::Arc<crate::structure::ParentTree>,
    stream_structures: std::collections::BTreeMap<
        pdf_syntax::ObjectId,
        std::sync::Arc<crate::structure::ParentTree>,
    >,
    output_intent: Option<super::ColourSpace>,
    optional_content: Option<crate::optional_content::OptionalContent>,
    page_resources: std::sync::Arc<Dictionary>,
    view: std::borrow::Cow<'a, crate::view::ViewState>,
    delegated: std::collections::BTreeSet<pdf_syntax::ObjectId>,
    page: pdf_render::Size,
    ledger: Option<&'a std::cell::RefCell<super::Ledger>>,
    stream: ContentStream,
    annotations_clipped_to: Option<pdf_render::ClipId>,
}

impl<'a> Interpreter<'a> {
    /// Which supplied document's page this reference `XObject` names, saying why where none does.
    ///
    /// ISO 32000-2 §8.10.4.1 addresses a `shall` to each of two classes of processor, and which
    /// one this is depends on what a host supplied rather than on the code:
    ///
    /// > Those PDF processors that do implement reference XObjects shall use the proxy in place
    /// > of the imported content if the latter is unavailable.
    ///
    /// So `None` means the proxy is drawn, which is that sentence, and the four ways of arriving
    /// at it are not one thing: three of them are reported and the fourth — no host supplied
    /// anything at all — is not, because it is the state every host is in by default and the
    /// clause states it as an alternative rather than as a gap (trap 11).
    fn imported_page(
        &mut self,
        form: &Dictionary,
        label: &str,
    ) -> Option<(&'a crate::reference::Supplied, usize)> {
        use crate::reference::Outcome;

        let reference = crate::reference::Reference::read(self.document, form)?;
        // Copied out of `self` before the call so that the answer borrows the supply rather than
        // the interpreter: a report below needs `&mut self` while the outcome is still in hand.
        let supply: &'a crate::reference::Supply = self.references;
        let named = reference
            .file
            .display_name()
            .unwrap_or_else(|| "an unnamed file".to_owned());
        match supply.find(&reference) {
            Outcome::NoneSupplied => None,
            Outcome::NotIdentified => {
                self.note(Unsupported::ReferenceXObject {
                    detail: format!(
                        "/{label} names {named} and states no /ID, so none of the {} supplied \
                         target document(s) can be matched to it",
                        supply.len()
                    ),
                });
                None
            }
            Outcome::Unavailable => {
                self.note(Unsupported::ReferenceXObject {
                    detail: format!(
                        "/{label} names {named}, and none of the {} supplied target document(s) \
                         states its /ID",
                        supply.len()
                    ),
                });
                None
            }
            Outcome::NoSuchPage { supplied, .. } => {
                // Table 95 says this happens without anybody doing anything wrong: the reference
                // "is a weak one and may be inadvertently invalidated if the referenced page is
                // changed or replaced in the target document after the reference is created".
                self.note(Unsupported::ReferenceXObject {
                    detail: format!(
                        "/{label} names a page of {} that the file does not have",
                        supplied.name()
                    ),
                });
                None
            }
            Outcome::Found {
                supplied,
                index,
                changed,
            } => {
                if changed {
                    // §8.10.4.1 Table 95 says what the entry is for — it "allows it to warn the
                    // user if the PDF file has changed since the reference was created" — and
                    // §14.4 says what this case is: "If only the first identifier matches, a
                    // different version of the correct PDF file has been found." Drawn, and said.
                    self.note(Unsupported::ReferenceXObject {
                        detail: format!(
                            "/{label} imports page {} of {}, whose /ID says it is a different \
                             version of the file the reference was made against",
                            index.saturating_add(1),
                            supplied.name()
                        ),
                    });
                }
                Some((supplied, index))
            }
        }
    }

    /// Draws one page of a supplied target document in the proxy's place (§8.10.4.1).
    ///
    /// > When the imported content replaces the proxy, it shall be transformed according to the
    /// > proxy object's transformation matrix and clipped to the boundaries of its bounding box,
    /// > as specified by the Matrix and BBox entries in the proxy's form dictionary
    ///
    /// Which is §8.10.1's steps b) and c) for an ordinary form, read off the same two entries, so
    /// the three lines below are `draw_xobject`'s own and the clause is why they are repeated
    /// rather than shared: what changes is the *content*, not the placement.
    ///
    /// §8.10.4.1's last sentence is honoured by construction — "[i]f the proxy object's form
    /// dictionary contains a Group entry, the specified group attributes shall apply to the
    /// imported page as well" — because the group is read off the proxy here exactly as
    /// `draw_xobject` reads it, and the imported page's content is what runs inside it.
    fn draw_imported_page(
        &mut self,
        proxy: &Dictionary,
        label: &str,
        state: &GraphicsState,
        supplied: &'a crate::reference::Supplied,
        index: usize,
    ) {
        let Some(page) = crate::page::Pages::new(supplied.document()).get(index) else {
            // `Supply::find` established the index against the same page tree, so this is the
            // tree answering differently twice rather than a reference being wrong. Reported
            // rather than dropped, because a silence here is a page missing for no stated reason.
            self.note(Unsupported::ReferenceXObject {
                detail: format!(
                    "/{label}: page {} of {} was found by /Ref and could not be read",
                    index.saturating_add(1),
                    supplied.name()
                ),
            });
            return;
        };

        let mut inner = state.clone();
        if let Some(matrix) = self.matrix(proxy) {
            inner.transform = matrix.then(inner.transform);
        }
        if let Some(bbox) = self.rectangle(proxy, "BBox") {
            let Some(clip) = self.rect_clip(bbox, inner.transform, inner.clip) else {
                self.note(Unsupported::LimitReached { limit: "max_clips" });
                return;
            };
            inner.clip = Some(clip);
        }

        // §7.8.2's nesting bound, asked here for the reason `Interpreter::run` asks it there: an
        // imported page may itself hold a reference `XObject`, and a supply holding two documents
        // that name each other is a cycle no entry in either file is wrong about. One counter for
        // every kind of nested stream, because what it bounds is one stack (`MAX_FORM_DEPTH`).
        if self.nesting >= super::MAX_FORM_DEPTH {
            self.note(Unsupported::LimitReached {
                limit: "MAX_FORM_DEPTH",
            });
            return;
        }

        // Table 31 lets `/Contents` be "an array of streams", so there is no one stream object to
        // window: the parts are assembled by the same reader a page of this document uses, under
        // the same `max_stream_len`, and what it could not decode is reported as itself.
        let (bytes, issues) = page.content_with_report(supplied.document());
        let content = super::reader::NestedContent::constructed(
            bytes.into(),
            format!("the imported page of {} (§8.10.4)", supplied.name()),
        );

        let frame = self.enter_imported(supplied.document(), &page, inner.clip);
        for issue in issues {
            self.note(Unsupported::Content { issue });
        }
        let outer_base = std::mem::replace(&mut self.base, inner.transform);
        let group = self.transparency_group(proxy);
        let resources = self.page_resources.as_ref().clone();
        match group {
            None => self.run(&content, &resources, &inner),
            Some(group) => {
                self.run_transparency_group(&group, &content, &resources, &inner, state);
            }
        }
        // §8.10.4.3's first consideration, and the only one of its two that binds a reader:
        //
        // > When the page imported by a reference XObject contains annotations (see 12.5,
        // > "Annotations"), all annotations that contain a printable, unhidden, visible
        // > appearance stream (12.5.5, "Appearance streams") shall be included in the rendering
        // > of the imported page.
        //
        // The same pass a page of this document gets, against the target page's own annotations
        // and the target document's own viewer state — which is what `enter_imported` swapped in,
        // and which is why those three adjectives come out right without being restated here:
        // Table 167's flags are read off each annotation by `draw_annotation`, as they are for
        // every page. The base transform is the imported page's default user space, which is
        // where its `/Rect`s are stated, and that is `inner.transform`.
        //
        // The clause's second consideration is a `may` and this reader takes it: "[l]ogical
        // structure information associated with a page … may be ignored when importing that page
        // into another document", which `enter_imported` does by handing the span an empty parent
        // tree. The clause gives the reason — elements on the imported page "are typically part of
        // a larger structure pertaining to the document as a whole" — and §14.7.5.4's identifiers
        // would in any case be the *target* document's, filed against the containing document's
        // tree.
        self.draw_annotations(&page, inner.transform);
        self.base = outer_base;
        self.leave_imported(frame);
    }

    /// Swaps in everything that is about the target document, for the span of its page.
    ///
    /// See [`ImportedFrame`] for why each of these is a correctness matter and not a cache.
    fn enter_imported(
        &mut self,
        document: &'a pdf_syntax::Document,
        page: &crate::page::Page,
        clip: Option<pdf_render::ClipId>,
    ) -> ImportedFrame<'a> {
        // §12.5.3's placement is the *reader's* magnification rather than either file's, so it is
        // the one thing carried across; everything else this state holds — §12.6.4.11's overrides,
        // the annotations a person added — is filed under an `ObjectId` of the containing
        // document and means nothing here.
        let mut view = crate::view::ViewState::of(document);
        view.set_magnification(self.view.magnification());
        ImportedFrame {
            document: std::mem::replace(&mut self.document, document),
            across: self.across.take(),
            fonts: std::mem::take(&mut self.fonts),
            icc_spaces: std::mem::take(&mut self.icc_spaces),
            shadings: std::mem::take(&mut self.shadings),
            resource_tables: std::mem::take(&mut self.resource_tables),
            image_masks: std::mem::take(&mut self.image_masks),
            image_rasters: std::mem::take(&mut self.image_rasters),
            // §8.10.4.3's second consideration, taken: an imported page's logical structure is
            // ignored, so nothing here has a tree to be read against.
            structure: std::mem::replace(
                &mut self.structure,
                std::sync::Arc::new(crate::structure::ParentTree::default()),
            ),
            stream_structures: std::mem::take(&mut self.stream_structures),
            // §14.11.5's output intent describes the document whose colours it calibrates, and
            // the imported page's colours are the target document's.
            output_intent: std::mem::replace(
                &mut self.output_intent,
                super::output_intent_space(document, Some(&page.dict)),
            ),
            // §8.11's configuration likewise: an optional content group is identified by *which
            // object* it is (§8.11.2.2), in one file.
            optional_content: std::mem::replace(
                &mut self.optional_content,
                crate::optional_content::OptionalContent::read(document),
            ),
            page_resources: std::mem::replace(
                &mut self.page_resources,
                std::sync::Arc::new(page.resources.clone()),
            ),
            view: std::mem::replace(&mut self.view, std::borrow::Cow::Owned(view)),
            // §6.3.2.2's delegation is a host naming widgets of the document it opened.
            delegated: std::mem::take(&mut self.delegated),
            page: std::mem::replace(&mut self.page, super::displayed_size(page)),
            // `pdf-archive`'s cross-check walks §7.8.3's lookups of *one* document against its own
            // survey of that document's streams, so a lookup made in another file's page has no
            // row to be compared with and is not written down (ADR 1055).
            ledger: self.ledger.take(),
            // §14.7.5.2's `/MCID` "uniquely identifies the marked-content sequence within its
            // content stream", and Table 357's `/Stm` names a stream by reference — in one file.
            stream: std::mem::replace(&mut self.stream, ContentStream::Unnameable),
            // §8.10.4.1: the imported page is "clipped to the boundaries of its bounding box",
            // and its annotations are part of its rendering, so they are drawn inside that clip
            // rather than over the containing page like the containing page's own.
            annotations_clipped_to: std::mem::replace(&mut self.annotations_clipped_to, clip),
        }
    }

    /// Puts back what [`Interpreter::enter_imported`] took.
    fn leave_imported(&mut self, frame: ImportedFrame<'a>) {
        let ImportedFrame {
            document,
            across,
            fonts,
            icc_spaces,
            shadings,
            resource_tables,
            image_masks,
            image_rasters,
            structure,
            stream_structures,
            output_intent,
            optional_content,
            page_resources,
            view,
            delegated,
            page,
            ledger,
            stream,
            annotations_clipped_to,
        } = frame;
        self.document = document;
        self.across = across;
        self.fonts = fonts;
        self.icc_spaces = icc_spaces;
        self.shadings = shadings;
        self.resource_tables = resource_tables;
        self.image_masks = image_masks;
        self.image_rasters = image_rasters;
        self.structure = structure;
        self.stream_structures = stream_structures;
        self.output_intent = output_intent;
        self.optional_content = optional_content;
        self.page_resources = page_resources;
        self.view = view;
        self.delegated = delegated;
        self.page = page;
        self.ledger = ledger;
        self.stream = stream;
        self.annotations_clipped_to = annotations_clipped_to;
    }
}

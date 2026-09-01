//! Drawing a page's annotations over its content, in `/Annots` order (§12.5.5).
//!
//! Each appearance is a form `XObject`, so what this module resolves is *where* it goes and
//! under which clip — `crate::annotation` decides what it looks like, and the same operator
//! loop that runs any other form runs it.

use std::sync::Arc;

use pdf_render::display_list::Clip;
use pdf_render::{
    BlendMode, Color, Command, FillRule, Paint, Path, PathCommand, Point, Rect, Stroke, Transform,
};
use pdf_syntax::{Dictionary, Object, ObjectId};

use crate::page::Page;

use super::ext_gstate::blend_mode;
use super::report::{ContentStream, Unsupported};
use super::{GraphicsState, Interpreter};

impl Interpreter<'_> {
    /// Draws the page's annotations over its content, in `/Annots` order.
    ///
    /// ISO 32000-2 §12.5.5: each appearance is a form `XObject`, so this resolves *where* it
    /// goes — `crate::annotation` does that — and then hands it to the same machinery that
    /// runs any other form. The only reason it is a separate pass rather than a `Do` is
    /// that nothing in the content stream refers to it.
    pub(super) fn draw_annotations(&mut self, page: &Page, base: Transform) {
        let annotations = self.document.get_key(&page.dict, "Annots");
        if let Some(entries) = annotations.as_array().map(<[Object]>::to_vec) {
            for entry in &entries {
                let resolved = self.document.resolve(entry);
                let Some(dict) = resolved.as_dict() else {
                    continue;
                };
                let dict = dict.clone();
                self.draw_annotation(&dict, entry.as_reference(), page, base);
            }
        }
        // §12.5.6.10's markups a *person* added, after the page's own and in the order they
        // were added — which is §12.5.5's rule applied to the log beside `/Annots`: an
        // annotation's group "shall be composited with a backdrop consisting of the page content
        // along with any previously painted annotations", and later is later either way.
        // They are drawn from the same three functions the file's own annotations take, because
        // an annotation this program constructed is not a second kind of annotation.
        let added: Vec<(ObjectId, Dictionary)> = self
            .view
            .added_on(page.id)
            .map(|added| (added.id, added.dict.clone()))
            .collect();
        for (id, dict) in added {
            self.draw_annotation(&dict, Some(id), page, base);
        }
    }

    /// Whether any annotation on this page sets ISO 32000-2 §12.5.3's `NoZoom`.
    ///
    /// Table 167's bit 4 is exactly what `ViewGeometry::adjustment` reports back as
    /// `ViewAdjust::view_dependent`, so this is the same question one pass earlier — asked before
    /// the pass runs because what it decides is whether the interpreter's state is worth keeping
    /// at all (`content::Checkpoint`).
    ///
    /// **It over-approximates in one direction only, and that is the direction that costs
    /// nothing.** An annotation this pass then declines to draw — §12.5.3's Hidden or `NoView`,
    /// §8.11.3.3's `/OC`, §12.6.4.11's action, §6.3.2.2's delegation — leaves the page not
    /// view-dependent after all, and the checkpoint is discarded once the pass has said so. The
    /// other direction cannot happen: nothing sets the flag that the file did not.
    ///
    /// The annotations a *person* added are asked too, for `draw_annotations`' reason: an
    /// annotation this program constructed is not a second kind of annotation.
    pub(super) fn any_no_zoom(&self, page: &Page) -> bool {
        let annotations = self.document.get_key(&page.dict, "Annots");
        let stated = annotations.as_array().is_some_and(|entries| {
            entries.iter().any(|entry| {
                let resolved = self.document.resolve(entry);
                resolved.as_dict().is_some_and(|dict| {
                    let view = entry
                        .as_reference()
                        .map(|id| self.view.annotation(id))
                        .unwrap_or_default();
                    crate::annotation::no_zoom_in_force(self.document, dict, view)
                })
            })
        });
        stated
            || self.view.added_on(page.id).any(|added| {
                crate::annotation::no_zoom_in_force(
                    self.document,
                    &added.dict,
                    self.view.annotation(added.id),
                )
            })
    }

    /// Draws one annotation, whether the file states it or a person added it.
    fn draw_annotation(
        &mut self,
        dict: &Dictionary,
        id: Option<ObjectId>,
        page: &Page,
        base: Transform,
    ) {
        // §6.3.2.2's "unless otherwise instructed": a host drawing this field in its own
        // control asked for the page without the picture of it, and §12.5.6.19 makes that
        // picture the field's appearance rather than page content. Silent for the same reason
        // the two conditions below are — an appearance somebody else is drawing is not one this
        // program failed to draw — and narrow for the reason `form::delegated_widgets` states:
        // the set is what `Query::Fields` answered, so nothing leaves the page that no control
        // replaced.
        if let Some(id) = id
            && self.delegated.contains(&id)
        {
            return;
        }
        // §8.11.3.3: "If an annotation contains an OC entry, it shall be visible for
        // screen or print only if the flags have the appropriate settings and the group
        // or membership dictionary indicates it shall be visible." The flags are
        // `decide`'s business (§12.5.3); this is the other half of the condition, and it
        // is silent because an annotation the document hides is not one we failed to
        // draw.
        if let Some(oc) = dict.get("OC").cloned()
            && !self.shows_optional_content(&oc)
        {
            return;
        }
        // §12.6.4.11: a hide action "hides or shows one or more annotations on the screen
        // by setting or clearing their Hidden flags", so what it states is the same flag
        // §12.5.3 defines and overrides what the file wrote there. Silent for the same
        // reason the line above is: an annotation something switched off is not one this
        // program failed to draw.
        // Everything the view state says about this annotation, in one call: whether a hide
        // action named it, which of §12.5.5's three appearances the pointer asks for, and —
        // for a widget — where its value comes from, which §12.7.6.3's reset and §12.7.8's
        // import each change and which decides what §12.7.4.3 lays out.
        let view = id.map(|id| self.view.annotation(id)).unwrap_or_default();
        if view.hidden_by_action == Some(true) {
            return;
        }
        match crate::annotation::decide(
            self.document,
            dict,
            view,
            crate::annotation::ViewGeometry {
                rotate: page.rotate,
                magnification: self.view.magnification(),
            },
        ) {
            crate::annotation::Decision::Nothing => {}
            crate::annotation::Decision::Unsupported(detail) => {
                self.note(Unsupported::Annotation { detail });
            }
            crate::annotation::Decision::Draw {
                appearance,
                owed,
                highlight,
                adjust,
            } => {
                // What the subtype's clause asks for and `crate::appearance` could not
                // construct — a field's value, a bevel's shadow — said out loud beside the
                // part that *is* drawn, rather than either being lost.
                if let Some(detail) = owed {
                    self.note(Unsupported::Annotation { detail });
                }
                let before = self.text.len();
                // §12.5.5: "Any transformation applied to the annotation as a whole shall
                // be applied to the appearance within it" — so §12.5.3's adjustment goes
                // between the appearance's own placement and the page's transform, where it
                // is still in default user space and can undo what the page does to it.
                self.view_dependent |= adjust.view_dependent;
                let base = adjust.transform.then(base);
                self.draw_appearance(&appearance, base, &page.resources);
                self.describe_annotation(dict, before);
                // §12.5.6.19's `/H`, over the appearance rather than instead of it: the
                // clause calls it a *highlighting* mode, and what it highlights is whatever
                // the annotation looks like.
                if let Some(mark) = highlight {
                    self.draw_highlight(mark, base);
                }
            }
        }
    }

    /// §14.9.3's third location for an alternate description: the annotation itself.
    ///
    /// > Any type of annotation (see 12.5, "Annotations") that does not already have a text
    /// > representation, through a Contents entry in the annotation dictionary
    ///
    /// The condition is the clause's own and it is checked rather than assumed: `from` is where
    /// the readback stood before the appearance ran, so an appearance that drew nameable text
    /// *has* a text representation and its `/Contents` is not a substitute for it. One that drew
    /// none — a stamp, a figure, a signature whose glyphs no method can name — has nothing to
    /// vocalise, and the clause says what to say instead.
    ///
    /// The span is empty by construction, because the description replaces nothing: it is what
    /// a screen reader is given *in place of* an annotation that reads as nothing at all.
    /// `speech` puts a word break on each side of it, which is §14.9.3's rule for two
    /// consecutive descriptions and is what keeps it from running into the page's own text.
    fn describe_annotation(&mut self, annotation: &Dictionary, from: usize) {
        if self.text.len() > from {
            return;
        }
        let Object::String(bytes) = self.document.get_key(annotation, "Contents") else {
            return;
        };
        let text = pdf_syntax::text_string(&bytes);
        if text.is_empty() {
            return;
        }
        self.described.push(crate::accessibility::Described {
            range: from..from,
            alt: Some(text),
            expansion: None,
            language: None,
        });
    }

    /// Runs one appearance stream, clipped to its `/BBox`.
    /// Draws §12.5.6.19's `/H` mark over a pressed annotation.
    ///
    /// The clause states the effect arithmetically — "colour values shall be transformed by the
    /// function f(x) = 1 - x" — and §11.3.5.2's Difference mode against white is exactly that:
    /// `B(cb, cs) = |cb - cs|` with every source component at 1 leaves `1 - cb`. So both modes
    /// are one white shape under one blend mode, and neither needs anything new in the display
    /// list or in either backend.
    ///
    /// Not clipped to the appearance's `/BBox`: the clause's subject is "the contents of the
    /// annotation rectangle", which is the rectangle rather than whatever the appearance drew
    /// inside it.
    fn draw_highlight(&mut self, mark: crate::annotation::Mark, base: Transform) {
        let (rect, width) = match mark {
            crate::annotation::Mark::Rectangle(rect) => (rect, None),
            crate::annotation::Mark::Border { rect, width } => (rect, Some(width)),
        };
        let mut path = Path::new();
        path.push(PathCommand::MoveTo(Point::new(rect[0], rect[1])));
        path.push(PathCommand::LineTo(Point::new(rect[2], rect[1])));
        path.push(PathCommand::LineTo(Point::new(rect[2], rect[3])));
        path.push(PathCommand::LineTo(Point::new(rect[0], rect[3])));
        path.push(PathCommand::Close);
        let path = Arc::new(path);
        let paint = Paint::Solid(Color::rgb(1.0, 1.0, 1.0));
        match width {
            None => self.draw(Command::Fill {
                path,
                transform: base,
                fill_rule: FillRule::NonZero,
                paint,
                clip: None,
                mask: None,
                blend: BlendMode::Difference,
            }),
            // §12.5.4 draws a border "completely inside the annotation rectangle", so the
            // stroke is inset by half its own width — the same rule `appearance.rs` applies to
            // the border it draws, and this has to land on the same pixels.
            Some(width) => {
                let half = width / 2.0;
                let inset = [
                    rect[0] + half,
                    rect[1] + half,
                    rect[2] - half,
                    rect[3] - half,
                ];
                let mut path = Path::new();
                path.push(PathCommand::MoveTo(Point::new(inset[0], inset[1])));
                path.push(PathCommand::LineTo(Point::new(inset[2], inset[1])));
                path.push(PathCommand::LineTo(Point::new(inset[2], inset[3])));
                path.push(PathCommand::LineTo(Point::new(inset[0], inset[3])));
                path.push(PathCommand::Close);
                self.draw(Command::Stroke {
                    path: Arc::new(path),
                    transform: base,
                    stroke: Stroke {
                        width,
                        ..Stroke::default()
                    },
                    paint,
                    clip: None,
                    mask: None,
                    blend: BlendMode::Difference,
                });
            }
        }
    }

    fn draw_appearance(
        &mut self,
        appearance: &crate::annotation::Appearance,
        base: Transform,
        page_resources: &Dictionary,
    ) {
        // §12.5.5's stream decoded only as far as its damage, said beside what it drew rather
        // than where it was read — see `crate::annotation::Appearance::damaged` for why the two
        // are different places, and [`Interpreter::content_stream`] for the clause.
        if let Some(stream) = appearance.damaged.clone() {
            self.note(Unsupported::DamagedContentStream { stream });
        }
        let data = match &appearance.content {
            // §12.5.5's stream is one of §7.8.2's four, and it is read as a source rather
            // than as a buffer for the reason ADR 0427 gives: a bomb inside an appearance is
            // as effective as one inside a page, and the memo's own condition decides which
            // of the two shapes it gets.
            crate::annotation::Content::Stored(stream) => {
                let Ok(content) = crate::content::reader::NestedContent::of(
                    self.document,
                    stream,
                    "an annotation's appearance stream (§12.5.5)".to_owned(),
                ) else {
                    self.note(Unsupported::Annotation {
                        detail: "undecodable appearance stream".to_owned(),
                    });
                    return;
                };
                content
            }
            crate::annotation::Content::Constructed { bytes, .. } => {
                crate::content::reader::NestedContent::constructed(
                    Arc::from(bytes.as_slice()),
                    "a constructed appearance stream (§12.7.4.3)".to_owned(),
                )
            }
        };

        let transform = appearance.transform.then(base);
        let mut state = GraphicsState::initial(transform);
        // Table 166: `/ca` and `/CA` are the opacities a *constructed* appearance's nonstroking
        // and stroking operations use. A stored stream carries 1.0 here, because §12.5.2 has a
        // reader ignore both — see `crate::annotation`.
        state.fill_alpha = appearance.fill_alpha;
        state.stroke_alpha = appearance.stroke_alpha;
        if let Some(name) = &appearance.blend {
            state.blend = blend_mode(name.as_bytes());
        }

        // §8.10.2: a form `XObject`'s `/BBox` "shall be" the clip for its content. §12.5.5
        // relies on that — the whole algorithm is about making the box cover `/Rect`, and
        // an appearance drawing outside its own box would spill across the page.
        //
        // **A construction is not a form XObject and may have no box at all**, since the
        // three-hundred-and-fourteenth session: four subtypes state their geometry "in default
        // user space" rather than inside a box, and a file whose `/Rect` does not contain what
        // its own `/L` or `/QuadPoints` states was having those marks clipped away in silence.
        // `crate::appearance::Constructed::bounded` is the reading; ADR 0193 is the argument.
        //
        // §14.11.2.1's clip is not part of that exception either, and it is not a `Clip` here:
        // an annotation is drawn over the page and is not exempt from where the page's contents
        // stop, but that boundary is the whole list's and is stated once, as
        // `DisplayList::set_content_clip`. So what this builds is the box alone.
        let clip = match appearance.bbox {
            Some(bbox) => {
                let mut path = Path::new();
                path.push(PathCommand::MoveTo(Point::new(bbox[0], bbox[1])));
                path.push(PathCommand::LineTo(Point::new(bbox[2], bbox[1])));
                path.push(PathCommand::LineTo(Point::new(bbox[2], bbox[3])));
                path.push(PathCommand::LineTo(Point::new(bbox[0], bbox[3])));
                path.push(PathCommand::Close);
                let Ok(clip) = self.list.add_clip(Clip {
                    path,
                    transform,
                    parent: None,
                    fill_rule: FillRule::NonZero,
                }) else {
                    self.note(Unsupported::LimitReached { limit: "max_clips" });
                    return;
                };
                Some(clip)
            }
            None => None,
        };
        state.clip = clip;

        // §7.8.3: an appearance stream is a form `XObject` (§12.5.5), and one written before
        // PDF 1.2 may omit `/Resources` altogether, in which case the page's dictionary is
        // what its names are looked up in. An empty dictionary instead loses every named font
        // and image the appearance draws with.
        //
        // **This comment used to quote a sentence that is no longer in the standard**, and the
        // four-hundred-and-nineteenth session found it while reading this clause for `Do`.
        // Errata Collection 3 Issue #128 strikes §7.8.3's bullet — "All resources that are
        // referenced from those forms and fonts shall be inherited from the resource dictionary
        // of the page on which they are used" — and replaces it with NOTE 3, which is
        // informative and reports the rule rather than stating it: "PDF files written obeying
        // earlier versions of PDF may have omitted the Resources entry in form XObjects, Type 3
        // glyph descriptions or annotation appearance streams used on a page. Those earlier
        // versions state that resources that were referenced from those content streams can be
        // inherited from the resource dictionary of the page on which they are used." The
        // behaviour is unchanged and is now a *choice* about malformed and pre-2.0 files rather
        // than a `shall` — and NOTE 3 is the wider of the two, since it names an annotation
        // appearance stream where the struck bullet named only forms and Type 3 fonts, which is
        // exactly the case this line is. ADR 0255.
        let resources = match &appearance.content {
            crate::annotation::Content::Stored(stream) => self
                .document
                .get_key(&stream.dict, "Resources")
                .as_dict()
                .cloned()
                .unwrap_or_else(|| page_resources.clone()),
            // A constructed stream names at most the fonts §12.7.4.3's `/DA` string reaches,
            // which `crate::appearance` took from Table 224's `/DR`. Everything else it draws
            // is a path in a device colour and names nothing.
            crate::annotation::Content::Constructed { resources, .. } => resources.clone(),
        };

        // §8.7.2, of where a pattern's matrix points:
        //
        // > Similarly, if a pattern is used within a form XObject (see 8.10, "Form XObjects" ),
        // > the pattern matrix maps pattern space to the form's default user space (that is, the
        // > form coordinate space at the time the form is painted with the Do operator).
        //
        // **An appearance stream is a form XObject** — §12.5.5 says so and the comment above
        // this one repeats it — so its patterns map to *its* default space, not to the page's.
        // `run_form` has done this since it was written; this path is the other way into a
        // form's content and did not, so a shading pattern in an annotation's appearance was
        // positioned by the page's default transform. `issue7821.pdf` is a stamp whose rounded
        // box is filled with a `PatternType 2` axial shading: the page's crop box starts at x
        // 445.966 and the shading's `/Coords` run from 163.729 to 315.349, so the whole axis
        // landed off the visible page and `/Extend [true true]` painted the box one flat colour.
        // ADR 0160.
        let outer_base = std::mem::replace(&mut self.base, transform);
        // §14.7.5.2's identifier is unique within its own content stream, and Table 357's `/Stm`
        // names an appearance stream as readily as a form — "see 8.10, "Form XObjects" and 12.5.5,
        // "Appearance streams"". So a sequence closing in here is *not* the page's, whatever
        // number it carries — and where the appearance dictionary named the stream by an indirect
        // reference, it is a stream `/Stm` can name, by the object it is. `Unnameable` is left
        // for what a reference genuinely cannot reach: a stream written directly into the
        // dictionary, or a construction (§12.7.4.3), which is this program's bytes and has no
        // object at all. This used to be `Unnameable` for every appearance, a limit `doc/todo/31`
        // carried; `crate::annotation::Appearance::source` is the reference carried through.
        let outer_stream = std::mem::replace(
            &mut self.stream,
            appearance
                .source
                .map_or(ContentStream::Unnameable, ContentStream::Object),
        );
        // §14.7.5.4's route back is per stream, and an appearance stream may state a
        // `/StructParents` of its own like any other; a stored one that does is read with its own
        // array rather than with the page's.
        let outer_structure = match &appearance.content {
            crate::annotation::Content::Stored(stream) => {
                self.enter_stream_structure(appearance.source, &stream.dict)
            }
            crate::annotation::Content::Constructed { .. } => None,
        };
        // Depth 1 rather than 0: an appearance is itself a form, so a chain of forms
        // inside it is bounded the same way one inside the page content is.
        let mark = self.list.command_count();
        self.run(&data, &resources, &state, 1);
        self.leave_stream_structure(outer_structure);
        self.stream = outer_stream;
        // §8.10.2's box clips the appearance, and where it cuts nothing it is taken back off.
        // A widget's border is the case this was found on: `bug1863910.pdf` states
        // `0.5 0.5 149 21 re s` inside a `/BBox [0 0 150 22]`, so a one-point stroke's outer
        // edge lies *exactly* on the box, and multiplying the stroke's anti-aliased coverage by
        // the clip's cost it **22% of the page's ink**. ADR 0165.
        if let Some(bbox) = appearance.bbox {
            let box_in_page = Rect::from_corners(
                transform.apply(Point::new(bbox[0], bbox[1])),
                transform.apply(Point::new(bbox[2], bbox[3])),
            );
            self.unclip_redundant(mark, box_in_page, Transform::IDENTITY, None);
        }
        // §12.5.5's other transparency sentence, which this path answers by *construction* for
        // the case it names first and not at all for the case it names second: an appearance
        // with no `/Group` "shall be treated as a non-isolated, non-knockout transparency
        // group", which §11.4.4's NOTE 5 makes identical to painting the elements straight onto
        // the page — and one that states a `/Group` gets "the isolated and knockout values
        // specified in the group dictionary", which nothing here builds. Asked of the commands
        // this appearance drew rather than of the entry alone, because both stated values are
        // visible only under conditions the marks decide; `note_appearance_group` has each.
        //
        // A construction (§12.7.4.3) is this program's own bytes and states no `/Group`.
        if let crate::annotation::Content::Stored(stream) = &appearance.content
            && let Some(group) = self.transparency_group(&stream.dict)
        {
            let drawn = self.list.commands()[mark..].to_vec();
            self.note_appearance_group(&group, &drawn);
        }
        self.base = outer_base;
    }
}

//! `ET`: §9.3.6's clipping accumulated over a text object and §9.3.8's knockout judged whole.

use pdf_render::display_list::Clip;
use pdf_render::{Command, FillRule, Transform};

use super::TextObject;
use crate::content::report::Unsupported;
use crate::content::transparency::implicit_knockout_group;
use crate::content::{GraphicsState, Interpreter};

impl Interpreter<'_> {
    /// Turns the glyph outlines a text object accumulated into a clip, at its `ET`.
    ///
    /// ISO 32000-2 §9.3.6:
    ///
    /// > At the end of the text object identified by the ET operator the accumulated glyph
    /// > outlines, if any, shall be combined into a single path, treating the individual
    /// > outlines as subpaths of that path and applying the non-zero winding number rule
    /// > (see 8.5.3.3.2, "Non-zero winding number rule"). The current clipping path in the
    /// > graphics state shall be set to the intersection of this path with the previous
    /// > clipping path.
    ///
    /// Intersection is what the display list's `parent` chain already means, so the new clip
    /// is a child of the one in effect. It is set on the live graphics state rather than on a
    /// saved copy because the clause continues: "It remains in effect until a previous
    /// clipping path is restored by an invocation of the Q operator" — so it outlives the
    /// text object, and `Q` is the only thing that ends it.
    ///
    /// # An empty accumulator is not an empty clip
    ///
    /// > If no glyphs are shown or if the only glyphs shown have no outlines (for example,
    /// > if they are ASCII SPACE characters (20h)), no clipping shall occur.
    ///
    /// Clipping to an empty path would hide everything drawn after the text object, which is
    /// the opposite of what the clause says and would be invisible to every metric this tree
    /// owns except pixels somebody else produced. A text object in mode 7 showing one space
    /// is not a hypothetical: it is what a producer emits when a line of OCR text happens to
    /// be blank.
    pub(in crate::content) fn end_text_object(
        &mut self,
        text: &mut TextObject,
        state: &mut GraphicsState,
    ) {
        // §9.3.8's knockout is a property of the finished object, so this is where it can be
        // judged: two or more glyphs marked the page under a paint that composites, and `Tk`
        // asked for them to knock one another out instead.
        //
        // The condition is deliberately narrow. Treating every text object drawn while `Tk`
        // is true as a group would wrap almost every page in the world, since true is the
        // initial value, and would say nothing: with opaque glyphs and the Normal blend mode
        // the two models produce identical pixels.
        //
        // The clause states the construction exactly, and it is the one §11.4.6's knockout
        // group already has: "the behaviour shall be equivalent to treating the entire
        // text object as if it were a non-isolated knockout transparency group … where each
        // glyph is an individual element in that group's transparency stack", after which
        // "the group results shall be composited with the backdrop, using the Normal blend
        // mode and alpha and soft mask values of 1.0" — which is this command's four other
        // fields. The graphics state is *not* reset for the elements, unlike §11.6.6's group
        // XObject, and it is not: each glyph command already carries the alpha, mask and
        // blend mode in force when it was shown.
        //
        // §11.7.4.4's implicit group is decided here too, and it has to be: a glyph shown in
        // mode 2 or 6 owes a knockout group of its own fill and stroke, and where the object
        // above is built that group is *inside* it. It does not have to be stated, because it
        // computes the same picture flat: in a knockout group every element composites with
        // the initial backdrop, so at each point the topmost element wins, and nesting cannot
        // change which element that is. So the whole-object group subsumes every glyph's, and
        // the per-glyph groups are built only where there is no whole-object group to be
        // inside — `push_combined_glyphs` builds them on the other branch.
        let knockout_owed = text.knockout_owed;
        // The readings the object's glyphs were shown under, and the enclosing record back.
        let (reading, in_force) = match text.enclosing_reading.take() {
            Some(outer) => self.close_reading_scope(outer),
            None => (self.settled_readings(), state.alpha_is_shape),
        };
        if knockout_owed || !text.combined.is_empty() {
            let glyphs = text.composited.len();
            let elements = self.list.split_off_commands(text.start);
            let stated = knockout_owed
                .then(|| {
                    implicit_knockout_group(
                        &elements,
                        reading,
                        self.enclosing_knockout,
                        self.image_masks.shape_masks(),
                        &mut self.one_colour,
                    )
                })
                .flatten();
            if let Some(group) = stated {
                self.draw(Command::Group {
                    commands: group.elements,
                    alpha: 1.0,
                    clip: None,
                    mask: None,
                    blend: group.blend,
                    isolated: group.isolated,
                    knockout: true,
                    // Stated rather than asked: this group carries no clip of its own, and
                    // §8.5.4's intersection at the blit is the only thing the flag decides.
                    // A round that gives it one owes the question — which §11.4.6's
                    // knockout no longer answers by itself (ADR 0554).
                    alpha_is_shape: false,
                    blending: None,
                });
            } else {
                if knockout_owed {
                    self.note(Unsupported::TextKnockout { glyphs });
                }
                self.push_combined_glyphs(elements, text);
            }
        }
        text.knockout_owed = false;
        text.combined.clear();
        text.composited.clear();
        // Whether as §9.3.8's group or glyph by glyph, the glyphs are elements of the
        // enclosing content now, so the reading they were shown under joins its record — and
        // the reading in force at `ET` stays in force, since §9.3.8 lets "[c]hanges made to
        // graphics state parameters within the text object ... persist beyond the end of the
        // text object" (ADR 1319).
        self.absorb_reading(reading, text.start);
        self.note_alpha_source(in_force);

        let path = std::mem::take(&mut text.clip);
        if path.is_empty() {
            return;
        }
        let clip = Clip {
            path,
            // The outlines were mapped into page space as they were collected, because one
            // path cannot carry one transform per glyph.
            transform: Transform::IDENTITY,
            fill_rule: FillRule::NonZero,
            parent: state.clip,
        };
        match self.list.add_clip(clip) {
            Ok(id) => state.clip = Some(id),
            Err(_) => self.note(Unsupported::LimitReached { limit: "max_clips" }),
        }
    }

    /// Pushes a text object's commands back, wrapping §11.7.4.4's fill-and-stroke pairs.
    ///
    /// ISO 32000-2 §11.7.4.4, of a combined fill and stroke — which "include the B , B\* , b ,
    /// and b\* operators … and the painting of glyphs with text rendering mode 2 or 6":
    ///
    /// > In all other cases, a non-isolated knockout group shall be established. Within the
    /// > group, the fill and stroke shall be performed with their respective prevailing alpha
    /// > constants and the prevailing blend mode. The group results shall then be composited
    /// > with the backdrop, using an alpha value of 1.0 and the Normal blend mode.
    ///
    /// The pairs that reach here are the ones §11.7.4.4 sends to its second bullet: the first
    /// needs overprinting enabled for both operations and the two alpha constants equal, and
    /// [`Interpreter::combined_overprint`](crate::content::Interpreter::combined_overprint) has already built its group or found it to be the
    /// two commands as they stand. The construction is identical to the one the `B` operator
    /// gets in [`Interpreter::end_path`](crate::content::Interpreter::end_path), and NOTE 2 says what it is for — "to avoid having
    /// a non-opaque stroke composite with the result of the fill in the region of overlap,
    /// which would produce a double border effect".
    ///
    /// A pair the backends cannot draw as a knockout — one carrying a soft mask, or a fill a
    /// tiling pattern turned into a group — is pushed flat and named once for the whole text
    /// object, because a report per glyph would name the same gap a hundred times on one line.
    fn push_combined_glyphs(&mut self, elements: Vec<Command>, text: &TextObject) {
        let mut owed = false;
        let mut pairs = text.combined.iter().peekable();
        let mut index = text.start;
        let mut rest = elements.into_iter();
        while let Some(command) = rest.next() {
            let pair = pairs.next_if(|(from, _, _)| *from == index).copied();
            let Some((from, to, reading)) = pair else {
                self.draw(command);
                index = index.saturating_add(1);
                continue;
            };
            let mut parts = vec![command];
            parts.extend(
                rest.by_ref()
                    .take(to.saturating_sub(from).saturating_sub(1)),
            );
            index = to;
            if let Some(group) = implicit_knockout_group(
                &parts,
                reading,
                self.enclosing_knockout,
                self.image_masks.shape_masks(),
                &mut self.one_colour,
            ) {
                self.draw(Command::Group {
                    commands: group.elements,
                    alpha: 1.0,
                    clip: None,
                    mask: None,
                    blend: group.blend,
                    isolated: group.isolated,
                    knockout: true,
                    // §11.4.6's accumulation is not §11.4.4's union, so this group's raster is not
                    // asked to carry Table 139's shape — see `group_alpha_is_shape`.
                    alpha_is_shape: false,
                    blending: None,
                });
            } else {
                owed = true;
                for part in parts {
                    self.draw(part);
                }
            }
        }
        if owed {
            self.note(Unsupported::CompositedInParts {
                detail: "a glyph filled and stroked by text rendering mode 2 or 6",
            });
        }
    }
}

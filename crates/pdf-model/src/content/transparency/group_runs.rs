//! One transparency group's content run for each space it composites in, and the second run of a
//! press pair: the readback the second run must not duplicate, the black half, and the commands a
//! run leaves (ADR 2080).

use std::sync::Arc;

use pdf_render::{BlendMode, Command};
use pdf_syntax::Dictionary;

use crate::colour::{Compositing, Press};

use crate::content::{GraphicsState, Interpreter};

use super::blending_space::{OwnSpaceRun, own_space_conversion};
use super::commands::{any_command, command_composites};
use super::knockout::paired;
use super::{GroupBody, GroupRun, TransparencyGroup};

/// Where the interpreter's readback stood before a content stream was run a second time.
///
/// A group that composites in a four-component space is one content stream interpreted
/// twice — §11.4.7's construction one scope down — and the second run resolves colours
/// differently while placing the same glyphs in the same places. Everything a *reader* gets
/// out of the page would therefore arrive twice; this records where each accumulator stood
/// so that [`Interpreter::rewind_readback`] can put the second run's copy back off. What is
/// deliberately **not** rewound: `operations` (the second run is real work the budget has to
/// count), `unsupported` (keyed, so a duplicate report collapses), and the display list's
/// clip and soft-mask tables (the second run's commands reference the entries it registered).
pub(super) struct ReadbackMark {
    text: usize,
    text_layer: usize,
    described: usize,
    artifacts: usize,
    marked: usize,
    associated: usize,
    glyphs: usize,
    codes_without_a_glyph: usize,
    codes_reaching_a_blank_glyph: usize,
    codes_without_a_vertical_form: usize,
    codes_without_a_character: crate::content::UnnamedCodes,
    glyph_coverage: std::collections::BTreeMap<String, crate::content::Coverage>,
    inferred_separators: usize,
    word_gaps: usize,
    text_operations: usize,
    reversed_chars: usize,
    text_cursor: Option<(f32, f32)>,
}

impl<'a> Interpreter<'a> {
    /// Records where every readback accumulator stands. See [`ReadbackMark`].
    pub(super) fn readback_mark(&self) -> ReadbackMark {
        ReadbackMark {
            text: self.text.len(),
            text_layer: self.text_layer.len(),
            described: self.described.len(),
            artifacts: self.artifacts.len(),
            marked: self.marked.len(),
            associated: self.associated.len(),
            glyphs: self.glyphs,
            codes_without_a_glyph: self.codes_without_a_glyph,
            codes_reaching_a_blank_glyph: self.codes_reaching_a_blank_glyph,
            codes_without_a_vertical_form: self.codes_without_a_vertical_form,
            codes_without_a_character: self.codes_without_a_character,
            glyph_coverage: self.glyph_coverage.clone(),
            inferred_separators: self.inferred_separators,
            word_gaps: self.word_gaps.len(),
            text_operations: self.text_operations,
            reversed_chars: self.reversed_chars,
            text_cursor: self.text_cursor,
        }
    }

    /// Puts the readback back where [`Interpreter::readback_mark`] recorded it.
    pub(super) fn rewind_readback(&mut self, mark: ReadbackMark) {
        self.text.truncate(mark.text);
        self.word_gaps.truncate(mark.word_gaps);
        self.text_layer.truncate(mark.text_layer);
        self.described.truncate(mark.described);
        self.artifacts.truncate(mark.artifacts);
        self.marked.truncate(mark.marked);
        self.associated.truncate(mark.associated);
        self.glyphs = mark.glyphs;
        self.codes_without_a_glyph = mark.codes_without_a_glyph;
        self.codes_reaching_a_blank_glyph = mark.codes_reaching_a_blank_glyph;
        self.codes_without_a_vertical_form = mark.codes_without_a_vertical_form;
        self.codes_without_a_character = mark.codes_without_a_character;
        self.glyph_coverage = mark.glyph_coverage;
        self.inferred_separators = mark.inferred_separators;
        self.text_operations = mark.text_operations;
        self.reversed_chars = mark.reversed_chars;
        self.text_cursor = mark.text_cursor;
    }

    /// The second run of a group compositing in a press — its elements in the black
    /// component — paired with the first, or `None` where the pair is given up.
    ///
    /// Given up, each with the report or record it always had, where nothing composites in
    /// the group (an opaque Normal mark carries its colour through whatever space it is
    /// carried through), where a group inside changed the space with something compositing in
    /// it and could not be drawn there, and where the two runs drew different structures.
    ///
    /// **Two runs of a valid content stream can draw different structures**, and the one way
    /// this tree knows is [`super::implicit_knockout::blend_at_the_do`]'s one solid colour: each run compares only its
    /// own plane's components, so a `B` filling `0 0 0 0.3 k` and stroking `0 0 0 0.7 K` under
    /// `/BM /Darken` is one colour to the chromatic run and two to the black one. Where the
    /// runs parted after that admission was made (`admitted` is the count before the
    /// chromatic run), both are run again with it refused, which puts both halves on the
    /// construction that holds whatever the colours are, and `chromatic` becomes the first of
    /// the two — the readback of the first chromatic run stands and the reruns' is rewound,
    /// because what a reader gets out of the page does not depend on a construction (ADR 1657).
    /// The comparison is still checked rather than assumed, for whatever else could part them.
    #[expect(
        clippy::too_many_arguments,
        reason = "the group's run as the caller holds it: its body, the press, both compositing \
                  targets, the chromatic list to replace and the count to compare"
    )]
    fn black_half(
        &mut self,
        press: &Arc<Press>,
        own: &Compositing,
        body: &GroupBody<'_, 'a>,
        resources: &Dictionary,
        inner: &GraphicsState,
        mark: usize,
        chromatic: &mut Vec<Command>,
        admitted: usize,
    ) -> Option<pdf_render::GroupBlending> {
        if self.nested_space_departed || !any_command(chromatic, &command_composites) {
            return None;
        }
        let black_plane = Compositing::Subtractive(
            crate::colour::Plane::Black,
            Arc::clone(press),
            self.spots_beside.clone(),
        );
        let rewind = self.readback_mark();
        let black = self.run_group_in(&black_plane, body, resources, inner, mark);
        self.rewind_readback(rewind);
        let pair = |black| pdf_render::GroupBlending::FourComponents {
            space: press.blending_space(),
            black,
        };
        if paired(chromatic, &black) {
            return Some(pair(black));
        }
        if self.one_colour.refused || self.one_colour.admitted == admitted {
            return None;
        }
        self.one_colour.refused = true;
        let rewind = self.readback_mark();
        let again = self.run_group_in(own, body, resources, inner, mark);
        let black = self.run_group_in(&black_plane, body, resources, inner, mark);
        self.rewind_readback(rewind);
        self.one_colour.refused = false;
        if !paired(&again, &black) {
            return None;
        }
        *chromatic = again;
        Some(pair(black))
    }

    /// One run of a group's content compositing in `compositing`, its elements split off the
    /// list at `mark`, with the reading scope restarted for it and `self.compositing` put back.
    fn run_group_in(
        &mut self,
        compositing: &Compositing,
        body: &GroupBody<'_, 'a>,
        resources: &Dictionary,
        inner: &GraphicsState,
        mark: usize,
    ) -> Vec<Command> {
        let saved = std::mem::replace(&mut self.compositing, compositing.clone());
        self.restart_reading_scope(inner.alpha_is_shape, mark);
        self.run_group_body(body, resources, inner);
        self.compositing = saved;
        self.list.split_off_commands(mark)
    }

    /// Runs a group's content and collects its commands — twice where the group states a
    /// blending colour space of four components, once more where the two runs diverge, and
    /// once under [`Compositing::Grey`] or [`Compositing::Calibrated`] where it states one
    /// component or three ([`Interpreter::group_own_space`]).
    ///
    /// The first run is the one whose readback is kept, because a colour changes no glyph's
    /// place; every run after it is rewound (see [`ReadbackMark`]). Where
    /// [`Interpreter::group_press`] names a press, the first run resolves colours into its
    /// chromatic half and the second into its black half — §11.4.7's construction one scope
    /// down, see `pdf_render::GroupBlending` — and the two are paired only if their
    /// structures agree. Two cases fall back to one run on the device's components, each
    /// with the report it always had: a group in which **nothing composites**, where
    /// §11.3.4's per-component question cannot change a pixel (an opaque Normal mark
    /// carries its colour through whatever space it is carried through, which is the same
    /// condition the report fires on), and a run that stated §11.7.5.3's black generation,
    /// which the conversion into the space does not read.
    ///
    /// The third value is which of §11.6.4.3's `/AIS` readings the content ran under —
    /// scoped here, seeded from the state at the `Do`, because the entry is a graphics
    /// state parameter and the page-wide flag it used to be refused knockout groups whole
    /// forms away from any statement of it.
    pub(super) fn group_commands(
        &mut self,
        group: &TransparencyGroup,
        body: &GroupBody<'_, 'a>,
        resources: &Dictionary,
        states: (&GraphicsState, &GraphicsState),
        changed: bool,
    ) -> GroupRun {
        let (inner, outer) = states;
        let cmyk_outside = self.enter_cmyk_group(group, resources);
        let mark = self.list.command_count();
        // §11.7.5.2's fifth condition: "[t]he foregoing four conditions were also true at the
        // time the `Do` operator was invoked for the group containing the object, as well as
        // for any direct ancestor groups." They are read off `outer`, the state at the `Do` —
        // §11.6.6 has already reset them on `inner` — and anded into what was already in force,
        // which is what "as well as for any direct ancestor groups" asks for. `Do` is a
        // non-stroking painting operation, so it is the non-stroking constant; the image
        // condition cannot apply to a form.
        let opaque_at_do = self.opaque_ancestry
            && outer.fill_alpha >= 1.0
            && outer.blend == BlendMode::Normal
            && outer.soft_mask.is_none();
        let outer_ancestry = std::mem::replace(&mut self.opaque_ancestry, opaque_at_do);
        let outer_readings = self.open_reading_scope(inner.alpha_is_shape);
        // §11.7.5.3's second bullet, read off `outer` because that is the state at the `Do`:
        // "the rendering intent used shall be the current rendering intent in effect at the
        // time the `Do` operator is applied to the group". §11.6.6 has already reset the
        // group's own parameters on `inner`, so `inner` is the wrong state to ask — and the
        // parameters this names are not among the ones it resets in any case.
        // Saved first: on a spot plane the question below may narrow what the group's content
        // resolves colours in without drawing the group in a space of its own (ADR 1329).
        let saved = self.compositing.clone();
        let own = self.spot_group_compositing(
            group,
            resources,
            outer.rendering(),
            changed,
            outer.pattern_tint.is_some(),
        );
        let own_space = own.is_some();
        // Scoped only where the group is being drawn in a space of its own: everywhere else
        // the record has to propagate *up* to whatever such run this group may be inside.
        let saved_departed = if own_space {
            std::mem::replace(&mut self.nested_space_departed, false)
        } else {
            self.nested_space_departed
        };
        if let Some(own) = &own {
            self.compositing = own.clone();
        }
        let admitted = self.one_colour.admitted;
        self.run_group_body(body, resources, inner);
        self.compositing = saved.clone();
        let mut commands = self.list.split_off_commands(mark);
        let mut pair = None;
        let mut in_own_space = false;
        if let Some(own) = &own
            && !commands.is_empty()
        {
            // `Some(out)` is drawn in its own space and leaves by `out`; `None` gave the
            // space up and runs the content again in what the parent composites in — with
            // the report or record it always had, since a report about a space can only be
            // made where the device's components are what is being composited on, which the
            // rerun is where the parent is the device, and elsewhere the record is what the
            // enclosing run reads.
            let drawn = match own {
                Compositing::Subtractive(_, press, _) => self
                    .black_half(
                        press,
                        own,
                        body,
                        resources,
                        inner,
                        mark,
                        &mut commands,
                        admitted,
                    )
                    .map_or(OwnSpaceRun::GivenUp, |pair| OwnSpaceRun::Drawn(Some(pair))),
                // A group inside changed the space with something compositing in it and
                // could not be drawn there, so its `Do` owes a conversion no list here
                // carries.
                Compositing::Grey | Compositing::Calibrated(_) | Compositing::Additive(_) => {
                    if self.nested_space_departed {
                        OwnSpaceRun::GivenUp
                    } else {
                        OwnSpaceRun::Drawn(own_space_conversion(own))
                    }
                }
                // The device's own components inside a parent that composites elsewhere:
                // a raster of its own only where something composites in it, since an
                // opaque Normal mark converted into the parent per mark is the same picture.
                Compositing::Device => {
                    if !self.nested_space_departed && any_command(&commands, &command_composites) {
                        OwnSpaceRun::Drawn(None)
                    } else {
                        OwnSpaceRun::GivenUp
                    }
                }
                // Never handed out by `group_compositing`: a mask's channel is derived, not
                // painted at a `Do`.
                Compositing::Luminosity(_) => OwnSpaceRun::GivenUp,
            };
            let drawn = match drawn {
                OwnSpaceRun::Drawn(out) => self.conversion_into_parent(
                    own,
                    out,
                    outer.rendering(),
                    outer.black_generation.as_ref(),
                ),
                OwnSpaceRun::GivenUp => OwnSpaceRun::GivenUp,
            };
            match drawn {
                OwnSpaceRun::Drawn(out) => {
                    in_own_space = true;
                    pair = out;
                }
                OwnSpaceRun::GivenUp => {
                    commands = self.rerun_inheriting(body, resources, inner, mark);
                }
            }
        }
        if own_space {
            self.nested_space_departed = saved_departed;
        }
        self.opaque_ancestry = outer_ancestry;
        self.cmyk_group = cmyk_outside;
        // The enclosing record comes back as it was: whether this group's readings join it is
        // a question about what the group *became*, which `run_transparency_group` answers
        // once the command is built.
        let (ais_inside, _) = self.close_reading_scope(outer_readings);
        GroupRun {
            commands,
            pair,
            alpha_sources: ais_inside,
            in_own_space,
        }
    }

    /// One more run of a group's content with colours resolved for what the parent
    /// composites in, replacing what a run in the group's own space drew. See
    /// [`Interpreter::group_commands`] for the cases.
    pub(super) fn rerun_inheriting(
        &mut self,
        body: &GroupBody<'_, 'a>,
        resources: &Dictionary,
        inner: &GraphicsState,
        mark: usize,
    ) -> Vec<Command> {
        let rewind = self.readback_mark();
        self.restart_reading_scope(inner.alpha_is_shape, mark);
        self.run_group_body(body, resources, inner);
        self.rewind_readback(rewind);
        self.list.split_off_commands(mark)
    }
}

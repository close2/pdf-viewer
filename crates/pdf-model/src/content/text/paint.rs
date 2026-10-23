//! §9.3.6's fill, stroke and clip of one outline glyph, and the implicit group around its parts.

use std::rc::Rc;
use std::sync::Arc;

use pdf_render::{BlendMode, ClipId, Command, FillRule, Path, Transform};

use super::TextObject;
use super::glyph_painting::{GlyphPainting, Implicit};
use crate::content::pattern::{PatternPaint, Tiled};
use crate::content::transparency::{Painted, outline_bounds};
use crate::content::{GraphicsState, Interpreter};

impl Interpreter<'_> {
    /// Fills one glyph outline, which a pattern makes more than a `Fill` command.
    ///
    /// §9.2.3 lets a glyph be painted "in any colour", and §8.7.2 makes a pattern one: "All
    /// patterns shall be treated as colours". A *tiling* pattern is not a paint, though — it
    /// is a cell replayed across an area — so a glyph filled with one is its outline tiled,
    /// exactly as a path is. The transform is the *glyph's* rather than the text object's,
    /// because the outline is in glyph space.
    ///
    /// Answers whether a tiling cell the glyph was filled through composites by its own marks.
    fn fill_glyph(
        &mut self,
        outline: &Arc<Path>,
        transform: Transform,
        painted: (&GraphicsState, &GraphicsState),
        clip: Option<ClipId>,
        blend: BlendMode,
    ) -> bool {
        // The state the glyph is painted under, and the one its *colour* is built from: the
        // two differ only inside §11.7.4.4's first-bullet group, where the parts paint at an
        // alpha constant of 1.0 (`Interpreter::show_program_glyph`).
        let (state, paints) = painted;
        // Borrowed rather than cloned: this runs once per glyph, and cloning the whole
        // `Option<PatternPaint>` would bump a shading's refcount on every glyph of a page whose
        // text is painted with one.
        if let Some(PatternPaint::Tiling(tiling)) = &state.fill_pattern {
            let tiling = Rc::clone(tiling);
            // `paints` rather than `state`, for the same reason the colour is built from it:
            // the cell's own marks are the part §11.7.4.4's first bullet paints at an alpha
            // constant of 1.0, and its group applies the stated one once.
            return self.tile(
                outline,
                transform,
                Tiled::Fill(FillRule::NonZero),
                &tiling,
                paints,
            );
        }
        let transfer = self.mark_transfer(state, Painted::of(state, false));
        let paint = self.fill_paint(paints);
        self.draw_mark(
            Command::Fill {
                // The font hands out shared outlines and the display list keeps them shared: a
                // page of text is the same few dozen glyphs over and over, so this is a refcount
                // rather than a copy of the segments.
                path: Arc::clone(outline),
                transform,
                // Glyph outlines are non-zero filled; even-odd would hollow out counters that
                // overlap, such as in a bold 'B'.
                fill_rule: FillRule::NonZero,
                paint,
                clip,
                mask: state.soft_mask,
                blend,
            },
            transfer,
        );
        false
    }

    /// Strokes one glyph outline, ISO 32000-2 §9.3.6 rendering modes 1, 2, 5 and 6.
    ///
    /// `glyph_to_user` maps the outline from glyph space to the *user* space in effect,
    /// which is the whole reason this is not two lines beside the fill. The clause puts the
    /// stroke's parameters in that space:
    ///
    /// > The graphics state parameters affecting those operations, such as line width, shall
    /// > be interpreted in user space rather than in text space.
    ///
    /// A [`Command::Stroke`]'s width and dash lengths are in its path's own space, so
    /// leaving the outline in em units would have divided the width by the font size and
    /// stretched it by the horizontal scaling — an 11-point glyph would have been outlined
    /// about eleven times too thickly, and a horizontally scaled one anisotropically. Moving
    /// the geometry instead is exact for any text matrix, including one that shears; the
    /// cost is a copy of the outline per stroked glyph, which is paid only by the modes that
    /// stroke and never on the ordinary fill path.
    ///
    /// # A glyph stroked in a tiling pattern is tiled
    ///
    /// §8.7.2's "All patterns shall be treated as colours" makes a glyph's stroke colour no
    /// different from a path's, so a `Tr 1` glyph whose `SCN` names a tiling pattern is never
    /// outlined in whatever solid colour was last set — the silent fallback principle 3 forbids —
    /// but drawn by the same [`Interpreter::tile`](crate::content::Interpreter::tile) the fill route takes and over the same outline
    /// this function has already moved into user space. ADR 0735.
    ///
    /// Answers whether a tiling cell the glyph was stroked through composites by its own marks.
    fn stroke_glyph(
        &mut self,
        outline: &Arc<Path>,
        glyph_to_user: Transform,
        painted: (&GraphicsState, &GraphicsState),
        blend: BlendMode,
    ) -> bool {
        // As [`Interpreter::fill_glyph`](crate::content::Interpreter::fill_glyph): the state painted under, and the one the colour is
        // built from.
        let (state, paints) = painted;
        let mut in_user_space = Path::new();
        in_user_space.extend_transformed(outline, glyph_to_user);
        let in_user_space = Arc::new(in_user_space);
        if let Some(PatternPaint::Tiling(tiling)) = &state.stroke_pattern {
            let tiling = Rc::clone(tiling);
            // `paints`, as [`Interpreter::fill_glyph`](crate::content::Interpreter::fill_glyph).
            return self.tile(
                &in_user_space,
                state.transform,
                Tiled::Stroke(&state.stroke),
                &tiling,
                paints,
            );
        }
        let glyph_stroke_clip = self.paint_clip(state, false);
        let transfer = self.mark_transfer(state, Painted::of(state, true));
        let paint = self.stroke_paint(paints);
        self.draw_mark(
            Command::Stroke {
                path: in_user_space,
                transform: state.transform,
                stroke: state.stroke.clone(),
                paint,
                clip: glyph_stroke_clip,
                mask: state.soft_mask,
                blend,
            },
            transfer,
        );
        false
    }

    /// Draws one glyph of an outline font, in whichever of §9.3.6's three operations apply.
    ///
    /// `places` is the glyph's two transforms: into page space, and into user space — the
    /// second is what a stroke needs, since §9.3.6 makes the stroke's width a user-space
    /// quantity like any other path's.
    pub(super) fn show_program_glyph(
        &mut self,
        outline: &Arc<Path>,
        places: [Transform; 2],
        painted: (&GraphicsState, &GraphicsState, Option<ClipId>),
        text: &mut TextObject,
        painting: GlyphPainting,
    ) {
        let [transform, glyph_to_user] = places;
        // The state the glyph is painted under, the one its *colour* is built from — the two
        // differ only inside §11.7.4.4's first-bullet group — and the clip its fill takes.
        let (state, paints, fill_clip) = painted;
        if painting.fills || painting.strokes {
            // Marked the page; see `Interpretation::glyphs`. An empty outline — a space in a
            // font that has one — is a glyph the font drew and is counted, because the
            // question this answers is what *kind* of page this is.
            self.glyphs = self.glyphs.saturating_add(1);
        }
        let parts_at = self.list.command_count();
        // §11.7.4.4's pair is read under the readings its own parts are painted under, which
        // only a glyph that may combine asks about.
        let enclosing_reading = (painting.combining || painting.tiled_pair)
            .then(|| self.open_knockout_reading_scope(state.alpha_is_shape));
        let mut cells_composite = false;
        if painting.fills {
            cells_composite |= self.fill_glyph(
                outline,
                transform,
                (state, paints),
                fill_clip,
                painting.fill_blend,
            );
        }
        if painting.strokes {
            cells_composite |= self.stroke_glyph(
                outline,
                glyph_to_user,
                (state, paints),
                painting.stroke_blend,
            );
        }
        if painting.implicit != Implicit::None {
            self.wrap_in_the_implicit_group(state, parts_at, painting.implicit);
        }
        // §11.7.4.4 makes this glyph's fill and stroke one object; the range is recorded and
        // `ET` decides what to build from it. Fewer than two commands is a glyph that marked
        // the page once — an empty outline, or a fill a tiling pattern drew nothing for — and
        // there is nothing for it to composite with.
        if let Some(outer) = enclosing_reading {
            let (reading, _) = self.close_reading_scope(outer);
            self.absorb_reading(reading, parts_at);
            if (painting.combining || cells_composite)
                && self.list.command_count() > parts_at.saturating_add(1)
            {
                text.combined
                    .push((parts_at, self.list.command_count(), reading));
            }
        }
        if painting.clipping {
            // §9.3.6 wants "a single path, treating the individual outlines as subpaths of
            // that path", and the glyphs of one text object have as many transforms as there
            // are glyphs — so the transform is baked in here and the clip carries none. Note
            // that a hidden layer still reaches this line.
            text.clip.extend_transformed(outline, transform);
        }
        // §9.3.8 asks whether the glyphs composite, and a glyph painted through a cell whose
        // marks do composites whatever the state at the `Tj` says.
        if painting.knockout_can_show || (painting.knockout_by_cells && cells_composite) {
            text.note_knockout(outline_bounds(outline, transform));
        }
    }

    /// §11.7.4's implicit group around the commands the glyph starting at `mark` left behind.
    ///
    /// Out of line because the test above it is what every glyph of every page pays and the
    /// answer is [`Implicit::None`] on all but a handful. Measured under callgrind on page 101
    /// of ISO 32000-2 — the tree's densest text page — interpreted fifty times, against the same
    /// build with §11.7.4's decisions and constructions planted away:
    ///
    /// | what stands in `show_text`'s glyph loop | instructions | of one interpretation |
    /// |---|---|---|
    /// | nothing: the rest of §11.7.4 only | +513 947 | 0.041% |
    /// | this call, guarded and out of line | **+2 788 934** | **0.223%** |
    /// | the three-armed match inlined instead | +3 163 973 | 0.253% |
    ///
    /// So the split is worth 375 039 instructions and the wrap *point* is worth the rest: a
    /// call in a loop this large costs the registers spilled around it, and the only way to
    /// stop paying that is not to build the clause's group. It is a page's glyphs times two
    /// instructions or so, and it buys §11.7.4.3's last paragraph and §11.7.4.4's first bullet
    /// on every glyph that owes one. `CLAUDE.md` asks for the number beside the technique;
    /// [`Interpreter::overprint_blend`](crate::content::Interpreter::overprint_blend) carries the same split one clause over.
    #[inline(never)]
    fn wrap_in_the_implicit_group(
        &mut self,
        state: &GraphicsState,
        mark: usize,
        implicit: Implicit,
    ) {
        match implicit {
            Implicit::Special => self.implicit_overprint_group(state, mark),
            Implicit::FirstBullet => self.first_bullet_group(state, mark),
            Implicit::None => {}
        }
    }
}

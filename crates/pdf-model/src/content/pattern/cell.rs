//! §8.7.3.2: one tiling cell run once: the graphics state it starts from, its bounding box, the
//! clip its repeats can drop, and whether anything in it composites (ADR 2081).

use std::rc::Rc;
use std::sync::Arc;

use pdf_render::{BlendMode, ClipId, Command, Point, Rect, Transform};

use crate::content::transparency::{AlphaSourcesSeen, any_command, command_composites};
use crate::content::{GraphicsState, Interpreter};

use super::repeat::CellFold;
use super::tiling::Tiling;

impl Interpreter<'_> {
    /// Takes a pattern cell's `/BBox` clip back off it where the clip removes no geometry.
    ///
    /// Returns whether it did, in which case the caller stops applying the box to the cells
    /// that follow — every cell is the same figure translated, so the first one decides.
    ///
    /// # Why a clip that removes nothing is not free
    ///
    /// Table 74 says a cell's box "shall be used to clip the pattern cell", and applying it
    /// where the cell draws nothing outside it is correct and costs a picture. A clip mask is
    /// anti-aliased, so a mark lying *on* the boundary keeps a fraction of its coverage in the
    /// boundary pixel and the neighbouring cell keeps the rest, and two fractions composite as
    /// `1 − (1−a)(1−b)` rather than adding. `issue16038.pdf` rules a grid with a line spanning
    /// exactly its own cell and lost **15% of the ink its geometry states** to that — measured
    /// by removing the clip, `AMBIGUOUS_TILING_CELL_CLIP`. The clip is load-bearing on the
    /// same page's *other* pattern, whose rule sits on the cell edge and is meant to be halved,
    /// which is why this is a question rather than a rule.
    ///
    /// # Why it is decided after the cell is drawn rather than before
    ///
    /// The extent of a cell's marks is not known until its content stream has run, and running
    /// it twice is not free of consequence: the readback, the text layer, the artifact spans
    /// and §9.3.8's overlap bookkeeping all accumulate as it goes. So the cell is drawn *with*
    /// the clip and the clip is removed afterwards, which needs no rollback at all — the
    /// commands carry their geometry and name their clip, and only the name changes.
    ///
    /// Conservative in three places, each of which keeps a picture rather than a saving: a
    /// command whose extent cannot be bounded, a command whose clip is a *chain* the cell's own
    /// content built on top of the box, and a box that fails to contain what the cell drew.
    fn unclip_redundant_cell(
        &mut self,
        mark: usize,
        corners: [f32; 4],
        offset: Transform,
        to_pattern: Transform,
        outer: Option<ClipId>,
    ) -> bool {
        let [x0, y0, x1, y1] = corners;
        let box_in_pattern = Rect::from_corners(
            offset.apply(Point::new(x0, y0)),
            offset.apply(Point::new(x1, y1)),
        );
        self.unclip_redundant(mark, box_in_pattern, to_pattern, outer)
    }

    /// The graphics state a tiling pattern's cell starts in, placed by `to_page` (ISO 32000-2
    /// §8.7.3.1, §11.6.7).
    ///
    /// §8.7.3.1's second step installs the state the parent content stream began with
    /// ([`super::initial::PatternInitial`]'s `stream`), and §11.6.7 says which of its parameters do not carry:
    ///
    /// > As always for transparency groups, those parameters related to transparency (blend
    /// > mode, soft mask, and alpha constant) shall be initialised to their standard default
    /// > values.
    ///
    /// Three parameters are named and the alpha source is not among them, so a cell inside a
    /// form invoked under `/AIS true` reads its own constants as shape (§11.6.4.4). The clip is
    /// the caller's: a cell is bounded by the painted area and Table 74's box, both inside the
    /// parent stream's own clip.
    fn cell_state(&self, to_page: Transform) -> GraphicsState {
        let mut cell = (*self.pattern_initial.stream).clone();
        cell.transform = to_page;
        cell.blend = BlendMode::Normal;
        cell.soft_mask = None;
        cell.fill_alpha = 1.0;
        cell.stroke_alpha = 1.0;
        cell
    }

    /// Runs the pattern's content stream, once, for the cell `to_page` places.
    ///
    /// **Once for the whole tiling**, since ADR 0430: every other site is that cell's commands
    /// displaced, so this is the one interpretation the pattern gets. See [`pdf_render::Cell`]
    /// for why the two are the same picture.
    ///
    /// Returns the clip Table 74's box produced for it, which is what
    /// [`Interpreter::settle_cell_box`] may take back off the commands afterwards, or `None`
    /// where the pattern states no usable box — and the readings of §11.6.4.3's `/AIS` the
    /// cell's marks were painted under, which are the cell's own: §8.7.3.1 runs it under "the
    /// graphics state that was in effect at the beginning of the pattern's parent content
    /// stream", and §11.6.7 adds that "\[t]he definition shall not inherit the current values
    /// of the graphics state parameters at the time it is evaluated". So the record is scoped
    /// to the cell and seeded from the state it starts in, never from the painting mark's, and
    /// [`Interpreter::compose_tiling`] decides whether it joins the enclosing one (ADR 1306).
    fn run_cell(
        &mut self,
        tiling: &Tiling,
        mark: &GraphicsState,
        to_page: Transform,
        outer: Option<ClipId>,
    ) -> (Option<ClipId>, AlphaSourcesSeen) {
        let mut cell = self.cell_state(to_page);
        // Table 74: "These boundaries shall be used to clip the pattern cell." The box is in
        // pattern space, so it travels with the cell's own offset, and it sits *inside* the
        // path's clip rather than replacing it — a cell is bounded by both. A file whose box
        // is unusable keeps the path clip alone.
        let box_clip = tiling
            .bbox
            .and_then(|corners| self.rect_clip(corners, to_page, outer));
        cell.clip = box_clip.or(outer);
        // An uncoloured pattern is a stencil: the colour given alongside the pattern name is
        // what pours through it. §8.6.8 is what makes that true of a cell whose content stream
        // *does* try to set a colour — it is the second of the clause's two circumstances, and
        // the colour operators inside it "shall be ignored" exactly as they are inside a `d1`
        // glyph description.
        let saved_uncoloured = self.uncoloured;
        if let Some(source) = &tiling.tint {
            let binding = Rc::new(super::TintBinding {
                source: source.clone(),
                rendering: mark.rendering(),
                generation: mark.black_generation.clone(),
            });
            let tint = binding.paint(self, true);
            cell.fill = tint;
            cell.stroke_colour = tint;
            cell.pattern_tint = Some(binding);
            // The selected tint owns its source; inherited ordinary sources are unrelated.
            cell.fill_source = None;
            cell.stroke_source = None;
            // The colour poured through the stencil is the tint alone, whatever colour the
            // parent stream began with — a pattern colour among them.
            cell.fill_pattern = None;
            cell.stroke_pattern = None;
            self.uncoloured = true;
        }
        // §8.7.2's last sentence about nesting (ADR 0430):
        //
        // > A pattern can be used within another pattern
        //
        // — and the rest of that sentence says the inner pattern's matrix defines its
        // relationship to the pattern space of the *outer* pattern. It is paraphrased rather
        // than quoted because the standard sets "relationship" broken across a line.
        //
        // So a pattern named inside a cell is anchored to *this cell's* space, exactly as one
        // named inside a form is anchored to the form's (`Interpreter::form`, which has done
        // this since it was written). Anchoring it to the page instead made the tiles differ
        // from one another, which §8.7.3.1's own picture forbids — "the effect is as if the
        // figure were painted on the surface of a clear glass tile, **identical copies** of
        // which were then laid down in an array" — and `issue8565.pdf` is the corpus document
        // that showed it: one radial gradient, page-anchored, under a cell the size of the
        // page.
        let outer_base = std::mem::replace(&mut self.base, to_page);
        self.enter_ledger_frame(crate::content::ledger::Route::TilingPattern, tiling.source);
        let enclosing_reading = self.open_reading_scope(cell.alpha_is_shape);
        self.run(&tiling.content, &tiling.resources, &cell);
        let (reading, _) = self.close_reading_scope(enclosing_reading);
        self.leave_ledger_frame();
        self.base = outer_base;
        self.uncoloured = saved_uncoloured;
        (box_clip, reading)
    }

    /// What Table 74's box clip is doing to the cell, answered once for the whole tiling.
    ///
    /// Returns which of the cell's marks are to be folded, or nothing where the box came off
    /// the cell entirely. The two questions are asked in this order because they are the same
    /// question at two strengths: [`Interpreter::unclip_redundant_cell`] removes a box that
    /// cuts nothing at all, and [`Interpreter::fold_repeated_marks`] deals with a box that cuts
    /// a mark the cell states again a step away — a rule drawn on the box's own edge, which is
    /// one mark of the tiling described twice.
    ///
    /// Both answers reach every site, and since ADR 0430 they reach it by being *copied* rather
    /// than by being applied again: the cell is settled here and the sites are copies of what
    /// this left.
    pub(super) fn settle_cell_box(
        &mut self,
        mark: usize,
        corners: [f32; 4],
        placement: (Transform, Transform),
        step: (f32, f32),
        clips: (Option<ClipId>, Option<ClipId>),
    ) -> CellFold {
        let (offset, to_pattern) = placement;
        if self.unclip_redundant_cell(mark, corners, offset, to_pattern, clips.1) {
            return CellFold::new();
        }
        self.plan_repeated_marks(mark, corners, placement, step, clips.0)
    }

    /// Takes a `/BBox` clip back off the commands it enclosed, where it removes no geometry.
    ///
    /// The rule and its whole argument are `unclip_redundant_cell`'s, and the argument is not
    /// about tiling patterns: **a clip mask is anti-aliased, so a mark lying on the boundary
    /// keeps only a fraction of the boundary pixel** — and where the boundary cuts nothing, that
    /// fraction is ink the geometry states and nothing removes.
    ///
    /// `box_` is the clip's rectangle and `to_box` maps a command's own coordinates into the
    /// space that rectangle is stated in.
    ///
    /// Conservative in the same three places: a command whose extent cannot be bounded, a
    /// command whose clip is a *chain* built on top of the box, and a box that fails to contain
    /// what was drawn. Each keeps the picture rather than the saving. ADRs 0155 and 0165.
    pub(in crate::content) fn unclip_redundant(
        &mut self,
        mark: usize,
        box_: Rect,
        to_box: Transform,
        outer: Option<ClipId>,
    ) -> bool {
        let inside = self.list.commands().get(mark..).unwrap_or_default();
        if inside.is_empty() {
            return false;
        }
        let Some(bbox_clip) = inside.first().and_then(Command::clip) else {
            // The box produced no clip — `rect_clip` refused it, or there was none — so the
            // commands already carry the outer clip and there is nothing to take off.
            return false;
        };
        if Some(bbox_clip) == outer {
            return false;
        }
        for command in inside {
            let contained = pdf_render::marked_bounds(command, to_box)
                .is_some_and(|marks| box_.contains(marks));
            if !contained || command.clip() != Some(bbox_clip) {
                return false;
            }
        }

        let mut commands = self.list.split_off_commands(mark);
        for command in &mut commands {
            command.set_clip(outer);
        }
        for command in commands {
            self.draw(command);
        }
        true
    }

    /// Whether a mark from `mark` on composites (`transparency::command_composites`).
    pub(super) fn marks_composite(&self, mark: usize) -> bool {
        self.list
            .commands()
            .get(mark..)
            .is_some_and(|marks| any_command(marks, &command_composites))
    }

    /// Runs a cell under the two conditions §11.7.5.2 carries into it, and answers the function
    /// the finished tiling carries out.
    ///
    /// `inside` is the clause's fifth condition — whether the mark painting the pattern and every
    /// group above it were fully opaque — and the sixth is accumulated over the cell's own marks:
    ///
    /// > If the current colour is a tiling pattern, all objects in the definition of its pattern
    /// > cell also satisfy the foregoing conditions.
    ///
    /// It is a condition on the object *painted with* the pattern, so the elementary object the
    /// clause chooses a function for at a point a tile covers is that object and not the cell's
    /// marks — and the cell's marks could not carry one anyway, being interpreted once and copied
    /// to every site (§8.7.3.1, ADR 0430). So [`Interpreter::tiling_cell`] withholds the function
    /// from them, and `transfer` comes back only where the cell leaves the condition standing.
    /// ADR 1266.
    pub(super) fn run_cell_scoped(
        &mut self,
        tiling: &Tiling,
        mark: &GraphicsState,
        to_page: Transform,
        clip: Option<ClipId>,
        inside: bool,
        transfer: Option<Arc<crate::content::Transfer>>,
    ) -> (
        Option<ClipId>,
        Option<Arc<crate::content::Transfer>>,
        AlphaSourcesSeen,
    ) {
        let ancestry = std::mem::replace(&mut self.opaque_ancestry, inside);
        let outer_cell = std::mem::replace(&mut self.tiling_cell, true);
        let outer_opaque = std::mem::replace(&mut self.tiling_cell_opaque, true);
        let (box_clip, reading) = self.run_cell(tiling, mark, to_page, clip);
        let cell_opaque = std::mem::replace(&mut self.tiling_cell_opaque, outer_opaque);
        self.tiling_cell = outer_cell;
        self.opaque_ancestry = ancestry;
        (box_clip, transfer.filter(|_| cell_opaque), reading)
    }
}

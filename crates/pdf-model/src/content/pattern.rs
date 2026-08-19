//! §8.7's patterns and shadings: the `sh` operator, shading paints and tiling cells.
//!
//! A shading pattern travels into the display list as a paint; a tiling pattern is a content
//! stream read **once** and its marks copied to every site the lattice reaches, which is why
//! most of this module is about placing, clipping and folding cells rather than about colour.

use std::rc::Rc;
use std::sync::Arc;

use pdf_render::display_list::Clip;
use pdf_render::{
    BlendMode, ClipId, Color, Command, DisplayListError, FillRule, Paint, Path, PathCommand, Point,
    Rect, Shading, ShadingKind, Transform,
};
use pdf_syntax::{Dictionary, Name, Object};

use crate::colour::ColourSpace;

use super::colour::{BlackPoint, convert};
use super::report::Unsupported;
use super::run::narrow;
use super::transparency::{any_command, command_blends};
use super::{GraphicsState, Interpreter, MAX_FORM_DEPTH, MAX_OPERATIONS};

/// What a `/Pattern` colour space's `scn` selected.
///
/// The two kinds are drawn in completely different ways. A shading pattern is a paint and
/// travels into the display list as one. A tiling pattern is a *content stream*, drawn once
/// inside a clip shaped like the path being filled and copied to every other tile position —
/// so it never becomes a paint and is expanded here instead.
#[derive(Debug, Clone)]
pub(super) enum PatternPaint {
    /// A shading pattern (`/PatternType 2`), with Table 77's `/BBox` where it states one.
    ///
    /// The box travels as a rectangle and a transform rather than as a `ClipId`, because
    /// the clause makes it a clip "in addition to the current clipping path … in effect at
    /// that time" — the time being when the *pattern is painted*, which may be several `q`
    /// levels away from the `scn` that selected it.
    Shading(Arc<Shading>, Option<([f32; 4], Transform)>),
    /// A tiling pattern (`/PatternType 1`).
    Tiling(Rc<Tiling>),
}

/// What one pattern cell's commands fold, by position within the cell.
///
/// Each entry names a command's offset from the cell's first and which of its subpaths are the
/// second statement of a mark another cell also makes (§8.7.3.1, §11.6.2 — ADR 0213). It is
/// decided on the one cell that is interpreted and every site inherits it by being a copy of
/// that cell, for the reason [`Interpreter::fold_repeated_marks`] gives.
type CellFold = Vec<(usize, pdf_render::Repeats)>;

/// A tiling pattern: a cell of content, and how to repeat it.
#[derive(Debug)]
pub(super) struct Tiling {
    /// The cell's content stream, read **once** for the whole tiling (ADR 0430).
    ///
    /// Routed like the other three of §7.8.2's nested streams since the cell stopped being
    /// re-interpreted per site: a decode the memo declines is windowed, and a window is read
    /// once here rather than four thousand times. ADR 0427's exception ends with the loop that
    /// caused it.
    content: super::reader::NestedContent,
    /// The resources its operators name.
    resources: Dictionary,
    /// Spacing between cells, in pattern space. Never zero.
    step: (f32, f32),
    /// ISO 32000-2 §8.7.3.1 Table 74's `/BBox`, the pattern cell's bounding box, in pattern
    /// space.
    ///
    /// > These boundaries shall be used to clip the pattern cell.
    ///
    /// Carried per cell rather than applied once, because the clause clips *each* cell: a
    /// cell whose content runs past its own box would otherwise spill into the next cell's,
    /// and where `/XStep` exceeds the box — which is how a pattern tiles with gaps — it would
    /// spill into the gap between them. `/BBox` is required, so a pattern without one is
    /// malformed; it is then not clipped, which is the only reading that draws anything.
    bbox: Option<[f32; 4]>,
    /// Maps pattern space to the page's default space.
    to_page: Transform,
    /// The colour an uncoloured pattern is poured through, if it is uncoloured.
    ///
    /// `/PaintType 2` cells carry no colour of their own; the colour comes from `scn`.
    tint: Option<Color>,
}

impl Interpreter<'_> {
    /// The clip in force for a paint, including a shading pattern's own `/BBox`.
    ///
    /// ISO 32000-2 §8.7.4.3 Table 77 makes `/BBox` "a temporary clipping boundary … in
    /// addition to the current clipping path and any other clipping boundaries in effect at
    /// that time", so it nests *inside* whatever the graphics state already has rather than
    /// replacing it. A pattern with no `/BBox`, or a paint that is not a shading pattern,
    /// gets the state's clip unchanged and costs nothing.
    ///
    /// `None` for the clip is "no clip"; the error case — a display list already holding as
    /// many clips as it can — falls back to the state's own, because losing a bounding box
    /// draws too much and losing the whole command draws nothing.
    pub(super) fn paint_clip(&mut self, state: &GraphicsState, fill: bool) -> Option<ClipId> {
        let pattern = if fill {
            state.fill_pattern.as_ref()
        } else {
            state.stroke_pattern.as_ref()
        };
        let Some(PatternPaint::Shading(shading, bbox)) = pattern else {
            return state.clip;
        };
        let shading = Arc::clone(shading);
        let bbox = *bbox;
        let clip = match bbox {
            Some((corners, transform)) => self
                .rect_clip(corners, transform, state.clip)
                .or(state.clip),
            None => state.clip,
        };
        self.domain_clip(&shading, clip)
    }

    /// The parallelogram a function-based shading's domain occupies, as a clip.
    ///
    /// ISO 32000-2 §8.7.4.5.2 states where a type 1 shading marks and where it does not:
    ///
    /// > The transformation matrix ( Matrix ) then maps the domain rectangle into a
    /// > corresponding rectangle or parallelogram in the target coordinate space. Points wi
    /// > thin the shading's bounding box ( BBox ) that fall outside this transformed domain
    /// > rectangle shall be painted with the shading's background colour ( Background ); if
    /// > the shading dictionary has no Background entry, such points shall be left unpainted.
    ///
    /// **"Or parallelogram" is the whole of it**, and it is why this is a clip rather than a
    /// property of the sampled grid. `function_based_shading.pdf` states
    /// `/Matrix [85 85 -85 85 515 382]` — a rotation — so its domain occupies a diamond, and
    /// this reader painted a *square* against four references' diamond for the project's whole
    /// life. The backend's pattern is padded outside its grid, which is right for the
    /// interpolation and says nothing about where the shading ends; where a shading ends is
    /// this clause's answer and so the interpreter's to compose, exactly as Table 77's
    /// `/BBox` is.
    ///
    /// **`Background` is not implemented and not reported**, which is Table 77's own gap
    /// rather than this function's: the ledger's §8.7.4.3 row carries it, two corpus documents
    /// write one, and the entry applies only where a shading is used as a *pattern*. So this
    /// leaves the outside unpainted, which is the clause's branch for a shading that states no
    /// background — and a shading that states one gets the same treatment silently. An earlier
    /// version of this comment claimed such a shading was refused before reaching here; it is
    /// not.
    ///
    /// Nothing happens for any other shading type: an axial or radial shading says where it
    /// stops through `/Extend`, which its ramp already carries, and a mesh through its
    /// triangles.
    fn domain_clip(&mut self, shading: &Shading, parent: Option<ClipId>) -> Option<ClipId> {
        let ShadingKind::Sampled { domain, .. } = shading.kind.as_ref() else {
            return parent;
        };
        // Table 78's order is [x min x max y min y max], which is not `rect_clip`'s.
        let [x0, x1, y0, y1] = *domain;
        let mut path = Path::new();
        path.push(PathCommand::MoveTo(Point::new(x0, y0)));
        path.push(PathCommand::LineTo(Point::new(x1, y0)));
        path.push(PathCommand::LineTo(Point::new(x1, y1)));
        path.push(PathCommand::LineTo(Point::new(x0, y1)));
        path.push(PathCommand::Close);
        self.list
            .add_clip(Clip {
                path,
                // The shading's own `/Matrix` is composed into this already, which is what
                // makes the domain rectangle's corners the right four points to send.
                transform: shading.transform,
                fill_rule: FillRule::NonZero,
                parent,
            })
            .ok()
            .or(parent)
    }

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

    /// Runs the pattern's content stream, once, for the cell `to_page` places.
    ///
    /// **Once for the whole tiling**, since ADR 0430: every other site is that cell's commands
    /// displaced, so this is the one interpretation the pattern gets. See [`pdf_render::Cell`]
    /// for why the two are the same picture.
    ///
    /// Returns the clip Table 74's box produced for it, which is what
    /// [`Interpreter::settle_cell_box`] may take back off the commands afterwards, or `None`
    /// where the pattern states no usable box.
    fn run_cell(
        &mut self,
        tiling: &Tiling,
        to_page: Transform,
        outer: Option<ClipId>,
    ) -> Option<ClipId> {
        let mut cell = GraphicsState::initial(to_page);
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
        if let Some(tint) = tiling.tint {
            cell.fill = tint;
            cell.stroke_colour = tint;
            self.uncoloured = true;
        }
        // §8.7.2's last sentence about nesting, which this reader did not apply until the
        // five-hundred-and-ninety-fifth session:
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
        self.run(
            &tiling.content,
            &tiling.resources,
            &cell,
            MAX_FORM_DEPTH - 1,
        );
        self.base = outer_base;
        self.uncoloured = saved_uncoloured;
        box_clip
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
    fn settle_cell_box(
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

    /// Finds a mark the cell states twice, a lattice step apart (§8.7.3.1, §11.6.2).
    ///
    /// Returns one entry per command that has one, by position within the cell.
    /// [`Interpreter::fold_repeated_marks`] carries the answer out, on this cell and on every
    /// cell after it: they are one figure at translations of each other, so what folds in one
    /// folds in all.
    ///
    /// # The figure this is for, and why the clip is not the answer
    ///
    /// A producer builds a continuous rule out of a repeating cell by drawing it **on** the
    /// box's edge and stating it twice, at the bottom of the cell and at the top. Table 74's
    /// clip is what keeps that from painting the rule twice at full width — each cell keeps the
    /// half inside its own box — and the halves meet exactly, in geometry. They do not meet on
    /// the raster: a clip mask is anti-aliased, so the boundary pixel keeps a fraction of one
    /// half and a fraction of the other, and two fractions painted one after another composite
    /// as `1 − (1−a)(1−b)` rather than adding. `issue16038.pdf`'s second square came out 13%
    /// under the ink its own geometry states, where §10.7.4 asks for at least it.
    ///
    /// §11.6.2 says the two halves may not composite at all:
    ///
    /// > Portions of an object shall not be composited with one another, even if they are
    /// > described in a way that would seem to cause overlaps
    ///
    /// and §11.6.7 is what makes the whole tiling one object's paint rather than many —
    /// "the colour, shape, and opacity values resulting from the evaluation of the pattern
    /// definition shall be used as the object's source colour ( 𝐶𝑠 ), object shape ( f j ), and
    /// object opacity ( qi )". So the tiling is evaluated to one shape first, and two cells'
    /// contributions to one device pixel are two portions of that shape.
    ///
    /// The fix is therefore neither a buffer nor a clip: it is to notice that the cell's two
    /// copies of the rule are **one mark of the tiling**, keep one of them, and draw it whole.
    /// [`pdf_render::repeated_subpaths`] carries the conditions under which that paints the same set
    /// of points, and refuses where it would not.
    fn plan_repeated_marks(
        &self,
        mark: usize,
        corners: [f32; 4],
        placement: (Transform, Transform),
        step: (f32, f32),
        box_clip: Option<ClipId>,
    ) -> CellFold {
        let (offset, to_pattern) = placement;
        // No box clip was built — `rect_clip` refused it — so nothing halves a mark and there is
        // nothing to fold away from.
        let Some(box_clip) = box_clip else {
            return CellFold::new();
        };
        let [x0, y0, x1, y1] = corners;
        let tiles = pdf_render::Tiles {
            step,
            cell: Rect::from_corners(
                offset.apply(Point::new(x0, y0)),
                offset.apply(Point::new(x1, y1)),
            ),
        };
        let mut plan = CellFold::new();
        for (at, command) in self.list.commands().iter().enumerate().skip(mark) {
            // A command the cell's own content clipped further is skipped, for the reason
            // `unclip_redundant` leaves one alone: the box is then only part of what bounds it.
            if command.clip() != Some(box_clip) {
                continue;
            }
            if let Some(repeats) = pdf_render::repeated_subpaths(command, tiles, to_pattern) {
                plan.push((at.saturating_sub(mark), repeats));
            }
        }
        plan
    }

    /// Carries out what [`Interpreter::plan_repeated_marks`] decided, on the cell at `mark`.
    ///
    /// Every cell states the same figure at a translation, so one cell's answer is every cell's.
    /// The guard is that the command at each planned position still draws a path with the number
    /// of subpaths the answer counted, and a cell that does not is reported rather than cut by
    /// index into something else. Nothing in the interpreter can produce one — the content stream
    /// and the graphics state are identical from cell to cell and only the transform differs — so
    /// the report is there to make a surprise audible rather than to describe a known case.
    fn fold_repeated_marks(
        &mut self,
        mark: usize,
        plan: &CellFold,
        clips: (Option<ClipId>, Option<ClipId>),
    ) {
        let (box_clip, outer) = clips;
        let mut commands = self.list.split_off_commands(mark);
        let mut owed = false;
        for (at, repeats) in plan {
            let folded = commands
                .get(*at)
                .filter(|command| command.clip() == box_clip)
                .and_then(Command::path)
                .and_then(|path| pdf_render::without_subpaths(path, repeats));
            match (folded, commands.get_mut(*at)) {
                (Some(path), Some(command)) => {
                    if let Some(slot) = command.path_mut() {
                        *slot = Arc::new(path);
                        command.set_clip(outer);
                    } else {
                        owed = true;
                    }
                }
                _ => owed = true,
            }
        }
        for command in commands {
            self.list.push(command);
        }
        if owed {
            self.note(Unsupported::Shading {
                name: "a tiling pattern's cells do not all state the same figure".to_owned(),
            });
        }
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
    pub(super) fn unclip_redundant(
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
            self.list.push(command);
        }
        true
    }

    /// Draws every site of a tiling but the one its cell was interpreted at.
    ///
    /// §8.7.3.1: "The pattern cell shall be replicated at fixed horizontal and vertical
    /// intervals to fill the area to be painted" — so what each site needs is the cell's marks
    /// displaced, which is [`pdf_render::Cell::repeat`].
    ///
    /// # What bounds it, which is what bounded it before
    ///
    /// Each copy is charged to [`MAX_OPERATIONS`], the budget the cell's *operators* were
    /// charged to when every site interpreted the content stream again. The trade is exact in
    /// the direction that matters: a command costs at least one operator to state, so no page
    /// that finished its tiling before reaches the bound now, and a cell that copies four
    /// million commands stops at the same place a cell that ran four million operators did.
    /// `MAX_TILES` is still the bound on the site count itself.
    fn repeat_cell(
        &mut self,
        cell: &pdf_render::Cell,
        tiling: &Tiling,
        spans: ((i32, i32), (i32, i32)),
    ) {
        let ((first_column, last_column), (first_row, last_row)) = spans;
        for row in first_row..=last_row {
            for column in first_column..=last_column {
                if (column, row) == (first_column, first_row) {
                    continue;
                }
                let by = displacement(
                    tiling,
                    column.saturating_sub(first_column),
                    row.saturating_sub(first_row),
                );
                match cell.repeat(&mut self.list, by) {
                    Ok(copied) => {
                        self.operations = self.operations.saturating_add(copied);
                        if self.operations > MAX_OPERATIONS {
                            self.note(Unsupported::LimitReached {
                                limit: "MAX_OPERATIONS",
                            });
                            return;
                        }
                    }
                    Err(error) => {
                        self.note(Unsupported::LimitReached {
                            limit: match error {
                                DisplayListError::TooManySoftMasks => "max_soft_masks",
                                _ => "max_clips",
                            },
                        });
                        return;
                    }
                }
            }
        }
    }

    /// Paints a tiling pattern over the area a path covers.
    ///
    /// The path becomes a clip, the pattern's cell is drawn once inside it, and its marks are
    /// copied to every other tile position. Expanding the tiling here rather than inventing a
    /// display-list paint for it keeps the list flat: a backend never learns what a pattern is,
    /// and the result is resolution-independent because the cell is real geometry rather than a
    /// rendered image.
    pub(super) fn tile(
        &mut self,
        path: &Arc<Path>,
        transform: Transform,
        rule: FillRule,
        tiling: &Tiling,
        state: &GraphicsState,
    ) {
        /// Most cells one pattern fill may draw.
        ///
        /// A small cell over a large area is an enormous number of tiles, and the content
        /// stream inside each one is unbounded. This is the bound that keeps a pattern
        /// from becoming a decompression bomb with extra steps.
        ///
        /// **And it is the only bound on this loop, which the four-hundred-and-thirty-fifth
        /// session measured rather than assumed.** A cell's content runs through this same
        /// interpreter, so its operators count against [`MAX_OPERATIONS`] — but an *empty*
        /// cell executes no operator, and the trip count is then whatever `/XStep` and
        /// `/YStep` say. With this lifted to 4 194 304, a pattern whose cell is empty ran
        /// 1 000 000 tiles in **889 ms reporting nothing**: 0.89 µs a tile, and a `/XStep` of
        /// 0.001 over a 600-unit fill states 3.6 × 10¹¹ of them, which is four days.
        ///
        /// The 48 documents of 65 944 that reach it all *terminate* when it is lifted —
        /// 0.06 s to 14.2 s, wanting 4104 to 895 500 tiles — so the population is legitimate
        /// hatching rather than an attack, and 14 of the 48 want fewer than twice the bound.
        /// It is left at 4096 because **the count is the wrong quantity and a larger count is
        /// no safer**: `7680183.pdf` wants 42 282 tiles and takes 14.2 s while `2760154.pdf`
        /// wants 765 440 and takes 8.7. A bound on the *work* is what this should become, and
        /// `doc/todo/49` carries it. ADR 0271, `tests/hostile_budgets.rs`.
        ///
        /// **Since ADR 0430 the loop copies where it used to interpret**, so the work per site
        /// is the cell's *commands* rather than its operators — and
        /// [`Interpreter::repeat_cell`] charges each copy to [`MAX_OPERATIONS`], which is
        /// where the second bound now bites. This one still bounds the trip count, which is
        /// what an empty cell needs and what a bound on the work would replace.
        const MAX_TILES: usize = 4096;

        // The pattern is anchored to the page, so the question "which cells does this path
        // touch" has to be asked in the pattern's own coordinates.
        let Some(to_pattern) = tiling.to_page.invert() else {
            self.note(Unsupported::Shading {
                name: "a tiling pattern's matrix is degenerate".to_owned(),
            });
            return;
        };
        let path_to_pattern = transform.then(to_pattern);

        let Some(bounds) = bounds_of(path, path_to_pattern) else {
            return;
        };
        let ((first_column, last_column), (first_row, last_row)) = spans(tiling, bounds);

        let columns = last_column.saturating_sub(first_column).saturating_add(1);
        let rows = last_row.saturating_sub(first_row).saturating_add(1);
        let total = usize::try_from(columns)
            .unwrap_or(usize::MAX)
            .saturating_mul(usize::try_from(rows).unwrap_or(usize::MAX));
        if total > MAX_TILES {
            self.note(Unsupported::LimitReached { limit: "MAX_TILES" });
            return;
        }

        // The path clips every cell, so a tile that falls outside it contributes nothing.
        let clip = Clip {
            path: (**path).clone(),
            transform,
            fill_rule: rule,
            parent: state.clip,
        };
        let Ok(clip) = self.list.add_clip(clip) else {
            self.note(Unsupported::LimitReached { limit: "max_clips" });
            return;
        };
        let clip = Some(clip);

        // §11.6.7: "the pattern definition shall be treated as if it were implicitly enclosed
        // in a non-isolated transparency group: a non-knockout group for tiling patterns …
        // The definition shall not inherit the current values of the graphics state
        // parameters at the time it is evaluated; those parameters shall take effect only
        // when the resulting pattern is later used to paint an object." So every cell below
        // runs with the transparency parameters at their defaults — which is what
        // `GraphicsState::initial` gives it — and the state's own blend mode, alpha constant
        // and soft mask are applied *once*, to the finished tiling, by the group pushed after
        // the loop. NOTE 2 asks for exactly that shape: "[i]n a raster-based implementation of
        // tiling, it is advisable to treat all tiles as a single transparency group. This
        // avoids artifacts due to multiple marking of pixels along the boundaries between
        // adjacent tiles."
        //
        // Until the hundred-and-seventeenth session each cell inherited them instead, so an
        // `0.5 ca` under a pattern was applied per tile rather than to the pattern, and the
        // graphics state's soft mask reached nothing at all.
        let mark = self.list.command_count();

        // The one interpretation the whole tiling gets, at the first site the span reaches.
        // §8.7.3.1's cell "shall be replicated at fixed horizontal and vertical intervals", and
        // a replica is this cell's commands displaced: see [`pdf_render::Cell`] for what makes
        // the two the same picture, and ADR 0430 for what it saves. Until the
        // five-hundred-and-ninety-fifth session the content stream was run once per site, which
        // is what made a bomb inside a cell cost its decode four thousand times over.
        let at = pdf_render::Mark::of(&self.list);
        let offset = Transform::translate(
            tiling.step.0 * as_f32(first_column),
            tiling.step.1 * as_f32(first_row),
        );
        let box_clip = self.run_cell(tiling, offset.then(tiling.to_page), clip);
        // Table 74's box, and the marks it halves: both are settled on the cell itself, so
        // every site is a copy of the settled figure rather than a repetition of the question.
        if let Some(corners) = tiling.bbox {
            let plan = self.settle_cell_box(
                mark,
                corners,
                (offset, to_pattern),
                tiling.step,
                (box_clip, clip),
            );
            if !plan.is_empty() {
                self.fold_repeated_marks(mark, &plan, (box_clip, clip));
            }
        }

        // Taken after the box and the fold are settled, so that what every site copies is the
        // finished cell rather than the question.
        let cell = pdf_render::Cell::drawn(&self.list, at);
        self.repeat_cell(
            &cell,
            tiling,
            ((first_column, last_column), (first_row, last_row)),
        );

        // The state's transparency parameters, applied once to the finished tiling. Where
        // they are all at their defaults there is nothing for a group to do and §11.4.4's
        // NOTE 5 says so in as many words — "the effect of compositing objects as a group is
        // the same as that of compositing them separately (without grouping)" — so the
        // commands stay inline and no page pays a buffer for a pattern that composites
        // trivially, which is almost every patterned page in the corpus.
        let composites =
            state.fill_alpha < 1.0 || state.blend != BlendMode::Normal || state.soft_mask.is_some();
        if !composites {
            return;
        }
        let parts = self.list.split_off_commands(mark);
        if parts.is_empty() {
            return;
        }
        // §11.6.7 makes the implicit group *non-isolated*, and this one is isolated. Its own
        // NOTE 1 is what makes that exact wherever no element blends — "in the common case in
        // which the pattern consists entirely of objects painted with the Normal blend mode …
        // the results depend only on the colour, shape, and opacity of the pattern cell and
        // not on those of the backdrop" — and a cell that sets a blend mode of its own is the
        // case it is not, which is §11.4.4's report.
        // — and since ADR 0237 the display list can say the cell's own backdrop instead of
        // substituting §11.4.5's, on the three conditions `Command::Group`'s `isolated`
        // states. What is left to report is a cell composited under a blend mode of its own,
        // and a cell inside a knockout group, where the collapse those conditions rest on
        // does not hold.
        let isolated = self.inside_knockout
            || state.blend != BlendMode::Normal
            || !any_command(&parts, &command_blends);
        // §11.4.6's NOTE 6 reaches the implicit group too, because §11.6.7 makes the cell an
        // *element* of whatever paints it: a pattern painted inside a knockout group whose
        // initial backdrop is transparent has that backdrop rather than its immediate one, so
        // the isolated construction is the clause and there is no backdrop being excluded.
        if isolated && !self.transparent_initial_backdrop && any_command(&parts, &command_blends) {
            self.note(Unsupported::TransparencyGroup {
                detail: "non-isolated, and an element blends with the backdrop it excludes"
                    .to_owned(),
            });
        }
        self.list.push(Command::Group {
            commands: parts,
            alpha: state.fill_alpha,
            // The tiles carry the path's clip already; a second copy on the group would be
            // the same region resolved twice.
            clip: None,
            mask: state.soft_mask,
            blend: state.blend,
            isolated,
            knockout: false,
            // §8.7.3.1: a pattern cell's colours are resolved in the compositing already in
            // force, so the implicit group introduces no space of its own.
            blending: None,
        });
    }

    /// Paints a shading across the current clip, for the `sh` operator.
    ///
    /// `sh` covers the whole clipping region rather than a path, so the geometry drawn is
    /// the page itself and the clip does the shaping. Where the shading does not extend,
    /// it paints nothing, so the covered area is only ever as large as the shading says.
    pub(super) fn paint_shading(
        &mut self,
        name: &Name,
        resources: &Dictionary,
        state: &GraphicsState,
    ) {
        // `sh` marks the page and changes nothing else, so a hidden layer skips it whole —
        // including the report a shading we cannot build would otherwise make about a
        // shading that was never going to be drawn.
        if self.is_hidden() {
            return;
        }
        // The name's bytes do the finding (§7.3.5, `resources.rs`); the text says so afterwards.
        let label = String::from_utf8_lossy(name.as_bytes()).into_owned();
        let Some(object) = self.resource_entry(resources, "Shading", name) else {
            self.note(Unsupported::Shading {
                name: format!("/{label} is not in /Shading"),
            });
            return;
        };

        // `sh` is drawn in the current user space, unlike a pattern.
        //
        // Table 77's `/BBox` is read here rather than inside `shading::build`, because it is
        // stated "in the shading's target coordinate space" — which is the space the caller
        // paints into, not the shading's own — and because the clause makes it a *clip*
        // rather than a property of the gradient: "this bounding box shall be applied as a
        // temporary clipping boundary when the shading is painted, in addition to the
        // current clipping path and any other clipping boundaries in effect at that time".
        let clip = crate::shading::bbox_of(self.document, &object).map_or(state.clip, |corners| {
            self.rect_clip(corners, state.transform, state.clip)
                .or(state.clip)
        });
        match self.shadings.build(
            self.document,
            &object,
            resources,
            state.transform,
            state.smoothness,
            &self.compositing,
        ) {
            Ok(shading) => {
                // §8.7.4.5.2's domain, which for a type 1 shading is where it marks at all.
                let clip = self.domain_clip(&shading, clip);
                let mut path = Path::new();
                path.push(PathCommand::MoveTo(Point::new(0.0, 0.0)));
                path.push(PathCommand::LineTo(Point::new(self.page.width, 0.0)));
                path.push(PathCommand::LineTo(Point::new(
                    self.page.width,
                    self.page.height,
                )));
                path.push(PathCommand::LineTo(Point::new(0.0, self.page.height)));
                path.push(PathCommand::Close);

                self.list.push(Command::Fill {
                    path: Arc::new(path),
                    // The page rectangle is already in page space, so it needs no further
                    // transform; the shading carries its own.
                    transform: Transform::IDENTITY,
                    fill_rule: FillRule::NonZero,
                    // §11.6.4.4's non-stroking constant applies to `sh` as to any other
                    // non-stroking painting operation.
                    paint: Paint::Shading(shading_with_alpha(&Arc::new(shading), state.fill_alpha)),
                    clip,
                    mask: state.soft_mask,
                    blend: state.blend,
                });
            }
            Err(error) => self.note(Unsupported::Shading {
                name: format!("/{label}: {error}"),
            }),
        }
    }

    /// Resolves a pattern name, for `scn` in a `/Pattern` colour space.
    ///
    /// §8.7.3.2 makes the operand a name into Table 34's `/Pattern` subdictionary:
    ///
    /// > This name shall be the key of an entry in the Pattern subdictionary of the current
    /// > resource dictionary (see 7.8.3, "Resource dictionaries"), whose value shall be the
    /// > stream object representing the pattern.
    ///
    /// A name that finds nothing there leaves the paint at §8.6.8's initial value for a
    /// `Pattern` space — "a pattern object that causes nothing to be painted" — so every
    /// subsequent fill and stroke in that space marks the page with nothing. That is why the
    /// miss is reported rather than left to look like a producer's transparent figure.
    pub(super) fn pattern(
        &mut self,
        name: &Name,
        resources: &Dictionary,
        tint: &[f32],
        state: &GraphicsState,
        fill: bool,
    ) -> Option<PatternPaint> {
        let label = String::from_utf8_lossy(name.as_bytes()).into_owned();
        let Some(object) = self.resource(resources, "Pattern", name) else {
            self.note_missing_resource("Pattern", name, "is not in /Pattern");
            return None;
        };
        let dict = match &object {
            Object::Dictionary(dict) => dict.clone(),
            Object::Stream(stream) => stream.dict.clone(),
            // §8.7.3.2's "value shall be the stream object representing the pattern" for a
            // tiling one, Table 75's dictionary for a shading one — so a `/Pattern` entry that
            // is neither is a pattern with no definition, which is the same failure as a name
            // the subdictionary omits arriving one step later.
            _ => {
                self.note_missing_resource("Pattern", name, "is not a dictionary or a stream");
                return None;
            }
        };

        match self.document.get_key(&dict, "PatternType").as_integer() {
            Some(1) => {
                return self
                    .tiling(&label, &object, &dict, tint, state, fill)
                    .map(PatternPaint::Tiling);
            }
            Some(2) => {}
            other => {
                self.note(Unsupported::Shading {
                    name: format!("/{label} is pattern type {}", other.unwrap_or(0)),
                });
                return None;
            }
        }

        // A pattern is positioned relative to the page's default space, not to the
        // transform in force where it is used. Getting this wrong moves every gradient on
        // the page by whatever the current transform happened to be.
        let matrix = crate::shading::matrix_of(self.document, &dict, "Matrix");
        // Unresolved on purpose: `shading::Cache` is keyed by the reference, and a pattern
        // painted a thousand times states the same one every time.
        let shading_object = dict.get("Shading").cloned().unwrap_or(Object::Null);

        match self.shadings.build(
            self.document,
            &shading_object,
            resources,
            matrix.then(self.base),
            state.smoothness,
            &self.compositing,
        ) {
            Ok(shading) => Some(PatternPaint::Shading(
                Arc::new(shading),
                // Stated "in the shading's target coordinate space", which for a pattern is
                // the pattern space — the shading's own `/Matrix` (type 1 only) is applied
                // inside `build` and comes *after* this.
                crate::shading::bbox_of(self.document, &shading_object)
                    .map(|corners| (corners, matrix.then(self.base))),
            )),
            Err(error) => {
                self.note(Unsupported::Shading {
                    name: format!("/{label}: {error}"),
                });
                None
            }
        }
    }

    /// Reads a tiling pattern's cell and how it repeats.
    fn tiling(
        &mut self,
        name: &str,
        object: &Object,
        dict: &Dictionary,
        tint: &[f32],
        state: &GraphicsState,
        fill: bool,
    ) -> Option<Rc<Tiling>> {
        let stream = object.as_stream()?;
        // §8.7.3.1: "The appearance of the pattern cell shall be defined by a content stream
        // containing the painting operators needed to paint one instance of the cell." So the
        // cell is §7.8.2's sequence of instructions, a prefix of it is a cell with fewer marks
        // in the same places, and the tiling replicates that shorter cell at the file's own
        // `/XStep` and `/YStep` — nothing stands in place of the marks the damage took.
        // See [`Interpreter::content_stream`].
        //
        // A cell whose stream cannot be decoded at all is **reported**, which it was not before
        // the five-hundred-and-ninety-fifth session: the refusal was dropped here and the page
        // came back complete with the pattern silently unpainted. A form says
        // `undecodable form /Fx` in the same circumstance and always has, and this is the same
        // sentence for the other of §7.8.2's five.
        let Some(content) =
            self.content_stream(stream, &format!("a tiling pattern /{name} (§8.7.3.1)"))
        else {
            self.note(Unsupported::Operator {
                operator: format!("undecodable tiling pattern /{name}"),
            });
            return None;
        };

        // `/XStep` and `/YStep` may differ from the cell's bounding box, which is how a
        // pattern tiles with gaps or with overlap. Zero would mean an infinite number of
        // cells in one place, so the specification forbids it and so does this.
        let step_x = self
            .document
            .get_key(dict, "XStep")
            .as_number()
            .map_or(0.0, narrow);
        let step_y = self
            .document
            .get_key(dict, "YStep")
            .as_number()
            .map_or(0.0, narrow);
        let bbox = self.document.get_key(dict, "BBox");
        let bbox: Vec<f32> = bbox
            .as_array()
            .map(|items| {
                items
                    .iter()
                    .filter_map(|item| self.document.resolve(item).as_number().map(narrow))
                    .collect()
            })
            .unwrap_or_default();
        // A missing or zero step falls back to the cell's own size, which is what a
        // producer means by it and what other readers assume.
        let step = (
            non_zero(step_x).or_else(|| cell_extent(&bbox, 0))?,
            non_zero(step_y).or_else(|| cell_extent(&bbox, 1))?,
        );
        // Normalised to (left, bottom, right, top): the clause names the four edges in that
        // order and producers write the corners in any of them, exactly as they do for a page
        // box. A box with no extent in either direction clips everything away, and Table 74's
        // NOTE 1 says otherwise — "[a] BBox of zero height or width will still paint one
        // pixel" — so it is left unclipped rather than emptied.
        let cell_box = match bbox.as_slice() {
            [x0, y0, x1, y1] if (x1 - x0).abs() > 0.0 && (y1 - y0).abs() > 0.0 => {
                Some([x0.min(*x1), y0.min(*y1), x0.max(*x1), y0.max(*y1)])
            }
            _ => None,
        };

        let resources = self
            .document
            .get_key(dict, "Resources")
            .as_dict()
            .cloned()
            .unwrap_or_default();

        // `/PaintType 2` describes a stencil rather than a picture: the cell carries no
        // colour and the current colour is poured through it.
        let tint = match self.document.get_key(dict, "PaintType").as_integer() {
            Some(2) => {
                let space = if fill {
                    &state.fill_space
                } else {
                    &state.stroke_space
                };
                // A bare `/Pattern` names no underlying space, so the operand count is the
                // only evidence of what the colour is — the same fallback `scn` uses when
                // a declared space and its operands disagree.
                let space = match space {
                    ColourSpace::Pattern { base: None } => match tint.len() {
                        3 => ColourSpace::Rgb,
                        4 => ColourSpace::Cmyk,
                        _ => ColourSpace::Gray,
                    },
                    other => other.clone(),
                };
                // Through `convert`, so that an uncoloured cell painted inside a
                // `/Luminosity` mask group is poured through §11.5.3's luminosity like every
                // other colour there. `BlackPoint::Default`: the tint arrived with `scn` and
                // §8.6.5.9's setting belongs to the state that paints the cell.
                Some(convert(
                    &space,
                    tint,
                    BlackPoint::Default,
                    &self.compositing,
                ))
            }
            _ => None,
        };

        Some(Rc::new(Tiling {
            content,
            resources,
            step,
            bbox: cell_box,
            to_page: crate::shading::matrix_of(self.document, dict, "Matrix").then(self.base),
            tint,
        }))
    }
}

/// Where one site of a tiling sits relative to the site its cell was drawn at, in page space.
///
/// ISO 32000-2 §8.7.3.1 puts site (i, j) at i × `/XStep` and j × `/YStep` in *pattern* space, so
/// the displacement between two sites is that difference carried through the pattern matrix —
/// and through its linear part alone, because the matrix's own translation is common to both
/// sites and cancels. The result is therefore a pure translation of page space whatever the
/// pattern matrix rotates or shears, which is what [`pdf_render::Cell::repeat`] wants.
fn displacement(tiling: &Tiling, columns: i32, rows: i32) -> Transform {
    let to_page = tiling.to_page;
    let x = tiling.step.0 * as_f32(columns);
    let y = tiling.step.1 * as_f32(rows);
    Transform::translate(
        to_page.a.mul_add(x, to_page.c * y),
        to_page.b.mul_add(x, to_page.d * y),
    )
}

/// The bounding box of a path once transformed, as `(min_x, min_y, max_x, max_y)`.
fn bounds_of(path: &Path, transform: Transform) -> Option<(f32, f32, f32, f32)> {
    let mut bounds: Option<(f32, f32, f32, f32)> = None;
    let mut visit = |point: Point| {
        let at = transform.apply(point);
        if !at.x.is_finite() || !at.y.is_finite() {
            return;
        }
        bounds = Some(match bounds {
            None => (at.x, at.y, at.x, at.y),
            Some((x0, y0, x1, y1)) => (x0.min(at.x), y0.min(at.y), x1.max(at.x), y1.max(at.y)),
        });
    };
    for command in path.commands() {
        match command {
            PathCommand::MoveTo(point) | PathCommand::LineTo(point) => visit(*point),
            // A curve stays inside the hull of its control points, so those bound it —
            // loosely, which only ever draws tiles that turn out to be clipped away.
            PathCommand::CurveTo(a, b, c) => {
                visit(*a);
                visit(*b);
                visit(*c);
            }
            PathCommand::Close => {}
        }
    }
    bounds
}

/// Which tiles of a pattern the given bounds in pattern space touch, by column and by row.
///
/// Where the cell itself is matters, which is [`span`]'s subject. Table 74 makes `/BBox`
/// required, and a pattern that states none is tiled as though its cell began at the origin —
/// which is what this did for every pattern until the two-hundred-and-eighteenth session.
fn spans(tiling: &Tiling, bounds: (f32, f32, f32, f32)) -> ((i32, i32), (i32, i32)) {
    let cell = tiling
        .bbox
        .unwrap_or([0.0, 0.0, tiling.step.0, tiling.step.1]);
    (
        span(
            bounds.0,
            bounds.2,
            tiling.step.0,
            cell[0].min(cell[2]),
            cell[0].max(cell[2]),
        ),
        span(
            bounds.1,
            bounds.3,
            tiling.step.1,
            cell[1].min(cell[3]),
            cell[1].max(cell[3]),
        ),
    )
}

/// The range of tile indices covering an interval, given a step and where the cell itself sits.
///
/// §8.7.3.1 places the pattern cell where its content stream draws it and replicates that at
/// multiples of `/XStep` and `/YStep` — so the offsets needed to cover `low..high` are measured
/// from the **cell's own extent**, not from the pattern space's origin. Tile `k` covers
/// `cell + k × step`, so it is wanted when `cell_low + k × step <= high` and
/// `cell_high + k × step >= low`.
///
/// **This took `cell_low` and `cell_high` from the two-hundred-and-eighteenth session and did
/// not before**, which was invisible for as long as it was because Table 74's `/BBox` is nearly
/// always at the pattern's origin: the ±1 of slack `floor` and `ceil` give covers a cell within
/// one step of it. `issue13561_reduced.pdf` states `/BBox [35.4 396.6 287.4 588]` against a
/// `/YStep` of 191.4 — two steps out — and every tile landed two rows below the page.
fn span(low: f32, high: f32, step: f32, cell_low: f32, cell_high: f32) -> (i32, i32) {
    /// Bounds the index range so a huge path or a tiny step cannot overflow.
    const LIMIT: f32 = 1e6;

    let first = ((low - cell_high) / step).floor().clamp(-LIMIT, LIMIT);
    let last = ((high - cell_low) / step).ceil().clamp(-LIMIT, LIMIT);
    #[expect(
        clippy::cast_possible_truncation,
        reason = "both are clamped to a million, well inside i32"
    )]
    {
        (first as i32, last as i32)
    }
}

/// Widens a tile index for arithmetic in pattern space.
fn as_f32(index: i32) -> f32 {
    #[expect(
        clippy::cast_precision_loss,
        reason = "tile indices are clamped to a million, exact in f32"
    )]
    {
        index as f32
    }
}

/// Returns a step only if it is usable as one.
///
/// A zero step would place every cell on top of the last, which is an infinite loop rather
/// than a pattern; the specification forbids it. A negative one is legal and tiles in the
/// other direction, so only its magnitude matters here.
fn non_zero(step: f32) -> Option<f32> {
    let step = step.abs();
    (step.is_finite() && step > 0.0).then_some(step)
}

/// The width or height of a pattern cell's bounding box, as a fallback step.
fn cell_extent(bbox: &[f32], axis: usize) -> Option<f32> {
    let low = bbox.get(axis)?;
    let high = bbox.get(axis.checked_add(2)?)?;
    non_zero(high - low)
}

/// A shading with a constant alpha applied, sharing the original where it is opaque.
///
/// The share is the common case and the one worth keeping cheap: a pattern set once paints
/// every path filled until the colour changes again, and copying its 256-sample ramp — or a
/// mesh's triangles — per fill would be a copy per path for nothing.
pub(super) fn shading_with_alpha(shading: &Arc<Shading>, alpha: f32) -> Arc<Shading> {
    if alpha < 1.0 {
        Arc::new(shading.with_alpha(alpha))
    } else {
        Arc::clone(shading)
    }
}

//! §8.7.3.1: a cell's marks copied to every site the lattice reaches, folded where a backend can
//! repeat them, and held to the copy budget that bounds them (ADR 2081).

use std::sync::Arc;

use pdf_render::{
    BlendMode, ClipId, Color, Command, DisplayListError, Paint, Path, Point, Rect, SoftMask,
    SoftMaskId, SoftMaskKind, Stroke, Transform,
};

use crate::content::report::Unsupported;
use crate::content::{Interpreter, list_budget};

use super::lattice::{displacement, extent};
use super::reach::Reach;
use super::tiling::Tiling;

/// What one pattern cell's commands fold, by position within the cell.
///
/// Each entry names a command's offset from the cell's first and which of its subpaths are the
/// second statement of a mark another cell also makes (§8.7.3.1, §11.6.2 — ADR 0213). It is
/// decided on the one cell that is interpreted and every site inherits it by being a copy of
/// that cell, for the reason [`Interpreter::fold_repeated_marks`] gives.
pub(super) type CellFold = Vec<(usize, pdf_render::Repeats)>;

impl Interpreter<'_> {
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
    /// The fix is therefore neither a buffer nor a clip: it is to notice that the cell's two copies
    /// of the rule are **one mark of the tiling**, keep one of them, and draw it whole.
    /// [`pdf_render::repeated_subpaths`] carries the conditions under which that paints the same
    /// set of points, and refuses where it would not.
    pub(super) fn plan_repeated_marks(
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
    pub(super) fn fold_repeated_marks(
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
            self.draw(command);
        }
        if owed {
            self.note(Unsupported::Shading {
                name: "a tiling pattern's cells do not all state the same figure".to_owned(),
            });
        }
    }

    /// Most commands one tiling may copy, whatever its cell holds and whatever the fill spans.
    ///
    /// A site is a copy of the cell's commands (ADR 0430), and a copy costs what a command
    /// costs: about 225 bytes of display list and, on this machine, two to three microseconds
    /// of rasterisation — so a tiling's cost is the commands it copies, and that is the unit this
    /// bound is in. `MAX_TILES` bounded the same loop in *sites* until ADR 0810, at 4096
    /// whatever the cell held, and the unit was the whole of what was wrong with it: a fill of
    /// 4480 sites of a two-command cell (`PDFIUM-1122-0.pdf`, 8960 commands) was refused its
    /// top fifth while a cell of forty commands was afforded 163 840.
    ///
    /// **The value is a choice, and it is sixteen times the count at a one-command cell.** What
    /// it admits and refuses was measured before it was chosen (ADR 0810): every tiling of the
    /// crawl's `7803372.pdf` (21 320), `4650000.pdf` (17 384), `7680183.pdf` (7610 at most of
    /// 249) and the head above draws whole, and the two that do not are the two that want a
    /// hundred thousand sites or more — `2760154.pdf`'s 762 930 and `PDFIUM-1497-2.pdf`'s
    /// 448 632 at four commands apiece, 1.8 million commands for one operator, eleven seconds
    /// and nine tenths of a gibibyte for the page. Why not the page's whole budget: one
    /// operator's expansion would then starve every operator after it, which is what
    /// `PDFIUM-1497-2.pdf` did to its own frame and title block when this was tried, and a
    /// page of sixty such fills would cost eleven seconds where the count cost two.
    ///
    /// **What those two are cut of is measured, and it is small** (ADR 0828). Both arms in one
    /// sitting, `examples/open_one` at scale 1: `PDFIUM-1497-2.pdf` draws a **byte-identical**
    /// raster with the bound lifted, for 1.87 s against 10.53 and 0.19 GiB against 0.93;
    /// `2760154.pdf` is 0.33 s and 0.02 GiB against 2.08 s and 0.42, and its whole tiling is worth
    /// a mean of 1.087 of 255 — 33.583 of ink against 34.670, the pale wash behind its title.
    /// Eighty-one and ninety-four per cent of that gap is *rasterisation* rather than this list, so
    /// a display-list paint carrying a cell and its lattice for a backend to replicate as geometry
    /// would buy the memory and a fifth of the shorter gap. The sentence about a cell "evaluated
    /// once and then replicated" is §11.6.7's NOTE 1, which says it of the *opaque* imaging model;
    /// §8.7.3.1's NOTE 2 is about `/XStep` and `/YStep` differing from the `/BBox` (ADR 0827).
    /// `doc/todo/49` carries the closed item and `doc/checks/fixed-documents.toml` the two pages.
    const MAX_TILE_COPIES: usize = 65_536;

    /// Most edge tests one page may spend on `reach.rs`, proving which sites a fill can reach.
    ///
    /// The scan is a saving rather than a requirement, and a saving has to cost less than what
    /// it saves. A row costs [`super::reach::Reach::cost`] edge tests to ask about — every edge twice
    /// and every curve box once — and a row that reaches *no* site spends no copies, so nothing
    /// else in `repeat_cell` charges for it: a fill of a hundred thousand edges clustered at the
    /// foot of a lattice of a million rows would scan the other nine hundred thousand for
    /// nothing, at about four hundred billion tests. That is the same shape `MAX_TILES` was
    /// retired for — a loop whose trip count a file states and whose body a budget cannot see —
    /// one level in, so it is answered the same way: in the unit of the work.
    ///
    /// **Page-wide rather than per tiling**, because a page may state as many fills as its
    /// stream has room for and a per-tiling allowance would multiply by that. Four
    /// million tests is about forty milliseconds here, and **the value is fifteen times the
    /// heaviest page measured** (ADR 0810): `7680183.pdf`, 249 hatched polygons of a plan and
    /// the page every other figure in that ADR is worst on, spends **under 300 000** across all
    /// of them; `PDFIUM-1497-2.pdf` and `2760154.pdf` spend fewer than 5000 rows' worth apiece.
    ///
    /// Running out is not a refusal and is not reported: the caller stops asking and keeps
    /// every site, which is what it does for a stroke and what it did before `reach.rs` existed.
    /// The direction is the safe one — more sites, never fewer — and those sites are still
    /// bounded by [`Self::MAX_TILE_COPIES`] and by `MAX_LIST_BYTES`.
    pub(in crate::content) const MAX_REACH_SCAN: usize = 4_194_304;

    /// Draws every site of a tiling but the one its cell was interpreted at.
    ///
    /// §8.7.3.1: "The pattern cell shall be replicated at fixed horizontal and vertical
    /// intervals to fill the area to be painted" — so what each site needs is the cell's marks
    /// displaced, which is [`pdf_render::Cell::repeat`].
    ///
    /// # What bounds it, which is what bounded it before
    ///
    /// Each copy is charged, before it is made, to the tiling's own [`Self::MAX_TILE_COPIES`]
    /// and to the page's `MAX_LIST_BYTES` — the memory the copy keeps, every command and every
    /// clip in it. No count of operators is asked (ADR 1809): a copy is not an operator, and what a
    /// runaway replication can exhaust is the list, which the second budget measures in its own
    /// unit.
    ///
    /// **Three things bound the site count, and none of them is a count of sites** (ADR 0810).
    /// A charge per copy cannot see one case: a cell that draws *nothing* copies nothing, so a
    /// loop over it would run the trip count `/XStep` and `/YStep` state — 3.6 × 10¹¹ of them for
    /// a thousandth of a unit over a 600-unit fill, about four days at 0.89 µs a trip (ADR
    /// 0271). But a cell with no marks replicated any number of times is no marks:
    /// §8.7.3.1's replication has nothing to replicate, so the loop is not entered at all. Every
    /// other cell costs at least one command a site, charged to the page's budget above and to
    /// [`Self::MAX_TILE_COPIES`], the tiling's own — the same cost bounded twice, once for the
    /// page and once so that one operator's expansion cannot take the page's memory from the
    /// operators after it. And a site is only copied where the fill's interior can reach its cell at all
    /// (`reach.rs`), so a hatched wall costs its own area and not its hull's.
    ///
    /// The prefix a budget affords is whole rows, on the same argument ADR 0477 made for the
    /// constant: the sites are laid down row-major from the span's own corner, and a budget
    /// running out mid-row would put a ragged edge where the file states none. And a row is
    /// only the sites the fill's interior reaches — `reach` is that question, answered a row at
    /// a time — so a hatched wall costs its own area in copies and not its hull's (ADR 0810).
    ///
    /// **Asking that question is work too, and the page has a budget for it.** A row that
    /// reaches no site spends no copy, so a lattice of a million rows a fill barely touches
    /// would be scanned for nothing at no charge to either budget above — the same shape the
    /// retired count was retired for, one level in. [`Self::MAX_REACH_SCAN`] bounds the page's
    /// whole scan; past it the rows are taken whole, which loses the saving and nothing else.
    pub(super) fn repeat_cell(
        &mut self,
        cell: &pdf_render::Cell,
        tiling: &Tiling,
        spans: ((i32, i32), (i32, i32)),
        reach: Option<&Reach>,
    ) {
        // A cell with no marks replicated any number of times is no marks, and the trip count
        // a file may state is the one cost a copy of nothing never charges — so the loop is not
        // entered. This line is what retired `MAX_TILES` (ADR 0810).
        if cell.is_empty() {
            return;
        }
        let ((first_column, last_column), (first_row, last_row)) = spans;
        let columns = (first_column, last_column);
        let mut rows_laid = 0usize;
        let mut spent = 0usize;
        // What the next copy will charge to the list's bytes: the last copy's charge, clips
        // included, and before the first a command's own size for each of the cell's.
        let mut copy_bytes = cell.len().saturating_mul(list_budget::COMMAND_BYTES);
        for row in first_row..=last_row {
            // The sites of this row the fill's interior can reach at all (`reach.rs`); a
            // stroke's hull is taken whole, and so is every row once the page has spent its
            // scan (see [`Self::MAX_REACH_SCAN`] for why asking is itself charged).
            let sites = match reach {
                Some(reach) if self.reach_scanned < Self::MAX_REACH_SCAN => {
                    self.reach_scanned = self.reach_scanned.saturating_add(reach.cost());
                    reach.row(row, columns)
                }
                _ => vec![columns],
            };
            let count: usize = sites.iter().map(|&(a, b)| extent(a, b)).sum();

            // §8.7.3.1 puts the requirement on the processor rather than on the file: "[w]hen
            // performing painting operations such as S (stroke) or f (fill), the PDF processor
            // shall paint the cell on the current page as many times as necessary to fill an
            // area." A budget means some pages cannot have as many times as necessary — but
            // painting the cell *no* times is the furthest a processor can get from that
            // sentence, and the sites the budget does afford are the producer's own marks. So
            // the shortfall is reported and the affordable prefix is drawn, which is §7.8.2's
            // rule for a stream that decoded part-way (ADR 0343) applied to the second of the
            // two things a tiling is made of; the first — the cell's own content stream — has
            // drawn its prefix since ADR 0359. The prefix is whole rows, asked for row by row:
            // the sites are laid down row-major from the span's own corner, and a budget that
            // ran out mid-row would put a ragged edge where the file states none (ADR 0477).
            // Where one row alone is over budget the row is cut instead, by the check inside
            // the loop, since a prefix of one row is all there is to keep.
            let affordable = Self::MAX_TILE_COPIES
                .saturating_sub(spent)
                .checked_div(cell.len())
                .unwrap_or(0);
            if count > affordable && rows_laid > 0 {
                self.note(Unsupported::LimitReached {
                    limit: "MAX_TILE_COPIES",
                });
                return;
            }
            for (low, high) in sites {
                for column in low..=high {
                    if (column, row) == (first_column, first_row) {
                        continue;
                    }
                    let by = displacement(
                        tiling,
                        column.saturating_sub(first_column),
                        row.saturating_sub(first_row),
                    );
                    // Asked **before** the copy, not after it, and that is the whole of what
                    // keeps a nested tiling bounded. A cell that holds a tiling holds every one
                    // of its copies, so a chain of patterns each filling with the next is 9ⁿ
                    // commands — the span takes a neighbour on each side even for a fill inside
                    // one cell — and a check after the copy stopped only *this* loop: the parent
                    // tiling then copied a list already past the budget eight more times, and
                    // its parent that, so `ContentStreamCycleType3insideType3.pdf` cost 25 GB
                    // and a minute the day the nesting bound was raised past its cycle (ADR
                    // 0793). Refusing the copy that would cross the budget bounds the whole
                    // list at the budget plus one cell.
                    if spent.saturating_add(cell.len()) > Self::MAX_TILE_COPIES {
                        self.note(Unsupported::LimitReached {
                            limit: "MAX_TILE_COPIES",
                        });
                        return;
                    }
                    // The same question in bytes, asked before the copy for the same reason
                    // (ADR 1507): a copy also carries the cell's clips, which no count of
                    // commands sees.
                    if self.list_past(copy_bytes) {
                        return;
                    }
                    let before = self.list_budget.charged();
                    match cell.repeat(&mut self.list, by) {
                        Ok(copied) => {
                            self.charge_copied(copied);
                            if self.list_spent(0) {
                                return;
                            }
                            copy_bytes = self.list_budget.charged().saturating_sub(before);
                            spent = spent.saturating_add(copied);
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
            rows_laid = rows_laid.saturating_add(1);
        }
    }

    /// A stroke's own region, as the soft mask [`super::Tiled::Stroke`] cuts its tiles to.
    ///
    /// ISO 32000-2 §11.6.4.2 states an object's shape as 1.0 inside and 0.0 outside the path
    /// for a mark painted in a uniform colour, and then says what a pattern does to it:
    ///
    /// > For objects painted with a tiling pattern (8.7.3, "Tiling patterns") or a shading
    /// > pattern (8.7.4, "Shading patterns"), the shape shall be further constrained by the
    /// > objects that define the pattern (see 11.6.7, "Patterns and transparency").
    ///
    /// The clause's two factors are the two this construction multiplies: the mark's own shape,
    /// which is this mask, and the pattern's objects, which are the tiles it is put on.
    /// §11.5.2 states the first of the two derivations a soft mask has:
    ///
    /// > The mask value at each point shall then be derived from the alpha of the group.
    ///
    /// So a group whose one element is this stroke, taken for its alpha, is this stroke's
    /// shape — including whatever coverage a rasteriser gives its anti-aliased edge, which is
    /// the same quantity it would have given the stroke itself. §11.6.5.1 says the colour is
    /// irrelevant — "\[t]he colours of the constituent objects shall be ignored" — so the
    /// element is painted opaque white and nothing reads it.
    ///
    /// The mask is recorded as a shape ([`crate::image::ShapeMasks`]), because that is what it
    /// is: a knockout element's shape keeps it where §11.6.4.3's opacity would be taken off,
    /// and taking it off left the element the shape of the whole cell (ADR 1306).
    ///
    /// `None` where the display list can hold no further mask, which the caller reports.
    pub(super) fn stroke_shape(
        &mut self,
        path: &Arc<Path>,
        transform: Transform,
        stroke: &Stroke,
    ) -> Option<SoftMaskId> {
        let mask = SoftMask {
            commands: vec![Command::Stroke {
                path: Arc::clone(path),
                transform,
                stroke: stroke.clone(),
                paint: Paint::Solid(Color::WHITE),
                // The clip belongs to the tiles rather than to the shape: intersecting it
                // here as well would resolve one region twice, which is what the group below
                // this mask already declines to do.
                clip: None,
                mask: None,
                blend: BlendMode::Normal,
            }],
            kind: SoftMaskKind::Alpha,
            transfer: None,
            luminance: None,
            black: None,
        };
        let id = self.list.add_soft_mask(mask).ok()?;
        self.image_masks.shape_masks_mut().record(id);
        Some(id)
    }
}

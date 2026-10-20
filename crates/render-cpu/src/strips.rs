//! The strips a target is cut into, and the planner that cuts it — ADRs 0139, 0219 and 1882.
//!
//! A strip is a run of whole rows drawn on its own thread from its own replay of the list. The
//! cut falls only on a row no mark forbids, so the strips together are the page drawn in one piece
//! to the byte (ADR 0139); the planner reads the list on the pool and decides the cut on one
//! thread (ADR 1882).

use rayon::iter::{IntoParallelIterator as _, ParallelIterator as _};
use std::collections::HashMap;
use std::sync::Arc;

use pdf_render::{DisplayList, Selection, TargetSpec};

use crate::mask_cache::{MASK_BUDGET, MaskCache};
use crate::soft_mask::mask_consumer_reach;
use crate::stroke::outlined_whole;
use crate::surface::{Band, Surface};
use crate::{CpuRasterError, CpuRasterizer, area, glimpse, scan};

/// Most horizontal strips one page is divided into, whatever the machine offers.
///
/// A strip carries its own mask cache, its own group buffers and its own soft masks, so the
/// per-strip constants are what bound this rather than the thread count: sixteen strips of a
/// 842-row page are 52 rows apiece, which is shorter than many clip bands. The cores beyond it
/// are not idle in a viewer — pages are rendered on their own threads above this one.
///
/// Public for a gate whose clock is this backend's as it ships: on a pool at least this wide —
/// the global pool of any machine of at least this many CPUs — [`CpuRasterizer::new`] asks for
/// exactly this count, so a gate that states it with [`CpuRasterizer::with_strips`] draws what that
/// machine draws and no longer depends on the CPUs its own process was given (ADRs 1742, 1859).
pub const MAX_STRIPS: u32 = 16;

/// Most work a split may replay, as a multiple of the command list itself.
///
/// See [`pdf_render::replay_ratio`]: a command reaching two strips is built, bounded and
/// pipelined twice, and on a page of a few page-wide commands that duplication is the whole
/// render. A quarter more is what a page of small marks costs and a page of page-wide ones
/// cannot reach.
const MAX_REPLAY: f64 = 1.25;

/// Rows a per-pixel pass over the finished page hands one thread at a time.
///
/// Large enough that a page under this many rows is done on one thread, which is where the
/// scheduling would cost more than the work.
pub(crate) const PIXEL_PASS_ROWS: usize = 64;

/// Fewest rows a strip may have, below which the target is drawn serially.
///
/// A strip pays for a mask cache and a replay of the command list, and a page thumbnail is not
/// worth either. It is also the floor the cost-balanced cut may not go under, which is what holds
/// the owner's witness at a 1 000-row window to ten strips on twelve threads and on twenty-four;
/// half of it was measured and not taken (ADR 1915).
pub(crate) const MIN_STRIP_ROWS: u32 = 64;

impl CpuRasterizer {
    /// ISO 32000-2 §10.8.3's spot planes drawn, one raster each, for steps b) to d) to multiply
    /// in (`pdf_render::separation`, ADRs 1317 and 1329). Each plane is a page of its own for
    /// `check_group_blit`'s budget.
    ///
    /// # Errors
    ///
    /// As [`CpuRasterizer::encode`], and an allocation refused.
    pub(crate) fn spot_planes(
        &self,
        separation: &pdf_render::SpotSeparation,
        surface: Surface,
    ) -> Result<Vec<tiny_skia::Pixmap>, CpuRasterError> {
        let target = surface.shown.target(surface.page);
        let mut spots = Vec::with_capacity(separation.planes().len());
        for plane in separation.planes() {
            pdf_render::check_group_blit(plane, target)?;
            let mut raster = tiny_skia::Pixmap::new(target.width, target.height).ok_or(
                CpuRasterError::Allocation {
                    width: target.width,
                    height: target.height,
                },
            )?;
            self.encode_in_strips(&mut raster, plane, surface)?;
            spots.push(raster);
        }
        Ok(spots)
    }

    /// Draws the whole list into `pixmap`, in parallel where the page permits it exactly.
    ///
    /// # Why a strip is not simply a band
    ///
    /// [`Band`] already restricts a command to the rows its clip admits, and cutting the page
    /// into runs of rows and replaying the list into each is the same geometry one level up
    /// (ADR 0137). What makes it a decision rather than a refactor is that **it is only exact
    /// at some rows**: rasterising a path into a surface that does not contain it chops the
    /// path against the surface's edge, and a *curve* chopped at an edge is re-parameterised,
    /// so its coverage differs from the unclipped curve's by up to a quarter of a channel.
    /// ADR 0138 shipped nothing for that reason — four oracle pages stopped agreeing with the
    /// reference consensus — and ADR 0139 is the probe that says exactly where the difference
    /// is: a cut at a row no curve crosses is **bit-identical** to the serial render, and one
    /// at a row a curve crosses is not.
    ///
    /// So [`pdf_render::unsplittable_rows`] names the rows a curve crosses and
    /// [`pdf_render::strip_boundaries_avoiding`] cuts only at the others. A page that grants
    /// none — one wide gradient under one curved clip, `bug1721218_reduced.pdf` — is drawn
    /// serially, which is not a fallback but the only division of it that draws the same page.
    ///
    /// That rule is about *geometry*, and it was not the whole of it: the *arithmetic* had to
    /// be the page's too, which is [`Surface`] and ADR 0219.
    ///
    /// # Errors
    ///
    /// As [`CpuRasterizer::encode`]. A strip that fails takes the render with it: half a page
    /// is not a result.
    pub(crate) fn encode_in_strips(
        &self,
        pixmap: &mut tiny_skia::Pixmap,
        list: &DisplayList,
        whole: Surface,
    ) -> Result<(), CpuRasterError> {
        self.encode_in_strips_watched(pixmap, Selection::whole(list), whole, None)
    }

    /// [`Self::encode_in_strips`] of a selection of a list's commands — the page's, or a window's
    /// of it (ADR 1980) — with each strip answering `showing`'s glances for its own rows where a
    /// caller is looking: the page itself, and never a plane drawn beside it.
    ///
    /// `whole` is what the rasterisation shows — the page, or a window of it on the page's own grid
    /// (ADR 2132) — and the cuts are planned on the target that is to a caller, in its own rows.
    pub(crate) fn encode_in_strips_watched(
        &self,
        pixmap: &mut tiny_skia::Pixmap,
        drawn: Selection<'_>,
        whole: Surface,
        showing: Option<&glimpse::Showing>,
    ) -> Result<(), CpuRasterError> {
        let list = drawn.list();
        let (page, target) = (whole.page, whole.shown.target(whole.page));
        let boundaries = self.selection_plan(drawn, target);
        let strips = boundaries.len().saturating_sub(1);
        // Computed once per rasterisation, before any mask is built, so that every build —
        // serial, in a strip, or nested inside another mask's — restricts against the same
        // complete answer. A list with no soft masks skips even the walk.
        let reach = if list.soft_mask_count() == 0 {
            Arc::new(HashMap::new())
        } else {
            Arc::new(mask_consumer_reach(drawn, page.transform))
        };
        // Measured once before any strip is queued — by the planner where it divides the page,
        // which reads them (ADR 1882), and here where it does not — so that every strip asks the
        // same bounds and none measures them again (ADR 1841); kept with the list, so a scroll at
        // this magnification measures nothing at all.
        let bounds = list.mark_bounds(page.transform);
        if strips < 2 {
            let surface = whole;
            let mut masks = MaskCache::new(surface, self.anti_alias, MASK_BUDGET).with_reach(reach);
            return self.encode_page(
                &mut pixmap.as_mut(),
                (drawn, &bounds, 0),
                surface,
                &mut masks,
                showing,
            );
        }

        // A strip is a replay of the whole list, so a deeply reduced image is reduced once per
        // strip — work that is per *source* sample and that `pdf_render::replay_ratio` cannot
        // see, because it bounds a replay by the rows a command covers. Reduced here, on this
        // thread, before any strip is queued: the strips then all hit, and this is the one place
        // the reduction may use rayon without being re-entered by a strip (ADR 0731).
        self.reduced_images
            .warm(drawn.commands(), target.transform, whole_target(target));
        // The budget is divided rather than multiplied: the masks of a strip are a strip tall,
        // so the same total memory buys the same coverage of the page it did serially.
        let budget = MASK_BUDGET.checked_div(strips).unwrap_or(MASK_BUDGET);
        let width = pixmap.width();
        let stride = (width as usize).saturating_mul(4);
        let mut rest = pixmap.data_mut();
        let mut pieces = Vec::with_capacity(strips);
        for pair in boundaries.windows(2) {
            let (top, bottom) = (pair.first().copied().unwrap_or(0), pair.get(1).copied());
            let Some(bottom) = bottom else { break };
            let rows = bottom.saturating_sub(top);
            let (piece, tail) =
                rest.split_at_mut((rows as usize).saturating_mul(stride).min(rest.len()));
            rest = tail;
            pieces.push((top, rows, piece));
        }

        pieces
            .into_par_iter()
            .try_for_each(|(top, rows, piece)| -> Result<(), CpuRasterError> {
                // The page's own target travels into the strip, not a shifted copy of it:
                // where the strip starts is [`Surface`]'s other field, and it reaches the
                // geometry once, last. ADR 0219 is what a shifted copy cost.
                let surface = Surface {
                    rows: Band {
                        top: whole.rows.top.saturating_add(top),
                        height: rows,
                    },
                    ..whole
                };
                let mut piece = tiny_skia::PixmapMut::from_bytes(piece, width, rows).ok_or(
                    CpuRasterError::Allocation {
                        width,
                        height: rows,
                    },
                )?;
                let mut masks =
                    MaskCache::new(surface, self.anti_alias, budget).with_reach(Arc::clone(&reach));
                self.encode_page(
                    &mut piece,
                    (drawn, &bounds, 0),
                    surface,
                    &mut masks,
                    showing,
                )
            })
    }
}

/// The whole target, as a rectangle in its own pixel space.
///
/// One caller — [`reduced_images::ReducedImages::warm`], which asks whether a command marks the raster at
/// all before paying for its reduction, and which is not the strip loop's own test because it runs
/// before there are strips.
///
/// [`reduced_images::ReducedImages::warm`]: crate::reduced_images::ReducedImages::warm
#[expect(
    clippy::cast_precision_loss,
    reason = "`rasterize` refuses a target beyond MAX_EXTENT = 2^24, which is the largest \
              integer f32 represents exactly"
)]
pub(crate) fn whole_target(target: TargetSpec) -> pdf_render::Rect {
    pdf_render::Rect::from_corners(
        pdf_render::Point::new(0.0, 0.0),
        pdf_render::Point::new(target.width as f32, target.height as f32),
    )
}

/// The rows at which to cut this target, or one strip's worth if it may not be cut.
///
/// The strip count asked for is the width of the rayon pool the strips run on, bounded by
/// [`MAX_STRIPS`] and by [`MIN_STRIP_ROWS`]; what comes back may be fewer, because a cut is only
/// made at a row no curve crosses — where a stroke is not one this backend draws a row at a time,
/// its whole extent ([`drawn_row_by_row`], ADRs 1819 and 1837). **The picture is very nearly independent of the answer**,
/// which is the property ADR 0139 exists to establish and `strip_parallelism.rs` asserts: over the
/// tracked corpus's 967 first pages, 794 draw the same bytes at one strip and at every count from
/// two to sixteen, and the page that moves most moves 0.36% of its pixels (ADRs 1758, 1819).
///
/// **"Very nearly" is measured, and both halves are worth carrying.** A confined process draws a
/// page in one strip, because it may not ask how many cores it has (ADR 0218), so one strip and
/// every division above one have to agree — `doc/PDF20_AN001-BPC.pdf` page 1 is the witness. A
/// strip's offset is applied *after* a mark's own transform is composed with the page's, because
/// folding it in first makes the sum round at another magnitude (ADR 0219), and that page is exact
/// — see [`ToDevice`](crate::surface::ToDevice).
///
/// What survives is a dependency's. `tiny-skia` maps a point as `y·sy + ty`, and shifting `ty` by
/// a whole number of rows moves the sum into another binade; ADR 0219 says why no arrangement of
/// this crate's arithmetic closes it, and ADR 1758 measures what is left over the corpus: one level
/// for one coverage, and up to three where a pixel takes several rounded results or a blend
/// function steeper than one — `raster_golden.rs` holds every first page to it.
///
/// **The list is read on the pool, and the cut is decided on this thread.** Every number the
/// decision reads — each command's extent, each row's cost, the forbidden rows, the replay of a
/// division — is [`pdf_render::strip_inputs`]'s, which is the serial functions' answer bit for bit,
/// so the cuts are the ones one thread would choose and no pixel moves. On the owner's witness at
/// six and seven times the fit that took the planner from 231–351 ms to 39–48 ms on twenty-four
/// threads, and from 206–359 ms to 55–102 ms on four (ADR 1882).
pub(crate) fn plan_strips(
    list: Selection<'_>,
    target: TargetSpec,
    asked: Option<u32>,
    anti_alias: bool,
) -> Vec<u32> {
    // Where the caller did not say, as many as the pool the strips run on is wide: a strip more
    // than it can run at once is a replay of every command that reaches it, bought for a balance
    // work stealing gives only an uneven division (ADR 1859). Read from the pool rather than the
    // machine because a caller may draw inside a narrower one — `RAYON_NUM_THREADS`, or a pool it
    // installed. The global pool's first use asks `available_parallelism`, which reads
    // `/proc/self/cgroup` on Linux, and a caller drawing inside a confinement with no filesystem is
    // *killed* for that rather than told no; such a caller states the number with `with_strips`,
    // and this is then not asked at all (ADR 0218).
    let wanted = asked
        .unwrap_or_else(|| {
            rayon::current_num_threads()
                .try_into()
                .unwrap_or(MAX_STRIPS)
        })
        .min(MAX_STRIPS)
        .min(target.height.checked_div(MIN_STRIP_ROWS).unwrap_or(0));
    if wanted < 2 {
        return vec![0, target.height];
    }
    // Read on the pool the strips are about to run on, every number the serial reading's, so
    // the cuts are the ones a single thread would choose (`pdf_render::StripInputs`). The bounds
    // are the ones the draw measures anyway (ADR 1841), measured here first so that the reading
    // takes each command's from them rather than measuring it twice more (ADR 1882).
    let bounds = list.list().mark_bounds(target.transform);
    let inputs = pdf_render::strip_inputs(list, target, &bounds, &|mark| {
        drawn_row_by_row(mark, anti_alias, target)
    });
    let costs = inputs.costs();
    let unsplittable = inputs.unsplittable();
    // ADR 0139's rows for every fill beside the backend's answer for strokes: the division this
    // backend made before it asked fills at all.
    let between_fills = inputs.between_fills();

    // Asked for from the most strips downwards, keeping a division only where its replay is
    // affordable. A strip pays again for every command that reaches it — see
    // `pdf_render::replay_ratio` — and on a page of a few page-wide commands that is the whole
    // cost: `issue12841_reduced.pdf` is two of them and was 105 ms serially against 166 ms in
    // sixteen strips. The bound is a quarter more work than the list itself, which every text
    // page measured in ADR 0137 sits far inside (1.01 to 1.13 at eight strips) and a page of
    // page-wide commands cannot reach at any count above one.
    //
    // **A row a fill no longer forbids is a permission, not a place a cut must go.** The split
    // balances an estimate and is blind to what a cut crosses: freed, the cuts of a page of a few
    // large curved fills run through them and replay them, and the bound then refused every count
    // down to one — `shading_extend.pdf` went from sixteen strips to one at 150 dpi; and the
    // estimate's balance can make fewer strips than the rows between the fills afford, which drew
    // the owner's drawing in 2.25 s on eight strips against 2.11 s on nine. So each count is asked
    // of both rows, and the division kept is the one of most strips either affords within the
    // bound. No smaller count can make more strips than it asks for, so the search stops once the
    // best division found has as many. ADR 1837.
    let affordable = |division: &[u32]| inputs.replay_ratio(division) <= MAX_REPLAY;
    let mut best: Option<Vec<u32>> = None;
    let mut count = wanted;
    while count > 1 && strips_in(best.as_deref()) < count {
        for rows in [&unsplittable, &between_fills] {
            let candidate =
                pdf_render::strip_boundaries_avoiding(&costs, rows, count, MIN_STRIP_ROWS);
            if strips_in(Some(&candidate)) > strips_in(best.as_deref()) && affordable(&candidate) {
                best = Some(candidate);
            }
        }
        count = count.checked_div(2).unwrap_or(0);
    }
    best.unwrap_or_else(|| vec![0, target.height])
}

/// How many strips a division's boundaries make, and none for no division.
fn strips_in(boundaries: Option<&[u32]>) -> u32 {
    boundaries.map_or(0, |cuts| {
        u32::try_from(cuts.len().saturating_sub(1)).unwrap_or(u32::MAX)
    })
}

/// Whether this backend draws `mark` a row at a time, so that a strip cut anywhere across it
/// draws the bytes the whole page does — the question [`pdf_render::unsplittable_rows_where`]
/// leaves to a backend (ADRs 1819, 1837).
///
/// It does where the mark reaches [`area`]: that converter evaluates every row's crossings from
/// the path's own device coordinates, so a row of a strip and the same row of the page are the
/// same two evaluations, a curve's chords included, and nothing is clipped against the strip's
/// edge to be re-stated. What is left is ADR 0219's shifted origin, which every mark in every strip
/// but the first already carries whether or not a cut crosses it.
///
/// Every condition below is a road [`CpuRasterizer::draw_stroke`] or [`CpuRasterizer::draw_fill`]
/// can take *away* from that converter, asked of the page rather than of a strip so that one answer
/// holds for every division. Both kinds of mark share three:
///
/// - **Anti-aliasing withdrawn, or a knockout element**: [`carries_coverage_as_alpha`] is false
///   and `scan` hands the shape to `tiny-skia`'s converter, which clips it. A half of a shaped
///   pair and a mark in a soft mask's group are counted with them because the planner does not
///   follow how they are composed.
/// - **An extent [`area`]'s range cannot hold in every strip**: past `scan::MEASURED_LIMIT` a mark
///   is drawn by `tiny-skia`'s converter without anti-aliasing, which clips it, and a strip moves
///   every coordinate up by its first row, so the page's extent has to fit both before and after any
///   such move. The library's own range, `scan::SUPERSAMPLED_LIMIT`, is not asked: inside the
///   measured one a mark reaches [`area`] on either side of 8191, because that converter states
///   every coordinate against the mark's own region (ADR 2058), so the marks of a page wider or
///   taller than the library's range forbid no row for it (ADR 2068).
/// - **No extent at all**, which states nothing a range can be asked of.
///
/// A fill has one of its own, and it is about the measurement rather than the road:
///
/// - **A span too large to be measured in one accumulator**: [`area`] suspects a mark of
///   overlapping itself as a whole, from every row of it, and settles every row as the filled set
///   where any row shows it. It asks the rows a surface clamps away as well, so the answer is the
///   mark's in every division — but only where the whole span fits the cell budget; past it each
///   run of rows is suspected alone (ADR 1819 §5), which a cut would change. The accumulator is as
///   wide as the mark's own columns, which a strip of whole rows never changes, so the span is
///   asked at those columns rather than at the page's width (ADR 1847). A stroke's outline is
///   suspected always, so its rows never depend on the span.
///
/// A stroke has one exception and two more conditions, and a fill none of them:
///
/// - **A thin segment is free whatever its width**: [`pdf_render::thin_segment()`]'s marks never
///   reach a converter, and `thin_segment::draw` admits and measures each on the page's own grid,
///   moving only its rows to the surface — so it asks under the transform this question is asked
///   under, and every division draws the same bytes for it (ADR 1847, trap 146).
/// - **A curve that may fold**: one bending tighter than its half-width is stroked as convex pieces
///   (ADR 1348), which `scan::mask_fill` leaves to `tiny-skia`, so a curved stroke is asked the
///   draw's own question of it ([`outlined_whole`], ADR 1847); one that cannot fold is the
///   stroker's outline, and the round caps and joins a stroker puts on any path reach [`area`] as
///   every outline does. A *filled* curve is never pieces: it reaches [`area`] as a glyph does,
///   flattened there.
/// - **A width under one level of a pixel**, a thin segment's apart: §10.7.4's substitutions draw
///   it, through `tiny-skia`'s hairline or a widened band ([`pdf_render::unmeasurable_width`], ADR
///   1102). A fill's own
///   substitutions — §10.7.4's sub-pixel bands and the marks of a collapsed subpath — are shapes
///   drawn by the same `scan::fill` and so by [`area`].
///
/// What it does not need to ask is the region's size: [`area`] measures a tall region a run of
/// rows at a time, so no strip's height decides which converter draws a mark. Nor the paint: a
/// shading drawn as a raster is still put through its shape by `scan::fill`, and a paint is read
/// at absolute device positions, which is why ADR 0139's rule never asked about it either — an
/// unfiltered image's samples are chosen at the page's own pixel centres for that reason (ADR
/// 2100), and a filtered one carries ADR 0219's shifted origin as every mark below a cut does.
///
/// [`carries_coverage_as_alpha`]: crate::compose::carries_coverage_as_alpha
pub(crate) fn drawn_row_by_row(
    mark: pdf_render::PlannedMark<'_>,
    anti_alias: bool,
    target: TargetSpec,
) -> bool {
    // A thin segment never reaches a converter: `thin_segment::draw` admits and measures it on the
    // page's own grid and moves only its rows to the surface, so every division draws the same
    // bytes for it. Asked under the transform and the composition the draw asks under — the
    // page's, and painting over — so that one answer is the draw's in every strip (ADR 1847, trap
    // 146). Asked second, of the strokes the converter's question refuses, because the first is
    // cheaper and answers almost every stroke of a page: asked first, it added a third to the
    // owner's drawing's planning.
    converted_row_by_row(mark, anti_alias, target)
        || matches!(mark, pdf_render::PlannedMark::Stroke(stroke)
            if anti_alias
                && !stroke.replaces
                && pdf_render::thin_segment(
                    stroke.path,
                    stroke.stroke,
                    stroke.paint,
                    stroke.blend,
                    stroke.at,
                )
                .is_some())
}

/// [`drawn_row_by_row`]'s answer for a mark that reaches [`area`], the conditions its comment lists
/// but the first.
fn converted_row_by_row(
    mark: pdf_render::PlannedMark<'_>,
    anti_alias: bool,
    target: TargetSpec,
) -> bool {
    let (replaces, extent) = match mark {
        pdf_render::PlannedMark::Fill(fill) => {
            // The span `area` takes a fill's suspicion of overlap over: the path's own box
            // carried onto the device, as its region is, with two rows of margin for the
            // rounding of a box mapped twice — and as wide as that region, which is the mark's own
            // columns on the page in every division, since a strip is a run of whole rows. Two
            // columns of margin for the same rounding, and never wider than the page, which
            // clamps the region (ADR 1847).
            let span = fill.path.hull().map(|hull| hull.mapped(fill.at)).map(|on| {
                let columns = (on.max.x.ceil() - on.min.x.floor() + 4.0).max(0.0);
                (on.max.y.ceil() - on.min.y.floor() + 4.0, columns)
            });
            #[expect(
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                reason = "a non-negative column count, saturated by the cast and then clamped to \
                          the page's own width"
            )]
            let fits = span.is_some_and(|(rows, columns)| {
                let columns = (columns as u32).min(target.width);
                area::whole_span_fits(rows, columns)
            });
            if !fits {
                return false;
            }
            (fill.replaces, fill.extent)
        }
        pdf_render::PlannedMark::Stroke(stroke) => {
            if stroke
                .path
                .commands()
                .iter()
                .any(|segment| matches!(segment, pdf_render::PathCommand::CurveTo(..)))
                && !outlined_whole(&stroke)
            {
                return false;
            }
            let width = stroke.stroke.device_width(stroke.at);
            if pdf_render::unmeasurable_width(stroke.at).is_none_or(|floor| width <= floor) {
                return false;
            }
            (stroke.replaces, stroke.extent)
        }
    };
    if !anti_alias || replaces {
        return false;
    }
    let Some(extent) = extent else {
        return false;
    };
    // A row of margin beyond the bound, which already counts a mitre at its limit.
    let limit = scan::MEASURED_LIMIT - 1.0;
    #[expect(
        clippy::cast_precision_loss,
        reason = "`rasterize` refuses a target beyond MAX_EXTENT = 2^24 rows, exact in f32"
    )]
    let rows = target.height as f32;
    extent.min.x >= -limit
        && extent.max.x <= limit
        && extent.min.y - rows >= -limit
        && extent.max.y <= limit
}

//! Where a mark meets the residue of its clip chain: the one site, for the walk's own tiles
//! and for the ones the fan-out made (ADR 1395).
//!
//! ISO 32000-2 §10.7.4:
//!
//! > Subsequent painting operations shall affect a region that is the intersection of the
//! > set of pixels defined by the clipping region with the set of pixels for the region to
//! > be painted.
//!
//! A pixel one set holds whole or misses whole meets the other by `min` exactly. A pixel
//! both sets cut holds the area of their intersection, which two bytes do not determine:
//! there the area is computed from both sets' edges ([`raster::area_in_pixel`], ADR 1467).
//! Both inputs reach this one function on every path — the mark's polylines from the walk
//! or from the job that made them, the chain's from the frame's cache — so what a frame
//! draws does not depend on how many threads drew it.

use std::sync::Arc;

use super::Encoder;
use super::clips::ResolvedClip;
use crate::error::RenderError;
use raster_scene::{Point, Rect};

use crate::raster::{self, CoverageMask, MeetWork, Polyline, RowEdges, Rule};

mod deferred;
mod helpers;
mod kept;

pub(in crate::encode) use deferred::ExactMeet;
pub(in crate::encode) use helpers::Helpers;
pub(crate) use kept::KeptMeets;
pub(in crate::encode) use kept::KeptRegion;

/// The mark a residue meets: the polylines its coverage was filled from, the rule, and the
/// mark's edges bucketed over its tile's rows where the job that filled it built them.
///
/// **The edges are built beside the fill, on the fan-out's threads, where a job's draw
/// carries a residue** (ADR 1513). The meet reads a mark's edges in the rows where both sets
/// cut a pixel, and building them was most of what the walk's thread spent on a meet. A row's
/// bucket holds every edge that reaches the row whichever rows were built (ADR 1467), so
/// edges over the whole tile meet each of those rows to the same bytes as edges built over
/// those rows alone; where the whole tile's edges pass the bound, `edges` is `None` and the
/// meet builds over its own rows, which the bound may still admit.
#[derive(Debug, Clone, Copy)]
pub(super) struct Mark<'p> {
    pub(super) polylines: &'p [Polyline],
    pub(super) rule: Rule,
    pub(super) edges: Option<&'p RowEdges>,
}

/// A mark's inputs as a meet is handed them: borrowed from the walk, which keeps them, or owned,
/// as a fan-out job's product is — a meet whose exact pixels are made later takes owned inputs
/// rather than copies them (ADR 1541). A job's edges over its whole tile are up to four times
/// the tile's own bound (`JOB_EDGE_REACH`), and copied for each of the stroked Type 3 page's
/// meets they cost its turn about a millisecond.
#[derive(Debug)]
pub(super) enum MarkInputs<'p> {
    /// Inputs the caller keeps.
    Borrowed(Mark<'p>),
    /// Inputs the caller hands over: the polylines, the rule and the job's edges.
    Owned(Vec<Polyline>, Rule, Option<Box<RowEdges>>),
}

impl MarkInputs<'_> {
    /// The mark these inputs describe, borrowed.
    pub(super) fn mark(&self) -> Mark<'_> {
        match self {
            Self::Borrowed(mark) => *mark,
            Self::Owned(polylines, rule, edges) => Mark {
                polylines,
                rule: *rule,
                edges: edges.as_deref(),
            },
        }
    }
}

/// The longest key a meet is kept under, in words.
///
/// A key is built and hashed on every meet, and only a render that repeats the one before
/// reads it again. On the stroked Type 3 page of `doc/performance.md`'s table, whose renders
/// never repeat, keys of thousands of words for its 126 meets a pair cost 10.8 M instructions,
/// about a millisecond and a half of its step; on `bug1721218_reduced.pdf`, the page the memo
/// is for, 99% of keys are under 90 words (ADR 1517).
const KEY_WORDS: usize = 1_024;

/// How many row-bucket entries a set's edges may take in one meet, at least: they are
/// bounded by the tile's own bytes or by this, whichever is larger, so a small tile with a
/// tall thin edge is still answered.
const MARK_EDGE_FLOOR: usize = 4_096;

/// What a meet decided: whether its bytes are a function of the mark and the chain alone, and
/// the pixels both sets cut, whose exact areas are made when the frame settles (ADR 1541).
#[derive(Debug)]
pub(super) struct Met {
    /// True unless a budget decided the chain's edges, which is what a caller keeping the
    /// whole tile for the next render must know (ADR 1529).
    pub(super) unbounded: bool,
    /// The meet's exact pixels, still to be made; `None` where the tile's bytes are final.
    pub(super) exact: Option<ExactMeet>,
}

impl Encoder<'_> {
    /// Meet a mark's coverage tile with the residue of `resolved`'s chain, or leave the tile
    /// as it is where the chain is rectangles alone.
    ///
    /// **A meet the render before made is not made again** ([`KeptMeets`], ADR 1517): the
    /// tile it met is a function of the words [`Encoder::meet_words`] lists, so a tile under
    /// the same words is handed the same bytes.
    ///
    /// **The pixels both sets cut are left at `min` and handed back** ([`Met::exact`]): their
    /// areas are made when the frame settles its meets, beside each other, and written over
    /// those bytes where the caller packs the tile ([`Encoder::place_exact`], ADR 1541). The
    /// meet is kept for the next render then, with its finished bytes.
    ///
    /// Answers whether the met bytes are a function of the mark and the chain alone
    /// ([`Met::unbounded`]). A caller keeping the whole tile passes `own_words` false: a tile
    /// it did not find kept is a mark whose meet was not kept either, since the meet's words
    /// hold the tile's and more, so building them would only cost.
    pub(super) fn meet_residue(
        &mut self,
        tile: &mut CoverageMask,
        resolved: &ResolvedClip,
        inputs: MarkInputs<'_>,
        own_words: bool,
    ) -> Result<Met, RenderError> {
        let mark = inputs.mark();
        let settled = |unbounded| Met {
            unbounded,
            exact: None,
        };
        if resolved.residues.is_none() {
            return Ok(settled(true));
        }
        // The queue drains here as [`Encoder::residue_intersection`] drains it, so that a
        // kept meet leaves the walk's order — which marks commit before this one — as a
        // computed one leaves it.
        self.drain_queue()?;
        let words = if own_words {
            self.meet_words(tile, resolved, mark)?
        } else {
            None
        };
        if let Some(met) = words.as_deref().and_then(|words| self.kept.find(words)) {
            tile.coverage.copy_from_slice(&met);
            return Ok(settled(true));
        }
        let Some(clip) =
            self.residue_intersection(resolved, tile.left, tile.top, tile.width, tile.height)?
        else {
            return Ok(settled(true));
        };
        let cut = both_cut(tile, &clip);
        let chain = match cut_rows(tile, &cut) {
            Some(rows) => self.residue_edges(resolved, rows, edge_limit(tile))?,
            None => None,
        };
        // A meet with no pixel both sets cut is `min` whatever the frame has spent; one with
        // such pixels is kept only where no budget decided its chain's edges.
        let unbounded = cut.is_empty() || chain.as_ref().is_some_and(|chain| chain.unbounded);
        // Its own span (ADR 0023): what it computes is the mark's coverage — geometry by the
        // phase's own definition.
        let span = self.clock.start();
        residue_meet(tile, &clip);
        self.clock.geometry(span);
        // A tile with a fractional pixel was filled from polylines; a mark that reached its
        // meet without them keeps `min` rather than meeting an empty set.
        let filled = !cut.is_empty() && !mark.polylines.is_empty();
        let mut exact = chain
            .filter(|_| filled)
            .map(|chain| ExactMeet::new(tile, cut, chain.links, inputs));
        if let Some(words) = words
            && unbounded
        {
            match exact.as_mut() {
                Some(exact) => exact.keep_as_meet(words),
                None => self.kept.keep(
                    words.into_boxed_slice(),
                    &Arc::from(tile.coverage.as_slice()),
                ),
            }
        }
        Ok(Met { unbounded, exact })
    }

    /// The words that name what a meet computes from, or `None` for a meet whose words would
    /// pass [`KEY_WORDS`] and which is neither looked up nor kept: the mark's tile — its place, its
    /// extent and its bytes — the mark's rule and polylines, and the number its chain's
    /// residue was named by ([`KeptMeets::chain`]). Each part leads with its length, so the
    /// words read back one way only.
    ///
    /// The mark's own edges, where its job built them ([`Mark::edges`]), are not in the
    /// words: they meet each row as edges built at the meet would (ADR 1513), so they decide
    /// what a meet costs and not what it draws.
    #[expect(clippy::cast_sign_loss)] // a corner's bits, read back as the same `i32`
    fn meet_words(
        &mut self,
        tile: &CoverageMask,
        resolved: &ResolvedClip,
        mark: Mark<'_>,
    ) -> Result<Option<Vec<u32>>, RenderError> {
        let points: usize = mark.polylines.iter().map(|p| p.points.len()).sum();
        let length = 16_usize
            .saturating_add(points.saturating_mul(2))
            .saturating_add(mark.polylines.len().saturating_mul(2))
            .saturating_add(tile.coverage.len() / 4);
        if length > KEY_WORDS {
            return Ok(None);
        }
        let chain = self.chain_number(resolved)?;
        let mut words = Vec::with_capacity(length);
        words.extend([tile.left as u32, tile.top as u32, tile.width, tile.height]);
        words.extend(tile.coverage.chunks(4).map(|chunk| {
            chunk
                .iter()
                .rev()
                .fold(0_u32, |word, &byte| (word << 8) | u32::from(byte))
        }));
        mark_words(mark.polylines, mark.rule, chain, &mut words);
        Ok(Some(words))
    }

    /// The words that name a whole coverage tile as [`Encoder::coverage_tile`] makes it, or
    /// `None` past [`KEY_WORDS`]: the tile's place and extent, then the words
    /// [`Encoder::meet_words`] ends with — the rule, the polylines and the chain's number. The
    /// filled bytes are a function of the first three (`raster::fill_mask`) and the meet of
    /// those bytes and the fourth, so the words name the tile without its bytes (ADR 1529).
    #[expect(clippy::cast_sign_loss)] // a corner's bits, read back as the same `i32`
    pub(super) fn tile_words(
        &mut self,
        (left, top, width, height): (i32, i32, u32, u32),
        polylines: &[Polyline],
        rule: Rule,
        resolved: &ResolvedClip,
    ) -> Result<Option<Vec<u32>>, RenderError> {
        let points: usize = polylines.iter().map(|p| p.points.len()).sum();
        let length = 8_usize
            .saturating_add(points.saturating_mul(2))
            .saturating_add(polylines.len().saturating_mul(2));
        if length > KEY_WORDS {
            return Ok(None);
        }
        let chain = self.chain_number(resolved)?;
        let mut words = Vec::with_capacity(length);
        words.extend([left as u32, top as u32, width, height]);
        mark_words(polylines, rule, chain, &mut words);
        Ok(Some(words))
    }

    /// The number `resolved`'s residue is named by in [`KeptMeets`], asked once a render for
    /// each chain this scene states.
    pub(super) fn chain_number(&mut self, resolved: &ResolvedClip) -> Result<u64, RenderError> {
        let Some(leaf) = resolved.leaf() else {
            return Ok(0);
        };
        if let Some(number) = self.kept.chain_of(leaf) {
            return Ok(number);
        }
        let content = self.residue_content(resolved)?;
        Ok(self.kept.chain(leaf, content))
    }
}

/// A mark's rule, its polylines and its chain's number as key words, each part leading with
/// its length so that the words read back one way only.
fn mark_words(polylines: &[Polyline], rule: Rule, chain: u64, words: &mut Vec<u32>) {
    words.push(match rule {
        Rule::NonZero => 0,
        Rule::EvenOdd => 1,
    });
    words.push(u32::try_from(polylines.len()).unwrap_or(u32::MAX));
    for polyline in polylines {
        words.push(u32::try_from(polyline.points.len()).unwrap_or(u32::MAX));
        words.push(u32::from(polyline.closed));
        words.extend(
            polyline
                .points
                .iter()
                .flat_map(|point| [point.x.to_bits(), point.y.to_bits()]),
        );
    }
    // The low half first: a number is two words.
    #[expect(clippy::cast_possible_truncation)] // each half of a `u64`, kept
    words.extend([chain as u32, (chain >> 32) as u32]);
}

impl Encoder<'_> {
    /// Meet the residue tile an image samples with the image's own set — `outline`, the
    /// image's device parallelogram intersected with the chain's clip rectangle, the set
    /// `image.wgsl` takes each pixel's area of (ADR 1480, ADR 1492) — where `overlap` is that
    /// area in a pixel as the shader computes it.
    ///
    /// The shader meets that area with the tile by `min`, which is exact wherever one set
    /// holds the pixel whole or misses it. In the pixels where both the image and the residue
    /// are fractional the tile is replaced by the area of their intersection, from both sets'
    /// edges as a path's meet computes it; `min` then reads that area, which is never above
    /// the image's own. Every other pixel of the tile keeps the residue's byte, so an image the
    /// residue holds whole or misses whole draws what it did.
    pub(super) fn meet_residue_with_shape(
        &mut self,
        tile: &mut CoverageMask,
        resolved: &ResolvedClip,
        outline: Vec<Point>,
        overlap: impl Fn(f32, f32) -> f32,
    ) -> Result<(), RenderError> {
        if outline.len() < 3 {
            return Ok(());
        }
        let cut = shape_cut(tile, overlap);
        let Some(rows) = cut_rows(tile, &cut) else {
            return Ok(());
        };
        let Some(chain) = self.residue_edges(resolved, rows, edge_limit(tile))? else {
            return Ok(());
        };
        let shape = [Polyline::polygon(outline)];
        let span = self.clock.start();
        let mark = Mark {
            polylines: &shape,
            rule: Rule::NonZero,
            edges: None,
        };
        if let Some(exact) = exact_areas(tile, &cut, &chain.links, mark) {
            for (&index, value) in cut.iter().zip(exact) {
                let met = &mut tile.coverage[index];
                *met = value.min(*met);
            }
        }
        self.clock.geometry(span);
        Ok(())
    }
}

/// The overlap `image.wgsl`'s `shape_at` takes of an axis-preserving image's rectangle
/// `[x0, y0, x1, y1]` with pixel `(px, py)`: the extent in each axis, multiplied.
pub(super) fn rectangle_overlap([x0, y0, x1, y1]: [f32; 4], px: f32, py: f32) -> f32 {
    let ex = (x1.min(px + 1.0) - x0.max(px)).max(0.0);
    let ey = (y1.min(py + 1.0) - y0.max(py)).max(0.0);
    ex * ey
}

/// The area of the convex polygon `outline` inside pixel `(px, py)` — the oblique image's
/// parallelogram met with the clip rectangle, which `image.wgsl`'s `oblique_area` states in
/// `f32` — computed in `f64` by cutting the polygon to the pixel's four sides
/// (Sutherland–Hodgman) and taking the shoelace area. Only which pixels are fractional is read
/// from it; the bytes come from the meet.
#[expect(clippy::cast_possible_truncation)] // an area in `0 ..= 1`
pub(super) fn polygon_overlap(outline: &[Point], px: f32, py: f32) -> f32 {
    let (x0, y0) = (f64::from(px), f64::from(py));
    let mut polygon: Vec<(f64, f64)> = outline
        .iter()
        .map(|p| (f64::from(p.x), f64::from(p.y)))
        .collect();
    let sides = [
        (0, x0, true),
        (0, x0 + 1.0, false),
        (1, y0, true),
        (1, y0 + 1.0, false),
    ];
    for (axis, bound, above) in sides {
        if polygon.is_empty() {
            return 0.0;
        }
        polygon = keep_side(&polygon, axis, bound, above);
    }
    (twice_area(&polygon).abs() * 0.5).clamp(0.0, 1.0) as f32
}

/// The convex polygon `corners` cut to the rectangle `rect` — an oblique image's
/// parallelogram met with its chain's clip rectangle, the set `image.wgsl` paints.
#[expect(clippy::cast_possible_truncation)] // back to the scene's own `f32` points
pub(super) fn clip_to_rect(corners: &[Point], rect: Rect) -> Vec<Point> {
    let mut polygon: Vec<(f64, f64)> = corners
        .iter()
        .map(|p| (f64::from(p.x), f64::from(p.y)))
        .collect();
    let (x0, y0) = (f64::from(rect.min.x), f64::from(rect.min.y));
    let (x1, y1) = (f64::from(rect.max.x), f64::from(rect.max.y));
    for (axis, bound, above) in [(0, x0, true), (0, x1, false), (1, y0, true), (1, y1, false)] {
        if polygon.is_empty() {
            break;
        }
        polygon = keep_side(&polygon, axis, bound, above);
    }
    polygon
        .into_iter()
        .map(|(x, y)| Point::new(x as f32, y as f32))
        .collect()
}

/// A convex polygon convex-cut to one side of `axis = bound`, above it when `above`.
fn keep_side(polygon: &[(f64, f64)], axis: usize, bound: f64, above: bool) -> Vec<(f64, f64)> {
    let side = |p: (f64, f64)| {
        let v = if axis == 0 { p.0 } else { p.1 };
        if above { v - bound } else { bound - v }
    };
    let mut out = Vec::with_capacity(polygon.len().saturating_add(1));
    for (&here, &next) in polygon.iter().zip(polygon.iter().cycle().skip(1)) {
        let (sh, sn) = (side(here), side(next));
        if sh >= 0.0 {
            out.push(here);
        }
        if (sh >= 0.0) != (sn >= 0.0) {
            let t = sh / (sh - sn);
            out.push((
                here.0 + t * (next.0 - here.0),
                here.1 + t * (next.1 - here.1),
            ));
        }
    }
    out
}

/// Twice a polygon's signed area, the shoelace sum.
fn twice_area(polygon: &[(f64, f64)]) -> f64 {
    polygon
        .iter()
        .zip(polygon.iter().cycle().skip(1))
        .map(|(&a, &b)| a.0 * b.1 - b.0 * a.1)
        .sum()
}

/// The pixels of `tile` where the residue's byte is fractional and so is the image's own
/// area in the pixel, `overlap`.
#[expect(clippy::cast_precision_loss)] // pixel corners of a tile whose extent is a viewport's
#[expect(clippy::cast_possible_wrap, clippy::cast_possible_truncation)]
#[expect(clippy::arithmetic_side_effects)]
fn shape_cut(tile: &CoverageMask, overlap: impl Fn(f32, f32) -> f32) -> Vec<usize> {
    let width = tile.width as usize;
    let fractional = |v: u8| v != 0 && v != u8::MAX;
    tile.coverage
        .iter()
        .enumerate()
        .filter(|&(index, &clip)| {
            if !fractional(clip) {
                return false;
            }
            let px = (tile.left + (index % width) as i32) as f32;
            let py = (tile.top + (index / width) as i32) as f32;
            let area = overlap(px, py);
            area > 0.0 && area < 1.0
        })
        .map(|(index, _)| index)
        .collect()
}

/// The pixels of `tile` where both the mark and the clip are fractional — the only pixels
/// where `min` can differ from the intersection's area.
fn both_cut(tile: &CoverageMask, clip: &CoverageMask) -> Vec<usize> {
    let fractional = |v: u8| v != 0 && v != u8::MAX;
    tile.coverage
        .iter()
        .zip(&clip.coverage)
        .enumerate()
        .filter(|(_, (m, c))| fractional(**m) && fractional(**c))
        .map(|(index, _)| index)
        .collect()
}

/// The device rows `cut` spans, as `(top, count)`, or `None` for no pixel.
#[expect(clippy::cast_possible_truncation, clippy::cast_possible_wrap)] // a row inside a tile
// whose extent is a viewport's
#[expect(clippy::arithmetic_side_effects)] // `last ≥ first`, both rows of the tile
fn cut_rows(tile: &CoverageMask, cut: &[usize]) -> Option<(i32, u32)> {
    let width = tile.width as usize;
    let first = cut.first()? / width;
    let last = cut.last()? / width;
    Some((tile.top + first as i32, (last - first + 1) as u32))
}

/// How many row-bucket entries one set's edges may take for a meet over `tile`: the tile's
/// own bytes, or [`MARK_EDGE_FLOOR`] for a small one — the coverage the meet already holds
/// bounds the edges it reads.
fn edge_limit(tile: &CoverageMask) -> usize {
    tile.coverage.len().max(MARK_EDGE_FLOOR)
}

/// How many row-bucket entries the edges a fan-out job builds over its whole tile may take
/// ([`Mark::edges`]): four times what one meet over that tile may read. A meet reads only
/// the rows where both sets cut a pixel, and the job cannot know which rows those are, so it
/// buckets every row; on the stroked Type 3 page of `doc/performance.md`'s table the largest
/// whole tile charged 2.07 times its meet's bound (ADR 1513). Past this the job keeps no
/// edges and the meet builds over its own rows, as it does for the walk's tiles.
const JOB_EDGE_REACH: usize = 4;

/// The fewest point-rows — a mark's polyline points times its tile's rows, a bound on the
/// edge parts a build buckets — for which a fan-out job builds its mark's edges itself.
///
/// A smaller mark's edges cost the walk's thread little to build at the meet, and only where
/// its tile has a pixel both sets cut; built in every job they cost more than they save. On
/// `bug1721218_reduced.pdf` 19 536 residue jobs averaged 66 point-rows and none reached a
/// thousand, and building all of them took its turn from 270 to 274 ms; the stroked Type 3
/// page's 189 were all above a thousand (ADR 1513).
pub(super) const JOB_EDGE_WORK: usize = 1_024;

/// [`edge_limit`] for a job's build over the whole of `tile` ([`JOB_EDGE_REACH`]).
pub(super) fn job_edge_limit(tile: &CoverageMask) -> usize {
    edge_limit(tile).saturating_mul(JOB_EDGE_REACH)
}

/// The intersection's area in each of the `cut` pixels of `tile`, as bytes, or `None`
/// where the mark's edges would pass their bound and the tile keeps `min`.
#[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // an area in 0..=1,
// and pixel indices inside a tile whose extent is a viewport's
#[expect(clippy::cast_possible_wrap, clippy::arithmetic_side_effects)]
fn exact_areas(
    tile: &CoverageMask,
    cut: &[usize],
    links: &[RowEdges],
    mark: Mark<'_>,
) -> Option<Vec<u8>> {
    if mark.polylines.is_empty() {
        return None;
    }
    let width = tile.width as usize;
    let (top, rows) = cut_rows(tile, cut)?;
    let limit = edge_limit(tile);
    let built;
    // The job's edges over the whole tile, asked the bound the meet's own rows would ask
    // (ADR 1513): past it the meet keeps `min`, as a build over those rows would have.
    let edges = if let Some((edges, Some(charge))) =
        mark.edges.map(|edges| (edges, edges.charge(top, rows)))
    {
        if usize::try_from(charge).map_or(true, |charge| charge > limit) {
            return None;
        }
        edges
    } else {
        built = RowEdges::of(mark.polylines, mark.rule, top, rows, limit)?;
        &built
    };
    let sets: Vec<&RowEdges> = std::iter::once(edges).chain(links).collect();
    let mut work = MeetWork::default();
    Some(
        cut.iter()
            .map(|&index| {
                let x = tile.left + (index % width) as i32;
                let y = tile.top + (index / width) as i32;
                let area = raster::area_in_pixel(x, y, &sets, &mut work);
                (area * 255.0).round() as u8
            })
            .collect(),
    )
}

/// The mark's coverage met with the chain's: `tile ← min(tile, clip)`, per pixel.
///
/// The same paragraph of §10.7.4 says which side of the interval
/// `[max(0, s + c − 1), min(s, c)]` an estimate may take: "[t]he area covered by painted
/// pixels shall always be at least as large as the area of the original shape." `min` is
/// the least value never below the intersection, and it is the intersection wherever one
/// set holds the other in the pixel (ADR 1444) — every pixel [`exact_areas`] does not
/// answer.
fn residue_meet(tile: &mut CoverageMask, clip: &CoverageMask) {
    for (m, l) in tile.coverage.iter_mut().zip(&clip.coverage) {
        *m = (*m).min(*l);
    }
}

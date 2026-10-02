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

use super::Encoder;
use super::clips::ResolvedClip;
use crate::error::RenderError;
use raster_scene::{Point, Rect};

use crate::raster::{self, CoverageMask, MeetWork, Polyline, RowEdges, Rule};

/// The mark a residue meets: the polylines its coverage was filled from, and the rule.
#[derive(Debug, Clone, Copy)]
pub(super) struct Mark<'p> {
    pub(super) polylines: &'p [Polyline],
    pub(super) rule: Rule,
}

/// How many row-bucket entries a set's edges may take in one meet, at least: they are
/// bounded by the tile's own bytes or by this, whichever is larger, so a small tile with a
/// tall thin edge is still answered.
const MARK_EDGE_FLOOR: usize = 4_096;

impl Encoder<'_> {
    /// Meet a mark's coverage tile with the residue of `resolved`'s chain, or leave the tile
    /// as it is where the chain is rectangles alone.
    pub(super) fn meet_residue(
        &mut self,
        tile: &mut CoverageMask,
        resolved: &ResolvedClip,
        mark: Mark<'_>,
    ) -> Result<(), RenderError> {
        let Some(clip) =
            self.residue_intersection(resolved, tile.left, tile.top, tile.width, tile.height)?
        else {
            return Ok(());
        };
        let cut = both_cut(tile, &clip);
        let links = match cut_rows(tile, &cut) {
            Some(rows) => self.residue_edges(resolved, rows, edge_limit(tile))?,
            None => None,
        };
        // Its own span (ADR 0023): what it computes is the mark's coverage — geometry by the
        // phase's own definition.
        let span = self.clock.start();
        let exact = links.and_then(|links| exact_areas(tile, &cut, &links, mark));
        residue_meet(tile, &clip);
        if let Some(exact) = exact {
            for (&index, value) in cut.iter().zip(exact) {
                // Never above either set's own coverage: the bytes are each the set's area
                // rounded, and the intersection is inside both.
                let met = &mut tile.coverage[index];
                *met = value.min(*met);
            }
        }
        self.clock.geometry(span);
        Ok(())
    }
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
        let Some(links) = self.residue_edges(resolved, rows, edge_limit(tile))? else {
            return Ok(());
        };
        let shape = [Polyline::polygon(outline)];
        let span = self.clock.start();
        let mark = Mark {
            polylines: &shape,
            rule: Rule::NonZero,
        };
        if let Some(exact) = exact_areas(tile, &cut, &links, mark) {
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
    // A tile with a fractional pixel was filled from polylines; a mark that reached its
    // meet without them keeps `min` rather than meeting an empty set.
    if mark.polylines.is_empty() {
        return None;
    }
    let width = tile.width as usize;
    let (top, rows) = cut_rows(tile, cut)?;
    let edges = RowEdges::of(mark.polylines, mark.rule, top, rows, edge_limit(tile))?;
    let sets: Vec<&RowEdges> = std::iter::once(&edges).chain(links).collect();
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

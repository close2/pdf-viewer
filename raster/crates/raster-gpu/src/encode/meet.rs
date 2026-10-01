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

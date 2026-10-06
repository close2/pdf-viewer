//! A meet's exact areas measured from convex polygons, where the mark is one and the chain is,
//! inside the tile, convex pieces standing apart (ADR 1582).
//!
//! ISO 32000-2 §10.7.4 makes a clipped mark's coverage the intersection of two sets of pixels,
//! and [`super::exact_areas`] measures that intersection in a pixel from both sets' edges, band
//! by band, for any sets. **Where every set is the intersection of half-planes, so is their
//! intersection**: a convex polygon, which [`ConvexMeet`] cuts to each pixel and measures by the
//! shoelace — the same area, from the same `f32` points, in `f64`. On `bug1721218_reduced.pdf`
//! every meet is a shading's disc under a clip of three thousand dots, each pair a pixel or two
//! across, and the bands' construction spent about 45 000 instructions a pixel on it.
//!
//! **It runs where the general meet runs, on the frame's helpers, and decides there** — after
//! the walk has made everything the general meet reads, so that a meet this module declines is
//! the general meet unchanged and the walk's thread does the same work either way. What it reads
//! beyond that is each link's flattening and row index, which the frame keeps already.

use std::sync::Arc;

use super::{Mark, cut_rows, edge_limit};
use crate::raster::{self, Convex, ConvexMeet, ConvexWork, CoverageMask, Polyline, RowIndex};

/// The most convex polygons one meet is measured as: one for each way of taking a piece of
/// every link. A dot clip met by a shading's disc is one; a meet past this is measured from
/// its edges, whose cost does not grow with the product.
const MEETS_PER_TILE: usize = 16;

/// One link of a chain as a meet made beside the walk reads it: its polylines, and its row
/// index where the frame kept one.
#[derive(Debug, Clone)]
pub(in crate::encode) struct ChainLink {
    pub(in crate::encode) polylines: Arc<[Polyline]>,
    pub(in crate::encode) index: Option<Arc<RowIndex>>,
}

/// The intersection's area in each of the `cut` pixels of `tile`, as bytes, measured from
/// convex polygons; `None` where the meet is not one this module measures, and the general
/// meet answers it.
///
/// **It answers exactly the meets the general meet answers.** The general meet leaves a tile
/// at `min` only where the mark's edges over the meet's rows would pass their bound, and the
/// mark is measured here only where that bound is proved not passed: a build over `rows`
/// charges each edge once for every row it reaches and each horizontal edge once
/// ([`raster::RowEdges::of`]); a horizontal line crosses a convex polygon's boundary twice, so
/// its edges' heights inside the rows sum to at most twice the rows, and each edge reaches at
/// most two rows beyond its height — so `2 · rows + 3 · points` bounds the charge. The chain's
/// edges the walk made already, as it does for every meet.
///
/// Inside the tile each link is the disjoint union of its pieces ([`pieces`]), so the
/// intersection is the disjoint union of one convex polygon for each way of taking a piece of
/// every link, and a pixel's area is the sum of theirs.
#[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // an area in 0..=1,
// and pixel indices inside a tile whose extent is a viewport's
#[expect(clippy::cast_possible_wrap, clippy::arithmetic_side_effects)]
pub(super) fn areas(
    tile: &CoverageMask,
    cut: &[usize],
    mark: Mark<'_>,
    chain: &[ChainLink],
) -> Option<Vec<u8>> {
    let (_, rows) = cut_rows(tile, cut)?;
    let points: usize = mark.polylines.iter().map(|p| p.points.len()).sum();
    let charge = usize::try_from(rows)
        .ok()?
        .checked_mul(2)?
        .checked_add(points.checked_mul(3)?)?;
    if charge > edge_limit(tile) {
        return None;
    }
    let own = Convex::of(mark.polylines)?;
    let mut choices: Vec<Vec<(&Polyline, Convex)>> = vec![vec![(mark.polylines.first()?, own)]];
    for link in chain {
        let pieces = pieces(link, tile)?;
        if choices.len().checked_mul(pieces.len())? > MEETS_PER_TILE {
            return None;
        }
        let mut grown = Vec::with_capacity(choices.len() * pieces.len());
        for chosen in &choices {
            for &(subpath, convex) in &pieces {
                let mut next = chosen.clone();
                next.push((link.polylines.get(subpath)?, convex));
                grown.push(next);
            }
        }
        choices = grown;
    }
    let meets: Vec<ConvexMeet> = choices.iter().map(|sets| ConvexMeet::of(sets)).collect();
    let width = tile.width as usize;
    let mut work = ConvexWork::default();
    Some(
        cut.iter()
            .map(|&index| {
                let x = tile.left + (index % width) as i32;
                let y = tile.top + (index / width) as i32;
                let area: f64 = meets
                    .iter()
                    .map(|meet| meet.area_in_pixel(x, y, &mut work))
                    .sum();
                (area.clamp(0.0, 1.0) * 255.0).round() as u8
            })
            .collect(),
    )
}

/// The subpaths of `link` whose boxes meet `tile`, each with the side it keeps its inside on,
/// where each is one convex polygon and no two boxes meet; `None` where one is not, or where a
/// link of many subpaths has no row index to find them by.
///
/// A subpath whose box misses the tile winds nothing inside it, which is why the others are
/// all a meet over the tile reads; and two convex subpaths whose boxes stand apart share no
/// point, so every point of the tile winds at most once, and §8.5.3.3's two rules agree.
fn pieces(link: &ChainLink, tile: &CoverageMask) -> Option<Vec<(usize, Convex)>> {
    // A tile's corners are a target's, well inside `f32`'s exact integers, and an `i32` corner
    // plus a `u32` extent cannot wrap in `i64`.
    #[expect(clippy::cast_precision_loss, clippy::arithmetic_side_effects)]
    let (x0, y0, x1, y1) = (
        tile.left as f32,
        tile.top as f32,
        (i64::from(tile.left) + i64::from(tile.width)) as f32,
        (i64::from(tile.top) + i64::from(tile.height)) as f32,
    );
    let meets = |b: &(f32, f32, f32, f32)| b.2 >= x0 && b.0 <= x1 && b.3 >= y0 && b.1 <= y1;
    if let [polyline] = &*link.polylines {
        return match raster::polyline_bounds(&link.polylines) {
            Some(bounds) if meets(&bounds) => {
                Some(vec![(0, Convex::of(std::slice::from_ref(polyline))?)])
            }
            _ => Some(Vec::new()),
        };
    }
    let listed = link
        .index
        .as_deref()?
        .subpaths_meeting(tile.top, tile.height as usize)?;
    let mut pieces = Vec::new();
    let mut boxes: Vec<(f32, f32, f32, f32)> = Vec::new();
    for (subpath, bounds) in listed {
        let Some(bounds) = bounds.filter(meets) else {
            continue;
        };
        let apart = |o: &(f32, f32, f32, f32)| {
            o.2 < bounds.0 || bounds.2 < o.0 || o.3 < bounds.1 || bounds.3 < o.1
        };
        if !boxes.iter().all(apart) {
            return None;
        }
        let polyline = link.polylines.get(subpath)?;
        pieces.push((subpath, Convex::of(std::slice::from_ref(polyline))?));
        boxes.push(bounds);
    }
    Some(pieces)
}

#[cfg(test)]
mod tests;

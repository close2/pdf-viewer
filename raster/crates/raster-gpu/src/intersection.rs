//! The meet's polygon arithmetic, reachable from outside the crate for the `meet` fuzz target
//! (ADR 1495).
//!
//! ISO 32000-2 §10.7.4 makes painting under a clip the intersection of two sets of pixels, and
//! where a residue clip and a mark both cover a pixel partly the device computes that
//! intersection's area from their edges (ADR 1467). The edges come from a document's paths, so
//! their coincidences, degeneracies and crossings are whatever a producer wrote. This module
//! states the computation over plain points, so that a target can hand it two hostile polygons
//! and compare the answer against an integration it does by other means; nothing in the crate
//! calls it.

use raster_scene::Point;

use crate::raster::{MeetWork, Polyline, RowEdges, Rule, area_in_pixel};

/// One set: closed subpaths of points in device space, filled under one of §8.5.3.3's rules.
#[derive(Debug, Clone, Copy)]
pub struct Set<'a> {
    /// Each subpath's points; a fill closes every one of them (§8.5.3.1).
    pub subpaths: &'a [Vec<(f32, f32)>],
    /// §8.5.3.3.3's even-odd rule where true, §8.5.3.3.2's nonzero winding rule where false.
    pub even_odd: bool,
}

/// The area, in `0 ..= 1`, of the intersection of every set inside device pixel `(x, y)`, as the
/// meet computes it, or `None` where a set's edges over the rows `top .. top + rows` would hold
/// more than `limit` entries — the bound the device keeps by refusing rather than allocating.
#[must_use]
pub fn area_in_pixel_of(
    (x, y): (i32, i32),
    sets: &[Set<'_>],
    (top, rows): (i32, u32),
    limit: usize,
) -> Option<f64> {
    let built: Option<Vec<RowEdges>> = sets
        .iter()
        .map(|set| {
            let polylines: Vec<Polyline> = set
                .subpaths
                .iter()
                .map(|points| Polyline {
                    points: points.iter().map(|&(px, py)| Point::new(px, py)).collect(),
                    closed: true,
                    tangents: Vec::new(),
                })
                .collect();
            let rule = if set.even_odd {
                Rule::EvenOdd
            } else {
                Rule::NonZero
            };
            RowEdges::of(&polylines, rule, top, rows, limit)
        })
        .collect();
    let built = built?;
    let borrowed: Vec<&RowEdges> = built.iter().collect();
    Some(area_in_pixel(x, y, &borrowed, &mut MeetWork::default()))
}

#[cfg(test)]
mod tests {
    use super::{Set, area_in_pixel_of};

    /// The meet's own first case through the seam: the rectangle `[-3, 9] × [2.2, 9]` and the
    /// half-plane `x ≤ 2.6` meet in pixel `(2, 2)` over `[2, 2.6] × [2.2, 3]`, `0.48`; and a limit
    /// of no entries is refused rather than answered.
    #[test]
    fn the_seam_answers_the_meets_closed_form_and_keeps_its_limit() {
        let rectangle = [vec![(-3.0, 2.2), (9.0, 2.2), (9.0, 9.0), (-3.0, 9.0)]];
        let half = [vec![(-3.0, -3.0), (2.6, -3.0), (2.6, 9.0), (-3.0, 9.0)]];
        let sets = [
            Set {
                subpaths: &rectangle,
                even_odd: false,
            },
            Set {
                subpaths: &half,
                even_odd: false,
            },
        ];
        let area = area_in_pixel_of((2, 2), &sets, (-4, 16), usize::MAX).expect("no limit");
        assert!((area - 0.48).abs() < 1e-6, "{area} against 0.48");
        assert_eq!(area_in_pixel_of((2, 2), &sets, (-4, 16), 0), None);
    }
}

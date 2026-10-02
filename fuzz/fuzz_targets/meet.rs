//! Fuzzes the meet: the area of several fills' intersection inside one pixel, which the device
//! computes from the fills' edges where a residue clip and a mark both cover a pixel partly
//! (ISO 32000-2 §10.7.4, ADR 1467). A differential target: the answer is compared against an
//! integration this file does by other means, and a difference is a finding.
//!
//! The edges are a document's paths, so every coincidence, collinear run, horizontal edge and
//! crossing inside one pixel is a producer's to write. The input states two or three sets, each a
//! rule and one or two closed subpaths of three to eight points, on a grid of a sixteenth of a
//! pixel (so that coincidences are common) or, where the first byte asks, of a 2048th.
//!
//! **The reference.** Inside the pixel, the length of the horizontal line at height `y` that every
//! set holds is a linear function of `y` between any two consecutive heights at which a vertex
//! lies, an edge crosses one of the pixel's sides, or two edges cross — between those nothing
//! changes order and nothing enters or leaves. So the reference collects every such height by
//! brute force, evaluates that length at the quarter and three-quarter points of each piece —
//! §8.5.3.3's winding count along the line, set by set — and sums each piece's trapezoid, which
//! for a linear function is exact. It shares no code with the meet's bands, winding-from-the-left
//! and strip walk; what the two have in common is the definition of the sets.
//!
//! Beyond never panicking — overflow checks stay on in this profile — two properties:
//!
//! - **The area lies in `0 ..= 1`.**
//! - **The area is the reference's**, to within `1e-6` of a pixel.

#![no_main]
#![expect(
    clippy::arithmetic_side_effects,
    clippy::panic,
    reason = "the reference is floating-point geometry over at most 48 edges, and a fuzz target states its properties by failing"
)]

use libfuzzer_sys::fuzz_target;
use raster_gpu::intersection::{Set, area_in_pixel_of};

/// One edge, ends ordered top to bottom, with the direction the path ran.
#[derive(Clone, Copy)]
struct Edge {
    x0: f64,
    y0: f64,
    x1: f64,
    y1: f64,
    up: bool,
}

impl Edge {
    fn x_at(&self, y: f64) -> f64 {
        self.x0 + (y - self.y0) * ((self.x1 - self.x0) / (self.y1 - self.y0))
    }
}

/// Every non-horizontal edge of a set's subpaths, each closed.
fn edges(subpaths: &[Vec<(f32, f32)>]) -> Vec<Edge> {
    let mut out = Vec::new();
    for points in subpaths {
        for (i, &(ax, ay)) in points.iter().enumerate() {
            let (bx, by) = points[(i + 1) % points.len()];
            let (ax, ay, bx, by) = (f64::from(ax), f64::from(ay), f64::from(bx), f64::from(by));
            if ay.total_cmp(&by).is_eq() {
                continue;
            }
            out.push(if ay < by {
                Edge {
                    x0: ax,
                    y0: ay,
                    x1: bx,
                    y1: by,
                    up: false,
                }
            } else {
                Edge {
                    x0: bx,
                    y0: by,
                    x1: ax,
                    y1: ay,
                    up: true,
                }
            });
        }
    }
    out
}

/// The intervals of `[left, right]` a set holds along the line at height `y`.
fn held(edges: &[Edge], even_odd: bool, y: f64, (left, right): (f64, f64)) -> Vec<(f64, f64)> {
    let mut crossings: Vec<(f64, i32)> = edges
        .iter()
        .filter(|e| e.y0 <= y && y < e.y1)
        .map(|e| (e.x_at(y), if e.up { -1 } else { 1 }))
        .collect();
    crossings.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut out = Vec::new();
    let mut winding = 0;
    for pair in crossings.windows(2) {
        winding += pair[0].1;
        let inside = if even_odd {
            winding % 2 != 0
        } else {
            winding != 0
        };
        let (from, to) = (pair[0].0.max(left), pair[1].0.min(right));
        if inside && from < to {
            out.push((from, to));
        }
    }
    out
}

/// The length of `[left, right]` every set holds at height `y`.
fn length_at(sets: &[(Vec<Edge>, bool)], y: f64, span: (f64, f64)) -> f64 {
    let mut common = vec![span];
    for (edges, even_odd) in sets {
        let mine = held(edges, *even_odd, y, span);
        let mut next = Vec::new();
        for &(a0, a1) in &common {
            for &(b0, b1) in &mine {
                let (from, to) = (a0.max(b0), a1.min(b1));
                if from < to {
                    next.push((from, to));
                }
            }
        }
        common = next;
    }
    common.iter().map(|(from, to)| to - from).sum()
}

/// The reference area of the sets' intersection inside the pixel `(x, y)`.
fn reference(sets: &[(Vec<Edge>, bool)], (column, row): (i32, i32)) -> f64 {
    let (left, right) = (f64::from(column), f64::from(column) + 1.0);
    let (top, bottom) = (f64::from(row), f64::from(row) + 1.0);
    let all: Vec<Edge> = sets
        .iter()
        .flat_map(|(edges, _)| edges.iter().copied())
        .collect();
    let mut heights = vec![top, bottom];
    for e in &all {
        heights.extend([e.y0, e.y1]);
        for side in [left, right] {
            let (low, high) = (e.x0.min(e.x1), e.x0.max(e.x1));
            if low < side && side < high {
                heights.push(e.y0 + (side - e.x0) * ((e.y1 - e.y0) / (e.x1 - e.x0)));
            }
        }
    }
    for (i, a) in all.iter().enumerate() {
        for b in &all[i + 1..] {
            let (from, to) = (a.y0.max(b.y0), a.y1.min(b.y1));
            if from >= to {
                continue;
            }
            let (da, db) = (a.x_at(from) - b.x_at(from), a.x_at(to) - b.x_at(to));
            if (da < 0.0) != (db < 0.0) && da != 0.0 && db != 0.0 {
                heights.push(from + (to - from) * (da / (da - db)));
            }
        }
    }
    heights.retain(|h| (top..=bottom).contains(h));
    heights.sort_by(f64::total_cmp);
    heights.dedup();
    heights
        .windows(2)
        .map(|piece| {
            let (from, to) = (piece[0], piece[1]);
            let height = to - from;
            f64::midpoint(
                length_at(sets, from + height / 4.0, (left, right)),
                length_at(sets, from + 3.0 * height / 4.0, (left, right)),
            ) * height
        })
        .sum()
}

/// A set's closed subpaths, each its points.
type Subpaths = Vec<Vec<(f32, f32)>>;

/// A cursor over the input.
struct Reader<'a>(&'a [u8]);

impl Reader<'_> {
    fn byte(&mut self) -> Option<u8> {
        let (&first, rest) = self.0.split_first()?;
        self.0 = rest;
        Some(first)
    }
    fn coordinate(&mut self, fine: bool) -> Option<f32> {
        if fine {
            let pair = [self.byte()?, self.byte()?];
            Some(f32::from(i16::from_le_bytes(pair)) / 2048.0 + 2.0)
        } else {
            Some(f32::from(self.byte()?) / 16.0 - 4.0)
        }
    }
}

fuzz_target!(|data: &[u8]| {
    let mut input = Reader(data);
    let Some(head) = input.byte() else {
        return;
    };
    let count = 2 + usize::from(head & 1);
    let fine = head & 2 != 0;
    let pixel = (i32::from((head >> 2) & 3), i32::from((head >> 4) & 3));
    let mut stated: Vec<(Subpaths, bool)> = Vec::new();
    for _ in 0..count {
        let Some(shape) = input.byte() else {
            return;
        };
        let mut subpaths = Vec::new();
        for _ in 0..=usize::from((shape >> 1) & 1) {
            let Some(points) = input.byte() else {
                return;
            };
            let mut path = Vec::new();
            for _ in 0..3 + usize::from(points % 6) {
                let (Some(px), Some(py)) = (input.coordinate(fine), input.coordinate(fine)) else {
                    return;
                };
                path.push((px, py));
            }
            subpaths.push(path);
        }
        stated.push((subpaths, shape & 1 != 0));
    }
    let sets: Vec<Set<'_>> = stated
        .iter()
        .map(|(subpaths, even_odd)| Set {
            subpaths,
            even_odd: *even_odd,
        })
        .collect();
    let area = area_in_pixel_of(pixel, &sets, (-20, 48), usize::MAX)
        .unwrap_or_else(|| panic!("no limit was stated, so the meet answers"));
    assert!(
        (-1e-9..=1.0 + 1e-9).contains(&area),
        "the meet's area {area} lies in 0 ..= 1"
    );
    let edged: Vec<(Vec<Edge>, bool)> = stated
        .iter()
        .map(|(subpaths, even_odd)| (edges(subpaths), *even_odd))
        .collect();
    let expected = reference(&edged, pixel);
    assert!(
        (area - expected).abs() <= 1e-6,
        "the meet's area {area} against the reference's {expected} in pixel {pixel:?}"
    );
});

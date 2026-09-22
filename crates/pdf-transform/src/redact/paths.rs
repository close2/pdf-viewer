//! The geometry a redaction does to a painted path, ISO 32000-2 §12.5.6.23.
//!
//! §12.5.6.23 requires a processor applying a redaction to "remove all content identified by the
//! redaction annotation" and to "remove all traces of the specified content". For a glyph the
//! unit removed is the code; for an image it is the sample. A painted path has no such unit —
//! the mark is the region the path encloses — so the removal is **geometric**: the path's marks
//! are cut to the complement of the redaction region, and the surviving geometry is written back
//! into the content stream as fresh path operators. Nothing is covered and nothing is clipped;
//! the coordinates that described the removed marks are gone from the file.
//!
//! # The construction, and why it is exact
//!
//! The region is an axis-aligned box in the display list's space. Its four edge lines cut the
//! plane into nine cells — three slabs across by three slabs up — of which the middle cell *is*
//! the region and the other eight tile its complement. Each of those eight is convex, being an
//! intersection of at most four half-planes, and the eight have pairwise disjoint interiors.
//!
//! So `P \ R` is the union over the eight outer cells of `P ∩ cell`, and each term is a
//! Sutherland–Hodgman clip of a polygon against a convex window: exact, one pass per half-plane,
//! and orientation-preserving, so a nonzero-winding fill and an even-odd fill both survive it
//! (each subpath is clipped on its own and the cells do not overlap, so no interior is wound
//! twice and no crossing is counted twice). The degenerate edges the algorithm lays along a
//! window boundary enclose no area and are invisible to either fill rule — which is why this is
//! a fill's construction and a stroke is refused rather than cut (§8.5.3's stroke is the
//! *outline* of the path, and splitting a path introduces caps the producer did not write).
//!
//! # What keeps the survivors byte-exact
//!
//! A vertex that survives is copied, never recomputed: the half-plane test is evaluated in the
//! display list's space, and the vertex it keeps is the source's own user-space pair. Only a
//! vertex the cut *creates* is arithmetic, and it is interpolated along the source edge in user
//! space. So the geometry outside the region is the producer's own numbers.
//!
//! # The margin, and the guard that makes it a proof
//!
//! A created vertex is written back as decimal text and read back by a processor whose real
//! numbers are single precision (§7.3.3). Both roundings can move a cut edge, and moving it
//! *into* the region would leave a sliver of the redacted marks alive. So the region is widened
//! by [`REGION_PAD`] before the cut — the removal reaches that much past the quad, which is the
//! safe direction — and [`Cut::margin_holds`] refuses the page unless the worst displacement the
//! two roundings can produce is strictly smaller than the widening. A margin nobody has checked
//! is not a margin.

/// How far past the region the cut reaches, in the display list's units, so that writing the cut
/// vertices as decimal text and reading them back as §7.3.3 reals cannot leave a sliver of the
/// redacted marks alive. One hundredth of a point: below what any device resolves, and large
/// enough that [`Cut::margin_holds`] passes for every coordinate a page of ordinary size holds.
pub(super) const REGION_PAD: f64 = 0.01;

/// How many decimal places a created vertex is written with.
pub(super) const DECIMALS: usize = 6;

/// Half a unit in the last place [`DECIMALS`] writes, which is the most this writer's own
/// rounding can displace a created vertex.
const DECIMAL_HALF_ULP: f64 = 0.000_000_5;

/// The largest number of surviving polygons a cut may produce before the page is refused.
///
/// Each region multiplies the polygon count by at most eight (the cells its edges cut the plane
/// into), so a path meeting many regions could otherwise grow a content stream without bound.
/// The bound is a resource bound and says so, rather than a silent truncation (trap 38).
const MAX_POLYGONS: usize = 512;

/// A path's subpath: its vertices in the content stream's own user space, implicitly closed.
pub(super) type SubPath = Vec<(f64, f64)>;

/// An affine map from the content stream's user space into the display list's space, held at
/// double precision because the cut's decisions are made in it.
#[derive(Clone, Copy)]
pub(super) struct Mapping {
    /// The linear part, row-major: `x' = a·x + c·y + e`, `y' = b·x + d·y + f`.
    pub(super) a: f64,
    pub(super) b: f64,
    pub(super) c: f64,
    pub(super) d: f64,
    pub(super) e: f64,
    pub(super) f: f64,
}

impl Mapping {
    /// The point's display-space image.
    fn apply(self, point: (f64, f64)) -> (f64, f64) {
        (
            self.a.mul_add(point.0, self.c.mul_add(point.1, self.e)),
            self.b.mul_add(point.0, self.d.mul_add(point.1, self.f)),
        )
    }

    /// The largest factor by which the linear part can stretch a displacement — the row sums of
    /// the absolute matrix, which bound the operator norm from above. A displacement `δ` in user
    /// space moves its image by at most `norm · δ`, which is what the margin guard needs.
    fn norm(self) -> f64 {
        (self.a.abs() + self.c.abs()).max(self.b.abs() + self.d.abs())
    }
}

/// Which of the two display-space axes a half-plane cuts across.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Axis {
    X,
    Y,
}

/// A closed half-plane of the display list's space, bounded by a line parallel to an axis.
#[derive(Clone, Copy)]
struct HalfPlane {
    axis: Axis,
    bound: f64,
    /// Whether the kept side is the one with the larger coordinate.
    keep_above: bool,
}

impl HalfPlane {
    /// How far inside the half-plane a display-space point lies: non-negative inside, and zero
    /// exactly on the boundary.
    fn depth(self, at: (f64, f64)) -> f64 {
        let value = match self.axis {
            Axis::X => at.0,
            Axis::Y => at.1,
        };
        if self.keep_above {
            value - self.bound
        } else {
            self.bound - value
        }
    }
}

/// What a cut produced, and the numbers the margin guard is checked against.
pub(super) struct Cut {
    /// The surviving polygons, in the content stream's own user space.
    pub(super) polygons: Vec<SubPath>,
    /// The largest coordinate magnitude any created vertex carries, which decides how coarse the
    /// single-precision grid is where that vertex lands.
    largest: f64,
    /// The mapping's norm, which turns a user-space displacement into a display-space one.
    norm: f64,
}

impl Cut {
    /// Whether the rounding a created vertex will undergo stays inside [`REGION_PAD`].
    ///
    /// Two roundings displace a created vertex: this writer's own, which is half a unit in the
    /// last of [`DECIMALS`] decimal places, and the single-precision one §7.3.3 permits a
    /// processor reading the file back, which is half an ulp of the coordinate's own magnitude.
    /// Their sum, carried into the display list's space by the mapping's norm, must be strictly
    /// less than the widening the cut was computed with — otherwise the cut edge could land
    /// inside the true region and leave a sliver of the redacted marks alive, so the page is
    /// refused instead.
    pub(super) fn margin_holds(&self) -> bool {
        // `f32::EPSILON / 2` is half an ulp of a single-precision number relative to its own
        // magnitude; `DECIMAL_HALF_ULP` is half a unit in the last place this writer emits.
        let single = self.largest * f64::from(f32::EPSILON) / 2.0;
        (single + DECIMAL_HALF_ULP) * self.norm < REGION_PAD
    }
}

/// Subtracts every region from the path's subpaths, in the path's own user space.
///
/// `regions` are axis-aligned boxes `[x0, y0, x1, y1]` in the display list's space, which
/// `to_display` maps the path's coordinates into. Each is widened by [`REGION_PAD`] first, so the
/// removal reaches slightly past the quad rather than slightly short of it.
///
/// `None` where the result would exceed [`MAX_POLYGONS`]: a resource bound, refused by the caller
/// rather than truncated.
pub(super) fn subtract(
    subpaths: &[SubPath],
    regions: &[[f64; 4]],
    to_display: Mapping,
) -> Option<Cut> {
    let mut polygons: Vec<SubPath> = subpaths
        .iter()
        .filter(|points| points.len() >= 3)
        .cloned()
        .collect();
    let mut largest = 0.0f64;
    for region in regions {
        let mut next: Vec<SubPath> = Vec::new();
        for cell in outer_cells(*region) {
            for polygon in &polygons {
                let clipped = clip_to_cell(polygon, &cell, to_display);
                if encloses_area(&clipped) {
                    next.push(clipped);
                }
            }
            if next.len() > MAX_POLYGONS {
                return None;
            }
        }
        polygons = next;
    }
    // Every vertex the cut created is one the source did not hold; the guard is checked against
    // the largest of them, because a single-precision ulp grows with the magnitude.
    let sources: std::collections::HashSet<(u64, u64)> = subpaths
        .iter()
        .flatten()
        .map(|point| (point.0.to_bits(), point.1.to_bits()))
        .collect();
    for point in polygons.iter().flatten() {
        if !sources.contains(&(point.0.to_bits(), point.1.to_bits())) {
            largest = largest.max(point.0.abs()).max(point.1.abs());
        }
    }
    Some(Cut {
        polygons,
        largest,
        norm: to_display.norm(),
    })
}

/// The eight convex cells that tile the complement of a region, each as the half-planes whose
/// intersection it is.
///
/// The region's four edge lines cut the plane into a three-by-three grid of slabs; the middle
/// cell is the region itself and is left out. The eight have pairwise disjoint interiors and
/// cover everything else, which is what makes the union of the clips an exact difference.
fn outer_cells(region: [f64; 4]) -> Vec<Vec<HalfPlane>> {
    let [x0, y0, x1, y1] = region;
    let (x0, y0) = (x0 - REGION_PAD, y0 - REGION_PAD);
    let (x1, y1) = (x1 + REGION_PAD, y1 + REGION_PAD);
    let slabs = |low: f64, high: f64, axis: Axis| {
        [
            vec![HalfPlane {
                axis,
                bound: low,
                keep_above: false,
            }],
            vec![
                HalfPlane {
                    axis,
                    bound: low,
                    keep_above: true,
                },
                HalfPlane {
                    axis,
                    bound: high,
                    keep_above: false,
                },
            ],
            vec![HalfPlane {
                axis,
                bound: high,
                keep_above: true,
            }],
        ]
    };
    let across = slabs(x0, x1, Axis::X);
    let up = slabs(y0, y1, Axis::Y);
    let mut cells = Vec::with_capacity(8);
    for (i, column) in across.iter().enumerate() {
        for (j, row) in up.iter().enumerate() {
            if i == 1 && j == 1 {
                continue;
            }
            let mut cell = column.clone();
            cell.extend_from_slice(row);
            cells.push(cell);
        }
    }
    cells
}

/// One polygon clipped to one convex cell, half-plane by half-plane (Sutherland–Hodgman).
///
/// A vertex the clip keeps is the source's own pair, copied; only a vertex the clip creates is
/// arithmetic, and it is interpolated in user space along the source edge at the parameter the
/// display-space depths give. That is what keeps the surviving geometry the producer's numbers.
fn clip_to_cell(polygon: &[(f64, f64)], cell: &[HalfPlane], to_display: Mapping) -> SubPath {
    let mut current: SubPath = polygon.to_vec();
    for plane in cell {
        if current.len() < 3 {
            return Vec::new();
        }
        let depths: Vec<f64> = current
            .iter()
            .map(|point| plane.depth(to_display.apply(*point)))
            .collect();
        let mut out: SubPath = Vec::with_capacity(current.len().saturating_add(2));
        for index in 0..current.len() {
            let next = wrapped(index, current.len());
            let (here, there) = (current[index], current[next]);
            let (inside, beyond) = (depths[index], depths[next]);
            if inside >= 0.0 {
                out.push(here);
            }
            if (inside >= 0.0) != (beyond >= 0.0) {
                let span = inside - beyond;
                if span != 0.0 && span.is_finite() {
                    let t = inside / span;
                    out.push((
                        (there.0 - here.0).mul_add(t, here.0),
                        (there.1 - here.1).mul_add(t, here.1),
                    ));
                }
            }
        }
        current = out;
    }
    current
}

/// The index after `index` around a closed ring of `len` vertices.
fn wrapped(index: usize, len: usize) -> usize {
    let next = index.saturating_add(1);
    if next >= len { 0 } else { next }
}

/// Whether a polygon encloses any area at all — the shoelace sum, which is zero for a polygon
/// the clip degenerated to a line and for one of fewer than three vertices.
fn encloses_area(polygon: &[(f64, f64)]) -> bool {
    if polygon.len() < 3 {
        return false;
    }
    let mut twice = 0.0f64;
    for index in 0..polygon.len() {
        let here = polygon[index];
        let there = polygon[wrapped(index, polygon.len())];
        twice += here.0.mul_add(there.1, -(there.0 * here.1));
    }
    twice.abs() > 0.0
}

/// Writes the surviving polygons as §8.5.2 path construction operators — `m`, `l` and `h` — so
/// the geometry in the output is the cut geometry and nothing describes the removed marks.
pub(super) fn write_polygons(out: &mut String, polygons: &[SubPath]) {
    for polygon in polygons {
        for (index, point) in polygon.iter().enumerate() {
            write_coordinate(out, point.0);
            out.push(' ');
            write_coordinate(out, point.1);
            out.push_str(if index == 0 { " m" } else { " l" });
            out.push(' ');
        }
        out.push_str("h\n");
    }
}

/// Writes one coordinate as a §7.3.3 real: fixed point, never exponent notation, with trailing
/// zeros trimmed so a coordinate the source held as an integer is written as one.
fn write_coordinate(out: &mut String, value: f64) {
    use std::fmt::Write as _;
    if !value.is_finite() {
        out.push('0');
        return;
    }
    let text = format!("{value:.DECIMALS$}");
    let trimmed = if text.contains('.') {
        text.trim_end_matches('0').trim_end_matches('.')
    } else {
        text.as_str()
    };
    if trimmed.is_empty() || trimmed == "-" {
        out.push('0');
    } else {
        let _ = write!(out, "{trimmed}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const IDENTITY: Mapping = Mapping {
        a: 1.0,
        b: 0.0,
        c: 0.0,
        d: 1.0,
        e: 0.0,
        f: 0.0,
    };

    /// The shoelace area of a polygon, unsigned.
    fn area(polygon: &[(f64, f64)]) -> f64 {
        let mut twice = 0.0;
        for index in 0..polygon.len() {
            let here = polygon[index];
            let there = polygon[wrapped(index, polygon.len())];
            twice += here.0 * there.1 - there.0 * here.1;
        }
        twice.abs() / 2.0
    }

    fn total(polygons: &[SubPath]) -> f64 {
        polygons.iter().map(|polygon| area(polygon)).sum()
    }

    /// A square with a bite taken out of one corner keeps exactly the area outside the region.
    #[test]
    fn a_corner_bite_leaves_the_complement_s_area() {
        let square = vec![(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)];
        let cut = subtract(&[square], &[[5.0, 5.0, 20.0, 20.0]], IDENTITY).expect("inside budget");
        // 100 minus the 5×5 corner, and the pad widens the bite by a hundredth on two sides.
        let expected = 100.0 - (5.0 + REGION_PAD) * (5.0 + REGION_PAD);
        assert!(
            (total(&cut.polygons) - expected).abs() < 1e-9,
            "area {} vs {expected}",
            total(&cut.polygons)
        );
        assert!(cut.margin_holds());
    }

    /// A square wholly inside the region survives as nothing at all.
    #[test]
    fn a_path_inside_the_region_is_deleted_entirely() {
        let square = vec![(2.0, 2.0), (4.0, 2.0), (4.0, 4.0), (2.0, 4.0)];
        let cut = subtract(&[square], &[[0.0, 0.0, 10.0, 10.0]], IDENTITY).expect("inside budget");
        assert!(cut.polygons.is_empty());
    }

    /// A square the region does not reach keeps its own vertices, bit for bit.
    #[test]
    fn a_path_clear_of_the_region_keeps_its_vertices() {
        let square = vec![(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)];
        let cut = subtract(
            std::slice::from_ref(&square),
            &[[5.0, 5.0, 6.0, 6.0]],
            IDENTITY,
        )
        .expect("inside budget");
        assert_eq!(cut.polygons, vec![square]);
    }

    /// A region straight through the middle leaves two pieces and no bridge between them.
    #[test]
    fn a_band_across_the_middle_leaves_two_pieces() {
        let square = vec![(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)];
        let cut = subtract(&[square], &[[-1.0, 4.0, 11.0, 6.0]], IDENTITY).expect("inside budget");
        assert_eq!(cut.polygons.len(), 2, "one piece below, one above");
        let expected = 10.0 * (10.0 - 2.0 - 2.0 * REGION_PAD);
        assert!((total(&cut.polygons) - expected).abs() < 1e-9);
    }

    /// A ring — an outer square and an inner one — keeps both subpaths where the region misses,
    /// which is what an even-odd fill needs to still read as a hole.
    #[test]
    fn a_ring_keeps_both_of_its_subpaths() {
        let outer = vec![(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)];
        let inner = vec![(3.0, 3.0), (7.0, 3.0), (7.0, 7.0), (3.0, 7.0)];
        let cut = subtract(&[outer, inner], &[[20.0, 20.0, 30.0, 30.0]], IDENTITY)
            .expect("inside budget");
        assert_eq!(cut.polygons.len(), 2);
    }

    /// The mapping decides the cut: the same path under a scaling transform is cut where the
    /// region is in the display list's space, not where the numbers are in the path's.
    #[test]
    fn the_cut_is_taken_in_the_display_list_s_space() {
        let square = vec![(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)];
        let doubled = Mapping {
            a: 2.0,
            d: 2.0,
            ..IDENTITY
        };
        let cut = subtract(&[square], &[[10.0, -1.0, 30.0, 30.0]], doubled).expect("inside budget");
        // In user space the region covers x from 5 to 15, so the half at x < 5 survives.
        let expected = 10.0 * (5.0 - REGION_PAD / 2.0);
        assert!(
            (total(&cut.polygons) - expected).abs() < 1e-9,
            "area {}",
            total(&cut.polygons)
        );
    }

    /// A coordinate large enough that single precision cannot resolve the margin refuses.
    #[test]
    fn a_coordinate_too_large_for_the_margin_is_not_held() {
        let square = vec![(0.0, 0.0), (1.0e9, 0.0), (1.0e9, 1.0e9), (0.0, 1.0e9)];
        let cut =
            subtract(&[square], &[[5.0e8, -1.0, 2.0e9, 2.0e9]], IDENTITY).expect("inside budget");
        assert!(!cut.margin_holds(), "the margin cannot be proven here");
    }

    /// The operators written back are §8.5.2's, and a coordinate the source held as an integer
    /// is written as one.
    #[test]
    fn the_written_operators_are_a_closed_subpath() {
        let mut out = String::new();
        write_polygons(&mut out, &[vec![(1.0, 2.0), (3.5, 2.0), (3.5, 4.25)]]);
        assert_eq!(out, "1 2 m 3.5 2 l 3.5 4.25 l h\n");
    }
}

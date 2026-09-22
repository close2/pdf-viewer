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
//! Sutherland–Hodgman clip against a convex window: exact, one pass per half-plane, and
//! orientation-preserving, so a nonzero-winding fill and an even-odd fill both survive it (each
//! subpath is clipped on its own and the cells do not overlap, so no interior is wound twice and
//! no crossing is counted twice). The degenerate edges the algorithm lays along a window boundary
//! enclose no area and are invisible to either fill rule.
//!
//! # A §8.5.2.2 Bézier segment is cut, not flattened
//!
//! The clip's only question about a segment is *where it crosses the window's boundary line*, and
//! for a cubic that is a root rather than an approximation. A half-plane's depth is an affine
//! functional of the point and the mapping into the display list's space is affine, so the depth
//! along a segment is a polynomial whose Bernstein coefficients are the depths of its own control
//! points — degree one for a line, three for a cubic. [`crossings`] solves it, and the segment is
//! split at those parameters by de Casteljau ([`kurbo::ParamCurve::subsegment`]), which produces
//! sub-curves that lie on the source curve exactly. Nothing is flattened and no mark moves.
//!
//! # A §8.5.3.2 stroke is cut as the outline it marks
//!
//! "The S operator shall paint a line along the current path", and §8.5.3.2's marks are that
//! line's *outline*: a stroke is the region swept by a pen of the current line width, with the
//! caps and joins the graphics state names. So the removal is the same geometric cut applied to
//! the outline — [`crate::redact`] expands the stroke with `kurbo::stroke` and hands the result
//! here as a fill. What keeps that honest is [`is_polygonal`]: the expansion of a round cap, a
//! round join or a curved segment is an *approximation* of arcs, and replacing the producer's
//! marks outside the region with an approximation would be this program inventing a mark. The
//! outline is admitted only where it came back made of straight lines, which is the case the
//! expansion computes exactly, and every other stroke is refused by name (trap 5).
//!
//! # What keeps the survivors byte-exact
//!
//! A control point that survives is copied, never recomputed: the depth test is evaluated in the
//! display list's space, and the point it keeps is the source's own user-space pair. A segment
//! with no crossing crosses whole, so a subpath the cut does not reach comes out with the
//! producer's own numbers. Only a point the cut *creates* is arithmetic.
//!
//! # The margin, and the guard that makes it a proof
//!
//! A created point is written back as decimal text and read back by a processor whose real
//! numbers are single precision (§7.3.3). Both roundings can move a cut edge, and moving it
//! *into* the region would leave a sliver of the redacted marks alive. So the region is widened
//! by [`REGION_PAD`] before the cut — the removal reaches that much past the quad, which is the
//! safe direction — and [`Cut::margin_holds`] refuses the page unless the worst displacement the
//! two roundings can produce is strictly smaller than the widening. A margin nobody has checked
//! is not a margin. ADRs 1195, 1236.

use kurbo::{BezPath, ParamCurve, PathEl, PathSeg, Point, Shape};

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

/// A path's subpath: its segments in the content stream's own user space, implicitly closed.
///
/// A [`kurbo::BezPath`] rather than a vertex list because §8.5.2.2's `c`, `v` and `y` describe a
/// cubic and the cut is exact on one: the curve's own control points are what survive, and the
/// parameters it is split at are roots rather than samples.
pub(super) type SubPath = BezPath;

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
    fn apply(self, point: Point) -> Point {
        Point::new(
            self.a.mul_add(point.x, self.c.mul_add(point.y, self.e)),
            self.b.mul_add(point.x, self.d.mul_add(point.y, self.f)),
        )
    }

    /// The largest factor by which the linear part can stretch a displacement — the row sums of
    /// the absolute matrix, which bound the operator norm from above. A displacement `δ` in user
    /// space moves its image by at most `norm · δ`, which is what the margin guard needs.
    pub(super) fn norm(self) -> f64 {
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
    fn depth(self, at: Point) -> f64 {
        let value = match self.axis {
            Axis::X => at.x,
            Axis::Y => at.y,
        };
        if self.keep_above {
            value - self.bound
        } else {
            self.bound - value
        }
    }

    /// The same for a point of the path's own space.
    fn depth_of(self, at: Point, to_display: Mapping) -> f64 {
        self.depth(to_display.apply(at))
    }
}

/// What a cut produced, and the numbers the margin guard is checked against.
pub(super) struct Cut {
    /// The surviving subpaths, in the content stream's own user space.
    pub(super) polygons: Vec<SubPath>,
    /// The largest coordinate magnitude any created point carries, which decides how coarse the
    /// single-precision grid is where that point lands.
    largest: f64,
    /// The mapping's norm, which turns a user-space displacement into a display-space one.
    norm: f64,
}

impl Cut {
    /// Whether the rounding a created point will undergo stays inside [`REGION_PAD`].
    ///
    /// Two roundings displace a created point: this writer's own, which is half a unit in the
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

/// Whether a path is made of straight lines alone, which is when an expansion of it is exact.
///
/// The one question [`crate::redact`] asks of `kurbo::stroke`'s output: an outline that came back
/// with a curve in it is an approximation of an arc — a round cap, a round join, or the offset of
/// a curved segment — and cutting an approximation would replace the producer's marks outside the
/// region with marks this program computed. Asked of the **output** rather than of the graphics
/// state, so the guard is what the expansion actually produced rather than this tree's model of
/// when it produces it.
pub(super) fn is_polygonal(path: &BezPath) -> bool {
    path.elements().iter().all(|element| {
        matches!(
            element,
            PathEl::MoveTo(_) | PathEl::LineTo(_) | PathEl::ClosePath
        )
    })
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
        .filter(|path| encloses_area(path))
        .cloned()
        .collect();
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
    // Every point the cut created is one the source did not hold; the guard is checked against
    // the largest of them, because a single-precision ulp grows with the magnitude.
    let sources: std::collections::HashSet<(u64, u64)> =
        subpaths.iter().flat_map(control_points).collect();
    let largest = polygons
        .iter()
        .flat_map(control_points_with_values)
        .filter(|(key, _)| !sources.contains(key))
        .map(|(_, point)| point.x.abs().max(point.y.abs()))
        .fold(0.0f64, f64::max);
    Some(Cut {
        polygons,
        largest,
        norm: to_display.norm(),
    })
}

/// Every control point of a subpath, as the bit patterns the source set is compared by.
fn control_points(path: &SubPath) -> Vec<(u64, u64)> {
    control_points_with_values(path)
        .into_iter()
        .map(|(key, _)| key)
        .collect()
}

/// Every control point of a subpath, with the point beside its bit pattern.
fn control_points_with_values(path: &SubPath) -> Vec<((u64, u64), Point)> {
    let mut out = Vec::new();
    let mut push = |point: Point| out.push(((point.x.to_bits(), point.y.to_bits()), point));
    for element in path.elements() {
        match *element {
            PathEl::MoveTo(point) | PathEl::LineTo(point) => push(point),
            PathEl::QuadTo(one, two) => {
                push(one);
                push(two);
            }
            PathEl::CurveTo(one, two, three) => {
                push(one);
                push(two);
                push(three);
            }
            PathEl::ClosePath => {}
        }
    }
    out
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

/// One subpath clipped to one convex cell, half-plane by half-plane (Sutherland–Hodgman).
fn clip_to_cell(polygon: &SubPath, cell: &[HalfPlane], to_display: Mapping) -> SubPath {
    let mut current = polygon.clone();
    for plane in cell {
        if current.elements().is_empty() {
            return BezPath::new();
        }
        current = clip_to_half_plane(&current, *plane, to_display);
    }
    current
}

/// One subpath clipped to one half-plane.
///
/// Each segment is split where it crosses the boundary and the pieces on the kept side are
/// emitted in order; where a run of pieces was dropped, the next kept piece is reached by a
/// straight edge, whose two ends both lie on the boundary line. That edge is the one the
/// algorithm lays along the window and is what closes the surviving ring.
fn clip_to_half_plane(path: &SubPath, plane: HalfPlane, to_display: Mapping) -> SubPath {
    let mut out = BezPath::new();
    let mut previous_end: Option<Point> = None;
    for segment in path.segments() {
        for piece in split_at_crossings(segment, plane, to_display) {
            if plane.depth_of(piece.eval(0.5), to_display) < 0.0 {
                continue;
            }
            let start = piece.start();
            match previous_end {
                None => out.move_to(start),
                Some(end) if end != start => out.line_to(start),
                Some(_) => {}
            }
            match piece {
                PathSeg::Line(line) => out.line_to(line.p1),
                PathSeg::Quad(quad) => out.quad_to(quad.p1, quad.p2),
                PathSeg::Cubic(cubic) => out.curve_to(cubic.p1, cubic.p2, cubic.p3),
            }
            previous_end = Some(piece.end());
        }
    }
    if previous_end.is_some() {
        // The source's closing segment comes back as a `LineTo` onto the point the ring began
        // at, which `ClosePath` then draws again. Dropping it keeps the survivor's element list
        // the same shape as the source's, so a subpath the cut did not reach comes out with the
        // producer's own points and no more.
        let elements = out.elements();
        if elements.len() > 2
            && let (Some(PathEl::MoveTo(begin)), Some(PathEl::LineTo(end))) =
                (elements.first().copied(), elements.last().copied())
            && begin == end
        {
            let keep = elements.len().saturating_sub(1);
            out.truncate(keep);
        }
        out.close_path();
    }
    out
}

/// One segment split at every parameter where it meets the half-plane's boundary line.
///
/// A segment that does not meet it comes back whole, which is what keeps an untouched curve's
/// control points the producer's own bit patterns.
fn split_at_crossings(segment: PathSeg, plane: HalfPlane, to_display: Mapping) -> Vec<PathSeg> {
    let parameters = crossings(segment, plane, to_display);
    if parameters.is_empty() {
        return vec![segment];
    }
    let mut pieces = Vec::with_capacity(parameters.len().saturating_add(1));
    let mut from = 0.0;
    for to in parameters {
        pieces.push(segment.subsegment(from..to));
        from = to;
    }
    pieces.push(segment.subsegment(from..1.0));
    pieces
}

/// The parameters in `(0, 1)` at which a segment crosses a half-plane's boundary, sorted.
///
/// The depth is an affine functional of the point and the mapping is affine, so the depth along
/// the segment is a polynomial in the parameter whose **Bernstein** coefficients are the depths
/// of the segment's own control points. Converting those to the power basis and solving is
/// therefore the exact crossing rather than a search, for a line and for a §8.5.2.2 cubic alike.
fn crossings(segment: PathSeg, plane: HalfPlane, to_display: Mapping) -> Vec<f64> {
    let depth = |point: Point| plane.depth_of(point, to_display);
    let mut roots: Vec<f64> = match segment {
        PathSeg::Line(line) => {
            let (d0, d1) = (depth(line.p0), depth(line.p1));
            solve_linear(d0, d1 - d0)
        }
        PathSeg::Quad(quad) => {
            let (d0, d1, d2) = (depth(quad.p0), depth(quad.p1), depth(quad.p2));
            kurbo::common::solve_quadratic(
                d0,
                2.0f64.mul_add(d1, -(2.0 * d0)),
                d2 - 2.0f64.mul_add(d1, -d0),
            )
            .to_vec()
        }
        PathSeg::Cubic(cubic) => {
            let (d0, d1) = (depth(cubic.p0), depth(cubic.p1));
            let (d2, d3) = (depth(cubic.p2), depth(cubic.p3));
            kurbo::common::solve_cubic(
                d0,
                3.0 * (d1 - d0),
                3.0f64.mul_add(d2 - d1, -(3.0 * (d1 - d0))),
                d3 - d0 + 3.0 * (d1 - d2),
            )
            .to_vec()
        }
    };
    roots.retain(|t| *t > 0.0 && *t < 1.0 && t.is_finite());
    roots.sort_by(f64::total_cmp);
    roots.dedup();
    roots
}

/// The root of `c0 + c1·t`, as the solvers above answer: none where the line is parallel.
fn solve_linear(c0: f64, c1: f64) -> Vec<f64> {
    if c1 == 0.0 {
        return Vec::new();
    }
    vec![-c0 / c1]
}

/// Whether a subpath encloses any area at all — the signed area of the closed curve, which is
/// zero for one the clip degenerated to a line and for one with no segment.
fn encloses_area(path: &SubPath) -> bool {
    path.elements().len() >= 3 && path.area().abs() > 0.0
}

/// Writes the surviving subpaths as §8.5.2 path construction operators — `m`, `l`, `c` and `h` —
/// so the geometry in the output is the cut geometry and nothing describes the removed marks.
pub(super) fn write_polygons(out: &mut String, polygons: &[SubPath]) {
    for polygon in polygons {
        let mut current = Point::ZERO;
        for element in polygon.elements() {
            match *element {
                PathEl::MoveTo(point) => {
                    write_operator(out, &[point], "m");
                    current = point;
                }
                PathEl::LineTo(point) => {
                    write_operator(out, &[point], "l");
                    current = point;
                }
                // §8.5.2.2 gives a content stream cubics alone, so a quadratic is raised to the
                // cubic that draws the same curve: the exact degree elevation, whose inner
                // control points are two thirds of the way from each end towards the source's.
                PathEl::QuadTo(one, two) => {
                    write_operator(out, &[lifted(current, one), lifted(two, one), two], "c");
                    current = two;
                }
                // §8.5.2.2 states `v` and `y` as abbreviations of `c` with a repeated control
                // point; `c` spells every cubic, so one operator writes them all back.
                PathEl::CurveTo(one, two, three) => {
                    write_operator(out, &[one, two, three], "c");
                    current = three;
                }
                PathEl::ClosePath => out.push_str("h\n"),
            }
        }
        if !matches!(polygon.elements().last(), Some(PathEl::ClosePath)) {
            out.push_str("h\n");
        }
    }
}

/// One inner control point of the cubic that draws a quadratic exactly: two thirds of the way
/// from an end point towards the quadratic's own control point.
fn lifted(end: Point, control: Point) -> Point {
    Point::new(
        (control.x - end.x).mul_add(2.0 / 3.0, end.x),
        (control.y - end.y).mul_add(2.0 / 3.0, end.y),
    )
}

/// Writes one operator's points and its keyword.
fn write_operator(out: &mut String, points: &[Point], keyword: &str) {
    for point in points {
        write_coordinate(out, point.x);
        out.push(' ');
        write_coordinate(out, point.y);
        out.push(' ');
    }
    out.push_str(keyword);
    out.push('\n');
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

    /// A closed subpath through the given points.
    fn ring(points: &[(f64, f64)]) -> SubPath {
        let mut path = BezPath::new();
        for (index, (x, y)) in points.iter().enumerate() {
            let point = Point::new(*x, *y);
            if index == 0 {
                path.move_to(point);
            } else {
                path.line_to(point);
            }
        }
        path.close_path();
        path
    }

    fn total(polygons: &[SubPath]) -> f64 {
        polygons.iter().map(|path| path.area().abs()).sum()
    }

    /// A square with a bite taken out of one corner keeps exactly the area outside the region.
    #[test]
    fn a_corner_bite_leaves_the_complement_s_area() {
        let square = ring(&[(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)]);
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
        let square = ring(&[(2.0, 2.0), (4.0, 2.0), (4.0, 4.0), (2.0, 4.0)]);
        let cut = subtract(&[square], &[[0.0, 0.0, 10.0, 10.0]], IDENTITY).expect("inside budget");
        assert!(cut.polygons.is_empty());
    }

    /// A square the region does not reach keeps its own control points, bit for bit.
    #[test]
    fn a_path_clear_of_the_region_keeps_its_vertices() {
        let square = ring(&[(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)]);
        let cut = subtract(
            std::slice::from_ref(&square),
            &[[5.0, 5.0, 6.0, 6.0]],
            IDENTITY,
        )
        .expect("inside budget");
        assert_eq!(cut.polygons.len(), 1);
        assert_eq!(
            control_points(&cut.polygons[0]),
            control_points(&square),
            "every surviving point is the source's own"
        );
    }

    /// A region straight through the middle leaves two pieces and no bridge between them.
    #[test]
    fn a_band_across_the_middle_leaves_two_pieces() {
        let square = ring(&[(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)]);
        let cut = subtract(&[square], &[[-1.0, 4.0, 11.0, 6.0]], IDENTITY).expect("inside budget");
        assert_eq!(cut.polygons.len(), 2, "one piece below, one above");
        let expected = 10.0 * (10.0 - 2.0 - 2.0 * REGION_PAD);
        assert!((total(&cut.polygons) - expected).abs() < 1e-9);
    }

    /// A ring — an outer square and an inner one — keeps both subpaths where the region misses,
    /// which is what an even-odd fill needs to still read as a hole.
    #[test]
    fn a_ring_keeps_both_of_its_subpaths() {
        let outer = ring(&[(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)]);
        let inner = ring(&[(3.0, 3.0), (7.0, 3.0), (7.0, 7.0), (3.0, 7.0)]);
        let cut = subtract(&[outer, inner], &[[20.0, 20.0, 30.0, 30.0]], IDENTITY)
            .expect("inside budget");
        assert_eq!(cut.polygons.len(), 2);
    }

    /// The mapping decides the cut: the same path under a scaling transform is cut where the
    /// region is in the display list's space, not where the numbers are in the path's.
    #[test]
    fn the_cut_is_taken_in_the_display_list_s_space() {
        let square = ring(&[(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)]);
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
        let square = ring(&[(0.0, 0.0), (1.0e9, 0.0), (1.0e9, 1.0e9), (0.0, 1.0e9)]);
        let cut =
            subtract(&[square], &[[5.0e8, -1.0, 2.0e9, 2.0e9]], IDENTITY).expect("inside budget");
        assert!(!cut.margin_holds(), "the margin cannot be proven here");
    }

    /// The operators written back are §8.5.2's, and a coordinate the source held as an integer
    /// is written as one.
    #[test]
    fn the_written_operators_are_a_closed_subpath() {
        let mut out = String::new();
        write_polygons(&mut out, &[ring(&[(1.0, 2.0), (3.5, 2.0), (3.5, 4.25)])]);
        assert_eq!(out, "1 2 m\n3.5 2 l\n3.5 4.25 l\nh\n");
    }

    /// A §8.5.2.2 cubic is split at the root where it meets the region's edge, and the piece that
    /// survives is a curve rather than a chord: the curve's own arithmetic, not a flattening.
    ///
    /// The half-disc `y = 0` to a cubic arch over `x ∈ [0, 10]` is cut at `x = 5`. What the test
    /// pins is the property flattening would break — the surviving area is the cubic's exact
    /// integral, which [`kurbo::Shape::area`] computes in closed form, and the cut curve is still
    /// a `CurveTo`.
    #[test]
    fn a_cubic_is_split_at_its_root_and_stays_a_curve() {
        let mut arch = BezPath::new();
        arch.move_to(Point::new(0.0, 0.0));
        arch.curve_to(
            Point::new(0.0, 10.0),
            Point::new(10.0, 10.0),
            Point::new(10.0, 0.0),
        );
        arch.close_path();
        let whole = arch.area().abs();
        let cut = subtract(&[arch], &[[5.0, -1.0, 20.0, 20.0]], IDENTITY).expect("inside budget");
        assert_eq!(cut.polygons.len(), 1);
        assert!(
            cut.polygons[0]
                .elements()
                .iter()
                .any(|element| matches!(element, PathEl::CurveTo(..))),
            "the survivor is still a curve: {:?}",
            cut.polygons[0].elements()
        );
        // The arch is symmetric about x = 5, so half its area survives, less the pad's sliver.
        let survived = cut.polygons[0].area().abs();
        assert!(
            (survived - whole / 2.0).abs() < 0.2,
            "{survived} against {}",
            whole / 2.0
        );
        assert!(cut.margin_holds());
    }

    /// A curve clear of the region crosses whole, control points and all.
    #[test]
    fn a_curve_clear_of_the_region_is_not_split() {
        let mut arch = BezPath::new();
        arch.move_to(Point::new(0.0, 0.0));
        arch.curve_to(
            Point::new(0.0, 10.0),
            Point::new(10.0, 10.0),
            Point::new(10.0, 0.0),
        );
        arch.close_path();
        let cut = subtract(
            std::slice::from_ref(&arch),
            &[[50.0, 50.0, 60.0, 60.0]],
            IDENTITY,
        )
        .expect("inside budget");
        assert_eq!(control_points(&cut.polygons[0]), control_points(&arch));
    }

    /// The split is on the curve, not near it: every point of a piece is the source curve's own
    /// point at the corresponding parameter.
    ///
    /// This is what separates cutting a §8.5.2.2 Bézier from flattening one, and it is the
    /// property the pixel proof in `tests/redact.rs` rests on. De Casteljau's construction is
    /// exact in arithmetic, so what is checked here is that the parameters line up: the piece
    /// over `[0, t]`, evaluated at `u`, is the source evaluated at `t·u`.
    #[test]
    fn a_split_piece_lies_on_the_source_curve() {
        let source = kurbo::CubicBez::new(
            Point::new(0.0, 0.0),
            Point::new(0.0, 10.0),
            Point::new(10.0, 10.0),
            Point::new(10.0, 0.0),
        );
        let plane = HalfPlane {
            axis: Axis::X,
            bound: 5.0,
            keep_above: false,
        };
        let parameters = crossings(PathSeg::Cubic(source), plane, IDENTITY);
        assert_eq!(parameters.len(), 1, "{parameters:?}");
        let split = parameters[0];
        // The arch is symmetric about x = 5, so it meets that line at its own midpoint.
        assert!((split - 0.5).abs() < 1e-12, "{split}");
        let piece = PathSeg::Cubic(source).subsegment(0.0..split);
        for step in 0..=10 {
            let u = f64::from(step) / 10.0;
            let on_piece = piece.eval(u);
            let on_source = source.eval(split * u);
            assert!(
                (on_piece - on_source).hypot() < 1e-12,
                "{on_piece:?} against {on_source:?}"
            );
        }
    }

    /// [`is_polygonal`] is asked of an expansion's output, and it answers about the output.
    #[test]
    fn a_polygonal_outline_is_told_from_a_curved_one() {
        let square = ring(&[(0.0, 0.0), (1.0, 0.0), (1.0, 1.0)]);
        assert!(is_polygonal(&square));
        let mut curved = BezPath::new();
        curved.move_to(Point::new(0.0, 0.0));
        curved.curve_to(
            Point::new(1.0, 0.0),
            Point::new(1.0, 1.0),
            Point::new(0.0, 1.0),
        );
        curved.close_path();
        assert!(!is_polygonal(&curved));
    }
}

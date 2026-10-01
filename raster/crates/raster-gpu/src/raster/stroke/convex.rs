//! The convex pieces a stroke is cut into, and the one boolean operation the tiling needs of
//! them: a convex polygon less another, as convex fragments (ADR 1375).
//!
//! The difference of two convex polygons is a set of convex polygons with no boolean library
//! behind it: for `Q`'s edges in order, the part of `P` outside edge `j` and inside edges
//! `0 … j−1` is one convex fragment, and those fragments are disjoint and cover `P − Q`. A
//! fragment is `P` cut by half-planes, so it keeps `P`'s winding, and the two sides of every
//! cut share their crossing points to the bit because both are the one value computed once.

use raster_scene::Point;

use super::super::flatten::Polyline;
use super::sweep::Bounds;

/// A convex piece, with the lines its edges lie on.
pub(super) struct Convex {
    pub(super) points: Vec<Point>,
    pub(super) bounds: Bounds,
    /// `1.0` where the signed area is positive, `-1.0` where negative: which side of
    /// each edge is the inside.
    pub(super) orientation: f64,
    /// The line through each edge, oriented so that the piece is on its positive side.
    lines: Vec<Line>,
}

impl Convex {
    /// The piece, or `None` where it has no area to hold or to deposit.
    pub(super) fn new(piece: &Polyline) -> Option<Self> {
        let area = signed_area(&piece.points);
        if piece.points.len() < 3 || !area.is_finite() || area == 0.0 {
            return None;
        }
        let orientation = area.signum();
        Some(Self {
            points: piece.points.clone(),
            bounds: Bounds::of(&piece.points),
            orientation,
            lines: edges(&piece.points)
                .map(|(from, to)| Line::through(from, to, orientation))
                .collect(),
        })
    }

    /// Whether a line through an edge of either piece has the other wholly on its outside.
    /// An edge of no length has no line, and is not asked.
    pub(super) fn apart_from(&self, other: &Self) -> bool {
        let separates =
            |line: &Line, points: &[Point]| line.has_length() && line.holds_none_of(points);
        self.lines.iter().any(|line| separates(line, &other.points))
            || other.lines.iter().any(|line| separates(line, &self.points))
    }

    /// Each of `fragments` less this piece: a fragment whose box misses this piece's, or
    /// that a line through an edge of either separates from it, keeps its shape — cutting
    /// it along lines that pass beside it would only multiply its pieces. `true` where any
    /// fragment was cut.
    ///
    /// The fragments are rewritten in place through `scratch`'s buffers, in the order a
    /// fresh list would hold them: the tiling asks this millions of times on a tight page,
    /// and a list and a carried remainder allocated per question cost a quarter of its
    /// instructions (ADR 1421).
    pub(super) fn subtract_from_each(
        &self,
        fragments: &mut Vec<(Vec<Point>, Bounds)>,
        orientation: f64,
        scratch: &mut Scratch,
    ) -> bool {
        if !fragments
            .iter()
            .any(|(_, bounds)| bounds.meets(self.bounds))
        {
            return false;
        }
        let mut cut_any = false;
        let Scratch { cut, inside } = scratch;
        cut.clear();
        for (fragment, bounds) in fragments.drain(..) {
            let separated = !bounds.meets(self.bounds)
                || self.lines.iter().any(|line| line.holds_none_of(&fragment))
                || edges(&fragment).any(|(from, to)| {
                    Line::through(from, to, orientation).holds_none_of(&self.points)
                });
            if separated {
                cut.push((fragment, bounds));
            } else {
                self.subtract_from(fragment, orientation, inside, cut);
                cut_any = true;
            }
        }
        std::mem::swap(fragments, cut);
        cut_any
    }

    /// `fragment − self`, as convex fragments that are disjoint and keep `fragment`'s
    /// winding (`orientation`), appended to `out` with their boxes: for each of this piece's
    /// edges in turn, the part of what is left that lies outside the edge is emitted and the
    /// part inside is carried on. What is carried past the last edge lies inside this piece
    /// and is dropped, and so is a part too thin to be told from rounding ([`sliver`]).
    /// `inside` is a buffer for what is carried, kept for the next call.
    fn subtract_from(
        &self,
        fragment: Vec<Point>,
        orientation: f64,
        inside: &mut Vec<Point>,
        out: &mut Vec<(Vec<Point>, Bounds)>,
    ) {
        let (mut rest, mut carried) = (fragment, std::mem::take(inside));
        for line in &self.lines {
            match line.split(&rest, &mut carried) {
                // Wholly inside this edge: carried on as it is.
                Split::Inside => {}
                // Wholly outside it: emitted as it is, and nothing is left to carry.
                Split::Outside => {
                    if rest.len() >= 3 {
                        let bounds = Bounds::of(&rest);
                        if !sliver(&rest, bounds, orientation) {
                            out.push((rest, bounds));
                        }
                    }
                    *inside = carried;
                    return;
                }
                Split::Across(outside) => {
                    if outside.len() >= 3 {
                        let bounds = Bounds::of(&outside);
                        if !sliver(&outside, bounds, orientation) {
                            out.push((outside, bounds));
                        }
                    }
                    std::mem::swap(&mut rest, &mut carried);
                    if rest.len() < 3 {
                        break;
                    }
                }
            }
        }
        *inside = if rest.capacity() > carried.capacity() {
            rest
        } else {
            carried
        };
    }
}

/// The buffers one tiling's subtractions reuse.
#[derive(Default)]
pub(super) struct Scratch {
    /// The fragments being rebuilt.
    cut: Vec<(Vec<Point>, Bounds)>,
    /// What a subtraction carries past each edge.
    inside: Vec<Point>,
}

/// A polygon's edges, the closing one included.
#[expect(clippy::arithmetic_side_effects)] // `(i + 1) % len` with `len` non-zero
fn edges(points: &[Point]) -> impl Iterator<Item = (Point, Point)> + '_ {
    let len = points.len();
    (0..len).map(move |i| (points[i], points[(i + 1) % len]))
}

/// The line through an edge, as its start point and its direction turned to face the
/// inside of the polygon that edge belongs to.
///
/// A side is measured from the edge's own start point rather than from a constant term,
/// so a point that *is* a vertex of the edge measures exactly zero: two pieces that share
/// a corner to the bit are found to touch there, not to overlap by a rounding error.
#[derive(Debug, Clone, Copy)]
struct Line {
    from: (f64, f64),
    along: (f64, f64),
}

impl Line {
    fn through(from: Point, to: Point, orientation: f64) -> Self {
        Self {
            from: (f64::from(from.x), f64::from(from.y)),
            along: (
                orientation * (f64::from(to.x) - f64::from(from.x)),
                orientation * (f64::from(to.y) - f64::from(from.y)),
            ),
        }
    }

    /// Positive on the inside, zero on the line.
    fn side(&self, p: Point) -> f64 {
        self.along.0 * (f64::from(p.y) - self.from.1)
            - self.along.1 * (f64::from(p.x) - self.from.0)
    }

    /// Whether the edge the line was drawn through has a length, and so a direction.
    fn has_length(&self) -> bool {
        self.along.0 != 0.0 || self.along.1 != 0.0
    }

    /// Whether every point is outside this line or on it: a separating line.
    fn holds_none_of(&self, points: &[Point]) -> bool {
        points.iter().all(|p| self.side(*p) <= 0.0)
    }

    /// A convex polygon cut by this line: which side it lies on where it lies on one, and
    /// otherwise the part outside, with the part inside written to `inside`.
    ///
    /// A point exactly on the line belongs to both parts, so a polygon lying wholly on one
    /// side — touching the line or not — is that side's whole; a polygon on the line
    /// throughout is inside. Each crossing point is computed once and pushed to both parts,
    /// which is what makes the two meet edge to edge. A polygon on one side is not copied:
    /// the caller has it already (ADR 1421).
    fn split(&self, polygon: &[Point], inside: &mut Vec<Point>) -> Split {
        let (mut low, mut high) = (f64::INFINITY, f64::NEG_INFINITY);
        for p in polygon {
            let side = self.side(*p);
            low = low.min(side);
            high = high.max(side);
        }
        if low >= 0.0 {
            return Split::Inside;
        }
        if high <= 0.0 {
            return Split::Outside;
        }
        inside.clear();
        let mut outside = Vec::with_capacity(polygon.len().saturating_add(1));
        for (p, q) in edges(polygon) {
            let (sp, sq) = (self.side(p), self.side(q));
            if sp >= 0.0 {
                inside.push(p);
            }
            if sp <= 0.0 {
                outside.push(p);
            }
            if (sp > 0.0 && sq < 0.0) || (sp < 0.0 && sq > 0.0) {
                let t = sp / (sp - sq);
                #[expect(clippy::cast_possible_truncation)] // a point between two `f32` points
                let crossing = Point::new(
                    (f64::from(p.x) + t * (f64::from(q.x) - f64::from(p.x))) as f32,
                    (f64::from(p.y) + t * (f64::from(q.y) - f64::from(p.y))) as f32,
                );
                inside.push(crossing);
                outside.push(crossing);
            }
        }
        Split::Across(outside)
    }
}

/// Where a convex polygon lies against a line ([`Line::split`]).
enum Split {
    /// Wholly on the inside, or on the line.
    Inside,
    /// Wholly on the outside, touching the line or not.
    Outside,
    /// On both sides: the part outside, the part inside having been written out.
    Across(Vec<Point>),
}

/// Whether a fragment is narrower than the rounding that made it (ADR 1444).
///
/// A side is taken in `f64` from `f32` points, where its rounding is far below the `f32`
/// grid; but a crossing point is rounded back to `f32`, which moves it by at most half the
/// `f32` spacing at its magnitude on each axis, less than one spacing `δ = m · f32::EPSILON`
/// in all, `m` the fragment's largest coordinate. A fragment cut between crossing points that
/// round to nearly one point can therefore hold an area of that rounding's order with either
/// sign: a sliver along a cut, which is no part of the set and whose sign says nothing about
/// its winding.
///
/// Its measure is its width, in the coordinates' own units: a fragment whose area taken with
/// its piece's winding is at most `δ` times its box's perimeter — which a convex polygon's own
/// perimeter never exceeds — is nowhere wider than the rounding could have made it, and is
/// dropped. What that removes is at most `δ` along each side of the box, `6.1e-5` of a pixel
/// at a coordinate of a thousand, and it is what keeps every fragment wound its piece's way.
/// The box rather than the perimeter because the tiling asks this of every fragment it cuts,
/// and a square root per edge cost `bug1743245.pdf` 14% of a frame's instructions (ADR 1444).
/// Computed in `f64` (trap 67): in `f32` the product would round at the scale it measures.
fn sliver(points: &[Point], bounds: Bounds, orientation: f64) -> bool {
    let (x0, y0) = (f64::from(bounds.min.x), f64::from(bounds.min.y));
    let (x1, y1) = (f64::from(bounds.max.x), f64::from(bounds.max.y));
    let largest = x0.abs().max(y0.abs()).max(x1.abs()).max(y1.abs());
    let spacing = largest * f64::from(f32::EPSILON);
    orientation * signed_area(points) <= spacing * 2.0 * ((x1 - x0) + (y1 - y0))
}

/// The polygon's signed area, the shoelace sum halved — in `f64`, so that a piece's sign
/// is not a rounding error's.
fn signed_area(points: &[Point]) -> f64 {
    edges(points)
        .map(|(a, b)| f64::from(a.x) * f64::from(b.y) - f64::from(b.x) * f64::from(a.y))
        .sum::<f64>()
        / 2.0
}

//! The pieces of a tightly bent stroke, made disjoint so that each point of the set is
//! covered once (ADR 1375).
//!
//! [`fill`](super::super::fill) integrates winding over a pixel and clamps it afterwards,
//! so two pieces that overlap inside one pixel on the stroke's rim count the overlap
//! twice there (trap 58). [`inner_cut`](super::inner_cut) makes neighbouring pieces meet
//! edge to edge where the half-width allows it. Where the path bends more tightly than
//! the half-width it does not: the inner offset folds over itself, each segment's
//! rectangle reaches past the centre of the bend, and a run of rectangles overlaps in a
//! star whose points are the rim. ISO 32000-2 §8.4.3.2 says what the union of those
//! pieces is:
//!
//! > stroking a path shall entail painting all points whose perpendicular distance from
//! > the path in user space is less than or equal to half the line width
//!
//! — a set, in which a point painted twice is painted exactly as much as a point painted
//! once. This module states that set as a tiling instead of a covering: each piece loses
//! whatever the pieces before it already hold, `P_k − (P_0 ∪ … ∪ P_{k−1})`, which leaves
//! the union exactly as it was and no two fragments sharing any area.
//!
//! Every piece [`stroke_polylines`](super::stroke_polylines) emits is convex, and the
//! difference of two convex polygons is a set of convex polygons with no boolean library
//! behind it: for `Q`'s edges in order, the part of `P` outside edge `j` and inside edges
//! `0 … j−1` is one convex fragment, and those fragments are disjoint and cover `P − Q`.
//! A fragment is `P` cut by half-planes, so it keeps `P`'s winding, and the two sides of
//! every cut share their crossing points to the bit because both are the one value
//! computed once.
//!
//! Only the pieces that meet a tight bend take part — the segments and joins at a vertex
//! [`inner_cut`](super::inner_cut) declined, and whatever piece's box meets one of theirs;
//! the rest already meet their neighbours edge to edge. The work is quadratic in the pieces
//! that take part, so it is bounded: [`MAX_PIECES`] pieces in, [`MAX_FRAGMENTS`] fragments
//! out. Past either bound the subpath keeps its overlapping pieces — the same set, dearer
//! only in the rim pixels an overlap touches — because a width and a path are a document's
//! numbers and an allocation is not (CLAUDE.md principle 3).

use raster_scene::Point;

use super::super::flatten::Polyline;

/// The most pieces one subpath may bring to the tiling, and the most it may name as
/// meeting a tight bend: the tiling is quadratic in them. A thick curve's tight run is a
/// few pieces per flattened vertex — 65 for `stroke_set`'s hook at 8× — and a glyph
/// outline stroked wider than itself about a hundred and fifty.
const MAX_PIECES: usize = 1024;

/// The most fragments one subpath's tiling may produce before it is abandoned for the
/// overlapping pieces it came from.
const MAX_FRAGMENTS: usize = 65_536;

/// The pieces that meet a tight bend — those `at_a_tight_bend` names, and every piece whose
/// box meets one of theirs — re-cut so that no two of them share any area and their union
/// is unchanged; the rest as they came, since they meet their neighbours edge to edge
/// already. Or every piece as it came, where the tiling would pass [`MAX_PIECES`] or
/// [`MAX_FRAGMENTS`].
///
/// A piece with no area deposits nothing and holds nothing, and is dropped from the tiling.
pub(super) fn disjoint(pieces: Vec<Polyline>, at_a_tight_bend: &[bool]) -> Vec<Polyline> {
    if at_a_tight_bend.iter().filter(|seed| **seed).count() > MAX_PIECES {
        return pieces;
    }
    let convex: Vec<Option<Convex>> = pieces.iter().map(Convex::new).collect();
    let seeds: Vec<Bounds> = convex
        .iter()
        .zip(at_a_tight_bend)
        .filter_map(|(piece, seed)| piece.as_ref().filter(|_| *seed).map(|p| p.bounds))
        .collect();
    let tiled: Vec<&Convex> = convex
        .iter()
        .flatten()
        .filter(|piece| seeds.iter().any(|seed| seed.meets(piece.bounds)))
        .collect();
    if tiled.len() > MAX_PIECES {
        return pieces;
    }
    let mut out: Vec<Polyline> = Vec::with_capacity(pieces.len());
    for (k, piece) in tiled.iter().enumerate() {
        let mut fragments = vec![(piece.points.clone(), piece.bounds)];
        // Nearest first: a piece's neighbours hold most of what it shares, so taking them
        // away first leaves little for the rest to cut.
        for earlier in tiled[..k].iter().rev() {
            fragments = earlier.subtract_from_each(fragments, piece.orientation);
            if fragments.is_empty() || out.len().saturating_add(fragments.len()) > MAX_FRAGMENTS {
                break;
            }
        }
        if out.len().saturating_add(fragments.len()) > MAX_FRAGMENTS {
            return pieces;
        }
        out.extend(fragments.into_iter().map(|(points, _)| Polyline {
            points,
            closed: true,
        }));
    }
    out.extend(
        pieces
            .into_iter()
            .zip(&convex)
            .filter(|(_, piece)| {
                piece
                    .as_ref()
                    .is_some_and(|piece| !seeds.iter().any(|seed| seed.meets(piece.bounds)))
            })
            .map(|(piece, _)| piece),
    );
    out
}

/// An axis-aligned box round a piece, for skipping the pairs that cannot meet.
#[derive(Debug, Clone, Copy)]
struct Bounds {
    min: Point,
    max: Point,
}

impl Bounds {
    fn of(points: &[Point]) -> Self {
        let mut bounds = Self {
            min: Point::new(f32::INFINITY, f32::INFINITY),
            max: Point::new(f32::NEG_INFINITY, f32::NEG_INFINITY),
        };
        for p in points {
            bounds.min = Point::new(bounds.min.x.min(p.x), bounds.min.y.min(p.y));
            bounds.max = Point::new(bounds.max.x.max(p.x), bounds.max.y.max(p.y));
        }
        bounds
    }

    /// Whether the two boxes share any area. Boxes that only touch along an edge share
    /// none, which is how pieces that already meet edge to edge are passed over.
    fn meets(self, other: Self) -> bool {
        self.min.x < other.max.x
            && other.min.x < self.max.x
            && self.min.y < other.max.y
            && other.min.y < self.max.y
    }
}

/// A convex piece, with the lines its edges lie on.
struct Convex {
    points: Vec<Point>,
    bounds: Bounds,
    /// `1.0` where the signed area is positive, `-1.0` where negative: which side of
    /// each edge is the inside.
    orientation: f64,
    /// The line through each edge, oriented so that the piece is on its positive side.
    lines: Vec<Line>,
}

impl Convex {
    /// The piece, or `None` where it has no area to hold or to deposit.
    fn new(piece: &Polyline) -> Option<Self> {
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

    /// Each of `fragments` less this piece: a fragment whose box misses this piece's, or
    /// that a line through an edge of either separates from it, keeps its shape — cutting
    /// it along lines that pass beside it would only multiply its pieces.
    fn subtract_from_each(
        &self,
        fragments: Vec<(Vec<Point>, Bounds)>,
        orientation: f64,
    ) -> Vec<(Vec<Point>, Bounds)> {
        if !fragments
            .iter()
            .any(|(_, bounds)| bounds.meets(self.bounds))
        {
            return fragments;
        }
        let mut cut = Vec::with_capacity(fragments.len());
        for (fragment, bounds) in fragments {
            let separated = !bounds.meets(self.bounds)
                || self.lines.iter().any(|line| line.holds_none_of(&fragment))
                || edges(&fragment).any(|(from, to)| {
                    Line::through(from, to, orientation).holds_none_of(&self.points)
                });
            if separated {
                cut.push((fragment, bounds));
            } else {
                cut.extend(self.subtract_from(fragment).into_iter().map(|points| {
                    let bounds = Bounds::of(&points);
                    (points, bounds)
                }));
            }
        }
        cut
    }

    /// `fragment − self`, as convex fragments that are disjoint and keep `fragment`'s
    /// winding: for each of this piece's edges in turn, the part of what is left that
    /// lies outside the edge is emitted and the part inside is carried on. What is
    /// carried past the last edge lies inside this piece and is dropped.
    fn subtract_from(&self, fragment: Vec<Point>) -> Vec<Vec<Point>> {
        let mut out = Vec::new();
        let (mut rest, mut inside) = (fragment, Vec::new());
        for line in &self.lines {
            let outside = line.split(&rest, &mut inside);
            if outside.len() >= 3 {
                out.push(outside);
            }
            std::mem::swap(&mut rest, &mut inside);
            if rest.len() < 3 {
                break;
            }
        }
        out
    }
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

    /// Whether every point is outside this line or on it: a separating line.
    fn holds_none_of(&self, points: &[Point]) -> bool {
        points.iter().all(|p| self.side(*p) <= 0.0)
    }

    /// A convex polygon cut by this line: the part outside is returned and the part
    /// inside written to `inside`.
    ///
    /// A point exactly on the line belongs to both parts, so a polygon lying wholly on one
    /// side — touching the line or not — comes back whole on that side and empty on the
    /// other. Each crossing point is computed once and pushed to both parts, which is what
    /// makes the two meet edge to edge.
    fn split(&self, polygon: &[Point], inside: &mut Vec<Point>) -> Vec<Point> {
        inside.clear();
        let (mut low, mut high) = (f64::INFINITY, f64::NEG_INFINITY);
        for p in polygon {
            let side = self.side(*p);
            low = low.min(side);
            high = high.max(side);
        }
        if low >= 0.0 {
            inside.extend_from_slice(polygon);
            return Vec::new();
        }
        if high <= 0.0 {
            return polygon.to_vec();
        }
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
        outside
    }
}

/// The polygon's signed area, the shoelace sum halved — in `f64`, so that a piece's sign
/// is not a rounding error's.
fn signed_area(points: &[Point]) -> f64 {
    edges(points)
        .map(|(a, b)| f64::from(a.x) * f64::from(b.y) - f64::from(b.x) * f64::from(a.y))
        .sum::<f64>()
        / 2.0
}

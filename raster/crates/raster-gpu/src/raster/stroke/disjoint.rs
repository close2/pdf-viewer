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
pub(super) fn disjoint(pieces: Vec<Polyline>, at_a_tight_bend: &[bool]) -> Tiled {
    let untouched = |pieces| Tiled {
        pieces,
        whole: false,
    };
    if at_a_tight_bend.iter().filter(|seed| **seed).count() > MAX_PIECES {
        return untouched(pieces);
    }
    let convex: Vec<Option<Convex>> = pieces.iter().map(Convex::new).collect();
    let seeds: Vec<Bounds> = convex
        .iter()
        .zip(at_a_tight_bend)
        .filter_map(|(piece, seed)| piece.as_ref().filter(|_| *seed).map(|p| p.bounds))
        .collect();
    // Asked once per piece rather than once for the tiling and again for the rest.
    let takes_part: Vec<bool> = convex
        .iter()
        .map(|piece| {
            piece
                .as_ref()
                .is_some_and(|piece| seeds.iter().any(|seed| seed.meets(piece.bounds)))
        })
        .collect();
    let tiled: Vec<&Convex> = convex
        .iter()
        .zip(&takes_part)
        .filter_map(|(piece, takes_part)| piece.as_ref().filter(|_| *takes_part))
        .collect();
    if tiled.len() > MAX_PIECES {
        return untouched(pieces);
    }
    // The boxes side by side, for the scan below that reads nothing else of most pieces.
    let boxes: Vec<Bounds> = tiled.iter().map(|piece| piece.bounds).collect();
    let mut scratch = Scratch::default();
    let mut fragments = Vec::new();
    let mut out: Vec<Polyline> = Vec::with_capacity(pieces.len());
    for (k, piece) in tiled.iter().enumerate() {
        fragments.clear();
        fragments.push((piece.points.clone(), piece.bounds));
        // The box round what is left of this piece. An earlier piece whose box misses it
        // misses every fragment's box, which lies inside it, so it would hand the fragments
        // back unchanged ([`Convex::subtract_from_each`]) and is passed over without asking
        // (ADR 1421).
        let mut reach = piece.bounds;
        // Nearest first: a piece's neighbours hold most of what it shares, so taking them
        // away first leaves little for the rest to cut.
        for j in (0..k).rev() {
            if boxes[j].meets(reach)
                && tiled[j].subtract_from_each(&mut fragments, piece.orientation, &mut scratch)
            {
                reach = Bounds::round(fragments.iter().map(|(_, bounds)| *bounds));
            }
            if fragments.is_empty() || out.len().saturating_add(fragments.len()) > MAX_FRAGMENTS {
                break;
            }
        }
        if out.len().saturating_add(fragments.len()) > MAX_FRAGMENTS {
            return untouched(pieces);
        }
        out.extend(
            fragments
                .drain(..)
                .map(|(points, _)| Polyline::polygon(points)),
        );
    }
    out.extend(
        pieces
            .into_iter()
            .zip(&convex)
            .zip(&takes_part)
            .filter(|((_, piece), takes_part)| piece.is_some() && !**takes_part)
            .map(|((piece, _), _)| piece),
    );
    let whole = tiled.len() == convex.iter().flatten().count()
        || the_rest_stand_apart(&convex, &takes_part);
    Tiled { pieces: out, whole }
}

/// Whether every piece that took no part in the tiling shares no area with any other piece,
/// so that with the tiled fragments, which share none among themselves, no point of the
/// subpath is inside two pieces (ADR 1421).
///
/// Asked of the pieces as they came: a fragment lies inside the piece it was cut from, so a
/// piece apart from that piece is apart from every fragment of it. Two convex pieces are
/// apart where a line through an edge of either has the other wholly on its outside — a
/// neighbour meeting edge to edge is, on the edge they share. The pairs are found by a sweep
/// along `x`, and past [`MAX_FRAGMENTS`] box comparisons the question answers no, which
/// leaves it to the fill (ADR 1389) as before.
#[expect(clippy::arithmetic_side_effects)] // a count below `MAX_FRAGMENTS` plus a length
fn the_rest_stand_apart(convex: &[Option<Convex>], takes_part: &[bool]) -> bool {
    let pieces: Vec<(bool, &Convex)> = convex
        .iter()
        .zip(takes_part)
        .filter_map(|(piece, takes_part)| piece.as_ref().map(|piece| (*takes_part, piece)))
        .collect();
    let mut order: Vec<usize> = (0..pieces.len()).collect();
    order.sort_unstable_by(|&a, &b| {
        pieces[a]
            .1
            .bounds
            .min
            .x
            .total_cmp(&pieces[b].1.bounds.min.x)
    });
    let (mut active, mut tests) = (Vec::<usize>::new(), 0_usize);
    for k in order {
        let (tiled_k, piece_k) = pieces[k];
        active.retain(|&m| pieces[m].1.bounds.max.x > piece_k.bounds.min.x);
        tests += active.len();
        if tests > MAX_FRAGMENTS {
            return false;
        }
        for &m in &active {
            let (tiled_m, piece_m) = pieces[m];
            // Two tiled pieces' fragments share nothing by construction.
            if !(tiled_k && tiled_m)
                && piece_k.bounds.meets(piece_m.bounds)
                && !piece_k.apart_from(piece_m)
            {
                return false;
            }
        }
        active.push(k);
    }
    true
}

/// What [`disjoint`] made of a subpath's pieces.
pub(super) struct Tiled {
    /// The pieces, the tiled ones re-cut.
    pub(super) pieces: Vec<Polyline>,
    /// Whether no point is inside two of the pieces, so that they tile the subpath's set:
    /// every piece with an area took part in the tiling, or those that did not stand apart
    /// from every other piece.
    pub(super) whole: bool,
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

    /// The box round `boxes`, or an empty box (one that meets nothing) where there are none.
    fn round(boxes: impl Iterator<Item = Self>) -> Self {
        boxes.fold(
            Self {
                min: Point::new(f32::INFINITY, f32::INFINITY),
                max: Point::new(f32::NEG_INFINITY, f32::NEG_INFINITY),
            },
            |a, b| Self {
                min: Point::new(a.min.x.min(b.min.x), a.min.y.min(b.min.y)),
                max: Point::new(a.max.x.max(b.max.x), a.max.y.max(b.max.y)),
            },
        )
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

    /// Whether a line through an edge of either piece has the other wholly on its outside.
    /// An edge of no length has no line, and is not asked.
    fn apart_from(&self, other: &Self) -> bool {
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
    fn subtract_from_each(
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
                self.subtract_from(fragment, inside, cut);
                cut_any = true;
            }
        }
        std::mem::swap(fragments, cut);
        cut_any
    }

    /// `fragment − self`, as convex fragments that are disjoint and keep `fragment`'s
    /// winding, appended to `out` with their boxes: for each of this piece's edges in turn,
    /// the part of what is left that lies outside the edge is emitted and the part inside is
    /// carried on. What is carried past the last edge lies inside this piece and is dropped.
    /// `inside` is a buffer for what is carried, kept for the next call.
    fn subtract_from(
        &self,
        fragment: Vec<Point>,
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
                        out.push((rest, bounds));
                    }
                    *inside = carried;
                    return;
                }
                Split::Across(outside) => {
                    if outside.len() >= 3 {
                        let bounds = Bounds::of(&outside);
                        out.push((outside, bounds));
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
struct Scratch {
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

/// The polygon's signed area, the shoelace sum halved — in `f64`, so that a piece's sign
/// is not a rounding error's.
fn signed_area(points: &[Point]) -> f64 {
    edges(points)
        .map(|(a, b)| f64::from(a.x) * f64::from(b.y) - f64::from(b.x) * f64::from(a.y))
        .sum::<f64>()
        / 2.0
}

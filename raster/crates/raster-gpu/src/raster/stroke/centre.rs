//! The path a stroke is drawn along, and the directions its pieces are square to (ADR 1397).
//!
//! ISO 32000-2 §8.4.3.4 draws a join "at the corners of paths that are stroked", where
//! "consecutive segments of a path connect at an angle", and each of Table 54's three
//! shapes is built from the two segments' own strokes at that point: the miter extends the
//! "outer edges of the strokes for the two segments" until they meet, the round join is an
//! arc "connecting the outer edges of the strokes for the two segments", and for the bevel
//! the "two segments shall be finished with butt caps". A segment's stroke at its
//! end runs along the segment's own direction there, and a curve's is its tangent; §8.4.3.3
//! squares a butt cap off "at the endpoint of the path" by the same direction. Flattening
//! keeps a curve's points and loses that direction, so [`flatten`](mod@super::super::flatten)
//! records it at every point where a curve begins or ends, and this module hands each piece
//! the direction its end is square to: the tangent where the end chord can carry it
//! ([`carried`]), the chord everywhere else.
//!
//! Where two pieces meet at a corner their inner sides overlap, and [`inner_cut`] finds the
//! point where their inner edges cross, so that each is cut back to it and the two meet edge
//! to edge (ADR 1361).

use raster_scene::Point;

use super::super::flatten::Polyline;
use super::{direction, distance};

/// A subpath as the stroker walks it: its points with exact repeats removed, whether it is
/// closed, and the path's own direction arriving at and leaving each point where a curve
/// decides one.
pub(super) struct Centre {
    pub(super) points: Vec<Point>,
    pub(super) closed: bool,
    /// Per point, the direction a curve arrives along; empty where no curve does anywhere.
    arriving: Vec<Option<Point>>,
    /// Per point, the direction a curve leaves along; empty where no curve does anywhere.
    leaving: Vec<Option<Point>>,
    /// Per point, whether it lies inside one curve rather than where a segment of the path
    /// begins or ends; empty where no curve does anywhere.
    inside: Vec<bool>,
}

impl Centre {
    /// `polyline` with coincident neighbours (and the closing repeat, when closed) removed —
    /// so that flattening artefacts cannot produce zero-length pieces — and its tangents
    /// carried to the points that remain; `None` for a lone point, which is degenerate and
    /// pre-split upstream (§8.5.3.2).
    ///
    /// Where a run of coincident points becomes one, the direction arriving is the first
    /// one's (where the path arrived) and the one leaving is the last one's (where it went on),
    /// and the point is inside a curve only where every one of the run is.
    ///
    /// A curve records a [`Tangent`](super::super::flatten::Tangent) where it begins and where
    /// it ends and nothing between, so the points after a leaving direction and before the
    /// next arriving one are the one curve's own flattening ([`Centre::inside_a_curve`]).
    #[expect(clippy::arithmetic_side_effects)] // indices below the lengths just built
    pub(super) fn of(polyline: &Polyline) -> Option<Self> {
        let curved = !polyline.tangents.is_empty();
        let mut points: Vec<Point> = Vec::with_capacity(polyline.points.len());
        let (mut arriving, mut leaving, mut inside) = (Vec::new(), Vec::new(), Vec::new());
        let mut tangents = polyline.tangents.iter().peekable();
        let mut in_a_curve = false;
        for (i, &p) in polyline.points.iter().enumerate() {
            let tangent = tangents.next_if(|t| t.at == i);
            let within = in_a_curve && tangent.is_none();
            #[expect(clippy::float_cmp)] // exact: a zero-length piece, not a near one
            if points.last().is_none_or(|q| q.x != p.x || q.y != p.y) {
                points.push(p);
                if curved {
                    arriving.push(None);
                    leaving.push(None);
                    inside.push(within);
                }
            } else if curved {
                inside[points.len() - 1] &= within;
            }
            if let Some(tangent) = tangent {
                let at = points.len() - 1;
                if arriving[at].is_none() {
                    arriving[at] = tangent.arriving;
                }
                if tangent.leaving.is_some() {
                    leaving[at] = tangent.leaving;
                }
                in_a_curve = tangent.leaving.is_some();
            }
        }
        #[expect(clippy::float_cmp)]
        if polyline.closed
            && points.len() > 1
            && points[0].x == points[points.len() - 1].x
            && points[0].y == points[points.len() - 1].y
        {
            points.pop();
            // The path arrives back at its start along whatever arrived at the repeat.
            if let Some(back) = arriving.pop().flatten() {
                arriving[0] = Some(back);
            }
            leaving.pop();
            inside.pop();
        }
        (points.len() >= 2).then_some(Self {
            points,
            closed: polyline.closed,
            arriving,
            leaving,
            inside,
        })
    }

    /// Whether point `j` lies inside one curve's flattening, where no two segments of the path
    /// meet.
    ///
    /// ISO 32000-2 §8.4.3.4: "Join styles shall be significant only at points where
    /// consecutive segments of a path connect at an angle". A curve is one segment, and the
    /// points its flattening adds are no corner of the path; what is drawn there is
    /// §8.4.3.2's set of points within the half-width of the chords, which at a point between
    /// two chords is the disc round it — a round join, whatever join the stroke names
    /// (ADR 1455).
    pub(super) fn inside_a_curve(&self, j: usize) -> bool {
        self.inside.get(j).copied().unwrap_or(false)
    }

    /// How many segments the subpath has: one fewer than its points when open, one per
    /// point when closed.
    #[expect(clippy::arithmetic_side_effects)] // at least two points
    pub(super) fn segments(&self) -> usize {
        if self.closed {
            self.points.len()
        } else {
            self.points.len() - 1
        }
    }

    /// Segment `i`'s two points.
    #[expect(clippy::arithmetic_side_effects)] // `i` below the segment count
    pub(super) fn segment(&self, i: usize) -> (Point, Point) {
        (self.points[i], self.points[(i + 1) % self.points.len()])
    }

    /// The segments arriving at and leaving point `j`, where it is a corner of the path:
    /// every point of a closed subpath, every point but the two ends of an open one.
    #[expect(clippy::arithmetic_side_effects)] // `j` below the point count, at least two
    pub(super) fn meeting(&self, j: usize) -> Option<(usize, usize)> {
        let n = self.points.len();
        let joined = self.closed || (j > 0 && j + 1 < n);
        joined.then(|| ((j + n - 1) % n, j % self.segments()))
    }

    /// Every segment's [`Turned`] ends, less those at a point where the curve turns no
    /// more than its own flattening does.
    ///
    /// §8.4.3.4 makes a join "significant only at points where consecutive segments of a
    /// path connect at an angle". Where one curve continues another along the same tangent,
    /// or turns by less than the chords either side of the point already do, the point is
    /// no corner the flattening can tell from its own: it is drawn as every point inside a
    /// flattened curve is, square to the chords and joined between them, and the tangent
    /// ends are kept for the corners the chords understate (ADR 1397).
    pub(super) fn turned_ends(&self, hw: f32) -> Vec<Turned> {
        let mut turned: Vec<Turned> = (0..self.segments()).map(|i| self.turned(i, hw)).collect();
        for j in 0..self.points.len() {
            let Some((before, after)) = self.meeting(j) else {
                continue;
            };
            let (end, start) = (turned[before].end, turned[after].start);
            if end.is_none() && start.is_none() {
                continue;
            }
            let (a, v) = self.segment(before);
            let (_, b) = self.segment(after);
            let (c1, c2) = (direction(a, v), direction(v, b));
            if angle(end.unwrap_or(c1), start.unwrap_or(c2)) <= angle(c1, c2) {
                turned[before].end = None;
                turned[after].start = None;
            }
        }
        turned
    }

    /// The directions segment `i`'s piece is squared off to at its two ends: the tangent a
    /// curve arrives or leaves along, where the chord can carry it, else `None` for the
    /// chord's own.
    ///
    /// Where both ends of one chord turn — a curve flattened to a single chord — each is
    /// given half of it, so the two turned ends cannot pass one another.
    #[expect(clippy::arithmetic_side_effects)] // `i` below the segment count
    fn turned(&self, i: usize, hw: f32) -> Turned {
        let (a, b) = self.segment(i);
        let to = (i + 1) % self.points.len();
        let start = self.leaving.get(i).copied().flatten();
        let end = self.arriving.get(to).copied().flatten();
        let share = if start.is_some() && end.is_some() {
            0.5
        } else {
            1.0
        };
        Turned {
            start: start.and_then(|t| carried(t, a, b, hw, share)),
            end: end.and_then(|t| carried(t, a, b, hw, share)),
        }
    }
}

/// The angle between two unit directions, from `0` to `π`.
fn angle(d1: Point, d2: Point) -> f32 {
    (d1.x * d2.y - d1.y * d2.x)
        .abs()
        .atan2(d1.x * d2.x + d1.y * d2.y)
}

/// The unit directions one segment's piece is squared off to, where they are not its chord's.
#[derive(Debug, Clone, Copy, Default)]
pub(super) struct Turned {
    pub(super) start: Option<Point>,
    pub(super) end: Option<Point>,
}

/// The unit direction `tangent` gives the end of a piece whose chord runs from `a` to `b`,
/// or `None` where the chord cannot carry it and the end stays square to the chord.
///
/// Square to the last chord, the line a curve's stroke ends on is turned by half the
/// chord's angle, one corner past the end and the other short of it. Turning the piece's
/// end edge by that angle keeps the piece a simple quadrilateral while its far corners stay
/// ahead of the turned ones, which is `hw · tan θ ≤ share · |ab|` — `share` being the part
/// of the chord this end may use: a chord shorter than that (a curve bent more tightly than
/// the half-width, at a coarse flattening) keeps the chord's square end, and so does a
/// tangent pointing back along the chord (`cos θ ≤ 0`, a cusp at the end).
fn carried(tangent: Point, a: Point, b: Point, hw: f32, share: f32) -> Option<Point> {
    let u = direction(Point::new(0.0, 0.0), tangent);
    let d = direction(a, b);
    let (cos, sin) = (u.x * d.x + u.y * d.y, (u.x * d.y - u.y * d.x).abs());
    (cos > 0.0 && hw * sin <= distance(a, b) * share * cos).then_some(u)
}

/// Where the inner edges of two pieces meeting at a vertex cross, and on which side of the
/// stroke that is (`left`: the side of the left normal).
#[derive(Debug, Clone, Copy)]
pub(super) struct InnerCut {
    pub(super) point: Point,
    pub(super) left: bool,
}

/// One piece as the cut sees it: its two points and the unit directions its two ends are
/// square to.
#[derive(Debug, Clone, Copy)]
pub(super) struct Side {
    pub(super) from: Point,
    pub(super) to: Point,
    pub(super) start: Point,
    pub(super) end: Point,
}

/// The inner cut where `incoming` meets `outgoing` at `incoming.to`, or `None` where the
/// two pieces must overlap instead (ADR 1361).
///
/// Two pieces meeting at an angle overlap on the inner side of the turn, in the kite between
/// the vertex and the point where their inner edges cross. Under [`fill`](super::super::fill)'s
/// accumulation an overlap costs nothing inside the stroke, where the winding is clamped, but
/// in a pixel on the stroke's rim it is **counted twice**. Cutting both pieces along the line
/// from the vertex to that crossing makes them meet edge to edge instead, and the part each
/// loses is inside the other, so the union — §8.4.3.2's set — is unchanged.
///
/// The crossing is only a corner of both pieces while it lies in the half of each inner
/// edge nearest the vertex: past that, the cuts at an edge's two ends could meet, and a curve
/// that bends more tightly than the half-width is exactly where they do. Such a vertex is left
/// uncut, and the pieces that meet it are tiled by [`disjoint`](mod@super::disjoint) instead
/// (ADR 1375).
///
/// Between two pieces square to their chords the inner edges are the chords' offsets, and
/// the crossing lies `t = hw · tan(θ / 2)` back along each from the vertex, `θ` the turn;
/// that is computed as such. Where either piece has an end turned to a curve's tangent its
/// inner edge runs between its own offset corners, and the crossing of the two edges is
/// found as the intersection of those two lines (ADR 1397).
pub(super) fn inner_cut(incoming: Side, outgoing: Side, hw: f32) -> Option<InnerCut> {
    let (d1, d2) = (incoming.end, outgoing.start);
    let cross = d1.x * d2.y - d1.y * d2.x;
    if cross == 0.0 {
        return None;
    }
    // The inner side is the side the path turns towards: the left normal's side when
    // `cross > 0`, which is the join's gap on the other side.
    let left = cross > 0.0;
    let square = |side: Side| {
        let chord = direction(side.from, side.to);
        #[expect(clippy::float_cmp)] // exact: a piece square to its chord to the bit
        let same = |u: Point| u.x == chord.x && u.y == chord.y;
        same(side.start) && same(side.end)
    };
    let point = if square(incoming) && square(outgoing) {
        chord_cut(incoming, outgoing, hw, cross, left)?
    } else {
        edge_crossing(incoming, outgoing, hw, left)?
    };
    Some(InnerCut { point, left })
}

/// [`inner_cut`] between two pieces square to their chords.
fn chord_cut(incoming: Side, outgoing: Side, hw: f32, cross: f32, left: bool) -> Option<Point> {
    let (d1, d2, v) = (incoming.end, outgoing.start, incoming.to);
    let dot = d1.x * d2.x + d1.y * d2.y;
    // `tan(θ / 2) = sin θ / (1 + cos θ)`; a reversal has no crossing at all.
    let t = hw * cross.abs() / (1.0 + dot);
    let reach = 0.5 * distance(incoming.from, v).min(distance(v, outgoing.to));
    if !t.is_finite() || t > reach {
        return None;
    }
    let side = if left { hw } else { -hw };
    Some(Point::new(
        v.x - d1.y * side - d1.x * t,
        v.y + d1.x * side - d1.y * t,
    ))
}

/// [`inner_cut`] where an end is turned: the crossing of the two pieces' inner edges, each
/// the line between the piece's own offset corners, in `f64`, and only within the half of
/// each edge nearest the vertex.
#[expect(clippy::cast_possible_truncation)] // a point between two `f32` points
fn edge_crossing(incoming: Side, outgoing: Side, hw: f32, left: bool) -> Option<Point> {
    let side = if left { f64::from(hw) } else { -f64::from(hw) };
    // A point `p` moved `side` along the left normal of the unit direction `d`.
    let offset = |p: Point, d: Point| {
        (
            f64::from(p.x) - f64::from(d.y) * side,
            f64::from(p.y) + f64::from(d.x) * side,
        )
    };
    let (p1, q1) = (
        offset(incoming.from, incoming.start),
        offset(incoming.to, incoming.end),
    );
    let (p2, q2) = (
        offset(outgoing.from, outgoing.start),
        offset(outgoing.to, outgoing.end),
    );
    let (along_in, along_out) = ((q1.0 - p1.0, q1.1 - p1.1), (q2.0 - p2.0, q2.1 - p2.1));
    let between = (p2.0 - p1.0, p2.1 - p1.1);
    let denominator = along_in.0 * along_out.1 - along_in.1 * along_out.0;
    // How far along each edge, from its start, the two lines cross.
    let on_in = (between.0 * along_out.1 - between.1 * along_out.0) / denominator;
    let on_out = (between.0 * along_in.1 - between.1 * along_in.0) / denominator;
    let within = on_in.is_finite()
        && on_out.is_finite()
        && (0.5..=1.0).contains(&on_in)
        && (0.0..=0.5).contains(&on_out);
    within.then(|| {
        Point::new(
            (p1.0 + on_in * along_in.0) as f32,
            (p1.1 + on_in * along_in.1) as f32,
        )
    })
}

/// Whether the path changes direction at a vertex at all, arriving along `d1` and leaving
/// along `d2`, other than straight back: a vertex [`inner_cut`] declines for that reason is
/// not a tight bend.
pub(super) fn turns(d1: Point, d2: Point) -> bool {
    d1.x * d2.y - d1.y * d2.x != 0.0
}

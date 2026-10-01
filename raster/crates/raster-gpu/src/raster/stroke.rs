//! Stroking: a resolved stroke becomes closed polygons, to be filled non-zero.
//!
//! One thing: ISO 32000-2 §8.4.3's geometry. **A stroke is not coverage and this
//! module produces none** — it takes [`Polyline`]s from [`flatten`](mod@super::flatten)
//! and hands back [`Polyline`]s, one closed polygon per segment, per join (§8.4.3.4)
//! and per cap (§8.4.3.3), for [`fill`](super::fill) to rasterise under
//! [`Rule::NonZero`](super::Rule). The pieces need no boolean union, and that is the
//! whole reason the expansion may be this simple. §8.4.3.2 says what their union is:
//!
//! > stroking a path shall entail painting all points whose perpendicular distance from
//! > the path in user space is less than or equal to half the line width
//!
//! Two invariants make the pieces add up to that set rather than to something near it
//! (ADR 1361):
//!
//! - **Every piece is wound the way every other is**, on both turn directions. A piece
//!   wound against the body cancels it where they overlap and subtracts from it in a
//!   pixel they share, which is the defect [`cap_fan`] states and [`join_at`] orders its
//!   corners against; `stroke_set`'s fixtures draw every shape both ways to hold it.
//! - **Pieces meet edge to edge wherever they can**, because [`fill`](super::fill)
//!   integrates winding over a pixel and clamps it afterwards: an overlap is free inside
//!   the stroke but counted twice in a rim pixel. A segment's piece is cut at the inner
//!   side of each join ([`inner_cut`]) and the outer side's join starts on the piece's own
//!   end points. Where a curve bends more tightly than the half-width the cuts would
//!   cross; there the pieces that meet the bend are re-cut into a tiling instead
//!   ([`disjoint`](mod@disjoint), ADR 1375).
//!
//! A piece's ends, its caps and its joins are square to the path's own direction at each
//! point, which where a curve begins or ends is its tangent rather than its last chord
//! ([`centre`](mod@centre), ADR 1389, ADR 1397).

use raster_scene::{LineCap, LineJoin, Point, Stroke};

use super::flatten::{DeviceTransform, Polyline, convex};

mod centre;
mod convex;
mod disjoint;
mod sweep;

use centre::{Centre, InnerCut, Side, Turned, inner_cut, turns};
use disjoint::Subpath;

/// The device width a stroke resolves to under a placement (ADR 0085).
///
/// ISO 32000-2 §8.4.3.2:
///
/// > A line width of 0 shall denote the thinnest line that can be rendered at device
/// > resolution: 1 device pixel wide.
///
/// And §10.7.5's automatic stroke adjustment, where the stroke asked for it: a width
/// under half a device pixel is drawn at one. The comparison is the caller's own —
/// their `Stroke::device_width` asks `width < 0.5 · (1 / stretch)` in path space,
/// which is `width · stretch < 0.5` here — and the substituted width is exactly one
/// device pixel where theirs is `(1 / stretch) · stretch`, the same value to within an
/// ulp (ADR 0082's contract covers the ulp).
pub(crate) fn resolve_width(stroke: Stroke, t: DeviceTransform) -> f32 {
    let stretch = t.max_stretch();
    if !stretch.is_finite() || stretch <= 0.0 {
        return stroke.width.max(1.0);
    }
    let device = stroke.width * stretch;
    if stroke.width <= 0.0 || (stroke.adjust && device < 0.5) {
        1.0
    } else {
        device
    }
}

/// Expand a stroke into closed polygons (ISO 32000-2 §8.4.3: a piece per segment,
/// §8.4.3.4's joins at interior vertices, §8.4.3.3's caps at open ends), all wound one
/// way, for filling with the non-zero rule.
///
/// The device width arrives from [`resolve_width`] (ADR 0085: §8.4.3.2's zero and
/// §10.7.5's adjustment applied at encode); dashing is already applied and degenerate
/// subpaths pre-split upstream (section 4.5 of the brief); consecutive coincident points are
/// skipped here so flattening artefacts cannot produce zero-length pieces.
///
/// The fixtures' form: the encoder reads [`stroke_pieces`], which says besides whether the
/// pieces tile the set (ADR 1445).
#[cfg(test)]
pub(crate) fn stroke_polylines(
    polylines: &[Polyline],
    stroke: Stroke,
    device_width: f32,
) -> Vec<Polyline> {
    stroke_pieces(polylines, stroke, device_width).pieces
}

/// A stroke's pieces, and whether they tile its set by construction.
pub(crate) struct Stroked {
    /// The closed polygons, all wound one way.
    pub pieces: Vec<Polyline>,
    /// Whether no point is inside two pieces, so that the fill winds every point `0` or one
    /// value and its integral is the set's area in every pixel. A subpath vouches for its own
    /// pieces where it is one straight segment — its body and its two caps, which share their
    /// corners to the bit — or closed and convex with every corner cut, whose pieces are the
    /// convex ring's decomposition into a strip per edge and a sector per corner (ADR 1397),
    /// or bent so tightly that the tiling re-cut every one of its pieces (ADR 1421); a stroke
    /// of several subpaths where each does and no piece of one overlaps a piece of another,
    /// or where the tiling of the whole stroke re-cut the pieces that do (ADR 1431).
    /// `false` says only that the stroker cannot vouch for it.
    pub tiles: bool,
}

/// [`stroke_polylines`], saying besides whether the pieces tile the set ([`Stroked::tiles`]).
pub(crate) fn stroke_pieces(polylines: &[Polyline], stroke: Stroke, device_width: f32) -> Stroked {
    let hw = device_width * 0.5;
    let subpaths: Vec<Subpath> = polylines
        .iter()
        .filter_map(Centre::of)
        .map(|centre| stroke_subpath(&centre, stroke, hw))
        .collect();
    let (pieces, tiles) = disjoint::tile_stroke(subpaths);
    Stroked { pieces, tiles }
}

/// One subpath's pieces, beside each whether it meets a tight bend, and whether they tile
/// the subpath's set by construction ([`Stroked::tiles`]) before any tiling.
#[expect(clippy::arithmetic_side_effects)] // indices below the point count, at least two
fn stroke_subpath(centre: &Centre, stroke: Stroke, hw: f32) -> Subpath {
    let pts = &centre.points;
    let n = pts.len();
    let segment_count = centre.segments();
    // Each segment's piece as the cuts and joins see it: its ends and the directions they
    // are square to, the curve's tangent where the chord can carry it (ADR 1397).
    let turned: Vec<Turned> = centre.turned_ends(hw);
    let sides: Vec<Side> = (0..segment_count)
        .map(|i| {
            let (from, to) = centre.segment(i);
            let chord = direction(from, to);
            Side {
                from,
                to,
                start: turned[i].start.unwrap_or(chord),
                end: turned[i].end.unwrap_or(chord),
            }
        })
        .collect();
    let meeting = |j: usize| centre.meeting(j);
    // Each vertex's inner cut, computed once so that the two pieces meeting there share its
    // point to the bit (ADR 1361); and which vertices turn without one, a bend tighter than
    // the half-width (ADR 1375).
    let mut tight = vec![false; n];
    let cuts: Vec<Option<InnerCut>> = (0..n)
        .map(|j| {
            let (before, after) = meeting(j)?;
            let cut = inner_cut(sides[before], sides[after], hw);
            tight[j] = cut.is_none() && turns(sides[before].end, sides[after].start);
            cut
        })
        .collect();
    // The pieces, and beside each whether it meets a tight bend.
    let (mut pieces, mut at_a_tight_bend) = (Vec::new(), Vec::new());
    // One piece per segment, less those another segment's piece already holds.
    let held = held_by_a_neighbour(pts, centre.closed, segment_count);
    let left_normal = |d: Point| Point::new(-d.y * hw, d.x * hw);
    for i in (0..segment_count).filter(|&i| !held[i]) {
        let (from, to) = (i, (i + 1) % n);
        let ends = PieceEnds {
            start: cuts[from],
            end: cuts[to],
            start_normal: turned[i].start.map(left_normal),
            end_normal: turned[i].end.map(left_normal),
        };
        pieces.push(segment_piece(pts[from], pts[to], hw, ends));
        at_a_tight_bend.push(tight[from] || tight[to]);
    }
    // A join at every corner, between the directions the two pieces end square to; inside a
    // curve, where no segments meet, and where a curve meets a segment along its own tangent,
    // where they meet at no angle, the disc that is the set there (ADR 1455, ADR 1468).
    for j in 0..n {
        let Some((before, after)) = meeting(j) else {
            continue;
        };
        let join = if centre.inside_a_curve(j) || centre.tangent_continuous(j) {
            LineJoin::Round
        } else {
            stroke.join
        };
        join_at(
            &mut pieces,
            pts[j],
            (sides[before].end, sides[after].start),
            hw,
            join,
            stroke.miter_limit,
        );
        at_a_tight_bend.resize(pieces.len(), tight[j]);
    }
    // Caps at open ends, square to the path's own direction there (§8.4.3.3; ADR 1389).
    if !centre.closed {
        let (first, last) = (sides[0], sides[segment_count - 1]);
        cap_at(
            &mut pieces,
            pts[0],
            Point::new(-first.start.x, -first.start.y),
            hw,
            stroke.cap,
        );
        cap_at(&mut pieces, pts[n - 1], last.end, hw, stroke.cap);
        at_a_tight_bend.resize(pieces.len(), false);
    }
    let tiles = if centre.closed {
        !held.contains(&true)
            && !tight.contains(&true)
            && (0..n)
                .all(|j| cuts[j].is_some() || !turns(sides[(j + n - 1) % n].end, sides[j].start))
            && convex(pts)
    } else {
        segment_count == 1
    };
    // Where the path bends more tightly than the half-width the pieces overlap in a star
    // whose points are the rim, and the fill would count each overlap twice there; the
    // tiling ([`disjoint::tile_stroke`]) holds the same set with every point covered once.
    //
    // **Kept although the fill now asks for the set (ADR 1389), because that question has a
    // bound and the tiling does not need it** (ADR 1407). Untiled, the hook and the tight L
    // are drawn to the byte as tiled; but a stroke of hundreds of pieces in a few pixels
    // spends the fill's sweep before it is answered, and the integral then counts every
    // overlap — all 205 such strokes of `bug1743245.pdf`, up to 184 levels too dark, and 19
    // corpus pages past a sixteenth. Every lane that integrates without the set, the GPU
    // triangles among them, takes these same pieces.
    Subpath {
        pieces,
        at_a_tight_bend,
        tiles,
    }
}

/// The unit vector from `a` to `b`, or the zero vector when there is no direction to
/// give — which is a piece the caller has already established is not zero-length.
///
/// # Why the second computation exists
///
/// `dx * dx` is not the length of anything on its own, and it leaves the representable
/// range at both ends long before `dx` does: it overflows to infinity above `1.9e19`
/// and underflows to zero below `1.1e-22`. **Both ends are inside the scene contract.**
/// `MAX_COORDINATE` is `1e9` on an outline point *and* on a transform coefficient, and a
/// device delta is a composed transform applied to such a point, so `dx` reaches `1e27`;
/// at the other end nothing bounds a delta from below except the float grid, and
/// `stroke_polylines`' dedupe compares coordinates for *equality*, which two points
/// `1e-30` apart pass.
///
/// Neither end used to be caught, and neither failed quietly at the same place:
/// - `1e27` gave `len = inf`, so the normal was `(0, 0)`, so the stroke's quad was
///   degenerate and the mark **deposited no ink at all**;
/// - `1e-30` gave `len = 0`, so the normal was `(NaN, ±inf)`, and `fill_mask`'s prefix
///   sum carries a NaN to the end of its row where `abs().min(1.0)` returns **1.0** for
///   it — a fully painted row across the tile. CLAUDE.md's rule is that a document's
///   numbers must "never produce NaN geometry"; this was the one place in the tree that
///   did.
///
/// `hypot` is the same length without the square and is exact at both ends. It is the
/// *second* path rather than the only one because it is a libm call on the hottest
/// stroke loop there is — the caller's reading of hayro #630 is the standing warning
/// about exactly that — and because every segment the fast path already handles must
/// keep the arithmetic it had, to the bit, so that no page of the corpus moves.
fn direction(a: Point, b: Point) -> Point {
    let (dx, dy) = (b.x - a.x, b.y - a.y);
    let len = (dx * dx + dy * dy).sqrt();
    if len.is_finite() && len > 0.0 {
        return Point::new(dx / len, dy / len);
    }
    let len = dx.hypot(dy);
    if len.is_finite() && len > 0.0 {
        Point::new(dx / len, dy / len)
    } else {
        // `a` and `b` are the same point to the last bit, or one of them is not finite —
        // neither of which `stroke_polylines` hands us, since it dedupes exact repeats
        // and its input is bounded by `MAX_COORDINATE`. A zero direction makes every
        // piece built from it degenerate, which deposits nothing (§8.5.3.2's degenerate
        // subpath), rather than letting a NaN into the accumulator.
        Point::new(0.0, 0.0)
    }
}

/// The length of `a → b`, by the same two computations as [`direction`] and for the
/// same reason: a square that leaves `f32` must not decide a comparison.
fn distance(a: Point, b: Point) -> f32 {
    let (dx, dy) = (b.x - a.x, b.y - a.y);
    let len = (dx * dx + dy * dy).sqrt();
    if len.is_finite() && len > 0.0 {
        len
    } else {
        dx.hypot(dy)
    }
}

/// Which segments' pieces lie wholly inside a neighbour's, and are therefore not emitted.
///
/// A path that turns exactly back on itself runs its second segment along the first one's
/// line, so the shorter of the two rectangles is a subset of the longer — the out-and-back
/// outline a closed two-point subpath is, above all (`0 0 m 10 0 l s`). The set is the
/// longer rectangle either way; emitting both would count the shared part twice in every
/// rim pixel, for the reason [`inner_cut`] gives. Only an exact reversal qualifies — a turn
/// the arithmetic cannot tell from straight back is one [`join_at`] also treats as one —
/// and of two equal segments the one with the lower index goes, which leaves the last
/// segment of any chain of reversals standing, so nothing a piece held is lost.
#[expect(clippy::arithmetic_side_effects)] // indices below `pts.len()`, which is at least 2
fn held_by_a_neighbour(pts: &[Point], closed: bool, segment_count: usize) -> Vec<bool> {
    let mut held = vec![false; segment_count];
    let vertices = if closed {
        0..pts.len()
    } else {
        1..pts.len() - 1
    };
    for v in vertices {
        let (before, after) = ((v + segment_count - 1) % segment_count, v % segment_count);
        let prev = pts[(v + pts.len() - 1) % pts.len()];
        let next = pts[(v + 1) % pts.len()];
        let (d1, d2) = (direction(prev, pts[v]), direction(pts[v], next));
        let turned_back = d1.x * d2.y - d1.y * d2.x == 0.0 && d1.x * d2.x + d1.y * d2.y < 0.0;
        if !turned_back || before == after {
            continue;
        }
        let (l1, l2) = (distance(prev, pts[v]), distance(pts[v], next));
        let shorter = if l1 < l2 {
            before
        } else if l2 < l1 {
            after
        } else {
            before.min(after)
        };
        held[shorter] = true;
    }
    held
}

/// How a segment's piece ends at each of its two points: an inner cut where a join
/// has one, and the normal of the path's own direction there where a curve decides it
/// and it is not the chord's.
#[derive(Debug, Clone, Copy)]
struct PieceEnds {
    start: Option<InnerCut>,
    end: Option<InnerCut>,
    start_normal: Option<Point>,
    end_normal: Option<Point>,
}

/// The piece one segment contributes: its rectangle (§8.4.3.2's points within the
/// half-width, across the segment's own length), with the inner corner at either end
/// replaced by that vertex's [`InnerCut`] where it has one.
///
/// Where a curve begins or ends the end is squared off to the curve's own direction rather
/// than to the chord ([`Centre::turned_ends`], ADR 1389, ADR 1397): the rectangle becomes
/// the quadrilateral whose end edge is the cap's or the join's line, and the piece still
/// meets the cap or the join edge to edge.
///
/// The points are visited in the rectangle's own order — left side forward, right side
/// back — so a cut piece is wound as every other piece is. At a cut end the vertex
/// itself is inserted between the inner point and the uncut half of the end, because
/// that half is the edge the join on the outer side shares.
fn segment_piece(a: Point, b: Point, hw: f32, ends: PieceEnds) -> Polyline {
    let n = normal(a, b, hw);
    let (na, nb) = (ends.start_normal.unwrap_or(n), ends.end_normal.unwrap_or(n));
    let (a_left, a_right) = (
        Point::new(a.x + na.x, a.y + na.y),
        Point::new(a.x - na.x, a.y - na.y),
    );
    let (b_left, b_right) = (
        Point::new(b.x + nb.x, b.y + nb.y),
        Point::new(b.x - nb.x, b.y - nb.y),
    );
    let mut points = Vec::with_capacity(6);
    match ends.start {
        Some(cut) if cut.left => points.push(cut.point),
        _ => points.push(a_left),
    }
    match ends.end {
        Some(cut) if cut.left => points.extend([cut.point, b, b_right]),
        Some(cut) => points.extend([b_left, b, cut.point]),
        None => points.extend([b_left, b_right]),
    }
    match ends.start {
        Some(cut) if cut.left => points.extend([a_right, a]),
        Some(cut) => points.extend([cut.point, a]),
        None => points.push(a_right),
    }
    Polyline::polygon(points)
}

/// The left normal of `a → b`, scaled to the half-width.
fn normal(a: Point, b: Point, hw: f32) -> Point {
    let d = direction(a, b);
    Point::new(-d.y * hw, d.x * hw)
}

/// The join at `v` (§8.4.3.4) between a piece arriving along the unit direction `d1` and
/// one leaving along `d2` — each the direction that piece's end is square to, a curve's
/// tangent where it has one (ADR 1397) — filling the gap on the outer side of the turn.
fn join_at(
    out: &mut Vec<Polyline>,
    v: Point,
    (d1, d2): (Point, Point),
    hw: f32,
    join: LineJoin,
    miter_limit: f32,
) {
    let cross = d1.x * d2.y - d1.y * d2.x;
    if cross == 0.0 {
        // Straight on, the pieces already meet edge to edge. Turned straight back, a
        // round join's arc is a half turn round `v` beyond the first segment's end —
        // the shape of a round cap there, and [`cap_fan`] is built for exactly pi — while
        // a miter is past any limit and its bevel has no area.
        if matches!(join, LineJoin::Round) && d1.x * d2.x + d1.y * d2.y < 0.0 {
            out.push(cap_fan(v, d1, Point::new(-d1.y * hw, d1.x * hw), hw));
        }
        return;
    }
    // The gap opens on the side away from the turn.
    let s = if cross > 0.0 { -1.0 } else { 1.0 };
    let n1 = Point::new(-d1.y * hw * s, d1.x * hw * s);
    let n2 = Point::new(-d2.y * hw * s, d2.x * hw * s);
    let p1 = Point::new(v.x + n1.x, v.y + n1.y);
    let p2 = Point::new(v.x + n2.x, v.y + n2.y);
    // The order the two outer corners are visited in is what winds the piece, and it
    // must be the quads' order on both turns (ADR 1361). A quad `a+n, b+n, b−n, a−n`
    // with `n` the left normal has the same orientation whatever its direction; the
    // wedge `v, p1, p2` has it when the gap is on the left (`cross < 0`) and the
    // opposite one when the gap is on the right, because flipping `s` mirrors the
    // wedge along with its side. Visiting `p2` first on that turn undoes the mirror.
    let (first, second) = if cross > 0.0 { (p2, p1) } else { (p1, p2) };
    match join {
        LineJoin::Bevel => out.push(Polyline::polygon(vec![v, first, second])),
        LineJoin::Miter => {
            // §8.4.3.5: the miter stands until length/width exceeds the limit; then
            // the join is a bevel. Ratio = 1 / cos(half-angle), via the unit normals.
            let dot = (n1.x * n2.x + n1.y * n2.y) / (hw * hw);
            let denom = 1.0 + dot;
            let ratio_sq = 2.0 / denom.max(f32::EPSILON);
            if ratio_sq <= miter_limit * miter_limit {
                let scale = 1.0 / denom.max(f32::EPSILON);
                let m = Point::new(v.x + (n1.x + n2.x) * scale, v.y + (n1.y + n2.y) * scale);
                out.push(Polyline::polygon(vec![v, first, m, second]));
            } else {
                out.push(Polyline::polygon(vec![v, first, second]));
            }
        }
        LineJoin::Round => {
            out.push(arc_fan(v, first, second, hw));
        }
    }
}

fn cap_at(out: &mut Vec<Polyline>, end: Point, dir: Point, hw: f32, cap: LineCap) {
    // `dir` points outward, away from the stroked segment.
    let n = Point::new(-dir.y * hw, dir.x * hw);
    match cap {
        LineCap::Butt => {}
        LineCap::Square => out.push(Polyline::polygon(vec![
            Point::new(end.x + n.x, end.y + n.y),
            Point::new(end.x + n.x + dir.x * hw, end.y + n.y + dir.y * hw),
            Point::new(end.x - n.x + dir.x * hw, end.y - n.y + dir.y * hw),
            Point::new(end.x - n.x, end.y - n.y),
        ])),
        // §8.4.3.3, Table 53: "[a] semicircular arc with a diameter equal to the line
        // width shall be drawn around the endpoint and shall be filled in."
        LineCap::Round => out.push(cap_fan(end, dir, n, hw)),
    }
}

/// The semicircle a round cap is, swept from one side of the stroke round **through
/// `dir`** to the other — `dir` pointing away from the segment, as [`cap_at`] takes it.
///
/// Built from `dir` rather than from the two endpoint angles, and that is the whole
/// point. A cap sweeps **exactly pi**, and an arc stated by its endpoints alone has two
/// readings at exactly pi that no "shorter way round" rule can separate: [`arc_fan`] took
/// whichever way `atan2`'s branch cut happened to give, which was the outward semicircle
/// at the end of a subpath and the *inward* one at its start.
///
/// An inward semicircle is not merely invisible. It lies inside the stroke body and is
/// wound **against** it, so the non-zero rule (§10.7.4's fill, `fill_mask`) cancels the
/// two and punches a hole of exactly the area the far cap adds. Both ends of every
/// round-capped subpath were wrong, and the two errors were equal and opposite: the
/// caller's ink-total instrument read a round cap as depositing exactly what a butt cap
/// does (`QUORRA_FEEDBACK.md` section 21.1), which is the sum, not the picture.
///
/// The chords are [`arc_steps`]', as for any other arc, taken in pairs through
/// [`arc_waist`]'s point, as for any other arc.
///
/// Its two corners are `end ± n`, the very points the segment's own piece ends on,
/// rather than the same points recomputed through `cos` and `sin`: a fan that meets
/// the body a rounding error away leaves a sliver of rim counted once or twice.
fn cap_fan(end: Point, dir: Point, n: Point, hw: f32) -> Polyline {
    // `cap_at`'s `n` is `dir` turned a quarter turn, so the cap's two corners sit at
    // `base ± pi/2` and the outward point at `base`. Sweeping downward from `+pi/2`
    // passes through `base`, which is what makes this the outward half — and gives the
    // fan the stroke body's own winding, so it adds rather than cancels.
    let base = dir.y.atan2(dir.x);
    let pairs = arc_steps(std::f32::consts::PI, hw).div_ceil(2);
    let waist = hw * arc_waist(std::f32::consts::PI, pairs);
    let halves = pairs.saturating_mul(2);
    let mut points = Vec::with_capacity(halves.saturating_add(2));
    points.push(end);
    points.push(Point::new(end.x + n.x, end.y + n.y));
    for i in 1..halves {
        #[expect(clippy::cast_precision_loss)] // twice MAX_ARC_STEPS at the most
        let t = base + std::f32::consts::FRAC_PI_2
            - std::f32::consts::PI * (i as f32) / (halves as f32);
        let r = if i % 2 == 1 { waist } else { hw };
        points.push(Point::new(end.x + r * t.cos(), end.y + r * t.sin()));
    }
    points.push(Point::new(end.x - n.x, end.y - n.y));
    Polyline::polygon(points)
}

/// The coarsest angle one step of an arc advances. Deterministic (brief section 4.6),
/// and the step [`arc_steps`] takes for any arc of radius under 16 device pixels.
const ARC_STEP: f32 = 0.35;

/// The most steps one arc of up to a half turn is cut into: a stroke's width is a
/// document's number, and the step count is an allocation (CLAUDE.md principle 3).
/// At this count an arc is within the tolerance up to a radius of about 13 000 device
/// pixels; a wider one is drawn coarser than that, and no finer.
const MAX_ARC_STEPS: usize = 256;

/// How many steps an arc of `sweep` radians and radius `radius` is cut into, so that no
/// step's chord would fall further inside the arc than
/// [`FLATTEN_TOLERANCE`](super::flatten::FLATTEN_TOLERANCE) — §10.7.2's bound, the one
/// every other curve here is flattened to (ADR 1361). The chords are then taken in pairs, each
/// pair a step of twice the angle drawn through [`arc_waist`]'s point, which encloses the
/// step's sector where one chord a step fell inside it — the same vertex count (ADR 1443).
///
/// A chord of angle `a` falls `r · (1 − cos(a / 2))` inside, which is at most
/// `r · a² / 8`; so `a = sqrt(8 · tolerance / r)` is within the bound, and `sqrt` is
/// correctly rounded, so the count is the same on every machine. A fixed angle is not:
/// its sag grows with the radius, and at 0.35 a round join of 64 device pixels fell a
/// whole pixel short of its arc.
fn arc_steps(sweep: f32, radius: f32) -> usize {
    let bound = (8.0 * super::flatten::FLATTEN_TOLERANCE / radius).sqrt();
    let step = if bound.is_finite() && bound > 0.0 {
        bound.min(ARC_STEP)
    } else {
        ARC_STEP
    };
    // Clamped to `MAX_ARC_STEPS` before the conversion, which a `usize` holds exactly.
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_precision_loss
    )]
    let steps = (sweep.abs() / step).ceil().min(MAX_ARC_STEPS as f32) as usize;
    steps.max(1)
}

/// How far out, in units of the radius, the point halfway along one step of an arc stands,
/// so that the two chords through it enclose the step's own sector (ADR 1443).
///
/// A step of angle `a` is a sector of area `r² · a / 2`. The triangles from the centre to
/// its two ends and a point at radius `ρ` on its bisector hold `r · ρ · sin(a / 2)`, which is
/// the sector's where `ρ = r · (a / 2) / sin(a / 2)` — a hair past the arc, `r · a² / 24` at
/// the most, where the one chord fell `r · a² / 8` inside it. §10.7.4: "[t]he area covered by
/// painted pixels shall always be at least as large as the area of the original shape", and
/// one chord per step is an inscribed polygon that takes the arc's segments out of it.
fn arc_waist(sweep: f32, steps: usize) -> f32 {
    #[expect(clippy::cast_precision_loss)] // steps is at most MAX_ARC_STEPS
    let half = sweep.abs() / (steps as f32) * 0.5;
    let sin = half.sin();
    if sin > 0.0 { half / sin } else { 1.0 }
}

/// A fan of points approximating the arc from `from` to `to` around `centre` (both on
/// the circle of radius `radius`), as one closed polygon including the centre: [`arc_steps`]'
/// chords in pairs, each pair through [`arc_waist`]'s point, which encloses the pair's own
/// sector (ADR 1443).
///
/// **The caller must guarantee a sweep of less than pi**, because that is what makes
/// "the shorter way round" below name one arc rather than two. [`join_at`] is the only
/// caller and it does: it returns before this on `cross == 0.0`, so the two segments are
/// never collinear and never a reversal, and the gap a join fills is strictly under a
/// half turn. A cap *is* exactly a half turn and has [`cap_fan`] for that reason.
fn arc_fan(centre: Point, from: Point, to: Point, radius: f32) -> Polyline {
    let a0 = (from.y - centre.y).atan2(from.x - centre.x);
    let a1 = (to.y - centre.y).atan2(to.x - centre.x);
    let mut sweep = a1 - a0;
    // Take the shorter way round: a join or cap never sweeps more than pi.
    if sweep > std::f32::consts::PI {
        sweep -= 2.0 * std::f32::consts::PI;
    } else if sweep < -std::f32::consts::PI {
        sweep += 2.0 * std::f32::consts::PI;
    }
    let pairs = arc_steps(sweep, radius).div_ceil(2);
    let waist = radius * arc_waist(sweep, pairs);
    let halves = pairs.saturating_mul(2);
    let mut points = Vec::with_capacity(halves.saturating_add(2));
    points.push(centre);
    points.push(from);
    for i in 1..halves {
        #[expect(clippy::cast_precision_loss)] // twice MAX_ARC_STEPS at the most
        let t = a0 + sweep * (i as f32) / (halves as f32);
        let r = if i % 2 == 1 { waist } else { radius };
        points.push(Point::new(centre.x + r * t.cos(), centre.y + r * t.sin()));
    }
    points.push(to);
    Polyline::polygon(points)
}

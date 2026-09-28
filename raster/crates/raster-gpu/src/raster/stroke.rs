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

use raster_scene::{LineCap, LineJoin, Point, Stroke};

use super::flatten::{DeviceTransform, Polyline};

mod disjoint;

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
#[expect(clippy::arithmetic_side_effects)]
pub(crate) fn stroke_polylines(
    polylines: &[Polyline],
    stroke: Stroke,
    device_width: f32,
) -> Vec<Polyline> {
    let hw = device_width * 0.5;
    let mut out = Vec::new();
    for polyline in polylines {
        // Dedupe coincident neighbours (and the closing wrap, when closed).
        let mut pts: Vec<Point> = Vec::with_capacity(polyline.points.len());
        for &p in &polyline.points {
            #[expect(clippy::float_cmp)] // exact: a zero-length piece, not a near one
            if pts.last().is_none_or(|q| q.x != p.x || q.y != p.y) {
                pts.push(p);
            }
        }
        #[expect(clippy::float_cmp)]
        if polyline.closed
            && pts.len() > 1
            && pts[0].x == pts[pts.len() - 1].x
            && pts[0].y == pts[pts.len() - 1].y
        {
            pts.pop();
        }
        if pts.len() < 2 {
            continue; // a lone point: degenerate, pre-split upstream (§8.5.3.2)
        }

        let segment_count = if polyline.closed {
            pts.len()
        } else {
            pts.len() - 1
        };
        // Each vertex's inner cut, computed once so that the two pieces meeting there
        // share its point to the bit (ADR 1361); and which vertices turn without one,
        // a bend tighter than the half-width (ADR 1375).
        let mut tight = vec![false; pts.len()];
        let cuts: Vec<Option<InnerCut>> = (0..pts.len())
            .map(|j| {
                let joined = polyline.closed || (j > 0 && j + 1 < pts.len());
                if !joined {
                    return None;
                }
                let (prev, next) = (
                    pts[(j + pts.len() - 1) % pts.len()],
                    pts[(j + 1) % pts.len()],
                );
                let cut = inner_cut(prev, pts[j], next, hw);
                tight[j] = cut.is_none() && turns(prev, pts[j], next);
                cut
            })
            .collect();
        // The pieces, and beside each whether it meets a tight bend.
        let (mut pieces, mut at_a_tight_bend) = (Vec::new(), Vec::new());
        // One piece per segment, less those another segment's piece already holds.
        let held = held_by_a_neighbour(&pts, polyline.closed, segment_count);
        for i in (0..segment_count).filter(|&i| !held[i]) {
            let (from, to) = (i, (i + 1) % pts.len());
            pieces.push(segment_piece(pts[from], pts[to], hw, cuts[from], cuts[to]));
            at_a_tight_bend.push(tight[from] || tight[to]);
        }
        // Joins at interior vertices (all vertices when closed).
        let join_count = if polyline.closed {
            pts.len()
        } else {
            pts.len().saturating_sub(2)
        };
        for j in 0..join_count {
            let prev = pts[j];
            let v = pts[(j + 1) % pts.len()];
            let next = pts[(j + 2) % pts.len()];
            join_at(
                &mut pieces,
                prev,
                v,
                next,
                hw,
                stroke.join,
                stroke.miter_limit,
            );
            at_a_tight_bend.resize(pieces.len(), tight[(j + 1) % pts.len()]);
        }
        // Caps at open ends.
        if !polyline.closed {
            let first_dir = direction(pts[0], pts[1]);
            let last = pts.len() - 1;
            let last_dir = direction(pts[last - 1], pts[last]);
            cap_at(
                &mut pieces,
                pts[0],
                Point::new(-first_dir.x, -first_dir.y),
                hw,
                stroke.cap,
            );
            cap_at(&mut pieces, pts[last], last_dir, hw, stroke.cap);
            at_a_tight_bend.resize(pieces.len(), false);
        }
        // Where the path bends more tightly than the half-width the pieces overlap in a
        // star whose points are the rim, and the fill would count each overlap twice
        // there; the tiling holds the same set with every point covered once.
        if at_a_tight_bend.contains(&true) {
            out.extend(disjoint::disjoint(pieces, &at_a_tight_bend));
        } else {
            out.extend(pieces);
        }
    }
    out
}

/// Whether the path changes direction at `v` at all, other than straight back: a vertex
/// [`inner_cut`] declines for that reason is not a tight bend.
fn turns(prev: Point, v: Point, next: Point) -> bool {
    let (d1, d2) = (direction(prev, v), direction(v, next));
    d1.x * d2.y - d1.y * d2.x != 0.0
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

/// Where the two inner offset lines of a vertex meet, and on which side of the stroke
/// that is (`left`: the side of the left normal [`normal`] gives).
#[derive(Debug, Clone, Copy)]
struct InnerCut {
    point: Point,
    left: bool,
}

/// The inner cut at `v`, or `None` where the two pieces meeting there must overlap
/// instead (ADR 1361).
///
/// Two segment rectangles meeting at an angle overlap on the inner side of the turn,
/// in the kite between `v` and the point where their inner edges cross. Under
/// [`fill`](super::fill)'s accumulation an overlap costs nothing inside the stroke,
/// where the winding is clamped, but in a pixel on the stroke's rim it is **counted
/// twice**: the pixel integrates `+a` and `+b` and reads `a + b`, not the area of their
/// union. On a thin curve every pixel is on the rim, so nearly every overlap is counted
/// in full (0.7 units of ink on a one-unit ring of 64 sides). Cutting both rectangles along the line from `v` to that crossing makes
/// them meet edge to edge instead, and the part each loses is inside the other, so
/// the union — §8.4.3.2's set — is unchanged.
///
/// The crossing lies `t = hw · tan(θ / 2)` back along each segment from `v`, where `θ`
/// is the turn. It is only a corner of both pieces while `t` is at most half of each
/// segment: past that, the cuts at a segment's two ends could meet, and a curve that
/// bends more tightly than the half-width is exactly where they do. Such a vertex is
/// left uncut, and the pieces that meet it are tiled by [`disjoint`](mod@disjoint)
/// instead (ADR 1375).
fn inner_cut(prev: Point, v: Point, next: Point, hw: f32) -> Option<InnerCut> {
    let d1 = direction(prev, v);
    let d2 = direction(v, next);
    let cross = d1.x * d2.y - d1.y * d2.x;
    let dot = d1.x * d2.x + d1.y * d2.y;
    // `tan(θ / 2) = sin θ / (1 + cos θ)`; a reversal has no crossing at all.
    let t = hw * cross.abs() / (1.0 + dot);
    let reach = 0.5 * distance(prev, v).min(distance(v, next));
    if cross == 0.0 || !t.is_finite() || t > reach {
        return None;
    }
    // The inner side is the side the path turns towards: the left normal's side when
    // `cross > 0`, which is [`join_at`]'s gap on the other side.
    let left = cross > 0.0;
    let side = if left { hw } else { -hw };
    Some(InnerCut {
        point: Point::new(v.x - d1.y * side - d1.x * t, v.y + d1.x * side - d1.y * t),
        left,
    })
}

/// The piece one segment contributes: its rectangle (§8.4.3.2's points within the
/// half-width, across the segment's own length), with the inner corner at either end
/// replaced by that vertex's [`InnerCut`] where it has one.
///
/// The points are visited in the rectangle's own order — left side forward, right side
/// back — so a cut piece is wound as every other piece is. At a cut end the vertex
/// itself is inserted between the inner point and the uncut half of the end, because
/// that half is the edge the join on the outer side shares.
fn segment_piece(
    a: Point,
    b: Point,
    hw: f32,
    start: Option<InnerCut>,
    end: Option<InnerCut>,
) -> Polyline {
    let n = normal(a, b, hw);
    let (a_left, a_right) = (
        Point::new(a.x + n.x, a.y + n.y),
        Point::new(a.x - n.x, a.y - n.y),
    );
    let (b_left, b_right) = (
        Point::new(b.x + n.x, b.y + n.y),
        Point::new(b.x - n.x, b.y - n.y),
    );
    let mut points = Vec::with_capacity(6);
    match start {
        Some(cut) if cut.left => points.push(cut.point),
        _ => points.push(a_left),
    }
    match end {
        Some(cut) if cut.left => points.extend([cut.point, b, b_right]),
        Some(cut) => points.extend([b_left, b, cut.point]),
        None => points.extend([b_left, b_right]),
    }
    match start {
        Some(cut) if cut.left => points.extend([a_right, a]),
        Some(cut) => points.extend([cut.point, a]),
        None => points.push(a_right),
    }
    Polyline {
        points,
        closed: true,
    }
}

/// The left normal of `a → b`, scaled to the half-width.
fn normal(a: Point, b: Point, hw: f32) -> Point {
    let d = direction(a, b);
    Point::new(-d.y * hw, d.x * hw)
}

fn join_at(
    out: &mut Vec<Polyline>,
    prev: Point,
    v: Point,
    next: Point,
    hw: f32,
    join: LineJoin,
    miter_limit: f32,
) {
    let d1 = direction(prev, v);
    let d2 = direction(v, next);
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
        LineJoin::Bevel => out.push(Polyline {
            points: vec![v, first, second],
            closed: true,
        }),
        LineJoin::Miter => {
            // §8.4.3.5: the miter stands until length/width exceeds the limit; then
            // the join is a bevel. Ratio = 1 / cos(half-angle), via the unit normals.
            let dot = (n1.x * n2.x + n1.y * n2.y) / (hw * hw);
            let denom = 1.0 + dot;
            let ratio_sq = 2.0 / denom.max(f32::EPSILON);
            if ratio_sq <= miter_limit * miter_limit {
                let scale = 1.0 / denom.max(f32::EPSILON);
                let m = Point::new(v.x + (n1.x + n2.x) * scale, v.y + (n1.y + n2.y) * scale);
                out.push(Polyline {
                    points: vec![v, first, m, second],
                    closed: true,
                });
            } else {
                out.push(Polyline {
                    points: vec![v, first, second],
                    closed: true,
                });
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
        LineCap::Square => out.push(Polyline {
            points: vec![
                Point::new(end.x + n.x, end.y + n.y),
                Point::new(end.x + n.x + dir.x * hw, end.y + n.y + dir.y * hw),
                Point::new(end.x - n.x + dir.x * hw, end.y - n.y + dir.y * hw),
                Point::new(end.x - n.x, end.y - n.y),
            ],
            closed: true,
        }),
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
/// The step count is [`arc_steps`]', as for any other arc.
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
    let steps = arc_steps(std::f32::consts::PI, hw);
    let mut points = Vec::with_capacity(steps.saturating_add(2));
    points.push(end);
    points.push(Point::new(end.x + n.x, end.y + n.y));
    for i in 1..steps {
        #[expect(clippy::cast_precision_loss)] // steps is at most MAX_ARC_STEPS
        let t =
            base + std::f32::consts::FRAC_PI_2 - std::f32::consts::PI * (i as f32) / (steps as f32);
        points.push(Point::new(end.x + hw * t.cos(), end.y + hw * t.sin()));
    }
    points.push(Point::new(end.x - n.x, end.y - n.y));
    Polyline {
        points,
        closed: true,
    }
}

/// The coarsest angle one step of an arc advances. Deterministic (brief section 4.6),
/// and the step [`arc_steps`] takes for any arc of radius under 16 device pixels.
const ARC_STEP: f32 = 0.35;

/// The most steps one arc of up to a half turn is cut into: a stroke's width is a
/// document's number, and the step count is an allocation (CLAUDE.md principle 3).
/// At this count an arc is within the tolerance up to a radius of about 13 000 device
/// pixels; a wider one is drawn coarser than that, and no finer.
const MAX_ARC_STEPS: usize = 256;

/// How many chords an arc of `sweep` radians and radius `radius` is cut into, so that
/// no chord falls further inside the arc than
/// [`FLATTEN_TOLERANCE`](super::flatten::FLATTEN_TOLERANCE) — §10.7.2's bound, the one
/// every other curve here is flattened to (ADR 1361).
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

/// A fan of points approximating the arc from `from` to `to` around `centre` (both on
/// the circle of radius `radius`), as one closed polygon including the centre.
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
    let steps = arc_steps(sweep, radius);
    let mut points = vec![centre, from];
    for i in 1..steps {
        #[expect(clippy::cast_precision_loss)]
        let t = a0 + sweep * (i as f32) / (steps as f32);
        points.push(Point::new(
            centre.x + radius * t.cos(),
            centre.y + radius * t.sin(),
        ));
    }
    points.push(to);
    Polyline {
        points,
        closed: true,
    }
}

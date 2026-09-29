//! Flattening: an outline's segments, under a transform, become device-space
//! polylines — and how finely.
//!
//! One thing: the conversion from `raster_scene`'s curves to the straight-line
//! approximation everything downstream in [`raster`](super) works on. Nothing here
//! knows what a coverage byte is; [`fill`](super::fill) does, and it is handed
//! [`Polyline`]s.
//!
//! The bound is ISO 32000-2 §10.7.2's flatness tolerance, read twice — as a distance
//! in device pixels ([`FLATTEN_TOLERANCE`]) and as a fraction of the curve's own size
//! ([`RELATIVE_FLATTEN_TOLERANCE`], ADR 0044) — and the tighter of the two binds.
//! Subdivision is at `t = 1/2`, which is exact in `f32`, so a flattening is the same
//! on every adapter and in every thread (ADR 0008's determinism, brief section 4.6).

use raster_scene::{Point, Segment};

/// Maximum distance, in device pixels, between a cubic and its flattening. 0.25 px
/// keeps the flattening error below half of one coverage step at the edge of a
/// pixel; the choice is recorded in ADR 0008 with its cost.
///
/// This is the bound ISO 32000-2 §10.7.2 states:
///
/// > The flatness tolerance controls the maximum permitted distance in device pixels
/// > between the mathematically correct path and an approximation constructed from
/// > straight line segments
///
/// — measured in device pixels, which is why it is applied after the transform. The
/// same clause's "PDF processors may choose to ignore any flatness tolerance specified
/// within a PDF file" is why the number is ours and not the document's.
pub(crate) const FLATTEN_TOLERANCE: f32 = 0.25;

/// The same distance as a fraction of the cubic's own device extent — the bound that
/// binds once a whole curve is no bigger than a few [`FLATTEN_TOLERANCE`]s.
///
/// A distance in device pixels says nothing about a shape smaller than itself: at a
/// quarter pixel a circle of diameter 1 flattens to four chords and deposits its
/// **inscribed square**, 36.3 % short of its own area. §10.7.2's NOTE 2 is explicit
/// that this is not what the tolerance is for:
///
/// > the purpose of the flatness tolerance is to control the precision of curve
/// > rendering, not to draw inscribed polygons. If the parameter's value is large
/// > enough to cause visible straight line segments to appear, the result is
/// > unpredictable.
///
/// 1/32 of the curve's own control-polygon diagonal holds any closed curve to at least
/// 16 chords per full turn, whose area is `(16/2π)·sin(2π/16) = 0.9745` of the circle's
/// — 2.55 % short at worst, against the 1–4 % that rounding coverage to a byte already
/// costs a mark of that size. It never loosens the absolute bound and therefore never
/// removes a segment: no circle of radius 2.4 device pixels or more changes at all.
/// ADR 0044 has the arithmetic and the rejected alternative.
pub(crate) const RELATIVE_FLATTEN_TOLERANCE: f32 = 0.031_25;

/// A device-space transform applied during flattening: the composed
/// command-times-viewport affine, as six f32s (kept away from `raster_scene::Affine`
/// only to avoid a needless dependency direction — the arithmetic is §8.3.3's).
#[derive(Debug, Clone, Copy)]
pub(crate) struct DeviceTransform {
    pub a: f32,
    pub b: f32,
    pub c: f32,
    pub d: f32,
    pub e: f32,
    pub f: f32,
}

impl DeviceTransform {
    fn apply(self, p: Point) -> Point {
        Point::new(
            self.a * p.x + self.c * p.y + self.e,
            self.b * p.x + self.d * p.y + self.f,
        )
    }

    /// The largest factor by which this transform lengthens a vector — the linear
    /// part's larger singular value, mirrored **statement for statement** from the
    /// caller's `Transform::max_stretch` so the width the encode resolves is the width
    /// their own resolution produced, to the bit (ADR 0085).
    pub(crate) fn max_stretch(self) -> f32 {
        let sum = self.a * self.a + self.b * self.b + self.c * self.c + self.d * self.d;
        let determinant = self.a * self.d - self.b * self.c;
        let discriminant = (sum * sum - 4.0 * determinant * determinant).max(0.0);
        f32::midpoint(sum, discriminant.sqrt()).max(0.0).sqrt()
    }
}

/// One flattened subpath: device-space points, and whether the source closed it.
#[derive(Debug, Clone)]
pub(crate) struct Polyline {
    pub points: Vec<Point>,
    pub closed: bool,
    /// The source path's own directions at the points where a curve begins or ends, in
    /// the order of `points`; empty for a subpath of straight segments, whose chords are
    /// their own directions. Only the stroker reads them (ADR 1389, ADR 1397).
    pub tangents: Vec<Tangent>,
}

/// The source path's own direction at one point of a subpath where a curve begins or ends
/// (ISO 32000-2 §8.4.3.3, §8.4.3.4).
///
/// §8.4.3.3 squares a butt cap off "at the endpoint of the path", and §8.4.3.4 draws a
/// join "at the corners of paths", where "consecutive segments of a path connect at an
/// angle": both are shaped by the direction the path runs at that point, which at a curve's
/// end is its tangent — the Bézier's derivative at `t = 1` arriving, and at `t = 0`
/// leaving. Flattening keeps the points and loses that direction: the last chord of a
/// quarter arc is turned from the arc's end tangent by half its angle. So the direction is
/// taken from the control points before they are dropped.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Tangent {
    /// The index into the subpath's `points`.
    pub at: usize,
    /// The direction a curve arrives at the point along, not normalised.
    pub arriving: Option<Point>,
    /// The direction a curve leaves the point along, not normalised.
    pub leaving: Option<Point>,
}

impl Polyline {
    /// A closed polygon with no curve behind it, as the stroker and the rectangle paths
    /// build them.
    pub(crate) fn polygon(points: Vec<Point>) -> Self {
        Self {
            points,
            closed: true,
            tangents: Vec::new(),
        }
    }
}

/// Record a curve's direction at point `at`, beside whatever the curve before it recorded
/// there: one curve arrives where the next leaves.
fn record(tangents: &mut Vec<Tangent>, at: usize, arriving: Option<Point>, leaving: Option<Point>) {
    match tangents.last_mut() {
        Some(last) if last.at == at => {
            last.arriving = last.arriving.or(arriving);
            last.leaving = leaving.or(last.leaving);
        }
        _ => tangents.push(Tangent {
            at,
            arriving,
            leaving,
        }),
    }
}

/// The first of `candidates` that is a direction at all: a cubic's derivative at an end
/// is zero where the control point coincides with the end point, and the curve then
/// leaves along the next control point that does not (the derivative's first non-zero
/// order).
fn first_direction(candidates: [Point; 3]) -> Option<Point> {
    candidates
        .into_iter()
        .find(|d| (d.x != 0.0 || d.y != 0.0) && d.x.is_finite() && d.y.is_finite())
}

/// Flatten an outline under a transform into polylines, one per subpath.
///
/// Curves subdivide at their midpoint until the control points sit within
/// [`cubic_tolerance`] of the chord — the standard flatness bound: for a cubic,
/// the curve deviates from the chord by at most 3/4 of the larger control-point
/// distance, so testing the controls bounds the curve. Subdivision at t = 1/2 is
/// exact f32 arithmetic (halving), keeping flattening deterministic everywhere.
pub(crate) fn flatten(segments: &[Segment], transform: DeviceTransform) -> Vec<Polyline> {
    flatten_keeping(segments, transform, false)
}

/// [`flatten`], keeping each curve's direction where it begins and ends ([`Tangent`]) for
/// the stroker: its caps and joins are square to those directions (ADR 1389, ADR 1397).
///
/// A fill never reads them, and keeping them is an allocation per curved subpath, which a
/// page of glyphs pays in every outline it rasterises — ISO 32000-2's page 101 drew 7% dearer
/// in instructions with every flattening keeping them (ADR 1397) — so only a stroke asks.
pub(crate) fn flatten_stroke(segments: &[Segment], transform: DeviceTransform) -> Vec<Polyline> {
    flatten_keeping(segments, transform, true)
}

/// [`flatten`], keeping the curves' [`Tangent`]s where `keep` asks for them.
fn flatten_keeping(segments: &[Segment], transform: DeviceTransform, keep: bool) -> Vec<Polyline> {
    let mut subpaths = Vec::new();
    let mut current: Vec<Point> = Vec::new();
    let mut tangents: Vec<Tangent> = Vec::new();
    let mut push_current = |current: &mut Vec<Point>, tangents: &mut Vec<Tangent>, closed| {
        if current.len() > 1 {
            subpaths.push(Polyline {
                points: std::mem::take(current),
                closed,
                tangents: std::mem::take(tangents),
            });
        } else {
            current.clear();
            tangents.clear();
        }
    };
    for segment in segments {
        match *segment {
            Segment::MoveTo(p) => {
                push_current(&mut current, &mut tangents, false);
                current.push(transform.apply(p));
            }
            // A line is its own chord, which the stroker already has: it records nothing.
            Segment::LineTo(p) => {
                if !current.is_empty() {
                    current.push(transform.apply(p));
                }
            }
            Segment::CubicTo { c1, c2, to } => {
                if let Some(&from) = current.last() {
                    let (c1, c2, to) = (
                        transform.apply(c1),
                        transform.apply(c2),
                        transform.apply(to),
                    );
                    // A cubic whose points all coincide goes nowhere and decides nothing.
                    let leaving = keep
                        .then(|| first_direction([c1, c2, to].map(|q| difference(from, q))))
                        .flatten();
                    #[expect(clippy::arithmetic_side_effects)] // `current` is not empty
                    let start = current.len() - 1;
                    if leaving.is_some() {
                        record(&mut tangents, start, None, leaving);
                    }
                    // Measured once for the whole cubic and carried down the
                    // subdivision, not recomputed per half: the bound is "within a
                    // fraction of *this curve*", and a bound that shrank with every
                    // split would be a fixed chord angle applied to shapes of every
                    // size. Per cubic rather than per outline for the opposite reason —
                    // one path can hold a page border and a one-pixel dot, and the dot
                    // must not inherit the border's extent.
                    let tolerance = cubic_tolerance(from, c1, c2, to);
                    flatten_cubic(from, c1, c2, to, tolerance, 0, &mut current);
                    if leaving.is_some() {
                        let arriving = first_direction([c2, c1, from].map(|q| difference(q, to)));
                        #[expect(clippy::arithmetic_side_effects)] // `current` is not empty
                        let end = current.len() - 1;
                        record(&mut tangents, end, arriving, None);
                    }
                }
            }
            Segment::Close => push_current(&mut current, &mut tangents, true),
        }
    }
    push_current(&mut current, &mut tangents, false);
    subpaths
}

/// Whether a closed polyline's `points` are convex and go round once: every turn the same
/// way (or straight on), and each coordinate changing direction at most twice round the
/// loop — which a five-pointed star, turning one way throughout but going round twice,
/// fails. Such a polygon crosses itself nowhere, which both the fill's question and the
/// stroker's rest on (ADR 1389, ADR 1397).
#[expect(clippy::arithmetic_side_effects)] // indices below the length
#[expect(clippy::float_cmp)] // signs are exactly `−1`, `0` or `+1`
pub(crate) fn convex(points: &[Point]) -> bool {
    let n = points.len();
    let steps = (0..n).filter_map(|i| {
        let (p, q) = (points[i], points[(i + 1) % n]);
        let d = (q.x - p.x, q.y - p.y);
        (d.0 != 0.0 || d.1 != 0.0).then_some(d)
    });
    let (mut first, mut previous) = (None, None::<(f32, f32)>);
    let (mut turn, mut flips) = (0.0_f32, [0_usize; 2]);
    let (mut sign_x, mut sign_y) = (0.0_f32, 0.0_f32);
    let mut look = |d: (f32, f32), turn: &mut f32, flips: &mut [usize; 2]| -> bool {
        if let Some(p) = previous {
            let cross = p.0 * d.1 - p.1 * d.0;
            if cross != 0.0 {
                if *turn != 0.0 && cross.signum() != *turn {
                    return false;
                }
                *turn = cross.signum();
            }
        }
        for (axis, (value, sign)) in [(d.0, &mut sign_x), (d.1, &mut sign_y)]
            .into_iter()
            .enumerate()
        {
            if value != 0.0 {
                if *sign != 0.0 && value.signum() != *sign {
                    flips[axis] += 1;
                }
                *sign = value.signum();
            }
        }
        previous = Some(d);
        true
    };
    for d in steps {
        first.get_or_insert(d);
        if !look(d, &mut turn, &mut flips) {
            return false;
        }
    }
    // Round the corner back to the start, so the last turn and flips are counted too.
    match first {
        Some(d) => look(d, &mut turn, &mut flips) && flips[0] <= 2 && flips[1] <= 2,
        None => true,
    }
}

/// `to − from`.
fn difference(from: Point, to: Point) -> Point {
    Point::new(to.x - from.x, to.y - from.y)
}

/// The flatness bound for one cubic: the tighter of the device tolerance and
/// [`RELATIVE_FLATTEN_TOLERANCE`] of the curve's own control-polygon diagonal
/// (ADR 0044).
///
/// The diagonal rather than the chord, because a cubic's chord can be zero while the
/// curve is not (a closed loop), and the control polygon contains the curve. Non-finite
/// coordinates cannot reach here — `Resources::upload_outline` refuses them and
/// `SceneBuilder` refuses a non-finite transform — but `min` would fall back to the
/// absolute bound if one ever did, which is the behaviour this had before.
pub(super) fn cubic_tolerance(p0: Point, p1: Point, p2: Point, p3: Point) -> f32 {
    let width = p0.x.max(p1.x).max(p2.x).max(p3.x) - p0.x.min(p1.x).min(p2.x).min(p3.x);
    let height = p0.y.max(p1.y).max(p2.y).max(p3.y) - p0.y.min(p1.y).min(p2.y).min(p3.y);
    FLATTEN_TOLERANCE.min(RELATIVE_FLATTEN_TOLERANCE * width.hypot(height))
}

fn flatten_cubic(
    p0: Point,
    p1: Point,
    p2: Point,
    p3: Point,
    tolerance: f32,
    depth: u8,
    out: &mut Vec<Point>,
) {
    // Flat when both controls are within tolerance of the chord: the curve is
    // bounded by the control polygon's deviation.
    let flat = {
        let dx = p3.x - p0.x;
        let dy = p3.y - p0.y;
        let d1 = ((p1.x - p0.x) * dy - (p1.y - p0.y) * dx).abs();
        let d2 = ((p2.x - p0.x) * dy - (p2.y - p0.y) * dx).abs();
        let len_sq = dx * dx + dy * dy;
        // Degenerate chord: fall back to control-point distance from p0.
        if len_sq <= f32::EPSILON {
            let c1 = (p1.x - p0.x).abs().max((p1.y - p0.y).abs());
            let c2 = (p2.x - p0.x).abs().max((p2.y - p0.y).abs());
            c1.max(c2) <= tolerance
        } else {
            (d1.max(d2)) * (d1.max(d2)) <= tolerance * tolerance * len_sq
        }
    };
    // The depth cap bounds work on hostile geometry; at 16 the segments are 2^-16 of
    // the curve and far below any tolerance a finite target can observe. The relative
    // bound cannot approach it from below: a control point is never further from the
    // chord than the control polygon's diagonal, and each split divides that distance
    // by about four, so 1/32 of the diagonal is reached in three levels whatever the
    // curve's size.
    if flat || depth >= 16 {
        out.push(p3);
        return;
    }
    let mid = |a: Point, b: Point| Point::new((a.x + b.x) * 0.5, (a.y + b.y) * 0.5);
    let q0 = mid(p0, p1);
    let q1 = mid(p1, p2);
    let q2 = mid(p2, p3);
    let r0 = mid(q0, q1);
    let r1 = mid(q1, q2);
    let split = mid(r0, r1);
    flatten_cubic(p0, q0, r0, split, tolerance, depth.saturating_add(1), out);
    flatten_cubic(split, r1, q2, p3, tolerance, depth.saturating_add(1), out);
}

/// The integer-pixel bounding box of a set of polylines, or `None` when empty.
pub(crate) fn polyline_bounds(polylines: &[Polyline]) -> Option<(f32, f32, f32, f32)> {
    let mut bounds: Option<(f32, f32, f32, f32)> = None;
    for polyline in polylines {
        for p in &polyline.points {
            bounds = Some(match bounds {
                None => (p.x, p.y, p.x, p.y),
                Some((x0, y0, x1, y1)) => (x0.min(p.x), y0.min(p.y), x1.max(p.x), y1.max(p.y)),
            });
        }
    }
    bounds
}

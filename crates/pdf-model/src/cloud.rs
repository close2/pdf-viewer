//! Table 169's cloudy border effect: the geometry ISO 32000-2 leaves open, chosen once.
//!
//! §12.5.4 introduces the entry with a `shall` — "some annotations (square, circle, and polygon)
//! may have a BE entry, which is a border effect dictionary that specifies an effect that shall
//! be applied to the border of the annotations" — and Table 169's `C` says what the effect is:
//!
//! > The border should appear "cloudy"; that is, the border should be drawn as a series of
//! > convex curved line segments in a manner that simulates the appearance of a cloud. The
//! > width and dash array specified by BS shall be honoured.
//!
//! That is a shape stated in words and no numbers: no radius, no count, and an `/I` intensity
//! "in the range 0 to 2" with no unit. So this module is ADR 0192's construction again — a
//! `shall` behind a silence about artwork — and what it chooses is written here, as a choice
//! (ADR 1057):
//!
//! - **Each segment is a semicircle on a chord of the border's own path**, bulging away from the
//!   shape's interior. A semicircle is the least a "convex curved line segment" can be while
//!   still meeting its neighbours at a point, and consecutive ones meeting at cusps is what a
//!   drawn cloud is.
//! - **The path is divided into chords of as near the chosen length as divides it evenly**, per
//!   edge, so that every corner of a rectangle or polygon is a cusp and no scallop straddles one.
//!   A curved edge — an ellipse, a `/Path` curve — is one edge, and its cusps are spaced by arc
//!   length along it.
//! - **The semicircle's radius is two points, plus two for each unit of intensity, and never
//!   less than the line's own width**: `/I 0` is the smallest cloud rather than none, because
//!   `/S /C` has already asked for one and the table makes 0 its default; and a scallop narrower
//!   than the stroke that draws it is a bumpy line rather than a curve anybody can see.
//!   [`radius`] is the whole of that arithmetic.
//!
//! Where the cusps go — inside the shape so that the arcs reach its boundary, or on a polygon's
//! own vertices — is the caller's, because it is the same question as where a straight border
//! goes and `crate::appearance` already answers it per subtype.

/// A cubic Bézier segment: two control points and an end, from wherever the path already is.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Curve {
    /// The first control point.
    pub first: [f32; 2],
    /// The second control point.
    pub second: [f32; 2],
    /// Where the segment ends.
    pub end: [f32; 2],
}

/// A closed scalloped path: a start point and the curves that return to it.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Cloud {
    /// Where the path begins, which is also where its last curve ends.
    pub start: [f32; 2],
    /// Two quarter arcs per scallop, in drawing order.
    pub curves: Vec<Curve>,
}

/// One edge of an outline: a polyline whose last point is the next edge's first.
pub(crate) type Edge = Vec<[f32; 2]>;

/// The radius of one scallop for Table 169's `/I`, on a line of `width`.
///
/// "A number describing the intensity of the effect, in the range 0 to 2", defaulting to 0.
/// Two points at 0 and two more per unit, so 2, 4 and 6 at the three whole intensities; a value
/// outside the table's range is held to its nearest end rather than allowed to draw a cloud of
/// no scallops or of one. A line wider than that draws scallops of its own width instead, so
/// that each is at least as wide as the stroke laid over it.
#[must_use]
pub(crate) fn radius(intensity: f32, width: f32) -> f32 {
    let held = if intensity.is_finite() {
        intensity.clamp(0.0, 2.0)
    } else {
        0.0
    };
    let chosen = 2.0 + 2.0 * held;
    if width.is_finite() {
        chosen.max(width)
    } else {
        chosen
    }
}

/// §8.5.2.2's constant for a quarter circle as one cubic Bézier, shared with the ellipse.
const ARC: f32 = crate::appearance::ARC;

/// Scallops a closed outline with semicircles of about `radius`, bulging outward.
///
/// `None` where there is nothing to scallop: no edge has any length, or the radius is not
/// positive. The scallops of each edge are sized to divide that edge evenly, so their radius is
/// the nearest to `radius` that does — never more than half again as large, since an edge shorter
/// than one chord still gets one scallop.
#[must_use]
pub(crate) fn cloud(outline: &[Edge], radius: f32) -> Option<Cloud> {
    if !radius.is_finite() || radius <= 0.0 {
        return None;
    }
    let cusps = cusps(outline, 2.0 * radius);
    if cusps.len() < 2 {
        return None;
    }
    // Which side is outside: a counter-clockwise outline (positive signed area in a y-up space)
    // has its exterior on the right of the direction of travel.
    let area: f32 = outline
        .iter()
        .flatten()
        .zip(outline.iter().flatten().skip(1))
        .map(|(a, b)| a[0] * b[1] - b[0] * a[1])
        .sum();
    let outward = if area >= 0.0 { 1.0 } else { -1.0 };

    let mut curves = Vec::with_capacity(cusps.len().saturating_mul(2));
    for (from, to) in cusps.iter().zip(cusps.iter().cycle().skip(1)) {
        let (dx, dy) = (to[0] - from[0], to[1] - from[1]);
        let chord = dx.hypot(dy);
        if chord <= f32::EPSILON {
            continue;
        }
        let r = chord * 0.5;
        let along = [dx / chord, dy / chord];
        let normal = [along[1] * outward, -along[0] * outward];
        let middle = [from[0] + dx * 0.5, from[1] + dy * 0.5];
        let apex = [middle[0] + normal[0] * r, middle[1] + normal[1] * r];
        let grip = r * ARC;
        // From the cusp up to the apex: the tangent leaves along the outward normal and
        // arrives along the chord's direction.
        curves.push(Curve {
            first: [from[0] + normal[0] * grip, from[1] + normal[1] * grip],
            second: [apex[0] - along[0] * grip, apex[1] - along[1] * grip],
            end: apex,
        });
        // And from the apex down to the next cusp, the mirror of it.
        curves.push(Curve {
            first: [apex[0] + along[0] * grip, apex[1] + along[1] * grip],
            second: [to[0] + normal[0] * grip, to[1] + normal[1] * grip],
            end: *to,
        });
    }
    let start = *cusps.first()?;
    (!curves.is_empty()).then_some(Cloud { start, curves })
}

/// The points where scallops meet, edge by edge, each edge's end left to the next edge's start.
///
/// An edge is never divided into more than [`MOST_SCALLOPS`] chords: a perimeter of a hundred
/// thousand points is a file's to state and a path of fifty thousand curves is not a mark anybody
/// can see, so past that the scallops grow rather than multiply.
fn cusps(outline: &[Edge], chord: f32) -> Vec<[f32; 2]> {
    let mut out = Vec::new();
    for edge in outline {
        let Some(first) = edge.first() else {
            continue;
        };
        let lengths: Vec<f32> = edge
            .windows(2)
            .map(|pair| (pair[1][0] - pair[0][0]).hypot(pair[1][1] - pair[0][1]))
            .collect();
        let total: f32 = lengths.iter().sum();
        if !total.is_finite() || total <= f32::EPSILON {
            continue;
        }
        // Evenly: the count nearest to what the chosen chord would give, and never zero.
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "clamped to 1.0..=MOST_SCALLOPS above, so the conversion is exact"
        )]
        let count = (total / chord).round().clamp(1.0, f32::from(MOST_SCALLOPS)) as u16;
        let pitch = total / f32::from(count);
        out.push(*first);
        for k in 1..count {
            out.push(along(edge, &lengths, pitch * f32::from(k)));
        }
    }
    out
}

/// The most scallops one edge is divided into; see [`cusps`].
const MOST_SCALLOPS: u16 = 4096;

/// The point a distance along a polyline, measured from its first point.
fn along(edge: &[[f32; 2]], lengths: &[f32], distance: f32) -> [f32; 2] {
    let mut remaining = distance;
    for (segment, length) in edge.windows(2).zip(lengths) {
        if remaining <= *length {
            let t = if *length > 0.0 {
                remaining / length
            } else {
                0.0
            };
            return [
                segment[0][0] + (segment[1][0] - segment[0][0]) * t,
                segment[0][1] + (segment[1][1] - segment[0][1]) * t,
            ];
        }
        remaining -= length;
    }
    edge.last().copied().unwrap_or([0.0, 0.0])
}

/// A rectangle as four straight edges, counter-clockwise from its lower-left corner.
#[must_use]
pub(crate) fn rectangle(box_: [f32; 4]) -> Vec<Edge> {
    polygon(&[
        [box_[0], box_[1]],
        [box_[2], box_[1]],
        [box_[2], box_[3]],
        [box_[0], box_[3]],
    ])
}

/// An ellipse inscribed in a box as one closed edge, flattened finely enough that the flattening
/// is invisible under a scallop: 128 segments put a 2000-point ellipse within a third of a point
/// of its chord polygon.
#[must_use]
pub(crate) fn ellipse(box_: [f32; 4]) -> Vec<Edge> {
    const SEGMENTS: u16 = 128;
    let (cx, cy) = ((box_[0] + box_[2]) * 0.5, (box_[1] + box_[3]) * 0.5);
    let (a, b) = ((box_[2] - box_[0]) * 0.5, (box_[3] - box_[1]) * 0.5);
    let point = |i: u16| {
        let angle = std::f32::consts::TAU * f32::from(i) / f32::from(SEGMENTS);
        [cx + a * angle.cos(), cy + b * angle.sin()]
    };
    // Closed: the first point is spelt again at the end, exactly, rather than as cos(2π).
    let edge: Edge = (0..SEGMENTS).map(point).chain([point(0)]).collect();
    vec![edge]
}

/// A closed polygon's vertices as straight edges, the last vertex joined back to the first.
#[must_use]
pub(crate) fn polygon(vertices: &[[f32; 2]]) -> Vec<Edge> {
    if vertices.len() < 2 {
        return Vec::new();
    }
    vertices
        .iter()
        .zip(vertices.iter().cycle().skip(1))
        .map(|(from, to)| vec![*from, *to])
        .collect()
}

/// A cubic Bézier flattened into one edge of sixteen straight pieces.
#[must_use]
pub(crate) fn flattened(from: [f32; 2], curve: Curve) -> Edge {
    const PIECES: u8 = 16;
    (0..=PIECES)
        .map(|i| {
            let t = f32::from(i) / f32::from(PIECES);
            let u = 1.0 - t;
            let (b0, b1, b2, b3) = (u * u * u, 3.0 * u * u * t, 3.0 * u * t * t, t * t * t);
            [
                b0 * from[0] + b1 * curve.first[0] + b2 * curve.second[0] + b3 * curve.end[0],
                b0 * from[1] + b1 * curve.first[1] + b2 * curve.second[1] + b3 * curve.end[1],
            ]
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Whether two points are the same to within a thousandth.
    fn same(a: [f32; 2], b: [f32; 2]) -> bool {
        (a[0] - b[0]).hypot(a[1] - b[1]) < 1e-3
    }

    /// Whether a point is outside the box by at least `by` on some side.
    fn outside(point: [f32; 2], box_: [f32; 4], by: f32) -> bool {
        point[0] <= box_[0] - by
            || point[0] >= box_[2] + by
            || point[1] <= box_[1] - by
            || point[1] >= box_[3] + by
    }

    /// Two points at 0, four at 1, six at 2; the table's range holds a value outside it; a wide
    /// line lifts the radius to its own width and a narrow one leaves it.
    #[test]
    fn the_radius_grows_by_two_points_per_unit_of_intensity_and_is_held_to_the_range() {
        assert!((radius(0.0, 1.0) - 2.0).abs() < f32::EPSILON);
        assert!((radius(1.0, 1.0) - 4.0).abs() < f32::EPSILON);
        assert!((radius(2.0, 1.0) - 6.0).abs() < f32::EPSILON);
        assert!(
            (radius(5.0, 1.0) - 6.0).abs() < f32::EPSILON,
            "above the range"
        );
        assert!((radius(-1.0, 1.0) - 2.0).abs() < f32::EPSILON, "below it");
        assert!(
            (radius(f32::NAN, 1.0) - 2.0).abs() < f32::EPSILON,
            "and not a number"
        );
        assert!(
            (radius(0.0, 6.0) - 6.0).abs() < f32::EPSILON,
            "a six-point line"
        );
        assert!(
            (radius(2.0, 6.0) - 6.0).abs() < f32::EPSILON,
            "which is what /I 2 gives anyway"
        );
    }

    /// A 40-point square at radius 5 divides each edge into four chords: sixteen scallops,
    /// thirty-two quarter arcs, every apex five points outside the square, closed at the start.
    #[test]
    fn a_square_is_scalloped_evenly_with_every_apex_outside_it() {
        let box_ = [10.0, 10.0, 50.0, 50.0];
        let cloud = cloud(&rectangle(box_), 5.0).expect("a cloud");
        assert_eq!(cloud.curves.len(), 32);
        assert!(same(cloud.start, [10.0, 10.0]), "{:?}", cloud.start);
        let last = cloud.curves.last().expect("a curve").end;
        assert!(same(last, cloud.start), "closed: {last:?}");
        for apex in cloud.curves.iter().step_by(2).map(|c| c.end) {
            assert!(
                outside(apex, box_, 4.99),
                "{apex:?} is an apex and lies five points outside"
            );
        }
        for cusp in cloud.curves.iter().skip(1).step_by(2).map(|c| c.end) {
            assert!(
                !outside(cusp, box_, 0.01),
                "{cusp:?} is a cusp and lies on the square"
            );
        }
    }

    /// The same square given clockwise still bulges outward: orientation is read, not assumed.
    #[test]
    fn a_clockwise_outline_bulges_outward_too() {
        let box_ = [10.0, 10.0, 50.0, 50.0];
        let mut edges = rectangle(box_);
        edges.reverse();
        for edge in &mut edges {
            edge.reverse();
        }
        let cloud = cloud(&edges, 5.0).expect("a cloud");
        for apex in cloud.curves.iter().step_by(2).map(|c| c.end) {
            assert!(outside(apex, box_, 4.99), "{apex:?}");
        }
    }

    /// An edge shorter than one chord still gets one scallop rather than none, so a shape is
    /// never left with a straight side because it was small.
    #[test]
    fn a_short_edge_gets_one_scallop() {
        let cloud = cloud(&rectangle([0.0, 0.0, 3.0, 3.0]), 5.0).expect("a cloud");
        assert_eq!(cloud.curves.len(), 8, "four edges, one scallop each");
    }

    /// An ellipse's cusps are spaced by arc length, so a circle of circumference 2π·50 at
    /// radius 5 has round(314/10) = 31 scallops, every apex outside the circle.
    #[test]
    fn a_circle_is_scalloped_by_arc_length() {
        let cloud = cloud(&ellipse([0.0, 0.0, 100.0, 100.0]), 5.0).expect("a cloud");
        assert_eq!(cloud.curves.len(), 62);
        for apex in cloud.curves.iter().step_by(2).map(|c| c.end) {
            let distance = (apex[0] - 50.0).hypot(apex[1] - 50.0);
            assert!(distance > 54.0, "{apex:?} is {distance} from the centre");
        }
    }

    /// Nothing to scallop is `None`, not a panic and not an empty path.
    #[test]
    fn a_degenerate_outline_or_radius_is_none() {
        assert!(cloud(&rectangle([5.0, 5.0, 5.0, 5.0]), 5.0).is_none());
        assert!(cloud(&rectangle([0.0, 0.0, 10.0, 10.0]), 0.0).is_none());
        assert!(cloud(&[], 5.0).is_none());
    }
}

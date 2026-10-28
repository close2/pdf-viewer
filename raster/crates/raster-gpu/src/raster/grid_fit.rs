//! ISO 32000-2 §10.7.5's first requirement at a placement: a thin rule along the device axes is
//! stroked one whole device pixel wide, through pixel centres.
//!
//! > When stroke adjustment is enabled, the line width and the coordinates of a stroke shall
//! > automatically be adjusted as necessary to produce lines of uniform thickness.
//!
//! The construction is the caller's own `pdf_render::grid_fit` (their ADR 2530), stated again
//! here because a scene states a stroke once and the placement that decides the fit is only known
//! at encode (ADR 0085). Under `/SA` a stroke whose device width is under [`FIT_BELOW`], under a
//! transform that stretches both axes alike, and whose every segment lies along a device axis has
//! each vertex moved, across the segments it ends, to the nearest pixel centre — at most half a
//! pixel, the clause's own bound — and is stroked one device pixel wide. A free end keeps where
//! the document ended it, a corner moves in both coordinates, and a curve or a diagonal keeps its
//! coordinates and its coverage-weighted thickness (their ADR 0848).
//!
//! **The fit is a function of the stroke and the placement**, the translation included, so a
//! fitted stroke is expanded at its placement rather than once under its linear part and moved
//! (ADR 1445): a rule fitted at one offset is a different set of pixels at another.

use raster_scene::{Point, Segment, Stroke};

use super::flatten::{DeviceTransform, Polyline, flatten_stroke};
use super::stroke::resolve_width;

/// The device width, in device pixels, below which a stroke is fitted to one whole pixel: one
/// whole pixel is within the clause's half a pixel of every width under this and of none over it.
/// The caller's `pdf_render::FIT_BELOW`, to the value.
pub(crate) const FIT_BELOW: f32 = 1.5;

/// How far, in device pixels, a segment's two ends may differ across an axis and still lie
/// along it: far under anything a reader could see, far over an `f32` page transform's rounding.
pub(crate) const ALONG_AN_AXIS: f32 = 1.0 / 64.0;

/// How far apart a transform's two stretches may be, as a fraction of the larger, for one stroke
/// width to be one device width.
pub(crate) const ALIKE: f32 = 1.0 / 1024.0;

/// The device polylines §10.7.5's first requirement strokes in place of `segments` under
/// `stroke` at the placement `t`, each to be stroked one device pixel wide — or `None` where it
/// asks for no fit, and the stroke is expanded as stated.
///
/// `None` for a stroke whose `/SA` is off, one at [`FIT_BELOW`] device pixels or wider, a path
/// with a curve or a segment off a device axis, and a transform that stretches the axes
/// differently or stretches nothing.
pub(crate) fn grid_fit(
    segments: &[Segment],
    stroke: Stroke,
    t: DeviceTransform,
) -> Option<Vec<Polyline>> {
    if !stroke.adjust {
        return None;
    }
    let widest = t.max_stretch();
    if !widest.is_finite() || widest <= 0.0 || widest - min_stretch(t) > ALIKE * widest {
        return None;
    }
    // `resolve_width` already holds §8.4.3.2's zero width and §10.7.5's half-pixel promotion, so
    // a stroke it makes one pixel is fitted for its position too. A width that is not a number is
    // not under the bound either.
    let device = resolve_width(stroke, t);
    if device.is_nan() || device >= FIT_BELOW {
        return None;
    }
    if segments
        .iter()
        .any(|segment| matches!(segment, Segment::CubicTo { .. }))
    {
        return None;
    }
    flatten_stroke(segments, t)
        .into_iter()
        .map(snapped)
        .collect()
}

/// `polyline` with each point moved across the segments it ends to the nearest pixel centre, or
/// `None` where a segment of it, the closing one included, lies off the device axes.
fn snapped(mut polyline: Polyline) -> Option<Polyline> {
    let count = polyline.points.len();
    let mut across = vec![(false, false); count];
    let closing = usize::from(polyline.closed);
    for from in 0..count.saturating_sub(1).saturating_add(closing) {
        // The closing segment's far end is the first point.
        let to = Some(from.saturating_add(1))
            .filter(|&next| next < count)
            .unwrap_or(0);
        let (p, q) = (polyline.points[from], polyline.points[to]);
        let horizontal = (q.y - p.y).abs() <= ALONG_AN_AXIS;
        let vertical = (q.x - p.x).abs() <= ALONG_AN_AXIS;
        match (horizontal, vertical) {
            // A segment of no length states no direction and moves nothing.
            (true, true) => {}
            (true, false) => (across[from].1, across[to].1) = (true, true),
            (false, true) => (across[from].0, across[to].0) = (true, true),
            (false, false) => return None,
        }
    }
    let centre = |v: f32| v.floor() + 0.5;
    for (point, (x, y)) in polyline.points.iter_mut().zip(across) {
        *point = Point::new(
            if x { centre(point.x) } else { point.x },
            if y { centre(point.y) } else { point.y },
        );
    }
    Some(polyline)
}

/// The smaller factor by which `t` lengthens a vector, mirrored statement for statement from the
/// caller's `Transform::min_stretch`, as [`DeviceTransform::max_stretch`] is from theirs.
fn min_stretch(t: DeviceTransform) -> f32 {
    let sum = t.a * t.a + t.b * t.b + t.c * t.c + t.d * t.d;
    let determinant = t.a * t.d - t.b * t.c;
    let discriminant = (sum * sum - 4.0 * determinant * determinant).max(0.0);
    f32::midpoint(sum, -discriminant.sqrt()).max(0.0).sqrt()
}

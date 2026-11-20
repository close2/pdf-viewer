//! A dash pattern too fine for any backend to cut, and the stroke drawn in its place — ISO 32000-2
//! §8.4.3.6 and §8.5.3.2, under principle 3's budgets.
//!
//! §8.4.3.6 admits any lengths that are "nonnegative and not all zero", so `[0.00001 0.00001] 0 d`
//! is a valid pattern, and along an 80-unit line it states four million dashes. Each backend's
//! dasher builds every one of them as geometry before a single pixel is drawn: `tiny-skia`'s
//! refuses past a million dashes a path and returns nothing, which drew the whole stroke as
//! nothing, and `kurbo`'s has no bound at all, so the same file costs the graphics device's
//! backends four million subpaths to build and expand. Neither is an answer a clause gives:
//! §8.5.3.2 has `S` paint the stroke "either solid or with a dash pattern", and a resource budget
//! that turns a valid pattern into no mark is trap 5's silent fallback.
//!
//! **The bound, and what is drawn past it, are a choice, decided once here for all three
//! backends** (trap 2; ADR 3334). Up to [`MAX_DASHES`] a pattern is cut as stated. Past it, the
//! dashes are finer than anything the bound lets a page draw one by one, and the stroke is drawn
//! solid at [`pattern_coverage`]: the share of the line the pattern's dashes and their caps
//! cover. That is §10.7.4's reading of a mark under the device's resolution — what reaches a pixel
//! is the area of the shape inside it — taken over a whole period of the pattern rather than per
//! dash, and it is exact wherever a period is finer than a pixel, which is where such counts are
//! met. The interpreter reports the bound by name where a stroke reaches it
//! (`Unsupported::LimitReached`), because a pattern drawn as its coverage is not the pattern.

use crate::geom::{Path, PathCommand, Point};
use crate::paint::{LineCap, Stroke};

/// The number of dashes a stroke's pattern may cut one path into before it is drawn as its
/// coverage instead.
///
/// Half of `tiny-skia`'s own refusal at a million (`MAX_DASH_COUNT` in its `dash.rs`), so that
/// [`dash_count`]'s estimate — an upper bound on the count that dasher makes, but computed in
/// another precision and over a polygon rather than its curve — can never admit a pattern that
/// dasher then refuses. Half a million dashes is about 56 MB of `kurbo` path elements on the
/// graphics device's backends, the most one stroke may build (ADR 3334).
pub const MAX_DASHES: f64 = 500_000.0;

/// An upper bound on the number of dashes `array` cuts `path` into.
///
/// The count a dasher makes is the path's length over the pattern's period, times the dashes in
/// one period, plus at most one per subpath for where the phase starts it. The length is taken
/// over each segment's control polygon, which is never shorter than the curve it controls, and
/// over each subpath's closing segment, which a dasher cuts as it cuts the rest. `0.0` for an
/// array that states no pattern (empty, or summing to nothing), which is stroked solid.
#[must_use]
pub fn dash_count(path: &Path, array: &[f32]) -> f64 {
    let period: f64 = array.iter().copied().map(f64::from).sum();
    if array.is_empty() || !period.is_finite() || period <= 0.0 {
        return 0.0;
    }
    #[expect(
        clippy::cast_precision_loss,
        reason = "a dash array's length is far under 2^52"
    )]
    let per_period = (array.len() as f64 / 2.0).max(1.0);
    let (length, subpaths) = polygon_length(path);
    length / period * per_period + subpaths
}

/// The coverage a stroke's dash pattern is drawn at in place of its dashes, or `None` where the
/// pattern is cut as stated: no pattern, or [`dash_count`] within [`MAX_DASHES`].
///
/// `width` is the stroke's width in the path's own space, as [`Stroke::device_width`] resolves it.
#[must_use]
pub fn past_the_dash_bound(path: &Path, stroke: &Stroke, width: f32) -> Option<f32> {
    (dash_count(path, &stroke.dash_array) > MAX_DASHES)
        .then(|| pattern_coverage(&stroke.dash_array, stroke.cap, width))
}

/// The share of a line's length a dash pattern's marks cover, from Table 53's three ends.
///
/// The even-indexed lengths are dashes and the odd ones gaps (§8.4.3.6). A butt-capped dash
/// covers its own length; a projecting square cap adds half the width at each end, so a whole
/// width a dash; a round cap adds a semicircle at each end, a disc of the width's diameter a dash,
/// whose area `π w² / 4` is a length of `π w / 4` across the line's width. Where the extended
/// dashes would overlap the share is one: the line is solid.
#[must_use]
pub fn pattern_coverage(array: &[f32], cap: LineCap, width: f32) -> f32 {
    let period: f32 = array.iter().sum();
    if period <= 0.0 || !period.is_finite() {
        return 1.0;
    }
    let dashes = array.iter().step_by(2);
    let on: f32 = dashes.clone().sum();
    #[expect(
        clippy::cast_precision_loss,
        reason = "a dash array's length is far under 2^24"
    )]
    let count = dashes.count() as f32;
    let width = width.max(0.0);
    let ends = match cap {
        LineCap::Butt => 0.0,
        LineCap::Square => width,
        LineCap::Round => std::f32::consts::FRAC_PI_4 * width,
    };
    ((on + count * ends) / period).clamp(0.0, 1.0)
}

/// The length of `path`'s control polygon, closing segments included, and its number of
/// subpaths.
fn polygon_length(path: &Path) -> (f64, f64) {
    // A square root rather than `hypot`: IEEE 754 rounds it exactly, so the count is the same
    // number on every platform and under Miri.
    let distance = |a: Point, b: Point| {
        let (dx, dy) = (
            f64::from(a.x) - f64::from(b.x),
            f64::from(a.y) - f64::from(b.y),
        );
        (dx * dx + dy * dy).sqrt()
    };
    let (mut length, mut subpaths) = (0.0, 0.0);
    let (mut current, mut start) = (Point::new(0.0, 0.0), Point::new(0.0, 0.0));
    for command in path.commands() {
        match *command {
            PathCommand::MoveTo(p) => {
                (current, start) = (p, p);
                subpaths += 1.0;
            }
            PathCommand::LineTo(p) => {
                length += distance(current, p);
                current = p;
            }
            PathCommand::CurveTo(c1, c2, p) => {
                length += distance(current, c1) + distance(c1, c2) + distance(c2, p);
                current = p;
            }
            PathCommand::Close => {
                length += distance(current, start);
                current = start;
            }
        }
    }
    (length, subpaths)
}

#[cfg(test)]
mod tests {
    use super::{MAX_DASHES, dash_count, past_the_dash_bound, pattern_coverage};
    use crate::geom::{Path, PathCommand, Point};
    use crate::paint::{LineCap, Stroke};

    fn rule(length: f32) -> Path {
        let mut path = Path::new();
        path.push(PathCommand::MoveTo(Point::new(0.0, 0.0)));
        path.push(PathCommand::LineTo(Point::new(length, 0.0)));
        path
    }

    /// An 80-unit rule under `[1 1]` is forty dashes, and one more for where the phase starts.
    #[test]
    fn a_rule_is_its_length_over_the_period() {
        assert_eq!(dash_count(&rule(80.0), &[1.0, 1.0]), 41.0);
        assert_eq!(dash_count(&rule(80.0), &[]), 0.0);
        assert_eq!(dash_count(&rule(80.0), &[0.0, 0.0]), 0.0);
    }

    /// The audit's `[0.00001 0.00001]` along 80 units is four million dashes: past the bound, and
    /// drawn at half the line, the share butt-capped dashes of equal on and off cover.
    #[test]
    fn a_pattern_past_the_bound_is_its_coverage() {
        let stroke = Stroke {
            dash_array: vec![0.000_01, 0.000_01],
            ..Stroke::default()
        };
        assert!(dash_count(&rule(80.0), &stroke.dash_array) > MAX_DASHES);
        assert_eq!(past_the_dash_bound(&rule(80.0), &stroke, 1.0), Some(0.5));
        let coarse = Stroke {
            dash_array: vec![0.01, 0.01],
            ..Stroke::default()
        };
        assert_eq!(past_the_dash_bound(&rule(80.0), &coarse, 1.0), None);
    }

    /// Table 53: a square cap adds a width to each dash and a round one `π w / 4`; overlapping
    /// extended dashes cover the whole line.
    #[test]
    fn the_caps_add_to_the_coverage() {
        assert_eq!(pattern_coverage(&[1.0, 3.0], LineCap::Butt, 1.0), 0.25);
        assert_eq!(pattern_coverage(&[1.0, 3.0], LineCap::Square, 1.0), 0.5);
        let round = pattern_coverage(&[1.0, 3.0], LineCap::Round, 1.0);
        assert!((round - (1.0 + std::f32::consts::FRAC_PI_4) / 4.0).abs() < 1e-6);
        assert_eq!(pattern_coverage(&[1.0, 1.0], LineCap::Square, 2.0), 1.0);
    }
}

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
//!
//! **The count is of what a region can show** ([`dashing`], ADR 3495). Length a path states off
//! the page marks no pixel, and a dasher handed only the part near the page, each piece entered
//! at the phase the whole subpath had there, cuts the same dashes on it. So a pattern past the
//! bound over the whole path is counted again over that part, and drawn as stated wherever that
//! count is within the bound. The bound still guards what a dasher is asked to build.

use crate::geom::{Path, PathCommand, Point, Rect, Transform};
use crate::paint::{LineCap, LineJoin, Stroke};

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

/// How one stroke's dash pattern is drawn, decided once for all three backends (trap 2).
#[derive(Debug, Clone, PartialEq)]
pub enum Dashing {
    /// Cut as stated over the path as given: no pattern, or a pattern whose [`dash_count`] over
    /// the whole path is within [`MAX_DASHES`].
    AsStated,
    /// Cut as stated over this path in place of the given one: the part of each subpath the
    /// [`DashRegion`] can show, each piece led in from off the region by a line whose length
    /// brings the pattern to the phase it has at that point of the subpath (ADR 3495).
    OnTheRegion(Path),
    /// Drawn solid at this coverage ([`pattern_coverage`]): the pattern is past the bound even
    /// over the part of the path the region can show (ADR 3334).
    Coverage(f32),
}

/// The rectangle a stroke can mark — the raster it is drawn into, or the page's own boundary —
/// and the map from the path's space onto it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DashRegion {
    /// The rectangle, in the space `from_path` maps onto.
    pub area: Rect,
    /// The path's space onto the area's.
    pub from_path: Transform,
}

/// How `stroke`'s pattern is drawn over `path`: as stated, as stated over the part `region` can
/// show, or solid at its coverage.
///
/// The count that decides is [`dash_count`], first over the whole path, which is every
/// ordinary page's answer and leaves its path untouched. Past [`MAX_DASHES`], and where a region
/// is given, the count is taken again over the path cut to it, which keeps of each
/// subpath of straight segments only what lies within `width`'s reach of the region (ADR 3495):
/// a dasher handed that path cuts every dash the region shows where the whole path cut it, and
/// the bound guards what is built rather than what the document stated off the page. `width` is
/// the stroke's width in the path's own space, as [`Stroke::device_width`] resolves it.
#[must_use]
pub fn dashing(path: &Path, stroke: &Stroke, width: f32, region: Option<DashRegion>) -> Dashing {
    let array = &stroke.dash_array;
    if dash_count(path, array) <= MAX_DASHES {
        return Dashing::AsStated;
    }
    if let Some(region) = region
        && let Some(cut) = on_the_region(path, stroke, width, region)
        && dash_count(&cut, array) <= MAX_DASHES
    {
        return Dashing::OnTheRegion(cut);
    }
    Dashing::Coverage(pattern_coverage(array, stroke.cap, width))
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

/// The path a dasher is handed in place of `path` where the region given to [`dashing`] cuts it,
/// or `None` where the region cannot be used: a map with no inverse, or a coordinate or a reach
/// that is not a number.
///
/// The region is grown by the reach of a mark made at its edge — half the width, times the
/// mitre limit under mitre joins or `√2` under projecting caps, whichever is longer, plus one unit
/// of the region's space for the whole pixel §10.7.4's clipping region grows the boundary to
/// (`crop_area`) — so that nothing removed, and no end or join the cut adds, reaches it. Each
/// subpath is then kept or cut:
///
/// - **Kept as stated** where every point of it lies in the grown region; where it holds a curve,
///   whose length is the dasher's own measure and so not a phase this can carry; where it is
///   closed and starts in the grown region, because opening it would put two ends where the
///   close drew a join; and where it is not one `m` followed by `l`s and at most a closing `h`.
/// - **Cut** otherwise, its closing segment a line like the rest: the part of each segment inside
///   the grown region is kept, and each run of kept parts that does not continue the subpath's
///   start is a subpath of its own. §8.4.3.6 restarts the pattern at the phase on every subpath,
///   so each such piece begins with a lead-in: a line from off the grown region, pointing away
///   from it, whose length in the path's space is the distance the subpath had travelled at the
///   piece's start, modulo the pattern's period. The dashes the lead-in makes lie wholly outside
///   the grown region, and the piece's first dash then starts where the whole path's did.
fn on_the_region(path: &Path, stroke: &Stroke, width: f32, region: DashRegion) -> Option<Path> {
    let period: f64 = stroke.dash_array.iter().copied().map(f64::from).sum();
    let map = Affine::from(region.from_path)?;
    let join = if stroke.join == LineJoin::Miter {
        f64::from(stroke.miter_limit).max(1.0)
    } else {
        1.0
    };
    let cap = if stroke.cap == LineCap::Square {
        std::f64::consts::SQRT_2
    } else {
        1.0
    };
    let reach = f64::from(width.max(0.0)) * f64::from(region.from_path.max_stretch()) / 2.0
        * join.max(cap)
        + 1.0;
    let area = Area {
        min: (
            f64::from(region.area.min.x) - reach,
            f64::from(region.area.min.y) - reach,
        ),
        max: (
            f64::from(region.area.max.x) + reach,
            f64::from(region.area.max.y) + reach,
        ),
    };
    if !(period.is_finite() && period > 0.0 && reach.is_finite()) {
        return None;
    }
    let mut cut = Path::new();
    let commands = path.commands();
    // Where each subpath starts: at every `m`, and at the first command whatever it is.
    let mut starts: Vec<usize> = commands
        .iter()
        .enumerate()
        .filter(|(_, command)| matches!(command, PathCommand::MoveTo(_)))
        .map(|(at, _)| at)
        .collect();
    if starts.first() != Some(&0) {
        starts.insert(0, 0);
    }
    let ends = starts.iter().skip(1).copied().chain([commands.len()]);
    for (start, end) in starts.iter().copied().zip(ends) {
        let subpath = &commands[start..end];
        if !cut_subpath(subpath, &map, &area, period, &mut cut)? {
            cut.extend(subpath);
        }
    }
    Some(cut)
}

/// Pushes the pieces of `subpath` the grown region shows onto `cut`, and answers whether it did:
/// `false` where the subpath is to be kept as stated ([`on_the_region`] says when), `None` where
/// a coordinate is not a number.
fn cut_subpath(
    subpath: &[PathCommand],
    map: &Affine,
    area: &Area,
    period: f64,
    cut: &mut Path,
) -> Option<bool> {
    let Some((PathCommand::MoveTo(first), rest)) = subpath.split_first() else {
        return Some(false);
    };
    let (closed, open_part) = match rest.split_last() {
        Some((PathCommand::Close, front)) => (true, front),
        _ => (false, rest),
    };
    let mut points = vec![*first];
    for command in open_part {
        let PathCommand::LineTo(p) = *command else {
            return Some(false);
        };
        points.push(p);
    }
    if closed {
        points.push(*first);
    }
    let mapped: Vec<(f64, f64)> = points.iter().map(|&p| map.apply(p)).collect();
    if mapped
        .iter()
        .any(|&(x, y)| !(x.is_finite() && y.is_finite()))
    {
        return None;
    }
    if mapped.iter().all(|&m| area.holds(m)) || (closed && area.holds(mapped[0])) {
        return Some(false);
    }
    let (mut travelled, mut piece_open) = (0.0, false);
    for (pair, image) in points.windows(2).zip(mapped.windows(2)) {
        let (a, b) = (pair[0], pair[1]);
        let length = distance(a, b);
        if let Some((t0, t1)) = area.clip(image[0], image[1]) {
            if !piece_open || t0 > 0.0 {
                let entry = along(a, b, t0);
                let lead = (travelled + t0 * length).rem_euclid(period);
                if lead > 0.0 {
                    let away = map.away(area, map.apply(entry), lead);
                    #[expect(
                        clippy::cast_possible_truncation,
                        reason = "a lead-in is shorter than one period of a pattern whose \\
                                  lengths are f32, beside a point inside the grown region"
                    )]
                    let from = Point::new(entry.x - away.0 as f32, entry.y - away.1 as f32);
                    cut.push(PathCommand::MoveTo(from));
                    cut.push(PathCommand::LineTo(entry));
                } else {
                    cut.push(PathCommand::MoveTo(entry));
                }
            }
            cut.push(PathCommand::LineTo(along(a, b, t1)));
            piece_open = t1 >= 1.0;
        } else {
            piece_open = false;
        }
        travelled += length;
    }
    Some(true)
}

/// The point at `t` along the segment from `a` to `b`, exactly `a` and `b` at its ends.
fn along(a: Point, b: Point, t: f64) -> Point {
    if t <= 0.0 {
        return a;
    }
    if t >= 1.0 {
        return b;
    }
    let lerp = |p: f32, q: f32| f64::from(p) + t * (f64::from(q) - f64::from(p));
    #[expect(
        clippy::cast_possible_truncation,
        reason = "a point between two f32 points is back in f32's range"
    )]
    Point::new(lerp(a.x, b.x) as f32, lerp(a.y, b.y) as f32)
}

/// The length of the segment from `a` to `b`.
///
/// A square root rather than `hypot`: IEEE 754 rounds it exactly, so a count is the same number
/// on every platform and under Miri.
fn distance(a: Point, b: Point) -> f64 {
    let (dx, dy) = (
        f64::from(a.x) - f64::from(b.x),
        f64::from(a.y) - f64::from(b.y),
    );
    (dx * dx + dy * dy).sqrt()
}

/// The grown region, in double precision: a path [`on_the_region`] cuts states coordinates far
/// past single precision's resolution of the region's.
struct Area {
    min: (f64, f64),
    max: (f64, f64),
}

impl Area {
    /// Whether `p` lies in the area, its boundary included.
    fn holds(&self, (x, y): (f64, f64)) -> bool {
        (self.min.0..=self.max.0).contains(&x) && (self.min.1..=self.max.1).contains(&y)
    }

    /// The parameters at which the segment from `a` to `b` enters and leaves the area, or `None`
    /// where it misses it: Liang and Barsky's clipping of a parametric line against the four
    /// half-planes, each narrowing `[0, 1]`.
    fn clip(&self, a: (f64, f64), b: (f64, f64)) -> Option<(f64, f64)> {
        let (dx, dy) = (b.0 - a.0, b.1 - a.1);
        let (mut enter, mut leave) = (0.0_f64, 1.0_f64);
        for (towards, room) in [
            (-dx, a.0 - self.min.0),
            (dx, self.max.0 - a.0),
            (-dy, a.1 - self.min.1),
            (dy, self.max.1 - a.1),
        ] {
            if towards == 0.0 {
                if room < 0.0 {
                    return None;
                }
            } else {
                let at = room / towards;
                if towards < 0.0 {
                    enter = enter.max(at);
                } else {
                    leave = leave.min(at);
                }
            }
        }
        (enter <= leave).then_some((enter, leave))
    }
}

/// An affine map in double precision, with the inverse of its linear part.
struct Affine {
    /// The matrix `[a b c d e f]` as [`Transform`] states it, widened.
    map: [f64; 6],
    /// The inverse of its linear part, `[a b c d]` in the same order.
    inverse: [f64; 4],
}

impl Affine {
    /// `transform` widened, or `None` where it has no inverse.
    fn from(transform: Transform) -> Option<Self> {
        let map = [
            transform.a,
            transform.b,
            transform.c,
            transform.d,
            transform.e,
            transform.f,
        ]
        .map(f64::from);
        let determinant = map[0] * map[3] - map[1] * map[2];
        if determinant == 0.0 || !determinant.is_finite() {
            return None;
        }
        Some(Self {
            map,
            inverse: [
                map[3] / determinant,
                -map[1] / determinant,
                -map[2] / determinant,
                map[0] / determinant,
            ],
        })
    }

    /// `point` in the area's space.
    fn apply(&self, point: Point) -> (f64, f64) {
        let m = self.map;
        let (x, y) = (f64::from(point.x), f64::from(point.y));
        (m[0] * x + m[2] * y + m[4], m[1] * x + m[3] * y + m[5])
    }

    /// The vector in the path's space, `length` long there, whose image points from the area
    /// towards `at`: from the nearest point of the area, or along the outward normal of the side
    /// `at` lies on. Every point a lead-in ending at `at` passes through then lies farther from
    /// the area than `at` does, which is what keeps its dashes off it.
    fn away(&self, area: &Area, at: (f64, f64), length: f64) -> (f64, f64) {
        let nearest = (
            at.0.clamp(area.min.0, area.max.0),
            at.1.clamp(area.min.1, area.max.1),
        );
        let mut out = (at.0 - nearest.0, at.1 - nearest.1);
        if out == (0.0, 0.0) {
            let sides = [
                (at.0 - area.min.0, (-1.0, 0.0)),
                (area.max.0 - at.0, (1.0, 0.0)),
                (at.1 - area.min.1, (0.0, -1.0)),
                (area.max.1 - at.1, (0.0, 1.0)),
            ];
            out = sides
                .iter()
                .min_by(|p, q| p.0.total_cmp(&q.0))
                .map_or((1.0, 0.0), |side| side.1);
        }
        let inverse = self.inverse;
        let back = (
            inverse[0] * out.0 + inverse[2] * out.1,
            inverse[1] * out.0 + inverse[3] * out.1,
        );
        let norm = back.0.hypot(back.1);
        (back.0 / norm * length, back.1 / norm * length)
    }
}

#[cfg(test)]
mod tests {
    use super::{DashRegion, Dashing, MAX_DASHES, dash_count, dashing, pattern_coverage};
    use crate::geom::{Path, PathCommand, Point, Rect, Transform};
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
        assert_eq!(
            dashing(&rule(80.0), &stroke, 1.0, None),
            Dashing::Coverage(0.5)
        );
        let coarse = Stroke {
            dash_array: vec![0.01, 0.01],
            ..Stroke::default()
        };
        assert_eq!(dashing(&rule(80.0), &coarse, 1.0, None), Dashing::AsStated);
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

    /// A 100 by 20 region at the origin, under the identity.
    fn region() -> DashRegion {
        DashRegion {
            area: Rect::from_corners(Point::new(0.0, 0.0), Point::new(100.0, 20.0)),
            from_path: Transform::IDENTITY,
        }
    }

    fn polyline(points: &[(f32, f32)], close: bool) -> Path {
        let mut path = Path::new();
        for (index, &(x, y)) in points.iter().enumerate() {
            let p = Point::new(x, y);
            path.push(if index == 0 {
                PathCommand::MoveTo(p)
            } else {
                PathCommand::LineTo(p)
            });
        }
        if close {
            path.push(PathCommand::Close);
        }
        path
    }

    /// A dash, from its start to its end.
    type Segment = ((f64, f64), (f64, f64));

    /// §8.4.3.6's dashes of a path of straight segments, each subpath restarting at the phase:
    /// the on intervals as segments, in double precision, independently of the code under test.
    fn dashes(path: &Path, array: &[f32], phase: f32) -> Vec<Segment> {
        let array: Vec<f64> = array.iter().copied().map(f64::from).collect();
        let period: f64 = array.iter().sum();
        let mut out = Vec::new();
        let mut subpaths: Vec<Vec<(f64, f64)>> = Vec::new();
        for command in path.commands() {
            match *command {
                PathCommand::MoveTo(p) => subpaths.push(vec![(f64::from(p.x), f64::from(p.y))]),
                PathCommand::LineTo(p) => {
                    if let Some(last) = subpaths.last_mut() {
                        last.push((f64::from(p.x), f64::from(p.y)));
                    }
                }
                PathCommand::Close => {
                    if let Some(last) = subpaths.last_mut() {
                        last.push(last[0]);
                    }
                }
                PathCommand::CurveTo(..) => unreachable!("the reference dashes lines only"),
            }
        }
        for points in subpaths {
            let mut at = f64::from(phase).rem_euclid(period);
            for pair in points.windows(2) {
                let (a, b) = (pair[0], pair[1]);
                let length = (b.0 - a.0).hypot(b.1 - a.1);
                let mut done = 0.0;
                while done < length {
                    // Where in the pattern `at` is, and how much of that entry is left.
                    let (mut index, mut into) = (0, at);
                    while into >= array[index] {
                        into -= array[index];
                        index = index.wrapping_add(1).checked_rem(array.len()).unwrap_or(0);
                    }
                    let step = (array[index] - into).min(length - done);
                    if index % 2 == 0 && step > 0.0 {
                        let point = |d: f64| {
                            (
                                a.0 + (b.0 - a.0) * d / length,
                                a.1 + (b.1 - a.1) * d / length,
                            )
                        };
                        out.push((point(done), point(done + step)));
                    }
                    done += step;
                    at = (at + step).rem_euclid(period);
                }
            }
        }
        out
    }

    /// The parts of `dashes` inside the ungrown region, each clipped to it, in order.
    fn on_the_page(dashes: &[Segment]) -> Vec<Segment> {
        dashes
            .iter()
            .filter_map(|&(a, b)| {
                // Only horizontal and vertical dashes reach here: clip each axis.
                let clamp = |p: (f64, f64)| (p.0.clamp(0.0, 100.0), p.1.clamp(0.0, 20.0));
                let inside =
                    |p: (f64, f64)| (0.0..=100.0).contains(&p.0) && (0.0..=20.0).contains(&p.1);
                let crosses = (a.1 - b.1).abs() < 1e-9
                    && (0.0..=20.0).contains(&a.1)
                    && a.0.min(b.0) < 100.0
                    && a.0.max(b.0) > 0.0;
                (inside(a) || inside(b) || crosses).then(|| (clamp(a), clamp(b)))
            })
            .filter(|(a, b)| (a.0 - b.0).hypot(a.1 - b.1) > 1e-6)
            .collect()
    }

    /// ADR 3495: a rule ten million units long under `[1 1]` is past the bound over its length
    /// and far within it over the region, and the dashes the region shows are the ones the whole
    /// rule cut, the lead-in having brought the pattern to the phase it had at the entry.
    #[test]
    fn a_pattern_past_the_bound_off_the_page_is_cut_on_it() {
        let path = polyline(&[(-1.0e7 - 0.25, 10.0), (1.0e7, 10.0)], false);
        let stroke = Stroke {
            dash_array: vec![1.0, 1.0],
            dash_phase: 0.5,
            ..Stroke::default()
        };
        assert!(dash_count(&path, &stroke.dash_array) > MAX_DASHES);
        let Dashing::OnTheRegion(cut) = dashing(&path, &stroke, 1.0, Some(region())) else {
            panic!("the rule is within the bound over the region");
        };
        assert!(dash_count(&cut, &stroke.dash_array) < 100.0);
        let whole = on_the_page(&dashes(&path, &stroke.dash_array, stroke.dash_phase));
        let drawn = on_the_page(&dashes(&cut, &stroke.dash_array, stroke.dash_phase));
        assert_eq!(whole.len(), drawn.len());
        for (w, d) in whole.iter().zip(&drawn) {
            assert!(
                (w.0.0 - d.0.0).abs() < 1e-3 && (w.1.0 - d.1.0).abs() < 1e-3,
                "{w:?} {d:?}"
            );
        }
        // Every point of the cut path lies within the reach of the region: half the width under
        // the mitre limit of ten, and one unit, so 6 units; the lead-in at most a period beyond.
        for command in cut.commands() {
            if let PathCommand::MoveTo(p) | PathCommand::LineTo(p) = *command {
                assert!(p.x >= -8.0 && p.x <= 108.0, "{p:?}");
            }
        }
    }

    /// A closed subpath leaves the region many times and enters it again; each piece keeps the
    /// phase, and no lead-in's dash reaches the region.
    #[test]
    fn every_piece_of_a_cut_subpath_keeps_its_phase() {
        let mut points = Vec::new();
        for step in 0..40 {
            #[expect(clippy::cast_precision_loss, reason = "a small counter")]
            let y = 0.5 * step as f32;
            if step % 2 == 0 {
                points.extend([(-3.0e5, y), (3.0e5, y)]);
            } else {
                points.extend([(3.0e5, y), (-3.0e5, y)]);
            }
        }
        points.insert(0, (-4.0e5, -4.0e5));
        let path = polyline(&points, true);
        let stroke = Stroke {
            dash_array: vec![0.3, 0.2, 0.1, 0.4],
            dash_phase: 0.05,
            ..Stroke::default()
        };
        let Dashing::OnTheRegion(cut) = dashing(&path, &stroke, 0.5, Some(region())) else {
            panic!("the rules are within the bound over the region");
        };
        let whole = on_the_page(&dashes(&path, &stroke.dash_array, stroke.dash_phase));
        let drawn = on_the_page(&dashes(&cut, &stroke.dash_array, stroke.dash_phase));
        assert_eq!(whole.len(), drawn.len());
        for (w, d) in whole.iter().zip(&drawn) {
            let gap = (w.0.0 - d.0.0).abs() + (w.1.0 - d.1.0).abs() + (w.0.1 - d.0.1).abs();
            assert!(gap < 1e-2, "{w:?} {d:?}");
        }
    }

    /// What the cut keeps as stated: a closed subpath starting on the region, and a curve. Each
    /// is then counted whole, and a pattern past the bound over it is drawn as its coverage.
    #[test]
    fn a_closed_subpath_starting_on_the_region_and_a_curve_are_not_cut() {
        let stroke = Stroke {
            dash_array: vec![1.0, 1.0],
            ..Stroke::default()
        };
        let closed = polyline(&[(50.0, 10.0), (2.0e6, 10.0), (2.0e6, 11.0)], true);
        assert_eq!(
            dashing(&closed, &stroke, 1.0, Some(region())),
            Dashing::Coverage(0.5)
        );
        let mut curve = Path::new();
        curve.push(PathCommand::MoveTo(Point::new(-2.0e6, 10.0)));
        curve.push(PathCommand::CurveTo(
            Point::new(0.0, 10.0),
            Point::new(50.0, 10.0),
            Point::new(2.0e6, 10.0),
        ));
        assert_eq!(
            dashing(&curve, &stroke, 1.0, Some(region())),
            Dashing::Coverage(0.5)
        );
    }

    /// A pattern past the bound over the region itself is still drawn as its coverage: the bound
    /// guards what a dasher is asked to build.
    #[test]
    fn a_pattern_past_the_bound_on_the_page_is_its_coverage() {
        let stroke = Stroke {
            dash_array: vec![0.000_01, 0.000_01],
            ..Stroke::default()
        };
        assert_eq!(
            dashing(&rule(80.0), &stroke, 1.0, Some(region())),
            Dashing::Coverage(0.5)
        );
    }
}

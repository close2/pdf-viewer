//! What a stroke deposits where its path has no length: ISO 32000-2 §8.5.3.2.
//!
//! A subpath can enclose no area *and* span no distance — `10 10 m h S`, or a `l` back to
//! the point it started from. The clause says exactly what such a subpath paints, and the
//! answer depends only on the line cap:
//!
//! > If a subpath is degenerate (consists of a single-point closed path or of two or more
//! > points at the same coordinates), the S operator shall paint it only if round line caps
//! > have been specified, producing a filled circle centred at the single point. If butt or
//! > projecting square line caps have been specified, S shall produce no output, because the
//! > orientation of the caps would be indeterminate.
//!
//! and, in the same paragraph, that a subpath which is only a `m` paints nothing at all:
//!
//! > A single-point open subpath (specified by a trailing m operator) shall produce no
//! > output.
//!
//! # Why this is not left to the rasterisers
//!
//! Because they answer it differently, and neither answers it the way the clause does. This
//! was measured rather than assumed, by stroking each shape at width 10 on a 100-unit page
//! and summing the ink:
//!
//! ```text
//! subpath              cap       tiny-skia   Vello    §8.5.3.2
//! m h                  butt          0.0       0.0     nothing
//! m h                  round        77.5       0.0     a circle, area 78.5
//! m h                  square      100.0       0.0     nothing
//! m (alone)            any         refused     0.0     nothing
//! ```
//!
//! `tiny-skia` follows Skia, whose stroker says "square caps and round caps draw even if the
//! segment length is zero"; `kurbo` drops a contour that expanded to nothing before it ever
//! reaches a cap. So the CPU backend painted a square where the standard asks for nothing,
//! the GPU backend painted nothing where it asks for a circle, and a path consisting only of
//! `m` was an *error* on one backend and silence on the other. That is trap 2's shape
//! exactly — a decision either backend can make alone is a decision neither has made — which
//! is why the rule is stated here, in the crate both of them consume, and why the circle is
//! this crate's own geometry rather than whatever each rasteriser's round cap happens to be.
//!
//! # The other half of the clause, which is not this rule
//!
//! > This rule shall apply only to zero-length subpaths of the path being stroked, and not
//! > to zero-length dashes in a dash pattern of a non-degenerate subpath. In the latter
//! > case, the line caps shall always be painted, since their orientation is determined by
//! > the direction of the underlying path except in the case of a degenerate subpath.
//!
//! So `[0 6] 0 d 1 J S` is a *dotted line* and every dot is painted, including under a
//! projecting square cap — the opposite answer to the one above, for a shape that looks the
//! same. [`dash_mark`] is that rule, and it needs the direction the other one cannot have.

use crate::geom::{Path, PathCommand, Point};
use crate::paint::LineCap;

/// A stroked path separated into the part with length and the part without.
///
/// Produced by [`split_degenerate`], which returns nothing at all for a path that has no
/// degenerate subpath — the overwhelmingly common case, and the one that must not allocate.
#[derive(Debug, Clone, PartialEq)]
pub struct DegenerateStroke {
    /// The subpaths that span a distance, to be stroked as the command asks.
    ///
    /// Empty when every subpath was degenerate, in which case nothing is stroked at all.
    pub stroked: Path,
    /// The circles §8.5.3.2 asks for, to be **filled** with the stroking paint.
    ///
    /// Empty under butt and projecting square caps, where the clause asks for no output.
    pub dots: Path,
}

/// Control-point distance for a quarter circle of unit radius.
///
/// The classic four-cubic circle: `4/3 · (√2 − 1)`, which is `0.552_284_749_8` and rounds to
/// this at `f32`. Its radial error peaks at about 0.027% of the radius — a fifth of a
/// thousandth of a pixel on a ten-pixel dot — and both backends receive the *same*
/// approximation, so it cannot be a reason for them to disagree.
const KAPPA: f32 = 0.552_284_8;

/// Splits a path's degenerate subpaths out of it, ISO 32000-2 §8.5.3.2.
///
/// `width` is the width the stroke will actually be drawn at — [`Stroke::device_width`],
/// not the field — because §8.4.3.2's zero-width minimum decides a dot's diameter just as it
/// decides a line's thickness: `0 w 1 J` at a single point is a one-device-pixel dot.
///
/// Returns `None` when there is nothing to separate, so that a path without degenerate
/// subpaths costs one pass over its commands and no allocation.
///
/// [`Stroke::device_width`]: crate::paint::Stroke::device_width
#[must_use]
pub fn split_degenerate(path: &Path, cap: LineCap, width: f32) -> Option<DegenerateStroke> {
    if !matches!(path.commands().first(), Some(PathCommand::MoveTo(_))) {
        // §8.5.2.1: "the first one invoked shall be m or re to begin a new subpath", and a
        // segment with no current point is an error rather than a shape. No corpus
        // document's first page produces one; passing such a path through unchanged leaves
        // whatever the rasteriser makes of it rather than inventing a subpath to classify.
        return None;
    }
    if !subpaths(path).any(|s| s.degenerate) {
        return None;
    }

    let mut stroked = Path::new();
    let mut dots = Path::new();
    for subpath in subpaths(path) {
        if subpath.degenerate {
            // A single-point *open* subpath — a `m` with nothing after it — paints nothing
            // under any cap, which is the clause's last sentence and is not the same rule as
            // the one above it.
            if subpath.segments > 0 && cap == LineCap::Round {
                append_circle(&mut dots, subpath.start, width);
            }
        } else {
            stroked.extend(&path.commands()[subpath.range()]);
        }
    }
    Some(DegenerateStroke { stroked, dots })
}

/// The length a zero-length dash is dispensed at so that its direction survives dashing.
///
/// A dash of no length arrives from any dasher as a point: two coordinates that are equal,
/// or — because a dash's position is found by inverting an arc length — very nearly equal.
/// Either way the direction §8.5.3.2 wants for a projecting square cap is gone, and both
/// `tiny-skia` and `kurbo` throw it away for the same reason. Dispensing the dash at a
/// length instead keeps it: the dash comes back as a segment pointing the way the path was
/// going, and 1/1000 of a user space unit is a hundredth of the thinnest line a 300 dpi
/// device draws, so nothing about where the mark lands changes.
///
/// The alternative was searching the source path for the segment nearest each dash, which is
/// quadratic in the number of dashes, needs a nearest-point-on-cubic solver, and would have
/// to exist twice because the two backends hold their geometry in different libraries'
/// types.
pub const ZERO_DASH: f32 = 1e-3;

/// The dash pattern to dispense so that a zero-length dash's caps can be oriented.
///
/// Returns `None` when nothing needs substituting, which is every pattern that has no
/// zero-length dash and every pattern under a butt cap — where §8.5.3.2's "the line caps
/// shall always be painted" deposits nothing, a butt cap having no extent.
///
/// Even-numbered entries are dashes and odd-numbered ones are gaps (§8.4.3.6), and the
/// length given to a dash is taken back from the gap that follows it, so the pattern's period
/// is unchanged to the bit and every dash lands exactly where it would have. A gap too short
/// to lend it is not something any pattern this renderer builds contains — `pdf-model`
/// normalises an odd-length array by repeating it, so entries pair up — and returning `None`
/// there leaves the pattern alone rather than moving a dash.
#[must_use]
pub fn dashes_showing_direction(array: &[f32], cap: LineCap) -> Option<Vec<f32>> {
    if cap == LineCap::Butt {
        return None;
    }
    // `<= 0.0` rather than `== 0.0`: every entry is non-negative by the time a display list
    // carries one, so the two are the same test, and this one needs no float comparison.
    if !array.iter().step_by(2).any(|dash| *dash <= 0.0) {
        return None;
    }
    let mut out = array.to_vec();
    for index in (0..out.len().saturating_sub(1)).step_by(2) {
        let (dash, gap) = (out[index], out[index.saturating_add(1)]);
        if dash > 0.0 {
            continue;
        }
        if gap <= ZERO_DASH * 2.0 {
            return None;
        }
        out[index] = ZERO_DASH;
        out[index.saturating_add(1)] = gap - ZERO_DASH;
    }
    Some(out)
}

/// Separates a *dashed* path's zero-length dashes from the rest, §8.5.3.2's second rule.
///
/// `dashed` is what a backend's own dasher produced from the pattern
/// [`dashes_showing_direction`] returned, so each dash the document gave no length is a
/// segment of about [`ZERO_DASH`] pointing along the path. Those become marks; the rest is
/// stroked, with the dash pattern removed since the dashes have already been dispensed.
///
/// The walk is here rather than in each backend because the two hold their geometry in
/// different libraries' types and would otherwise each decide for themselves what counts as
/// a dash of no length and which way its cap faces — which is the whole of trap 2.
#[must_use]
pub fn split_dash_marks(dashed: &Path, cap: LineCap, width: f32) -> DegenerateStroke {
    let mut stroked = Path::new();
    let mut dots = Path::new();
    for subpath in subpaths(dashed) {
        let span = Point::new(
            subpath.end.x - subpath.start.x,
            subpath.end.y - subpath.start.y,
        );
        // Twice `ZERO_DASH`, so that the arc-length inversion each dasher uses to place a
        // dash cannot move one across the boundary. The shortest positive dash entry in the
        // whole 974-document corpus is 0.02, ten times this; a dash truncated by the end of
        // the path could be shorter, and drawing its cap is what §8.5.3.2 asks for there too.
        if span.x.hypot(span.y) <= ZERO_DASH * 2.0 {
            dash_mark(&mut dots, subpath.start, width, cap, span);
        } else {
            stroked.extend(&dashed.commands()[subpath.range()]);
        }
    }
    DegenerateStroke { stroked, dots }
}

/// The mark a zero-length *dash* leaves, ISO 32000-2 §8.5.3.2's second rule.
///
/// Distinct from [`split_degenerate`] in its answer, not only in its inputs: a dash of no
/// length still paints its caps, "since their orientation is determined by the direction of
/// the underlying path". So a round cap is the same circle, a butt cap has no extent and
/// therefore still marks nothing, and a projecting square cap — which paints nothing at all
/// on a degenerate *subpath* — here paints a square of the line's width, turned to face
/// along `direction`.
///
/// `direction` need not be normalised and may be zero, which is §8.4.3.4's own case:
///
/// > A zero length dash occurring at a zero length subpath segment does not have a
/// > determinable direction and thus, if the line caps are non-round is rendered in an
/// > implementation-dependent manner.
///
/// With no direction to face, a square cap is given the round cap's circle: it is the one
/// shape that is the same under every orientation the square could have had, so the choice
/// does not depend on which of them a reader guesses.
pub fn dash_mark(into: &mut Path, centre: Point, width: f32, cap: LineCap, direction: Point) {
    let length = direction.x.hypot(direction.y);
    // With no direction to face, a square cap falls through to the circle: see the note
    // above on §8.4.3.4, which makes that case implementation-dependent.
    let oriented_square = cap == LineCap::Square && length > 0.0;
    match cap {
        LineCap::Butt => {}
        _ if !oriented_square => append_circle(into, centre, width),
        _ => {
            let half = width / 2.0;
            let along = Point::new(direction.x / length * half, direction.y / length * half);
            let across = Point::new(-along.y, along.x);
            let corner = |sa: f32, sb: f32| {
                Point::new(
                    centre.x + sa * along.x + sb * across.x,
                    centre.y + sa * along.y + sb * across.y,
                )
            };
            into.push(PathCommand::MoveTo(corner(1.0, 1.0)));
            into.push(PathCommand::LineTo(corner(-1.0, 1.0)));
            into.push(PathCommand::LineTo(corner(-1.0, -1.0)));
            into.push(PathCommand::LineTo(corner(1.0, -1.0)));
            into.push(PathCommand::Close);
        }
    }
}

/// Appends a circle of diameter `width` centred at `centre`, as four cubic segments.
fn append_circle(into: &mut Path, centre: Point, width: f32) {
    let r = width / 2.0;
    if r <= 0.0 || !r.is_finite() || !centre.x.is_finite() || !centre.y.is_finite() {
        return;
    }
    let k = r * KAPPA;
    let (cx, cy) = (centre.x, centre.y);
    into.push(PathCommand::MoveTo(Point::new(cx + r, cy)));
    into.push(PathCommand::CurveTo(
        Point::new(cx + r, cy + k),
        Point::new(cx + k, cy + r),
        Point::new(cx, cy + r),
    ));
    into.push(PathCommand::CurveTo(
        Point::new(cx - k, cy + r),
        Point::new(cx - r, cy + k),
        Point::new(cx - r, cy),
    ));
    into.push(PathCommand::CurveTo(
        Point::new(cx - r, cy - k),
        Point::new(cx - k, cy - r),
        Point::new(cx, cy - r),
    ));
    into.push(PathCommand::CurveTo(
        Point::new(cx + k, cy - r),
        Point::new(cx + r, cy - k),
        Point::new(cx + r, cy),
    ));
    into.push(PathCommand::Close);
}

/// One subpath's extent within a path's command list, and whether it has any length.
struct Subpath {
    /// Index of the subpath's first command.
    first: usize,
    /// Index one past its last command.
    last: usize,
    /// Where the subpath begins. Every point of a degenerate subpath equals it.
    start: Point,
    /// The last point the subpath reaches, which for a dash is where it points.
    end: Point,
    /// How many commands add a segment. Zero means a lone `m`.
    segments: usize,
    /// True when the subpath spans no distance at all.
    degenerate: bool,
}

impl Subpath {
    fn range(&self) -> core::ops::Range<usize> {
        self.first..self.last
    }
}

/// Walks a path's subpaths.
///
/// A subpath ends at the next `m` or at a `h`, because Table 58 says `h` "terminates the
/// current subpath. Appending another segment to the current path shall begin a new subpath,
/// even if the new segment begins at the endpoint reached by the h operation" — and that new
/// subpath begins where the closed one did, which is the point `h` returned to.
fn subpaths(path: &Path) -> impl Iterator<Item = Subpath> + '_ {
    let commands = path.commands();
    let mut index = 0usize;
    // The point the most recent `m` set. A subpath that begins after an `h` has no `m` of
    // its own and starts here — reading a `l` after an `h` as starting where it *ends* would
    // call every such subpath degenerate and turn a segment into a dot.
    let mut opened_at: Option<Point> = None;
    core::iter::from_fn(move || {
        loop {
            if index >= commands.len() {
                return None;
            }
            let first = index;
            let mut start = None;
            let mut end = None;
            let mut segments = 0usize;
            let mut degenerate = true;
            while index < commands.len() {
                match commands[index] {
                    PathCommand::MoveTo(p) => {
                        if start.is_some() {
                            break;
                        }
                        opened_at = Some(p);
                        start = Some(p);
                    }
                    PathCommand::LineTo(p) => {
                        segments = segments.saturating_add(1);
                        let from = *start.get_or_insert(opened_at.unwrap_or(p));
                        degenerate &= p == from;
                        end = Some(p);
                    }
                    PathCommand::CurveTo(a, b, at) => {
                        segments = segments.saturating_add(1);
                        let from = *start.get_or_insert(opened_at.unwrap_or(at));
                        degenerate &= a == from && b == from && at == from;
                        end = Some(at);
                    }
                    PathCommand::Close => {
                        // A `h` with no subpath open closes nothing: Table 58 says "if the
                        // current subpath is already closed, h shall do nothing".
                        if start.is_some() {
                            segments = segments.saturating_add(1);
                        }
                        index = index.saturating_add(1);
                        break;
                    }
                }
                index = index.saturating_add(1);
            }
            if let Some(start) = start {
                return Some(Subpath {
                    first,
                    last: index,
                    start,
                    end: end.unwrap_or(start),
                    segments,
                    degenerate,
                });
            }
            // The commands just consumed opened no subpath — a stray `h`. Keep walking
            // rather than ending the iteration, which would hide everything after it.
        }
    })
}

#[cfg(test)]
mod tests {
    use super::{DegenerateStroke, dash_mark, split_degenerate};
    use crate::geom::{Path, PathCommand, Point};
    use crate::paint::LineCap;

    fn path(commands: &[PathCommand]) -> Path {
        let mut p = Path::new();
        for c in commands {
            p.push(*c);
        }
        p
    }

    /// The clause's own example: a single-point closed path under each of the three caps.
    #[test]
    fn a_single_point_closed_path_is_a_circle_only_under_round_caps() {
        let dot = path(&[
            PathCommand::MoveTo(Point::new(5.0, 7.0)),
            PathCommand::Close,
        ]);

        let round = split_degenerate(&dot, LineCap::Round, 4.0).expect("degenerate");
        assert!(round.stroked.is_empty(), "nothing is left to stroke");
        assert!(!round.dots.is_empty(), "§8.5.3.2 asks for a filled circle");

        for cap in [LineCap::Butt, LineCap::Square] {
            let other = split_degenerate(&dot, cap, 4.0).expect("degenerate");
            assert!(other.stroked.is_empty());
            assert!(
                other.dots.is_empty(),
                "{cap:?}: the clause says S produces no output"
            );
        }
    }

    /// "or of two or more points at the same coordinates" — the open form of the same shape.
    #[test]
    fn two_points_at_the_same_coordinates_are_degenerate() {
        let dot = path(&[
            PathCommand::MoveTo(Point::new(5.0, 7.0)),
            PathCommand::LineTo(Point::new(5.0, 7.0)),
            PathCommand::LineTo(Point::new(5.0, 7.0)),
        ]);
        let split = split_degenerate(&dot, LineCap::Round, 4.0).expect("degenerate");
        assert!(split.stroked.is_empty());
        assert_eq!(
            split.dots.commands().len(),
            6,
            "one circle: a move, four curves and a close"
        );
    }

    /// A curve whose control points and endpoint all sit on the current point spans no
    /// distance either, and reaches the same rule.
    #[test]
    fn a_curve_that_never_leaves_its_start_is_degenerate() {
        let at = Point::new(1.0, 2.0);
        let dot = path(&[PathCommand::MoveTo(at), PathCommand::CurveTo(at, at, at)]);
        assert!(
            !split_degenerate(&dot, LineCap::Round, 4.0)
                .expect("degenerate")
                .dots
                .is_empty()
        );
    }

    /// A lone `m` paints nothing under *every* cap, which is a different sentence from the
    /// one about closed single-point subpaths and gives the opposite answer for round caps.
    #[test]
    fn a_single_point_open_subpath_paints_nothing_even_under_round_caps() {
        let stray = path(&[PathCommand::MoveTo(Point::new(5.0, 7.0))]);
        let split = split_degenerate(&stray, LineCap::Round, 4.0).expect("degenerate");
        assert!(split.stroked.is_empty());
        assert!(
            split.dots.is_empty(),
            "a trailing m is not a dot: §8.5.3.2's last sentence"
        );
    }

    /// The common case must cost nothing: an ordinary path is left alone.
    #[test]
    fn a_path_that_spans_a_distance_is_not_split_at_all() {
        let line = path(&[
            PathCommand::MoveTo(Point::new(0.0, 0.0)),
            PathCommand::LineTo(Point::new(10.0, 0.0)),
        ]);
        assert_eq!(split_degenerate(&line, LineCap::Round, 4.0), None);
    }

    /// A dot beside a real subpath keeps the real one, and `h` ends a subpath so that what
    /// follows it is judged on its own.
    #[test]
    fn only_the_degenerate_subpaths_are_removed() {
        let mixed = path(&[
            PathCommand::MoveTo(Point::new(0.0, 0.0)),
            PathCommand::LineTo(Point::new(10.0, 0.0)),
            PathCommand::Close,
            PathCommand::MoveTo(Point::new(3.0, 3.0)),
            PathCommand::Close,
            PathCommand::MoveTo(Point::new(0.0, 5.0)),
            PathCommand::LineTo(Point::new(10.0, 5.0)),
        ]);
        let DegenerateStroke { stroked, dots } =
            split_degenerate(&mixed, LineCap::Round, 4.0).expect("degenerate");
        assert_eq!(
            stroked.commands(),
            [
                PathCommand::MoveTo(Point::new(0.0, 0.0)),
                PathCommand::LineTo(Point::new(10.0, 0.0)),
                PathCommand::Close,
                PathCommand::MoveTo(Point::new(0.0, 5.0)),
                PathCommand::LineTo(Point::new(10.0, 5.0)),
            ]
        );
        assert_eq!(dots.commands().len(), 6, "one circle");
    }

    /// The dash rule inverts the subpath rule for a projecting square cap, and agrees with
    /// it for the other two.
    #[test]
    fn a_zero_length_dash_paints_its_cap_where_a_degenerate_subpath_does_not() {
        let along = Point::new(1.0, 0.0);
        let mut butt = Path::new();
        dash_mark(&mut butt, Point::new(0.0, 0.0), 4.0, LineCap::Butt, along);
        assert!(butt.is_empty(), "a butt cap has no extent to paint");

        let mut round = Path::new();
        dash_mark(&mut round, Point::new(0.0, 0.0), 4.0, LineCap::Round, along);
        assert_eq!(round.commands().len(), 6, "a circle");

        let mut square = Path::new();
        dash_mark(
            &mut square,
            Point::new(0.0, 0.0),
            4.0,
            LineCap::Square,
            along,
        );
        assert_eq!(
            square.commands(),
            [
                PathCommand::MoveTo(Point::new(2.0, 2.0)),
                PathCommand::LineTo(Point::new(-2.0, 2.0)),
                PathCommand::LineTo(Point::new(-2.0, -2.0)),
                PathCommand::LineTo(Point::new(2.0, -2.0)),
                PathCommand::Close,
            ],
            "a square of the line's width, centred on the dash"
        );
    }

    /// A square cap turns with the path. Along the y axis the same square is the same four
    /// corners in a different order, which is what distinguishes a rotation from a constant.
    #[test]
    fn a_square_dash_cap_faces_along_the_path() {
        let mut square = Path::new();
        dash_mark(
            &mut square,
            Point::new(0.0, 0.0),
            4.0,
            LineCap::Square,
            Point::new(0.0, 3.0),
        );
        assert_eq!(
            square.commands(),
            [
                PathCommand::MoveTo(Point::new(-2.0, 2.0)),
                PathCommand::LineTo(Point::new(-2.0, -2.0)),
                PathCommand::LineTo(Point::new(2.0, -2.0)),
                PathCommand::LineTo(Point::new(2.0, 2.0)),
                PathCommand::Close,
            ]
        );
    }
}

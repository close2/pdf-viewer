//! How far a stroke's outline reaches from its path.
//!
//! # Why there are two answers to one question
//!
//! [`Command::device_bounds`] already bounds a stroke, and does it in one line: the path's
//! memoised hull, expanded in every direction by `width × miter_limit`. That is the right
//! shape for the question it is asked — *may this command mark this strip?* — which the
//! rasteriser puts to every command once per strip, and which session 163 measured at 17.6%
//! of a dense page's render before the hull was memoised. A bound that walked the path again
//! would put that cost back.
//!
//! It is the wrong shape for a different question: *does this command mark outside this
//! rectangle?* An isotropic reach of `width × miter_limit` is up to twenty times the true
//! one, and it cannot show containment for the shape that most needs it — a butt-capped line
//! ending exactly on the boundary, which reaches half a width sideways and **nothing at all**
//! lengthwise. That shape is not a curiosity: it is how a tiling pattern rules a grid, and it
//! cost `issue16038.pdf` 15% of the ink its own geometry states (`AMBIGUOUS_TILING_CELL_CLIP`).
//!
//! So this module answers the second question, tightly, at the cost of walking the path — and
//! its callers ask it once per pattern rather than once per command per strip. Both answers
//! are bounds and neither may underestimate; `the_loose_bound_contains_the_tight_one` is what
//! keeps them from contradicting each other.
//!
//! # What "tight" means here, term by term
//!
//! A stroke covers "all points whose perpendicular distance from the path … is less than or
//! equal to half the line width" (ISO 32000-2 §8.4.3.2), plus what §8.4.3.5's caps and
//! §8.4.3.4's joins add. Taken one element at a time:
//!
//! - **A straight segment** from `a` to `b` covers the rectangle of width `w` centred on it,
//!   whose axis-aligned box is `box{a, b}` grown by `(w/2)·(|dy|, |dx|)/|b−a|`. For a
//!   horizontal segment that is `(0, w/2)`: no growth along the line at all, which is the
//!   whole point.
//! - **A curve** is bounded by its control points' hull grown by `w/2` in every direction. A
//!   cubic lies inside that hull and the stroke reaches `w/2` perpendicular to it, so the
//!   isotropic grow is a bound; a per-segment normal would need the flattened curve.
//! - **A join** reaches `w/2` from the vertex for round and bevel — a bevel's two corners are
//!   the adjoining rectangles' own corners — and `miter_limit × w/2` for a mitre, which is
//!   what the limit *means* (§8.4.3.5: the ratio of mitre length to line width).
//! - **A cap** adds nothing for butt, `w/2` around the endpoint for round, and `w/2·√2` for a
//!   projecting square, whose far corners sit at that distance when the segment runs at 45°.
//! - **A dash pattern** puts caps wherever a dash ends, so a non-butt cap on a dashed stroke
//!   grows the whole hull rather than its ends.

use crate::display_list::Command;
use crate::geom::{Path, PathCommand, Point, Rect, Transform};
use crate::paint::{LineCap, LineJoin, Stroke};

/// The region a command marks, ignoring its clip, or `None` where this cannot say.
///
/// [`Command::device_bounds`] with a tight answer for a stroke, and the same answer for
/// everything else. A group is its elements' union, walked here rather than refused, because
/// a caller asking about containment cannot treat "anywhere" as an answer and give a useful
/// one.
#[must_use]
pub fn marked_bounds(command: &Command, to_device: Transform) -> Option<Rect> {
    match command {
        Command::Stroke {
            path,
            transform,
            stroke,
            ..
        } => stroked_bounds(path, stroke, transform.then(to_device)),
        Command::Group { commands, .. } => {
            let mut union: Option<Rect> = None;
            for element in commands {
                let bounds = marked_bounds(element, to_device)?;
                union = Some(union.map_or(bounds, |u| u.union(bounds)));
            }
            union
        }
        other => other.device_bounds(to_device),
    }
}

/// The extent of the outline `stroke` deposits around `path`, mapped by `to_device`.
///
/// `None` for a path that names no point. The hull is computed in the path's own space —
/// where the width is stated — and its four corners are then mapped, exactly as
/// [`Path::bounds`] does, because a sheared transform takes a perpendicular offset to one
/// that is not perpendicular and the box of the image contains the image of the box.
#[must_use]
pub fn stroked_bounds(path: &Path, stroke: &Stroke, to_device: Transform) -> Option<Rect> {
    let half = stroke.device_width(to_device) / 2.0;
    if !half.is_finite() || half < 0.0 {
        return None;
    }
    let hull = hull(path, stroke, half)?;
    // A dash pattern ends a dash anywhere along the path, and each end wears a cap, so a
    // non-butt cap on a dashed stroke reaches from every point rather than from two.
    let everywhere = if stroke.dash_array.is_empty() {
        0.0
    } else {
        cap_reach(stroke, half)
    };
    let hull = Rect {
        min: Point::new(hull.min.x - everywhere, hull.min.y - everywhere),
        max: Point::new(hull.max.x + everywhere, hull.max.y + everywhere),
    };
    Some(hull.mapped(to_device))
}

/// Where a join can put ink, given the two directions meeting at its vertex.
///
/// ISO 32000-2 §8.4.3.4's miter join is where "[t]he outer edges of the strokes for the two
/// segments shall be extended until they meet at an angle, as in a picture frame", and §8.4.3.5
/// bounds how far that can be:
///
/// > The miter limit shall impose a maximum on the ratio of the miter length to the line
/// > width (see "Figure 15 -Miter length"). When the limit is exceeded, the join is converted
/// > from a miter to a bevel.
///
/// A *maximum*, not a length. **The old bound was the maximum alone** — a square of side
/// `2 × half × miter_limit` around every vertex, which for the default limit of 10 puts a
/// rectangle's corner five widths outside the rectangle. Sound as a bound, and useless as a
/// containment test: `pdf_model`'s rule for taking a redundant `/BBox` clip back off (ADR 0155)
/// could never fire for a stroked border, and `bug1863910.pdf` lost **22% of its ink** to a clip
/// that cut nothing.
///
/// The tip of a miter sits where the two outer offset lines cross, and *which* side is outer
/// depends on the turn — so both candidates are returned rather than the turn being worked out.
/// Their union is still far tighter than the square: at a right angle it is the stroke's own
/// outer corner rather than ten widths in every direction. Where the two directions nearly
/// reverse, the crossing runs away and the limit is the whole answer. ADR 0165.
fn join_extent(
    stroke: &Stroke,
    half: f32,
    incoming: Option<Point>,
    vertex: Point,
    outgoing: Option<Point>,
) -> Rect {
    let limit = half * stroke.miter_limit.max(1.0);
    let square = |reach: f32| {
        Rect::from_corners(
            Point::new(vertex.x - reach, vertex.y - reach),
            Point::new(vertex.x + reach, vertex.y + reach),
        )
    };
    if !matches!(stroke.join, LineJoin::Miter) {
        return square(half);
    }
    let Some((from, to)) = incoming.zip(outgoing) else {
        return square(limit);
    };
    let unit = |a: Point, b: Point| {
        let (dx, dy) = (b.x - a.x, b.y - a.y);
        let length = crate::geom::length(dx, dy);
        (length > 0.0).then(|| (dx / length, dy / length))
    };
    let (Some(into), Some(out)) = (unit(from, vertex), unit(vertex, to)) else {
        return square(limit);
    };
    // The outward normals of the two segments, on one arbitrary side; their sum bisects the
    // angle, and `|n1 + n2|² = 2(1 + cos φ)` falls to zero as the join doubles back.
    let (n1, n2) = ((-into.1, into.0), (-out.1, out.0));
    let (sx, sy) = (n1.0 + n2.0, n1.1 + n2.1);
    let square_length = sx.mul_add(sx, sy * sy);
    // **A miter over the limit is not a long miter; it is a bevel**, and a bevel reaches half a
    // width. §8.4.3.5: "When the limit is exceeded, the join is converted from a miter to a
    // bevel." Bounding it by the limit instead is the same mistake this function was written to
    // correct, one case in: `issue21068.pdf` draws each comb separator as a two-point subpath
    // closed on itself, so both of its joins double back, and bounding those by the limit put
    // every separator 4.5 units outside the `/BBox` that contains it.
    if square_length <= f32::EPSILON {
        return square(half);
    }
    let scale = 2.0 * half / square_length;
    let (dx, dy) = (sx * scale, sy * scale);
    if crate::geom::length(dx, dy) > limit {
        return square(half);
    }
    Rect::from_corners(
        Point::new(vertex.x - dx, vertex.y - dy),
        Point::new(vertex.x + dx, vertex.y + dy),
    )
}

/// How far past an endpoint §8.4.3.3's line cap reaches.
fn cap_reach(stroke: &Stroke, half: f32) -> f32 {
    /// A projecting square cap's far corner sits `√2` half-widths from the endpoint.
    const SQUARE_CAP_CORNER: f32 = std::f32::consts::SQRT_2;

    match stroke.cap {
        LineCap::Butt => 0.0,
        LineCap::Round => half,
        LineCap::Square => half * SQUARE_CAP_CORNER,
    }
}

/// The outline's extent in the path's own space, `half` being half the line width there.
fn hull(path: &Path, stroke: &Stroke, half: f32) -> Option<Rect> {
    let join_box = |incoming: Option<Point>, vertex: Point, outgoing: Option<Point>| {
        join_extent(stroke, half, incoming, vertex, outgoing)
    };
    let cap_reach = cap_reach(stroke, half);

    let mut hull: Option<Rect> = None;
    let add = |rect: Rect, hull: &mut Option<Rect>| {
        *hull = Some(hull.map_or(rect, |h: Rect| h.union(rect)));
    };
    let around = |at: Point, reach: f32| {
        Rect::from_corners(
            Point::new(at.x - reach, at.y - reach),
            Point::new(at.x + reach, at.y + reach),
        )
    };

    let mut cursor: Option<Point> = None;
    let mut start: Option<Point> = None;
    // The point a join would sit at, set once a subpath has a segment behind it, and the point
    // the incoming segment's tangent comes *from* — its own start for a line, its second control
    // point for a curve, which is where a Bézier's end tangent points.
    let mut joins_at: Option<Point> = None;
    let mut joins_from: Option<Point> = None;
    // Where the subpath's *first* segment goes, which is the outgoing direction of the join a
    // `Close` creates at the start point. Without it that one join falls back to the miter
    // limit, and one join at the limit is enough to put a rectangle's bound five widths out.
    let mut leaves_start: Option<Point> = None;
    for command in path.commands() {
        match *command {
            PathCommand::MoveTo(p) => {
                // The previous subpath ended here, open: its last point wears a cap.
                if let (Some(end), Some(_)) = (cursor, joins_at)
                    && cap_reach > 0.0
                {
                    add(around(end, cap_reach), &mut hull);
                }
                if joins_at.is_none()
                    && let Some(lone) = cursor
                {
                    // A subpath of one `m` marks nothing (§8.5.3.2), but it is still a point
                    // of the path and a bound may not exclude it.
                    add(around(lone, 0.0), &mut hull);
                }
                cursor = Some(p);
                start = Some(p);
                joins_at = None;
                joins_from = None;
                leaves_start = None;
                add(around(p, cap_reach), &mut hull);
            }
            PathCommand::LineTo(p) => {
                let from = cursor.or(start)?;
                add(segment(from, p, half), &mut hull);
                if let Some(vertex) = joins_at {
                    add(join_box(joins_from, vertex, Some(p)), &mut hull);
                }
                joins_at = Some(p);
                joins_from = Some(from);
                if leaves_start.is_none() {
                    leaves_start = Some(p);
                }
                cursor = Some(p);
                start.get_or_insert(from);
            }
            PathCommand::CurveTo(c1, c2, p) => {
                let from = cursor.or(start)?;
                for point in [from, c1, c2, p] {
                    add(around(point, half), &mut hull);
                }
                if let Some(vertex) = joins_at {
                    // A cubic's tangent at its start points at the first control point that is
                    // not the start itself.
                    let leaves_to = [c1, c2, p].into_iter().find(|point| *point != vertex);
                    add(join_box(joins_from, vertex, leaves_to), &mut hull);
                }
                joins_at = Some(p);
                joins_from = [c2, c1, from].into_iter().find(|point| *point != p);
                if leaves_start.is_none() {
                    leaves_start = [c1, c2, p].into_iter().find(|point| *point != from);
                }
                cursor = Some(p);
                start.get_or_insert(from);
            }
            PathCommand::Close => {
                if let (Some(from), Some(to)) = (cursor, start) {
                    add(segment(from, to, half), &mut hull);
                    // Closing makes both ends joins rather than caps. The join *at* the start
                    // point has the closing segment coming in and the subpath's first segment
                    // going out, which is what `leaves_start` was kept for.
                    add(join_box(Some(from), to, leaves_start), &mut hull);
                    if let Some(vertex) = joins_at {
                        add(join_box(joins_from, vertex, Some(to)), &mut hull);
                    }
                    cursor = Some(to);
                }
                joins_at = None;
                joins_from = None;
                leaves_start = None;
                start = None;
            }
        }
    }
    // The last subpath, if it was left open.
    if let (Some(end), Some(_)) = (cursor, joins_at)
        && cap_reach > 0.0
    {
        add(around(end, cap_reach), &mut hull);
    }

    hull
}

/// The axis-aligned box of the rectangle a straight segment's stroke covers.
///
/// `box{a, b}` grown by `(w/2)·(|dy|, |dx|)/|b−a|`, which is the half-width projected onto
/// each axis: a horizontal segment grows in y alone. A segment of no length has no direction
/// and falls back to the isotropic half-width, which is what §8.5.3.2's degenerate rules then
/// paint at most.
fn segment(a: Point, b: Point, half: f32) -> Rect {
    let (dx, dy) = (b.x - a.x, b.y - a.y);
    let length = crate::geom::length(dx, dy);
    let (gx, gy) = if length > 0.0 {
        (half * dy.abs() / length, half * dx.abs() / length)
    } else {
        (half, half)
    };
    let box_ = Rect::from_corners(a, b);
    Rect {
        min: Point::new(box_.min.x - gx, box_.min.y - gy),
        max: Point::new(box_.max.x + gx, box_.max.y + gy),
    }
}

#[cfg(test)]
#[expect(
    clippy::float_cmp,
    reason = "test code: these bounds are exact sums of exactly-representable values, and \
              a tolerance would hide the arithmetic being wrong"
)]
mod tests {
    use super::{marked_bounds, stroked_bounds};
    use crate::display_list::Command;
    use crate::geom::{Path, PathCommand, Point, Transform};
    use crate::paint::{BlendMode, Color, LineCap, LineJoin, Paint, Stroke};
    use std::sync::Arc;

    fn path(commands: &[PathCommand]) -> Path {
        let mut p = Path::new();
        for c in commands {
            p.push(*c);
        }
        p
    }

    fn line() -> Path {
        path(&[
            PathCommand::MoveTo(Point::new(10.0, 50.0)),
            PathCommand::LineTo(Point::new(90.0, 50.0)),
        ])
    }

    /// The case the whole module exists for: a butt-capped horizontal line reaches half a
    /// width in y and **nothing** in x.
    #[test]
    fn a_butt_capped_line_reaches_across_its_width_and_not_along_it() {
        let stroke = Stroke {
            width: 4.0,
            ..Stroke::default()
        };
        let bounds = stroked_bounds(&line(), &stroke, Transform::IDENTITY).expect("bounded");
        assert_eq!(bounds.min, Point::new(10.0, 48.0));
        assert_eq!(bounds.max, Point::new(90.0, 52.0));
    }

    /// A projecting square cap reaches `√2` half-widths, because its far corners do when the
    /// segment runs diagonally — and this bound is stated once for every direction.
    #[test]
    fn a_square_cap_reaches_further_than_a_butt_one() {
        let stroke = Stroke {
            width: 4.0,
            cap: LineCap::Square,
            ..Stroke::default()
        };
        let bounds = stroked_bounds(&line(), &stroke, Transform::IDENTITY).expect("bounded");
        let reach = 2.0 * std::f32::consts::SQRT_2;
        assert!((bounds.min.x - (10.0 - reach)).abs() < 1e-4);
        assert!((bounds.max.x - (90.0 + reach)).abs() < 1e-4);
    }

    /// A mitre reaches as far as its own angle says, capped by §8.4.3.5's limit.
    ///
    /// ISO 32000-2 §8.4.3.5:
    ///
    /// > The miter limit shall impose a maximum on the ratio of the miter length to the line
    /// > width (see "Figure 15 -Miter length"). When the limit is exceeded, the join is converted
    /// > from a miter to a bevel.
    ///
    /// A *maximum*, not a length. A miter's tip sits where the two outer offset lines cross, which
    /// for a right angle is the stroke's own outer corner; only a join that nearly doubles back
    /// reaches the cap. This bound was the cap alone until the two-hundred-and-fifth session,
    /// which made it useless as a containment test and cost `bug1863910.pdf` 22% of its ink
    /// (ADR 0165).
    #[test]
    fn a_mitre_reaches_as_far_as_its_angle_and_no_further_than_the_limit() {
        let corner = path(&[
            PathCommand::MoveTo(Point::new(10.0, 10.0)),
            PathCommand::LineTo(Point::new(50.0, 10.0)),
            PathCommand::LineTo(Point::new(50.0, 50.0)),
        ]);
        let stroke = Stroke {
            width: 4.0,
            miter_limit: 10.0,
            ..Stroke::default()
        };
        let bounds = stroked_bounds(&corner, &stroke, Transform::IDENTITY).expect("bounded");
        // The join at (50, 10) turns by a right angle, so its miter tip is the stroke's own
        // outer corner at (52, 8) — half a width out along each axis — rather than the limit's
        // twenty in every direction. The two open ends reach nothing.
        assert_eq!(bounds.max.x, 52.0, "{bounds:?}");
        assert_eq!(bounds.min.y, 8.0, "{bounds:?}");
        assert_eq!(bounds.min.x, 10.0, "the butt end reaches nothing along x");

        // A join sharp enough to exceed the limit is not a long miter — §8.4.3.5 converts it to
        // a bevel, which reaches half a width. Bounding it by the limit would put this spike
        // twenty units past its own end.
        let spike = path(&[
            PathCommand::MoveTo(Point::new(10.0, 10.0)),
            PathCommand::LineTo(Point::new(50.0, 10.0)),
            PathCommand::LineTo(Point::new(11.0, 11.0)),
        ]);
        let bevelled_by_the_limit =
            stroked_bounds(&spike, &stroke, Transform::IDENTITY).expect("bounded");
        assert!(
            (bevelled_by_the_limit.max.x - 52.0).abs() < 1e-3,
            "{bevelled_by_the_limit:?}"
        );

        let bevelled = stroked_bounds(
            &corner,
            &Stroke {
                join: LineJoin::Bevel,
                ..stroke
            },
            Transform::IDENTITY,
        )
        .expect("bounded");
        assert_eq!(bevelled.max.x, 52.0, "a bevel reaches half a width");
    }

    /// The bound this replaces must never be *inside* it, or one of the two is wrong.
    ///
    /// `Command::device_bounds` is the fast answer the strip planner asks per command per
    /// strip; this is the slow tight one. They answer the same question at different prices,
    /// so the loose one has to contain the tight one on every shape — checked over the shapes
    /// that differ most, including the ones with joins and caps, at three transforms.
    #[test]
    fn the_loose_bound_contains_the_tight_one() {
        let shapes = [
            line(),
            path(&[
                PathCommand::MoveTo(Point::new(10.0, 10.0)),
                PathCommand::LineTo(Point::new(50.0, 10.0)),
                PathCommand::LineTo(Point::new(50.0, 50.0)),
                PathCommand::Close,
            ]),
            path(&[
                PathCommand::MoveTo(Point::new(0.0, 0.0)),
                PathCommand::CurveTo(
                    Point::new(10.0, 40.0),
                    Point::new(60.0, -20.0),
                    Point::new(70.0, 10.0),
                ),
            ]),
        ];
        let strokes = [
            Stroke::default(),
            Stroke {
                width: 6.0,
                cap: LineCap::Square,
                join: LineJoin::Round,
                ..Stroke::default()
            },
            Stroke {
                width: 0.0,
                dash_array: vec![3.0, 2.0],
                cap: LineCap::Round,
                ..Stroke::default()
            },
        ];
        for shape in &shapes {
            for stroke in &strokes {
                for at in [
                    Transform::IDENTITY,
                    Transform::scale(3.0, 3.0),
                    Transform::new(1.0, 0.0, 2.0, 1.0, 5.0, -5.0),
                ] {
                    let command = Command::Stroke {
                        path: Arc::new(shape.clone()),
                        transform: Transform::IDENTITY,
                        stroke: stroke.clone(),
                        paint: Paint::Solid(Color::BLACK),
                        clip: None,
                        mask: None,
                        blend: BlendMode::Normal,
                    };
                    let loose = command.device_bounds(at).expect("bounded");
                    let tight = marked_bounds(&command, at).expect("bounded");
                    assert!(
                        loose.min.x <= tight.min.x + 1e-3
                            && loose.min.y <= tight.min.y + 1e-3
                            && loose.max.x + 1e-3 >= tight.max.x
                            && loose.max.y + 1e-3 >= tight.max.y,
                        "{loose:?} does not contain {tight:?}"
                    );
                }
            }
        }
    }
}

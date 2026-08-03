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
    Some(hull(path, stroke, half)?.mapped(to_device))
}

/// The outline's extent in the path's own space, `half` being half the line width there.
fn hull(path: &Path, stroke: &Stroke, half: f32) -> Option<Rect> {
    /// A projecting square cap's far corner sits `√2` half-widths from the endpoint.
    const SQUARE_CAP_CORNER: f32 = std::f32::consts::SQRT_2;

    let join_reach = match stroke.join {
        LineJoin::Miter => half * stroke.miter_limit.max(1.0),
        LineJoin::Round | LineJoin::Bevel => half,
    };
    let cap_reach = match stroke.cap {
        LineCap::Butt => 0.0,
        LineCap::Round => half,
        LineCap::Square => half * SQUARE_CAP_CORNER,
    };
    // A dash pattern ends a dash anywhere along the path, and each end wears a cap, so a
    // non-butt cap on a dashed stroke reaches from every point rather than from two.
    let everywhere = if stroke.dash_array.is_empty() {
        0.0
    } else {
        cap_reach
    };

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
    // The point a join would sit at, set once a subpath has a segment behind it.
    let mut joins_at: Option<Point> = None;
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
                add(around(p, cap_reach), &mut hull);
            }
            PathCommand::LineTo(p) => {
                let from = cursor.or(start)?;
                add(segment(from, p, half), &mut hull);
                if let Some(vertex) = joins_at {
                    add(around(vertex, join_reach), &mut hull);
                }
                joins_at = Some(p);
                cursor = Some(p);
                start.get_or_insert(from);
            }
            PathCommand::CurveTo(c1, c2, p) => {
                let from = cursor.or(start)?;
                for point in [from, c1, c2, p] {
                    add(around(point, half), &mut hull);
                }
                if let Some(vertex) = joins_at {
                    add(around(vertex, join_reach), &mut hull);
                }
                joins_at = Some(p);
                cursor = Some(p);
                start.get_or_insert(from);
            }
            PathCommand::Close => {
                if let (Some(from), Some(to)) = (cursor, start) {
                    add(segment(from, to, half), &mut hull);
                    // Closing makes both ends joins rather than caps.
                    add(around(to, join_reach), &mut hull);
                    if let Some(vertex) = joins_at {
                        add(around(vertex, join_reach), &mut hull);
                    }
                    cursor = Some(to);
                }
                joins_at = None;
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

    let hull = hull?;
    if everywhere > 0.0 {
        return Some(Rect {
            min: Point::new(hull.min.x - everywhere, hull.min.y - everywhere),
            max: Point::new(hull.max.x + everywhere, hull.max.y + everywhere),
        });
    }
    Some(hull)
}

/// The axis-aligned box of the rectangle a straight segment's stroke covers.
///
/// `box{a, b}` grown by `(w/2)·(|dy|, |dx|)/|b−a|`, which is the half-width projected onto
/// each axis: a horizontal segment grows in y alone. A segment of no length has no direction
/// and falls back to the isotropic half-width, which is what §8.5.3.2's degenerate rules then
/// paint at most.
fn segment(a: Point, b: Point, half: f32) -> Rect {
    let (dx, dy) = (b.x - a.x, b.y - a.y);
    let length = dx.hypot(dy);
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

    /// A mitre reaches the limit times half the width, and only where there is a join.
    #[test]
    fn a_mitre_reaches_from_the_join_and_a_line_has_none() {
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
        // The join at (50, 10) reaches 20 in every direction; the two open ends do not.
        assert_eq!(bounds.max.x, 70.0);
        assert_eq!(bounds.min.y, -10.0);
        assert_eq!(bounds.min.x, 10.0, "the butt end reaches nothing along x");

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

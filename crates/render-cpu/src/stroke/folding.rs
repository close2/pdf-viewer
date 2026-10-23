//! The outline of a stroke whose curve bends more tightly than its half-width — ISO 32000-2
//! §8.4.3.2's set of points, built as a union of simple pieces (ADRs 1348, 1374).

/// The outline of `path` stroked under `style`, built as a union of simple pieces where one of its
/// curves may bend more tightly than the stroke's half-width — ISO 32000-2 §8.4.3.2; `None` where
/// none can, which is nearly every stroke, and the stroker's own outline is then the stroke.
///
/// §8.4.3.2 defines a stroke as a set of points, "all points whose perpendicular distance from the
/// path in user space is less than or equal to half the line width", so a radius-5 circle at
/// `30 w` is the disk of radius 20. `tiny-skia`'s stroker offsets a closed curve's inside as a
/// second contour, reversed, and where the half-width passes the curve's radius of curvature that
/// offset turns inside out and its winding cancels the outer contour's: the disk came out as a ring
/// with a hole of radius 10. Handing the stroker the curve's chords instead does not cure it: two
/// chords meeting nearly straight are joined as one line, with no pivot on the inner side, and the
/// hole comes back once the chords are fine enough to be faithful to the curve.
///
/// So a curve that may fold is not offset. It is flattened — which is what §10.7.4 says a curve is by
/// the time a pixel is decided, "curves have been flattened to sequences of straight lines" — and
/// stated as the union of pieces each of which is convex and wound one way: every chord's own band,
/// `h` either side of it; at every vertex the join on its outer side. A point within `h` of a
/// chord's interior is in that chord's band, and a point within `h` of a vertex but of no band lies
/// in the angle between the two chords' normals on the outer side, which is the join's.
///
/// The stretches between such curves are the stroker's (ADR 1359): a curve bending less tightly
/// than the half-width is offset faithfully, in far fewer edges than its chords' bands, and
/// `issue14415.pdf`'s strokes fold at 348 of their 1827 curves. Each stretch is stroked with butt
/// ends, at sixteen times the device's resolution so that its offset curves lie as close to the set
/// as the chords do, flattened to the chords' tolerance and wound the way the pieces are; the caps
/// at every open end, and the join wherever a stretch meets a folding curve, are pieces, turned from
/// the stretch's exact tangent and reaching a little into it so that nothing abuts (trap 56). The
/// union, filled under the non-zero rule, is the set.
///
/// The joins are §8.4.3.4's at the path's own vertices — a mitre while §8.4.3.5's ratio
/// `1 / sin(φ/2)` is within the limit and a bevel past it, the disk's sector for a round join —
/// and the round join a smooth curve's stroke is at a vertex the flattening made.
///
/// # When
///
/// A curve can fold only where its radius of curvature `|B′|³ / |B′ × B″|` is under the
/// half-width, and that radius is at least `|B′|² / |B″|`. So a curve is left alone when the
/// smallest `|B′|` its hodograph's control polygon allows, squared, is at least the half-width
/// times the largest `|B″|` — a sufficient test that every stroke whose width is small beside its
/// curves passes. The chords lie within [`FOLD_FLATNESS`] of a device pixel of their curve, and so
/// do the arcs of every round join and cap. ADR 1348.
pub(crate) fn folding_stroke_outline(
    path: &tiny_skia::Path,
    style: &tiny_skia::Stroke,
    scale: f32,
) -> Option<tiny_skia::Path> {
    let half = style.width / 2.0;
    if !(half.is_finite() && half > 0.0 && scale.is_finite() && scale > 0.0) {
        return None;
    }
    if !curves_may_fold(path, half) {
        return None;
    }
    let tolerance = FOLD_FLATNESS / scale;
    // The turn that holds `hθ²/8` to the flatness, and finer by the half-width in device pixels
    // once that passes one: a wide stroke's rim lies `h` out from a chord that turns `θ`, so its
    // shortfall grows with `h` where a thin one's cannot show.
    let turn = (8.0 * tolerance / half / (half * scale).max(1.0)).sqrt();
    let pieces = Pieces {
        half,
        // A sixteenth of the flatness: far below anything the raster shows, and far above the
        // arithmetic's own residue at any coordinate a stroke is drawn at.
        overlap: tolerance / 16.0,
        // The joins' arcs to the finer flatness `crate::area` holds every fill to: a disk's rim is
        // nothing but arcs, so their shortfall is what a wide stroke's ink is short by, and a
        // thin stroke's arcs are a single side at either figure.
        step: arc_step(half, ARC_FLATNESS / scale),
        join: style.line_join,
        limit: style.miter_limit,
        cap: style.line_cap,
    };
    let folds = folding_segments(path, half);
    let mut out = tiny_skia::PathBuilder::new();
    for subpath in flattened_subpaths(path, (tolerance, turn), &folds) {
        pieces.push_subpath(&mut out, &subpath);
    }
    // What does not fold is the stroker's, which offsets a curve bending less tightly than the
    // half-width faithfully and states it in far fewer edges than its chords' bands (ADR 1359).
    // Each stretch is stroked with butt ends — its caps and the joins where it meets a folding
    // curve are pieces above — and wound the way every piece is, so that the union adds.
    let mut butt = style.clone();
    butt.line_cap = tiny_skia::LineCap::Butt;
    for (run, closed) in non_folding_runs(path, &folds) {
        // Sixteen times the device's resolution: at its own, the stroker's offset curves overshot a
        // ring of radius 23 by four units of ink, where the chords are held to [`FOLD_FLATNESS`].
        let Some(outline) = run.stroke(if closed { style } else { &butt }, scale * 16.0) else {
            continue;
        };
        push_wound_positively(&mut out, &outline, tolerance);
    }
    out.finish()
}

/// One segment of a subpath, as [`non_folding_runs`] reads it.
#[derive(Clone, Copy)]
struct Segment {
    /// Where it begins.
    from: tiny_skia::Point,
    /// What it is, with where it ends.
    kind: tiny_skia::PathSegment,
    /// Whether it is a curve that may bend more tightly than the half-width.
    folds: bool,
}

/// The stretches of `path`'s subpaths that no curve which may fold interrupts, each as its own
/// path and whether it is a whole closed subpath — see [`folding_stroke_outline`].
///
/// A closed subpath with a folding curve is walked from just after its first such curve, so that
/// no stretch runs across the point where it was opened; its closing segment is a stretch's like
/// any other.
fn non_folding_runs(path: &tiny_skia::Path, folds: &[bool]) -> Vec<(tiny_skia::Path, bool)> {
    let mut runs = Vec::new();
    let mut segments: Vec<Segment> = Vec::new();
    let mut start = tiny_skia::Point::zero();
    let mut current = tiny_skia::Point::zero();
    let mut finish = |segments: &mut Vec<Segment>, start: tiny_skia::Point, closed: bool| {
        if closed
            && let Some(last) = segments.last().map(|segment| end_of(segment.kind))
            && last != start
        {
            segments.push(Segment {
                from: last,
                kind: tiny_skia::PathSegment::LineTo(start),
                folds: false,
            });
        }
        let first_fold = segments.iter().position(|segment| segment.folds);
        match first_fold {
            None if !segments.is_empty() => {
                if let Some(run) = contour(segments, closed) {
                    runs.push((run, closed));
                }
            }
            None => {}
            Some(fold) => {
                let order: Vec<Segment> = if closed {
                    segments
                        .iter()
                        .cycle()
                        .skip(fold.saturating_add(1))
                        .take(segments.len())
                        .copied()
                        .collect()
                } else {
                    segments.clone()
                };
                let mut stretch: Vec<Segment> = Vec::new();
                for segment in order.into_iter().chain(std::iter::once(Segment {
                    from: start,
                    kind: tiny_skia::PathSegment::Close,
                    folds: true,
                })) {
                    if segment.folds {
                        if let Some(run) = contour(&stretch, false) {
                            runs.push((run, false));
                        }
                        stretch.clear();
                    } else {
                        stretch.push(segment);
                    }
                }
            }
        }
        segments.clear();
    };
    for (index, element) in path.segments().enumerate() {
        let folding = folds.get(index).copied().unwrap_or(false);
        match element {
            tiny_skia::PathSegment::MoveTo(to) => {
                finish(&mut segments, start, false);
                start = to;
                current = to;
            }
            tiny_skia::PathSegment::LineTo(to) => {
                segments.push(Segment {
                    from: current,
                    kind: element,
                    folds: false,
                });
                current = to;
            }
            tiny_skia::PathSegment::QuadTo(_, to) | tiny_skia::PathSegment::CubicTo(_, _, to) => {
                segments.push(Segment {
                    from: current,
                    kind: element,
                    folds: folding,
                });
                current = to;
            }
            tiny_skia::PathSegment::Close => {
                finish(&mut segments, start, true);
                current = start;
            }
        }
    }
    finish(&mut segments, start, false);
    runs
}

/// Where a segment ends.
fn end_of(kind: tiny_skia::PathSegment) -> tiny_skia::Point {
    match kind {
        tiny_skia::PathSegment::MoveTo(to)
        | tiny_skia::PathSegment::LineTo(to)
        | tiny_skia::PathSegment::QuadTo(_, to)
        | tiny_skia::PathSegment::CubicTo(_, _, to) => to,
        tiny_skia::PathSegment::Close => tiny_skia::Point::zero(),
    }
}

/// Consecutive segments as one contour, closed where `closed` says; `None` for none.
fn contour(segments: &[Segment], closed: bool) -> Option<tiny_skia::Path> {
    let first = segments.first()?;
    let mut builder = tiny_skia::PathBuilder::new();
    builder.move_to(first.from.x, first.from.y);
    for segment in segments {
        match segment.kind {
            tiny_skia::PathSegment::LineTo(to) => builder.line_to(to.x, to.y),
            tiny_skia::PathSegment::QuadTo(control, to) => {
                builder.quad_to(control.x, control.y, to.x, to.y);
            }
            tiny_skia::PathSegment::CubicTo(one, two, to) => {
                builder.cubic_to(one.x, one.y, two.x, two.y, to.x, to.y);
            }
            tiny_skia::PathSegment::MoveTo(_) | tiny_skia::PathSegment::Close => {}
        }
    }
    if closed {
        builder.close();
    }
    builder.finish()
}

/// Appends `outline` to `out` flattened to `tolerance` and wound so that its signed area is
/// positive, as [`push_convex`] winds every piece: a stroker's outline of one stretch is wound one
/// way throughout, and which way depends on the stretch's direction where it is closed.
///
/// Flattened here because the library's converter the union goes to flattens a curve far more
/// coarsely than the pieces are cut, and a ring of radius 23 it measured from the stroker's own
/// curves came in five units of ink short of the set; at the pieces' own tolerance it does not.
fn push_wound_positively(
    out: &mut tiny_skia::PathBuilder,
    outline: &tiny_skia::Path,
    tolerance: f32,
) {
    let mut contours: Vec<(Vec<tiny_skia::Point>, bool)> = Vec::new();
    for element in outline.segments() {
        match element {
            tiny_skia::PathSegment::MoveTo(to) => contours.push((vec![to], false)),
            tiny_skia::PathSegment::LineTo(to) => {
                if let Some((points, _)) = contours.last_mut() {
                    points.push(to);
                }
            }
            tiny_skia::PathSegment::QuadTo(control, to) => {
                if let Some((points, _)) = contours.last_mut()
                    && let Some(&from) = points.last()
                {
                    let steps = chords(
                        2.0 * second_difference(from, control, to).length(),
                        tolerance,
                    );
                    points.extend((1..=steps).map(|step| {
                        let t = fraction_of(step, steps);
                        let u = 1.0 - t;
                        tiny_skia::Point::from_xy(
                            u * u * from.x + 2.0 * u * t * control.x + t * t * to.x,
                            u * u * from.y + 2.0 * u * t * control.y + t * t * to.y,
                        )
                    }));
                }
            }
            tiny_skia::PathSegment::CubicTo(first, second, to) => {
                if let Some((points, _)) = contours.last_mut()
                    && let Some(&from) = points.last()
                {
                    let bend = 6.0
                        * second_difference(from, first, second)
                            .length()
                            .max(second_difference(first, second, to).length());
                    let steps = chords(bend, tolerance);
                    points.extend((1..=steps).map(|step| {
                        let t = fraction_of(step, steps);
                        let u = 1.0 - t;
                        let weights = (u * u * u, 3.0 * u * u * t, 3.0 * u * t * t, t * t * t);
                        tiny_skia::Point::from_xy(
                            weights.0 * from.x
                                + weights.1 * first.x
                                + weights.2 * second.x
                                + weights.3 * to.x,
                            weights.0 * from.y
                                + weights.1 * first.y
                                + weights.2 * second.y
                                + weights.3 * to.y,
                        )
                    }));
                }
            }
            tiny_skia::PathSegment::Close => {
                if let Some((_, closed)) = contours.last_mut() {
                    *closed = true;
                }
            }
        }
    }
    // A fill closes every contour, so the shoelace over each closed polygon is its signed area.
    let twice_area: f32 = contours
        .iter()
        .flat_map(|(points, _)| points.iter().zip(points.iter().cycle().skip(1)))
        .map(|(one, next)| one.cross(*next))
        .sum();
    let backwards = twice_area < 0.0;
    for (mut points, closed) in contours {
        if backwards {
            points.reverse();
        }
        let mut points = points.into_iter();
        let Some(first) = points.next() else {
            continue;
        };
        out.move_to(first.x, first.y);
        for point in points {
            out.line_to(point.x, point.y);
        }
        if closed {
            out.close();
        }
    }
}

/// What [`folding_stroke_outline`] builds each piece from.
struct Pieces {
    /// Half the line width, in the path's own space.
    half: f32,
    /// How far each band reaches past its chord's ends, in the path's own space.
    overlap: f32,
    /// The largest angle one side of a round join or cap may span, so that it lies within the
    /// flatness of its circle.
    step: f32,
    /// The file's join, at the vertices the file states.
    join: tiny_skia::LineJoin,
    /// §8.4.3.5's limit on a mitre.
    limit: f32,
    /// The file's cap, at an open subpath's two ends.
    cap: tiny_skia::LineCap,
}

impl Pieces {
    /// Every band, join and cap of one flattened subpath.
    fn push_subpath(&self, out: &mut tiny_skia::PathBuilder, subpath: &Flattened) {
        let points = &subpath.points;
        let Some(&last) = points.last() else {
            return;
        };
        if points.len() < 2 {
            return;
        }
        // The chords of curves that may fold, with the one back to the start where the subpath is
        // closed; every other chord is a stretch [`non_folding_runs`] hands to the stroker.
        let folds = |arriving: usize| subpath.folds.get(arriving).copied().unwrap_or(false);
        let closing = subpath
            .closed
            .then_some((last, points.first().copied(), folds(0)));
        for (start, end) in points
            .windows(2)
            .enumerate()
            .filter_map(|(index, pair)| {
                folds(index.saturating_add(1)).then_some((*pair.first()?, *pair.get(1)?))
            })
            .chain(closing.and_then(|(from, to, folding)| folding.then_some((from, to?))))
        {
            let normal = scaled(unit_normal(start, end), self.half);
            // Each band reaches a little past either end, so that it overlaps the band and the
            // join beside it rather than abutting them: pieces sharing an edge exactly leave a
            // sample on that edge to neither, and a disk's worth of them left a sixteenth out of
            // pixels in the middle of the stroke.
            let along = scaled(
                tiny_skia::Point::from_xy(normal.y, -normal.x),
                self.overlap / self.half,
            );
            let (start, end) = if along.dot(minus(end, start)) >= 0.0 {
                (minus(start, along), plus(end, along))
            } else {
                (plus(start, along), minus(end, along))
            };
            push_convex(
                out,
                &[
                    plus(start, normal),
                    plus(end, normal),
                    minus(end, normal),
                    minus(start, normal),
                ],
            );
        }
        // Every vertex with a chord on either side: all of them round a closed subpath, the inner
        // ones of an open one. The file's join where the file states the vertex, and the round
        // join a smooth curve's stroke has where the flattening made it.
        let count = points.len();
        for vertex in 0..count {
            let before = vertex
                .checked_sub(1)
                .or(subpath.closed.then_some(count.saturating_sub(1)));
            let after = Some(vertex.saturating_add(1))
                .filter(|&after| after < count)
                .or(subpath.closed.then_some(0));
            let (Some(before), Some(after)) = (before, after) else {
                continue;
            };
            // A join between two chords the stroker draws is the stroker's.
            if !folds(vertex) && !folds(after) {
                continue;
            }
            let (Some(&from), Some(&at), Some(&to)) =
                (points.get(before), points.get(vertex), points.get(after))
            else {
                continue;
            };
            // Where a stretch the stroker draws meets the join, the join is turned from the
            // stretch's own end, which is square to its exact tangent rather than to its last
            // chord, and reaches a little into it (ADR 1359).
            let into = (!folds(vertex))
                .then(|| subpath.arriving.get(vertex).copied().flatten())
                .flatten();
            let onward = (!folds(after))
                .then(|| subpath.leaving.get(vertex).copied().flatten())
                .flatten();
            let from = into.map_or(from, |direction| minus(at, direction));
            let to = onward.map_or(to, |direction| plus(at, direction));
            let stated = subpath.stated.get(vertex).copied().unwrap_or(false);
            let join = if stated {
                self.join
            } else {
                tiny_skia::LineJoin::Round
            };
            let tangents = stated.then(|| subpath.tangents_at(vertex)).flatten();
            self.push_join(
                out,
                (from, at, to),
                (join, tangents),
                (into.is_some(), onward.is_some()),
            );
        }
        if !subpath.closed {
            // A cap on a stretch the stroker draws is square to its exact tangent, as the
            // stretch's butt end is, and reaches a little into it (ADR 1359).
            // A cap is turned from the path's own tangent at its end, as §8.4.3.3 draws it about
            // the endpoint, and not from a folding curve's last chord (ADR 2252).
            if let (Some(&first), Some(&second)) = (points.first(), points.get(1)) {
                let exact = subpath.leaving.first().copied().flatten();
                let outward = exact.map_or(minus(first, second), |leaving| scaled(leaving, -1.0));
                self.push_cap(out, (outward, first), exact.is_some() && !folds(1));
            }
            if let Some(&before) = points.get(count.saturating_sub(2)) {
                let exact = subpath.arriving.last().copied().flatten();
                let outward = exact.unwrap_or_else(|| minus(last, before));
                self.push_cap(
                    out,
                    (outward, last),
                    exact.is_some() && !folds(count.saturating_sub(1)),
                );
            }
        }
    }

    /// `corner` moved `self.overlap` along `direction`: how a piece meeting a stretch the stroker
    /// draws reaches into it rather than abutting it, since abutting pieces lose the samples on
    /// their shared edge to both (trap 56).
    fn bridged(&self, corner: tiny_skia::Point, direction: tiny_skia::Point) -> tiny_skia::Point {
        let length = direction.length();
        if length > 0.0 {
            plus(corner, scaled(direction, self.overlap / length))
        } else {
            corner
        }
    }

    /// Adds the join at `at` between the chords from `from` and to `to`, on the outer side of their
    /// turn — ISO 32000-2 §8.4.3.4, with §8.4.3.5's limit on a mitre, decided and shaped by
    /// `tangents`, the directions the path itself arrives and leaves along, where it states the
    /// vertex.
    fn push_join(
        &self,
        out: &mut tiny_skia::PathBuilder,
        (from, at, to): (tiny_skia::Point, tiny_skia::Point, tiny_skia::Point),
        (join, tangents): (
            tiny_skia::LineJoin,
            Option<(tiny_skia::Point, tiny_skia::Point)>,
        ),
        (into_a_stretch, onward_a_stretch): (bool, bool),
    ) {
        let (first, second) = (unit_normal(from, at), unit_normal(at, to));
        // The outer side is the one the path turns away from.
        let outward = if minus(at, from).cross(minus(to, at)) > 0.0 {
            -self.half
        } else {
            self.half
        };
        let (near, far) = (scaled(first, outward), scaled(second, outward));
        let cosine = first.dot(second).clamp(-1.0, 1.0);
        // The corners on a stretch's side reach back into it; at most one side is a stretch,
        // since a join between two is the stroker's.
        let (back, ahead) = (minus(from, at), minus(to, at));
        let (centre, near_corner, far_corner) = if into_a_stretch {
            (
                self.bridged(at, back),
                self.bridged(plus(at, near), back),
                plus(at, far),
            )
        } else if onward_a_stretch {
            (
                self.bridged(at, ahead),
                plus(at, near),
                self.bridged(plus(at, far), ahead),
            )
        } else {
            (at, plus(at, near), plus(at, far))
        };
        match join {
            tiny_skia::LineJoin::Round => {
                let mut corners = vec![centre, near_corner];
                // The outer arc turns from `near` towards the way the path arrives, whichever way
                // it then leaves: at a turn of nearly a half turn `near` and `far` are opposite,
                // and the sign of their cross product, a rounding, put the arc behind the vertex
                // and left a notch where the path doubles back (ADR 2269).
                let turning = near.cross(minus(at, from));
                corners.extend(arc(at, (near, turning), cosine.acos(), self.step));
                corners.push(far_corner);
                push_convex(out, &corners);
            }
            tiny_skia::LineJoin::Miter | tiny_skia::LineJoin::MiterClip => {
                // The bevel between the chords' own bands, which their corners need whatever the
                // mitre is; and beyond it the mitre, which is the segments' own: decided, shaped and
                // put on its side by the path's tangents where it states the vertex. Near a vertex
                // a curve's first chord may lean off its tangent far enough to turn the other way,
                // so nothing of the mitre is taken from the chords there (ADR 2252).
                push_convex(out, &[centre, near_corner, far_corner]);
                let (first, second, outward) =
                    tangents.map_or((first, second, outward), |(arriving, leaving)| {
                        let outward = if arriving.cross(leaving) > 0.0 {
                            -self.half
                        } else {
                            self.half
                        };
                        // As vectors, for `push_cap`'s reason.
                        (
                            unit_normal(tiny_skia::Point::zero(), arriving),
                            unit_normal(tiny_skia::Point::zero(), leaving),
                            outward,
                        )
                    });
                let (near, far) = (scaled(first, outward), scaled(second, outward));
                let cosine = first.dot(second).clamp(-1.0, 1.0);
                // §8.4.3.5: the mitre's length over the line width is 1 / sin(φ / 2), φ the angle
                // between the segments, which is 1 / cos(θ / 2) of the turn θ between normals.
                let half_turn = f32::midpoint(1.0, cosine).sqrt();
                if half_turn > 0.0 && 1.0 / half_turn <= self.limit {
                    let tip = scaled(plus(near, far), 1.0 / (1.0 + cosine));
                    push_convex(out, &[at, plus(at, near), plus(at, tip), plus(at, far)]);
                }
            }
            tiny_skia::LineJoin::Bevel => {
                push_convex(out, &[centre, near_corner, far_corner]);
            }
        }
    }

    /// Adds the cap at `end`, the path leaving it along `outward` — ISO 32000-2 §8.4.3.3.
    ///
    /// The direction is taken as a vector and never as a point beside the end: a curve's tangent
    /// there runs to its nearest distinct control point, which may lie a part in a million away,
    /// and added to the end in `f32` it would be the end again and state no direction at all.
    fn push_cap(
        &self,
        out: &mut tiny_skia::PathBuilder,
        (outward, end): (tiny_skia::Point, tiny_skia::Point),
        on_a_stretch: bool,
    ) {
        let length = outward.length();
        if !(length.is_finite() && length > 0.0) {
            return;
        }
        // The path's own direction at the half-width's length, and square to it.
        let beyond = scaled(outward, self.half / length);
        let normal = tiny_skia::Point::from_xy(-beyond.y, beyond.x);
        // A cap on a stretch the stroker draws reaches back into its butt end.
        let back = |corner: tiny_skia::Point| {
            if on_a_stretch {
                self.bridged(corner, scaled(beyond, -1.0))
            } else {
                corner
            }
        };
        match self.cap {
            tiny_skia::LineCap::Butt => {}
            tiny_skia::LineCap::Round => {
                let mut corners = vec![back(plus(end, normal))];
                // A half turn from one side to the other, through the side beyond the end.
                let turning = normal.cross(beyond);
                corners.extend(arc(end, (normal, turning), std::f32::consts::PI, self.step));
                corners.push(back(minus(end, normal)));
                push_convex(out, &corners);
            }
            tiny_skia::LineCap::Square => push_convex(
                out,
                &[
                    back(plus(end, normal)),
                    plus(plus(end, normal), beyond),
                    plus(minus(end, normal), beyond),
                    back(minus(end, normal)),
                ],
            ),
        }
    }
}

/// The points strictly inside the arc about `centre` that begins at `centre + near` and spans
/// `angle`, turning the way `turning`'s sign says — positive the way from `x` to `y` — cut into
/// sides of at most `step`.
fn arc(
    centre: tiny_skia::Point,
    (near, turning): (tiny_skia::Point, f32),
    angle: f32,
    step: f32,
) -> Vec<tiny_skia::Point> {
    let sides = sides_for(angle, step);
    // Each side's corners on the radius at which its triangle has its circular sector's own area,
    // `R² sin φ = r² φ`, so that the polygon is neither short of the round join nor over it on the
    // whole: inscribed, every arc would come in short by a sixty-fourth of a pixel's worth along
    // its length, and a disk's rim is nothing but arcs.
    let side = angle / f32::from(u16::try_from(sides).unwrap_or(FOLD_CHORDS));
    let equal_area = if side > 0.0 {
        (side / side.sin()).sqrt()
    } else {
        1.0
    };
    let radius = near.length() * equal_area;
    let start = near.y.atan2(near.x);
    let sweep = if turning >= 0.0 { angle } else { -angle };
    (1..sides)
        .map(|side| {
            let turned = start + sweep * fraction_of(side, sides);
            tiny_skia::Point::from_xy(
                centre.x + radius * turned.cos(),
                centre.y + radius * turned.sin(),
            )
        })
        .collect()
}

/// The largest angle a side of an arc of `radius` may span and lie within `tolerance` of it.
fn arc_step(radius: f32, tolerance: f32) -> f32 {
    2.0 * (1.0 - (tolerance / radius).min(1.0)).acos()
}

/// How many sides an arc spanning `angle` is cut into for each to span at most `step`.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "the value is clamped into 1.0..=FOLD_CHORDS before the cast"
)]
fn sides_for(angle: f32, step: f32) -> u32 {
    let sides = (angle / step).ceil();
    if sides.is_finite() {
        sides.clamp(1.0, f32::from(FOLD_CHORDS)) as u32
    } else {
        1
    }
}

/// `a + b`.
fn plus(a: tiny_skia::Point, b: tiny_skia::Point) -> tiny_skia::Point {
    tiny_skia::Point::from_xy(a.x + b.x, a.y + b.y)
}

/// `a − b`.
fn minus(a: tiny_skia::Point, b: tiny_skia::Point) -> tiny_skia::Point {
    tiny_skia::Point::from_xy(a.x - b.x, a.y - b.y)
}

/// `point` times `factor`.
fn scaled(point: tiny_skia::Point, factor: f32) -> tiny_skia::Point {
    tiny_skia::Point::from_xy(point.x * factor, point.y * factor)
}

/// The unit normal to the chord from `start` to `end`, turned a quarter to one side; zero for a
/// chord of no length.
fn unit_normal(start: tiny_skia::Point, end: tiny_skia::Point) -> tiny_skia::Point {
    let along = minus(end, start);
    let length = along.length();
    if length > 0.0 {
        tiny_skia::Point::from_xy(-along.y / length, along.x / length)
    } else {
        tiny_skia::Point::zero()
    }
}

/// Adds a convex polygon as a closed subpath wound so that its signed area is positive, which is
/// what makes every piece of one outline add rather than cancel under the non-zero rule.
///
/// The area is taken about the first corner and in `f64`, where each product of two differences
/// of `f32` coordinates is exact. Taken about the origin in `f32`, each product is a coordinate
/// squared, rounded to a part in `2^24` of that: at the coordinates a page states, a sixteenth of
/// a unit and more, which is more than a small join has, so such a piece came out wound the other
/// way and took its area out of the bands it overlaps (ADR 2269).
fn push_convex(out: &mut tiny_skia::PathBuilder, corners: &[tiny_skia::Point]) {
    let Some(&origin) = corners.first() else {
        return;
    };
    let about = |corner: &tiny_skia::Point| {
        (
            f64::from(corner.x) - f64::from(origin.x),
            f64::from(corner.y) - f64::from(origin.y),
        )
    };
    let twice_area: f64 = corners
        .iter()
        .zip(corners.iter().cycle().skip(1))
        .map(|(one, next)| {
            let (one, next) = (about(one), about(next));
            one.0 * next.1 - one.1 * next.0
        })
        .sum();
    if !twice_area.is_finite() || twice_area == 0.0 {
        return;
    }
    let mut ordered: Vec<tiny_skia::Point> = corners.to_vec();
    if twice_area < 0.0 {
        ordered.reverse();
    }
    let mut ordered = ordered.into_iter();
    if let Some(first) = ordered.next() {
        out.move_to(first.x, first.y);
        for corner in ordered {
            out.line_to(corner.x, corner.y);
        }
        out.close();
    }
}

/// Whether any curve of `path` may have a radius of curvature under `radius` — see
/// [`folding_stroke_outline`].
pub(crate) fn curves_may_fold(path: &tiny_skia::Path, radius: f32) -> bool {
    let mut current = tiny_skia::Point::zero();
    path.segments().any(|segment| {
        let folds = match segment {
            tiny_skia::PathSegment::QuadTo(control, to) => {
                quadratic_may_fold([current, control, to], radius)
            }
            tiny_skia::PathSegment::CubicTo(first, second, to) => {
                cubic_may_fold([current, first, second, to], radius)
            }
            tiny_skia::PathSegment::MoveTo(_)
            | tiny_skia::PathSegment::LineTo(_)
            | tiny_skia::PathSegment::Close => false,
        };
        if let tiny_skia::PathSegment::MoveTo(to)
        | tiny_skia::PathSegment::LineTo(to)
        | tiny_skia::PathSegment::QuadTo(_, to)
        | tiny_skia::PathSegment::CubicTo(_, _, to) = segment
        {
            current = to;
        }
        folds
    })
}

/// Per segment of `path`, in the order [`tiny_skia::Path::segments`] gives them, whether it is a
/// curve that may bend more tightly than `radius` — asked once, and read by both halves of
/// [`folding_stroke_outline`].
fn folding_segments(path: &tiny_skia::Path, radius: f32) -> Vec<bool> {
    let mut current = tiny_skia::Point::zero();
    path.segments()
        .map(|segment| {
            let folds = match segment {
                tiny_skia::PathSegment::QuadTo(control, to) => {
                    quadratic_may_fold([current, control, to], radius)
                }
                tiny_skia::PathSegment::CubicTo(first, second, to) => {
                    cubic_may_fold([current, first, second, to], radius)
                }
                tiny_skia::PathSegment::MoveTo(_)
                | tiny_skia::PathSegment::LineTo(_)
                | tiny_skia::PathSegment::Close => false,
            };
            if let tiny_skia::PathSegment::MoveTo(to)
            | tiny_skia::PathSegment::LineTo(to)
            | tiny_skia::PathSegment::QuadTo(_, to)
            | tiny_skia::PathSegment::CubicTo(_, _, to) = segment
            {
                current = to;
            }
            folds
        })
        .collect()
}

/// One subpath as the points of its chords.
struct Flattened {
    /// The points, a repeated point dropped, since a chord of no length has no direction to join.
    points: Vec<tiny_skia::Point>,
    /// Per point, whether the path states it rather than the flattening making it — where the
    /// file's own join is drawn.
    stated: Vec<bool>,
    /// Whether the subpath is closed, its last point joined back to its first.
    closed: bool,
    /// Per point, whether the chord arriving at it is part of a curve that may fold, which is what
    /// [`Pieces::push_subpath`] states as pieces; the chord closing a closed subpath is a line and
    /// never is. The rest of the subpath is [`non_folding_runs`]'.
    folds: Vec<bool>,
    /// Per point, the direction the path arrives at it along, exactly — a curve's end tangent
    /// rather than its last chord's — where the path states the point; see [`Pieces::bridged`].
    arriving: Vec<Option<tiny_skia::Point>>,
    /// The same, for the direction the path leaves it along.
    leaving: Vec<Option<tiny_skia::Point>>,
}

impl Flattened {
    /// A subpath beginning at `start`.
    fn at(start: tiny_skia::Point) -> Self {
        Self {
            points: vec![start],
            stated: vec![true],
            closed: false,
            folds: vec![false],
            arriving: vec![None],
            leaving: vec![None],
        }
    }

    /// Adds a point, dropping one that repeats the last.
    fn push(&mut self, point: tiny_skia::Point, stated: bool, folds: bool) {
        if self.points.last() == Some(&point) {
            if let Some(last) = self.stated.last_mut() {
                *last |= stated;
            }
        } else {
            self.points.push(point);
            self.stated.push(stated);
            self.folds.push(folds);
            self.arriving.push(None);
            self.leaving.push(None);
        }
    }

    /// Records the exact directions a segment leaves its start and arrives at its end along,
    /// once its points are pushed: `start` is the index of the point it began at.
    fn tangents(
        &mut self,
        start: usize,
        (leaving, arriving): (tiny_skia::Point, tiny_skia::Point),
    ) {
        let last = self.points.len().saturating_sub(1);
        if last == start {
            return;
        }
        if let Some(slot) = self.leaving.get_mut(start)
            && leaving.length() > 0.0
        {
            *slot = Some(leaving);
        }
        if let Some(slot) = self.arriving.get_mut(last)
            && arriving.length() > 0.0
        {
            *slot = Some(arriving);
        }
    }

    /// The directions the path arrives at and leaves `vertex` along, where it records both.
    ///
    /// §8.4.3.5's angle is the one "between the segments", which at a vertex the path states is
    /// its own tangents', not its chords': a chord leans off its curve's tangent by more as the
    /// flatness coarsens, and a mitre decided from it appears or vanishes with the chords (ADR
    /// 2252).
    fn tangents_at(&self, vertex: usize) -> Option<(tiny_skia::Point, tiny_skia::Point)> {
        Some((
            self.arriving.get(vertex).copied().flatten()?,
            self.leaving.get(vertex).copied().flatten()?,
        ))
    }

    /// Adds a curve's chords: `steps` points along it, the last its end point.
    fn push_curve(
        &mut self,
        (steps, folds): (u32, bool),
        point_at: impl Fn(f32) -> tiny_skia::Point,
    ) {
        for step in 1..=steps {
            self.push(point_at(fraction_of(step, steps)), step == steps, folds);
        }
    }

    /// Closes the subpath: its last point is joined back to its first, and a last point that
    /// repeats the first is that join already.
    fn close(&mut self) {
        if self.points.len() > 1 && self.points.first() == self.points.last() {
            self.points.pop();
            self.stated.pop();
            self.leaving.pop();
            // The chord that arrived at the popped point now arrives at the first, and is still
            // what it was.
            if let (Some(arriving), Some(first)) = (self.folds.pop(), self.folds.first_mut()) {
                *first = arriving;
            }
            if let (Some(arriving), Some(first)) = (self.arriving.pop(), self.arriving.first_mut())
            {
                *first = arriving;
            }
        } else if let (Some(&first), Some(&last)) = (self.points.first(), self.points.last())
            && self.points.len() > 1
        {
            // The closing chord is a line, and its direction is exact.
            let closing = minus(first, last);
            if let Some(slot) = self.leaving.last_mut() {
                *slot = Some(closing);
            }
            if let Some(slot) = self.arriving.first_mut() {
                *slot = Some(closing);
            }
        }
        self.closed = true;
    }
}

/// Every subpath of `path` as the points of its chords, each within `tolerance` of its curve.
///
/// A second bound keeps the stroke's own boundary within the same distance: a chord turning by `θ`
/// is `h` from its band's edge, and that edge lies `hθ²/8` inside the curve's offset, so no chord
/// turns by more than `turn`. The two bounds meet on a wide stroke of a tight curve, whose rim is
/// all offset, and a thin stroke's `turn` is loose enough that the first decides.
fn flattened_subpaths(
    path: &tiny_skia::Path,
    (tolerance, turn): (f32, f32),
    folds: &[bool],
) -> Vec<Flattened> {
    let mut subpaths: Vec<Flattened> = Vec::new();
    let mut current = tiny_skia::Point::zero();
    for (index, segment) in path.segments().enumerate() {
        let folding = folds.get(index).copied().unwrap_or(false);
        if states_no_length(segment, current) {
            continue;
        }
        match segment {
            tiny_skia::PathSegment::MoveTo(to) => {
                subpaths.push(Flattened::at(to));
                current = to;
            }
            tiny_skia::PathSegment::LineTo(to) => {
                if let Some(subpath) = subpaths.last_mut() {
                    let start = subpath.points.len().saturating_sub(1);
                    subpath.push(to, true, false);
                    subpath.tangents(start, (minus(to, current), minus(to, current)));
                }
                current = to;
            }
            tiny_skia::PathSegment::QuadTo(control, to) => {
                let start = current;
                let steps = chords(
                    2.0 * second_difference(start, control, to).length(),
                    tolerance,
                )
                .max(turns(
                    hodograph_span(&[minus(control, start), minus(to, control)]),
                    turn,
                ));
                if let Some(subpath) = subpaths.last_mut() {
                    let first = subpath.points.len().saturating_sub(1);
                    subpath.push_curve((steps, folding), |t| {
                        let u = 1.0 - t;
                        tiny_skia::Point::from_xy(
                            u * u * start.x + 2.0 * u * t * control.x + t * t * to.x,
                            u * u * start.y + 2.0 * u * t * control.y + t * t * to.y,
                        )
                    });
                    subpath.tangents(first, end_tangents(&[start, control, to]));
                }
                current = to;
            }
            tiny_skia::PathSegment::CubicTo(first, second, to) => {
                let start = current;
                let bend = 6.0
                    * second_difference(start, first, second)
                        .length()
                        .max(second_difference(first, second, to).length());
                let steps = chords(bend, tolerance).max(turns(
                    hodograph_span(&[minus(first, start), minus(second, first), minus(to, second)]),
                    turn,
                ));
                if let Some(subpath) = subpaths.last_mut() {
                    let at = subpath.points.len().saturating_sub(1);
                    subpath.push_curve((steps, folding), |t| {
                        let u = 1.0 - t;
                        let weights = (u * u * u, 3.0 * u * u * t, 3.0 * u * t * t, t * t * t);
                        tiny_skia::Point::from_xy(
                            weights.0 * start.x
                                + weights.1 * first.x
                                + weights.2 * second.x
                                + weights.3 * to.x,
                            weights.0 * start.y
                                + weights.1 * first.y
                                + weights.2 * second.y
                                + weights.3 * to.y,
                        )
                    });
                    subpath.tangents(at, end_tangents(&[start, first, second, to]));
                }
                current = to;
            }
            tiny_skia::PathSegment::Close => {
                if let Some(subpath) = subpaths.last_mut() {
                    subpath.close();
                    if let Some(&start) = subpath.points.first() {
                        current = start;
                    }
                }
            }
        }
    }
    subpaths
}

/// Whether `segment`, begun at `start`, is a curve every control point of which is `start`: a
/// point, which adds nothing to the stroke and states no direction to join.
///
/// Its chords would not be the point. A Bézier's points are weighted sums, and weights summing to
/// one in exact arithmetic sum to a part in `2^24` either side of it in `f32`, so such a curve's
/// chords are a few steps of a rounding long and point anywhere, and the joins at either end of
/// them were turned from those directions: `issue14438.pdf`'s ink states its corners with such
/// curves, and lost the round join at them at most scales (ADR 2269).
fn states_no_length(segment: tiny_skia::PathSegment, start: tiny_skia::Point) -> bool {
    match segment {
        tiny_skia::PathSegment::QuadTo(control, to) => control == start && to == start,
        tiny_skia::PathSegment::CubicTo(first, second, to) => {
            first == start && second == start && to == start
        }
        tiny_skia::PathSegment::MoveTo(_)
        | tiny_skia::PathSegment::LineTo(_)
        | tiny_skia::PathSegment::Close => false,
    }
}

/// The directions a Bézier leaves its first control point along and arrives at its last along:
/// towards the nearest control point that differs, which is the curve's own tangent there.
fn end_tangents(control: &[tiny_skia::Point]) -> (tiny_skia::Point, tiny_skia::Point) {
    let (Some(&start), Some(&end)) = (control.first(), control.last()) else {
        return (tiny_skia::Point::zero(), tiny_skia::Point::zero());
    };
    let leaving = control
        .iter()
        .skip(1)
        .find(|point| **point != start)
        .map_or(tiny_skia::Point::zero(), |point| minus(*point, start));
    let arriving = control
        .iter()
        .rev()
        .skip(1)
        .find(|point| **point != end)
        .map_or(tiny_skia::Point::zero(), |point| minus(end, *point));
    (leaving, arriving)
}

/// How far a side of a round join's or cap's arc may lie from it, in device pixels — the figure
/// `crate::area` flattens every fill to, chosen there against the raster's own depth.
const ARC_FLATNESS: f32 = 1.0 / 256.0;

/// How far a chord of a folding curve may lie from it, in device pixels — the flatness §10.7.2
/// leaves to this processor, chosen by the project owner (`doc/questions/A559`, ADR 2252).
///
/// [`scan::Exact::Pieces`] is measured exactly from its union's boundary (ADR 2122), so what a
/// pixel reads is the flattened set's area, and the chords' own error is the only distance from
/// the curve's: a boundary crosses a pixel over at most `√2` of it, so a pixel is within `√2 / 20`
/// of its area, 18 levels of 255. The exact measure's cost is the pairs of pieces that overlap
/// (ADR 2164 §3), and a chord's band overlaps every band within its half-width either side, so
/// the twentieth is what holds that measure under a fifth more than the library's sixteenths cost
/// at a sixty-fourth: on `issue14415.pdf`, whose one-point strokes fold at a hundred and
/// seventy-five cusps, each chord is a band and a join.
///
/// [`scan::Exact::Pieces`]: crate::scan::Exact::Pieces
const FOLD_FLATNESS: f32 = 1.0 / 20.0;

/// The most chords one folding curve, or sides one arc, is cut into: a cost guard, reached only by
/// a curve whose bend is millions of times the flatness in device pixels.
const FOLD_CHORDS: u16 = 4096;

/// `p₀ − 2p₁ + p₂`.
fn second_difference(
    start: tiny_skia::Point,
    middle: tiny_skia::Point,
    end: tiny_skia::Point,
) -> tiny_skia::Point {
    tiny_skia::Point::from_xy(
        start.x - 2.0 * middle.x + end.x,
        start.y - 2.0 * middle.y + end.y,
    )
}

/// The chords a curve whose second derivative is at most `bend` needs to lie within `tolerance`
/// of it: the uniform polyline through `n + 1` points of a Bézier lies within `max|B″| / (8n²)`.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "the value is clamped into 1.0..=FOLD_CHORDS before the cast"
)]
fn chords(bend: f32, tolerance: f32) -> u32 {
    let steps = (bend / (8.0 * tolerance)).sqrt().ceil();
    if steps.is_finite() {
        steps.clamp(1.0, f32::from(FOLD_CHORDS)) as u32
    } else {
        u32::from(FOLD_CHORDS)
    }
}

/// The chords a curve whose tangent turns through at most `span` needs, spread evenly over its
/// parameter, for them to turn by `turn` on average.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "the value is clamped into 1.0..=FOLD_CHORDS before the cast"
)]
fn turns(span: f32, turn: f32) -> u32 {
    let steps = (span / turn).ceil();
    if steps.is_finite() {
        steps.clamp(1.0, f32::from(FOLD_CHORDS)) as u32
    } else {
        u32::from(FOLD_CHORDS)
    }
}

/// How far a curve's tangent can turn, from its hodograph's control points: the widest angle
/// between two of them where their hull keeps clear of the origin, and a whole turn where it does
/// not, since the tangent may then point anywhere.
fn hodograph_span(points: &[tiny_skia::Point]) -> f32 {
    let clear = match points {
        [one, two] => distance_to_segment(*one, *two) > 0.0,
        [one, two, three] => distance_to_triangle([*one, *two, *three]) > 0.0,
        _ => false,
    };
    if !clear {
        return std::f32::consts::TAU;
    }
    let mut widest = 0.0_f32;
    for (index, one) in points.iter().enumerate() {
        for two in points.get(index.saturating_add(1)..).unwrap_or_default() {
            let cosine = one.dot(*two) / (one.length() * two.length());
            widest = widest.max(cosine.clamp(-1.0, 1.0).acos());
        }
    }
    widest
}

/// Step `step` of `steps` as a parameter, exactly 1.0 at the last.
#[expect(
    clippy::cast_precision_loss,
    reason = "both are at most FOLD_CHORDS, exact in f32"
)]
fn fraction_of(step: u32, steps: u32) -> f32 {
    if step >= steps {
        1.0
    } else {
        step as f32 / steps as f32
    }
}

/// Whether a quadratic may have a radius of curvature under `radius` — see
/// [`folding_stroke_outline`]. `B′` runs along the segment between `2(p₁ − p₀)` and
/// `2(p₂ − p₁)` and `B″` is the constant `2(p₀ − 2p₁ + p₂)`.
///
/// The bound is loosest where the parameter runs slowly, near a control point close to its end
/// point, so a curve that fails it is halved and each half asked again, down to
/// [`FOLD_HALVINGS`]: the bound on a half is the curve's own over that half, and it tightens
/// towards the radius itself as the halves shrink. A half that still fails at the last halving is
/// taken to fold.
fn quadratic_may_fold(points: [tiny_skia::Point; 3], radius: f32) -> bool {
    quadratic_may_fold_within(points, radius, FOLD_HALVINGS)
}

/// [`quadratic_may_fold`], with `halvings` left.
fn quadratic_may_fold_within(
    [start, control, end]: [tiny_skia::Point; 3],
    radius: f32,
    halvings: u8,
) -> bool {
    let slowest = 2.0 * distance_to_segment(minus(control, start), minus(end, control));
    let bend = 2.0 * second_difference(start, control, end).length();
    if slowest * slowest >= radius * bend {
        return false;
    }
    let Some(halvings) = halvings.checked_sub(1) else {
        return true;
    };
    let (one, two) = (midpoint(start, control), midpoint(control, end));
    let middle = midpoint(one, two);
    quadratic_may_fold_within([start, one, middle], radius, halvings)
        || quadratic_may_fold_within([middle, two, end], radius, halvings)
}

/// Whether a cubic may have a radius of curvature under `radius` — see
/// [`folding_stroke_outline`]. `B′` lies in the triangle of `3(p₁ − p₀)`, `3(p₂ − p₁)` and
/// `3(p₃ − p₂)`, and `B″` is linear, so largest at an end: `6(p₀ − 2p₁ + p₂)` or
/// `6(p₁ − 2p₂ + p₃)`. Halved as [`quadratic_may_fold`] is, for its reason.
fn cubic_may_fold(points: [tiny_skia::Point; 4], radius: f32) -> bool {
    cubic_may_fold_within(points, radius, FOLD_HALVINGS)
}

/// [`cubic_may_fold`], with `halvings` left.
fn cubic_may_fold_within(
    [start, first, second, end]: [tiny_skia::Point; 4],
    radius: f32,
    halvings: u8,
) -> bool {
    let slowest = 3.0
        * distance_to_triangle([
            minus(first, start),
            minus(second, first),
            minus(end, second),
        ]);
    let bend = 6.0
        * second_difference(start, first, second)
            .length()
            .max(second_difference(first, second, end).length());
    if slowest * slowest >= radius * bend {
        return false;
    }
    let Some(halvings) = halvings.checked_sub(1) else {
        return true;
    };
    // de Casteljau at one half.
    let (near, across, far) = (
        midpoint(start, first),
        midpoint(first, second),
        midpoint(second, end),
    );
    let (before, after) = (midpoint(near, across), midpoint(across, far));
    let middle = midpoint(before, after);
    cubic_may_fold_within([start, near, before, middle], radius, halvings)
        || cubic_may_fold_within([middle, after, far, end], radius, halvings)
}

/// How many times a curve failing [`quadratic_may_fold`]'s or [`cubic_may_fold`]'s bound is
/// halved before it is taken to fold: a sixty-fourth of the curve is short enough that the bound
/// on it is within a few per cent of the radius it bounds.
const FOLD_HALVINGS: u8 = 6;

/// The point halfway between two.
fn midpoint(one: tiny_skia::Point, two: tiny_skia::Point) -> tiny_skia::Point {
    tiny_skia::Point::from_xy(f32::midpoint(one.x, two.x), f32::midpoint(one.y, two.y))
}

/// The distance from the origin to the segment from `start` to `end`.
fn distance_to_segment(start: tiny_skia::Point, end: tiny_skia::Point) -> f32 {
    let along = minus(end, start);
    let length = along.dot(along);
    let t = if length > 0.0 {
        (-start.dot(along) / length).clamp(0.0, 1.0)
    } else {
        0.0
    };
    plus(start, scaled(along, t)).length()
}

/// The distance from the origin to the triangle of three points, zero where it holds the origin.
fn distance_to_triangle([one, two, three]: [tiny_skia::Point; 3]) -> f32 {
    let sides = (one.cross(two), two.cross(three), three.cross(one));
    let inside = (sides.0 >= 0.0 && sides.1 >= 0.0 && sides.2 >= 0.0)
        || (sides.0 <= 0.0 && sides.1 <= 0.0 && sides.2 <= 0.0);
    if inside {
        return 0.0;
    }
    distance_to_segment(one, two)
        .min(distance_to_segment(two, three))
        .min(distance_to_segment(three, one))
}

#[cfg(test)]
mod tests {

    use super::{FOLD_FLATNESS, folding_stroke_outline, push_convex, push_wound_positively};

    /// Whether the pieces cover `(x, y)` under the non-zero rule, counted in `f64` along a ray to
    /// the right — a reference that shares nothing with `crate::area`.
    fn covers(pieces: &tiny_skia::Path, (x, y): (f64, f64)) -> bool {
        let mut winding = 0_i32;
        let (mut start, mut current) = ((0.0, 0.0), (0.0, 0.0));
        let cross = |from: (f64, f64), to: (f64, f64), winding: &mut i32| {
            if (from.1 <= y) != (to.1 <= y)
                && from.0 + (y - from.1) * (to.0 - from.0) / (to.1 - from.1) > x
            {
                *winding = winding.saturating_add(if to.1 > from.1 { 1 } else { -1 });
            }
        };
        for segment in pieces.segments() {
            match segment {
                tiny_skia::PathSegment::MoveTo(point) => {
                    start = (f64::from(point.x), f64::from(point.y));
                    current = start;
                }
                tiny_skia::PathSegment::LineTo(point) => {
                    let next = (f64::from(point.x), f64::from(point.y));
                    cross(current, next, &mut winding);
                    current = next;
                }
                tiny_skia::PathSegment::Close => {
                    cross(current, start, &mut winding);
                    current = start;
                }
                tiny_skia::PathSegment::QuadTo(..) | tiny_skia::PathSegment::CubicTo(..) => {
                    panic!("the pieces are polygons")
                }
            }
        }
        winding != 0
    }

    /// The points `(radius, degrees)` about `centre` that the pieces leave out.
    fn left_out(
        pieces: &tiny_skia::Path,
        centre: (f64, f64),
        (radii, degrees): (&[f64], std::ops::RangeInclusive<i32>),
    ) -> Vec<(f64, i32)> {
        let mut missing = Vec::new();
        for &radius in radii {
            for angle in degrees.clone().step_by(5) {
                let turned = f64::from(angle).to_radians();
                let point = (
                    centre.0 + radius * turned.cos(),
                    centre.1 + radius * turned.sin(),
                );
                if !covers(pieces, point) {
                    missing.push((radius, angle));
                }
            }
        }
        missing
    }

    /// A circle of radius 4 and one of radius 1, stroked at `8 w`, are the disks of radius 8 and 5
    /// (§8.4.3.2's set; the first's inside is within the half-width of its curve), and the union of
    /// the pieces `folding_stroke_outline` states for them is measured exactly by `crate::area`
    /// under the non-zero rule, at the scale the pieces were flattened for. The shortfall is held
    /// to what [`FOLD_FLATNESS`] promises: the set's rim, in device pixels, times that flatness.
    /// The expected areas are the Bézier circle's own — its area plus its length times the
    /// half-width plus `πh²`, the outer parallel body of a convex region — computed from the
    /// control points (`numpy`, 200 000 samples a curve): 201.0910 and 78.5398 (ADR 1374).
    ///
    /// Measured at scales 1 and 2 only: from 4 up the pieces overlap in every row past
    /// `crate::area`'s work bound, which is why `scan::mask_fill` measures them from their union's
    /// boundary instead (ADR 2122), and the test below holds that measure to the flattened set.
    #[test]
    fn a_small_disc_s_pieces_add_up_to_its_set_within_the_flatness() {
        let kappa = 0.5523_f32;
        let half = 4.0_f32;
        for (radius, set) in [(4.0_f32, 201.091_f64), (1.0, 78.5398)] {
            for scale in [1.0_f32, 2.0] {
                let (cx, cy, r, k) = (20.0_f32, 20.0_f32, radius, kappa * radius);
                let mut builder = tiny_skia::PathBuilder::new();
                builder.move_to(cx + r, cy);
                builder.cubic_to(cx + r, cy + k, cx + k, cy + r, cx, cy + r);
                builder.cubic_to(cx - k, cy + r, cx - r, cy + k, cx - r, cy);
                builder.cubic_to(cx - r, cy - k, cx - k, cy - r, cx, cy - r);
                builder.cubic_to(cx + k, cy - r, cx + r, cy - k, cx + r, cy);
                builder.close();
                let circle = builder.finish().expect("a circle");
                let style = tiny_skia::Stroke {
                    width: 2.0 * half,
                    line_join: tiny_skia::LineJoin::Round,
                    ..tiny_skia::Stroke::default()
                };
                let pieces = folding_stroke_outline(&circle, &style, scale).expect("it folds");
                let at = tiny_skia::Transform::identity();
                let region =
                    crate::area::region(&pieces, at, (40, 40), (0, 0)).expect("on the surface");
                let mut target = vec![0_u8; 40 * 40];
                assert!(crate::area::fill(
                    &mut crate::area::Buffers::default(),
                    &mut target,
                    (40, region),
                    (&pieces, true),
                    tiny_skia::FillRule::Winding,
                    at,
                ));
                let union = target.iter().map(|&level| f64::from(level)).sum::<f64>() / 255.0;
                let rim = 2.0 * std::f64::consts::PI * f64::from(radius + half);
                let bound = rim * f64::from(FOLD_FLATNESS / scale);
                assert!(
                    (union - set).abs() <= bound,
                    "radius {radius} at scale {scale}: the pieces' union is {union:.4} against \
                     {set:.4}, past {bound:.4}"
                );
            }
        }
    }

    /// The witness of `tests/folding_stroke_under_a_clip.rs` held to its flattened set, where the
    /// pieces can be reached: veraPDF's paperclip, `0.59 w` with round joins, at that page's
    /// matrix at 10x, and its pieces drawn sixteen times finer over the eight pixels square about
    /// the witness pixel `(798, 567)`, each pixel within the two drawings' roundings. That test
    /// holds the page to the curve, whose bound the library's converter passes; this one is red
    /// on it, `(796, 563)` reading 128 against the finer drawing's 130.65 (ADR 2269).
    #[test]
    fn the_paperclip_s_pieces_are_measured_as_their_flattened_set() {
        const LADDER: u32 = 16;
        const SIDE: u32 = 8;
        const WINDOW: (u32, u32) = (794, 563);
        // The paperclip test's `PAPERCLIP`, in the appearance stream's own space.
        const PAPERCLIP: &str = "0.51 13.63 m 0.51 13.25 0.48 4.38 0.48 3.74 c 0.48 3.29 0.49 \
            1.93 1.38 1.05 c 1.89 0.55 2.59 0.31 3.45 0.32 c 5.46 0.36 6.60 1.61 6.57 3.76 c 6.56 \
            4.66 6.57 10.39 6.57 10.45 c 6.57 10.70 6.36 10.90 6.11 10.90 c 5.86 10.90 5.65 10.70 \
            5.65 10.44 c 5.65 10.21 5.64 4.66 5.65 3.75 c 5.67 2.09 4.95 1.27 3.44 1.24 c 2.83 \
            1.23 2.35 1.39 2.03 1.71 c 1.40 2.32 1.40 3.39 1.40 3.75 c 1.40 4.37 1.43 13.24 1.43 \
            13.63 c 1.43 13.97 1.52 15.65 3.03 15.65 c 3.91 15.65 4.29 15.09 4.29 13.77 c 4.29 \
            13.63 l 4.30 13.30 4.28 9.30 4.27 7.24 c 4.27 7.23 4.27 7.22 4.27 7.21 c 4.28 7.00 \
            4.25 6.32 3.96 6.03 c 3.87 5.94 3.76 5.89 3.60 5.90 c 2.85 5.91 2.86 7.23 2.86 7.24 c \
            2.84 10.81 l 2.83 11.07 2.63 11.27 2.37 11.27 c 2.12 11.27 1.92 11.06 1.92 10.81 c \
            1.94 7.24 l 1.93 6.47 2.26 5.00 3.59 4.98 c 3.99 4.97 4.35 5.12 4.62 5.40 c 5.21 6.01 \
            5.20 7.06 5.19 7.24 c 5.19 7.50 5.22 13.24 5.20 13.66 c 5.20 13.77 l 5.21 14.92 4.92 \
            15.61 4.51 16.03 c 4.08 16.45 3.53 16.57 3.03 16.57 c 1.05 16.57 0.52 14.72 0.51 \
            13.63 c h";
        let mut builder = tiny_skia::PathBuilder::new();
        let mut operands: Vec<f32> = Vec::new();
        for word in PAPERCLIP.split_whitespace() {
            match word {
                "m" => builder.move_to(operands[0], operands[1]),
                "l" => builder.line_to(operands[0], operands[1]),
                "c" => builder.cubic_to(
                    operands[0],
                    operands[1],
                    operands[2],
                    operands[3],
                    operands[4],
                    operands[5],
                ),
                "h" => builder.close(),
                number => {
                    operands.push(number.parse().expect("an operand"));
                    continue;
                }
            }
            operands.clear();
        }
        let paperclip = builder.finish().expect("a paperclip");
        let style = tiny_skia::Stroke {
            width: 0.59,
            line_join: tiny_skia::LineJoin::Round,
            ..tiny_skia::Stroke::default()
        };
        let pieces = folding_stroke_outline(&paperclip, &style, 10.0).expect("it folds");
        let drawn = |(width, height): (u32, u32), at: tiny_skia::Transform| {
            let mut mask = tiny_skia::Mask::new(width, height).expect("a mask");
            crate::scan::mask_fill(
                &mut mask,
                &mut crate::area::Buffers::default(),
                &pieces,
                tiny_skia::FillRule::Winding,
                true,
                (crate::scan::Place::surface(at), &crate::scan::Exact::Pieces),
            );
            mask
        };
        // The paperclip test's `AT_TEN`, to the bit.
        let (e, f) = (750.341_06_f32, 717.119_6_f32);
        let page = drawn(
            (830, 720),
            tiny_skia::Transform::from_row(10.0, 0.0, 0.0, -10.0, e, f),
        );
        #[expect(
            clippy::cast_precision_loss,
            reason = "a ladder of 16 and a window corner under 830, exact in f32"
        )]
        let (ladder, corner) = (LADDER as f32, (WINDOW.0 as f32, WINDOW.1 as f32));
        let finer = tiny_skia::Transform::from_row(
            10.0 * ladder,
            0.0,
            0.0,
            -10.0 * ladder,
            (e - corner.0) * ladder,
            (f - corner.1) * ladder,
        );
        let fine = drawn((SIDE * LADDER, SIDE * LADDER), finer);
        for y in 0..SIDE {
            for x in 0..SIDE {
                let mut sum = 0_u32;
                for v in 0..LADDER {
                    for u in 0..LADDER {
                        let at = (y * LADDER + v) * SIDE * LADDER + x * LADDER + u;
                        sum += u32::from(fine.data()[at as usize]);
                    }
                }
                let area = f64::from(sum) / f64::from(LADDER * LADDER);
                let level = page.data()[((WINDOW.1 + y) * 830 + WINDOW.0 + x) as usize];
                assert!(
                    (f64::from(level) - area).abs() <= 1.0,
                    "pixel {:?} reads {level} where the same pieces drawn {LADDER} times finer \
                     cover {area:.2} of 255",
                    (WINDOW.0 + x, WINDOW.1 + y)
                );
            }
        }
    }

    /// ISO 32000-2 §10.7.4 measures a shape once "curves have been flattened to sequences of
    /// straight lines", so a pixel is owed the area of the flattened set over its square, and the
    /// measure `scan::mask_fill` takes of the pieces (ADR 2122) is held to it: the same pieces,
    /// flattened for the page's scale, drawn sixteen times finer and averaged over the block that
    /// becomes each pixel, within the two drawings' roundings, half a level apiece (ADR 2252).
    ///
    /// The disk of radius 5 at scale 10, a radius-1 circle stroked at `8 w`, whose every chord
    /// folds; one window on its rim where the boundary runs at a slant and one where it runs
    /// level, each eight pixels square and holding pixels the rim crosses.
    #[test]
    fn a_folding_stroke_s_pieces_are_measured_as_their_flattened_set() {
        const SCALE: f32 = 10.0;
        const LADDER: u32 = 16;
        const SIDE: u32 = 8;
        let (cx, cy, r, k) = (20.0_f32, 20.0_f32, 1.0_f32, 0.5523_f32);
        let mut builder = tiny_skia::PathBuilder::new();
        builder.move_to(cx + r, cy);
        builder.cubic_to(cx + r, cy + k, cx + k, cy + r, cx, cy + r);
        builder.cubic_to(cx - k, cy + r, cx - r, cy + k, cx - r, cy);
        builder.cubic_to(cx - r, cy - k, cx - k, cy - r, cx, cy - r);
        builder.cubic_to(cx + k, cy - r, cx + r, cy - k, cx + r, cy);
        builder.close();
        let circle = builder.finish().expect("a circle");
        let style = tiny_skia::Stroke {
            width: 8.0,
            line_join: tiny_skia::LineJoin::Round,
            ..tiny_skia::Stroke::default()
        };
        let pieces = folding_stroke_outline(&circle, &style, SCALE).expect("it folds");
        let drawn = |(width, height): (u32, u32), at: tiny_skia::Transform| {
            let mut mask = tiny_skia::Mask::new(width, height).expect("a mask");
            crate::scan::mask_fill(
                &mut mask,
                &mut crate::area::Buffers::default(),
                &pieces,
                tiny_skia::FillRule::Winding,
                true,
                (crate::scan::Place::surface(at), &crate::scan::Exact::Pieces),
            );
            mask
        };
        let page = drawn((400, 400), tiny_skia::Transform::from_scale(SCALE, SCALE));
        for corner in [(231_u32, 231_u32), (196, 146)] {
            #[expect(
                clippy::cast_precision_loss,
                reason = "a window corner under 400, exact in f32"
            )]
            let finer = tiny_skia::Transform::from_row(
                SCALE * LADDER as f32,
                0.0,
                0.0,
                SCALE * LADDER as f32,
                -((corner.0 * LADDER) as f32),
                -((corner.1 * LADDER) as f32),
            );
            let ladder = drawn((SIDE * LADDER, SIDE * LADDER), finer);
            let mut rim = 0;
            for y in 0..SIDE {
                for x in 0..SIDE {
                    let mut sum = 0_u32;
                    for v in 0..LADDER {
                        for u in 0..LADDER {
                            let at = (y * LADDER + v) * SIDE * LADDER + x * LADDER + u;
                            sum += u32::from(ladder.data()[at as usize]);
                        }
                    }
                    let area = f64::from(sum) / f64::from(LADDER * LADDER);
                    let level = page.data()[((corner.1 + y) * 400 + corner.0 + x) as usize];
                    if level > 0 && level < u8::MAX {
                        rim += 1;
                    }
                    assert!(
                        (f64::from(level) - area).abs() <= 1.0,
                        "pixel {:?} reads {level} where the same pieces drawn {LADDER} times \
                         finer cover {area:.2} of 255",
                        (corner.0 + x, corner.1 + y)
                    );
                }
            }
            assert!(
                rim > 0,
                "the window at {corner:?} holds no pixel the rim crosses"
            );
        }
    }

    /// ISO 32000-2 §8.4.3.5 decides a mitre by the angle φ "between the segments", and at a vertex
    /// the path states that is the angle between its own tangents. A line arriving straight down
    /// at `(8, 12)` and a curve leaving it straight up — the collar of the push pin in
    /// `annotation-fileattachment.pdf`, `0.6 w`, mitred under the default limit of 10 — meet at
    /// `φ = 0`, past every limit, so the join is a bevel and nothing of the stroke lies further
    /// below the vertex than the line's band, `h` and the bands' overlap. The curve's first chord
    /// leans off that tangent, and decided from the chord the join was a mitre reaching a unit down
    /// (ADR 2252).
    #[test]
    fn a_mitre_is_decided_by_the_path_s_tangents_and_not_by_its_chords() {
        let mut builder = tiny_skia::PathBuilder::new();
        builder.move_to(8.0, 18.48);
        builder.line_to(8.0, 12.0);
        builder.cubic_to(8.0, 12.8, 12.0, 12.8, 12.0, 12.0);
        builder.line_to(12.0, 18.48);
        let collar = builder.finish().expect("a collar");
        let style = tiny_skia::Stroke {
            width: 0.6,
            line_join: tiny_skia::LineJoin::Miter,
            miter_limit: 10.0,
            ..tiny_skia::Stroke::default()
        };
        for scale in [1.0_f32, 4.0, 10.0] {
            let pieces = folding_stroke_outline(&collar, &style, scale).expect("it folds");
            let lowest = pieces
                .points()
                .iter()
                .map(|point| point.y)
                .fold(f32::INFINITY, f32::min);
            assert!(
                lowest >= 12.0 - 0.3 - FOLD_FLATNESS / scale,
                "at scale {scale} a piece reaches down to {lowest}, below the bevel at 11.7"
            );
        }
    }

    /// ISO 32000-2 §8.4.3.3's projecting square cap: "The stroke shall continue beyond the
    /// endpoint of the path for a distance equal to half the line width and shall be squared
    /// off", so it is square to the path's own tangent at its end: an arch leaving `(10, 10)`
    /// straight up and arriving at `(14, 10)` straight down, `0.6 w`, is capped down to `y = 9.7`
    /// and no further at either end. Its chords lean off those tangents, and a cap turned from
    /// them reached below that.
    #[test]
    fn a_cap_is_square_to_the_path_s_tangent_and_not_to_its_chord() {
        let mut builder = tiny_skia::PathBuilder::new();
        builder.move_to(10.0, 10.0);
        builder.cubic_to(10.0, 10.8, 14.0, 10.8, 14.0, 10.0);
        let arch = builder.finish().expect("an arch");
        let style = tiny_skia::Stroke {
            width: 0.6,
            line_cap: tiny_skia::LineCap::Square,
            ..tiny_skia::Stroke::default()
        };
        for scale in [1.0_f32, 4.0, 10.0] {
            let pieces = folding_stroke_outline(&arch, &style, scale).expect("it folds");
            let lowest = pieces
                .points()
                .iter()
                .map(|point| point.y)
                .fold(f32::INFINITY, f32::min);
            assert!(
                lowest >= 10.0 - 0.3 - FOLD_FLATNESS / scale,
                "at scale {scale} a cap reaches down to {lowest}, below the square cap at 9.7"
            );
        }
    }

    /// §8.4.3.2's stroke holds every point within the half-width of the path, so the disk about a
    /// vertex is in it whatever the join. `issue14438.pdf`'s ink, `2 w` with round joins, turns a
    /// corner at `(399.299, 518.497)` and states the corner twice more as curves of no length: its
    /// chords were each a rounding long and pointed anywhere, the joins at either end of them were
    /// turned from those directions, and the outer side of the corner was left out at scales 1, 4
    /// and 64 (ADR 2269). The segments are the file's, as the stroke reaches `stroke::folding`.
    #[test]
    fn a_corner_stated_again_as_curves_of_no_length_keeps_its_join() {
        let mut builder = tiny_skia::PathBuilder::new();
        builder.move_to(400.749, 519.946);
        builder.cubic_to(400.507, 519.705, 400.507, 519.705, 400.265, 519.463);
        builder.cubic_to(399.782, 518.98, 399.782, 518.98, 399.299, 518.497);
        builder.cubic_to(399.299, 518.497, 399.299, 518.497, 399.299, 518.497);
        builder.cubic_to(399.299, 518.497, 399.299, 518.497, 399.299, 518.497);
        builder.cubic_to(399.299, 518.255, 399.299, 518.255, 399.299, 518.013);
        builder.cubic_to(399.299, 517.772, 399.299, 517.772, 399.299, 517.53);
        let ink = builder.finish().expect("a corner");
        let style = tiny_skia::Stroke {
            width: 2.0,
            line_cap: tiny_skia::LineCap::Round,
            line_join: tiny_skia::LineJoin::Round,
            ..tiny_skia::Stroke::default()
        };
        for scale in [1.0_f32, 4.0, 16.0, 64.0] {
            let pieces = folding_stroke_outline(&ink, &style, scale).expect("it folds");
            let missing = left_out(
                &pieces,
                (399.299, 518.497),
                (&[0.3, 0.6, 0.9, 0.98], 0..=355),
            );
            assert!(missing.is_empty(), "at scale {scale}: {missing:?}");
        }
    }

    /// Where a curve doubles back the round join between its chords turns through nearly a half
    /// turn, and its arc belongs beyond the vertex, through the way the path arrives. `inks.pdf`'s
    /// red ink, `4.5 w`, doubles back inside a curve near `(198.616, 426.803)`; its arc was turned
    /// by the sign of the cross product of two nearly opposite normals, a rounding, and at some
    /// scales — 16 among those below — went behind the vertex and left a notch in the end of the
    /// ink (ADR 2269).
    #[test]
    fn a_curve_that_doubles_back_is_rounded_beyond_the_turn() {
        let mut builder = tiny_skia::PathBuilder::new();
        builder.move_to(197.849_84, 426.816_04);
        builder.cubic_to(
            198.0122, 426.813_54, 198.131, 426.811_34, 198.238_72, 426.809_54,
        );
        builder.cubic_to(
            198.346_44, 426.8077, 198.433_67, 426.8063, 198.496_17, 426.8051,
        );
        builder.cubic_to(
            198.558_65, 426.803_96, 198.6186, 426.803_16, 198.613_75, 426.8025,
        );
        builder.cubic_to(
            198.6089, 426.801_85, 198.5731, 426.8015, 198.4671, 426.801_24,
        );
        builder.cubic_to(
            198.361_08, 426.800_93, 198.194_78, 426.8009, 197.977_69, 426.800_84,
        );
        let ink = builder.finish().expect("an ink that doubles back");
        let style = tiny_skia::Stroke {
            width: 4.5,
            line_cap: tiny_skia::LineCap::Round,
            line_join: tiny_skia::LineJoin::Round,
            ..tiny_skia::Stroke::default()
        };
        for scale in [1.0_f32, 4.0, 16.0, 64.0] {
            let pieces = folding_stroke_outline(&ink, &style, scale).expect("it folds");
            let missing = left_out(&pieces, (198.6, 426.803), (&[1.0, 2.0], -60..=60));
            assert!(missing.is_empty(), "at scale {scale}: {missing:?}");
        }
    }

    /// A piece is wound positively however far from the origin it lies: a triangle of a fifty-
    /// thousandth of a square unit at `(10 000, 10 000)`, whose area about the origin is a sum of
    /// products each rounded by more than a unit in `f32`, comes out wound positively both ways it
    /// is stated (ADR 2269).
    #[test]
    fn a_small_piece_far_from_the_origin_is_wound_positively() {
        let corners = [
            (10_000.0, 10_000.0),
            (10_000.01, 10_000.0),
            (10_000.0, 10_000.01),
        ];
        for order in [[0, 1, 2], [0, 2, 1], [1, 2, 0], [2, 1, 0]] {
            let stated: Vec<tiny_skia::Point> = order
                .iter()
                .map(|&at| tiny_skia::Point::from_xy(corners[at].0, corners[at].1))
                .collect();
            let mut out = tiny_skia::PathBuilder::new();
            push_convex(&mut out, &stated);
            let piece = out.finish().expect("a triangle");
            let points: Vec<(f64, f64)> = piece
                .points()
                .iter()
                .map(|point| (f64::from(point.x) - 10_000.0, f64::from(point.y) - 10_000.0))
                .collect();
            let twice: f64 = points
                .iter()
                .zip(points.iter().cycle().skip(1))
                .map(|(one, next)| one.0 * next.1 - one.1 * next.0)
                .sum();
            assert!(twice > 0.0, "{order:?} is wound with area {}", twice / 2.0);
        }
    }

    /// A stretch's outline joins the pieces wound the way every piece is, whichever way the
    /// stroker wound it (ADR 1359): a square traced clockwise and anticlockwise comes out with the
    /// same positive area. The stroker has wound every outline the fixtures draw positively, so
    /// this is the one place the reversal is exercised.
    #[test]
    fn a_stretch_is_wound_the_way_the_pieces_are() {
        for corners in [
            [(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)],
            [(0.0, 0.0), (0.0, 10.0), (10.0, 10.0), (10.0, 0.0)],
        ] {
            let mut builder = tiny_skia::PathBuilder::new();
            builder.move_to(corners[0].0, corners[0].1);
            for (x, y) in &corners[1..] {
                builder.line_to(*x, *y);
            }
            builder.close();
            let square = builder.finish().expect("a square");
            let mut out = tiny_skia::PathBuilder::new();
            push_wound_positively(&mut out, &square, 1.0 / 64.0);
            let wound = out.finish().expect("a square");
            let points: Vec<tiny_skia::Point> = wound.points().to_vec();
            let twice: f32 = points
                .iter()
                .zip(points.iter().cycle().skip(1))
                .map(|(one, next)| one.cross(*next))
                .sum();
            assert!(
                twice > 199.0,
                "{corners:?} is wound with area {}",
                twice / 2.0
            );
        }
    }
}

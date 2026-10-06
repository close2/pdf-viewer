//! The image of a stroke's set under a matrix of rank one: ISO 32000-2 §8.4.3.2 and §10.7.4.
//!
//! [`super::collapsed_stroke_by_transform`] argues the construction; this module is its geometry.
//! A stroke is a union of convex parts — each chord's band, each join, each cap, cut into dashes
//! by §8.4.3.6 — and a linear functional's extremes over a convex part are at its corners or,
//! for a disk sector, at the radius pointing along the functional. Each connected piece's image
//! is therefore one interval, and the stroke's image their union.

use super::ImageLine;
use crate::geom::{Path, PathCommand, Point};
use crate::paint::{LineCap, LineJoin, Stroke};

/// How many dashes one stroke may be cut into before its image is refused rather than computed —
/// §8.4.3.6's pattern is the document's, and a pattern a millionth of a unit long along a page is
/// a budget question before it is a geometry one (principle 3).
const MOST_DASHES: usize = 1 << 16;

/// How many chords one curve is flattened into at most.
const MOST_CHORDS: usize = 1 << 10;

/// How far, in page units along the image line, a flattened curve may sit from its curve: a
/// 256th, which is ADR 1348's figure for the arcs of round joins and a fraction of a pixel at every
/// zoom this viewer offers.
const FLATNESS: f64 = 1.0 / 256.0;

/// A vertex of a flattened subpath.
#[derive(Debug, Clone, Copy)]
struct Vertex {
    x: f64,
    y: f64,
    /// Whether the document stated this vertex, so §8.4.3.4's join applies there; a vertex the
    /// flattening made is where the curve's own set continues, a round join's sector (ADR 1348).
    stated: bool,
}

/// One connected piece of a stroke: a subpath, or a dash of one.
#[derive(Debug, Clone, Default)]
struct Piece {
    vertices: Vec<Vertex>,
    /// Whether the piece closes on itself, so that it has a join where an open one has two caps.
    closed: bool,
    /// The direction a piece of no length carries — a zero-length dash, whose caps §8.5.3.2
    /// orients by the path.
    direction: Option<(f64, f64)>,
}

/// The interval each connected piece of a stroke occupies on an [`ImageLine`].
#[derive(Debug)]
pub(super) struct StrokeImage<'a> {
    line: ImageLine,
    stroke: &'a Stroke,
    /// Half the line width, in user space.
    half: f64,
    /// `|functional|`, what a whole disk of radius one reaches along the line.
    reach: f64,
    /// The flattening tolerance in user space.
    tolerance: f64,
}

impl<'a> StrokeImage<'a> {
    pub(super) fn new(line: ImageLine, stroke: &'a Stroke) -> Option<Self> {
        let half = f64::from(stroke.width) / 2.0;
        let reach = line.functional.0.hypot(line.functional.1);
        if !half.is_finite() || half < 0.0 || !reach.is_finite() {
            return None;
        }
        let tolerance = if reach > 0.0 {
            FLATNESS / reach
        } else {
            f64::INFINITY
        };
        Some(Self {
            line,
            stroke,
            half,
            reach,
            tolerance,
        })
    }

    /// The merged intervals of every piece of `path`'s stroke, ascending; `None` over budget or
    /// where a value is not finite.
    pub(super) fn of(&self, path: &Path) -> Option<Vec<(f64, f64)>> {
        let mut intervals = Vec::new();
        let mut dashes = 0usize;
        for subpath in self.subpaths(path)? {
            match subpath {
                Subpath::Dot(at) => {
                    // §8.5.3.2: a degenerate subpath is painted only under round caps, as "a
                    // filled circle centred at the single point".
                    if self.stroke.cap == LineCap::Round {
                        let s = self.line.dot(at.0, at.1);
                        intervals.push((s - self.half * self.reach, s + self.half * self.reach));
                    }
                }
                Subpath::Line(piece) => {
                    for dash in self.dashed(piece, &mut dashes)? {
                        // A butt-capped dash of no length marks nothing.
                        let (low, high) = self.interval(&dash);
                        if low <= high {
                            intervals.push((low, high));
                        }
                    }
                }
            }
        }
        if intervals
            .iter()
            .any(|(low, high)| !low.is_finite() || !high.is_finite())
        {
            return None;
        }
        intervals.sort_by(|one, other| one.0.total_cmp(&other.0));
        let mut merged: Vec<(f64, f64)> = Vec::with_capacity(intervals.len());
        for (low, high) in intervals {
            match merged.last_mut() {
                Some(last) if low <= last.1 => last.1 = last.1.max(high),
                _ => merged.push((low, high)),
            }
        }
        Some(merged)
    }

    /// The path's subpaths, flattened, each a polyline or §8.5.3.2's point. A lone `m` is dropped:
    /// "A single-point open subpath (specified by a trailing m operator) shall produce no output."
    fn subpaths(&self, path: &Path) -> Option<Vec<Subpath>> {
        let mut out = Vec::new();
        let mut current: Option<Piece> = None;
        let mut segments = false;
        let mut start = (0.0, 0.0);
        let mut at = (0.0, 0.0);
        let finish = |piece: Option<Piece>, segments: bool, out: &mut Vec<Subpath>| {
            if let Some(piece) = piece
                && segments
            {
                out.push(if piece.vertices.len() < 2 {
                    Subpath::Dot(
                        piece
                            .vertices
                            .first()
                            .map_or((0.0, 0.0), |vertex| (vertex.x, vertex.y)),
                    )
                } else {
                    Subpath::Line(piece)
                });
            }
        };
        for command in path.commands() {
            match *command {
                PathCommand::MoveTo(point) => {
                    finish(current.take(), segments, &mut out);
                    segments = false;
                    at = pair(point);
                    start = at;
                    current = Some(Piece {
                        vertices: vec![vertex(at, true)],
                        ..Piece::default()
                    });
                }
                PathCommand::LineTo(point) => {
                    let piece = current.get_or_insert_with(|| Piece {
                        vertices: vec![vertex(at, true)],
                        ..Piece::default()
                    });
                    segments = true;
                    at = pair(point);
                    push_distinct(&mut piece.vertices, vertex(at, true));
                }
                PathCommand::CurveTo(one, two, to) => {
                    let piece = current.get_or_insert_with(|| Piece {
                        vertices: vec![vertex(at, true)],
                        ..Piece::default()
                    });
                    segments = true;
                    let control = [at, pair(one), pair(two), pair(to)];
                    self.flatten(control, &mut piece.vertices)?;
                    at = control[3];
                }
                PathCommand::Close => {
                    if let Some(mut piece) = current.take() {
                        piece.closed = true;
                        if piece.vertices.len() > 1 {
                            push_distinct(&mut piece.vertices, vertex(start, true));
                            if piece
                                .vertices
                                .last()
                                .is_some_and(|last| (last.x, last.y) == start)
                            {
                                piece.vertices.pop();
                            }
                        }
                        finish(Some(piece), true, &mut out);
                    }
                    segments = false;
                    at = start;
                }
            }
        }
        finish(current, segments, &mut out);
        Some(out)
    }

    /// Appends a cubic's chords after its first point, each interior vertex made by flattening.
    fn flatten(&self, control: [(f64, f64); 4], into: &mut Vec<Vertex>) -> Option<()> {
        let [p0, p1, p2, p3] = control;
        // The second differences bound |B″| / 6, and a chord of a parameter step 1/n lies within
        // |B″|max / (8 n²) of its arc.
        let second = |a: (f64, f64), b: (f64, f64), c: (f64, f64)| {
            (a.0 - 2.0 * b.0 + c.0).hypot(a.1 - 2.0 * b.1 + c.1)
        };
        let bend = second(p0, p1, p2).max(second(p1, p2, p3));
        let wanted = (6.0 * bend / (8.0 * self.tolerance)).sqrt().ceil();
        if !wanted.is_finite() && self.tolerance.is_finite() {
            return None;
        }
        let chords = if wanted.is_finite() && wanted >= 1.0 {
            #[expect(
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                reason = "a positive whole number, compared against a bound of 1024 before use"
            )]
            let wanted = wanted.min(1e9) as usize;
            if wanted > MOST_CHORDS {
                return None;
            }
            wanted
        } else {
            1
        };
        for step in 1..=chords {
            #[expect(
                clippy::cast_precision_loss,
                reason = "at most 1024 chords, exact in f64"
            )]
            let t = step as f64 / chords as f64;
            let u = 1.0 - t;
            let point = (
                u * u * u * p0.0
                    + 3.0 * u * u * t * p1.0
                    + 3.0 * u * t * t * p2.0
                    + t * t * t * p3.0,
                u * u * u * p0.1
                    + 3.0 * u * u * t * p1.1
                    + 3.0 * u * t * t * p2.1
                    + t * t * t * p3.1,
            );
            push_distinct(into, vertex(point, step == chords));
        }
        Some(())
    }

    /// The piece cut by §8.4.3.6's dash pattern, or the piece whole where there is none.
    fn dashed(&self, piece: Piece, dashes: &mut usize) -> Option<Vec<Piece>> {
        let pattern = &self.stroke.dash_array;
        if pattern.is_empty() {
            return Some(vec![piece]);
        }
        let lengths: Vec<f64> = pattern.iter().map(|length| f64::from(*length)).collect();
        let period: f64 = lengths.iter().sum();
        if !period.is_finite() || period <= 0.0 {
            return Some(vec![piece]);
        }
        // §8.4.3.6: each subpath starts the pattern afresh at the phase.
        let (mut index, mut left) = pattern_at(
            &lengths,
            f64::from(self.stroke.dash_phase).rem_euclid(period),
        )?;
        let on = |index: usize| index.is_multiple_of(2);
        let mut vertices = piece.vertices;
        if piece.closed
            && let Some(first) = vertices.first().copied()
        {
            vertices.push(first);
        }
        let mut out: Vec<Piece> = Vec::new();
        let mut current: Option<Piece> = on(index).then(|| Piece {
            vertices: vertices.first().copied().into_iter().collect(),
            ..Piece::default()
        });
        let began_on = current.is_some();
        for pair in vertices.windows(2) {
            let (from, to) = (pair[0], pair[1]);
            let (dx, dy) = (to.x - from.x, to.y - from.y);
            let length = dx.hypot(dy);
            let direction = (dx / length, dy / length);
            let mut travelled = 0.0;
            loop {
                *dashes = dashes.saturating_add(1);
                if *dashes > MOST_DASHES {
                    return None;
                }
                if left > length - travelled {
                    // The entry outlasts the chord: whatever is on continues through `to`.
                    left -= length - travelled;
                    if let Some(dash) = current.as_mut() {
                        push_distinct(&mut dash.vertices, to);
                        dash.direction.get_or_insert(direction);
                    }
                    break;
                }
                // The entry ends on this chord, and the pattern turns over there.
                travelled += left;
                let here = if travelled >= length {
                    Vertex {
                        stated: false,
                        ..to
                    }
                } else {
                    vertex(
                        (
                            from.x + direction.0 * travelled,
                            from.y + direction.1 * travelled,
                        ),
                        false,
                    )
                };
                if let Some(mut dash) = current.take() {
                    push_distinct(&mut dash.vertices, here);
                    dash.direction.get_or_insert(direction);
                    out.push(dash);
                }
                index = next_entry(index, lengths.len());
                left = *lengths.get(index)?;
                if on(index) {
                    current = Some(Piece {
                        vertices: vec![here],
                        direction: Some(direction),
                        ..Piece::default()
                    });
                }
            }
        }
        let ended_on = current.is_some();
        if let Some(dash) = current.take() {
            // A dash the path's end cuts to no length is not §8.5.3.2's zero-length dash, which is
            // an entry of the pattern that is zero: it marks nothing unless it continues, closed,
            // into the first.
            let cut_to_nothing =
                dash.vertices.len() < 2 && lengths.get(index).is_some_and(|entry| *entry > 0.0);
            if !cut_to_nothing || (piece.closed && began_on) {
                out.push(dash);
            }
        }
        if piece.closed && began_on && ended_on {
            join_at_the_start(&mut out);
        }
        Some(out)
    }

    /// The least and greatest `s` over one connected piece: its chords' bands, its joins, and its
    /// caps or, closed, the join where it meets itself.
    fn interval(&self, piece: &Piece) -> (f64, f64) {
        let mut low = f64::INFINITY;
        let mut high = f64::NEG_INFINITY;
        let mut take = |s: f64| {
            low = low.min(s);
            high = high.max(s);
        };
        let vertices = &piece.vertices;
        let normal = |d: (f64, f64)| (-d.1, d.0);
        let direction_of = |from: Vertex, to: Vertex| {
            let (dx, dy) = (to.x - from.x, to.y - from.y);
            let length = dx.hypot(dy);
            (dx / length, dy / length)
        };
        let at = |v: Vertex| self.line.dot(v.x, v.y);
        let across = |n: (f64, f64)| self.half * self.line.dot(n.0, n.1).abs();
        if vertices.len() < 2 {
            // A dash of no length: §8.5.3.2 paints its caps, oriented by the path.
            if let (Some(centre), Some(d)) = (vertices.first(), piece.direction) {
                let s = at(*centre);
                match self.stroke.cap {
                    LineCap::Butt => {}
                    LineCap::Round => {
                        take(s - self.half * self.reach);
                        take(s + self.half * self.reach);
                    }
                    LineCap::Square => {
                        let n = normal(d);
                        let spread = across(n) + self.half * self.line.dot(d.0, d.1).abs();
                        take(s - spread);
                        take(s + spread);
                    }
                }
            }
            return (low, high);
        }
        // Each chord's band: its ends, a half-width either side along its normal.
        let count = vertices.len();
        let chords = if piece.closed {
            count
        } else {
            count.saturating_sub(1)
        };
        // Chord `index` runs from vertex `index` to the next, the last of a closed piece back to
        // the first; `count` is at least two here.
        let chord = |index: usize| {
            let from = vertices.get(index).copied().unwrap_or(vertices[0]);
            let to = vertices
                .get(next_entry(index, count))
                .copied()
                .unwrap_or(vertices[0]);
            (from, to, direction_of(from, to))
        };
        for index in 0..chords {
            let (from, to, d) = chord(index);
            let spread = across(normal(d));
            for s in [at(from), at(to)] {
                take(s - spread);
                take(s + spread);
            }
        }
        // Joins at every vertex between two chords.
        let joins = if piece.closed {
            0..count
        } else {
            1..count.saturating_sub(1)
        };
        for index in joins {
            let before = index.checked_sub(1).unwrap_or(count.saturating_sub(1));
            let (_, _, into) = chord(before);
            let (vertex, _, out) = chord(index);
            self.join(vertex, into, out, &mut take);
        }
        if !piece.closed {
            let (first, _, d) = chord(0);
            self.cap(first, (-d.0, -d.1), &mut take);
            let (_, last, d) = chord(chords.saturating_sub(1));
            self.cap(last, d, &mut take);
        }
        (low, high)
    }

    /// What §8.4.3.4's join at `vertex` adds, between a chord arriving along `into` and one
    /// leaving along `out`. A bevel adds nothing its two chords' bands do not already reach.
    fn join(&self, vertex: Vertex, into: (f64, f64), out: (f64, f64), take: &mut impl FnMut(f64)) {
        let turn = into.0 * out.1 - into.1 * out.0;
        if turn == 0.0 {
            // Straight on, no join; straight back, a round join is the half disk ahead and a
            // mitre's ratio is unbounded, so a bevel.
            let reversed = into.0 * out.0 + into.1 * out.1 < 0.0;
            if reversed && (!vertex.stated || self.stroke.join == LineJoin::Round) {
                self.cap_round(self.line.dot(vertex.x, vertex.y), into, take);
            }
            return;
        }
        // The outer side is the one the path turns away from.
        let side = if turn > 0.0 { -1.0 } else { 1.0 };
        let outer = |d: (f64, f64)| (-d.1 * side, d.0 * side);
        let (one, two) = (outer(into), outer(out));
        let s = self.line.dot(vertex.x, vertex.y);
        let join = if vertex.stated {
            self.stroke.join
        } else {
            LineJoin::Round
        };
        match join {
            LineJoin::Bevel => {}
            LineJoin::Round => {
                self.sector(s, one, two, take);
            }
            LineJoin::Miter => {
                // §8.4.3.5: miterLength / lineWidth = 1 / sin(φ/2), and sin(φ/2) is
                // sqrt((1 + into · out) / 2) for the angle φ between the two segments.
                let cosine = into.0 * out.0 + into.1 * out.1;
                let sine_half = f64::midpoint(1.0, cosine).max(0.0).sqrt();
                if sine_half > 0.0 && 1.0 / sine_half <= f64::from(self.stroke.miter_limit) {
                    let scale = self.half / (1.0 + (one.0 * two.0 + one.1 * two.1));
                    let tip = (
                        vertex.x + (one.0 + two.0) * scale,
                        vertex.y + (one.1 + two.1) * scale,
                    );
                    take(self.line.dot(tip.0, tip.1));
                }
            }
        }
    }

    /// What §8.4.3.3's cap at `end` adds, the path leaving it along `outward`.
    fn cap(&self, end: Vertex, outward: (f64, f64), take: &mut impl FnMut(f64)) {
        let s = self.line.dot(end.x, end.y);
        let n = (-outward.1, outward.0);
        match self.stroke.cap {
            LineCap::Butt => {}
            LineCap::Round => self.cap_round(s, outward, take),
            LineCap::Square => {
                let beyond = self.half * self.line.dot(outward.0, outward.1);
                let across = self.half * self.line.dot(n.0, n.1).abs();
                take(s + beyond - across);
                take(s + beyond + across);
            }
        }
    }

    /// Table 53's round cap about the point at `s`: the half disk ahead of `outward`, as two
    /// quarter sectors.
    fn cap_round(&self, s: f64, outward: (f64, f64), take: &mut impl FnMut(f64)) {
        let n = (-outward.1, outward.0);
        self.sector(s, n, outward, take);
        self.sector(s, outward, (-n.0, -n.1), take);
    }

    /// The reach along the line of a disk sector of half the width about the point at `s`,
    /// spanning the unit directions from `one` to `two` (less than half a turn): the whole
    /// radius where the line's own direction lies inside the sector, and otherwise its two
    /// radii, which are the neighbouring bands' corners and are taken there.
    fn sector(&self, s: f64, one: (f64, f64), two: (f64, f64), take: &mut impl FnMut(f64)) {
        let g = self.line.functional;
        let span = one.0 * two.1 - one.1 * two.0;
        let inside = |v: (f64, f64)| {
            let from = one.0 * v.1 - one.1 * v.0;
            let to = v.0 * two.1 - v.1 * two.0;
            let facing = v.0 * (one.0 + two.0) + v.1 * (one.1 + two.1);
            facing > 0.0 && from * span >= 0.0 && to * span >= 0.0
        };
        for (v, sign) in [(g, 1.0), ((-g.0, -g.1), -1.0)] {
            if inside(v) {
                take(s + sign * self.half * self.reach);
            }
        }
        for radius in [one, two] {
            take(s + self.half * self.line.dot(radius.0, radius.1));
        }
    }
}

/// A closed subpath's dashes, where its pattern is on where it starts and still on where it ends:
/// the last and the first are one dash through its first vertex, joined there — or, the pattern
/// never turned off, the subpath whole.
fn join_at_the_start(dashes: &mut Vec<Piece>) {
    if dashes.len() > 1 {
        let first = dashes.remove(0);
        if let Some(last) = dashes.last_mut() {
            last.vertices.extend(first.vertices.into_iter().skip(1));
        }
    } else if let Some(whole) = dashes.first_mut() {
        whole.closed = true;
        whole.vertices.pop();
    }
}

/// The entry of the pattern `lengths` in force at `phase` into it, and how much of that entry is
/// left there.
fn pattern_at(lengths: &[f64], mut phase: f64) -> Option<(usize, f64)> {
    let mut index = 0usize;
    let mut left = *lengths.first()?;
    while phase > 0.0 {
        if phase >= left {
            phase -= left;
            index = next_entry(index, lengths.len());
            left = *lengths.get(index)?;
        } else {
            left -= phase;
            phase = 0.0;
        }
    }
    Some((index, left))
}

/// The entry after `index` in a pattern of `count`, turning over at the end.
fn next_entry(index: usize, count: usize) -> usize {
    let next = index.saturating_add(1);
    if next >= count { 0 } else { next }
}

/// A subpath, flattened.
#[derive(Debug)]
enum Subpath {
    /// §8.5.3.2's degenerate subpath: every point at one place.
    Dot((f64, f64)),
    /// A polyline of at least two distinct vertices.
    Line(Piece),
}

/// A vertex at `at`.
fn vertex(at: (f64, f64), stated: bool) -> Vertex {
    Vertex {
        x: at.0,
        y: at.1,
        stated,
    }
}

/// Appends `vertex` unless it repeats the last one, which would be a chord of no direction; a
/// repeat that is stated keeps the stated flag, since the document's vertex is there.
#[expect(
    clippy::float_cmp,
    reason = "a chord of exactly no length is the one with no direction"
)]
fn push_distinct(vertices: &mut Vec<Vertex>, vertex: Vertex) {
    match vertices.last_mut() {
        Some(last) if last.x == vertex.x && last.y == vertex.y => last.stated |= vertex.stated,
        _ => vertices.push(vertex),
    }
}

/// A point in `f64`.
fn pair(point: Point) -> (f64, f64) {
    (point.x.into(), point.y.into())
}

#[cfg(test)]
mod tests {
    //! Every expected interval below is the closed form of §8.4.3.2's set under the matrix,
    //! computed by hand from the path's own coordinates and written beside the assertion.

    use super::super::collapsed_stroke_by_transform;
    use crate::geom::{Path, PathCommand, Point, Transform};
    use crate::paint::{LineCap, LineJoin, Stroke};

    /// `1 0 0 0 0 50.3 cm`: every point onto `y = 50.3`, at its own `x`.
    const ONTO_X: Transform = Transform::new(1.0, 0.0, 0.0, 0.0, 0.0, 50.3);

    /// `1 0 -1 0 0 50 cm`: every point onto `y = 50`, at `x − y`.
    const ONTO_X_MINUS_Y: Transform = Transform::new(1.0, 0.0, -1.0, 0.0, 0.0, 50.0);

    fn polyline(points: &[(f32, f32)], closed: bool) -> Path {
        let mut path = Path::new();
        for (index, (x, y)) in points.iter().enumerate() {
            let point = Point::new(*x, *y);
            path.push(if index == 0 {
                PathCommand::MoveTo(point)
            } else {
                PathCommand::LineTo(point)
            });
        }
        if closed {
            path.push(PathCommand::Close);
        }
        path
    }

    fn stroke(width: f32, cap: LineCap, join: LineJoin) -> Stroke {
        Stroke {
            width,
            cap,
            join,
            ..Stroke::default()
        }
    }

    /// The image's intervals along the page's x-axis.
    fn intervals(path: &Path, style: &Stroke, transform: Transform) -> Vec<(f32, f32)> {
        let (image, frame) =
            collapsed_stroke_by_transform(path, style, transform).expect("an image");
        assert_eq!(frame, Transform::IDENTITY, "a line along a page axis");
        image
            .commands()
            .chunks(3)
            .map(|subpath| match subpath {
                [
                    PathCommand::MoveTo(from),
                    PathCommand::LineTo(to),
                    PathCommand::Close,
                ] => {
                    assert!(
                        from.y.to_bits() == transform.f.to_bits()
                            && to.y.to_bits() == transform.f.to_bits(),
                        "on the line"
                    );
                    (from.x, to.x)
                }
                other => panic!("one flat subpath per interval, not {other:?}"),
            })
            .collect()
    }

    fn assert_close(found: &[(f32, f32)], expected: &[(f64, f64)]) {
        assert_eq!(
            found.len(),
            expected.len(),
            "{found:?} against {expected:?}"
        );
        for ((low, high), (want_low, want_high)) in found.iter().zip(expected) {
            assert!(
                (f64::from(*low) - want_low).abs() < 1e-4
                    && (f64::from(*high) - want_high).abs() < 1e-4,
                "{found:?} against {expected:?}"
            );
        }
    }

    /// A diagonal segment's band reaches `w/2 · |n_x|` past its ends, where `n` is its normal;
    /// Table 53's round cap reaches the whole half-width, the projecting cap both.
    #[test]
    fn a_segments_image_is_its_band_projected() {
        let path = polyline(&[(20.0, 30.0), (60.0, 70.0)], false);
        let half = 2.5;
        let across = half / 2f64.sqrt();
        for (cap, beyond) in [
            (LineCap::Butt, across),
            (LineCap::Round, half),
            (LineCap::Square, 2.0 * across),
        ] {
            assert_close(
                &intervals(&path, &stroke(5.0, cap, LineJoin::Miter), ONTO_X),
                &[(20.0 - beyond, 60.0 + beyond)],
            );
        }
    }

    /// A path the matrix carries onto one point still has a width there: a vertical rule under
    /// a matrix that keeps only `x` is the band's own width, `x ± w/2`.
    #[test]
    fn a_path_the_matrix_makes_a_point_keeps_its_width() {
        let path = polyline(&[(40.0, 20.0), (40.0, 80.0)], false);
        assert_close(
            &intervals(&path, &stroke(5.0, LineCap::Butt, LineJoin::Miter), ONTO_X),
            &[(37.5, 42.5)],
        );
        // With no width it is a point, which §8.5.3.3.1's departure covers.
        assert_eq!(
            collapsed_stroke_by_transform(
                &path,
                &stroke(0.0, LineCap::Butt, LineJoin::Miter),
                ONTO_X
            ),
            None
        );
    }

    /// §8.4.3.4 and §8.4.3.5 at a right angle, `(10,10) → (50,10) → (50,50)` at `4 w`, seen
    /// along `x − y`: the bands reach 42, a round join `40 + 2√2`, a mitre its tip `(52, 8)` = 44
    /// — admitted at a limit of 1.5, since `1/sin(45°)` is 1.41421, and a bevel at 1.4.
    #[test]
    fn a_join_reaches_what_its_own_shape_reaches() {
        let path = polyline(&[(10.0, 10.0), (50.0, 10.0), (50.0, 50.0)], false);
        let low = 10.0 - 12.0;
        for (join, limit, high) in [
            (LineJoin::Bevel, 10.0, 42.0),
            (LineJoin::Round, 10.0, 40.0 + 2.0 * 2f64.sqrt()),
            (LineJoin::Miter, 1.5, 44.0),
            (LineJoin::Miter, 1.4, 42.0),
        ] {
            let style = Stroke {
                miter_limit: limit,
                ..stroke(4.0, LineCap::Butt, join)
            };
            assert_close(&intervals(&path, &style, ONTO_X_MINUS_Y), &[(low, high)]);
        }
    }

    /// §8.4.3.6: `[10 10] 0 d` along `x` from 10 to 70 is three dashes, each its own interval;
    /// a phase of 5 moves them back by five, and the first is cut to five.
    #[test]
    fn a_dash_pattern_is_the_union_of_its_dashes() {
        let path = polyline(&[(10.0, 30.0), (70.0, 30.0)], false);
        let mut style = stroke(2.0, LineCap::Butt, LineJoin::Miter);
        style.dash_array = vec![10.0, 10.0];
        assert_close(
            &intervals(&path, &style, ONTO_X),
            &[(10.0, 20.0), (30.0, 40.0), (50.0, 60.0)],
        );
        style.dash_phase = 5.0;
        assert_close(
            &intervals(&path, &style, ONTO_X),
            &[(10.0, 15.0), (25.0, 35.0), (45.0, 55.0), (65.0, 70.0)],
        );
        // Round caps join them all: each dash reaches a half-width past its ends. The last dash
        // would begin at 70, where the path ends; cut to no length there, it is not a
        // zero-length entry of the pattern and marks nothing, so the image ends at `68.5 + 1`.
        style.dash_phase = 0.0;
        style.dash_array = vec![1.0, 1.5];
        style.cap = LineCap::Round;
        assert_close(&intervals(&path, &style, ONTO_X), &[(9.0, 69.5)]);
    }

    /// §8.5.3.2's zero-length dash is painted with its caps, oriented by the path: `[0 20]` under
    /// projecting caps on a diagonal is a square of the width at each dash, reaching
    /// `w/2 · (|d_x| + |n_x|)` either side.
    #[test]
    fn a_zero_length_dash_is_its_caps() {
        let path = polyline(&[(0.0, 0.0), (30.0, 40.0)], false);
        let mut style = stroke(2.0, LineCap::Square, LineJoin::Miter);
        style.dash_array = vec![0.0, 20.0];
        // Dashes at arc length 0, 20 and 40 of a 50-long segment: x = 0, 12, 24.
        let spread = 0.6 + 0.8;
        assert_close(
            &intervals(&path, &style, ONTO_X),
            &[
                (-spread, spread),
                (12.0 - spread, 12.0 + spread),
                (24.0 - spread, 24.0 + spread),
            ],
        );
        style.cap = LineCap::Butt;
        assert_eq!(collapsed_stroke_by_transform(&path, &style, ONTO_X), None);
    }

    /// A closed subpath has joins where an open one has caps, and its dashes that run through
    /// its first vertex are one dash joined there.
    #[test]
    fn a_closed_subpath_is_joined_at_its_start() {
        let square = polyline(
            &[(10.0, 10.0), (30.0, 10.0), (30.0, 30.0), (10.0, 30.0)],
            true,
        );
        // Mitres at the four corners reach `x ± 1` like the bands: `[9, 31]`.
        assert_close(
            &intervals(
                &square,
                &stroke(2.0, LineCap::Butt, LineJoin::Miter),
                ONTO_X,
            ),
            &[(9.0, 31.0)],
        );
        // Seen along `x − y`, the mitre at (30, 10) reaches `(31, 9)`, 22; the one at (10, 30)
        // reaches `(9, 31)`, −22; the first vertex (10, 10) is a joined corner, not two caps.
        let mut style = stroke(2.0, LineCap::Butt, LineJoin::Miter);
        assert_close(
            &intervals(&square, &style, ONTO_X_MINUS_Y),
            &[(-22.0, 22.0)],
        );
        // `[50 31] 0 d`: on from (10,10) round to (30,30) and 10 along its third side, then off
        // for the rest of the 80 — so the first vertex is a butt cap rather than a join, and the
        // band that reached `x = 9` there is gone: `[10, 31]`. The gap is one unit longer than
        // the path needs, because a gap ending exactly at the start puts the answer on the last
        // bit of four `hypot`s, which the standard library does not promise to round correctly
        // and Miri, on purpose, does not.
        style.dash_array = vec![50.0, 31.0];
        assert_close(&intervals(&square, &style, ONTO_X), &[(10.0, 31.0)]);
    }

    /// §8.5.3.2: a degenerate subpath is painted only under round caps, as a circle.
    #[test]
    fn a_degenerate_subpath_is_a_dot_under_round_caps_only() {
        let dot = polyline(&[(40.0, 20.0), (40.0, 20.0)], false);
        assert_close(
            &intervals(&dot, &stroke(6.0, LineCap::Round, LineJoin::Miter), ONTO_X),
            &[(37.0, 43.0)],
        );
        assert_eq!(
            collapsed_stroke_by_transform(
                &dot,
                &stroke(6.0, LineCap::Square, LineJoin::Miter),
                ONTO_X
            ),
            None
        );
    }

    /// A circle of radius 10 about (50, 50), four cubics, at `4 w`: `[38, 62]` to the cubic
    /// approximation's own 0.027% of the radius and the flattening's 256th.
    #[test]
    fn a_curve_is_flattened_to_its_set() {
        let k = 5.522_848;
        let mut circle = Path::new();
        circle.push(PathCommand::MoveTo(Point::new(60.0, 50.0)));
        for [one, two, to] in [
            [(60.0, 50.0 + k), (50.0 + k, 60.0), (50.0, 60.0)],
            [(50.0 - k, 60.0), (40.0, 50.0 + k), (40.0, 50.0)],
            [(40.0, 50.0 - k), (50.0 - k, 40.0), (50.0, 40.0)],
            [(50.0 + k, 40.0), (60.0, 50.0 - k), (60.0, 50.0)],
        ] {
            let [one, two, to] = [one, two, to].map(|(x, y)| Point::new(x, y));
            circle.push(PathCommand::CurveTo(one, two, to));
        }
        circle.push(PathCommand::Close);
        let found = intervals(
            &circle,
            &stroke(4.0, LineCap::Butt, LineJoin::Miter),
            ONTO_X_MINUS_Y,
        );
        // Along `x − y` the circle's extremes are `±10√2` about 0, and the band `±2√2`.
        let reach = 12.0 * 2f64.sqrt();
        assert_eq!(found.len(), 1);
        assert!(
            (f64::from(found[0].0) + reach).abs() < 0.02
                && (f64::from(found[0].1) - reach).abs() < 0.02,
            "{found:?} against ±{reach}"
        );
    }

    /// A matrix onto a point, and one that is invertible, are not this construction's.
    #[test]
    fn only_a_matrix_of_rank_one_is_restated() {
        let path = polyline(&[(20.0, 30.0), (60.0, 70.0)], false);
        let style = stroke(5.0, LineCap::Round, LineJoin::Miter);
        for transform in [
            Transform::new(0.0, 0.0, 0.0, 0.0, 50.0, 50.0),
            Transform::IDENTITY,
        ] {
            assert_eq!(
                collapsed_stroke_by_transform(&path, &style, transform),
                None
            );
        }
    }
}

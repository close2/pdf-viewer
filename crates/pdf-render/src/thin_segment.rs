//! A straight stroke whose whole ink lies in a block of two by two device pixels, and the exact
//! area it puts in each of them — ISO 32000-2 §8.4.3.2, §8.4.3.3 and §10.7.4.
//!
//! # Why a mark this small is decided before it becomes geometry
//!
//! A page can be millions of such marks: the drawing that asked for this module states four
//! million round-capped dots a fifth of a pixel long at the size a window opens it in, and a
//! backend that hands each one to a general stroker builds a flattened outline, a scan converter's
//! edges and a compositing pipeline for a shape whose whole ink is a few hundredths of one pixel.
//! ADR 1817 measured what that costs and chose this answer: the mark is recognised here, at the
//! device transform, and its coverage is stated in closed form. ADR 1829 is the first increment.
//!
//! **It is decided in this crate rather than in a backend** so that every backend answers the
//! same marks (trap 2): the processor draws them through [`ThinSegment::cells`], and the graphics
//! device is to state the same shape as a primitive of its own.
//!
//! # The shape, from the clauses
//!
//! §8.4.3.2 defines the stroke as a set:
//!
//! > stroking a path shall entail painting all points whose perpendicular distance from the path
//! > in user space is less than or equal to half the line width.
//!
//! and §8.4.3.3's Table 53 states the three ends of an open segment:
//!
//! > The stroke shall be squared off at the endpoint of the path. There shall be no projection
//! > beyond the end of the path.
//!
//! > A semicircular arc with a diameter equal to the line width shall be drawn around the
//! > endpoint and shall be filled in.
//!
//! > The stroke shall continue beyond the endpoint of the path for a distance equal to half the
//! > line width and shall be squared off.
//!
//! For one straight segment that set is a band of the line's width with one of those ends at
//! either endpoint, and the device shape is its image. Under a transform that keeps the axes the
//! disc of the half-width maps to an axis-aligned ellipse whose semi-axes are
//! [`ThinSegment::half_x`] and [`ThinSegment::half_y`]. Dividing the device's two axes by them,
//! composed with that transform, is a similarity of the path's own space — a reflection or a
//! quarter turn, and a scale of one over the half-width — so in the divided frame the shape is the
//! clause's set again: the points within one of the divided segment, its ends square to it. An area
//! there times `half_x · half_y` is the area in the device.
//!
//! # The area, from §10.7.4's pixel
//!
//! §10.7.4 names the pixel as the half-open square `[i, i+1) × [j, j+1)`, and this tree's
//! anti-aliasing departure (`doc/todo/_scan-conversion.md`) paints a pixel at the area of the
//! shape inside it. In the divided frame a pixel is an axis-aligned rectangle, the band and the
//! projecting ends a convex quadrilateral, and a round end a half disc: the unit disc on the far
//! side of the line through the endpoint square to the segment. So the area in a rectangle is a
//! convex polygon clipped to it and measured by its shoelace, plus — for a round end — the unit
//! disc's area inside a convex polygon, which is the sum over the polygon's edges of the disc's area
//! inside the triangle each edge makes with the centre: the triangle where the edge is inside the
//! circle and the sector where it is outside ([`disc_in_triangle`]). The pieces overlap in no area,
//! so they add. Nothing is flattened and nothing is sampled, which is the difference from the
//! general construction: there a round end is the stroker's chords, and its area the polygon's.
//!
//! # The population, and what each exclusion keeps out
//!
//! - **One `m` and one `l`.** A second segment brings a join, and a close brings a join to the
//!   start. Of no length, only under a round cap: that is §8.5.3.2's dot,
//!
//!   > the S operator shall paint it only if round line caps have been specified, producing a
//!   > filled circle centred at the single point.
//!
//!   and the filled circle is this shape at length zero — the two half discs about one point, with
//!   no band between them — so it is measured by the same closed form and needs no direction
//!   (ADR 1857). Under a butt or projecting cap the clause asks for no output, which
//!   [`crate::split_degenerate`] answers. **A dot none of whose pixels holds one level stays there
//!   too**: [`crate::point_mark`] places it whole in the pixel §10.7.4 names by flooring its
//!   centre, at the least the raster can state, so that its ink does not depend on where the grid
//!   falls under it; the processor's floor for a thin segment lifts every pixel such a mark
//!   touches, which at a pixel's corner is four levels for a mark of two, and the device's line
//!   lane has no floor and would round each quarter to nothing.
//! - **No dash.** §8.4.3.6's pattern cuts the segment into pieces this shape is not.
//! - **A solid paint and §11.3.5's `Normal` blend.** The backend composites the coverage once with
//!   the paint's own colour; a shading would be evaluated per pixel and a computed blend function
//!   composites otherwise. The clip and the soft mask are not part of the question: they meet the
//!   coverage the way they meet every other mark's.
//! - **A transform that keeps the axes**, which is what makes the pen an axis-aligned ellipse and
//!   the divided frame a rescaling rather than a turn.
//! - **Ink within two pixel columns and two pixel rows.** The block bounds the work a mark costs
//!   to four pixels; a larger mark is the general construction's, where it costs little beside its
//!   pixels anyway.

use crate::geom::{Path, PathCommand, Point, Transform};
use crate::paint::{BlendMode, LineCap, Paint, Stroke};

/// A straight stroke that [`thin_segment`] admitted, in device pixels.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ThinSegment {
    /// The segment's first endpoint, in device space.
    pub from: Point,
    /// The segment's second endpoint, in device space. Equal to [`Self::from`] only under a round
    /// cap, where the shape is §8.5.3.2's filled circle (ADR 1857).
    pub to: Point,
    /// The pen's semi-axis along device x: the half-width's disc maps to an axis-aligned ellipse,
    /// and this is how far it reaches horizontally.
    pub half_x: f32,
    /// The pen's semi-axis along device y. Equal to [`Self::half_x`] under a similarity.
    pub half_y: f32,
    /// Table 53's end, the same at both endpoints.
    pub cap: LineCap,
}

/// The largest number of pixel columns, and of pixel rows, a [`ThinSegment`]'s ink may meet.
pub const THIN_SEGMENT_BLOCK: u8 = 2;

/// The stroke `path` under `stroke` as a [`ThinSegment`], or `None` for any mark outside the
/// population the module comment states.
///
/// `to_device` is the whole map from the path's space to the device's, the one
/// [`Stroke::device_width`] takes; the width is that function's, so §8.4.3.2's `0 w` and §10.7.5's
/// adjustment are answered where they always are.
#[must_use]
pub fn thin_segment(
    path: &Path,
    stroke: &Stroke,
    paint: &Paint,
    blend: BlendMode,
    to_device: Transform,
) -> Option<ThinSegment> {
    if !matches!(paint, Paint::Solid(_)) || blend != BlendMode::Normal {
        return None;
    }
    if !stroke.dash_array.is_empty() || !to_device.preserves_axes() {
        return None;
    }
    let [PathCommand::MoveTo(start), PathCommand::LineTo(end)] = path.commands() else {
        return None;
    };
    let determinant = to_device.determinant();
    if !(determinant.is_finite() && determinant != 0.0) {
        return None;
    }
    let width = stroke.device_width(to_device);
    if !(width.is_finite() && width > 0.0) {
        return None;
    }
    let (from, to) = (to_device.apply(*start), to_device.apply(*end));
    // A segment of no length on the device is a disc under a round cap — the capsule at length
    // zero, which is §8.5.3.2's filled circle where the path's own points coincide and the same
    // set to the rounding where the transform merged two — and under the other two caps it has no
    // direction to put an end along, so it stays with `split_degenerate`.
    if ![from.x, from.y, to.x, to.y].iter().all(|v| v.is_finite())
        || (from == to && stroke.cap != LineCap::Round)
    {
        return None;
    }
    // The pen — the disc of the half-width — mapped to the device. A transform that keeps the axes
    // has one of `a`, `c` zero and one of `b`, `d` zero, so each sum is the one stretch that axis
    // receives.
    let half = width / 2.0;
    let segment = ThinSegment {
        from,
        to,
        half_x: half * (to_device.a.abs() + to_device.c.abs()),
        half_y: half * (to_device.b.abs() + to_device.d.abs()),
        cap: stroke.cap,
    };
    let positive = |v: f32| v.is_finite() && v > 0.0;
    if !(positive(segment.half_x) && positive(segment.half_y)) {
        return None;
    }
    if from != to {
        return segment.pixels().is_some().then_some(segment);
    }
    // The module comment's last word on §8.5.3.2's dot: one under a level everywhere is
    // `point_mark`'s, on every backend. Its areas are taken here only for a dot, so that a
    // segment's classification stays the block's four comparisons.
    let (_, cells) = segment.cells()?;
    cells
        .iter()
        .any(|&area| area >= ONE_LEVEL)
        .then_some(segment)
}

/// One level of an eight-bit coverage, as a share of a pixel: the least a pixel can be painted at.
const ONE_LEVEL: f64 = 1.0 / 255.0;

/// A rectangle in device space, by its four sides, in the `f64` the coverage is computed in.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct InkBox {
    /// The smallest x the ink reaches.
    pub left: f64,
    /// The smallest y the ink reaches.
    pub top: f64,
    /// The largest x the ink reaches.
    pub right: f64,
    /// The largest y the ink reaches.
    pub bottom: f64,
}

/// The device pixels a [`ThinSegment`]'s ink meets: at most [`THIN_SEGMENT_BLOCK`] columns from
/// `column` and as many rows from `row`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PixelBlock {
    /// The first column's index, §10.7.4's `i` — a whole number, possibly negative.
    pub column: f64,
    /// The first row's index, §10.7.4's `j` — a whole number, possibly negative.
    pub row: f64,
    /// How many columns, one or two.
    pub columns: u8,
    /// How many rows, one or two.
    pub rows: u8,
}

impl ThinSegment {
    /// The device box the ink lies in: the band's corners for a butt or projecting end — a
    /// projecting end of a turned segment reaches past its endpoint by more than the pen — and the
    /// segment grown by the pen for a round one.
    ///
    /// In `f64` and from the same frame the areas are taken in, so that the pixels the box names
    /// are every pixel an area is found in.
    #[must_use]
    pub fn ink(&self) -> InkBox {
        let frame = self.frame();
        let (left, top, right, bottom) = match self.cap {
            LineCap::Round => (
                frame.p0.0.min(frame.p1.0) - 1.0,
                frame.p0.1.min(frame.p1.1) - 1.0,
                frame.p0.0.max(frame.p1.0) + 1.0,
                frame.p0.1.max(frame.p1.1) + 1.0,
            ),
            LineCap::Butt | LineCap::Square => frame.band().iter().fold(
                (
                    f64::INFINITY,
                    f64::INFINITY,
                    f64::NEG_INFINITY,
                    f64::NEG_INFINITY,
                ),
                |(left, top, right, bottom), &(x, y)| {
                    (left.min(x), top.min(y), right.max(x), bottom.max(y))
                },
            ),
        };
        InkBox {
            left: left * frame.scale.0,
            top: top * frame.scale.1,
            right: right * frame.scale.0,
            bottom: bottom * frame.scale.1,
        }
    }

    /// The pixels the ink meets, or `None` where it meets more than [`THIN_SEGMENT_BLOCK`] columns
    /// or rows — or where a coordinate is not finite.
    ///
    /// A pixel is half-open, so ink ending exactly on a pixel boundary does not meet the pixel
    /// beyond it.
    #[must_use]
    pub fn pixels(&self) -> Option<PixelBlock> {
        let ink = self.ink();
        let span = |low: f64, high: f64| -> Option<(f64, u8)> {
            if !(low.is_finite() && high.is_finite()) {
                return None;
            }
            let first = low.floor();
            let count = high.ceil() - first;
            if count <= 1.0 {
                Some((first, 1))
            } else if count <= f64::from(THIN_SEGMENT_BLOCK) {
                Some((first, 2))
            } else {
                None
            }
        };
        let (column, columns) = span(ink.left, ink.right)?;
        let (row, rows) = span(ink.top, ink.bottom)?;
        Some(PixelBlock {
            column,
            row,
            columns,
            rows,
        })
    }

    /// The shape's whole area, in square device pixels: the band, and the ends Table 53 adds.
    #[must_use]
    pub fn area(&self) -> f64 {
        let frame = self.frame();
        let unit = match self.cap {
            LineCap::Butt => 2.0 * frame.length,
            LineCap::Square => 2.0 * frame.length + 4.0,
            LineCap::Round => 2.0 * frame.length + std::f64::consts::PI,
        };
        unit * frame.scale.0 * frame.scale.1
    }

    /// The area of the shape inside device pixel `(column, row)` — §10.7.4's
    /// `[column, column + 1) × [row, row + 1)` — in `0.0..=1.0`.
    #[must_use]
    pub fn coverage(&self, column: f64, row: f64) -> f64 {
        self.frame()
            .area_within((column, row, column + 1.0, row + 1.0))
            .clamp(0.0, 1.0)
    }

    /// The areas of every pixel of [`Self::pixels`]'s block, by rows and within a row by columns:
    /// the first `columns` entries are the first row's. `None` where the block is.
    ///
    /// **Fewer areas are taken than there are pixels**, and that is what this costs per mark. The
    /// shape lies inside its block, so the block's pixels partition it: one pixel holds the whole
    /// area, which [`Self::area`] states without any geometry; two hold the area on one side of the
    /// line between them and the rest; four need the quarter on one corner and the two halves it
    /// lies in, and the other three follow by subtraction.
    #[must_use]
    pub fn cells(&self) -> Option<(PixelBlock, [f64; 4])> {
        let block = self.pixels()?;
        let frame = self.frame();
        let whole = self.area();
        let ink = self.ink();
        // The ink box grown a pixel, so that a region bounded by it on a side the block does not
        // divide holds the whole shape on that side.
        let (left, top, right, bottom) = (
            ink.left - 1.0,
            ink.top - 1.0,
            ink.right + 1.0,
            ink.bottom + 1.0,
        );
        let (x, y) = (block.column + 1.0, block.row + 1.0);
        let areas = match (block.columns, block.rows) {
            (1, 1) => [whole, 0.0, 0.0, 0.0],
            (2, 1) => {
                let first = frame.area_within((left, top, x, bottom));
                [first, whole - first, 0.0, 0.0]
            }
            (1, 2) => {
                let first = frame.area_within((left, top, right, y));
                [first, whole - first, 0.0, 0.0]
            }
            _ => {
                let corner = frame.area_within((left, top, x, y));
                let left_half = frame.area_within((left, top, x, bottom));
                let top_half = frame.area_within((left, top, right, y));
                [
                    corner,
                    top_half - corner,
                    left_half - corner,
                    whole - left_half - top_half + corner,
                ]
            }
        };
        Some((block, areas.map(|area| area.clamp(0.0, 1.0))))
    }

    /// The segment in the frame where the pen is the unit disc.
    fn frame(&self) -> Frame {
        let scale = (f64::from(self.half_x), f64::from(self.half_y));
        let p0 = (
            f64::from(self.from.x) / scale.0,
            f64::from(self.from.y) / scale.1,
        );
        let p1 = (
            f64::from(self.to.x) / scale.0,
            f64::from(self.to.y) / scale.1,
        );
        let run = (p1.0 - p0.0, p1.1 - p0.1);
        let length = run.0.hypot(run.1);
        let direction = if length > 0.0 {
            (run.0 / length, run.1 / length)
        } else {
            (1.0, 0.0)
        };
        Frame {
            p0,
            p1,
            direction,
            length,
            cap: self.cap,
            scale,
        }
    }
}

/// A [`ThinSegment`] with both device axes divided by the pen's semi-axes.
struct Frame {
    /// The first endpoint.
    p0: (f64, f64),
    /// The second endpoint.
    p1: (f64, f64),
    /// The unit vector from `p0` to `p1`.
    direction: (f64, f64),
    /// The distance from `p0` to `p1`.
    length: f64,
    /// Table 53's end.
    cap: LineCap,
    /// The pen's semi-axes, by which an area here is multiplied to be a device area.
    scale: (f64, f64),
}

impl Frame {
    /// The band — the segment's own rectangle of half-width one, grown by one past each end for a
    /// projecting cap — as its four corners, wound one way.
    fn band(&self) -> [(f64, f64); 4] {
        let (dx, dy) = self.direction;
        let normal = (-dy, dx);
        let reach = if self.cap == LineCap::Square {
            1.0
        } else {
            0.0
        };
        let start = (self.p0.0 - dx * reach, self.p0.1 - dy * reach);
        let end = (self.p1.0 + dx * reach, self.p1.1 + dy * reach);
        [
            (start.0 + normal.0, start.1 + normal.1),
            (end.0 + normal.0, end.1 + normal.1),
            (end.0 - normal.0, end.1 - normal.1),
            (start.0 - normal.0, start.1 - normal.1),
        ]
    }

    /// The shape's device area inside the device rectangle `(left, top, right, bottom)`.
    fn area_within(&self, (left, top, right, bottom): (f64, f64, f64, f64)) -> f64 {
        let region = (
            left / self.scale.0,
            top / self.scale.1,
            right / self.scale.0,
            bottom / self.scale.1,
        );
        let mut band = Polygon::of(&self.band());
        band.clip_to(region);
        let mut area = band.area();
        if self.cap == LineCap::Round {
            // Each end is the unit disc beyond the line through its endpoint square to the
            // segment, so the rectangle is cut by that line first and the disc measured inside.
            let (dx, dy) = self.direction;
            for (centre, outward) in [(self.p0, (-dx, -dy)), (self.p1, (dx, dy))] {
                let mut beyond = Polygon::rectangle(region);
                beyond.clip(|p| (p.0 - centre.0) * outward.0 + (p.1 - centre.1) * outward.1);
                area += beyond.disc_area(centre);
            }
        }
        area * self.scale.0 * self.scale.1
    }
}

/// A convex polygon of at most [`Polygon::CAPACITY`] vertices, kept on the stack: clipping a
/// quadrilateral by four half-planes, or a rectangle by one, never needs more.
struct Polygon {
    /// The vertices, the first `len` of them in use.
    points: [(f64, f64); Polygon::CAPACITY],
    /// How many vertices are in use.
    len: usize,
}

impl Polygon {
    /// A convex quadrilateral clipped by four half-planes gains at most one vertex for each.
    const CAPACITY: usize = 8;

    /// A polygon of these four vertices.
    fn of(vertices: &[(f64, f64); 4]) -> Self {
        let mut points = [(0.0, 0.0); Self::CAPACITY];
        if let Some(head) = points.get_mut(..4) {
            head.copy_from_slice(vertices);
        }
        Self { points, len: 4 }
    }

    /// The rectangle `(left, top, right, bottom)` as a polygon.
    fn rectangle((left, top, right, bottom): (f64, f64, f64, f64)) -> Self {
        Self::of(&[(left, top), (right, top), (right, bottom), (left, bottom)])
    }

    /// The vertices in use.
    fn vertices(&self) -> &[(f64, f64)] {
        self.points.get(..self.len).unwrap_or(&[])
    }

    /// Each edge of the polygon, from a vertex to the next and from the last back to the first.
    fn edges(&self) -> impl Iterator<Item = ((f64, f64), (f64, f64))> + '_ {
        let vertices = self.vertices();
        vertices
            .iter()
            .zip(vertices.iter().cycle().skip(1))
            .map(|(&a, &b)| (a, b))
    }

    /// Keeps the part where the linear function `side` is not negative — one step of Sutherland
    /// and Hodgman's clipping.
    fn clip(&mut self, side: impl Fn((f64, f64)) -> f64) {
        let mut kept = [(0.0, 0.0); Self::CAPACITY];
        let mut count = 0;
        for (current, next) in self.edges() {
            let (here, there) = (side(current), side(next));
            let mut push = |point: (f64, f64)| {
                if let Some(slot) = kept.get_mut(count) {
                    *slot = point;
                    count = count.saturating_add(1);
                }
            };
            if here >= 0.0 {
                push(current);
            }
            if (here >= 0.0) != (there >= 0.0) {
                let t = here / (here - there);
                push((
                    current.0 + (next.0 - current.0) * t,
                    current.1 + (next.1 - current.1) * t,
                ));
            }
        }
        self.points = kept;
        self.len = count;
    }

    /// Keeps the part inside the rectangle `(left, top, right, bottom)`.
    fn clip_to(&mut self, (left, top, right, bottom): (f64, f64, f64, f64)) {
        self.clip(|p| p.0 - left);
        self.clip(|p| right - p.0);
        self.clip(|p| p.1 - top);
        self.clip(|p| bottom - p.1);
    }

    /// The area, by the shoelace formula, whichever way the polygon is wound.
    fn area(&self) -> f64 {
        let twice: f64 = self.edges().map(|(a, b)| a.0 * b.1 - b.0 * a.1).sum();
        (twice / 2.0).abs()
    }

    /// The area of the unit disc centred at `centre` inside the polygon.
    fn disc_area(&self, centre: (f64, f64)) -> f64 {
        let signed: f64 = self
            .edges()
            .map(|(a, b)| {
                disc_in_triangle(
                    (a.0 - centre.0, a.1 - centre.1),
                    (b.0 - centre.0, b.1 - centre.1),
                )
            })
            .sum();
        signed.abs()
    }
}

/// The signed area of the unit disc centred at the origin inside the triangle `(0, a, b)`.
///
/// The edge from `a` to `b` is cut where it crosses the circle, and each piece contributes the
/// triangle it makes with the centre where it lies inside the circle, and the sector it subtends
/// where it lies outside — half its angle, the radius being one. The sign is the winding's, so the
/// edges of a polygon summed give the disc's area inside it.
#[must_use]
pub fn disc_in_triangle(a: (f64, f64), b: (f64, f64)) -> f64 {
    let edge = (b.0 - a.0, b.1 - a.1);
    let length = edge.0 * edge.0 + edge.1 * edge.1;
    if length <= 0.0 {
        return 0.0;
    }
    // Where `|a + t·edge|² = 1`: `length·t² + 2·half·t + (|a|² − 1) = 0`.
    let half = a.0 * edge.0 + a.1 * edge.1;
    let constant = a.0 * a.0 + a.1 * a.1 - 1.0;
    let discriminant = half * half - length * constant;
    let mut cuts = [0.0; 4];
    let mut count = 1;
    if discriminant > 0.0 {
        let root = discriminant.sqrt();
        for t in [(-half - root) / length, (-half + root) / length] {
            if t > 0.0
                && t < 1.0
                && let Some(slot) = cuts.get_mut(count)
            {
                *slot = t;
                count = count.saturating_add(1);
            }
        }
    }
    if let Some(slot) = cuts.get_mut(count) {
        *slot = 1.0;
        count = count.saturating_add(1);
    }
    let at = |t: f64| (a.0 + edge.0 * t, a.1 + edge.1 * t);
    let mut area = 0.0;
    for pair in cuts.get(..count).unwrap_or(&[]).windows(2) {
        let &[from, to] = pair else { continue };
        let (p, q) = (at(from), at(to));
        let middle = at(f64::midpoint(from, to));
        let cross = p.0 * q.1 - p.1 * q.0;
        // Strictly inside: a piece whose midpoint is on the circle touches it there and lies
        // outside everywhere else, which is a tangent edge, and its share is the sector's.
        if middle.0 * middle.0 + middle.1 * middle.1 < 1.0 {
            area += cross / 2.0;
        } else {
            area += cross.atan2(p.0 * q.0 + p.1 * q.1) / 2.0;
        }
    }
    area
}

#[cfg(test)]
mod tests {
    use super::{ThinSegment, disc_in_triangle, thin_segment};
    use crate::geom::{Path, PathCommand, Point, Transform};
    use crate::paint::{BlendMode, Color, LineCap, Paint, Stroke};

    fn segment(points: &[(f32, f32)]) -> Path {
        let mut path = Path::new();
        for (index, &(x, y)) in points.iter().enumerate() {
            let point = Point::new(x, y);
            path.push(if index == 0 {
                PathCommand::MoveTo(point)
            } else {
                PathCommand::LineTo(point)
            });
        }
        path
    }

    fn stroke(width: f32, cap: LineCap) -> Stroke {
        Stroke {
            width,
            cap,
            ..Stroke::default()
        }
    }

    fn black() -> Paint {
        Paint::Solid(Color {
            r: 0.0,
            g: 0.0,
            b: 0.0,
            a: 1.0,
        })
    }

    /// The witness's own mark: `0.12 0 0 0.12 0 0 cm x y m x y+1 l S` under `2 w` and `1 J`, at the
    /// window's fit of 0.5938 pixels a unit with the page's y flipped.
    fn witness_transform() -> Transform {
        let scale = 0.12 * 0.5938;
        Transform {
            a: scale,
            b: 0.0,
            c: 0.0,
            d: -scale,
            e: 100.3,
            f: 700.6,
        }
    }

    fn classify(path: &Path, stroke: &Stroke, at: Transform) -> Option<ThinSegment> {
        thin_segment(path, stroke, &black(), BlendMode::Normal, at)
    }

    #[test]
    fn the_witness_marks_are_admitted_with_a_round_pen_of_their_own_width() {
        let at = witness_transform();
        let round = stroke(2.0, LineCap::Round);
        for run in [(10.0, 21.0), (11.0, 21.0), (11.0, 20.0)] {
            let thin = classify(&segment(&[(10.0, 20.0), run]), &round, at)
                .expect("one short round-capped segment is the population, in any direction");
            assert!((thin.half_x - 0.12 * 0.5938).abs() < 1e-6);
            assert!((thin.half_y - thin.half_x).abs() < 1e-7);
        }
    }

    #[test]
    fn everything_outside_the_population_is_declined() {
        let at = witness_transform();
        let round = stroke(2.0, LineCap::Round);
        // A second segment, which brings a join.
        let two = segment(&[(10.0, 20.0), (10.0, 21.0), (10.0, 22.0)]);
        assert!(classify(&two, &round, at).is_none());
        // A close, which brings a join to the start.
        let mut closed = segment(&[(10.0, 20.0), (10.0, 21.0)]);
        closed.push(PathCommand::Close);
        assert!(classify(&closed, &round, at).is_none());
        // A zero-length subpath under the two caps §8.5.3.2 paints nothing for.
        for cap in [LineCap::Butt, LineCap::Square] {
            let dot = segment(&[(10.0, 20.0), (10.0, 20.0)]);
            assert!(classify(&dot, &stroke(2.0, cap), at).is_none(), "{cap:?}");
        }
        // A dash.
        let mut dashed = round.clone();
        dashed.dash_array = vec![0.5, 0.5];
        assert!(classify(&segment(&[(10.0, 20.0), (10.0, 21.0)]), &dashed, at).is_none());
        // A turned transform.
        let (sin, cos) = 0.5_f32.sin_cos();
        let turned = Transform {
            a: cos * at.a,
            b: sin * at.a,
            c: -sin * at.a,
            d: cos * at.a,
            ..at
        };
        assert!(classify(&segment(&[(10.0, 20.0), (10.0, 21.0)]), &round, turned).is_none());
        // A blend this backend family computes.
        let path = segment(&[(10.0, 20.0), (10.0, 21.0)]);
        assert!(thin_segment(&path, &round, &black(), BlendMode::Multiply, at).is_none());
        // Ink past two pixels.
        assert!(classify(&segment(&[(10.0, 20.0), (10.0, 80.0)]), &round, at).is_none());
        let wide = stroke(40.0, LineCap::Round);
        assert!(classify(&segment(&[(10.0, 20.0), (10.0, 21.0)]), &wide, at).is_none());
    }

    #[test]
    fn a_quarter_turn_keeps_the_axes_and_exchanges_the_pens_stretches() {
        let at = Transform {
            a: 0.0,
            b: 0.3,
            c: 0.2,
            d: 0.0,
            e: 5.4,
            f: 5.4,
        };
        let thin = classify(
            &segment(&[(0.0, 0.0), (0.0, 1.0)]),
            &stroke(1.0, LineCap::Square),
            at,
        )
        .expect("a quarter turn keeps the axes");
        // Device x is the path's y stretched by `c`, device y the path's x stretched by `b`.
        assert!((thin.half_x - 0.1).abs() < 1e-6);
        assert!((thin.half_y - 0.15).abs() < 1e-6);
    }

    #[test]
    fn a_projecting_end_of_a_turned_segment_reaches_past_the_pen() {
        // At 45° the end's corners lie `(1 + 1)/√2 · h` past the endpoint along each axis — √2
        // times the pen — and the box, and so the block, has to hold them.
        let at = Transform::new(1.0, 0.0, 0.0, 1.0, 0.0, 0.0);
        let thin = classify(
            &segment(&[(10.4, 10.4), (10.6, 10.6)]),
            &stroke(0.4, LineCap::Square),
            at,
        )
        .expect("in the block");
        let ink = thin.ink();
        let reach = 0.2 * std::f64::consts::SQRT_2;
        assert!((ink.left - (10.4 - reach)).abs() < 1e-6, "{ink:?}");
        assert!((ink.right - (10.6 + reach)).abs() < 1e-6, "{ink:?}");
    }

    #[test]
    fn the_unit_disc_inside_polygons_whose_areas_are_known() {
        let quarter = std::f64::consts::FRAC_PI_4;
        let disc_in = |vertices: &[(f64, f64)]| -> f64 {
            let mut sum = 0.0;
            for (index, &a) in vertices.iter().enumerate() {
                let b = vertices[(index + 1) % vertices.len()];
                sum += disc_in_triangle(a, b);
            }
            sum.abs()
        };
        let square = |x0: f64, y0: f64, x1: f64, y1: f64| [(x0, y0), (x1, y0), (x1, y1), (x0, y1)];
        assert!((disc_in(&square(-2.0, -2.0, 2.0, 2.0)) - 4.0 * quarter).abs() < 1e-12);
        assert!((disc_in(&square(0.0, 0.0, 1.0, 1.0)) - quarter).abs() < 1e-12);
        assert!((disc_in(&square(-1.0, -1.0, 0.0, 1.0)) - 2.0 * quarter).abs() < 1e-12);
        // The square inscribed in the disc lies wholly inside it.
        let side = std::f64::consts::FRAC_1_SQRT_2;
        assert!((disc_in(&square(-side, -side, side, side)) - 2.0).abs() < 1e-12);
        // A segment cut off by the chord x = ½: `r²(θ − sin θ)/2` with θ = 2π/3.
        let theta = 2.0 * std::f64::consts::PI / 3.0;
        let expected = (theta - theta.sin()) / 2.0;
        assert!((disc_in(&square(0.5, -3.0, 4.0, 3.0)) - expected).abs() < 1e-12);
    }

    #[test]
    fn the_pixels_of_a_mark_add_up_to_its_area_at_every_cap_direction_and_placement() {
        for cap in [LineCap::Butt, LineCap::Round, LineCap::Square] {
            for run in [(0.6, 0.0), (0.0, 0.6), (0.4, 0.3), (-0.2, 0.5)] {
                for step in 0..40_u8 {
                    let shift = f32::from(step) / 40.0;
                    let at = Transform::new(0.5, 0.0, 0.0, 0.37, 3.0 + shift, 3.0 + shift * 0.7);
                    let path = segment(&[(0.0, 0.0), run]);
                    let thin = classify(&path, &stroke(0.9, cap), at).unwrap_or_else(|| {
                        panic!("{cap:?} along {run:?} at {shift}: not admitted")
                    });
                    let (block, cells) = thin.cells().expect("admitted, so in its block");
                    let (mut inside, mut each) = (0.0, 0.0);
                    for row in 0..block.rows {
                        for column in 0..block.columns {
                            let index = usize::from(row * block.columns + column);
                            let x = block.column + f64::from(column);
                            let y = block.row + f64::from(row);
                            let alone = thin.coverage(x, y);
                            assert!(
                                (alone - cells[index]).abs() < 1e-12,
                                "{cap:?} along {run:?} at {shift}: pixel ({x}, {y}) is {alone} \
                                 alone and {} by subtraction",
                                cells[index]
                            );
                            inside += cells[index];
                            each += alone;
                        }
                    }
                    // One pixel further on every side than the block, which must hold nothing.
                    let mut around = 0.0;
                    for row in -1..=i16::from(block.rows) {
                        for column in -1..=i16::from(block.columns) {
                            let x = block.column + f64::from(column);
                            around += thin.coverage(x, block.row + f64::from(row));
                        }
                    }
                    assert!(
                        (around - each).abs() < 1e-12,
                        "{cap:?} at {shift}: ink past its block"
                    );
                    assert!(
                        (inside - thin.area()).abs() < 1e-9,
                        "{cap:?} along {run:?} at {shift}: {inside} against {}",
                        thin.area()
                    );
                }
            }
        }
    }

    #[test]
    fn a_zero_length_round_capped_segment_is_the_filled_circle_of_its_width() {
        // §8.5.3.2's dot: the witness's 28 684 of them are `m` and `l` to the same point.
        let at = Transform::new(1.0, 0.0, 0.0, 1.0, 0.0, 0.0);
        let dot = classify(
            &segment(&[(3.5, 7.5), (3.5, 7.5)]),
            &stroke(0.8, LineCap::Round),
            at,
        )
        .expect("a round dot is the population");
        let disc = std::f64::consts::PI * 0.4 * 0.4;
        assert!((dot.area() - disc).abs() < 1e-6, "{}", dot.area());
        // Centred in one pixel, the whole disc is inside it.
        assert!((dot.coverage(3.0, 7.0) - disc).abs() < 1e-6);
        // Centred on a pixel corner, each of the four pixels holds a quarter of it.
        let corner = classify(
            &segment(&[(4.0, 8.0), (4.0, 8.0)]),
            &stroke(0.8, LineCap::Round),
            at,
        )
        .expect("in the block");
        let (block, cells) = corner.cells().expect("in its block");
        assert_eq!((block.columns, block.rows), (2, 2));
        for cell in cells {
            assert!((cell - disc / 4.0).abs() < 1e-6, "{cells:?}");
        }
        // Under unequal scales it is the ellipse of the pen's two semi-axes.
        let stretched = Transform::new(0.5, 0.0, 0.0, 0.25, 0.0, 0.0);
        let ellipse = classify(
            &segment(&[(7.0, 7.0), (7.0, 7.0)]),
            &stroke(2.0, LineCap::Round),
            stretched,
        )
        .expect("a transform that keeps the axes");
        let expected = std::f64::consts::PI * 0.5 * 0.25;
        assert!((ellipse.area() - expected).abs() < 1e-6);
    }

    #[test]
    fn a_band_along_an_axis_is_the_product_of_its_two_overlaps() {
        // The separable closed form `rectangle_coverage` states, for a butt-capped band.
        let at = Transform::new(1.0, 0.0, 0.0, 1.0, 0.0, 0.0);
        let butt = stroke(0.5, LineCap::Butt);
        // x from 3.25 to 3.75, y from 7.25 to 7.75: all of it in pixel (3, 7).
        let thin =
            classify(&segment(&[(3.25, 7.5), (3.75, 7.5)]), &butt, at).expect("in the block");
        assert!((thin.coverage(3.0, 7.0) - 0.25).abs() < 1e-12);
        // x 3.5..4.25, y 7.5..8.0: pixel (3, 7) holds 0.5 × 0.5 and pixel (4, 7) 0.25 × 0.5.
        let thin =
            classify(&segment(&[(3.5, 7.75), (4.25, 7.75)]), &butt, at).expect("in the block");
        assert!((thin.coverage(3.0, 7.0) - 0.25).abs() < 1e-12);
        assert!((thin.coverage(4.0, 7.0) - 0.125).abs() < 1e-12);
    }
}

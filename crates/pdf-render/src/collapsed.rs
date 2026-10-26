//! What a fill deposits where its shape encloses no area: ISO 32000-2 §10.7.4.
//!
//! `848 1085 10159 0 re f` is a rectangle ten thousand units wide and none tall, and it is how
//! `issue4260_reduced.pdf` rules every line of its grid. An antialiasing rasteriser computes
//! that shape's coverage as zero at every pixel and draws nothing, which is what this renderer
//! did for the whole of its life while three reference renderers drew the grid.
//!
//! # What the clause determines
//!
//! §10.7.4:
//!
//! > A shape shall be scan-converted by painting any pixel whose half-open square region
//! > intersects the shape, no matter how small the intersection is. This ensures that no shape
//! > ever disappears as a result of unfavourable placement relative to the device pixel grid,
//! > as might happen with other possible scan conversion rules. The area covered by painted
//! > pixels shall always be at least as large as the area of the original shape. This rule
//! > applies both to fill operations and to strokes with non-zero width.
//!
//! Read alone that sentence can be argued both ways, and the argument is worth writing down
//! because the answer is not obvious. Two paragraphs earlier the same subclause treats a shape
//! as half-open — "include the boundaries along their 'floor' sides, but not along their
//! 'ceiling' sides" — and for a rectangle whose floor and ceiling are the *same* line, that
//! rule read literally leaves an empty region and nothing to paint. So the subclause's two
//! halves collide, and neither is subordinate to the other on its face.
//!
//! What settles it is the neighbouring clause, §8.5.3.3.1:
//!
//! > If a subpath is degenerate (consists entirely of one or more points at the same
//! > coordinates), the subpath shall be considered to enclose the single device pixel lying
//! > under that point
//!
//! A single point — the *most* degenerate shape a fill can have — is stated to enclose a
//! pixel. A zero-height rectangle contains such points and encloses at least what they do, so
//! a reading under which it paints nothing makes the smaller shape mark more of the page than
//! the larger one that contains it. The half-open convention exists so that two shapes sharing
//! an edge neither overlap nor leave a seam; using it to annihilate a shape is not the job it
//! was given, and §10.7.4's own stated consequence — "no shape ever disappears" — says so.
//!
//! The point itself is painted, by the same sentence (ADR 2330). §8.5.3.3.1 goes on to call its
//! answer "device-dependent and not generally useful", and that clause carries no modal verb: it
//! says which pixel the point encloses depends on the device's grid, and that a producer gains
//! little by writing one, not that a processor may paint none. §10.7.4's NOTE 1 reads
//! *intersects* the same way for every shape with an empty interior — "a filling region is
//! considered to intersect every pixel through which its boundary passes, even if the interior
//! of the filling region is empty" — so a zero-area intersection is an intersection, and the
//! pixel the point lies in is the one the clause names.
//!

//! # What is painted
//!
//! The thinnest mark the device has, which is the answer §8.4.3.2 already gives for a line
//! whose width is zero and §10.7.5 for one under half a pixel. Those are the same width and
//! [`thinnest_line`] is where it is stated, so this rule adds no second opinion about what a
//! device's minimum is — only a third place that asks for it.
//!
//! # Where it is painted, and why that is not the shape's own fractional position
//!
//! §10.7.4 states the answer twice more, and both statements are about *pixels* rather than
//! about a width:
//!
//! > Normally, the intersection of two regions is defined as the intersection of their
//! > interiors. However, for purposes of scan conversion, a filling region is considered to
//! > intersect every pixel through which its boundary passes, even if the interior of the
//! > filling region is empty.
//!
//! > A zero-width or zero-height rectangle paints a line 1 pixel wide.
//!
//! and the pixel a point belongs to is named in the same subclause:
//!
//! > let i = floor( x ) and j = floor( y ). The pixel that contains this point is the one
//! > identified as ( i, j )
//!
//! So the mark is the **run of whole device pixels the collapsed axis passes through**, and
//! that run is found by flooring the axis's device coordinate — not a band of one pixel's
//! width centred wherever the shape happened to fall between two pixel centres (ADR 0208). The
//! difference is visible rather than theoretical: an anti-aliasing rasteriser splits such a band
//! across two rows at every placement but one, so a page ruled with a grid of them —
//! `issue4260_reduced.pdf` is one — would come out as a mixture of crisp lines and fuzzy grey
//! double ones. A mark whose
//! appearance depends on where it falls relative to the grid is precisely what the clause's own
//! stated purpose forbids: "no shape ever disappears as a result of unfavourable placement
//! relative to the device pixel grid".
//!
//! Only the collapsed axis is snapped. Along the other one the subpath has a stated extent, and
//! what an extent gets on this device is the coverage its area implies — §10.7.4's first
//! documented departure, argued in the ledger's row for this clause. Snapping the length as well
//! would extend a departure from the clause into an axis where the clause is being followed.
//!
//! Snapping needs the placement transform to carry a device axis onto a path axis, so it is done
//! only where [`Transform::preserves_axes`] holds — a scale and a translation, or a quarter turn,
//! which is what every mark in the corpus rides. Under a rotation or a shear the collapsed axis
//! lies across the pixel grid and "the run of pixels it passes through" is a staircase rather
//! than a rectangle; there the band of one device pixel stays, as the stated fallback.
//!
//! # What does *not* move with it: a hairline stroke
//!
//! A `0 w` stroke down the same line still lands where the document puts it, and the two
//! constructions are therefore no longer byte-identical. That is deliberate, and the clauses
//! separate them rather than this renderer doing so.
//!
//! A degenerate fill has no width at all, so every pixel of its mark is this processor's
//! construction under a rule §10.7.4 states without condition. A stroke has a width the document
//! stated, and moving its *coordinates* onto the grid is §10.7.5's other half —
//!
//! > When stroke adjustment is enabled, the line width and the coordinates of a stroke shall
//! > automatically be adjusted as necessary to produce lines of uniform thickness.
//!
//! — which is conditioned on the graphics state's `/SA`, and applying it unconditionally is what
//! `AMBIGUOUS_STROKE_ADJUSTMENT`'s reading of `bug1743245.pdf` rests on this tree not doing.
//! §10.7.4 says the same thing from its own side by exempting the zero-width stroke from the
//! rule that governs the fill: "Zero-width strokes may be done in an implementation-defined
//! manner that may include fewer pixels than the rule implies." The fill's placement is stated;
//! the stroke's is left to the implementation and then made conditional. ADR 0208.
//!
//! The mark is *geometry* built here rather than a hairline stroke each backend applies,
//! for the reason [`crate::degenerate`] states a circle rather than trusting a round cap
//! (trap 2): a decision a backend can make alone is a decision none of them has made. It is also filled
//! separately from the path it came out of, under the non-zero rule, because a mark added to
//! the path itself would join that path's winding — under the even-odd rule a mark landing
//! inside a filled region would punch a hole in it.
//!
//! # A point, and a line across the axes
//!
//! Two more shapes enclose no area, and each is the same sentence's (ADR 2330):
//!
//! - **A point** — every point of the subpath, control points included, at one coordinate — is
//!   §8.5.3.3.1's degenerate subpath, and its mark is "the single device pixel lying under that
//!   point": the pixel §10.7.4 identifies by flooring the point's device coordinates, whole,
//!   under any invertible placement, stated back in the path's own space through the inverse.
//!   An `m` alone at the end of a path never arrives here, because §8.5.3.3.1 disregards it and
//!   `pdf-model` drops it before any backend looks.
//! - **A line across the axes** — every point on one line through two of them, which extends in
//!   both axes — encloses no area under either rule, so its mark is the band of one device pixel
//!   along that line, centred on it, from the furthest point one way to the furthest the other:
//!   the construction a line across the grid already takes when a matrix of rank one carries a
//!   rectangle onto it (ADR 1360). The band is stated in device space, where "one device pixel"
//!   is a width in every direction, and carried back through the inverse.
//!
//! Both are decided exactly, as the axis case is: a cubic lies on a line exactly when its four
//! control points do, since the line's equation of the curve is a polynomial whose Bernstein
//! coefficients are the control points' values, and `on_one_line` decides collinearity with no
//! rounding at all.
//!
//! # What this does not cover
//!
//! A subpath whose boundary retraces itself off any one line — out along two sides of a triangle
//! and back the same way — also encloses no area, and is filled as itself, which marks nothing.
//! Recognising it is the winding integral's question asked exactly of every subpath that is not
//! on a line, a cancellation of directed edges no simpler test answers; no document this tree
//! holds writes one, and §10.7.4's row records it as a departure with that cost (ADR 2330).

use crate::geom::{Path, PathCommand, Point, Transform};
use crate::paint::{Stroke, thinnest_line};

mod stroke_image;

use stroke_image::StrokeImage;

/// A fill's path separated into the part that encloses an area and the marks left by the part
/// that does not.
///
/// Produced by [`split_collapsed_fill`], which returns nothing at all for a path with no
/// collapsed subpath — the overwhelmingly common case, and the one that must not allocate.
#[derive(Debug, Clone, PartialEq)]
pub struct CollapsedFill {
    /// The subpaths that enclose an area, to be filled under the command's own rule.
    ///
    /// Empty when every subpath collapsed, in which case only the marks are drawn.
    pub filled: Path,
    /// The rectangles §10.7.4 asks for, to be filled under the **non-zero** rule with the
    /// command's own paint.
    ///
    /// One per collapsed subpath, each one device pixel thick along the axis the subpath has
    /// no extent in and exactly as long as the subpath is along the other. Under an
    /// axis-preserving transform that pixel is a *whole* device pixel — the run the collapsed
    /// axis passes through, which is what the clause states; under a rotation or a shear it is
    /// a band centred on the subpath's own line. The module comment argues both. A point's
    /// mark is the device pixel under it, and a line across the axes is the band of one device
    /// pixel along it (ADR 2330).
    pub marks: Path,
}

/// Splits a fill path's area-less subpaths out of it, ISO 32000-2 §10.7.4.
///
/// `to_device` maps the path's own space onto the device pixel grid, and it is the whole of
/// what this function needs: the mark's thickness comes from it through [`thinnest_line`],
/// exactly as a [`Stroke`]'s substituted width does, and its *placement* comes from it because
/// §10.7.4's rule is stated about device pixels. Taking the transform rather than a width is
/// what stops a caller from snapping against one space and measuring against another.
///
/// Returns `None` when there is nothing to separate, so that an ordinary path costs one pass
/// over its commands and no allocation. [`Path::collapses`] memoises that pass, which matters:
/// this question is asked once per fill command per strip the page is cut into. Also `None`
/// where the transform states no width at all — [`thinnest_line`]'s own refusal, for one that
/// is not finite or that lengthens nothing, leaving no space of the path's own for a mark to
/// be stated in. A transform that is singular while still stretching one axis does state a
/// width, and gets the band: it has flattened the whole page onto a line, so there is no pixel
/// grid left to snap the mark to and nothing on the device for it to be wrong about.
///
/// [`thinnest_line`]: crate::paint::thinnest_line
/// [`Stroke`]: crate::paint::Stroke
#[must_use]
pub fn split_collapsed_fill(path: &Path, to_device: Transform) -> Option<CollapsedFill> {
    if !path.collapses() {
        return None;
    }
    let thinnest = thinnest_line(to_device)?;
    // §10.7.4's rule is about whole device pixels, so it can only be applied where a device
    // axis is a path axis. Both halves are needed: the forward transform to find the pixel
    // run, the inverse to state it back in the space the returned geometry is in.
    let grid = to_device
        .preserves_axes()
        .then(|| to_device.invert())
        .flatten()
        .map(|inverse| Grid { to_device, inverse });
    // A point and a line across the axes are stated in device space and carried back, so they
    // need the inverse under any placement; without one there is no grid to name a pixel of.
    let inverse = to_device.invert();

    let mut filled = Path::new();
    let mut marks = Path::new();
    for extent in subpath_extents(path) {
        match extent.collapse(path.commands()) {
            Some(Collapse::Along(axis)) => append_mark(&mut marks, &extent, axis, thinnest, grid),
            Some(Collapse::Across { from, to }) => {
                if let Some(inverse) = inverse {
                    append_band_across(&mut marks, (from, to), to_device, inverse);
                }
            }
            Some(Collapse::Point(point)) => {
                if let Some(inverse) = inverse {
                    append_pixel_under(&mut marks, point, to_device, inverse);
                }
            }
            None => filled.extend(&path.commands()[extent.range()]),
        }
    }
    Some(CollapsedFill { filled, marks })
}

/// Appends the band of one device pixel along a line across the axes, ISO 32000-2 §10.7.4.
///
/// `ends` are the subpath's two furthest points along its line, in path space. The band is built
/// in device space — the segment between their images, widened by half a device pixel each side
/// along its own normal — and each corner is carried back through `inverse`, so it is one device
/// pixel thick whatever the placement stretches (ADR 2330).
fn append_band_across(
    into: &mut Path,
    ends: (Point, Point),
    to_device: Transform,
    inverse: Transform,
) {
    let (p, q) = (to_device.apply(ends.0), to_device.apply(ends.1));
    let (dx, dy) = (
        f64::from(q.x) - f64::from(p.x),
        f64::from(q.y) - f64::from(p.y),
    );
    let length = dx.hypot(dy);
    if length == 0.0 || !length.is_finite() {
        return;
    }
    // Half a device pixel along the unit normal.
    let (nx, ny) = (-dy / length / 2.0, dx / length / 2.0);
    let corner = |at: Point, side: f64| {
        let device = Point::new(
            narrow(f64::from(at.x) + side * nx),
            narrow(f64::from(at.y) + side * ny),
        );
        inverse.apply(device)
    };
    append_quad(
        into,
        [
            corner(p, 1.0),
            corner(q, 1.0),
            corner(q, -1.0),
            corner(p, -1.0),
        ],
    );
}

/// Appends the device pixel a point lies in, ISO 32000-2 §8.5.3.3.1 and §10.7.4.
///
/// §8.5.3.3.1: the degenerate subpath "shall be considered to enclose the single device pixel
/// lying under that point", and §10.7.4 identifies that pixel: "let i = floor( x ) and j =
/// floor( y ). The pixel that contains this point is the one identified as ( i, j )". Its four
/// corners are carried back through `inverse`, so the mark is that pixel under any placement
/// (ADR 2330).
fn append_pixel_under(into: &mut Path, point: Point, to_device: Transform, inverse: Transform) {
    let device = to_device.apply(point);
    let (column, row) = (device.x.floor(), device.y.floor());
    let corner = |x: f32, y: f32| inverse.apply(Point::new(x, y));
    append_quad(
        into,
        [
            corner(column, row),
            corner(column + 1.0, row),
            corner(column + 1.0, row + 1.0),
            corner(column, row + 1.0),
        ],
    );
}

/// Appends a closed quadrilateral, or nothing where a corner is not finite — as
/// [`append_mark`] declines a mark it cannot state.
fn append_quad(into: &mut Path, corners: [Point; 4]) {
    if corners
        .iter()
        .any(|corner| !corner.x.is_finite() || !corner.y.is_finite())
    {
        return;
    }
    let [first, second, third, fourth] = corners;
    into.push(PathCommand::MoveTo(first));
    into.push(PathCommand::LineTo(second));
    into.push(PathCommand::LineTo(third));
    into.push(PathCommand::LineTo(fourth));
    into.push(PathCommand::Close);
}

/// An `f64` device coordinate carried back to the `f32` every path is stated in.
#[expect(
    clippy::cast_possible_truncation,
    reason = "a device coordinate computed in f64 from f32 inputs and half a pixel, rounded once \
              to the f32 the path is stated in; a non-finite result is declined by append_quad"
)]
fn narrow(value: f64) -> f32 {
    value as f32
}

/// `path` stated on the line `transform` carries it onto, where that transform has no inverse —
/// ISO 32000-2 §10.7.4; `None` otherwise. The second value is the transform the restated path is
/// to be drawn under: the identity where the line runs along a page axis, and otherwise the
/// rotation that lays the restated path's own x-axis along the line (ADR 1360).
///
/// The restated path is flat in its own space, so [`split_collapsed_fill`] gives it §10.7.4's
/// mark: along a page axis the run of whole device pixels, and across the axes the band of one
/// device pixel that is that function's construction wherever the grid is turned.
///
/// **Which matrices qualify is decided exactly.** A 2 × 2 matrix of `f32` entries has rank one
/// when `ad = bc` and it is not zero, and a product of two `f32` values is exact in `f64` (48
/// significand bits of 53), so that comparison is exact: a matrix whose determinant cancels to
/// zero only in `f32` arithmetic — a full-rank matrix of enormous entries, which the corpus holds
/// in a damaged stream — is refused as before, since it carries the path onto no line at all.
///
/// **A matrix whose image of the path is one point** — rank zero, which carries every point onto
/// `(e, f)`, or rank one where this path lies along the line it projects away — restates the path
/// as that point in page space, so [`split_collapsed_fill`] paints the device pixel under it:
/// the shape §10.7.4 scan-converts is a point there, and a point is §8.5.3.3.1's degenerate
/// subpath, enclosing "the single device pixel lying under that point" (ADR 2330).
///
/// Curves are restated by their control points, which an affine map carries exactly.
#[must_use]
pub fn collapsed_by_transform(path: &Path, transform: Transform) -> Option<(Path, Transform)> {
    if transform.invert().is_some() {
        return None;
    }
    if let Some(point) = onto_a_point(transform) {
        let names_a_point = path
            .commands()
            .iter()
            .any(|c| command_points(c).next().is_some());
        return names_a_point.then(|| (single_point(point), Transform::IDENTITY));
    }
    let line = ImageLine::of(transform)?;
    let mut restated = Path::new();
    let (mut lowest, mut highest) = (f64::INFINITY, f64::NEG_INFINITY);
    let mut map = |point: Point| {
        let along = line.along(point);
        lowest = lowest.min(along);
        highest = highest.max(along);
        line.point(along)
    };
    for command in path.commands() {
        restated.push(match *command {
            PathCommand::MoveTo(point) => PathCommand::MoveTo(map(point)),
            PathCommand::LineTo(point) => PathCommand::LineTo(map(point)),
            PathCommand::CurveTo(one, two, to) => PathCommand::CurveTo(map(one), map(two), map(to)),
            PathCommand::Close => PathCommand::Close,
        });
    }
    // `lowest == highest` is a path whose image is one point, which the restatement states as a
    // point and the fill paints as the pixel under it.
    let spans = lowest.is_finite() && highest.is_finite() && lowest <= highest;
    let finite = restated
        .commands()
        .iter()
        .all(|command| command_points(command).all(|p| p.x.is_finite() && p.y.is_finite()));
    (spans && finite).then_some((restated, line.frame()))
}

/// Where a matrix of rank zero carries every point, `(e, f)`; `None` for any other matrix.
///
/// Rank zero is four entries that are exactly zero: a small entry is a matrix of rank one or
/// two, which carries the path onto a line or keeps its area.
fn onto_a_point(transform: Transform) -> Option<Point> {
    let zero = [transform.a, transform.b, transform.c, transform.d]
        .iter()
        .all(|entry| *entry == 0.0);
    let at = Point::new(transform.e, transform.f);
    (zero && at.x.is_finite() && at.y.is_finite()).then_some(at)
}

/// A path that is one degenerate subpath at `point`.
fn single_point(point: Point) -> Path {
    let mut path = Path::new();
    path.push(PathCommand::MoveTo(point));
    path.push(PathCommand::Close);
    path
}

/// A stroke's image under a matrix with no inverse, stated as the fill of that image — ISO
/// 32000-2 §10.7.4 and §8.4.3.2; `None` where the matrix is invertible or the stroke's set is
/// empty.
///
/// §8.4.3.2 states the stroke as a set in user space — "stroking a path shall entail painting all
/// points whose perpendicular distance from the path in user space is less than or equal to half
/// the line width" — together with §8.4.3.3's caps, §8.4.3.4's joins, §8.4.3.5's limit and
/// §8.4.3.6's dashes, and §10.7.4 scan-converts the shape whose "coordinates are mapped into
/// device space". A matrix of rank one maps the plane onto a line by a linear functional `s`
/// (`ImageLine::along`), so the image of the stroke's set is `s` of that set: each connected
/// piece of it — a subpath, or a dash — is compact, so its image is the interval from the least
/// to the greatest `s` over it, and the set is a union of convex parts whose extremes are closed
/// forms. That is the whole construction, and no stroker is involved: the interval is computed
/// from the path and the width alone, in the crate every backend reads (trap 2). ADR 1360.
///
/// The returned path is one flat subpath per interval, which [`split_collapsed_fill`] draws as
/// §10.7.4's line, and the transform is [`collapsed_by_transform`]'s frame. The width is the
/// document's own: a zero width contributes nothing beyond the path's own image, and the line one
/// device pixel wide that §8.4.3.2 asks of it is the mark itself. An interval of no length is a
/// point, restated as one so that the fill paints the pixel under it; and under a matrix of rank
/// zero the image of a set that is not empty is `(e, f)`, the same point (ADR 2330).
#[must_use]
pub(crate) fn collapsed_stroke_by_transform(
    path: &Path,
    stroke: &Stroke,
    transform: Transform,
) -> Option<(Path, Transform)> {
    if transform.invert().is_some() {
        return None;
    }
    if let Some(point) = onto_a_point(transform) {
        // The set is not empty exactly when its image on some line is not: the image on the
        // page's x-axis is asked of the same construction, and only whether it names an interval
        // is read from it.
        let probe = ImageLine::of(Transform::new(1.0, 0.0, 0.0, 0.0, point.x, point.y))?;
        let marks = StrokeImage::new(probe, stroke)?
            .of(path)
            .is_some_and(|intervals| !intervals.is_empty());
        return marks.then(|| (single_point(point), Transform::IDENTITY));
    }
    let line = ImageLine::of(transform)?;
    let intervals = StrokeImage::new(line, stroke)?.of(path)?;
    let mut restated = Path::new();
    for (lowest, highest) in intervals {
        let (from, to) = (line.point(lowest), line.point(highest));
        if !(from.x.is_finite() && from.y.is_finite() && to.x.is_finite() && to.y.is_finite()) {
            return None;
        }
        restated.push(PathCommand::MoveTo(from));
        if from != to {
            restated.push(PathCommand::LineTo(to));
        }
        restated.push(PathCommand::Close);
    }
    (!restated.is_empty()).then_some((restated, line.frame()))
}

/// Every point a command names, control points included.
fn command_points(command: &PathCommand) -> impl Iterator<Item = Point> {
    let (points, count) = match *command {
        PathCommand::MoveTo(point) | PathCommand::LineTo(point) => ([point; 3], 1),
        PathCommand::CurveTo(one, two, to) => ([one, two, to], 3),
        PathCommand::Close => ([Point::new(0.0, 0.0); 3], 0),
    };
    points.into_iter().take(count)
}

/// The line a matrix of rank one carries the plane onto, and the functional that says where on
/// it a point lands.
#[derive(Debug, Clone, Copy)]
struct ImageLine {
    /// Where the origin of user space lands, in page space.
    origin: (f64, f64),
    /// The line's unit direction in page space.
    direction: (f64, f64),
    /// `s(p) = functional · p`: how far along [`Self::direction`] from [`Self::origin`] the image
    /// of `p` lies. The matrix's rows projected on the direction.
    functional: (f64, f64),
}

impl ImageLine {
    /// The line `transform` maps onto, or `None` where its rank is not exactly one.
    #[expect(
        clippy::float_cmp,
        reason = "the rank is decided exactly: products of two f32 values are exact in f64, so \
                  equality here is the matrix's own singularity and not an accident of rounding"
    )]
    fn of(transform: Transform) -> Option<Self> {
        let x_axis = (f64::from(transform.a), f64::from(transform.b));
        let y_axis = (f64::from(transform.c), f64::from(transform.d));
        let origin = (f64::from(transform.e), f64::from(transform.f));
        let finite = [x_axis.0, x_axis.1, y_axis.0, y_axis.1, origin.0, origin.1]
            .iter()
            .all(|value| value.is_finite());
        if !finite || x_axis.0 * y_axis.1 != x_axis.1 * y_axis.0 {
            return None;
        }
        // The images of the two user axes are parallel; the longer is the better-conditioned
        // direction, and a zero column contributes nothing to either choice.
        let column = if x_axis.0.hypot(x_axis.1) >= y_axis.0.hypot(y_axis.1) {
            x_axis
        } else {
            y_axis
        };
        let length = column.0.hypot(column.1);
        if length == 0.0 || !length.is_finite() {
            return None;
        }
        let direction = (column.0 / length, column.1 / length);
        Some(Self {
            origin,
            direction,
            functional: (
                direction.0 * x_axis.0 + direction.1 * x_axis.1,
                direction.0 * y_axis.0 + direction.1 * y_axis.1,
            ),
        })
    }

    /// Where on the line `point` lands.
    fn along(self, point: Point) -> f64 {
        self.dot(point.x.into(), point.y.into())
    }

    /// `functional · (x, y)`.
    fn dot(self, x: f64, y: f64) -> f64 {
        self.functional.0 * x + self.functional.1 * y
    }

    /// Whether the line runs along a page axis, so that the restated path is stated in page space.
    fn on_an_axis(self) -> bool {
        self.direction.0 == 0.0 || self.direction.1 == 0.0
    }

    /// The point `along` the line, in the space [`Self::frame`] maps onto the page.
    #[expect(
        clippy::cast_possible_truncation,
        reason = "the restated path is f32 like every path; the value is checked finite by the \
                  caller, and one rounding of an exact f64 is closer than the f32 transform was"
    )]
    fn point(self, along: f64) -> Point {
        if self.on_an_axis() {
            Point::new(
                (self.origin.0 + self.direction.0 * along) as f32,
                (self.origin.1 + self.direction.1 * along) as f32,
            )
        } else {
            Point::new(along as f32, 0.0)
        }
    }

    /// The transform a path restated by [`Self::point`] is drawn under.
    #[expect(
        clippy::cast_possible_truncation,
        reason = "a unit direction and the matrix's own f32 translation, carried back to f32"
    )]
    fn frame(self) -> Transform {
        if self.on_an_axis() {
            Transform::IDENTITY
        } else {
            let (x, y) = (self.direction.0 as f32, self.direction.1 as f32);
            Transform::new(x, y, -y, x, self.origin.0 as f32, self.origin.1 as f32)
        }
    }
}

/// The device pixel grid, in both directions, for a placement transform that has axes to snap to.
#[derive(Debug, Clone, Copy)]
struct Grid {
    /// Path space onto the device.
    to_device: Transform,
    /// Its inverse, which states a run of device pixels back in the path's own space.
    inverse: Transform,
}

/// Whether any of a path's subpaths encloses no area because it lies on one line or at one
/// point.
///
/// The predicate behind [`Path::collapses`], which memoises it. Kept separate from
/// [`split_collapsed_fill`] so that the common answer — no — costs one walk and no allocation;
/// a subpath with an area is told apart by its third point that leaves the line through its
/// first two, which for an ordinary outline is its third point.
pub(crate) fn any_subpath_collapses(path: &Path) -> bool {
    subpath_extents(path).any(|extent| extent.collapse(path.commands()).is_some())
}

/// The axis a subpath has no extent along.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Axis {
    /// Every point shares one y, so the mark is a horizontal rule.
    Horizontal,
    /// Every point shares one x, so the mark is a vertical rule.
    Vertical,
}

/// How a subpath that encloses no area lies.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Collapse {
    /// On a line along a path axis, with no extent along the other.
    Along(Axis),
    /// On a line across both path axes, between its two furthest points along that line.
    Across {
        /// The point furthest one way along the line.
        from: Point,
        /// The point furthest the other way.
        to: Point,
    },
    /// At one point: §8.5.3.3.1's degenerate subpath.
    Point(Point),
}

/// Whether every one of `points` lies on the line through `a` and `b`, decided exactly.
///
/// The test is whether the cross product `(b − a) × (p − a)` is zero, which it is exactly when
/// `p` is on the line. Each difference of two `f32` coordinates is taken in `f64` and checked exact
/// by Knuth's two-sum, and each product of two such differences is compared with its rounding
/// error carried by a fused multiply-add, so the comparison is of the exact products and not of
/// two roundings of them. A difference that is not exact in `f64` — coordinates more than 2²⁹
/// apart in magnitude — answers `false`, which leaves the subpath to be filled as itself: the
/// test claims a line only where it has proved one.
fn on_one_line(a: Point, b: Point, points: impl Iterator<Item = Point>) -> bool {
    let (Some(ux), Some(uy)) = (exact_difference(b.x, a.x), exact_difference(b.y, a.y)) else {
        return false;
    };
    // `a` and `b` are on their own line, and skipping them keeps the fused multiply-add off
    // an ordinary outline, whose third point is off the line by more than a rounding.
    points.filter(|p| *p != a && *p != b).all(|p| {
        let (Some(vx), Some(vy)) = (exact_difference(p.x, a.x), exact_difference(p.y, a.y)) else {
            return false;
        };
        exact_products_equal((ux, vy), (uy, vx))
    })
}

/// `u − v` in `f64`, or `None` where that is not the exact difference: the rounding error of the
/// subtraction is computed exactly, so its being zero is the test of exactness itself.
fn exact_difference(u: f32, v: f32) -> Option<f64> {
    let (u, minus_v) = (f64::from(u), -f64::from(v));
    let sum = u + minus_v;
    // Knuth's two-sum: `error` is exactly `u − v − sum`, whatever the magnitudes.
    let virtual_minus_v = sum - u;
    let error = (u - (sum - virtual_minus_v)) + (minus_v - virtual_minus_v);
    (sum.is_finite() && error == 0.0).then_some(sum)
}

/// Whether `a.0 · a.1` and `b.0 · b.1` are exactly equal.
///
/// A product rounds once, and `mul_add` returns its rounding error exactly (the operands here
/// are differences of `f32` values, so neither the product nor its error is near the range in
/// which `f64` loses precision), so two products are equal exactly when their roundings and
/// their errors both are. The roundings are compared first because `mul_add` is a software
/// call on a target without FMA, and a point off the line almost never rounds to its product.
#[expect(
    clippy::float_cmp,
    reason = "exact equality of exact products is the whole test; a margin would call a sliver \
              with an area a line"
)]
fn exact_products_equal(a: (f64, f64), b: (f64, f64)) -> bool {
    let (left, right) = (a.0 * a.1, b.0 * b.1);
    left == right && a.0.mul_add(a.1, -left) == b.0.mul_add(b.1, -right)
}

/// One subpath's extent within a path's command list.
///
/// Visible to [`crate::sub_pixel`], which asks the same question of the same subpaths one rule
/// along — a shape thinner than a pixel rather than one with no extent at all — and would
/// otherwise carry a second walk that could disagree with this one about where a subpath ends.
pub(crate) struct Extent {
    /// Index of the subpath's first command.
    first: usize,
    /// Index one past its last command.
    last: usize,
    /// The corner with the smallest coordinates, over control points as well as endpoints.
    pub(crate) min: Point,
    /// The corner with the largest.
    pub(crate) max: Point,
    /// Where a subpath opened by a segment after an `h`, with no `m` of its own, begins: the
    /// point its first segment starts from, which is not among its own commands.
    reopened_at: Option<Point>,
}

impl Extent {
    pub(crate) fn range(&self) -> core::ops::Range<usize> {
        self.first..self.last
    }

    /// How this subpath lies where it encloses no area, or `None` if it encloses one.
    ///
    /// Exact equality rather than a tolerance, deliberately: a shape thinner than a pixel but
    /// not *flat* is drawn by this renderer at a coverage its area decides — exactly by
    /// [`crate::sub_pixel_bands`] where that applies, and otherwise by the rasteriser's own
    /// measure of the area. Either way it is §10.7.4's first documented departure and not a
    /// shape that has disappeared. Only a subpath with literally no area vanishes at every
    /// placement and every resolution, which is what makes this rule a statement about the
    /// geometry rather than about the scale.
    ///
    /// A subpath at one point is §8.5.3.3.1's degenerate subpath, and one on a line across the
    /// axes is found by [`on_one_line`] from its own points, `commands` being the path the
    /// extent was walked from (ADR 2330).
    #[expect(
        clippy::float_cmp,
        reason = "exactness is the rule: a margin here would claim a shape with an area is \
                  flat, and what such a shape gets is antialiased coverage rather than a mark"
    )]
    fn collapse(&self, commands: &[PathCommand]) -> Option<Collapse> {
        match (self.min.x == self.max.x, self.min.y == self.max.y) {
            (false, true) => Some(Collapse::Along(Axis::Horizontal)),
            (true, false) => Some(Collapse::Along(Axis::Vertical)),
            (true, true) => Some(Collapse::Point(self.min)),
            (false, false) => self.across(commands.get(self.range())?),
        }
    }

    /// The two furthest points of a subpath whose every point lies on one line, or `None`.
    ///
    /// The line is the one through the subpath's first point and the first point that differs
    /// from it, which exists because the extent is not a point.
    fn across(&self, commands: &[PathCommand]) -> Option<Collapse> {
        let points = || {
            self.reopened_at
                .into_iter()
                .chain(commands.iter().flat_map(command_points))
        };
        let first = points().next()?;
        let second = points().find(|p| *p != first)?;
        if !on_one_line(first, second, points()) {
            return None;
        }
        // Every point is on the line, so its position along `second − first` orders them.
        let (ux, uy) = (
            f64::from(second.x) - f64::from(first.x),
            f64::from(second.y) - f64::from(first.y),
        );
        let along = |p: &Point| {
            (f64::from(p.x) - f64::from(first.x)) * ux + (f64::from(p.y) - f64::from(first.y)) * uy
        };
        let from = points().min_by(|p, q| along(p).total_cmp(&along(q)))?;
        let to = points().max_by(|p, q| along(p).total_cmp(&along(q)))?;
        Some(Collapse::Across { from, to })
    }
}

/// Appends the rectangle a collapsed subpath marks, ISO 32000-2 §10.7.4.
///
/// The whole device pixels the collapsed axis passes through where `grid` says the transform
/// has axes to snap to, and otherwise a band of one device pixel centred on the line the
/// subpath lies along — which is where a stroke of that width down the same line would sit,
/// since §8.4.3.2 defines stroking as painting "all points whose perpendicular distance from
/// the path … is less than or equal to half the line width". The module comment argues why the
/// first is the clause and the second only its approximation.
fn append_mark(into: &mut Path, extent: &Extent, axis: Axis, thinnest: f32, grid: Option<Grid>) {
    let (min, max) = grid
        .and_then(|grid| snapped_span(extent, grid))
        .unwrap_or_else(|| band_span(extent, axis, thinnest));
    if !min.x.is_finite() || !min.y.is_finite() || !max.x.is_finite() || !max.y.is_finite() {
        return;
    }
    into.push(PathCommand::MoveTo(min));
    into.push(PathCommand::LineTo(Point::new(max.x, min.y)));
    into.push(PathCommand::LineTo(max));
    into.push(PathCommand::LineTo(Point::new(min.x, max.y)));
    into.push(PathCommand::Close);
}

/// The mark as a band of one device pixel about the subpath's own line.
///
/// The fallback of [`append_mark`]: what a mark is under a rotation or a shear, where the
/// collapsed axis crosses the pixel grid and the run of pixels it passes through is a staircase
/// rather than a rectangle.
fn band_span(extent: &Extent, axis: Axis, thinnest: f32) -> (Point, Point) {
    let half = thinnest / 2.0;
    match axis {
        Axis::Horizontal => (
            Point::new(extent.min.x, extent.min.y - half),
            Point::new(extent.max.x, extent.min.y + half),
        ),
        Axis::Vertical => (
            Point::new(extent.min.x - half, extent.min.y),
            Point::new(extent.min.x + half, extent.max.y),
        ),
    }
}

/// The mark as the run of whole device pixels the collapsed axis passes through, in path space.
///
/// ISO 32000-2 §10.7.4 identifies a pixel by flooring the coordinates of a point inside it, so
/// the collapsed axis at device coordinate `v` lies in the pixel row or column `floor(v)` and
/// the mark spans `floor(v)` to `floor(v) + 1`. The other axis keeps the subpath's own extent.
///
/// `None` where the subpath's device bounding box is degenerate in neither axis or in both,
/// which an axis-preserving transform of a subpath collapsed along exactly one axis cannot
/// produce — the guard is here because reading it off the arithmetic is cheaper than trusting
/// the caller's, and a mark placed on the wrong axis would be a line across the page.
#[expect(
    clippy::float_cmp,
    reason = "exactness is what identifies the collapsed axis: an axis-preserving transform \
              carries a shared coordinate to a shared coordinate through the same arithmetic, \
              and any inexactness here means the transform was not axis-preserving after all"
)]
fn snapped_span(extent: &Extent, grid: Grid) -> Option<(Point, Point)> {
    let a = grid.to_device.apply(extent.min);
    let b = grid.to_device.apply(extent.max);
    let (x0, x1) = (a.x.min(b.x), a.x.max(b.x));
    let (y0, y1) = (a.y.min(b.y), a.y.max(b.y));
    let (lo, hi) = if x0 == x1 && y0 != y1 {
        let column = x0.floor();
        (Point::new(column, y0), Point::new(column + 1.0, y1))
    } else if y0 == y1 && x0 != x1 {
        let row = y0.floor();
        (Point::new(x0, row), Point::new(x1, row + 1.0))
    } else {
        return None;
    };
    if !lo.x.is_finite() || !lo.y.is_finite() || !hi.x.is_finite() || !hi.y.is_finite() {
        return None;
    }
    // An axis-preserving transform carries a rectangle to a rectangle, so two opposite corners
    // of the pixel run are two opposite corners of the mark — whether the axes were kept or
    // exchanged by a quarter turn, which is why the sides are sorted rather than assumed.
    let (p, q) = (grid.inverse.apply(lo), grid.inverse.apply(hi));
    Some((
        Point::new(p.x.min(q.x), p.y.min(q.y)),
        Point::new(p.x.max(q.x), p.y.max(q.y)),
    ))
}

/// Walks a path's subpaths, reporting each one's bounding extent.
///
/// The bound is taken over *control* points as well as endpoints, which is exact for the
/// question being asked: a cubic Bézier lies within the convex hull of its four control
/// points, so control points sharing one y is the same statement as the curve having no
/// vertical extent, and no flattening is needed to see it.
///
/// A subpath ends at the next `m` or at a `h`, for the reason [`crate::degenerate`]'s own walk
/// gives: Table 58 makes `h` terminate the current subpath.
pub(crate) fn subpath_extents(path: &Path) -> impl Iterator<Item = Extent> + '_ {
    let commands = path.commands();
    let mut index = 0usize;
    // Where the most recent `m` put the current point. A subpath opened by a segment after an
    // `h` has no `m` of its own and starts here.
    let mut opened_at: Option<Point> = None;
    core::iter::from_fn(move || {
        loop {
            if index >= commands.len() {
                return None;
            }
            let first = index;
            let mut bounds: Option<(Point, Point)> = None;
            let include = |p: Point, bounds: &mut Option<(Point, Point)>| match bounds {
                Some((min, max)) => {
                    min.x = min.x.min(p.x);
                    min.y = min.y.min(p.y);
                    max.x = max.x.max(p.x);
                    max.y = max.y.max(p.y);
                }
                None => *bounds = Some((p, p)),
            };
            let mut opened = false;
            let mut reopened_at = None;
            while index < commands.len() {
                match commands[index] {
                    PathCommand::MoveTo(p) => {
                        if opened {
                            break;
                        }
                        opened = true;
                        opened_at = Some(p);
                        include(p, &mut bounds);
                    }
                    PathCommand::LineTo(p) => {
                        if !opened && let Some(start) = opened_at {
                            include(start, &mut bounds);
                            reopened_at = Some(start);
                        }
                        opened = true;
                        include(p, &mut bounds);
                    }
                    PathCommand::CurveTo(a, b, at) => {
                        if !opened && let Some(start) = opened_at {
                            include(start, &mut bounds);
                            reopened_at = Some(start);
                        }
                        opened = true;
                        include(a, &mut bounds);
                        include(b, &mut bounds);
                        include(at, &mut bounds);
                    }
                    PathCommand::Close => {
                        index = index.saturating_add(1);
                        break;
                    }
                }
                index = index.saturating_add(1);
            }
            if let Some((min, max)) = bounds {
                return Some(Extent {
                    first,
                    last: index,
                    min,
                    max,
                    reopened_at,
                });
            }
            // The commands just consumed named no point — a stray `h`. Keep walking rather
            // than ending the iteration, which would hide every subpath after it.
        }
    })
}

/// What a **clipping** path whose subpaths do not all enclose an area admits:
/// ISO 32000-2 §10.7.4.
///
/// Two fills, because the clause defines the region by a fill and the path's two kinds of
/// subpath are not filled under one rule: the subpaths that enclose an area take the operator's
/// own rule, and the marks §10.7.4 leaves for the ones that do not take the **non-zero** rule,
/// for [`split_collapsed_fill`]'s reason — the marks are separate shapes rather than parts of
/// one winding, and two of them that cross would cancel to a hole under the even-odd rule.
///
/// [`One`] is the case where that union is also one path under one rule, which is every clip
/// vocabulary this tree draws through; [`Union`] is the case where it is not, and a backend that
/// cannot compose two fills refuses it by name rather than admitting the wrong set of pixels.
///
/// [`One`]: ClipRegion::One
/// [`Union`]: ClipRegion::Union
#[derive(Debug, Clone, PartialEq)]
pub enum ClipRegion {
    /// One path scan-converted under one rule, which `vello`'s `push_clip_layer`, raster's
    /// `SceneBuilder::clip` and `tiny_skia::Mask` each state directly.
    One(Path, crate::paint::FillRule),
    /// The union of two fills, which no backend's clip vocabulary states as one clip.
    Union {
        /// The subpaths that enclose an area, under the operator's own rule.
        filled: (Path, crate::paint::FillRule),
        /// The marks §10.7.4 leaves for the subpaths that do not, under the non-zero rule.
        marks: Path,
    },
}

/// The region a **clipping** path encloses, where some subpath of it encloses no area:
/// ISO 32000-2 §10.7.4.
///
/// §10.7.4 defines a clip by the fill of the same path —
///
/// > For clipping, the clipping region consists of the set of pixels that would be included by
/// > a fill operation.
///
/// — §8.5.4 says the same thing from the operator's side, "For a given path definition, the same
/// area that would be filled by the f operator is the area that would be used for a clip", and
/// two paragraphs earlier §10.7.4 says what a fill of a flat rectangle includes:
///
/// > A zero-width or zero-height rectangle paints a line 1 pixel wide.
///
/// So a clipping path that rules a line admits that line's pixels, and a backend that scan
/// converts the path alone admits none of them: the two halves of one definition then disagree,
/// and `5 20.5 30 0 re f` paints a row of pixels that `5 20.5 30 0 re W n` refuses to let
/// through. [`split_collapsed_fill`] is what a fill asks; this is the same question asked by a
/// clip, so that the answer cannot differ between the two operations or between backends.
///
/// Returns `None` where the path states no collapsed subpath, which is every ordinary clip and
/// costs one memoised walk; otherwise the region, in the clipping path's **own** space, which is
/// where `to_device` maps from.
///
/// # When the two fills are one, and when they are not
///
/// Where every subpath collapsed, the region is the marks alone, under the non-zero rule whatever
/// the operator asked.
///
/// Where the path also encloses an area, appending a mark to that path *is* the union exactly
/// where the mark's own rectangle lies outside every area-enclosing subpath's rectangle: a point
/// outside a subpath's bounds has winding number zero and crossing count zero from that subpath,
/// so both rules read the appended rectangle as itself there. Under the even-odd rule a mark is
/// also kept apart from another mark it overlaps, for the crossing reason above. Where a mark
/// fails either test the region is a [`ClipRegion::Union`] and the caller composes the two fills
/// — exactly, on a backend that can, and by name on one that cannot. ADRs 1064 and 1231.
///
/// The rectangles are compared for a **positive** overlap rather than for a shared edge: a mark
/// whose rectangle merely touches an area subpath's has its whole interior outside that subpath,
/// which is what makes appending it the union.
#[must_use]
pub fn clip_region(
    path: &Path,
    rule: crate::paint::FillRule,
    to_device: Transform,
) -> Option<ClipRegion> {
    use crate::geom::Rect;
    use crate::paint::FillRule;

    let CollapsedFill { filled, marks } = split_collapsed_fill(path, to_device)?;
    if marks.is_empty() {
        return None;
    }
    // A path whose remaining commands name no point at all encloses nothing, so there the marks
    // are the whole of the region.
    if filled.is_empty() || filled.hull().is_none() {
        return Some(ClipRegion::One(marks, FillRule::NonZero));
    }
    let areas: Vec<Rect> = subpath_extents(&filled)
        .map(|extent| Rect::from_corners(extent.min, extent.max))
        .collect();
    let mut kept: Vec<Rect> = Vec::new();
    let mut appendable = true;
    for extent in subpath_extents(&marks) {
        let rect = Rect::from_corners(extent.min, extent.max);
        let meets_an_area = areas.iter().any(|area| overlap_has_area(*area, rect));
        let meets_a_mark =
            rule == FillRule::EvenOdd && kept.iter().any(|other| overlap_has_area(*other, rect));
        if meets_an_area || meets_a_mark {
            appendable = false;
            break;
        }
        kept.push(rect);
    }
    if !appendable {
        return Some(ClipRegion::Union {
            filled: (filled, rule),
            marks,
        });
    }
    let mut region = filled;
    region.extend(marks.commands());
    Some(ClipRegion::One(region, rule))
}

/// Whether two rectangles share more than a boundary.
///
/// [`crate::geom::Rect::intersection`] answers `Some` for rectangles that only touch, which is
/// the right answer for a bound and the wrong one here: a mark abutting an area subpath's
/// rectangle has every interior point of its own outside that subpath, so appending it to the
/// path adds exactly itself.
fn overlap_has_area(a: crate::geom::Rect, b: crate::geom::Rect) -> bool {
    a.min.x < b.max.x && b.min.x < a.max.x && a.min.y < b.max.y && b.min.y < a.max.y
}

#[cfg(test)]
mod tests {
    use super::{CollapsedFill, split_collapsed_fill};
    use crate::geom::{Path, PathCommand, Point, Transform};

    fn path(commands: &[PathCommand]) -> Path {
        let mut p = Path::new();
        for c in commands {
            p.push(*c);
        }
        p
    }

    /// `x y w 0 re`, the shape `issue4260_reduced.pdf` rules its grid with.
    fn zero_height_rectangle(y: f32) -> Path {
        path(&[
            PathCommand::MoveTo(Point::new(10.0, y)),
            PathCommand::LineTo(Point::new(110.0, y)),
            PathCommand::LineTo(Point::new(110.0, y)),
            PathCommand::LineTo(Point::new(10.0, y)),
            PathCommand::Close,
        ])
    }

    /// The rectangle a mark should be, given its two opposite corners.
    fn rectangle(min: Point, max: Point) -> [PathCommand; 5] {
        [
            PathCommand::MoveTo(min),
            PathCommand::LineTo(Point::new(max.x, min.y)),
            PathCommand::LineTo(max),
            PathCommand::LineTo(Point::new(min.x, max.y)),
            PathCommand::Close,
        ]
    }

    /// A clipping path that rules a line encloses that line's pixels, ISO 32000-2 §10.7.4.
    ///
    /// "For clipping, the clipping region consists of the set of pixels that would be included
    /// by a fill operation", so the region of a wholly collapsed path is exactly the marks the
    /// fill of it paints — and under the **non-zero** rule whatever the operator asked, because
    /// two marks that cross are two shapes rather than one winding.
    #[test]
    fn a_clip_whose_subpaths_all_collapse_is_its_marks_under_the_non_zero_rule() {
        let region = super::clip_region(
            &zero_height_rectangle(50.3),
            crate::paint::FillRule::EvenOdd,
            Transform::IDENTITY,
        )
        .expect("a region");
        let super::ClipRegion::One(region, rule) = region else {
            panic!("a wholly collapsed clipping path is one fill");
        };
        assert_eq!(rule, crate::paint::FillRule::NonZero);
        assert_eq!(
            region.commands(),
            rectangle(Point::new(10.0, 50.0), Point::new(110.0, 51.0))
        );
    }

    /// An ordinary clipping path is left exactly as it is, and costs no allocation.
    #[test]
    fn a_clip_that_encloses_an_area_is_not_rewritten() {
        let square = path(&rectangle(Point::new(0.0, 0.0), Point::new(10.0, 10.0)));
        assert!(
            super::clip_region(
                &square,
                crate::paint::FillRule::NonZero,
                Transform::IDENTITY
            )
            .is_none()
        );
    }

    /// A mark beside the area the rest of the path encloses is appended; one *meeting* it makes
    /// the region a union of two fills instead.
    ///
    /// §10.7.4 asks a mixed clipping path for the union of two fills taken under two different
    /// rules, and no backend's clip vocabulary states a union — so a mark joins the path only
    /// where the rest of the path has winding zero and crossing count zero, which is outside
    /// every area-enclosing subpath's own rectangle, and appending it there adds exactly itself.
    /// Where it does not, the two fills are handed over as two (ADRs 1064, 1231).
    #[test]
    fn a_mark_meeting_the_area_a_mixed_clip_encloses_makes_the_region_a_union() {
        let rule = crate::paint::FillRule::EvenOdd;
        let mut inside = path(&rectangle(Point::new(0.0, 0.0), Point::new(100.0, 100.0)));
        inside.extend(zero_height_rectangle(50.3).commands());
        let super::ClipRegion::Union { filled, marks } =
            super::clip_region(&inside, rule, Transform::IDENTITY).expect("a region")
        else {
            panic!("a mark crossing the square is not one fill");
        };
        assert_eq!(filled.1, rule, "the operator's own rule fills the area");
        assert_eq!(
            filled.0.commands(),
            rectangle(Point::new(0.0, 0.0), Point::new(100.0, 100.0))
        );
        assert_eq!(
            marks.commands(),
            rectangle(Point::new(10.0, 50.0), Point::new(110.0, 51.0))
        );

        let mut beside = path(&rectangle(Point::new(0.0, 0.0), Point::new(5.0, 5.0)));
        beside.extend(zero_height_rectangle(50.3).commands());
        let super::ClipRegion::One(region, _) =
            super::clip_region(&beside, rule, Transform::IDENTITY).expect("a region")
        else {
            panic!("a mark clear of the square is one fill");
        };
        let mut expected = rectangle(Point::new(0.0, 0.0), Point::new(5.0, 5.0)).to_vec();
        expected.extend_from_slice(&rectangle(Point::new(10.0, 50.0), Point::new(110.0, 51.0)));
        assert_eq!(region.commands(), expected.as_slice());
    }

    /// The two over-drops the substitute used to carry, each one measured against §10.7.4's own
    /// definition of the region.
    ///
    /// **A mark in the gap between two separated area subpaths is appended.** The bound the
    /// appending test asks about is each area subpath's own rectangle, because a point outside
    /// *that* has winding zero and crossing count zero from *that* subpath; one rectangle drawn
    /// around every area subpath at once spans the gap between them and dropped a mark the union
    /// contains.
    ///
    /// **A mark that only abuts an area subpath is appended.** Every interior point of a mark
    /// whose rectangle shares an edge with an area subpath's lies outside that subpath, so
    /// appending it still adds exactly itself.
    #[test]
    fn a_mark_between_two_areas_and_one_that_only_abuts_are_both_appended() {
        let rule = crate::paint::FillRule::NonZero;
        let mut gap = path(&rectangle(Point::new(0.0, 0.0), Point::new(5.0, 100.0)));
        gap.extend(&rectangle(Point::new(200.0, 0.0), Point::new(205.0, 100.0)));
        gap.extend(zero_height_rectangle(50.3).commands());
        let super::ClipRegion::One(region, _) =
            super::clip_region(&gap, rule, Transform::IDENTITY).expect("a region")
        else {
            panic!("a mark in the gap between two areas is one fill");
        };
        assert_eq!(
            region.commands().len(),
            15,
            "the two areas and the mark are all three in the region"
        );

        // The mark runs from x = 10 to x = 110 in row 50, so a square ending at x = 10 shares
        // its left edge and nothing else.
        let mut abutting = path(&rectangle(Point::new(0.0, 40.0), Point::new(10.0, 60.0)));
        abutting.extend(zero_height_rectangle(50.3).commands());
        let super::ClipRegion::One(region, _) =
            super::clip_region(&abutting, rule, Transform::IDENTITY).expect("a region")
        else {
            panic!("a mark that only abuts an area is one fill");
        };
        let mut expected = rectangle(Point::new(0.0, 40.0), Point::new(10.0, 60.0)).to_vec();
        expected.extend_from_slice(&rectangle(Point::new(10.0, 50.0), Point::new(110.0, 51.0)));
        assert_eq!(region.commands(), expected.as_slice());
    }

    /// The rule itself: a rectangle of no height marks the whole pixel row it lies in, at
    /// whatever fraction of a row it happens to sit — §10.7.4 identifies that row by flooring.
    #[test]
    fn a_rectangle_of_no_height_marks_the_pixel_row_it_lies_in() {
        let split = split_collapsed_fill(&zero_height_rectangle(50.3), Transform::IDENTITY)
            .expect("collapsed");
        assert!(split.filled.is_empty(), "nothing is left to fill");
        assert_eq!(
            split.marks.commands(),
            rectangle(Point::new(10.0, 50.0), Point::new(110.0, 51.0))
        );
    }

    /// The same shape in the other axis, which is the other half of a ruled grid.
    #[test]
    fn a_rectangle_of_no_width_marks_the_pixel_column_it_lies_in() {
        let column = path(&[
            PathCommand::MoveTo(Point::new(20.7, 0.0)),
            PathCommand::LineTo(Point::new(20.7, 80.0)),
            PathCommand::Close,
        ]);
        let split = split_collapsed_fill(&column, Transform::IDENTITY).expect("collapsed");
        assert_eq!(
            split.marks.commands(),
            rectangle(Point::new(20.0, 0.0), Point::new(21.0, 80.0))
        );
    }

    /// A line exactly on a pixel boundary belongs to the pixel above it, not to both.
    ///
    /// §10.7.4's pixel region "includes its lower but not its upper boundaries", so the shape at
    /// device y = 50 is in row 50. The band this rule used to lay down straddled the boundary
    /// and put half its ink in row 49, which is the placement dependence the snap removes.
    #[test]
    fn a_line_on_a_pixel_boundary_takes_the_pixel_above_it() {
        let split = split_collapsed_fill(&zero_height_rectangle(50.0), Transform::IDENTITY)
            .expect("collapsed");
        assert_eq!(
            split.marks.commands(),
            rectangle(Point::new(10.0, 50.0), Point::new(110.0, 51.0))
        );
    }

    /// The pixel is the *device's*, so the mark's thickness in path space follows the scale.
    ///
    /// Halving the scale makes one device pixel two path units, and the row the line lies in is
    /// row 25 rather than row 50 — which a test at one scale could not tell from a constant
    /// (trap 2).
    #[test]
    fn the_row_is_found_in_device_space_and_stated_back_in_the_paths() {
        let split = split_collapsed_fill(&zero_height_rectangle(50.6), Transform::scale(0.5, 0.5))
            .expect("collapsed");
        assert_eq!(
            split.marks.commands(),
            rectangle(Point::new(10.0, 50.0), Point::new(110.0, 52.0)),
            "device row 25 is path units 50 to 52"
        );
    }

    /// A quarter turn exchanges the axes and the mark follows, because the run of pixels is
    /// found in device space and carried back through the transform's own inverse.
    #[test]
    fn a_quarter_turn_marks_a_column_of_the_device_and_a_row_of_the_page() {
        let quarter = Transform::new(0.0, 1.0, -1.0, 0.0, 0.0, 0.0);
        assert!(quarter.preserves_axes(), "the case under test");
        let split = split_collapsed_fill(&zero_height_rectangle(50.3), quarter).expect("collapsed");
        assert_eq!(
            split.marks.commands(),
            rectangle(Point::new(10.0, 50.0), Point::new(110.0, 51.0)),
            "device column -51 is path y 50 to 51"
        );
    }

    /// Under a shear no device axis is a path axis, so the band stays — the stated fallback.
    #[test]
    fn a_shear_keeps_the_band_because_a_pixel_run_is_a_staircase() {
        let shear = Transform::new(1.0, 0.25, 0.0, 1.0, 0.0, 0.0);
        assert!(!shear.preserves_axes(), "the case under test");
        let thinnest = crate::paint::thinnest_line(shear).expect("invertible");
        let split = split_collapsed_fill(&zero_height_rectangle(50.3), shear).expect("collapsed");
        assert_eq!(
            split.marks.commands(),
            rectangle(
                Point::new(10.0, 50.3 - thinnest / 2.0),
                Point::new(110.0, 50.3 + thinnest / 2.0)
            )
        );
    }

    /// A transform that flattens the page onto a line still states a width, and gets the band.
    ///
    /// The one shape of transform where the snap declines and the arithmetic still has a number
    /// to work with: `max_stretch` is 1 along x, so there is a width, and the inverse does not
    /// exist, so there is no device pixel grid to floor a coordinate onto.
    #[test]
    fn a_singular_transform_that_still_stretches_keeps_the_band() {
        let flattened = Transform::scale(1.0, 0.0);
        assert_eq!(flattened.invert(), None, "the case under test");
        let split =
            split_collapsed_fill(&zero_height_rectangle(3.0), flattened).expect("collapsed");
        assert_eq!(
            split.marks.commands(),
            rectangle(Point::new(10.0, 2.5), Point::new(110.0, 3.5))
        );
    }

    /// The common case must cost nothing: a shape with area is not split at all.
    #[test]
    fn a_shape_that_encloses_an_area_is_not_split() {
        let square = path(&[
            PathCommand::MoveTo(Point::new(0.0, 0.0)),
            PathCommand::LineTo(Point::new(10.0, 0.0)),
            PathCommand::LineTo(Point::new(10.0, 10.0)),
            PathCommand::LineTo(Point::new(0.0, 10.0)),
            PathCommand::Close,
        ]);
        assert_eq!(split_collapsed_fill(&square, Transform::IDENTITY), None);
    }

    /// A shape thinner than a pixel is *not* this rule's business: it has an area, an
    /// antialiasing rasteriser gives it coverage, and this renderer's departure over such
    /// shapes is argued elsewhere. Only a flat one is a shape that cannot appear at all.
    #[test]
    fn a_shape_thinner_than_the_thinnest_line_is_left_alone() {
        let sliver = path(&[
            PathCommand::MoveTo(Point::new(0.0, 0.0)),
            PathCommand::LineTo(Point::new(10.0, 0.0)),
            PathCommand::LineTo(Point::new(10.0, 0.001)),
            PathCommand::LineTo(Point::new(0.0, 0.001)),
            PathCommand::Close,
        ]);
        assert_eq!(split_collapsed_fill(&sliver, Transform::IDENTITY), None);
    }

    /// A point is §8.5.3.3.1's degenerate subpath, "the single device pixel lying under that
    /// point": device `(5.5, 7.25)` under a scale of two is pixel `(5, 7)`, stated back in the
    /// path's own space (ADR 2330).
    #[test]
    fn a_single_point_subpath_marks_the_pixel_under_it() {
        let dot = path(&[
            PathCommand::MoveTo(Point::new(2.75, 3.625)),
            PathCommand::Close,
        ]);
        let split = split_collapsed_fill(&dot, Transform::scale(2.0, 2.0)).expect("collapsed");
        assert!(
            split.filled.is_empty(),
            "the point encloses no area of its own"
        );
        assert_eq!(
            split.marks.commands(),
            [
                PathCommand::MoveTo(Point::new(2.5, 3.5)),
                PathCommand::LineTo(Point::new(3.0, 3.5)),
                PathCommand::LineTo(Point::new(3.0, 4.0)),
                PathCommand::LineTo(Point::new(2.5, 4.0)),
                PathCommand::Close,
            ]
        );
    }

    /// Collinear points across the axes are a line, and the mark runs between the two furthest
    /// of them whatever order the subpath visits them in; a point a hair off the line is an
    /// area, which this rule leaves to the fill.
    #[test]
    fn collinear_points_across_the_axes_are_a_line_and_a_hair_off_it_is_not() {
        let on = path(&[
            PathCommand::MoveTo(Point::new(4.0, 2.0)),
            PathCommand::LineTo(Point::new(1.0, 0.5)),
            PathCommand::LineTo(Point::new(9.0, 4.5)),
            PathCommand::Close,
        ]);
        let split = split_collapsed_fill(&on, Transform::IDENTITY).expect("a line");
        assert!(split.filled.is_empty());
        let corners: Vec<Point> = split
            .marks
            .commands()
            .iter()
            .filter_map(|command| match command {
                PathCommand::MoveTo(p) | PathCommand::LineTo(p) => Some(*p),
                _ => None,
            })
            .collect();
        // Half a pixel along the unit normal `(−1, 2)/√5` each side of (1, 0.5) and (9, 4.5).
        let half = 0.5 / 5f32.sqrt();
        let expected = [
            Point::new(1.0 - half, 0.5 + 2.0 * half),
            Point::new(9.0 - half, 4.5 + 2.0 * half),
            Point::new(9.0 + half, 4.5 - 2.0 * half),
            Point::new(1.0 + half, 0.5 - 2.0 * half),
        ];
        assert_eq!(corners.len(), 4);
        for want in expected {
            assert!(
                corners
                    .iter()
                    .any(|corner| (corner.x - want.x).abs() < 1e-5
                        && (corner.y - want.y).abs() < 1e-5),
                "{want:?} is not among {corners:?}"
            );
        }
        let off = path(&[
            PathCommand::MoveTo(Point::new(4.0, 2.0)),
            PathCommand::LineTo(Point::new(1.0, 0.5)),
            PathCommand::LineTo(Point::new(9.0, 4.500_001)),
            PathCommand::Close,
        ]);
        assert_eq!(split_collapsed_fill(&off, Transform::IDENTITY), None);
    }

    /// The exact line test answers what exact arithmetic answers where `f64` products of the same
    /// differences round to one value: `(1 + 2⁻²⁷)²` and `1 + 2⁻²⁶` are one double once rounded,
    /// and differ by `2⁻⁵⁴`.
    #[test]
    fn the_line_test_compares_exact_products() {
        let e = 2f64.powi(-27);
        assert_eq!(
            ((1.0 + e) * (1.0 + e)).to_bits(),
            (1.0 + 2.0 * e).to_bits(),
            "the case under test"
        );
        assert!(!super::exact_products_equal(
            (1.0 + e, 1.0 + e),
            (1.0, 1.0 + 2.0 * e)
        ));
        assert!(super::exact_products_equal((3.0, 4.0), (6.0, 2.0)));
        assert_eq!(super::exact_difference(1.0e30, 1.0e-30), None);
        assert_eq!(super::exact_difference(3.5, 1.25), Some(2.25));
    }

    /// The two halves of a mixed path go their separate ways, and the surviving one keeps its
    /// commands exactly — it is still to be filled under the command's own rule.
    #[test]
    fn only_the_collapsed_subpaths_become_marks() {
        let mut mixed = path(&[
            PathCommand::MoveTo(Point::new(0.0, 0.0)),
            PathCommand::LineTo(Point::new(10.0, 0.0)),
            PathCommand::LineTo(Point::new(10.0, 10.0)),
            PathCommand::Close,
        ]);
        mixed.extend(zero_height_rectangle(20.0).commands());
        let CollapsedFill { filled, marks } =
            split_collapsed_fill(&mixed, Transform::IDENTITY).expect("collapsed");
        assert_eq!(
            filled.commands(),
            [
                PathCommand::MoveTo(Point::new(0.0, 0.0)),
                PathCommand::LineTo(Point::new(10.0, 0.0)),
                PathCommand::LineTo(Point::new(10.0, 10.0)),
                PathCommand::Close,
            ]
        );
        assert_eq!(marks.commands().len(), 5, "one rectangle");
    }

    /// A curve whose control points share one y has no vertical extent either, and the hull
    /// answers that without flattening anything.
    #[test]
    fn a_curve_flat_in_one_axis_collapses_too() {
        let flat = path(&[
            PathCommand::MoveTo(Point::new(0.0, 7.0)),
            PathCommand::CurveTo(
                Point::new(3.0, 7.0),
                Point::new(6.0, 7.0),
                Point::new(9.0, 7.0),
            ),
        ]);
        let split = split_collapsed_fill(&flat, Transform::IDENTITY).expect("collapsed");
        assert_eq!(split.marks.commands().len(), 5);
        assert!(split.filled.is_empty());
    }

    /// A transform that lengthens nothing leaves no width to state a mark in, and the split
    /// must decline rather than invent one of no thickness or of infinite thickness.
    #[test]
    fn a_transform_that_states_no_width_makes_no_mark() {
        for to_device in [
            Transform::scale(0.0, 0.0),
            Transform::new(f32::NAN, 0.0, 0.0, 1.0, 0.0, 0.0),
            Transform::new(f32::INFINITY, 0.0, 0.0, 1.0, 0.0, 0.0),
        ] {
            assert_eq!(
                split_collapsed_fill(&zero_height_rectangle(3.0), to_device),
                None
            );
        }
    }
}

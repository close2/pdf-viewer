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
//! The two clauses differ in one further respect, and it is the reason this rule is
//! implemented while §8.5.3.3.1's point is not. §8.5.3.3.1 calls its own answer
//! "device-dependent and not generally useful" in the same breath as stating it; §10.7.4
//! attaches no such hedge to a shape with a direction, and a producer that writes a
//! ten-thousand-unit rule means a line rather than an accident.
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
//! width centred whereever the shape happened to fall between two pixel centres, which is what
//! this module built until the three-hundred-and-sixty-eighth session. The difference is
//! visible rather than theoretical: an anti-aliasing rasteriser splits such a band across two
//! rows at every placement but one, so a page ruled with a grid of them — `issue4260_reduced.pdf`
//! is one — came out as a mixture of crisp lines and fuzzy grey double ones. A mark whose
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
//! for the reason [`crate::degenerate`] states a circle rather than trusting a round cap: a
//! decision either backend can make alone is a decision neither has made. It is also filled
//! separately from the path it came out of, under the non-zero rule, because a mark added to
//! the path itself would join that path's winding — under the even-odd rule a mark landing
//! inside a filled region would punch a hole in it.
//!
//! # What this does not cover
//!
//! A subpath that encloses no area while extending in *both* axes — three collinear points at
//! an angle, say. The test here is that the subpath's extent is exactly zero along one axis,
//! which makes the mark's direction the axis it is not zero along and needs no arithmetic
//! beyond a comparison; a diagonal one would need the flattened outline's collinearity and a
//! direction derived from it. No corpus document has been observed to write one, and a rule
//! whose shape is guessed is worse than a stated absence.

use crate::geom::{Path, PathCommand, Point, Transform};
use crate::paint::thinnest_line;

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
    /// a band centred on the subpath's own line. The module comment argues both.
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

    let mut filled = Path::new();
    let mut marks = Path::new();
    for extent in subpath_extents(path) {
        match extent.collapse() {
            Some(axis) => append_mark(&mut marks, &extent, axis, thinnest, grid),
            None => filled.extend(&path.commands()[extent.range()]),
        }
    }
    Some(CollapsedFill { filled, marks })
}

/// The device pixel grid, in both directions, for a placement transform that has axes to snap to.
#[derive(Debug, Clone, Copy)]
struct Grid {
    /// Path space onto the device.
    to_device: Transform,
    /// Its inverse, which states a run of device pixels back in the path's own space.
    inverse: Transform,
}

/// Whether any of a path's subpaths encloses no area because one of its extents is zero.
///
/// The predicate behind [`Path::collapses`], which memoises it. Kept separate from
/// [`split_collapsed_fill`] so that the common answer — no — costs one walk and no allocation.
pub(crate) fn any_subpath_collapses(path: &Path) -> bool {
    subpath_extents(path).any(|extent| extent.collapse().is_some())
}

/// The axis a subpath has no extent along.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Axis {
    /// Every point shares one y, so the mark is a horizontal rule.
    Horizontal,
    /// Every point shares one x, so the mark is a vertical rule.
    Vertical,
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
}

impl Extent {
    pub(crate) fn range(&self) -> core::ops::Range<usize> {
        self.first..self.last
    }

    /// The axis this subpath has collapsed along, or `None` if it encloses an area.
    ///
    /// Exact equality rather than a tolerance, deliberately: a shape thinner than a pixel but
    /// not *flat* is drawn by this renderer at a coverage its area decides — exactly by
    /// [`crate::sub_pixel_bands`] where that applies, and otherwise to the rasteriser's own
    /// quantum, which on `render-cpu` is a quarter of a pixel per axis (ADR 0474). Either way it
    /// is §10.7.4's first documented departure (ADR 0025's neighbour, argued in
    /// `doc/todo/_scan-conversion.md`) and not a shape that has disappeared. Only a subpath
    /// with literally no extent vanishes at every placement and every resolution, which is
    /// what makes this rule a statement about the geometry rather than about the scale.
    ///
    /// A subpath collapsed along *both* axes is a point, which §8.5.3.3.1 governs and this
    /// renderer records as a departure rather than drawing; it is not a case for this rule,
    /// which would have no direction to lay a mark along.
    #[expect(
        clippy::float_cmp,
        reason = "exactness is the rule: a margin here would claim a shape with an area is \
                  flat, and what such a shape gets is antialiased coverage rather than a mark"
    )]
    fn collapse(&self) -> Option<Axis> {
        match (self.min.x == self.max.x, self.min.y == self.max.y) {
            (false, true) => Some(Axis::Horizontal),
            (true, false) => Some(Axis::Vertical),
            _ => None,
        }
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
                        }
                        opened = true;
                        include(p, &mut bounds);
                    }
                    PathCommand::CurveTo(a, b, at) => {
                        if !opened && let Some(start) = opened_at {
                            include(start, &mut bounds);
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
                });
            }
            // The commands just consumed named no point — a stray `h`. Keep walking rather
            // than ending the iteration, which would hide every subpath after it.
        }
    })
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

    /// A point is §8.5.3.3.1's case rather than this one, and this rule must not claim it:
    /// there is no axis to lay a mark along.
    #[test]
    fn a_single_point_subpath_is_not_this_rule() {
        let dot = path(&[
            PathCommand::MoveTo(Point::new(5.0, 5.0)),
            PathCommand::Close,
        ]);
        assert_eq!(split_collapsed_fill(&dot, Transform::IDENTITY), None);
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

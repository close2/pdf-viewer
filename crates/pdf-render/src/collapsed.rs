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

use crate::geom::{Path, PathCommand, Point};

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
    /// no extent in and exactly as long as the subpath is along the other.
    pub marks: Path,
}

/// Splits a fill path's area-less subpaths out of it, ISO 32000-2 §10.7.4.
///
/// `thinnest` is one device pixel expressed in the path's own space — [`thinnest_line`] —
/// because that is the space the returned geometry is in, exactly as a [`Stroke`]'s width is.
///
/// Returns `None` when there is nothing to separate, so that an ordinary path costs one pass
/// over its commands and no allocation. [`Path::collapses`] memoises that pass, which matters:
/// this question is asked once per fill command per strip the page is cut into.
///
/// [`thinnest_line`]: crate::paint::thinnest_line
/// [`Stroke`]: crate::paint::Stroke
#[must_use]
pub fn split_collapsed_fill(path: &Path, thinnest: f32) -> Option<CollapsedFill> {
    if !path.collapses() || !thinnest.is_finite() || thinnest <= 0.0 {
        return None;
    }

    let mut filled = Path::new();
    let mut marks = Path::new();
    for extent in subpath_extents(path) {
        match extent.collapse() {
            Some(axis) => append_mark(&mut marks, &extent, axis, thinnest),
            None => filled.extend(&path.commands()[extent.range()]),
        }
    }
    Some(CollapsedFill { filled, marks })
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
struct Extent {
    /// Index of the subpath's first command.
    first: usize,
    /// Index one past its last command.
    last: usize,
    /// The corner with the smallest coordinates, over control points as well as endpoints.
    min: Point,
    /// The corner with the largest.
    max: Point,
}

impl Extent {
    fn range(&self) -> core::ops::Range<usize> {
        self.first..self.last
    }

    /// The axis this subpath has collapsed along, or `None` if it encloses an area.
    ///
    /// Exact equality rather than a tolerance, deliberately: a shape thinner than a pixel but
    /// not *flat* is drawn by this renderer at the coverage its area implies, which is
    /// §10.7.4's first documented departure (ADR 0025's neighbour, argued in `oracle.rs`'s
    /// `CONTRADICTED_ANTIALIASED_EDGES`) and not a shape that has disappeared. Only a subpath
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

/// Appends the rectangle a collapsed subpath marks.
///
/// One device pixel thick, centred on the line the subpath lies along — the same placement a
/// stroke of that width down the same line would have, since §8.4.3.2 defines stroking as
/// painting "all points whose perpendicular distance from the path … is less than or equal to
/// half the line width".
fn append_mark(into: &mut Path, extent: &Extent, axis: Axis, thinnest: f32) {
    let half = thinnest / 2.0;
    let (min, max) = match axis {
        Axis::Horizontal => (
            Point::new(extent.min.x, extent.min.y - half),
            Point::new(extent.max.x, extent.min.y + half),
        ),
        Axis::Vertical => (
            Point::new(extent.min.x - half, extent.min.y),
            Point::new(extent.min.x + half, extent.max.y),
        ),
    };
    if !min.x.is_finite() || !min.y.is_finite() || !max.x.is_finite() || !max.y.is_finite() {
        return;
    }
    into.push(PathCommand::MoveTo(min));
    into.push(PathCommand::LineTo(Point::new(max.x, min.y)));
    into.push(PathCommand::LineTo(max));
    into.push(PathCommand::LineTo(Point::new(min.x, max.y)));
    into.push(PathCommand::Close);
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
fn subpath_extents(path: &Path) -> impl Iterator<Item = Extent> + '_ {
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
    use crate::geom::{Path, PathCommand, Point};

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

    /// The rule itself: a rectangle of no height becomes one of exactly one device pixel,
    /// centred where the shape was and as long as the shape is.
    #[test]
    fn a_rectangle_of_no_height_marks_a_line_one_pixel_thick() {
        let split = split_collapsed_fill(&zero_height_rectangle(50.0), 2.0).expect("collapsed");
        assert!(split.filled.is_empty(), "nothing is left to fill");
        assert_eq!(
            split.marks.commands(),
            [
                PathCommand::MoveTo(Point::new(10.0, 49.0)),
                PathCommand::LineTo(Point::new(110.0, 49.0)),
                PathCommand::LineTo(Point::new(110.0, 51.0)),
                PathCommand::LineTo(Point::new(10.0, 51.0)),
                PathCommand::Close,
            ]
        );
    }

    /// The same shape in the other axis, which is the other half of a ruled grid.
    #[test]
    fn a_rectangle_of_no_width_marks_a_line_across_the_other_axis() {
        let column = path(&[
            PathCommand::MoveTo(Point::new(20.0, 0.0)),
            PathCommand::LineTo(Point::new(20.0, 80.0)),
            PathCommand::Close,
        ]);
        let split = split_collapsed_fill(&column, 1.0).expect("collapsed");
        assert_eq!(
            split.marks.commands(),
            [
                PathCommand::MoveTo(Point::new(19.5, 0.0)),
                PathCommand::LineTo(Point::new(20.5, 0.0)),
                PathCommand::LineTo(Point::new(20.5, 80.0)),
                PathCommand::LineTo(Point::new(19.5, 80.0)),
                PathCommand::Close,
            ]
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
        assert_eq!(split_collapsed_fill(&square, 1.0), None);
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
        assert_eq!(split_collapsed_fill(&sliver, 1.0), None);
    }

    /// A point is §8.5.3.3.1's case rather than this one, and this rule must not claim it:
    /// there is no axis to lay a mark along.
    #[test]
    fn a_single_point_subpath_is_not_this_rule() {
        let dot = path(&[
            PathCommand::MoveTo(Point::new(5.0, 5.0)),
            PathCommand::Close,
        ]);
        assert_eq!(split_collapsed_fill(&dot, 1.0), None);
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
        let CollapsedFill { filled, marks } = split_collapsed_fill(&mixed, 1.0).expect("collapsed");
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
        let split = split_collapsed_fill(&flat, 1.0).expect("collapsed");
        assert_eq!(split.marks.commands().len(), 5);
        assert!(split.filled.is_empty());
    }

    /// A degenerate transform leaves nothing to state a width in, and the split must decline
    /// rather than invent a mark of no thickness or of infinite one.
    #[test]
    fn no_thickness_means_no_mark() {
        for thinnest in [0.0, -1.0, f32::INFINITY, f32::NAN] {
            assert_eq!(
                split_collapsed_fill(&zero_height_rectangle(3.0), thinnest),
                None
            );
        }
    }
}

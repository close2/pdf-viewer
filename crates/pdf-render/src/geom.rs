//! Geometric primitives.
//!
//! Coordinates are `f32`. PDF user space is nominally 1/72 inch per unit, and page
//! dimensions are bounded by the format at 14 400 units, so `f32` has ample
//! precision for positions while halving the memory traffic of a display list
//! relative to `f64` — which matters because a text-heavy page produces tens of
//! thousands of commands.

/// A point in a 2D coordinate space.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point {
    /// Horizontal coordinate.
    pub x: f32,
    /// Vertical coordinate.
    pub y: f32,
}

impl Point {
    /// Creates a point from its coordinates.
    #[must_use]
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

/// A width and height, in the same units as [`Point`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Size {
    /// Extent along the x axis.
    pub width: f32,
    /// Extent along the y axis.
    pub height: f32,
}

impl Size {
    /// Creates a size from its extents.
    #[must_use]
    pub const fn new(width: f32, height: f32) -> Self {
        Self { width, height }
    }
}

/// An axis-aligned rectangle, stored as its minimum and maximum corners.
///
/// A rectangle is *normalised* when `min.x <= max.x` and `min.y <= max.y`. PDF
/// rectangle arrays carry no such guarantee, so values originating from a document
/// must be normalised before use.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    /// Corner with the smaller coordinates in a normalised rectangle.
    pub min: Point,
    /// Corner with the larger coordinates in a normalised rectangle.
    pub max: Point,
}

impl Rect {
    /// Creates a rectangle from two opposing corners, normalising them.
    #[must_use]
    pub fn from_corners(a: Point, b: Point) -> Self {
        Self {
            min: Point::new(a.x.min(b.x), a.y.min(b.y)),
            max: Point::new(a.x.max(b.x), a.y.max(b.y)),
        }
    }

    /// Width of the rectangle. Zero for a degenerate rectangle.
    #[must_use]
    pub fn width(self) -> f32 {
        self.max.x - self.min.x
    }

    /// Height of the rectangle. Zero for a degenerate rectangle.
    #[must_use]
    pub fn height(self) -> f32 {
        self.max.y - self.min.y
    }

    /// Returns the smallest rectangle containing both inputs.
    #[must_use]
    pub fn union(self, other: Self) -> Self {
        Self {
            min: Point::new(self.min.x.min(other.min.x), self.min.y.min(other.min.y)),
            max: Point::new(self.max.x.max(other.max.x), self.max.y.max(other.max.y)),
        }
    }
}

/// A 2D affine transform.
///
/// The component order matches the PDF `cm` operator's six operands, so a matrix
/// read from a content stream maps across without reordering:
///
/// ```text
/// | a  b  0 |
/// | c  d  0 |
/// | e  f  1 |
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Transform {
    /// Row 1, column 1: x scale.
    pub a: f32,
    /// Row 1, column 2: y shear.
    pub b: f32,
    /// Row 2, column 1: x shear.
    pub c: f32,
    /// Row 2, column 2: y scale.
    pub d: f32,
    /// Row 3, column 1: x translation.
    pub e: f32,
    /// Row 3, column 2: y translation.
    pub f: f32,
}

impl Transform {
    /// The identity transform.
    pub const IDENTITY: Self = Self {
        a: 1.0,
        b: 0.0,
        c: 0.0,
        d: 1.0,
        e: 0.0,
        f: 0.0,
    };

    /// Creates a transform from the six components of a PDF matrix.
    #[expect(
        clippy::many_single_char_names,
        reason = "a..f are the specification's own names for the matrix components; \
                  renaming them would make this harder to review against ISO 32000-2"
    )]
    #[must_use]
    pub const fn new(a: f32, b: f32, c: f32, d: f32, e: f32, f: f32) -> Self {
        Self { a, b, c, d, e, f }
    }

    /// Creates a translation.
    #[must_use]
    pub const fn translate(tx: f32, ty: f32) -> Self {
        Self {
            a: 1.0,
            b: 0.0,
            c: 0.0,
            d: 1.0,
            e: tx,
            f: ty,
        }
    }

    /// Creates a scale about the origin.
    #[must_use]
    pub const fn scale(sx: f32, sy: f32) -> Self {
        Self {
            a: sx,
            b: 0.0,
            c: 0.0,
            d: sy,
            e: 0.0,
            f: 0.0,
        }
    }

    /// Returns `self` followed by `other`.
    ///
    /// Reading order matches application order: `a.then(b)` applies `a` first. This
    /// is the reverse of conventional matrix-product notation, chosen because
    /// content-stream interpretation always composes in application order and the
    /// inverted spelling is a persistent source of transform bugs.
    #[must_use]
    pub fn then(self, other: Self) -> Self {
        Self {
            a: self.a * other.a + self.b * other.c,
            b: self.a * other.b + self.b * other.d,
            c: self.c * other.a + self.d * other.c,
            d: self.c * other.b + self.d * other.d,
            e: self.e * other.a + self.f * other.c + other.e,
            f: self.e * other.b + self.f * other.d + other.f,
        }
    }

    /// Applies the transform to a point.
    #[must_use]
    pub fn apply(self, p: Point) -> Point {
        Point::new(
            self.a * p.x + self.c * p.y + self.e,
            self.b * p.x + self.d * p.y + self.f,
        )
    }

    /// Returns the signed area scale factor.
    ///
    /// Used to convert user-space line widths and flatness tolerances into device
    /// space. A determinant of zero denotes a degenerate transform that collapses
    /// all geometry to a line or point.
    #[must_use]
    pub fn determinant(self) -> f32 {
        self.a * self.d - self.b * self.c
    }
}

impl Default for Transform {
    fn default() -> Self {
        Self::IDENTITY
    }
}

/// A single path-construction step.
///
/// Curves are cubic Bézier only. PDF has no quadratic curve operator, and font
/// outlines that use quadratics (TrueType) are elevated to cubics during glyph
/// loading so that the rest of the pipeline handles exactly one curve type.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PathCommand {
    /// Starts a new subpath at the given point.
    MoveTo(Point),
    /// Adds a straight segment from the current point.
    LineTo(Point),
    /// Adds a cubic Bézier segment with two control points and an endpoint.
    CurveTo(Point, Point, Point),
    /// Closes the current subpath with a straight segment to its start.
    Close,
}

/// A sequence of path-construction steps.
///
/// A path carries no transform, paint, or fill rule; those live on the
/// [`Command`](crate::display_list::Command) that references it. This separation
/// lets one path be drawn repeatedly — filled, then stroked, then used as a clip —
/// without duplicating its geometry.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Path {
    commands: Vec<PathCommand>,
}

impl Path {
    /// Creates an empty path.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            commands: Vec::new(),
        }
    }

    /// Appends a command.
    pub fn push(&mut self, command: PathCommand) {
        self.commands.push(command);
    }

    /// Returns the path's commands in construction order.
    #[must_use]
    pub fn commands(&self) -> &[PathCommand] {
        &self.commands
    }

    /// Returns `true` if the path contains no commands.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }
}

#[cfg(test)]
#[expect(
    clippy::float_cmp,
    reason = "these assertions compare exactly representable values on purpose — the \
              y-flip must land precisely on row zero, and pixel extents are integers \
              — so an approximate comparison would weaken the test"
)]
mod tests {
    use super::{Point, Transform};

    /// `then` must read in application order — the property the doc comment promises,
    /// and the one whose violation produces transform bugs that are hard to localise.
    #[test]
    fn then_applies_self_before_other() {
        let scale_then_translate = Transform::scale(2.0, 2.0).then(Transform::translate(10.0, 0.0));

        let p = scale_then_translate.apply(Point::new(1.0, 0.0));

        // Scale first (1 -> 2), then translate (2 -> 12). The other order would give 22.
        assert_eq!(p.x, 12.0);
        assert_eq!(p.y, 0.0);
    }

    #[test]
    fn identity_leaves_points_unchanged() {
        let p = Point::new(3.5, -7.25);
        assert_eq!(Transform::IDENTITY.apply(p), p);
    }

    #[test]
    fn composing_with_identity_is_a_no_op() {
        let t = Transform::new(2.0, 0.5, -0.5, 3.0, 7.0, -1.0);
        assert_eq!(t.then(Transform::IDENTITY), t);
        assert_eq!(Transform::IDENTITY.then(t), t);
    }

    #[test]
    fn determinant_detects_a_collapsing_transform() {
        assert_eq!(Transform::scale(3.0, 4.0).determinant(), 12.0);
        assert_eq!(Transform::scale(0.0, 4.0).determinant(), 0.0);
    }

    #[test]
    fn rect_from_corners_normalises_inverted_input() {
        // PDF rectangle arrays carry no ordering guarantee, so this is a real case.
        let r = super::Rect::from_corners(Point::new(10.0, 20.0), Point::new(0.0, 5.0));
        assert_eq!(r.min, Point::new(0.0, 5.0));
        assert_eq!(r.max, Point::new(10.0, 20.0));
        assert_eq!(r.width(), 10.0);
        assert_eq!(r.height(), 15.0);
    }
}

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
    /// A determinant of zero denotes a degenerate transform that collapses all geometry to
    /// a line or point. It is *not* the factor a length is scaled by — see
    /// [`Self::max_stretch`], which is, and which a shear separates from this one.
    #[must_use]
    pub fn determinant(self) -> f32 {
        self.a * self.d - self.b * self.c
    }

    /// Returns the largest factor by which this transform lengthens a vector.
    ///
    /// This is the linear part's larger singular value. It is what a line width is
    /// multiplied by to reach its widest extent in the target space, which is the question
    /// ISO 32000-2 §8.4.3.2 asks — "the effect produced in device space depends on the
    /// current transformation matrix … If the CTM specifies scaling by different factors in
    /// the horizontal and vertical dimensions, the thickness of stroked lines in device
    /// space shall vary according to their orientation."
    ///
    /// `determinant().abs().sqrt()` is the *geometric mean* of the two singular values and
    /// agrees with this only where they are equal — every similarity transform, which is
    /// what a page transform is. A shear separates them without changing the determinant at
    /// all, so a length bound derived from the determinant can be arbitrarily too small.
    #[must_use]
    pub fn max_stretch(self) -> f32 {
        // For the linear part M, the singular values are the square roots of the
        // eigenvalues of MᵗM, whose trace is `sum` and whose determinant is `det²`. The
        // quadratic's larger root is therefore (sum + √(sum² − 4·det²)) / 2, and the
        // discriminant is non-negative for any real matrix — clamped anyway, because
        // rounding can take it a hair below zero when the two roots coincide.
        let sum = self.a * self.a + self.b * self.b + self.c * self.c + self.d * self.d;
        let determinant = self.determinant();
        let discriminant = (sum * sum - 4.0 * determinant * determinant).max(0.0);
        f32::midpoint(sum, discriminant.sqrt()).max(0.0).sqrt()
    }

    /// Returns the transform that undoes this one.
    ///
    /// `None` when the transform collapses geometry to a line or a point, which has no
    /// inverse. Needed wherever a question is asked in one space about a shape defined in
    /// another — which region of a pattern's own coordinates a filled path covers, for
    /// instance.
    #[must_use]
    pub fn invert(self) -> Option<Self> {
        let determinant = self.determinant();
        if !determinant.is_finite() || determinant == 0.0 {
            return None;
        }
        Some(Self {
            a: self.d / determinant,
            b: -self.b / determinant,
            c: -self.c / determinant,
            d: self.a / determinant,
            e: (self.c * self.f - self.e * self.d) / determinant,
            f: (self.e * self.b - self.a * self.f) / determinant,
        })
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

    /// Removes the last command, if any.
    ///
    /// ISO 32000-2 §8.5.3.3.1 disregards a path's trailing single-point subpath, and Table
    /// 58 has one `m` override the one before it: both are stated as the *removal* of a
    /// command already appended, so a path has to be able to un-append one.
    pub fn pop(&mut self) {
        self.commands.pop();
    }

    /// Replaces the last command, leaving an empty path empty.
    pub fn replace_last(&mut self, command: PathCommand) {
        if let Some(last) = self.commands.last_mut() {
            *last = command;
        }
    }

    /// Appends a run of commands unchanged.
    ///
    /// Distinct from [`Self::extend_transformed`], which maps every point: this is for
    /// copying part of one path into another in the space both already share.
    pub fn extend(&mut self, commands: &[PathCommand]) {
        self.commands.extend_from_slice(commands);
    }

    /// Returns the path's commands in construction order.
    #[must_use]
    pub fn commands(&self) -> &[PathCommand] {
        &self.commands
    }

    /// Appends another path's commands, with every point mapped by `transform`.
    ///
    /// A path normally travels with a transform beside it on the command that references
    /// it, which is cheaper and keeps the geometry shared. This exists for the two cases
    /// where that is not expressive enough, and both are text (ISO 32000-2 §9.3.6): a glyph
    /// outline has to reach *user* space before it can be stroked, because a line width is
    /// stated in that space and a stroke's width is in its path's; and the clipping render
    /// modes combine every glyph of a text object into one path, which cannot carry the
    /// glyphs' several transforms.
    ///
    /// Baking a transform into the points is exact here rather than an approximation: an
    /// affine map takes a cubic Bézier to the cubic Bézier through its mapped control
    /// points, so no curve is flattened and no segment is added.
    pub fn extend_transformed(&mut self, other: &Self, transform: Transform) {
        self.commands.reserve(other.commands.len());
        for command in &other.commands {
            self.commands.push(match *command {
                PathCommand::MoveTo(p) => PathCommand::MoveTo(transform.apply(p)),
                PathCommand::LineTo(p) => PathCommand::LineTo(transform.apply(p)),
                PathCommand::CurveTo(a, b, c) => {
                    PathCommand::CurveTo(transform.apply(a), transform.apply(b), transform.apply(c))
                }
                PathCommand::Close => PathCommand::Close,
            });
        }
    }

    /// Returns `true` if the path contains no commands.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }

    /// The smallest rectangle containing the path once `transform` has been applied, or
    /// `None` for a path that names no point.
    ///
    /// # It is a bound and never an underestimate
    ///
    /// The corners come from the *control* points rather than from the curve, and a cubic
    /// Bézier lies inside the convex hull of its four control points, so a curve that bulges
    /// less than its handles gives a rectangle larger than the ink. That is the direction a
    /// caller asking "can this command mark these rows" needs: a bound that is too large
    /// costs work, and one that is too small loses pixels.
    ///
    /// The exact bound would need each segment's extrema, which is a root-finding problem per
    /// curve and buys nothing for the two callers this has — deciding which horizontal strip
    /// of a target a command belongs to (ADR 0137), and estimating what a strip costs.
    #[must_use]
    pub fn bounds(&self, transform: Transform) -> Option<Rect> {
        let mut bounds: Option<Rect> = None;
        let mut extend = |point: Point| {
            let point = transform.apply(point);
            bounds = Some(bounds.map_or(Rect::from_corners(point, point), |rect| {
                rect.union(Rect::from_corners(point, point))
            }));
        };
        for command in &self.commands {
            match *command {
                PathCommand::MoveTo(p) | PathCommand::LineTo(p) => extend(p),
                PathCommand::CurveTo(a, b, c) => {
                    extend(a);
                    extend(b);
                    extend(c);
                }
                PathCommand::Close => {}
            }
        }
        bounds
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

    /// Every point of every command kind is mapped, and `Close` carries no point.
    ///
    /// The failure this pins is a partial transform — mapping a curve's endpoint and not
    /// its control points, which draws a glyph that is *nearly* right and is exactly the
    /// kind of defect no metric in this tree can see.
    #[test]
    fn extend_transformed_maps_every_point_of_every_command() {
        use super::{Path, PathCommand};

        let mut glyph = Path::new();
        glyph.push(PathCommand::MoveTo(Point::new(1.0, 0.0)));
        glyph.push(PathCommand::LineTo(Point::new(2.0, 0.0)));
        glyph.push(PathCommand::CurveTo(
            Point::new(3.0, 0.0),
            Point::new(4.0, 0.0),
            Point::new(5.0, 0.0),
        ));
        glyph.push(PathCommand::Close);

        let mut combined = Path::new();
        combined.extend_transformed(&glyph, Transform::scale(10.0, 1.0));
        // A second glyph under a different transform: the case a single path plus a single
        // transform cannot express, which is why this method exists.
        combined.extend_transformed(&glyph, Transform::translate(100.0, 0.0));

        assert_eq!(
            combined.commands(),
            [
                PathCommand::MoveTo(Point::new(10.0, 0.0)),
                PathCommand::LineTo(Point::new(20.0, 0.0)),
                PathCommand::CurveTo(
                    Point::new(30.0, 0.0),
                    Point::new(40.0, 0.0),
                    Point::new(50.0, 0.0)
                ),
                PathCommand::Close,
                PathCommand::MoveTo(Point::new(101.0, 0.0)),
                PathCommand::LineTo(Point::new(102.0, 0.0)),
                PathCommand::CurveTo(
                    Point::new(103.0, 0.0),
                    Point::new(104.0, 0.0),
                    Point::new(105.0, 0.0)
                ),
                PathCommand::Close,
            ]
        );
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

#[cfg(test)]
mod invert_tests {
    use super::{Point, Transform};

    /// Inverting and reapplying must return the original point.
    ///
    /// Checked with a transform that scales, shears, rotates and translates at once,
    /// because an inverse that is wrong only in its translation still passes on a pure
    /// scale — which is the mistake that is easy to make and hard to see.
    #[test]
    fn a_transform_composed_with_its_inverse_is_the_identity() {
        let transform = Transform::new(2.0, 0.5, -0.25, 3.0, 17.0, -9.0);
        let inverse = transform.invert().expect("this transform is invertible");

        for point in [
            Point::new(0.0, 0.0),
            Point::new(1.0, 0.0),
            Point::new(0.0, 1.0),
            Point::new(-13.5, 42.25),
        ] {
            let round_trip = inverse.apply(transform.apply(point));
            assert!(
                (round_trip.x - point.x).abs() < 1e-3 && (round_trip.y - point.y).abs() < 1e-3,
                "{point:?} became {round_trip:?}"
            );
        }
    }

    /// `max_stretch` is the largest factor a vector's length is multiplied by.
    ///
    /// Checked against the definition rather than against the closed form: the maximum over
    /// a fine sweep of directions must reach the returned value and never exceed it. That is
    /// the property callers rely on, and it catches a sign or a swapped root in the
    /// quadratic, which a spot value would not.
    #[test]
    fn max_stretch_bounds_every_direction() {
        let cases = [
            Transform::scale(3.0, 3.0),
            Transform::scale(4.0, 0.25),
            // A rotation by 30 degrees: every direction is stretched by exactly one.
            Transform::new(0.866_025_4, 0.5, -0.5, 0.866_025_4, 0.0, 0.0),
            // A pure shear, whose determinant is 1 and whose longest direction is not.
            Transform::new(1.0, 0.0, 2.0, 1.0, 0.0, 0.0),
            Transform::new(2.0, 0.5, -0.25, 3.0, 10.0, -4.0),
        ];
        for transform in cases {
            let bound = transform.max_stretch();
            let mut worst = 0.0_f32;
            for step in 0..3600 {
                #[expect(
                    clippy::cast_precision_loss,
                    reason = "test code: the loop counter is under four thousand, which f32 \
                              represents exactly"
                )]
                let angle = step as f32 * std::f32::consts::TAU / 3600.0;
                // Translation is irrelevant to a length, so the vector's image is the
                // difference of two mapped points.
                let origin = transform.apply(Point::new(0.0, 0.0));
                let tip = transform.apply(Point::new(angle.cos(), angle.sin()));
                worst = worst.max((tip.x - origin.x).hypot(tip.y - origin.y));
            }
            assert!(
                worst <= bound * 1.000_1,
                "{transform:?}: a direction stretched by {worst}, above the bound {bound}"
            );
            assert!(
                worst >= bound * 0.999,
                "{transform:?}: the bound {bound} is not reached; the worst was {worst}"
            );
        }
    }

    /// A shear separates the two singular values without changing the determinant.
    ///
    /// The reason `max_stretch` exists rather than `determinant().abs().sqrt()`, and the
    /// reason the comment that used to claim the determinant was "the safe way round" for a
    /// stroke's margin was wrong: here the determinant says 1 and a length triples.
    #[test]
    fn a_shear_stretches_more_than_its_determinant_says() {
        let shear = Transform::new(1.0, 0.0, 2.0, 1.0, 0.0, 0.0);
        assert!((shear.determinant() - 1.0).abs() < 1e-6);
        assert!(
            shear.max_stretch() > 2.0,
            "a shear of 2 stretches by {}",
            shear.max_stretch()
        );
    }

    #[test]
    fn a_collapsing_transform_has_no_inverse() {
        // Both rows the same: everything maps onto one line.
        assert!(
            Transform::new(1.0, 1.0, 2.0, 2.0, 0.0, 0.0)
                .invert()
                .is_none()
        );
        assert!(
            Transform::new(0.0, 0.0, 0.0, 0.0, 5.0, 5.0)
                .invert()
                .is_none()
        );
    }
}

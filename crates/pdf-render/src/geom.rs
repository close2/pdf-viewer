//! Geometric primitives.
//!
//! Coordinates are `f32`. PDF user space is nominally 1/72 inch per unit, and page
//! dimensions are bounded by the format at 14 400 units, so `f32` has ample
//! precision for positions while halving the memory traffic of a display list
//! relative to `f64` — which matters because a text-heavy page produces tens of
//! thousands of commands.

/// The length of the vector `(dx, dy)`, computed so that every platform agrees on it.
///
/// **Not `f32::hypot`, and the difference decides pixels.** `hypot` is a libm function, and
/// Rust promises only that it *approximates* the Euclidean distance: an implementation may be
/// out in the last places, and two machines' libms may be out differently. Multiplication,
/// addition and `sqrt` are IEEE 754 operations, each correctly rounded, so this expression is
/// the same number everywhere.
///
/// That distinction is idle in arithmetic and load-bearing here, because this crate turns
/// lengths into **decisions taken at a threshold**: whether an image is magnified
/// ([`crate::paint::Image::is_smoothed`]), how far its grid is reduced, whether a miter has
/// passed §8.4.3.5's limit, whether a dash has no length at all under §8.5.3.2. A comparison
/// against an exact boundary flips on the last place, and `render-cpu` is `render-gpu`'s
/// oracle — so a threshold that answered differently on a different machine would be a
/// disagreement about every image drawn at exactly 1:1, on a page nobody had changed.
///
/// **Found by Miri**, whose deliberate non-determinism for imprecise floating-point operations
/// failed four of this crate's tests on the CI runner and a different three on the machine that
/// wrote them (ADR 0189).
///
/// The cost is `hypot`'s one real advantage: `dx * dx` overflows above about 1.8e19 and
/// underflows to zero below about 1e-19, where `hypot` scales to avoid both. Page geometry is
/// bounded by the format at 14 400 units and every caller here already guards a zero or
/// non-finite length, so neither end is reachable from a document.
#[must_use]
pub(crate) fn length(dx: f32, dy: f32) -> f32 {
    (dx * dx + dy * dy).sqrt()
}

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

    /// The rectangle grown by `reach` on every side.
    ///
    /// Negative values shrink it and may cross the corners over, which is why the result is
    /// re-normalised: a bound that inverted itself would compare as containing everything.
    #[must_use]
    pub fn grown(self, reach: f32) -> Self {
        Self::from_corners(
            Point::new(self.min.x - reach, self.min.y - reach),
            Point::new(self.max.x + reach, self.max.y + reach),
        )
    }

    /// The smallest axis-aligned rectangle containing this one's image under `transform`.
    ///
    /// An affine map takes a rectangle to a parallelogram, so the four mapped corners are
    /// what bound it — and the box of the image contains the image of the box, which is what
    /// lets a bound be computed in one space and used in another.
    #[must_use]
    pub fn mapped(self, transform: Transform) -> Self {
        let corners = [
            transform.apply(self.min),
            transform.apply(Point::new(self.max.x, self.min.y)),
            transform.apply(self.max),
            transform.apply(Point::new(self.min.x, self.max.y)),
        ];
        let mut mapped = Self::from_corners(corners[0], corners[0]);
        for corner in corners {
            mapped = mapped.union(Self::from_corners(corner, corner));
        }
        mapped
    }

    /// The rectangle both cover, or `None` where they do not meet.
    ///
    /// A rectangle that merely *touches* another along an edge or at a corner is an
    /// intersection rather than nothing, because §7.9.5's own NOTE is that "[r]ectangles can
    /// have a width of zero or height of zero" — a degenerate result is still a place, and
    /// discarding it would answer "nowhere" for content that lies exactly on a boundary.
    #[must_use]
    pub fn intersection(self, other: Self) -> Option<Self> {
        let min = Point::new(self.min.x.max(other.min.x), self.min.y.max(other.min.y));
        let max = Point::new(self.max.x.min(other.max.x), self.max.y.min(other.max.y));
        (min.x <= max.x && min.y <= max.y).then_some(Self { min, max })
    }

    /// Whether every point of `other` lies within this rectangle, edges included.
    #[must_use]
    pub fn contains(self, other: Self) -> bool {
        self.min.x <= other.min.x
            && self.min.y <= other.min.y
            && self.max.x >= other.max.x
            && self.max.y >= other.max.y
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

    /// Whether this transform maps the axes onto the axes: a scale and a translation, or a
    /// quarter turn.
    ///
    /// What it is asked for is whether an axis-aligned shape stays axis-aligned — which is what
    /// [`crate::collapsed`] needs before it can snap a mark to a device pixel, what
    /// [`crate::sub_pixel`] needs before it can stretch one into a pixel line, and what
    /// [`Self::bounds`] relies on to map a hull instead of walking it. The two shapes are the two
    /// ways a 2×2 matrix can have a zero in each row and column.
    ///
    /// An off-diagonal that is *exactly* zero is the property being asked about: a small one
    /// shears, and a caller relying on this needs the strict answer.
    ///
    /// **The doc comment here used to be the second half of [`Self::max_stretch`]'s**, spliced
    /// into the middle of this one and leaving that function's last sentence stranded above its
    /// own signature. Found in the three-hundred-and-eighty-ninth session by a round that needed
    /// to read it.
    #[must_use]
    pub fn preserves_axes(self) -> bool {
        (self.b == 0.0 && self.c == 0.0) || (self.a == 0.0 && self.d == 0.0)
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

    /// Returns the smallest factor by which this transform lengthens a vector.
    ///
    /// The linear part's *smaller* singular value, and [`Self::max_stretch`]'s companion. Where
    /// that one answers how wide a stroke of a given width can become — §8.4.3.2's question —
    /// this one answers how narrow, which is the question a caller asks when it needs a shape
    /// that is at least one device pixel across **whichever way it runs**:
    /// [`crate::substitute_width`] is stated as its reciprocal.
    ///
    /// For a similarity — which is what a page transform is — the two are equal, and their
    /// product is `determinant().abs()` for any transform at all.
    #[must_use]
    pub fn min_stretch(self) -> f32 {
        // The same quadratic [`Self::max_stretch`] derives, taking its smaller root. The
        // clamps are that function's and for its reasons: rounding can put the discriminant a
        // hair below zero when the roots coincide, and the root itself a hair below zero when
        // the transform is singular.
        let sum = self.a * self.a + self.b * self.b + self.c * self.c + self.d * self.d;
        let determinant = self.determinant();
        let discriminant = (sum * sum - 4.0 * determinant * determinant).max(0.0);
        f32::midpoint(sum, -discriminant.sqrt()).max(0.0).sqrt()
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
#[derive(Debug, Default)]
pub struct Path {
    commands: Vec<PathCommand>,
    /// The untransformed hull of [`Self::bounds`], built on first use.
    ///
    /// A glyph outline is shared through an `Arc` and asked for its bounds once per strip the
    /// rasteriser cuts the page into, and walking forty control points each time was **17.6%
    /// of a dense page's rasterisation** — 541 300 calls over twenty renders of ISO 32000-2's
    /// page 101, measured with `callgrind_annotate --tree=caller` in session 163. The walk
    /// happens once now and every later call maps this rectangle.
    hull: std::sync::OnceLock<Option<Rect>>,
    /// Whether any subpath encloses no area, built on first use.
    ///
    /// Memoised for the same reason [`Self::hull`] is and against the same measurement: the
    /// question is asked once per fill command per strip the rasteriser cuts the page into,
    /// and a dense text page is five thousand fills of forty control points each. The answer
    /// is a property of the commands and not of the transform — an affine map takes a shape
    /// with no extent along an axis to a shape with no extent along its image — so one walk
    /// answers it at every scale the page is ever drawn at.
    collapses: std::sync::OnceLock<bool>,
    /// The narrowest side of any axis-aligned rectangle among the subpaths, built on first use.
    ///
    /// Memoised for the reason [`Self::collapses`] is, and the answer is a property of the
    /// commands for the same reason: an affine map *scales* an extent, so which rectangle is the
    /// narrowest never changes and the only thing a transform decides is whether that narrowest
    /// side is under a device pixel — one multiplication and one comparison, in front of a walk.
    narrowest_rectangle: std::sync::OnceLock<Option<f32>>,
}

impl Clone for Path {
    /// Clones the commands and *not* the caches, which the clone will rebuild if it is asked.
    ///
    /// Copying them would be correct — both are functions of the commands — and `OnceLock`
    /// is not `Clone`, so this says the cheap true thing rather than reaching for `get()`.
    fn clone(&self) -> Self {
        Self {
            commands: self.commands.clone(),
            hull: std::sync::OnceLock::new(),
            collapses: std::sync::OnceLock::new(),
            narrowest_rectangle: std::sync::OnceLock::new(),
        }
    }
}

impl PartialEq for Path {
    /// Two paths are equal when they state the same commands. The cache is a memo of that and
    /// never a second fact about it.
    fn eq(&self, other: &Self) -> bool {
        self.commands == other.commands
    }
}

impl Path {
    /// Creates an empty path.
    #[must_use]
    pub fn new() -> Self {
        Self {
            commands: Vec::new(),
            hull: std::sync::OnceLock::new(),
            collapses: std::sync::OnceLock::new(),
            narrowest_rectangle: std::sync::OnceLock::new(),
        }
    }

    /// The narrowest side of any axis-aligned rectangle among this path's subpaths, in the path's
    /// own space, ISO 32000-2 §10.7.4.
    ///
    /// `None` where no subpath is such a rectangle. Where one is, comparing this against the
    /// device's own pixel says whether [`sub_pixel_bands`] has anything to do, which is the
    /// question a fill command asks once per strip and which must not walk the path each time.
    ///
    /// [`sub_pixel_bands`]: crate::sub_pixel::sub_pixel_bands
    #[must_use]
    pub fn narrowest_rectangle(&self) -> Option<f32> {
        *self
            .narrowest_rectangle
            .get_or_init(|| crate::sub_pixel::narrowest_rectangle(self))
    }

    /// Whether any subpath of this path encloses no area, ISO 32000-2 §10.7.4.
    ///
    /// True where a subpath's extent along one axis is exactly zero while the other is not —
    /// a rectangle written with a zero side, a run of points along one line — which is the
    /// shape an antialiasing rasteriser computes no coverage for at any resolution. What is
    /// then drawn instead is [`split_collapsed_fill`]'s business; this only says whether to
    /// ask, and it is memoised because a fill command is asked once per strip.
    ///
    /// [`split_collapsed_fill`]: crate::collapsed::split_collapsed_fill
    #[must_use]
    pub fn collapses(&self) -> bool {
        *self
            .collapses
            .get_or_init(|| crate::collapsed::any_subpath_collapses(self))
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
        // The untransformed hull, walked once and kept. Mapping it is exact wherever the
        // transform keeps the axes: `a·x + e` is monotone in `x`, so the same control point
        // attains the same extreme and the same arithmetic produces the same `f32`. Under a
        // shear or a general rotation it is not, so the walk runs — which is the ordinary case
        // for a page and the rare one for a glyph.
        if transform.preserves_axes() {
            return Some(self.hull()?.mapped(transform));
        }
        self.walked(transform)
    }

    /// The path's untransformed hull, walked once and kept.
    ///
    /// Public because a caller that wants to grow the hull *before* mapping it — which is
    /// what a stroke's reach is, since a line width is stated in the path's own space — would
    /// otherwise have to map, grow and hope the transform was a similarity.
    #[must_use]
    pub fn hull(&self) -> Option<Rect> {
        *self.hull.get_or_init(|| self.walked(Transform::IDENTITY))
    }

    /// The area this path encloses, signed by the direction its subpaths run in.
    ///
    /// Positive where the subpaths run counter-clockwise in the path's own space, negative
    /// where they run clockwise, and zero for a path that encloses nothing. A subpath that is
    /// not closed is treated as closed, which is what §8.5.3.3's fill rules do with one and
    /// therefore the only reading under which the number describes what would be painted.
    ///
    /// **What it is for is the *sign*, and one caller has it**: ISO 32000-2 §9.3.6 combines a
    /// text object's glyph outlines into a single path under the non-zero winding number rule,
    /// so two glyphs drawn in opposite directions cancel where they overlap instead of
    /// uniting — NOTE 2 of that clause says so outright. A face this program chose for a
    /// document that supplied none has no direction the file stated, so the direction is this
    /// program's to normalise; see `pdf_font`'s substituted-outline handling.
    ///
    /// The area of each cubic is exact rather than flattened. Integrating Green's theorem
    /// ∮(x dy − y dx)/2 over a cubic Bézier gives a weighted sum of the cross products of its
    /// four control points, and the weights below are that integral's — a curve that bulges
    /// away from its control polygon therefore contributes what it encloses and not what its
    /// handles suggest.
    #[must_use]
    pub fn signed_area(&self) -> f32 {
        // Green's theorem over a cubic: ∫₀¹(x y' − y x')dt/2 = Σ wᵢⱼ (xᵢyⱼ − xⱼyᵢ) with the
        // weights below, which fall out of ∫ bᵢ bⱼ' dt over the Bernstein basis. The straight
        // segment is the same integral over the linear basis, where it is half one cross
        // product.
        let cross = |a: Point, b: Point| a.x * b.y - b.x * a.y;
        let mut area = 0.0;
        let mut previous: Option<Point> = None;
        let mut start: Option<Point> = None;
        for command in &self.commands {
            match *command {
                PathCommand::MoveTo(p) => {
                    // A new subpath closes the one before it, which is what a fill does with
                    // an unclosed subpath (§8.5.3.3).
                    if let (Some(from), Some(to)) = (previous, start) {
                        area += cross(from, to) / 2.0;
                    }
                    previous = Some(p);
                    start = Some(p);
                }
                PathCommand::LineTo(p) => {
                    if let Some(from) = previous {
                        area += cross(from, p) / 2.0;
                    }
                    previous = Some(p);
                }
                PathCommand::CurveTo(a, b, c) => {
                    if let Some(from) = previous {
                        area += 0.3 * cross(from, a)
                            + 0.15 * cross(from, b)
                            + 0.05 * cross(from, c)
                            + 0.15 * cross(a, b)
                            + 0.15 * cross(a, c)
                            + 0.3 * cross(b, c);
                    }
                    previous = Some(c);
                }
                PathCommand::Close => {
                    if let (Some(from), Some(to)) = (previous, start) {
                        area += cross(from, to) / 2.0;
                    }
                    previous = start;
                }
            }
        }
        if let (Some(from), Some(to)) = (previous, start) {
            area += cross(from, to) / 2.0;
        }
        area
    }

    /// The same shape with every subpath running the other way.
    ///
    /// The geometry is untouched: each subpath keeps its start point and its segments, and only
    /// the order they are travelled in changes. Filling the result under either of §8.5.3.3's
    /// rules paints exactly the same pixels, because reversing every subpath negates every
    /// winding number and both rules test the winding number's *magnitude*. What changes is how
    /// it combines with a path that was **not** reversed, which is §9.3.6 NOTE 2's case and the
    /// only reason this exists.
    ///
    /// An unclosed subpath comes back closed, which costs nothing at a fill — §8.5.3.3 closes it
    /// anyway — and keeps the reversal expressible: the segment a fill implies has to be
    /// travelled first on the way back.
    #[must_use]
    pub fn reversed(&self) -> Self {
        let mut path = Self::new();
        path.commands.reserve(self.commands.len());
        // One subpath at a time: `run` holds each segment beside the point it left, which is
        // the point that segment ends at once the subpath is travelled backwards.
        let mut run: Vec<(PathCommand, Point)> = Vec::new();
        let mut start: Option<Point> = None;
        let mut previous: Option<Point> = None;
        for command in &self.commands {
            match *command {
                PathCommand::MoveTo(p) => {
                    reverse_subpath(start, &run, &mut path);
                    run.clear();
                    start = Some(p);
                    previous = Some(p);
                }
                PathCommand::LineTo(p) => {
                    if let Some(from) = previous {
                        run.push((PathCommand::LineTo(p), from));
                    }
                    previous = Some(p);
                }
                PathCommand::CurveTo(a, b, c) => {
                    if let Some(from) = previous {
                        run.push((PathCommand::CurveTo(a, b, c), from));
                    }
                    previous = Some(c);
                }
                // The closing segment is the one back to the subpath's start, which the
                // reversal writes for itself; what `h` decides here is that the subpath has
                // ended, and §8.5.2.1 leaves the current point on its start.
                PathCommand::Close => {
                    reverse_subpath(start, &run, &mut path);
                    run.clear();
                    previous = start;
                }
            }
        }
        reverse_subpath(start, &run, &mut path);
        path
    }

    /// [`Self::bounds`] without the cache: every control point, transformed and met.
    fn walked(&self, transform: Transform) -> Option<Rect> {
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

    /// Reports the device y range of every segment a horizontal cut would re-state, as
    /// `(top, bottom)`.
    ///
    /// # What "re-state" means, and why only some segments do it
    ///
    /// Rasterising a path into a target that does not contain it chops the path against the
    /// target's edge. What that costs depends on the segment, and ADR 0139 measured all three
    /// cases by filling one shape into a whole pixmap and into two pieces of it:
    ///
    /// | segment | bytes differing of 2.9 M | worst |
    /// |---|---|---|
    /// | axis-aligned edge | **0** | 0 |
    /// | oblique straight edge | 292–528 | 32 |
    /// | cubic | 2480–2744 | 64 |
    ///
    /// A cubic is *re-parameterised*: the piece inside the target is four new control points,
    /// so its coverage differs along the whole of it. An oblique line keeps its geometry but
    /// not its endpoints — the clipped endpoint is computed by interpolation and the edge's
    /// slope is then taken from it — so it differs too, by half as much. **An axis-aligned
    /// edge survives**: a horizontal one is kept or dropped whole, and clipping a vertical one
    /// at `y = r` moves its endpoint to a point the line already passed through.
    ///
    /// So a curve is always reported and a straight segment unless it is axis-aligned. A
    /// caller cutting a target into strips may cut at any row nothing reported here spans.
    ///
    /// # The transform is part of the answer
    ///
    /// Alignment is judged in the path's own space and requires `transform` to keep the axes —
    /// a scale and a translation, or a quarter turn. Under a shear or a general rotation
    /// nothing is axis-aligned in device space and every segment is reported, which is the
    /// side to err on: judging alignment on the transformed coordinates would rest on this
    /// crate and `tiny-skia` rounding one multiplication identically.
    pub fn oblique_spans(&self, transform: Transform, mut mark: impl FnMut(f32, f32)) {
        let axes = transform.preserves_axes();
        let mut previous: Option<Point> = None;
        let mut start: Option<Point> = None;
        let straight = |from: Option<Point>, to: Point, mark: &mut dyn FnMut(f32, f32)| {
            let Some(from) = from else { return };
            #[expect(
                clippy::float_cmp,
                reason = "the question is whether two endpoints state the same coordinate, \
                          which is an exact property of the numbers the file gave us"
            )]
            let aligned = axes && (from.x == to.x || from.y == to.y);
            if aligned {
                return;
            }
            let (one, other) = (transform.apply(from).y, transform.apply(to).y);
            mark(one.min(other), one.max(other));
        };
        for command in &self.commands {
            match *command {
                PathCommand::MoveTo(p) => {
                    previous = Some(p);
                    start = Some(p);
                }
                PathCommand::LineTo(p) => {
                    straight(previous, p, &mut mark);
                    previous = Some(p);
                }
                PathCommand::CurveTo(a, b, c) => {
                    let mut top = f32::INFINITY;
                    let mut bottom = f32::NEG_INFINITY;
                    for point in previous.into_iter().chain([a, b, c]) {
                        let y = transform.apply(point).y;
                        top = top.min(y);
                        bottom = bottom.max(y);
                    }
                    if top <= bottom {
                        mark(top, bottom);
                    }
                    previous = Some(c);
                }
                // §8.5.2.1's `h` closes the subpath with a straight segment back to its
                // start, which is a segment like any other.
                PathCommand::Close => {
                    if let Some(to) = start {
                        straight(previous, to, &mut mark);
                    }
                    previous = start;
                }
            }
        }
    }
}

#[cfg(test)]
mod direction_tests {
    use super::{Path, PathCommand, Point};

    /// A closed square of straight segments, counter-clockwise from the origin.
    fn square(side: f32) -> Path {
        let mut path = Path::new();
        path.push(PathCommand::MoveTo(Point::new(0.0, 0.0)));
        path.push(PathCommand::LineTo(Point::new(side, 0.0)));
        path.push(PathCommand::LineTo(Point::new(side, side)));
        path.push(PathCommand::LineTo(Point::new(0.0, side)));
        path.push(PathCommand::Close);
        path
    }

    /// The same square with every side written as a cubic whose handles sit on it.
    ///
    /// A cubic with its control points at the thirds of a straight segment *is* that segment,
    /// so this path and [`square`] enclose the same area exactly — which is what makes it a
    /// check on the cubic weights rather than on the arithmetic around them.
    fn curved_square(side: f32) -> Path {
        // The first corner repeated, so that the four sides are consecutive pairs.
        let corners = [
            Point::new(0.0, 0.0),
            Point::new(side, 0.0),
            Point::new(side, side),
            Point::new(0.0, side),
            Point::new(0.0, 0.0),
        ];
        let mut path = Path::new();
        path.push(PathCommand::MoveTo(Point::new(0.0, 0.0)));
        for side in corners.windows(2) {
            let (from, to) = (side[0], side[1]);
            let third =
                |t: f32| Point::new(from.x + (to.x - from.x) * t, from.y + (to.y - from.y) * t);
            path.push(PathCommand::CurveTo(third(1.0 / 3.0), third(2.0 / 3.0), to));
        }
        path.push(PathCommand::Close);
        path
    }

    /// The signed area is the area, and its sign is the direction the subpath runs in.
    #[test]
    fn a_squares_signed_area_is_its_area_by_both_constructions() {
        assert!((square(4.0).signed_area() - 16.0).abs() < 1e-4, "straight");
        assert!(
            (curved_square(4.0).signed_area() - 16.0).abs() < 1e-3,
            "cubic: {}",
            curved_square(4.0).signed_area()
        );
        assert!(
            (square(4.0).reversed().signed_area() + 16.0).abs() < 1e-4,
            "reversed"
        );
    }

    /// A subpath left unclosed encloses what closing it would, which is what a fill paints.
    #[test]
    fn an_unclosed_subpath_encloses_what_a_fill_would_paint() {
        let mut open = Path::new();
        open.push(PathCommand::MoveTo(Point::new(0.0, 0.0)));
        open.push(PathCommand::LineTo(Point::new(4.0, 0.0)));
        open.push(PathCommand::LineTo(Point::new(4.0, 4.0)));
        open.push(PathCommand::LineTo(Point::new(0.0, 4.0)));

        assert!((open.signed_area() - 16.0).abs() < 1e-4);
    }

    /// A counter inside an outer contour subtracts, which is the sign a glyph relies on.
    ///
    /// This is why the *total* decides a glyph's direction: an outer contour always encloses
    /// more than the counters inside it, so the sum carries the outer contour's sign.
    #[test]
    fn a_counter_wound_the_other_way_subtracts() {
        let mut glyph = square(10.0);
        // The counter sits inside the outer contour and runs the other way.
        glyph.extend_transformed(
            &square(4.0).reversed(),
            super::Transform::translate(3.0, 3.0),
        );

        assert!((glyph.signed_area() - (100.0 - 16.0)).abs() < 1e-3);
    }

    /// Reversing twice is the identity on the geometry, point for point.
    ///
    /// Not merely on the area: a reversal that dropped a control point or closed the wrong
    /// subpath would keep the area and change the shape, which is trap 1's failure one
    /// directory over.
    #[test]
    fn reversing_twice_restores_every_point() {
        let mut path = Path::new();
        path.push(PathCommand::MoveTo(Point::new(1.0, 2.0)));
        path.push(PathCommand::CurveTo(
            Point::new(3.0, 9.0),
            Point::new(7.0, 8.0),
            Point::new(6.0, 1.0),
        ));
        path.push(PathCommand::LineTo(Point::new(2.0, 0.5)));
        path.push(PathCommand::Close);
        path.push(PathCommand::MoveTo(Point::new(20.0, 20.0)));
        path.push(PathCommand::LineTo(Point::new(24.0, 20.0)));
        path.push(PathCommand::LineTo(Point::new(24.0, 24.0)));
        path.push(PathCommand::Close);

        let twice = path.reversed().reversed();

        // The same subpaths, each starting where it started and running the way it ran. The
        // start point of a closed subpath is arbitrary, so this asserts the stronger thing
        // the implementation actually promises.
        assert_eq!(twice.commands(), path.commands());
    }

    /// Every subpath of a many-subpath path is reversed, not only the first.
    #[test]
    fn every_subpath_turns_around() {
        let mut path = square(2.0);
        path.extend(square(3.0).commands());

        let reversed = path.reversed();

        assert!((reversed.signed_area() + 4.0 + 9.0).abs() < 1e-4);
    }
}

/// Writes one subpath into `path` travelled backwards, for [`Path::reversed`].
///
/// `run` is the subpath's segments in order, each beside the point it started from. Travelled
/// backwards, that point is where the segment now ends — and a cubic's two control points swap,
/// because the first handle belongs to whichever end is now the start.
fn reverse_subpath(start: Option<Point>, run: &[(PathCommand, Point)], path: &mut Path) {
    let (Some(start), Some(&(last, _))) = (start, run.last()) else {
        // A subpath of no segments encloses nothing and is dropped rather than reversed: a
        // lone `m` marks no pixels under either of §8.5.3.3's rules.
        return;
    };
    let end = match last {
        PathCommand::LineTo(p) | PathCommand::CurveTo(_, _, p) => p,
        PathCommand::MoveTo(_) | PathCommand::Close => start,
    };
    path.push(PathCommand::MoveTo(end));
    for &(command, from) in run.iter().rev() {
        match command {
            PathCommand::LineTo(_) => path.push(PathCommand::LineTo(from)),
            PathCommand::CurveTo(first, second, _) => {
                path.push(PathCommand::CurveTo(second, first, from));
            }
            PathCommand::MoveTo(_) | PathCommand::Close => {}
        }
    }
    path.push(PathCommand::Close);
}

#[cfg(test)]
mod hull_tests {
    use super::{Path, PathCommand, Point, Transform};

    /// A cached hull, mapped, must be the rectangle the walk produces — not merely a bound.
    ///
    /// Checked at every kind of transform the fast path claims: a scale, a translation, a
    /// negative scale (which exchanges the extremes) and a quarter turn. The slow path is
    /// checked too, because the branch is what decides which runs.
    #[test]
    fn the_cached_hull_is_the_walk_at_every_axis_preserving_transform() {
        let mut path = Path::new();
        path.push(PathCommand::MoveTo(Point::new(3.5, -2.25)));
        path.push(PathCommand::LineTo(Point::new(40.0, 11.0)));
        path.push(PathCommand::CurveTo(
            Point::new(-8.0, 60.0),
            Point::new(70.0, -30.0),
            Point::new(9.0, 5.0),
        ));
        path.push(PathCommand::Close);

        for transform in [
            Transform::IDENTITY,
            Transform::translate(17.0, -4.5),
            Transform::new(2.5, 0.0, 0.0, 3.0, 1.0, 2.0),
            // A negative scale, where the smallest x maps to the largest.
            Transform::new(-1.5, 0.0, 0.0, -2.0, 0.0, 0.0),
            // A quarter turn: the other shape `preserves_axes` admits.
            Transform::new(0.0, 1.0, -1.0, 0.0, 0.0, 0.0),
            // A shear, which takes the slow path.
            Transform::new(1.0, 0.4, 0.2, 1.0, 0.0, 0.0),
        ] {
            // A fresh path each time, so the first call and a later one are both exercised.
            let cold = path.clone();
            assert_eq!(
                cold.bounds(transform),
                cold.walked(transform),
                "cold, at {transform:?}"
            );
            assert_eq!(
                path.bounds(transform),
                path.walked(transform),
                "warm, at {transform:?}"
            );
        }
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
    use super::{Point, Rect, Transform};

    /// Two rectangles that overlap intersect in the region both cover.
    #[test]
    fn an_intersection_is_the_region_both_rectangles_cover() {
        let a = Rect::from_corners(Point::new(0.0, 0.0), Point::new(10.0, 10.0));
        let b = Rect::from_corners(Point::new(5.0, -5.0), Point::new(20.0, 4.0));
        let both = a.intersection(b).expect("they overlap");
        assert_eq!(both.min, Point::new(5.0, 0.0));
        assert_eq!(both.max, Point::new(10.0, 4.0));
    }

    /// Two that do not meet intersect in nothing, which is `None` rather than an inverted
    /// rectangle — a rectangle whose corners had crossed over would compare as containing
    /// everything.
    #[test]
    fn rectangles_that_do_not_meet_have_no_intersection() {
        let a = Rect::from_corners(Point::new(0.0, 0.0), Point::new(10.0, 10.0));
        let b = Rect::from_corners(Point::new(11.0, 0.0), Point::new(20.0, 10.0));
        assert_eq!(a.intersection(b), None);
    }

    /// Touching along an edge is an intersection, and it is degenerate.
    ///
    /// ISO 32000-2 §7.9.5's own NOTE is that "[r]ectangles can have a width of zero or height of
    /// zero", so a content rectangle that lies exactly on a clip's boundary is still a place and
    /// answering "nowhere" for it would lose one.
    #[test]
    fn rectangles_that_touch_intersect_degenerately() {
        let a = Rect::from_corners(Point::new(0.0, 0.0), Point::new(10.0, 10.0));
        let b = Rect::from_corners(Point::new(10.0, 2.0), Point::new(20.0, 8.0));
        let both = a.intersection(b).expect("an edge in common is a rectangle");
        assert_eq!(both.width(), 0.0);
        assert_eq!(both.height(), 6.0);
    }

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
        let r = Rect::from_corners(Point::new(10.0, 20.0), Point::new(0.0, 5.0));
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

    /// `min_stretch` is the smallest factor a vector's length is multiplied by.
    ///
    /// The mirror of the test above and checked the same way — the minimum over a fine sweep of
    /// directions must reach the returned value and never fall below it — because the property
    /// its caller relies on is the *floor*: a substitute shape stated at `1 / min_stretch` is at
    /// least one device pixel across whichever way the path runs.
    #[test]
    fn min_stretch_floors_every_direction() {
        let cases = [
            Transform::scale(3.0, 3.0),
            Transform::scale(4.0, 0.25),
            Transform::new(0.866_025_4, 0.5, -0.5, 0.866_025_4, 0.0, 0.0),
            Transform::new(1.0, 0.0, 2.0, 1.0, 0.0, 0.0),
            Transform::new(2.0, 0.5, -0.25, 3.0, 10.0, -4.0),
        ];
        for transform in cases {
            let floor = transform.min_stretch();
            let mut least = f32::INFINITY;
            for step in 0..3600 {
                #[expect(
                    clippy::cast_precision_loss,
                    reason = "test code: the loop counter is under four thousand, which f32 \
                              represents exactly"
                )]
                let angle = step as f32 * std::f32::consts::TAU / 3600.0;
                let origin = transform.apply(Point::new(0.0, 0.0));
                let tip = transform.apply(Point::new(angle.cos(), angle.sin()));
                least = least.min((tip.x - origin.x).hypot(tip.y - origin.y));
            }
            assert!(
                least >= floor * 0.999,
                "{transform:?}: a direction stretched by only {least}, below the floor {floor}"
            );
            assert!(
                least <= floor * 1.000_1,
                "{transform:?}: the floor {floor} is not reached; the least was {least}"
            );
            // The two singular values multiply to the determinant, for every transform.
            let product = floor * transform.max_stretch();
            assert!(
                (product - transform.determinant().abs()).abs() < 1e-3 * product.max(1.0),
                "{transform:?}: {product} against a determinant of {}",
                transform.determinant()
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

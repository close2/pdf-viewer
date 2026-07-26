//! Colour, fill rules, and stroke parameters.

/// A colour in device RGB with straight (non-premultiplied) alpha.
///
/// # Why colour is already resolved here
///
/// PDF supports `DeviceGray`, `DeviceRGB`, `DeviceCMYK`, `ICCBased`, Indexed,
/// Separation, `DeviceN`, and Lab colour spaces. Resolving those — including ICC
/// transforms and tint transform functions — is colour management, and it happens
/// in the content-stream interpreter, upstream of the display list.
///
/// Backends therefore never see a colour space. This keeps colour management in one
/// place and guarantees the CPU and GPU backends cannot disagree about it, which is
/// a precondition for using one to validate the other.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Color {
    /// Red component in `0.0..=1.0`.
    pub r: f32,
    /// Green component in `0.0..=1.0`.
    pub g: f32,
    /// Blue component in `0.0..=1.0`.
    pub b: f32,
    /// Alpha in `0.0..=1.0`, where `0.0` is fully transparent.
    pub a: f32,
}

impl Color {
    /// Opaque black.
    pub const BLACK: Self = Self {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        a: 1.0,
    };
    /// Opaque white.
    pub const WHITE: Self = Self {
        r: 1.0,
        g: 1.0,
        b: 1.0,
        a: 1.0,
    };
    /// Fully transparent.
    pub const TRANSPARENT: Self = Self {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        a: 0.0,
    };

    /// Creates an opaque colour from RGB components in `0.0..=1.0`.
    #[must_use]
    pub const fn rgb(r: f32, g: f32, b: f32) -> Self {
        Self { r, g, b, a: 1.0 }
    }

    /// Creates a colour from RGBA components in `0.0..=1.0`.
    #[must_use]
    pub const fn rgba(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }
}

/// How a region is painted.
///
/// Tiling patterns are not represented yet; the enum is non-exhaustive so that adding
/// them is not a breaking change for backends.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum Paint {
    /// A single uniform colour.
    Solid(Color),
    /// A smooth colour transition.
    ///
    /// Shared rather than owned because one shading commonly paints many commands: a
    /// pattern set as the fill colour applies to every path filled until the colour
    /// changes again.
    Shading(std::sync::Arc<crate::shading::Shading>),
}

/// Which points count as inside a path.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FillRule {
    /// A point is inside when the winding number is non-zero (PDF `f`).
    #[default]
    NonZero,
    /// A point is inside when the crossing count is odd (PDF `f*`).
    EvenOdd,
}

/// How the ends of an open subpath are drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LineCap {
    /// Terminates exactly at the endpoint (PDF line cap 0).
    #[default]
    Butt,
    /// A semicircle of diameter equal to the line width (PDF line cap 1).
    Round,
    /// A square extending half the line width past the endpoint (PDF line cap 2).
    Square,
}

/// How two connected segments are joined.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LineJoin {
    /// Extends the outer edges to meet, subject to the miter limit (PDF join 0).
    #[default]
    Miter,
    /// An arc of diameter equal to the line width (PDF join 1).
    Round,
    /// The outer corner is cut off with a straight edge (PDF join 2).
    Bevel,
}

/// Parameters controlling how a path is stroked.
#[derive(Debug, Clone, PartialEq)]
pub struct Stroke {
    /// Line width in the command's coordinate space.
    ///
    /// A width of `0.0` is legal in PDF and means the thinnest line the device can
    /// render — one pixel — rather than an invisible one. Backends must honour that.
    pub width: f32,
    /// Treatment of open subpath ends.
    pub cap: LineCap,
    /// Treatment of segment joins.
    pub join: LineJoin,
    /// Ratio at which a miter join is converted to a bevel.
    pub miter_limit: f32,
    /// Alternating on/off dash lengths. Empty means a solid line.
    pub dash_array: Vec<f32>,
    /// Distance into the dash pattern at which to start.
    pub dash_phase: f32,
}

impl Default for Stroke {
    /// The PDF initial graphics state: 1.0 width, butt caps, miter joins, limit 10.
    fn default() -> Self {
        Self {
            width: 1.0,
            cap: LineCap::default(),
            join: LineJoin::default(),
            miter_limit: 10.0,
            dash_array: Vec::new(),
            dash_phase: 0.0,
        }
    }
}

/// How a drawing operation combines with what is already present.
///
/// These are the sixteen PDF blend modes. The first four are separable and
/// inexpensive; the final four operate on whole colours and require the backdrop
/// colour, which constrains how aggressively a backend may batch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BlendMode {
    /// Source replaces backdrop, modulated by alpha.
    #[default]
    Normal,
    /// Multiplies the components.
    Multiply,
    /// Multiplies the complements.
    Screen,
    /// Multiply or screen, depending on the backdrop.
    Overlay,
    /// Selects the darker of the two.
    Darken,
    /// Selects the lighter of the two.
    Lighten,
    /// Brightens the backdrop to reflect the source.
    ColorDodge,
    /// Darkens the backdrop to reflect the source.
    ColorBurn,
    /// Multiply or screen, depending on the source.
    HardLight,
    /// Darkens or lightens, depending on the source.
    SoftLight,
    /// Absolute component-wise difference.
    Difference,
    /// Like `Difference` with lower contrast.
    Exclusion,
    /// Source hue with backdrop saturation and luminosity.
    Hue,
    /// Source saturation with backdrop hue and luminosity.
    Saturation,
    /// Source hue and saturation with backdrop luminosity.
    Color,
    /// Source luminosity with backdrop hue and saturation.
    Luminosity,
}

impl BlendMode {
    /// Returns `true` when the mode acts on each colour component independently.
    ///
    /// Separable modes can be evaluated per channel, which lets a backend process
    /// them in a single pass. The four non-separable modes need the complete
    /// backdrop colour and are therefore more expensive to batch.
    #[must_use]
    pub fn is_separable(self) -> bool {
        !matches!(
            self,
            Self::Hue | Self::Saturation | Self::Color | Self::Luminosity
        )
    }
}

/// Decoded image samples, ready to draw.
///
/// Always straight-alpha RGBA8, whatever the document's colour space and bit depth were:
/// converting once when the image is decoded means neither backend needs to know about
/// PDF colour spaces, which is the same reason [`Color`] is already resolved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Image {
    /// Width in samples.
    pub width: u32,
    /// Height in samples.
    pub height: u32,
    /// Row-major RGBA8 samples, top row first, with no row padding.
    pub data: std::sync::Arc<[u8]>,
}

impl Image {
    /// Returns `true` if the dimensions and buffer length agree.
    ///
    /// Worth checking at the boundary: a mismatch means an indexing bug downstream, and
    /// a backend that trusted the dimensions would read past the end of a short buffer or
    /// silently render garbage.
    #[must_use]
    pub fn is_consistent(&self) -> bool {
        let expected = (self.width as usize)
            .saturating_mul(self.height as usize)
            .saturating_mul(4);
        self.data.len() == expected && self.width > 0 && self.height > 0
    }
}

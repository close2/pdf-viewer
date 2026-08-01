//! Colour, fill rules, and stroke parameters.

use crate::geom::Transform;

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
    /// render — one pixel — rather than an invisible one. Backends must not read this
    /// field directly: [`Self::device_width`] is where that rule and §10.7.5's live, so
    /// that the two backends cannot answer it differently.
    pub width: f32,
    /// Whether ISO 32000-2 §10.7.5's automatic stroke adjustment is enabled (`/SA`).
    ///
    /// A graphics state parameter rather than a stroke parameter in the standard's own
    /// arrangement (Table 57), and carried here because a stroke is the only thing it
    /// affects and because that makes `q`/`Q` save and restore it for free. Initial value
    /// `false`, per Table 57.
    pub adjust: bool,
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

impl Stroke {
    /// The width to stroke with, in the path's own space, given the path-to-device map.
    ///
    /// Two of the standard's rules meet here, and both are about the one thickness a device
    /// cannot go below.
    ///
    /// ISO 32000-2 §8.4.3.2:
    ///
    /// > A line width of 0 shall denote the thinnest line that can be rendered at device
    /// > resolution: 1 device pixel wide.
    ///
    /// ISO 32000-2 §10.7.5, when `/SA` has enabled automatic stroke adjustment:
    ///
    /// > If stroke adjustment is enabled and the requested line width, transformed into
    /// > device space, is less than half a pixel, the stroke shall be rendered as a
    /// > single-pixel line.
    ///
    /// Both answers are the same width, and its NOTE says so: the second case "is
    /// equivalent to the effect produced by setting the line width to 0".
    ///
    /// # Why this is a function here rather than each backend's own hairline
    ///
    /// `tiny-skia` treats a width of `0.0` as a hairline and gets §8.4.3.2 right for free;
    /// Vello has no such mode and drew **nothing at all** for a zero-width stroke until the
    /// nineteenth session, on every page of every document since the backend existed. A
    /// backend-specific convention that one backend happens to share with PDF is not a
    /// reading of the clause, and the cross-backend comparison could not see the difference
    /// because no scene stroked a zero width. One device pixel expressed back in the path's
    /// own space is a width both backends can draw, and it is the same width for both.
    ///
    /// The minimum is stated in device pixels and applied in path space, so it is divided by
    /// the stretch it will be multiplied by. Where the transform scales the two axes
    /// differently there is no single answer — §8.4.3.2 says as much, "the thickness of
    /// stroked lines in device space shall vary according to their orientation" — and this
    /// takes the widest direction: [`Transform::max_stretch`] makes the substituted stroke
    /// exactly one pixel at its widest and thinner elsewhere, and makes §10.7.5's test fire
    /// only where the stroke is under half a pixel in *every* direction. For a similarity
    /// transform, which is what every page transform in this renderer is, the two singular
    /// values coincide and the choice does not arise.
    #[must_use]
    /// # A negative width is not a width, and this is where that is decided
    ///
    /// The clause says the parameter "shall be a non-negative number", so `-0.1 w` is outside
    /// its domain and no recovery is stated. `content.rs` clamps such a value to zero, which
    /// brings it into the domain, and this function then applies the clause's rule for zero —
    /// so a negative width draws one device pixel. That is a **choice**, not a derivation, and
    /// two others are equally available: the clause's own definition of stroking paints "all
    /// points whose perpendicular distance from the path … is less than or equal to half the
    /// line width", which for a negative width is no point at all; and the magnitude, which
    /// covers the same region a positive width of the same size would. `issue19633.pdf` is the
    /// corpus's only witness — one operator in one document — and the three readings put a
    /// visibly different line on it.
    ///
    pub fn device_width(&self, to_device: Transform) -> f32 {
        let stretch = to_device.max_stretch();
        // A degenerate transform has collapsed the path to a line or a point; there is no
        // width in device space to compare against and nothing to divide by.
        if !stretch.is_finite() || stretch <= 0.0 {
            return self.width;
        }
        let one_pixel = 1.0 / stretch;
        let in_device = self.width * stretch;
        if self.width <= 0.0 || (self.adjust && in_device < 0.5) {
            one_pixel
        } else {
            self.width
        }
    }
}

impl Default for Stroke {
    /// The PDF initial graphics state: 1.0 width, butt caps, miter joins, limit 10, and
    /// stroke adjustment off (Table 57).
    fn default() -> Self {
        Self {
            width: 1.0,
            adjust: false,
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
    /// Whether the samples may be smoothed when the image is drawn larger than its grid.
    ///
    /// ISO 32000-2 §8.9.5.3, the image dictionary's `/Interpolate`, whose default is false.
    /// The clause is about magnification — interpolation "is an attempt to produce a smooth
    /// transition between adjacent sample values when rendering an image whose resolution is
    /// significantly lower than that of the output device" — and it makes the entry a hint
    /// that "a PDF processor may ignore".
    ///
    /// So this bounds one direction only. A backend drawing an image *smaller* than its
    /// sample grid is resampling rather than interpolating, which the clause does not
    /// address and which nearest-neighbour would do badly; that stays the backend's choice.
    /// What the flag decides is whether a four-sample image blown up to a page is four
    /// squares or a blur, and three reference renderers draw squares.
    pub interpolate: bool,
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

    /// Whether every sample is fully opaque.
    ///
    /// Asked by `pdf-model` about the elements of a knockout group (§11.4.6): knockout and
    /// ordinary compositing give the same pixels wherever the upper object is opaque, so a
    /// report about the difference has to be able to tell one from the other. An image
    /// carrying a soft mask or a stencil is exactly the case a constant alpha cannot see.
    ///
    /// Linear in the samples, which is why the one caller asks it only for a knockout group.
    #[must_use]
    pub fn is_opaque(&self) -> bool {
        self.data.chunks_exact(4).all(|sample| sample[3] == u8::MAX)
    }

    /// Whether a backend should filter between samples when drawing under `placement`.
    ///
    /// `placement` maps the unit square onto the device, so the length of its two edges is
    /// how many device pixels the whole image covers.
    ///
    /// The rule has two halves, and only the first comes from ISO 32000-2 §8.9.5.3. Where a
    /// sample covers more than one device pixel the image is *magnified*, which is the case
    /// the clause is about, and there `/Interpolate` decides: false — its default — means
    /// each sample is drawn as the flat rectangle it is.
    ///
    /// Where the image is reduced instead, several samples share a pixel and something has to
    /// combine them. **This comment used to say the standard was silent about that, and it is
    /// not**: §10.7.4 requires point sampling and forbids averaging in as many words. The
    /// filter stays on regardless, and [`Image::area_averaged`] goes further still — that is a
    /// documented departure with its argument written down, not a gap in the specification.
    /// The distinction matters because "the clause says nothing" is a licence to choose and
    /// "the clause says the opposite" is a debt to record. ADR 0025.
    ///
    /// Both backends ask this rather than deciding for themselves, because the CPU backend
    /// is the oracle for the GPU one and a difference in this choice would show up as a
    /// disagreement about every magnified image.
    #[must_use]
    pub fn is_smoothed(&self, placement: Transform) -> bool {
        if self.interpolate {
            return true;
        }
        let across = placement.a.hypot(placement.b);
        let down = placement.c.hypot(placement.d);
        #[expect(
            clippy::cast_precision_loss,
            reason = "an image's dimensions are bounded well below f32's exact integer range \
                      by the decoder's own sample limit, and a pixel either way cannot change \
                      which side of this comparison a real image falls"
        )]
        let magnified = across > self.width as f32 || down > self.height as f32;
        !magnified
    }

    /// How many whole source samples share one device pixel along each image axis.
    ///
    /// `placement` maps the unit square onto the device, so `hypot` of each column is the
    /// device length of that edge of the image — which holds under rotation and skew as well
    /// as under a plain scale, and is the same quantity [`Image::is_smoothed`] measures.
    ///
    /// A factor is the *floor* of the ratio, so dividing the grid by it leaves between one
    /// and two source samples per device pixel: enough for a four-tap filter to see all of
    /// them, which is what makes [`Image::area_averaged`] and the backends' own bilinear
    /// filters complementary rather than redundant.
    fn reduction(&self, placement: Transform) -> (u32, u32) {
        #[expect(
            clippy::cast_precision_loss,
            reason = "an image's dimensions are bounded well below f32's exact integer range \
                      by the decoder's own sample limit"
        )]
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "the ratio is clamped to 1.0 ..= the dimension it came from before the \
                      cast, so it is a positive number no larger than a u32 this image's own \
                      width or height already is"
        )]
        fn factor(samples: u32, device: f32) -> u32 {
            if !device.is_finite() || device <= 0.0 {
                return 1;
            }
            ((samples as f32) / device)
                .floor()
                .clamp(1.0, samples as f32) as u32
        }

        (
            factor(self.width, placement.a.hypot(placement.b)),
            factor(self.height, placement.c.hypot(placement.d)),
        )
    }

    /// Averages each block of samples that would share one device pixel, or `None` if none do.
    ///
    /// # Why this is a choice rather than a derivation
    ///
    /// ISO 32000-2 §10.7.4 describes the opposite, and says so in as many words:
    ///
    /// > The position of the centre of such a pixel -in other words, the point whose
    /// > coordinate values have fractional parts of one-half -shall be mapped back into
    /// > source space to determine how to colour the pixel. There shall not be averaging over
    /// > the pixel area. If the resolution of the source image is higher than that of device
    /// > space, some source samples might not be used.
    ///
    /// That is point sampling, and this function is the averaging that sentence forbids. What
    /// licenses the departure is §10.7.1's own framing of the clause it sits in — "the
    /// specifics of the scan conversion algorithm are not defined as part of PDF. Different
    /// implementations can perform scan conversion in different ways; techniques that are
    /// appropriate for one device could be inappropriate for another" — together with the
    /// fact that this tree already departs from the same subclause's *first* rule, since
    /// anti-aliasing a fill is not "painting any pixel whose half-open square region
    /// intersects the shape". §10.7.4 describes a device that quantises coverage to whole
    /// pixels; a display does not, and neither does this renderer.
    ///
    /// The cost of the choice is recorded rather than assumed away: a producer who relied on
    /// a particular sample surviving the reduction — a one-pixel rule, a dither pattern —
    /// gets a softened version of it instead of that sample. The benefit is the case that
    /// made this schedulable: `bug1001080.pdf` sets its text in a Type 3 font whose glyphs
    /// are 39x53 fax bitmaps drawn about five pixels high, and under point sampling *or* a
    /// four-tap filter the crossbar of every `t` is one source row in fifty-three that
    /// neither one ever looks at. See ADR 0025.
    ///
    /// # Why here rather than in a backend
    ///
    /// For the same reason [`Image::is_smoothed`] is here: the CPU backend is the correctness
    /// oracle for the GPU one, and a resampling decision made twice is a decision the two can
    /// disagree about. This produces new samples, so both backends draw the same raster and
    /// their own filters then do the same residual, sub-two-fold work.
    #[must_use]
    pub fn area_averaged(&self, placement: Transform) -> Option<Self> {
        let (kx, ky) = self.reduction(placement);
        if (kx <= 1 && ky <= 1) || !self.is_consistent() {
            return None;
        }

        // `div_ceil`, so the samples at a right or bottom edge keep an output cell rather
        // than being dropped.
        let width = self.width.div_ceil(kx);
        let height = self.height.div_ceil(ky);

        // The block boundaries are then *proportional* rather than fixed multiples of the
        // factor, so the blocks tile the source grid exactly and no two differ by more than
        // one sample. Fixed multiples with a short block at the edge are the tempting version
        // and are wrong in a way that shows: `512 / 5` leaves a two-sample block occupying a
        // whole output cell, which squeezes the image into 99.4% of the unit square and moved
        // `firefox_logo.pdf` *further* from three references than no filtering at all.
        let columns = Bands::new(self.width, width);
        let rows = Bands::new(self.height, height);

        let mut data: Vec<u8> = Vec::with_capacity(
            (width as usize)
                .saturating_mul(height as usize)
                .saturating_mul(4),
        );
        for out_y in 0..height {
            let (y0, y1) = rows.at(out_y);
            for out_x in 0..width {
                let (x0, x1) = columns.at(out_x);
                data.extend_from_slice(&self.average_block(x0, y0, x1, y1));
            }
        }

        Some(Self {
            width,
            height,
            data: data.into(),
            interpolate: self.interpolate,
        })
    }

    /// The mean of one block of samples, as straight-alpha RGBA8.
    ///
    /// Averaged *premultiplied* and divided back out at the end. Averaging straight-alpha
    /// components directly would let a transparent sample's colour — which is carried but
    /// contributes nothing to the page — pull the result towards itself, so the soft edge of
    /// a shrunken glyph would take on whatever the encoder happened to store beneath its
    /// transparent samples.
    ///
    /// The sums are `u64` because a block is as large as the reduction is deep: a 12608x16806
    /// scan drawn four pixels wide puts thirteen million samples in one block, and thirteen
    /// million times 255 times 255 leaves `u32` far behind.
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "this runs once per source sample of every reduced image, and saturating \
                  arithmetic here measured +17% of the rasterisation of a page that is one \
                  2500x1364 photograph. It cannot overflow: a block holds at most as many \
                  samples as the image does, which `is_consistent` has already bounded by a \
                  u32, and each contributes at most 255 * 255 < 2^16 — so every sum is under \
                  2^48 in a u64. The index arithmetic is bounded the same way, by dimensions \
                  whose product `is_consistent` has checked against the buffer's own length"
    )]
    fn average_block(&self, x0: u32, y0: u32, x1: u32, y1: u32) -> [u8; 4] {
        // Colour premultiplied by alpha, then alpha itself.
        let mut colour = [0u64; 3];
        let mut alpha_sum = 0u64;
        let mut count = 0u64;

        for y in y0..y1 {
            let row = (y as usize) * (self.width as usize);
            let from = (row + x0 as usize) * 4;
            let to = (row + x1 as usize) * 4;
            let Some(span) = self.data.get(from..to) else {
                continue;
            };
            for sample in span.chunks_exact(4) {
                let alpha = u64::from(sample[3]);
                for (sum, component) in colour.iter_mut().zip(sample) {
                    *sum += u64::from(*component) * alpha;
                }
                alpha_sum += alpha;
                count += 1;
            }
        }

        // A block whose samples are all fully transparent carries no colour to recover, and
        // dividing by its alpha would be dividing by zero. Transparent black is what it is.
        if count == 0 || alpha_sum == 0 {
            return [0, 0, 0, 0];
        }

        // Undoing the premultiplication divides by the *mean* alpha, and the sample count
        // then cancels from both sides of the ratio — so the colour divides by `alpha_sum`
        // and only the alpha divides by `count`. Both round to nearest.
        let mut out = [0u8; 4];
        for (channel, sum) in out.iter_mut().zip(colour) {
            *channel = round_div(sum, alpha_sum);
        }
        out[3] = round_div(alpha_sum, count);
        out
    }
}

/// Divides `samples` positions into `cells` consecutive bands as evenly as they go.
///
/// Band `i` is `[i * samples / cells, (i + 1) * samples / cells)` in integer arithmetic, so
/// the bands tile `0..samples` exactly with no gap and no overlap, and no two differ in
/// length by more than one. That exactness is the point: it is what keeps the reduced image
/// covering the same region of the unit square as the one it replaces.
#[derive(Debug, Clone, Copy)]
struct Bands {
    samples: u64,
    cells: u64,
}

impl Bands {
    /// `cells` is expected in `1..=samples`, which is what `div_ceil` of a factor produces.
    fn new(samples: u32, cells: u32) -> Self {
        Self {
            samples: u64::from(samples),
            cells: u64::from(cells.max(1)),
        }
    }

    /// The half-open sample range of band `index`.
    fn at(self, index: u32) -> (u32, u32) {
        let edge = |i: u64| {
            let scaled = i.saturating_mul(self.samples).checked_div(self.cells);
            u32::try_from(scaled.unwrap_or(0).min(self.samples)).unwrap_or(u32::MAX)
        };
        let start = edge(u64::from(index));
        (start, edge(u64::from(index).saturating_add(1)).max(start))
    }
}

/// `numerator / denominator`, rounded to nearest and clamped into a byte.
///
/// Truncating instead would darken every reduced image by up to one level per component,
/// which is invisible on one image and is a systematic bias across a page of them.
fn round_div(numerator: u64, denominator: u64) -> u8 {
    if denominator == 0 {
        return 0;
    }
    let rounded = numerator
        .saturating_add(denominator / 2)
        .checked_div(denominator)
        .unwrap_or(0);
    u8::try_from(rounded.min(u64::from(u8::MAX))).unwrap_or(u8::MAX)
}

#[cfg(test)]
#[expect(
    clippy::arithmetic_side_effects,
    clippy::cast_possible_truncation,
    reason = "test code: every index and sample value here is a literal under a hundred, \
              so the arithmetic is checkable by eye and a fallible form would only obscure \
              what each fixture contains"
)]
mod resampling {
    use super::{Bands, Image, Transform};

    /// An image of `width` x `height` opaque samples, each carrying `f(x, y)` in red.
    fn image(width: u32, height: u32, f: impl Fn(u32, u32) -> u8) -> Image {
        let mut data = Vec::new();
        for y in 0..height {
            for x in 0..width {
                data.extend_from_slice(&[f(x, y), 0, 0, 255]);
            }
        }
        Image {
            width,
            height,
            data: data.into(),
            interpolate: false,
        }
    }

    /// The transform that draws the unit square across `across` x `down` device pixels.
    fn drawn_at(across: f32, down: f32) -> Transform {
        Transform::scale(across, down)
    }

    /// Reads the red channel of one sample.
    fn red(image: &Image, x: u32, y: u32) -> u8 {
        let at = ((y as usize) * (image.width as usize) + (x as usize)) * 4;
        image.data[at]
    }

    /// ISO 32000-2 §8.9.5.3: `/Interpolate` is a hint about the *magnified* case only.
    ///
    /// > Image interpolation is an attempt to produce a smooth transition between adjacent
    /// > sample values when rendering an image whose resolution is significantly lower than
    /// > that of the output device.
    ///
    /// That sentence is what makes this two questions rather than one: the whole clause is
    /// about magnification, and Table 87 defaults the entry to false. So a magnified image is
    /// drawn sample by flat sample unless the entry asks otherwise, and a *reduced* one is
    /// filtered whatever the entry says, because §10.7.4 rather than this clause governs
    /// there and several samples share a pixel regardless.
    ///
    /// Written as a table of both flags against both regimes, because the defect this guards
    /// is one arm of it: reading `/Interpolate` as the whole answer turns every reduced image
    /// into a point sample, and ignoring it turns every magnified one into a blur. Neither
    /// reports anything.
    #[test]
    fn interpolate_decides_a_magnified_image_and_nothing_else() {
        let plain = image(8, 8, |x, _| (x * 30) as u8);
        let asked = Image {
            interpolate: true,
            ..image(8, 8, |x, _| (x * 30) as u8)
        };

        // Magnified: eight samples across sixty-four device pixels.
        assert!(
            !plain.is_smoothed(drawn_at(64.0, 64.0)),
            "the default is flat rectangles, which is what the clause's `false` means"
        );
        assert!(
            asked.is_smoothed(drawn_at(64.0, 64.0)),
            "and the entry is what turns filtering on"
        );

        // Reduced: eight samples across four device pixels. §10.7.4's territory, not this
        // clause's, so the flag decides nothing.
        assert!(plain.is_smoothed(drawn_at(4.0, 4.0)));
        assert!(asked.is_smoothed(drawn_at(4.0, 4.0)));

        // Drawn at its own size, neither regime applies and neither answer can be seen.
        assert!(plain.is_smoothed(drawn_at(8.0, 8.0)));
    }

    /// An image drawn at its own size, or larger, keeps every sample it has.
    ///
    /// The cheap half of the decision, and the one that must stay cheap: it is asked for
    /// every image on every page, and the great majority are not reduced at all.
    #[test]
    fn an_image_that_is_not_reduced_is_not_resampled() {
        let source = image(8, 8, |x, _| (x * 30) as u8);
        assert!(source.area_averaged(drawn_at(8.0, 8.0)).is_none());
        assert!(source.area_averaged(drawn_at(64.0, 64.0)).is_none());
        // Just under two device pixels per sample in one axis is still not a reduction
        // this needs to help with: four taps see both samples.
        assert!(source.area_averaged(drawn_at(5.0, 8.0)).is_none());
    }

    /// Four samples reduced to one become their mean, not one of them.
    ///
    /// This is the whole of the departure from §10.7.4, at the smallest size it can be
    /// stated: point sampling would return one of `0`, `10`, `20`, `30`.
    #[test]
    fn a_block_of_samples_becomes_their_mean() {
        let source = image(2, 2, |x, y| ((y * 2 + x) * 10) as u8);
        let reduced = source
            .area_averaged(drawn_at(1.0, 1.0))
            .expect("a two-fold reduction in both axes");

        assert_eq!((reduced.width, reduced.height), (1, 1));
        assert_eq!(red(&reduced, 0, 0), 15, "the mean of 0, 10, 20 and 30");
    }

    /// A row that reduces and a column that does not are treated independently.
    #[test]
    fn each_axis_is_reduced_by_its_own_factor() {
        let source = image(8, 2, |x, _| (x * 10) as u8);
        let reduced = source
            .area_averaged(drawn_at(2.0, 2.0))
            .expect("a four-fold reduction across only");

        assert_eq!((reduced.width, reduced.height), (2, 2));
        assert_eq!(red(&reduced, 0, 0), 15, "the mean of 0, 10, 20 and 30");
        assert_eq!(red(&reduced, 1, 0), 55, "the mean of 40, 50, 60 and 70");
    }

    /// A single dark row survives a reduction that no four-tap filter would show.
    ///
    /// This is `bug1001080.pdf` in miniature: 53 rows drawn about five pixels high, with
    /// the crossbar of a `t` in one of them. Point sampling keeps it only if the centre
    /// happens to land on it; a bilinear filter reads the four neighbours of that centre
    /// and never reaches row 26 at all. An area average always carries it, at 1/11 of its
    /// weight, which is what makes the glyph legible rather than exact.
    #[test]
    fn one_dark_row_in_eleven_still_darkens_its_output_row() {
        let source = image(11, 11, |_, y| if y == 5 { 0 } else { 255 });
        let reduced = source
            .area_averaged(drawn_at(1.0, 1.0))
            .expect("an eleven-fold reduction");

        assert_eq!((reduced.width, reduced.height), (1, 1));
        let ink = red(&reduced, 0, 0);
        assert!(
            (200..255).contains(&ink),
            "one black row in eleven should darken the result without blanking it, got {ink}"
        );
    }

    /// Colour is averaged weighted by alpha, so a transparent sample contributes none of it.
    ///
    /// Straight-alpha components carry whatever the encoder stored under a fully
    /// transparent sample, and averaging them directly would let that reach the page.
    #[test]
    fn a_transparent_sample_contributes_no_colour() {
        let mut data = Vec::new();
        // Opaque red, then transparent green — whose green is a value nothing may show.
        data.extend_from_slice(&[255, 0, 0, 255]);
        data.extend_from_slice(&[0, 255, 0, 0]);
        let source = Image {
            width: 2,
            height: 1,
            data: data.into(),
            interpolate: false,
        };

        let reduced = source
            .area_averaged(drawn_at(1.0, 1.0))
            .expect("a two-fold reduction across");
        assert_eq!(
            (
                red(&reduced, 0, 0),
                reduced.data[1],
                reduced.data[2],
                reduced.data[3]
            ),
            (255, 0, 0, 128),
            "the surviving colour is the opaque sample's, at half coverage"
        );
    }

    /// A block with nothing but transparent samples has no colour to recover.
    #[test]
    fn a_wholly_transparent_block_stays_transparent() {
        // Four samples carrying a colour under an alpha of zero, which nothing may show.
        let source = Image {
            width: 2,
            height: 2,
            data: [9, 9, 9, 0].repeat(4).into(),
            interpolate: false,
        };
        let reduced = source
            .area_averaged(drawn_at(1.0, 1.0))
            .expect("a two-fold reduction");
        assert_eq!(reduced.data[..4], [0, 0, 0, 0]);
    }

    /// The reduced image covers exactly the region of the unit square the original did.
    ///
    /// The bands tile the source grid with no gap and no overlap, which is what stops the
    /// last, short block of a dimension that does not divide evenly from being stretched
    /// across a whole output cell. That defect is worth a test rather than a comment: it
    /// moved `firefox_logo.pdf` further from three references than doing nothing did, and
    /// it is invisible in every picture smaller than the one that showed it.
    #[test]
    fn the_bands_tile_their_axis_exactly() {
        for (samples, cells) in [(512u32, 103u32), (543, 109), (39, 5), (11, 1), (7, 7)] {
            let bands = Bands::new(samples, cells);
            let mut previous_end = 0;
            for index in 0..cells {
                let (start, end) = bands.at(index);
                assert_eq!(start, previous_end, "{samples}/{cells} band {index} starts");
                assert!(end > start, "{samples}/{cells} band {index} is non-empty");
                previous_end = end;
            }
            assert_eq!(previous_end, samples, "{samples}/{cells} covers its axis");
        }
    }

    /// Reduction is measured along the image's own axes, which rotation carries with it.
    #[test]
    fn a_rotated_image_is_reduced_by_the_length_of_its_own_edges() {
        let source = image(16, 4, |x, _| (x * 16) as u8);
        // A quarter turn: the image's 16-sample axis now runs down the device, four pixels
        // long, and its 4-sample axis runs across, four pixels long.
        let quarter_turn = Transform::new(0.0, 4.0, -4.0, 0.0, 0.0, 0.0);
        let reduced = source
            .area_averaged(quarter_turn)
            .expect("the long axis is reduced four-fold");
        assert_eq!(
            (reduced.width, reduced.height),
            (4, 4),
            "the 16-sample axis reduces and the 4-sample one does not"
        );
    }

    /// A degenerate transform asks for no reduction rather than for a division by zero.
    #[test]
    fn a_collapsed_transform_resamples_nothing_rather_than_dividing_by_zero() {
        let source = image(4, 4, |x, _| (x * 60) as u8);
        assert!(source.area_averaged(Transform::scale(0.0, 0.0)).is_none());
        assert!(
            source
                .area_averaged(Transform::scale(f32::NAN, f32::NAN))
                .is_none()
        );
    }
}

#[cfg(test)]
#[expect(
    clippy::float_cmp,
    reason = "test code: the widths here are exactly representable, and the whole point is \
              that the substituted width is the reciprocal of the scale rather than near it"
)]
mod stroke_width {
    use super::{Stroke, Transform};

    /// A zero width is one device pixel, expressed in the path's own space.
    ///
    /// ISO 32000-2 §8.4.3.2: "A line width of 0 shall denote the thinnest line that can be
    /// rendered at device resolution: 1 device pixel wide." At more than one scale, because a
    /// single scale cannot tell a reciprocal from a constant — the defect this class of test
    /// exists to catch, per trap 2.
    #[test]
    fn a_zero_width_is_one_device_pixel() {
        let stroke = Stroke {
            width: 0.0,
            ..Stroke::default()
        };
        assert_eq!(stroke.device_width(Transform::IDENTITY), 1.0);
        assert_eq!(stroke.device_width(Transform::scale(2.0, 2.0)), 0.5);
        assert_eq!(stroke.device_width(Transform::scale(0.25, 0.25)), 4.0);
    }

    /// Without `/SA`, a thin stroke stays as thin as the document asked.
    ///
    /// §10.7.5's substitution is conditional on the parameter, and this is the half a
    /// document that never says `/SA` relies on: a 0.1-unit hairline is drawn at a tenth of a
    /// pixel's coverage, not promoted to a whole one.
    #[test]
    fn a_thin_stroke_is_left_alone_without_stroke_adjustment() {
        let stroke = Stroke {
            width: 0.1,
            ..Stroke::default()
        };
        assert_eq!(stroke.device_width(Transform::IDENTITY), 0.1);
        assert_eq!(stroke.device_width(Transform::scale(2.0, 2.0)), 0.1);
    }

    /// With `/SA`, a stroke under half a device pixel becomes a single-pixel line.
    ///
    /// §10.7.5: "If stroke adjustment is enabled and the requested line width, transformed
    /// into device space, is less than half a pixel, the stroke shall be rendered as a
    /// single-pixel line." The test is at the boundary in both directions and at two scales,
    /// because the condition is on the *device* width: 0.4 units is under half a pixel at
    /// scale 1 and over it at scale 2, so the same stroke must be adjusted in one case and
    /// left alone in the other.
    #[test]
    fn stroke_adjustment_promotes_a_sub_half_pixel_line() {
        let thin = Stroke {
            width: 0.4,
            adjust: true,
            ..Stroke::default()
        };
        assert_eq!(thin.device_width(Transform::IDENTITY), 1.0);
        assert_eq!(thin.device_width(Transform::scale(2.0, 2.0)), 0.4);

        let thick = Stroke {
            width: 0.5,
            adjust: true,
            ..Stroke::default()
        };
        assert_eq!(thick.device_width(Transform::IDENTITY), 0.5);
    }

    /// A degenerate transform leaves the width alone rather than dividing by zero.
    ///
    /// The path has collapsed to a line or a point, so there is no device width to compare
    /// against; returning the field is the only answer that is not an infinity.
    #[test]
    fn a_collapsed_transform_leaves_the_width_alone() {
        let stroke = Stroke {
            width: 0.0,
            adjust: true,
            ..Stroke::default()
        };
        assert_eq!(stroke.device_width(Transform::scale(0.0, 0.0)), 0.0);
    }
}

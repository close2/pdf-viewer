//! Colour, fill rules, and stroke parameters.

use std::borrow::Cow;
use std::sync::Arc;

use rayon::prelude::*;

use crate::geom::Transform;

/// Source samples above which [`Image::area_averaged`] divides its output rows across
/// rayon's pool rather than walking them.
///
/// Measured rather than chosen — `examples/area_bench`, best of 100 runs a size, with the
/// mean beside it because a division's overhead shows up there first:
///
/// | source samples | serial | divided regardless |
/// |---|---|---|
/// | 64×64 = 4 096 | **0.008 ms** (mean 0.010) | 0.012 (0.031) |
/// | 128×128 = 16 384 | 0.052 (0.054) | **0.026** (0.043) |
/// | **256×256 = 65 536** | 0.207 (0.216) | **0.035** (0.065) |
/// | 512×512 = 262 144 | 0.459 (0.522) | **0.088** (0.149) |
///
/// The crossover is between the first two rows and the floor is set at the *third*, which is
/// deliberate: a floor belongs where the division starts being worth having rather than where
/// it starts being faster. Below 65 536 samples the whole reduction costs a fifth of a
/// millisecond, and the threads would be woken inside `render-cpu`'s strips — where the pool
/// is already saturated and this benchmark cannot see it.
const PARALLEL_FLOOR: u64 = 65_536;

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

    /// Creates an opaque grey, whose [`Self::grey_level`] is the value given.
    #[must_use]
    pub const fn grey(level: f32) -> Self {
        Self::rgb(level, level, level)
    }

    /// This colour's grey level, ignoring its alpha (ISO 32000-2 §10.4.2.2).
    ///
    /// > The gray value for a given RGB value shall be computed according to the NTSC video
    /// > standard, which determines how a colour television signal is rendered on a
    /// > black-and-white television set:
    ///
    /// The formula the clause then prints, whose weights are set out below the quotation
    /// because the standard sets them in mathematical italics that no transcription survives:
    /// `gray = 0.3 × red + 0.59 × green + 0.11 × blue`.
    ///
    /// §11.5.3's EXAMPLE 2 prints the same three weights for a `/Luminosity` soft mask, which
    /// is why this lives on the colour rather than in the mask: `pdf_model`'s
    /// `ColourSpace::ink` needs the same number for a colour it has *not* yet painted, and
    /// two copies of a three-constant formula is exactly the shape that made three
    /// `DeviceCMYK` conversions disagree in this tree once.
    ///
    /// Not the sRGB or Rec. 709 luminance coefficients both rasterisers offer: the clause
    /// states these three, so a library's own are a different formula rather than the same
    /// one.
    #[must_use]
    pub fn grey_level(&self) -> f32 {
        0.30_f32.mul_add(self.r, 0.59_f32.mul_add(self.g, 0.11 * self.b))
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
    Shading(Arc<crate::shading::Shading>),
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
        // A degenerate transform has collapsed the path to a line or a point; there is no
        // width in device space to compare against and nothing to divide by.
        let Some(one_pixel) = thinnest_line(to_device) else {
            return self.width;
        };
        // §10.7.5's "less than half a pixel", asked in the path's own space rather than in
        // device space, because that is the space both sides of the comparison are already in
        // and it costs no second decomposition of the transform.
        if self.width <= 0.0 || (self.adjust && self.width < 0.5 * one_pixel) {
            one_pixel
        } else {
            self.width
        }
    }
}

/// One device pixel, expressed in the space of a path that `to_device` maps to the device.
///
/// ISO 32000-2 §8.4.3.2 states the device's thinnest mark in *device* pixels — "1 device pixel
/// wide" — while every width this renderer hands a backend is in the path's own space, so the
/// quantity has to be carried across the transform. Both of this crate's readings of "the
/// thinnest thing this device can put down" resolve to it: [`Stroke::device_width`] for a line
/// §8.4.3.2 or §10.7.5 asks to be one pixel thick, and [`split_collapsed_fill`] for the fill
/// §10.7.4 says may not disappear. One function so that the two cannot drift apart, and so that
/// a backend never divides by a stretch of its own.
///
/// Returns `None` where the transform is singular or not finite: a path collapsed to a point
/// has no space of its own left for a width to be stated in.
///
/// [`split_collapsed_fill`]: crate::collapsed::split_collapsed_fill
#[must_use]
pub fn thinnest_line(to_device: Transform) -> Option<f32> {
    let stretch = to_device.max_stretch();
    if !stretch.is_finite() || stretch <= 0.0 {
        return None;
    }
    Some(1.0 / stretch)
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

/// What reducing an image under one placement would produce, decided without producing it.
///
/// [`Image::reduction`] answers it and [`Image::area_averaged`] carries it out, which is the
/// split that lets a backend cache a reduced raster: every field here is a function of the
/// source samples and the placement, so two placements with the same [`Reduction::factors`]
/// over the same source ask for the same bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Reduction {
    /// How many whole source samples share one device pixel, across and down.
    ///
    /// At least one of the two is greater than one — a reduction that reduces nothing is
    /// [`Image::reduction`]'s `None` rather than a factor pair of ones.
    pub factors: (u32, u32),
    /// Width of the reduced grid in samples.
    pub width: u32,
    /// Height of the reduced grid in samples.
    pub height: u32,
    /// What [`Image::is_smoothed`] answers about the *reduced* grid under the same placement.
    pub smoothed: bool,
}

/// How many whole source samples of an axis `samples` long share one device pixel, where the
/// axis covers `device` pixels.
///
/// One where nothing gathers — including where the placement is degenerate or not a number,
/// which is a collapsed image rather than a reduced one.
///
/// **`samples` must not be zero**: the clamp's minimum would then exceed its maximum, and
/// `f32::clamp` panics outright on that. [`Image::reduction`] is the only caller and rules it
/// out with [`Image::is_consistent`] before asking.
#[expect(
    clippy::cast_precision_loss,
    reason = "an image's dimensions are bounded well below f32's exact integer range \
              by the decoder's own sample limit"
)]
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "the ratio is clamped to 1.0 ..= the dimension it came from before the \
              cast, so it is a positive number no larger than a u32 the image's own \
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

/// Whether a backend should filter between the samples of a `width` × `height` grid.
///
/// One statement of the rule, asked by [`Image::is_smoothed`] about an image's own grid and by
/// [`Image::reduction`] about the reduced one — which is a grid no `Image` exists for yet.
fn smoothed(width: u32, height: u32, interpolate: bool, placement: Transform) -> bool {
    if interpolate {
        return true;
    }
    let across = crate::geom::length(placement.a, placement.b);
    let down = crate::geom::length(placement.c, placement.d);
    #[expect(
        clippy::cast_precision_loss,
        reason = "an image's dimensions are bounded well below f32's exact integer range \
                  by the decoder's own sample limit, and a pixel either way cannot change \
                  which side of this comparison a real image falls"
    )]
    let magnified = across > width as f32 || down > height as f32;
    !magnified
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
    pub data: Arc<[u8]>,
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
        smoothed(self.width, self.height, self.interpolate, placement)
    }

    /// The grid [`Image::area_averaged`] would produce under `placement`, without producing it.
    ///
    /// `None` where it would produce none: an image whose dimensions and buffer disagree, or
    /// one no device pixel gathers two samples of.
    ///
    /// **This is the cache key a backend needs and cannot otherwise have.** The reduced
    /// samples are a pure function of the source samples and the two factors, so a backend
    /// holding a reduced raster can decide whether the one it holds is the one this placement
    /// asks for *before* paying the reduction's cost — which is per source sample and is
    /// therefore the whole of a scanned page's redraw (ADR 0297). Answering it is two vector
    /// lengths and two divisions.
    #[must_use]
    pub fn reduction(&self, placement: Transform) -> Option<Reduction> {
        // Consistency is asked **before** the factors rather than beside them, and that
        // ordering is load-bearing: `factor` clamps a ratio into `1.0 ..= width`, and
        // `f32::clamp` panics outright when its minimum exceeds its maximum — which a
        // zero-width image makes it do.
        if !self.is_consistent() {
            return None;
        }

        // `placement` maps the unit square onto the device, so `geom::length` of each column
        // is the device length of that edge of the image — which holds under rotation and
        // skew as well as under a plain scale, and is the same quantity `is_smoothed`
        // measures, by the same function for the reason that function gives.
        //
        // A factor is the *floor* of the ratio, so dividing the grid by it leaves between one
        // and two source samples per device pixel: enough for a four-tap filter to see all of
        // them, which is what makes `area_averaged` and the backends' own bilinear filters
        // complementary rather than redundant.
        let factors = (
            factor(self.width, crate::geom::length(placement.a, placement.b)),
            factor(self.height, crate::geom::length(placement.c, placement.d)),
        );
        if factors.0 <= 1 && factors.1 <= 1 {
            return None;
        }
        // `div_ceil`, so the samples at a right or bottom edge keep an output cell rather
        // than being dropped.
        let width = self.width.div_ceil(factors.0);
        let height = self.height.div_ceil(factors.1);
        Some(Reduction {
            factors,
            width,
            height,
            // The reduced grid's own answer to the question `is_smoothed` asks, by the same
            // function: a backend that reused this raster would otherwise have to re-derive
            // it from dimensions it no longer holds. It is not implied by the factors —
            // a factor clamped to 1 in one axis leaves that axis magnified.
            smoothed: smoothed(width, height, self.interpolate, placement),
        })
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
    ///
    /// # What it costs, and what was done about it
    ///
    /// One source sample is read once, so the work is the *source's* size and not the
    /// reduced one — which is why a page with one 388-command photograph on it cost sixteen
    /// times the display-list translation of a 3675-command page of text (ADR 0228). Two
    /// things follow, and both are measured by `examples/area_bench`, best of 100 runs, with
    /// the column-band hoisting shown on its own so the division is not credited with it:
    ///
    /// | 3× reduction of | before | hoisted only | and divided |
    /// |---|---|---|---|
    /// | 1374×1374 | 4.57 ms | 3.50 ms | **0.65 ms** |
    /// | 2100×1448 | 7.34 ms | 5.99 ms | **0.86 ms** |
    /// | 2700×3450 | 22.39 ms | 19.81 ms | **2.93 ms** |
    ///
    /// The output is byte-identical — the example asserts it on every case it times — because
    /// neither change alters an arithmetic step. What they cost in readability is one `Vec` of
    /// column bands and one branch on a threshold, and the branch is the price of not paying
    /// rayon's split on an image too small to repay it.
    #[must_use]
    pub fn area_averaged(&self, placement: Transform) -> Option<Self> {
        // Every refusal this function makes — an inconsistent image, a placement no pixel of
        // which gathers two samples — is `Self::reduction`'s, so that a backend asking which
        // raster it would get and a backend asking for the raster cannot disagree.
        let Reduction { width, height, .. } = self.reduction(placement)?;

        // The block boundaries are then *proportional* rather than fixed multiples of the
        // factor, so the blocks tile the source grid exactly and no two differ by more than
        // one sample. Fixed multiples with a short block at the edge are the tempting version
        // and are wrong in a way that shows: `512 / 5` leaves a two-sample block occupying a
        // whole output cell, which squeezes the image into 99.4% of the unit square and moved
        // `firefox_logo.pdf` *further* from three references than no filtering at all.
        let rows = Bands::new(self.height, height);

        // Every output row asks for the same column bands, and `Bands::at` is two 64-bit
        // divisions — so asking once per *row* rather than once per cell takes 4.57 ms to
        // 3.50 ms on a 1374×1374 image reduced threefold, and 7.34 to 5.99 on a 2100×1448
        // (`examples/area_bench`, best of 100). The whole cost is one `Vec` the width of the
        // reduced image.
        let columns = Bands::new(self.width, width);
        let spans: Vec<(u32, u32)> = (0..width).map(|out_x| columns.at(out_x)).collect();

        // `row_bytes` cannot be zero, which is what the two `chunks_exact_mut` below need:
        // `is_consistent` has already refused a zero dimension, and `Self::reduction` clamps
        // each factor to at most that dimension, so `div_ceil` of it is at least one. The
        // buffer is a whole number of rows, so neither call drops a remainder either.
        let row_bytes = (width as usize).saturating_mul(4);
        let mut data: Vec<u8> = vec![0; row_bytes.saturating_mul(height as usize)];
        // A row's cells are a pure function of disjoint blocks of the source, so which
        // thread computes which row cannot change any byte of the answer — the same
        // property that made `pdf-model`'s colour conversion divisible (ADR 0147) and that
        // a rasterisation deliberately does not have (ADR 0138). [`PARALLEL_FLOOR`] carries
        // the crossover and the argument for putting the threshold above it.
        let fill = |out_y: usize, row: &mut [u8]| {
            let (y0, y1) = rows.at(u32::try_from(out_y).unwrap_or(u32::MAX));
            for (cell, &(x0, x1)) in row.chunks_exact_mut(4).zip(&spans) {
                cell.copy_from_slice(&self.average_block(x0, y0, x1, y1));
            }
        };
        if u64::from(self.width).saturating_mul(u64::from(self.height)) >= PARALLEL_FLOOR {
            data.par_chunks_exact_mut(row_bytes)
                .enumerate()
                .for_each(|(out_y, row)| fill(out_y, row));
        } else {
            for (out_y, row) in data.chunks_exact_mut(row_bytes).enumerate() {
                fill(out_y, row);
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

/// How many samples a raster drawn under a placement is wanted at, per axis.
///
/// The unit square is where every PDF image lives (§8.9.5.1), so a placement — the transform
/// carrying that square onto the device — states how many device pixels the image covers, and
/// that is the only resolution at which any question about its samples has an answer.
///
/// ISO 32000-2 §10.7.4 is where the standard says so, of a sampled image:
///
/// > The position of the centre of such a pixel -in other words, the point whose
/// > coordinate values have fractional parts of one-half -shall be mapped back into
/// > source space to determine how to colour the pixel.
///
/// One sample per device pixel is therefore the finest grid a device can distinguish, and it
/// is what [`Self::for_placement`] answers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Grid {
    /// Samples across, at least one.
    pub width: u32,
    /// Samples down, at least one.
    pub height: u32,
}

impl Grid {
    /// The grid a raster placed by `placement` is wanted at.
    ///
    /// `placement` maps the unit square onto the device, so [`crate::geom::length`] of each of
    /// its columns is the device length of that edge — the same quantity
    /// [`Image::is_smoothed`] and [`Image::area_averaged`] measure, by the same function and
    /// for the reason that function gives.
    ///
    /// Rounded **up**, so a raster asked for at this grid never has fewer samples than the
    /// pixels it will be drawn across, and clamped to at least one sample in each axis: a
    /// placement that has collapsed the square to a line still names a raster, and a grid of
    /// zero would name none.
    #[must_use]
    pub fn for_placement(placement: Transform) -> Self {
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "clamped to 1.0 ..= u32::MAX before the cast, so it is a positive number \
                      inside a u32's range"
        )]
        fn axis(device: f32) -> u32 {
            if !device.is_finite() {
                return 1;
            }
            device.ceil().clamp(1.0, f64::from(u32::MAX) as f32) as u32
        }

        Self {
            width: axis(crate::geom::length(placement.a, placement.b)),
            height: axis(crate::geom::length(placement.c, placement.d)),
        }
    }
}

/// Samples the display list names rather than holds, produced once the device scale is known.
///
/// # Why the display list cannot hold them
///
/// Two of ISO 32000-2's rasters have no grid of their own until something says how large they
/// will be drawn. §11.6.5.2's soft-mask image is the standing case: Table 143 makes its
/// `/Width` and `/Height`
///
/// > independent of it. Both images shall be mapped to the unit square in user space (as are
/// > all images), regardless of whether the samples coincide individually.
///
/// so an image and its mask can state grids that share no common refinement small enough to
/// build — `issue16263.pdf` writes a 2×2 image with a 34862×4332 mask, which is 604 MB of
/// RGBA on the finer of the two. §10.7.4 answers the question at *device* resolution, and the
/// interpreter deliberately does not know that: a display list is re-rasterisable at any zoom
/// without being interpreted again, which is what `zooming_rasterises_again_without_interpreting_again`
/// asserts. So the interpreter carries this instead, and a backend resolves it where it knows
/// the scale.
///
/// # What an implementation owes
///
/// [`Self::samples`] is asked for a grid and answers with a raster **no finer than it**, which
/// is the whole of the contract: a producer that can only offer its stated grid may return
/// that, and one that can decode at a chosen resolution — JPEG 2000's levels are the case this
/// is next owed for — may meet the request exactly. It is infallible because everything a
/// producer can check without decoding has already been checked and reported by the
/// interpreter; what is left is a decode that fails at draw time, and the answer to that is
/// the same one [`crate::Command`] gives everywhere else — draw what there is.
///
/// Implementations are `Send + Sync` because a display list is drawn on every core.
pub trait ImageAtDeviceScale: std::fmt::Debug + Send + Sync {
    /// The samples, on a grid no finer than `grid` in either axis.
    fn samples(&self, grid: Grid) -> Image;
}

/// A shared [`ImageAtDeviceScale`], so that a command carrying one stays cloneable.
#[derive(Clone)]
pub struct DeferredImage(Arc<dyn ImageAtDeviceScale>);

impl DeferredImage {
    /// Wraps a producer of samples.
    #[must_use]
    pub fn new(source: Arc<dyn ImageAtDeviceScale>) -> Self {
        Self(source)
    }

    /// The samples, on a grid no finer than `grid`.
    #[must_use]
    pub fn samples(&self, grid: Grid) -> Image {
        self.0.samples(grid)
    }
}

impl std::fmt::Debug for DeferredImage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("DeferredImage").field(&self.0).finish()
    }
}

impl PartialEq for DeferredImage {
    /// Two deferred images are the same one when they are the same object.
    ///
    /// Comparing the rasters would mean producing them, and the whole point of the type is
    /// that producing one is a decision about the device rather than about the display list.
    /// Identity is what the display list's own `PartialEq` needs — it exists so a test can say
    /// a list was rebuilt unchanged — and identity is what it gets.
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

/// Where a backend's image samples come from.
///
/// Almost every image in a document decodes to one raster on the grid the file states, and
/// [`Self::Decoded`] is that. The variant beside it is the vocabulary described on
/// [`ImageAtDeviceScale`]: an image whose samples cannot be settled until the device scale is.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum ImageSource {
    /// Samples on the grid the file states, ready to draw at any scale.
    Decoded(Image),
    /// Samples produced at whatever grid the device turns out to want.
    AtDeviceScale(DeferredImage),
}

impl ImageSource {
    /// The samples to draw under `placement`, borrowed where they already exist.
    ///
    /// `placement` maps the unit square onto the device. A [`Self::Decoded`] source borrows;
    /// a deferred one is produced at [`Grid::for_placement`], which is the one place this
    /// renderer decides what "at device resolution" means, so that the two backends cannot
    /// answer it differently — the same reason [`Image::area_averaged`] lives in this crate.
    #[must_use]
    pub fn at(&self, placement: Transform) -> Cow<'_, Image> {
        match self {
            Self::Decoded(image) => Cow::Borrowed(image),
            Self::AtDeviceScale(deferred) => {
                Cow::Owned(deferred.samples(Grid::for_placement(placement)))
            }
        }
    }

    /// Whether every sample is fully opaque, without producing any that do not yet exist.
    ///
    /// Asked by `pdf-model` about the elements of a knockout group (§11.4.6), where the
    /// question is whether an element's shape is its coverage. A deferred source answers
    /// **no**, which is not a conservative guess but the fact: the only reason this crate has
    /// a deferred source at all is a second raster contributing per-sample alpha.
    #[must_use]
    pub fn is_opaque(&self) -> bool {
        match self {
            Self::Decoded(image) => image.is_opaque(),
            Self::AtDeviceScale(_) => false,
        }
    }
}

impl From<Image> for ImageSource {
    fn from(image: Image) -> Self {
        Self::Decoded(image)
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

    /// An image with no samples answers `None` rather than panicking inside `f32::clamp`.
    ///
    /// `Image::reduction` clamps a ratio into `1.0 ..= width`, and `f32::clamp` panics when
    /// its minimum exceeds its maximum — which a zero-width image makes it do. Until session
    /// 391 the reduction was computed *before* `is_consistent`, so the only thing standing
    /// between a public method and that panic was that two of its three callers happened to
    /// ask the question first; `render-quorra`'s did not. The order is now the other way
    /// round and this is the guard on it.
    #[test]
    fn an_image_with_no_samples_is_not_reduced_and_does_not_panic() {
        for (width, height) in [(0u32, 0u32), (0, 8), (8, 0)] {
            let source = Image {
                width,
                height,
                data: Vec::new().into(),
                interpolate: false,
            };
            assert!(
                source.area_averaged(Transform::scale(4.0, 4.0)).is_none(),
                "{width}x{height}"
            );
        }
    }

    /// What a backend is told it would get is what it gets, over every regime of the two axes.
    ///
    /// The cache in `render-quorra` keeps a reduced raster under [`Image::reduction`]'s answer
    /// and never looks at the raster again, so a `Reduction` that disagreed with
    /// [`Image::area_averaged`] in any field would serve a raster of the wrong size or filter
    /// it the wrong way, on a hit, silently. The two regimes are asked *per axis* on purpose:
    /// `smoothed` is not a function of the factors, because an axis whose factor clamps to one
    /// is an axis that may be magnified while the other is reduced.
    #[test]
    fn a_reduction_describes_the_raster_the_averaging_produces() {
        let source = image(60, 36, |x, y| ((x * 3) ^ (y * 5)) as u8);
        let placements = [
            drawn_at(20.0, 12.0),
            drawn_at(20.0, 36.0),
            drawn_at(60.0, 12.0),
            drawn_at(600.0, 12.0),
            drawn_at(7.0, 5.0),
            drawn_at(60.0, 36.0),
            drawn_at(120.0, 72.0),
        ];
        for placement in placements {
            let across = placement.a;
            match (source.reduction(placement), source.area_averaged(placement)) {
                (Some(reduction), Some(reduced)) => {
                    assert_eq!(
                        (reduction.width, reduction.height),
                        (reduced.width, reduced.height),
                        "the grid at {across}"
                    );
                    assert_eq!(
                        reduction.smoothed,
                        reduced.is_smoothed(placement),
                        "the filter at {across}"
                    );
                    assert!(
                        reduction.factors.0 > 1 || reduction.factors.1 > 1,
                        "a reduction that reduces nothing is `None` at {across}"
                    );
                }
                (None, None) => {}
                (reduction, reduced) => panic!(
                    "at {across}: reduction {} and averaging {}",
                    if reduction.is_some() {
                        "answered"
                    } else {
                        "declined"
                    },
                    if reduced.is_some() {
                        "answered"
                    } else {
                        "declined"
                    },
                ),
            }
        }
    }

    /// An image the factors cannot be computed for declines both questions, and does not panic.
    #[test]
    fn an_image_with_no_samples_has_no_reduction_either() {
        for (width, height) in [(0u32, 0u32), (0, 8), (8, 0)] {
            let source = Image {
                width,
                height,
                data: Vec::new().into(),
                interpolate: false,
            };
            assert!(
                source.reduction(Transform::scale(4.0, 4.0)).is_none(),
                "{width}x{height}"
            );
        }
    }

    /// Dividing the rows across threads is the same arithmetic, so it is the same bytes.
    ///
    /// The reduction runs serially below `PARALLEL_FLOOR` and in parallel above it, and the
    /// two paths must be indistinguishable in their output — which is the whole argument for
    /// dividing it at all (ADR 0228). This straddles the floor at a size where the shape of
    /// the source is the same on both sides of it, so a difference could only be the split.
    #[test]
    fn the_divided_reduction_answers_what_the_serial_one_does() {
        let sample = |x: u32, y: u32, channel: u32| {
            u8::try_from((x.wrapping_mul(7) ^ y.wrapping_mul(13) ^ channel.wrapping_mul(29)) % 251)
                .unwrap_or(0)
        };
        let build = |side: u32| {
            let mut data = Vec::new();
            for y in 0..side {
                for x in 0..side {
                    for channel in 0..4 {
                        data.push(sample(x, y, channel));
                    }
                }
            }
            Image {
                width: side,
                height: side,
                data: data.into(),
                interpolate: false,
            }
        };
        // 192² = 36 864 samples is under the floor and 384² = 147 456 is over it, and both
        // reduce three-fold onto a grid whose bands are exactly three samples wide — so the
        // second's cells are the first's arithmetic repeated, cell for cell.
        let small = build(192);
        let large = build(384);
        let reduced_small = small
            .area_averaged(Transform::scale(64.0, 64.0))
            .expect("a three-fold reduction, serially");
        let reduced_large = large
            .area_averaged(Transform::scale(128.0, 128.0))
            .expect("a three-fold reduction, divided");
        assert_eq!((reduced_small.width, reduced_small.height), (64, 64));
        assert_eq!((reduced_large.width, reduced_large.height), (128, 128));
        for out_y in 0..64usize {
            for out_x in 0..64usize {
                let at = (out_y * 64 + out_x) * 4;
                let same = (out_y * 128 + out_x) * 4;
                assert_eq!(
                    reduced_small.data[at..at + 4],
                    reduced_large.data[same..same + 4],
                    "cell ({out_x}, {out_y})"
                );
            }
        }
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
mod device_scale {
    use std::sync::Arc;

    use super::{Grid, Image, ImageAtDeviceScale, ImageSource, Transform};

    /// A source that records nothing and answers with the grid it was asked for.
    ///
    /// One opaque white sample per cell, so a test can read a dimension off the answer and
    /// know it came from the request rather than from a raster somebody stored.
    #[derive(Debug)]
    struct AsAsked;

    impl ImageAtDeviceScale for AsAsked {
        fn samples(&self, grid: Grid) -> Image {
            let count = (grid.width as usize).saturating_mul(grid.height as usize);
            Image {
                width: grid.width,
                height: grid.height,
                data: vec![u8::MAX; count.saturating_mul(4)].into(),
                interpolate: false,
            }
        }
    }

    /// The grid is the device pixels the unit square covers, in each of its own axes.
    ///
    /// ISO 32000-2 §10.7.4 puts one sample under the centre of each device pixel, so the
    /// count is the pixels — rounded up, because a raster with fewer samples than pixels
    /// would be answering a question nobody asked. At more than one scale and with the two
    /// axes different, because a single square scale cannot tell one axis from the other.
    #[test]
    fn the_grid_is_the_device_pixels_the_unit_square_covers() {
        assert_eq!(
            Grid::for_placement(Transform::scale(120.0, 80.0)),
            Grid {
                width: 120,
                height: 80
            }
        );
        assert_eq!(
            Grid::for_placement(Transform::scale(10.25, 0.5)),
            Grid {
                width: 11,
                height: 1
            },
            "rounded up, and never to nothing"
        );
    }

    /// A quarter turn carries the axes with it, which is the same measurement
    /// [`Image::area_averaged`] makes and by the same function.
    #[test]
    fn a_rotated_placement_is_measured_along_the_squares_own_edges() {
        let quarter_turn = Transform::new(0.0, 40.0, -90.0, 0.0, 0.0, 0.0);
        assert_eq!(
            Grid::for_placement(quarter_turn),
            Grid {
                width: 40,
                height: 90
            }
        );
    }

    /// A collapsed or infinite placement asks for one sample rather than for none.
    #[test]
    fn a_degenerate_placement_still_names_a_grid() {
        for placement in [
            Transform::scale(0.0, 0.0),
            Transform::scale(f32::NAN, 4.0),
            Transform::scale(f32::INFINITY, 4.0),
        ] {
            let grid = Grid::for_placement(placement);
            assert!(grid.width >= 1 && grid.height >= 1, "{grid:?}");
        }
    }

    /// A decoded source is borrowed unchanged; a deferred one is produced at the device's
    /// grid, and produced *again* when the device's grid changes.
    ///
    /// The second half is what the whole vocabulary exists for: the interpreter does not know
    /// the scale, so a display list drawn at two magnifications has to ask twice.
    #[test]
    fn a_deferred_source_is_produced_at_the_grid_it_is_drawn_at() {
        let ready = ImageSource::Decoded(Image {
            width: 3,
            height: 2,
            data: vec![0u8; 24].into(),
            interpolate: false,
        });
        let borrowed = ready.at(Transform::scale(600.0, 400.0));
        assert_eq!(
            (borrowed.width, borrowed.height),
            (3, 2),
            "a decoded raster is what the file states, whatever the scale"
        );

        let deferred = ImageSource::AtDeviceScale(super::DeferredImage::new(Arc::new(AsAsked)));
        let small = deferred.at(Transform::scale(16.0, 9.0));
        let large = deferred.at(Transform::scale(160.0, 90.0));
        assert_eq!((small.width, small.height), (16, 9));
        assert_eq!((large.width, large.height), (160, 90));
    }

    /// A deferred source is never opaque, because the only reason one exists is a second
    /// raster contributing alpha (§11.6.5.2).
    ///
    /// §11.4.6's knockout report asks this of every element, and it may not decode anything
    /// to answer: a knockout group is a question about the display list, not about a device.
    #[test]
    fn a_deferred_source_is_not_opaque() {
        let deferred = ImageSource::AtDeviceScale(super::DeferredImage::new(Arc::new(AsAsked)));
        assert!(!deferred.is_opaque());
        assert!(
            deferred.at(Transform::scale(4.0, 4.0)).is_opaque(),
            "even where every sample it produces happens to be"
        );
    }

    /// Two handles on one producer are the same source; two producers are not.
    ///
    /// `Command`'s own `PartialEq` is what a test uses to say a display list was rebuilt
    /// unchanged, and comparing the rasters would mean producing them.
    #[test]
    fn two_handles_on_one_producer_compare_equal() {
        let shared: Arc<dyn ImageAtDeviceScale> = Arc::new(AsAsked);
        let one = super::DeferredImage::new(Arc::clone(&shared));
        let same = super::DeferredImage::new(shared);
        let other = super::DeferredImage::new(Arc::new(AsAsked));
        assert_eq!(one, same);
        assert_ne!(one, other);
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

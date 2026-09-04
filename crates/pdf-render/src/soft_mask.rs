//! Position-dependent opacity, derived from a transparency group (ISO 32000-2 §11.5).
//!
//! A soft mask is the third source of shape and opacity §11.6.4.1 names, beside the
//! object's own and the graphics state's constants, and the only one that varies across the
//! page. §11.5.1:
//!
//! > Such an independent source, called a soft mask , defines values that may vary across
//! > different points on the page.
//!
//! It is defined by drawing a transparency group and taking either its alpha (§11.5.2) or
//! the luminosity of its colour over a chosen backdrop (§11.5.3). That is why this type
//! carries a command list rather than a raster: the group is evaluated at *device*
//! resolution, which the display list deliberately does not know, so a backend rasterises
//! it exactly as it rasterises the page.
//!
//! # One derivation, two backends
//!
//! [`SoftMask::values`] turns rendered pixels into mask values, and both backends call it.
//! That is the same rule as [`crate::Image::area_averaged`]: a decision the CPU oracle and
//! the GPU backend could make differently is a decision that belongs here, once.

use crate::display_list::Command;
use crate::paint::Color;

/// Identifies a soft mask within a [`crate::DisplayList`].
///
/// Referenced rather than carried for the same reason a [`crate::ClipId`] is: a mask set by
/// `gs` applies to every object painted until it is replaced, and its group may hold
/// thousands of commands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SoftMaskId(u32);

impl SoftMaskId {
    /// Creates an identifier for the mask at `index`.
    ///
    /// Only [`crate::DisplayList::add_soft_mask`] should mint one; it is public so that a
    /// backend's tests can name a mask they registered themselves.
    #[must_use]
    pub fn new(index: u32) -> Self {
        Self(index)
    }

    /// Returns the index this identifier refers to.
    #[must_use]
    pub fn index(self) -> usize {
        self.0 as usize
    }
}

/// Which of §11.5's two derivations turns a group into mask values.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SoftMaskKind {
    /// §11.5.2: the group's alpha, its colour ignored. §11.6.5.1 says it of the key:
    ///
    /// > If the subtype is Alpha , the transparency group XObject G shall be evaluated to
    /// > compute a group alpha only. The colours of the constituent objects shall be ignored
    /// > and the colour compositing computations may be omitted.
    Alpha,
    /// §11.5.3: the luminosity of the group composited onto an opaque backdrop.
    ///
    /// > The second method of deriving a soft mask from a transparency group shall begin by
    /// > compositing the group with a fully opaque backdrop of a specified colour. The mask
    /// > value at any given point shall then be defined to be the luminosity of the
    /// > resulting colour.
    Luminosity {
        /// §11.6.5.1's `/BC`, resolved into whatever the group's elements are painted in.
        ///
        /// > Outside the transparency group's bounding box, the mask value shall be derived
        /// > by transforming the BC colour to luminosity and applying the transfer function
        /// > to the result.
        ///
        /// Ordinarily that is the backdrop's own colour, in the device's three components,
        /// which is what a group composited in `DeviceRGB` or in a CIE-based space needs.
        /// **Where the group's blending colour space is subtractive it is a grey**, because
        /// `pdf_model` paints such a group in the ink §10.4.2.3 weighs rather than in colour:
        /// a backdrop and the elements composited onto it have to be the same quantity or the
        /// compositing is not the clause's, and this is the field where they meet. What
        /// remains of §10.4.2.3 — the `min`, which §11.5.3 applies only after the compositing
        /// — is in [`Transfer`], which is why that field carries more than Table 142's `/TR`.
        ///
        /// The default is "the colour space's initial value, representing black", which is
        /// what makes the area outside a mask group's own marks mask everything away.
        backdrop: Color,
    },
}

/// Everything between the mask group's rendered channel and the mask value, sampled onto the
/// 256 values an eight-bit mask can hold.
///
/// That is §11.6.5.1's `/TR` —
///
/// > A function object (see 7.10, "Functions") specifying the transfer function that shall
/// > be used in deriving the mask values. The function shall accept one input, the computed
/// > group alpha or luminosity (depending on the value of the subtype S ), and shall return
/// > one output, the resulting mask value.
///
/// — and, for a group `pdf_model` painted in §10.4.2.3's ink rather than in colour, the rest
/// of §11.5.3's own arithmetic composed ahead of it: the `min` the clause applies *after* the
/// compositing. Composed into one table on purpose, because a backend that expresses a
/// luminosity mask natively computes the luminosity itself and applies this table to the
/// result — so a step outside the table would be a step one backend takes and the other does
/// not (`pdf_model::soft_mask`'s `derivation`).
///
/// A table rather than the function itself for the same reason [`crate::Ramp`] is one: a
/// backend has no business evaluating a PDF function, and colour and opacity are resolved
/// upstream so the two backends cannot disagree about them. The sampling is *exact* rather
/// than an approximation, because a mask value here is one byte and the table holds every
/// byte the function can be asked about.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Transfer {
    table: [u8; 256],
}

impl Transfer {
    /// Builds a table from a function already evaluated at every eight-bit input.
    ///
    /// `outputs[i]` is the function's value at `i / 255`, clamped to `0..=255` — §11.6.5.1
    /// requires exactly that clamp: "if it falls outside this range, it shall be forced to
    /// the nearest valid value".
    #[must_use]
    pub fn from_samples(table: [u8; 256]) -> Self {
        Self { table }
    }

    /// Returns the mask value for a computed alpha or luminosity.
    #[must_use]
    pub fn apply(&self, value: u8) -> u8 {
        // Indexing a 256-entry table by a `u8` cannot be out of bounds; `get` keeps that
        // fact from resting on the reader's memory rather than on the compiler's.
        self.table.get(value as usize).copied().unwrap_or(value)
    }

    /// Whether this table is the identity, which is what `/Identity` and an absent `/TR`
    /// mean.
    ///
    /// Asked by a backend that can express a mask natively but not an arbitrary curve.
    #[must_use]
    pub fn is_identity(&self) -> bool {
        self.table
            .iter()
            .enumerate()
            .all(|(index, &value)| usize::from(value) == index)
    }
}

/// §11.5.3's luminosity of a colour composited in a CIE-based blending colour space
/// (ISO 32000-2 §11.5.3).
///
/// > For CIE-based spaces, convert to the CIE 1931 XYZ space and use the Y component as the
/// > luminosity. This produces a colorimetrically correct luminosity.
///
/// The clause states one derivation and the space decides its *shape*, which is why this is
/// two shapes rather than two clauses:
///
/// - **Three curves summed.** The clause's own EXAMPLE 1 writes the formula out for `CalRGB`
///   as the sum of the three components, each decoded by its gamma and weighted by its `Y` in
///   the space's `Matrix` — a sum of three functions of one component each. A matrix profile's
///   `Y` is the same shape, its tone curves weighted by the middle row of its matrix.
/// - **A sampled grid over three components.** A profile whose conversion is a lookup table
///   has no such decomposition: its `Y` is one function of all three components at once, and
///   EXAMPLE 1's "[a]n analogous computation applies to other CIE-based colour spaces" is that
///   function rather than a licence to drop it. So it is sampled over `side³` points and
///   interpolated trilinearly, which is exactly how the same profile's conversion *out* is
///   carried ([`crate::ColourCube`]) and to the same fidelity — a property of the profile's
///   own smoothness between its grid points.
/// - **A sampled grid over four components.** §11.3.4 lists an `ICCBased` bi-directional
///   'CMYK' space among the blending colour spaces, and §8.6.5.1 makes it CIE-based, so
///   §11.5.3 asks the same `Y` of it. Four components are not three, so the group is
///   §11.4.7's *pair* of rasters ([`SoftMask::black`]) and the grid has a fourth axis,
///   interpolated over sixteen corners on the same `side` and in the same index order as
///   [`crate::BlendingSpace`]'s own — cyan running fastest and black slowest, because it is
///   sampled at the very points that space is.
///
/// Either way a mask group `pdf_model` painted in such a space's own components hands the
/// backend one of these and the luminosity of a composited pixel is read off it, clamped to
/// the unit a mask value holds.
///
/// The curves are sampled at 256 points per component so that a channel at full alpha lands
/// on a sample exactly, and interpolated between them where the backdrop's composite leaves a
/// channel between two.
#[derive(Debug, Clone, PartialEq)]
pub struct Luminance {
    /// Which of the two shapes above this is.
    shape: Shape,
}

/// The two shapes a [`Luminance`] takes, whose invariants [`Luminance`]'s constructors are
/// what establish — which is why this is private.
#[derive(Debug, Clone, PartialEq)]
enum Shape {
    /// `curves[i][axis]` is that axis's share of `Y` at the component `i ÷ 255`.
    Curves(std::sync::Arc<[[f32; 3]; 256]>),
    /// `side^axes` samples of `Y`, the first component running fastest and the last slowest,
    /// so index 0 is every component at 0 and the last is every component at 1. At least two
    /// samples an axis, exactly `side^axes` of them, and `axes` is three or four.
    Grid {
        /// How many components the grid is indexed by: three or four.
        axes: usize,
        /// How many samples the grid holds along each axis; at least two.
        side: usize,
        /// The samples themselves, in the index order above.
        samples: std::sync::Arc<[f32]>,
    },
}

impl Luminance {
    /// The luminosity of a space whose `Y` is a sum of one function of each component.
    #[must_use]
    pub fn curves(curves: std::sync::Arc<[[f32; 3]; 256]>) -> Self {
        Self {
            shape: Shape::Curves(curves),
        }
    }

    /// The luminosity of a space whose `Y` is one function of all three components, sampled
    /// over a grid — or `None` if the samples are not one.
    ///
    /// The two conditions are the ones [`Luminance::of`] depends on and neither is checked
    /// again: at least two samples per axis, and exactly `side³` of them.
    #[must_use]
    pub fn grid(side: usize, samples: std::sync::Arc<[f32]>) -> Option<Self> {
        Self::over(3, side, samples)
    }

    /// The luminosity of a space of **four** components, sampled over a grid of `side⁴`
    /// points — or `None` if the samples are not one.
    ///
    /// The same conditions one axis up, and the same index order as
    /// [`crate::BlendingSpace`]: cyan fastest, black slowest.
    #[must_use]
    pub fn ink_grid(side: usize, samples: std::sync::Arc<[f32]>) -> Option<Self> {
        Self::over(4, side, samples)
    }

    /// The checked constructor both grid shapes are.
    fn over(axes: usize, side: usize, samples: std::sync::Arc<[f32]>) -> Option<Self> {
        let wanted = side.checked_pow(u32::try_from(axes).ok()?)?;
        (side >= 2 && samples.len() == wanted).then_some(Self {
            shape: Shape::Grid {
                axes,
                side,
                samples,
            },
        })
    }

    /// The three curves, where this is the summed shape.
    #[must_use]
    pub fn as_curves(&self) -> Option<&[[f32; 3]; 256]> {
        match &self.shape {
            Shape::Curves(curves) => Some(curves),
            Shape::Grid { .. } => None,
        }
    }

    /// The grid's side and its samples, where this is the three-component sampled shape.
    #[must_use]
    pub fn as_grid(&self) -> Option<(usize, &[f32])> {
        match &self.shape {
            Shape::Grid {
                axes: 3,
                side,
                samples,
            } => Some((*side, samples)),
            Shape::Curves(_) | Shape::Grid { .. } => None,
        }
    }

    /// The grid's side and its samples, where this is the four-component sampled shape.
    #[must_use]
    pub fn as_ink_grid(&self) -> Option<(usize, &[f32])> {
        match &self.shape {
            Shape::Grid {
                axes: 4,
                side,
                samples,
            } => Some((*side, samples)),
            Shape::Curves(_) | Shape::Grid { .. } => None,
        }
    }

    /// How many components this `Y` is a function of: three, or four.
    ///
    /// What tells a backend whether the mask group is one raster or §11.4.7's pair, without
    /// its having to ask which shape carries the samples.
    #[must_use]
    pub fn axes(&self) -> usize {
        match &self.shape {
            Shape::Curves(_) => 3,
            Shape::Grid { axes, .. } => *axes,
        }
    }

    /// The `Y` of a colour, `components` holding as many of the space's components as
    /// [`Luminance::axes`] says. A component this has no entry for is read as 0.0.
    #[must_use]
    pub fn of(&self, components: &[f32]) -> f32 {
        let at = |index: usize| components.get(index).copied().unwrap_or(0.0);
        let luminosity = match &self.shape {
            Shape::Curves(curves) => {
                let mut sum = 0.0f32;
                for (axis, component) in [at(0), at(1), at(2)].into_iter().enumerate() {
                    let scaled = component.clamp(0.0, 1.0) * 255.0;
                    #[expect(
                        clippy::cast_possible_truncation,
                        clippy::cast_sign_loss,
                        reason = "`scaled` is in 0..=255, so its floor is a valid index"
                    )]
                    let low = (scaled as usize).min(254);
                    #[expect(clippy::cast_precision_loss, reason = "an index below 255")]
                    let fraction = scaled - low as f32;
                    let at = |index: usize| {
                        curves
                            .get(index)
                            .and_then(|sample| sample.get(axis))
                            .copied()
                            .unwrap_or(0.0)
                    };
                    sum += at(low).mul_add(1.0 - fraction, at(low.saturating_add(1)) * fraction);
                }
                sum
            }
            Shape::Grid {
                axes,
                side,
                samples,
            } => {
                let cells: Vec<(usize, [f32; 2])> =
                    (0..*axes).map(|axis| cell_of(*side, at(axis))).collect();
                multilinear(*side, samples, &cells)
            }
        };
        luminosity.clamp(0.0, 1.0)
    }
}

/// Which cell of an axis a component falls in, and the weights of that cell's two ends.
///
/// Split out of the interpolation below because both the three- and the four-component grid
/// ask it once per axis, and it is the one place a component is clamped to the unit interval.
fn cell_of(side: usize, value: f32) -> (usize, [f32; 2]) {
    let last = side.saturating_sub(1);
    #[expect(
        clippy::cast_precision_loss,
        reason = "a grid side and a cell index, both far below f32's exact range"
    )]
    let scaled = value.clamp(0.0, 1.0) * last as f32;
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "`scaled` is in 0..=last, so its floor is a valid index"
    )]
    let cell = (scaled as usize).min(last.saturating_sub(1));
    #[expect(
        clippy::cast_precision_loss,
        reason = "a cell index below the grid side"
    )]
    let fraction = scaled - cell as f32;
    (cell, [1.0 - fraction, fraction])
}

/// One scalar interpolated multilinearly over a grid of `side^cells.len()` samples.
///
/// The same weights and the same index order as [`crate::ColourCube`]'s grid and
/// [`crate::BlendingSpace`]'s — first axis fastest — over one number per sample rather than
/// three: a luminosity is a scalar and a device colour is not, and writing the shared half as
/// a generic would have made both harder to read than the few lines they each are.
///
/// The axis count is `cells.len()` rather than a constant because §11.5.3 asks the same `Y`
/// of a three-component space and of a four-component one, and the only difference between
/// them is how many corners the sum runs over: eight, or sixteen.
fn multilinear(side: usize, samples: &[f32], cells: &[(usize, [f32; 2])]) -> f32 {
    let corners = 1usize << cells.len();
    let mut sum = 0.0f32;
    for corner in 0..corners {
        let mut weight = 1.0f32;
        for (axis, cell) in cells.iter().enumerate() {
            weight *= cell.1[(corner >> axis) & 1];
        }
        if weight == 0.0 {
            continue;
        }
        // The index is built from the slowest axis down, each step multiplying by the side —
        // exactly `crate::BlendingSpace::convert`'s loop, one axis count wider.
        let mut index = 0usize;
        for (axis, cell) in cells.iter().enumerate().rev() {
            index = index
                .saturating_mul(side)
                .saturating_add(cell.0)
                .saturating_add((corner >> axis) & 1);
        }
        sum += weight * samples.get(index).copied().unwrap_or(0.0);
    }
    sum
}

/// A transparency group evaluated for its opacity rather than its colour (§11.5).
#[derive(Debug, Clone, PartialEq)]
pub struct SoftMask {
    /// The mask group's elements, in painting order.
    ///
    /// Positioned by "the transformation matrix specified by the Matrix entry in the
    /// transparency group's form dictionary … with the current transformation matrix at the
    /// moment the soft mask is established in the graphics state with the gs operator"
    /// (§11.6.5.1) — resolved into each command's own transform, as everywhere else here.
    ///
    /// [`crate::ClipId`]s inside refer to the enclosing [`crate::DisplayList`], for the same
    /// reason a [`Command::Group`]'s do.
    pub commands: Vec<Command>,
    /// Which derivation produces the mask values.
    pub kind: SoftMaskKind,
    /// Everything between the computed alpha or luminosity and the mask value, or `None`
    /// where that is the identity — see [`Transfer`], which since the
    /// three-hundred-and-eighty-third session carries more than §11.6.5.1's `/TR`.
    pub transfer: Option<Transfer>,
    /// §11.5.3's `Y` of a group composited in a CIE-based space, where the channels hold that
    /// space's components rather than a device colour; `None` where the luminosity is
    /// [`Color::grey_level`] of what the channels hold. Meaningful only under
    /// [`SoftMaskKind::Luminosity`].
    ///
    /// Its [`Luminance::axes`] and [`Self::black`] agree by construction: a `Y` of four
    /// components needs the second raster and a `Y` of three has none.
    pub luminance: Option<Luminance>,
    /// §11.4.7's second raster, where the mask group's blending colour space has four
    /// components (ISO 32000-2 §11.3.4, §11.5.3).
    ///
    /// `None` for every other mask, which is every mask this type carried until the
    /// nine-hundred-and-seventh session. See [`BlackHalf`] for what the pair is and why a
    /// mask needs one where a wholly opaque *group* does not.
    pub black: Option<BlackHalf>,
}

/// The black half of a mask group composited in four components (ISO 32000-2 §11.5.3,
/// §11.3.4, §11.4.7).
///
/// §11.3.4 applies the compositing formula per component, and a rasteriser has three
/// channels, so a group composited in an `ICCBased` 'CMYK' blending space is one content
/// stream drawn twice: [`SoftMask::commands`] carries the additive complements of cyan,
/// magenta and yellow, and this carries the complement of the black component in every
/// channel. That is exactly [`crate::GroupBlending::FourComponents`]'s construction, one
/// scope over — and it is the *whole* of the difference between the two lists, which
/// `pdf-model` verifies before pairing them.
///
/// **A mask needs the pair where a fully opaque group does not**, which is worth stating
/// because the group's own construction skips the second run when nothing inside it
/// composites. A group's four components are converted to the device at the end and an
/// opaque Normal mark carries its colour through whatever space it was carried in; a mask's
/// four components are converted to **one number** by §11.5.3's `Y`, which is a function of
/// all four however opaque the marks are. So the second list is drawn whenever the space has
/// four components.
#[derive(Debug, Clone, PartialEq)]
pub struct BlackHalf {
    /// The same elements as [`SoftMask::commands`], drawn in the black component.
    pub commands: Vec<Command>,
    /// The backdrop this half composites onto: §11.6.5.1's `/BC`, its black component
    /// complemented, in every channel.
    ///
    /// A second field rather than a second use of [`SoftMaskKind::Luminosity`]'s, because the
    /// two halves composite onto two different numbers — the clause's one backdrop colour
    /// has four components and each raster carries the ones it is drawing.
    pub backdrop: Color,
}

impl SoftMask {
    /// Converts one rendered pixel's straight-alpha RGBA into a mask value.
    ///
    /// **For a mask carrying [`Self::black`] this is the value at a point *neither* raster
    /// marked**, and a backend drawing such a mask calls [`Self::paired_value`] instead — see
    /// there for why one alpha covers both halves.
    ///
    /// The pixels are the mask group drawn onto a *transparent* backdrop, which is what
    /// both §11.5.2 and §11.5.3 ask for: the first factors the backdrop out, and the second
    /// composites onto the chosen `/BC` here, in one place, rather than filling a buffer
    /// with it and letting a blend mode inside the group see it. §11.4.7's page group has
    /// the same shape and `crate::impose_on_medium` is its counterpart.
    #[must_use]
    pub fn value(&self, pixel: [u8; 4]) -> u8 {
        self.paired_value(pixel, [0, 0, 0, 0])
    }

    /// Converts one point of §11.4.7's *pair* of rasters into a mask value.
    ///
    /// `chromatic` is the pixel [`Self::commands`] drew and `black` the one
    /// [`BlackHalf::commands`] drew at the same point, both straight-alpha RGBA. Where this
    /// mask carries no black half the second argument is not read at all, which is what makes
    /// [`Self::value`] a call to this one rather than a second derivation beside it.
    ///
    /// **The alpha is the chromatic half's, for both.** The two lists are one content stream
    /// under one set of shapes and opacities, so a point either half covered the other
    /// covered identically; carrying one alpha rather than two is that fact stated once
    /// instead of trusted twice. It is also why [`Self::value`]'s `[0, 0, 0, 0]` is the black
    /// half *at a point neither half marked* and not a general stand-in: a caller that hands
    /// an opaque chromatic pixel to [`Self::value`] on a paired mask gets the black
    /// component read as fully inked.
    ///
    /// The two halves are drawn from one content stream under one set of shapes and
    /// opacities, so their alphas agree by construction; each is composited onto its own half
    /// of §11.6.5.1's `/BC` and the four components are recovered from the two results
    /// (§11.3.4 stores the additive complement, so a component is `1 − channel`). Only then
    /// is §11.5.3's `Y` taken, because the luminosity of a composite is not the composite of
    /// luminosities: a press profile is not affine in its inks.
    #[must_use]
    pub fn paired_value(&self, chromatic: [u8; 4], black: [u8; 4]) -> u8 {
        let pixel = chromatic;
        let derived = match self.kind {
            // §11.5.2: "The mask value at each point shall then be derived from the alpha of
            // the group." Outside the group's bounding box the alpha is 0, which is the
            // clause's "the result of applying the transfer function to the input value 0.0"
            // and needs no separate case.
            SoftMaskKind::Alpha => pixel[3],
            // §11.5.3, for a device colour space: "convert the colour to DeviceGray by
            // implementation-defined means and use the resulting gray value as the
            // luminosity, with no compensation for gamma or other colour calibration", for
            // which EXAMPLE 2 gives Y = 0.30 R + 0.59 G + 0.11 B — [`Color::grey_level`],
            // §10.4.2.2's own formula.
            //
            // That is the whole of the derivation *for a group composited in the device's
            // three components*. A group whose blending space is subtractive is painted in a
            // grey by `pdf_model` instead, and this arithmetic then reads that grey back
            // unchanged — the three coefficients sum to 1.0 — leaving what the grey *means*
            // to [`Transfer`], which is where the second half of §10.4.2.3 lives. It has
            // lived there since the three-hundred-and-eighty-third session, which is what
            // lets the clause's `min` wait for the compositing the way §11.5.3 states.
            SoftMaskKind::Luminosity { backdrop } => {
                let alpha = f32::from(pixel[3]) / 255.0;
                // Source-over onto an opaque backdrop, in straight alpha: the result is
                // opaque, so its colour is the ordinary interpolation between the two.
                let over = |channel: u8, backdrop: f32| {
                    (f32::from(channel) / 255.0).mul_add(alpha, backdrop * (1.0 - alpha))
                };
                let composited = Color::rgb(
                    over(pixel[0], backdrop.r),
                    over(pixel[1], backdrop.g),
                    over(pixel[2], backdrop.b),
                );
                // And §11.5.3's other branch, for a group whose channels hold a CIE-based
                // space's components: the `Y` is the sum of one curve per component
                // ([`Luminance`]), which is the clause's EXAMPLE 1 for `CalRGB`, or a grid
                // interpolated — over three components, or over the four this half and the
                // black one carry between them.
                let luminosity = match self.luminance.as_ref() {
                    None => composited.grey_level(),
                    Some(luminance) => match self.black.as_ref() {
                        None => luminance.of(&[composited.r, composited.g, composited.b]),
                        Some(half) => {
                            let level = (f32::from(black[0]) / 255.0)
                                .mul_add(alpha, half.backdrop.r * (1.0 - alpha));
                            // §11.3.4's complement, undone: the rasters carry `1 − component`
                            // so that the blend functions see additive values, and §11.5.3's
                            // `Y` is a function of the components themselves.
                            luminance.of(&[
                                1.0 - composited.r,
                                1.0 - composited.g,
                                1.0 - composited.b,
                                1.0 - level,
                            ])
                        }
                    },
                };
                #[expect(
                    clippy::cast_possible_truncation,
                    clippy::cast_sign_loss,
                    reason = "clamped to 0..=255 on the line above the cast"
                )]
                {
                    (luminosity * 255.0).round().clamp(0.0, 255.0) as u8
                }
            }
        };
        self.transfer
            .as_ref()
            .map_or(derived, |transfer| transfer.apply(derived))
    }

    /// The mask value everywhere the group's marks did not reach (§11.6.5.1).
    ///
    /// A mask group is drawn onto a *transparent* buffer, so a pixel its `/BBox` does not
    /// reach — or that it simply never marks — is `[0, 0, 0, 0]`, and [`Self::value`] of that
    /// pixel is one number for the whole raster. It is what the clause asks for outside the
    /// box: the transfer function applied to 0.0 for `/Alpha`, and the backdrop's luminosity
    /// for `/Luminosity`.
    ///
    /// **Naming it is a performance decision and it is exact rather than approximate**: a
    /// backend that recognises the transparent pixel computes this once instead of running the
    /// derivation per pixel, and the two answers are the same by construction because they are
    /// the same call. On the two slowest documents in a 65 944-document sample of the web,
    /// 98.5% and 99.96% of every soft-mask raster is that one pixel — 3.87 billion of them on
    /// a page holding 4.3 million — because a page with 912 distinct masks evaluates each over
    /// the whole target. ADR 0271 has the measurement; `doc/todo/40` has the change that would
    /// stop the buffer being target-sized in the first place, which this does not do.
    #[must_use]
    pub fn outside(&self) -> u8 {
        self.value([0, 0, 0, 0])
    }

    /// Converts a rendered raster of the mask group into one mask value per pixel.
    ///
    /// `pixels` is straight-alpha RGBA8 — [`crate::Raster`]'s documented format, which is
    /// what both backends hand back. A trailing partial pixel is ignored rather than
    /// reported: it cannot arise from a raster either backend produces, and refusing to
    /// build a mask is a blank page where dropping a byte is nothing.
    ///
    /// **§11.6.5.1's outside-the-bounding-box rule needs no case here, and that is worth
    /// saying rather than leaving to be noticed.** Both backends draw the mask group into a
    /// buffer the size of the *whole target*, so a pixel the group's `/BBox` does not reach is
    /// `[0, 0, 0, 0]` and [`Self::value`] gives it the transfer function applied to 0.0 for
    /// `/Alpha` and to the backdrop's luminosity for `/Luminosity` — which is what the clause
    /// asks for, arrived at by the same arithmetic as every other pixel rather than by a second
    /// derivation that could drift from the first. This module carried an `outside_value()`
    /// helper for that rule until the hundred-and-seventy-fifth session and **no backend ever
    /// called it**: it was a path nobody took, which `CLAUDE.md` forbids, and the tests that
    /// used it now state the same thing through `value([0, 0, 0, 0])`.
    #[must_use]
    pub fn values(&self, pixels: &[u8]) -> Vec<u8> {
        let outside = self.outside();
        pixels
            .chunks_exact(4)
            .map(|pixel| {
                if pixel == [0, 0, 0, 0] {
                    outside
                } else {
                    self.value([pixel[0], pixel[1], pixel[2], pixel[3]])
                }
            })
            .collect()
    }

    /// [`Self::values`] for a mask carrying [`Self::black`]: one value per point of the pair.
    ///
    /// The two rasters cover the same points in the same order — the halves are one content
    /// stream drawn twice into two buffers of one size — so a point missing from either is a
    /// buffer a backend built wrong rather than a case to interpolate over, and it is
    /// answered with [`Self::outside`] rather than with a value derived from half a pair.
    ///
    /// The transparent-pixel shortcut is [`Self::values`]'s and is exact for the same reason:
    /// where *both* halves are untouched the point is outside everything the group marked,
    /// and the constant is the same call.
    #[must_use]
    pub fn paired_values(&self, chromatic: &[u8], black: &[u8]) -> Vec<u8> {
        let outside = self.outside();
        chromatic
            .chunks_exact(4)
            .enumerate()
            .map(|(at, pixel)| {
                let Some(other) = black
                    .get(at.saturating_mul(4)..)
                    .and_then(<[u8]>::first_chunk::<4>)
                else {
                    return outside;
                };
                if pixel == [0, 0, 0, 0] && *other == [0, 0, 0, 0] {
                    return outside;
                }
                self.paired_value([pixel[0], pixel[1], pixel[2], pixel[3]], *other)
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::{BlackHalf, Luminance, SoftMask, SoftMaskKind, Transfer};
    use crate::paint::Color;

    fn mask(kind: SoftMaskKind, transfer: Option<Transfer>) -> SoftMask {
        SoftMask {
            commands: Vec::new(),
            kind,
            transfer,
            luminance: None,
            black: None,
        }
    }

    /// §11.5.3 EXAMPLE 1's `Y` for a `CalRGB` group, as three curves: a gamma of 2 on each
    /// component and the sRGB primaries' `Y` weights, so the clause's formula is
    /// `0.2126 A² + 0.7152 B² + 0.0722 C²` and the curves are that formula term by term.
    #[test]
    fn a_luminance_of_three_curves_sums_the_clauses_own_formula() {
        let curves: [[f32; 3]; 256] = std::array::from_fn(|index| {
            #[expect(clippy::cast_precision_loss, reason = "an index below 256")]
            let component = index as f32 / 255.0;
            let decoded = component * component;
            [0.2126 * decoded, 0.7152 * decoded, 0.0722 * decoded]
        });
        let luminance = Luminance::curves(std::sync::Arc::new(curves));
        let close = |colour: Color, want: f32, what: &str| {
            let got = luminance.of(&[colour.r, colour.g, colour.b]);
            assert!((got - want).abs() < 1e-3, "{what}: {got} against {want}");
        };
        close(Color::rgb(1.0, 1.0, 1.0), 1.0, "white is the weights' sum");
        close(Color::rgb(0.0, 1.0, 0.0), 0.7152, "green is its own weight");
        close(
            Color::rgb(0.5, 0.0, 0.0),
            0.2126 * 0.25,
            "half red is gamma-decoded first",
        );
        close(
            Color::rgb(0.5, 0.5, 0.5),
            0.25,
            "a grey of ½ under gamma 2 is a Y of ¼",
        );

        let mask = SoftMask {
            commands: Vec::new(),
            kind: SoftMaskKind::Luminosity {
                backdrop: Color::BLACK,
            },
            transfer: None,
            luminance: Some(luminance),
            black: None,
        };
        assert_eq!(
            mask.value([128, 128, 128, 255]),
            64,
            "a composited grey of ½ masks at the clause's ¼, not at 0.30 + 0.59 + 0.11 of ½"
        );
        assert_eq!(
            mask.value([255, 255, 255, 128]),
            // Half of white over black composites the components to ½, whose Y under gamma 2
            // is ¼ — the compositing happens on the components and the curves come after.
            64,
            "the curves are applied after the composite, not before it"
        );
    }

    /// §11.5.3's `Y` over a sampled grid, for a space whose luminosity is not a sum of one
    /// function of each component (ADR 0851).
    ///
    /// The grid below holds a *linear* `Y` — sRGB's D50-adapted weights — at two samples an
    /// axis, and a linear map is reproduced exactly by trilinear interpolation of its corners,
    /// so the expected values are the formula itself and the interpolation is checked rather
    /// than approximated. The last case is the one the shape exists for: a value the summed
    /// curves cannot state, because it is not the sum of any three functions of one component.
    #[test]
    fn a_luminance_over_a_grid_interpolates_the_clauses_y() {
        let weights = [0.2225f32, 0.7169, 0.0606];
        let samples: Vec<f32> = (0..8usize)
            .map(|corner| {
                (0..3usize)
                    .map(|axis| {
                        #[expect(clippy::cast_precision_loss, reason = "a bit")]
                        let at = ((corner >> axis) & 1) as f32;
                        at * weights.get(axis).copied().unwrap_or(0.0)
                    })
                    .sum()
            })
            .collect();
        let luminance = Luminance::grid(2, std::sync::Arc::from(samples))
            .expect("eight samples of a side of 2");
        let close = |colour: Color, want: f32, what: &str| {
            let got = luminance.of(&[colour.r, colour.g, colour.b]);
            assert!((got - want).abs() < 1e-3, "{what}: {got} against {want}");
        };
        close(Color::rgb(0.0, 0.0, 0.0), 0.0, "black is no light");
        close(Color::rgb(1.0, 1.0, 1.0), 1.0, "white is the weights' sum");
        close(
            Color::rgb(0.0, 1.0, 0.0),
            0.7169,
            "a pure green is its own weight, not the device branch's 0.59",
        );
        close(
            Color::rgb(0.5, 0.5, 0.5),
            0.5,
            "and a point between the samples is the interpolation",
        );

        assert!(
            Luminance::grid(1, std::sync::Arc::from(vec![0.0f32])).is_none(),
            "a side of one has no cell to interpolate over"
        );
        assert!(
            Luminance::grid(2, std::sync::Arc::from(vec![0.0f32; 7])).is_none(),
            "and a grid that is not side³ samples is not a grid"
        );

        let mask = SoftMask {
            commands: Vec::new(),
            kind: SoftMaskKind::Luminosity {
                backdrop: Color::BLACK,
            },
            transfer: None,
            luminance: Some(luminance),
            black: None,
        };
        assert_eq!(
            mask.value([0, 255, 0, 255]),
            183,
            "a composited green masks at the grid's Y, not at 0.59 of 255"
        );
    }

    /// §11.5.3's `Y` over a grid of **four** components, read off §11.4.7's pair of rasters
    /// (ISO 32000-2 §11.5.3, §11.3.4, §11.4.7; ADR 0857).
    ///
    /// The grid holds a multiplicative absorption model — one factor per ink, which is the
    /// shape a press's own `Y` has and is multilinear, so interpolating the sixteen corners
    /// reproduces it exactly and every expected value below is the formula itself rather than
    /// a tolerance.
    ///
    /// What the last two cases are *for* is the claim the whole construction rests on: the
    /// luminosity of a composite is not the composite of luminosities. Half a cyan-and-magenta
    /// mark over paper composites the inks to ½ each and the model gives `0.95 × 0.70`; taking
    /// the `Y` of each and averaging gives `(0.36 + 1.0) ÷ 2`, which is a different number.
    /// Compositing first is what the clause asks for — §11.5.3 takes the luminosity of "the
    /// resulting colour" — and it is only possible because both halves are carried.
    #[test]
    fn a_luminance_over_four_axes_reads_the_pairs_own_components() {
        let absorb = [0.10f32, 0.60, 0.20, 0.80];
        let model = |inks: [f32; 4]| {
            let mut y = 1.0f32;
            for (ink, factor) in inks.iter().zip(absorb) {
                y *= factor.mul_add(-*ink, 1.0);
            }
            y
        };
        let samples: Vec<f32> = (0..16usize)
            .map(|corner| {
                #[expect(clippy::cast_precision_loss, reason = "a bit")]
                let at = |axis: usize| ((corner >> axis) & 1) as f32;
                model([at(0), at(1), at(2), at(3)])
            })
            .collect();
        let luminance = Luminance::ink_grid(2, std::sync::Arc::from(samples))
            .expect("sixteen samples of a side of 2");
        assert_eq!(luminance.axes(), 4);
        assert!(
            luminance.as_grid().is_none() && luminance.as_ink_grid().is_some(),
            "a four-axis grid is not a three-axis one, and the protocol tells them apart"
        );
        assert!(
            Luminance::ink_grid(2, std::sync::Arc::from(vec![0.0f32; 8])).is_none(),
            "eight samples are a cube, not a four-dimensional grid"
        );

        let close = |inks: [f32; 4], what: &str| {
            let got = luminance.of(&inks);
            let want = model(inks);
            assert!((got - want).abs() < 1e-4, "{what}: {got} against {want}");
        };
        close([0.0; 4], "no ink is the paper's whole luminosity");
        close([0.0, 1.0, 0.0, 0.0], "one ink is its own factor");
        close(
            [0.0, 0.0, 0.0, 1.0],
            "and the black component is read at all",
        );
        close(
            [0.5, 0.5, 0.0, 0.0],
            "a point between the samples is the interpolation",
        );

        pairs_of(luminance, &model);
    }

    /// The second half of the test above: the same `Y`, read off a *mask* rather than off the
    /// grid, which is where §11.4.7's pair meets §11.5.3's derivation.
    fn pairs_of(luminance: Luminance, model: &dyn Fn([f32; 4]) -> f32) {
        // `/BC` of no ink at all, so both backdrops are white.
        let mask = SoftMask {
            commands: Vec::new(),
            kind: SoftMaskKind::Luminosity {
                backdrop: Color::WHITE,
            },
            transfer: None,
            luminance: Some(luminance),
            black: Some(BlackHalf {
                commands: Vec::new(),
                backdrop: Color::WHITE,
            }),
        };
        assert_eq!(
            mask.outside(),
            255,
            "outside the group nothing is inked, so the paper masks nothing away"
        );
        // A black ink of 1.0 is `1 − 1` in the black raster and no chromatic ink at all.
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "a luminosity in 0..=1 scaled to a byte"
        )]
        let want = (model([0.0, 0.0, 0.0, 1.0]) * 255.0).round() as u8;
        assert_eq!(
            mask.paired_value([255, 255, 255, 255], [0, 0, 0, 255]),
            want,
            "the black component comes from the second raster and nowhere else"
        );
        assert_ne!(
            mask.value([255, 255, 255, 255]),
            mask.paired_value([255, 255, 255, 255], [255, 255, 255, 255]),
            "and `value` on a paired mask reads a black raster that painted nothing at a \
             point the chromatic one did, which is why a backend that meets `black` draws both"
        );

        // Half a cyan-and-magenta mark over paper, in both rasters: the inks composite, and
        // only then is the `Y` taken.
        let composited = mask.paired_value([128, 128, 255, 255], [255, 255, 255, 255]);
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "a luminosity in 0..=1 scaled to a byte"
        )]
        let want =
            (model([1.0 - 128.0 / 255.0, 1.0 - 128.0 / 255.0, 0.0, 0.0]) * 255.0).round() as u8;
        assert_eq!(
            composited, want,
            "the components composite, then the Y is taken"
        );
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "a luminosity in 0..=1 scaled to a byte"
        )]
        let averaged =
            (f32::midpoint(model([1.0, 1.0, 0.0, 0.0]), model([0.0; 4])) * 255.0).round() as u8;
        assert_ne!(
            composited, averaged,
            "and that is a different number from the average of the two luminosities"
        );

        let short = mask.paired_values(&[128, 128, 255, 255], &[]);
        assert_eq!(
            short,
            vec![mask.outside()],
            "a black raster that does not cover the point answers the outside value rather \
             than half a pair"
        );
    }

    /// §11.5.2: the alpha is the mask value, whatever colour carried it.
    #[test]
    fn an_alpha_mask_reads_the_alpha_and_ignores_the_colour() {
        let mask = mask(SoftMaskKind::Alpha, None);
        assert_eq!(mask.value([255, 0, 0, 64]), 64);
        assert_eq!(mask.value([0, 255, 0, 64]), 64);
        assert_eq!(
            mask.value([0, 0, 0, 0]),
            0,
            "§11.6.5.1, outside the group's box"
        );
    }

    /// §11.5.3 EXAMPLE 2's coefficients, which are not any library's luminance.
    ///
    /// Pure green is the case that separates them: 0.59 here, 0.7152 under Rec. 709 and
    /// 0.7154 under the SVG formula Vello's luminance mask uses. A mask built from green
    /// artwork is 21% more transparent under either of those, which is why this test states
    /// a colour rather than a grey.
    #[test]
    fn a_luminosity_mask_uses_the_clauses_own_coefficients() {
        let mask = mask(
            SoftMaskKind::Luminosity {
                backdrop: Color::BLACK,
            },
            None,
        );
        assert_eq!(mask.value([0, 255, 0, 255]), 150, "0.59 x 255 = 150.45");
        assert_eq!(mask.value([255, 0, 0, 255]), 77, "0.30 x 255 = 76.5");
        assert_eq!(mask.value([0, 0, 255, 255]), 28, "0.11 x 255 = 28.05");
        assert_eq!(mask.value([255, 255, 255, 255]), 255);
    }

    /// §11.6.5.1: outside the group's bounding box a luminosity mask takes the backdrop's.
    #[test]
    fn a_luminosity_mask_outside_the_group_is_the_backdrops_luminosity() {
        let white = mask(
            SoftMaskKind::Luminosity {
                backdrop: Color::WHITE,
            },
            None,
        );
        assert_eq!(white.value([0, 0, 0, 0]), 255);
        assert_eq!(
            white.value([0, 0, 0, 128]),
            127,
            "black at 128/255 coverage leaves 127/255 of the backdrop's luminosity"
        );

        let black = mask(
            SoftMaskKind::Luminosity {
                backdrop: Color::BLACK,
            },
            None,
        );
        assert_eq!(
            black.value([0, 0, 0, 0]),
            0,
            "§11.5.3 NOTE 2's usual choice"
        );
    }

    /// The transfer function is applied after the derivation, to both subtypes.
    #[test]
    fn the_transfer_function_maps_the_derived_value() {
        let mut table = [0_u8; 256];
        for (index, entry) in table.iter_mut().enumerate() {
            #[expect(
                clippy::cast_possible_truncation,
                reason = "255 - index is in 0..=255 for an index of a 256-entry table"
            )]
            {
                *entry = (255 - index) as u8;
            }
        }
        let inverted = Transfer::from_samples(table);
        assert!(!inverted.is_identity());

        let mask = mask(SoftMaskKind::Alpha, Some(inverted));
        assert_eq!(mask.value([0, 0, 0, 0]), 255, "an inverting /TR unmasks");
        assert_eq!(mask.value([0, 0, 0, 255]), 0);
    }

    /// The identity is recognised as such, since a backend may express only that.
    #[test]
    fn an_identity_table_is_identified() {
        let mut table = [0_u8; 256];
        for (index, entry) in table.iter_mut().enumerate() {
            #[expect(
                clippy::cast_possible_truncation,
                reason = "index of a 256-entry table is in 0..=255"
            )]
            {
                *entry = index as u8;
            }
        }
        assert!(Transfer::from_samples(table).is_identity());
    }

    /// A whole raster is converted pixel by pixel, in order.
    #[test]
    fn a_raster_converts_pixelwise() {
        let mask = mask(SoftMaskKind::Alpha, None);
        let pixels = [0, 0, 0, 10, 0, 0, 0, 200];
        assert_eq!(mask.values(&pixels), vec![10, 200]);
    }

    /// [`SoftMask::outside`] is [`SoftMask::value`] of the transparent pixel and nothing else.
    ///
    /// The whole warrant for the shortcut in [`SoftMask::values`] and in `render-cpu`'s
    /// `build_soft_mask` is that the two are the same call, so it is asserted over every kind
    /// of mask this type has rather than left to be read off the one-line body. ADR 0271.
    #[test]
    fn the_outside_value_is_the_transparent_pixels_own() {
        let mut inverting = [0_u8; 256];
        for (index, entry) in inverting.iter_mut().enumerate() {
            #[expect(
                clippy::cast_possible_truncation,
                reason = "255 - index is in 0..=255 for an index of a 256-entry table"
            )]
            {
                *entry = (255 - index) as u8;
            }
        }
        for kind in [
            SoftMaskKind::Alpha,
            SoftMaskKind::Luminosity {
                backdrop: Color::rgb(0.0, 0.0, 0.0),
            },
            // §11.6.5.1's `/BC` need not be black, and a non-zero backdrop is the case where
            // the constant is *not* the zero a clip band would stand for.
            SoftMaskKind::Luminosity {
                backdrop: Color::rgb(1.0, 1.0, 1.0),
            },
        ] {
            for transfer in [None, Some(Transfer::from_samples(inverting))] {
                let mask = mask(kind, transfer);
                assert_eq!(mask.outside(), mask.value([0, 0, 0, 0]));
                assert_eq!(
                    mask.values(&[0, 0, 0, 0, 0, 0, 0, 0]),
                    vec![mask.value([0, 0, 0, 0]); 2],
                    "the shortcut may not answer differently from the derivation"
                );
            }
        }
    }

    /// A `/Luminosity` mask on a white backdrop leaves the unmarked page *unmasked*.
    ///
    /// The case that makes [`SoftMask::outside`] worth naming rather than assuming zero:
    /// §11.5.3's derivation of an all-white backdrop is 255, so "outside the group's marks"
    /// is not "masked out" and a backend may not treat the two as the same thing.
    #[test]
    fn a_white_backdrop_makes_the_outside_opaque() {
        let mask = mask(
            SoftMaskKind::Luminosity {
                backdrop: Color::rgb(1.0, 1.0, 1.0),
            },
            None,
        );
        assert_eq!(mask.outside(), 255);
    }
}

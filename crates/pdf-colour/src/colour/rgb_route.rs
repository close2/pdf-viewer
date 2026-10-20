//! The way into and out of a `CalRGB` or `ICCBased` three-component blending space (§11.3.4, §11.6.6, §8.6.5).

use std::sync::Arc;

use pdf_render::Color;

use super::ColourSpace;
use super::cie::{
    D50, adapt, cal_rgb_xyz, degamma, gamma, linear_srgb_to_xyz_d50, srgb_to_xyz_d50,
    xyz_d50_to_linear_srgb,
};
use super::device_cmyk::channel;
use super::ink_search::solve_three;
use crate::icc::Rendering;

/// Which three-component CIE-based space an [`RgbRoute`] is the route into and out of.
///
/// What [`super::compositing::Compositing`]'s key and §11.6.6's "not equivalent to the group colour space" both
/// need, as [`super::grey_route::GreyIdentity`] is for one component: two `CalRGB` dictionaries stating one
/// gamma, one white point and one matrix are one space, and a profile is the profile
/// [`crate::icc::Profile::identity`] says it is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RgbIdentity {
    /// §8.6.5.3's space, by the bits of its `Gamma`, `WhitePoint` and `Matrix`.
    CalRgb {
        /// `Gamma`, as `f32::to_bits` apiece.
        gamma: [u32; 3],
        /// `WhitePoint`, as `f32::to_bits` apiece.
        white: [u32; 3],
        /// `Matrix`, as `f32::to_bits` apiece.
        matrix: [u32; 9],
    },
    /// A three-component `ICCBased` space, by its profile.
    Profile(u128),
}

/// How many samples [`RgbRoute`] takes of each input curve: one per value an eight-bit
/// channel can hold, so that a component at full alpha lands on a sample exactly.
const RGB_CURVE_SAMPLES: usize = 256;

/// How many samples [`RgbRoute`] takes of the device's transfer function.
///
/// More than the input curves', because this one is steep at black — the sRGB encoding has
/// a slope of 12.92 there — and it is sampled over linear light, where the interesting part
/// is the first few percent. At 1024 samples the linear interpolation is within a tenth of a
/// level of 255 of the function everywhere; at 256 it is within half a level, which the
/// raster would round away, but the finer table costs four kilobytes and settles it.
pub(super) const RGB_OUTPUT_SAMPLES: usize = 1024;

/// How many samples an axis takes of a profile whose conversion is a lookup table.
///
/// A table has no linear stage to carry as corners, so the whole conversion to linear light
/// is sampled, at the side a `B2A` table itself typically has. Thirty-three cubed is 36 000
/// samples, and what it departs from evaluating the profile is a property of that profile's
/// own smoothness between its grid points; a matrix profile and a `CalRGB` need no such
/// grid at all, which is [`RgbRoute::of`]'s first branch.
const RGB_TABLE_SIDE: usize = 33;

/// The way into a `CalRGB` or `ICCBased` 'RGB ' blending colour space, and the way out
/// (ISO 32000-2 §11.3.4, §11.6.6, §11.7.2, §8.6.5.3, §8.6.5.5).
///
/// [`super::grey_route::GreyRoute`]'s construction for three components rather than one. §11.6.6 has every
/// painting operator "convert source colours in a colour space (that are not equivalent to
/// the group colour space) to the group colour space before compositing objects into the
/// group", and §11.4.7 converts the composited result out to the device at the end. The
/// standard states the conversion *out* in full — §8.6.5.3's gamma decoding and `Matrix`, a
/// profile's `A2B` — and the conversion *in* in three parts:
///
/// - **A device colour of three components keeps its numbers**, by §11.7.2: "if the colour
///   space of any graphics object is a device colour space, and the current group or an
///   ancestor of the current group is defined with a CIE-based colour space with the same
///   number of colourants, then, for compositing purposes only, the colour space of the
///   graphics object shall be the CIE-based space of the nearest such ancestor". So a
///   `DeviceRGB` mark is composited as the components it states, unconverted.
/// - **A CIE-based colour goes in from its XYZ** (§10.3.1's conversion between CIE-based
///   spaces): through a profile's `B2A` or a matrix profile's inverted stages
///   ([`crate::icc::Profile::to_device`]), or through §8.6.5.3's stages run backwards for a
///   `CalRGB` — adapted onto the space's white point, the `Matrix` inverted, each component
///   clamped to the range the clause gives it and raised to the reciprocal of its gamma.
/// - **A device colour of another component count** — a grey, a `DeviceCMYK` — has, in the
///   clause's words, "no generally defined method for converting them", and "shall be
///   converted or mapped to a CIE-based colour space in an implementationdependent fashion".
///   This tree's way is the one ADR 0796 chose for a press: the colour's sRGB, which §10.3.2
///   has this processor establish for its device spaces, taken to XYZ and in from there.
///
/// The conversion out is carried to the backends as a [`pdf_render::ColourCube`] — the
/// space's own curves, its linear stage as eight corners, and the device's transfer function
/// — which for `CalRGB` and a matrix profile is the conversion exactly rather than a sampling
/// of it; a table profile is sampled over a grid.
///
/// **A profile without a way in is not a route.** §11.3.4 requires a blending-space profile
/// to be bi-directional; a table profile without a `B2A` this crate reads has no conversion
/// in, and [`RgbRoute::of`] answers `None`, which keeps the report the space had.
#[derive(Debug)]
pub struct RgbRoute {
    /// The conversion out, as the backends apply it.
    cube: pdf_render::ColourCube,
    /// Which space this is the route into.
    pub(super) identity: RgbIdentity,
    /// What the conversion in runs through.
    inward: Inward,
}

/// The stages an [`RgbRoute`] runs backwards on the way in.
#[derive(Debug)]
enum Inward {
    /// §8.6.5.3's space: adapt onto `white`, invert `columns`, undo `gamma`.
    CalRgb {
        white: [f32; 3],
        gamma: [f32; 3],
        columns: [[f32; 3]; 3],
    },
    /// A profile, whose `to_device` is its own inverse.
    Profile(Arc<crate::icc::Profile>),
}

/// The first two stages of an [`RgbRoute`]'s cube, and the way back in, for a space.
type Stages = (Vec<[f32; 3]>, usize, Vec<[f32; 3]>, Inward);

/// The eight corners of a linear map, in [`pdf_render::ColourCube`]'s index order.
pub(super) fn corners(linear_of: impl Fn([f32; 3]) -> [f32; 3]) -> Vec<[f32; 3]> {
    (0..8usize)
        .map(|corner| {
            #[expect(clippy::cast_precision_loss, reason = "a bit")]
            let at = |bit: usize| ((corner >> bit) & 1) as f32;
            linear_of([at(0), at(1), at(2)])
        })
        .collect()
}

/// One curve per component, sampled at [`RGB_CURVE_SAMPLES`] points.
fn curves(curve: impl Fn(usize, f32) -> f32) -> Vec<[f32; 3]> {
    (0..RGB_CURVE_SAMPLES)
        .map(|index| {
            #[expect(clippy::cast_precision_loss, reason = "an index below 256")]
            let component = index as f32 / (RGB_CURVE_SAMPLES - 1) as f32;
            [
                curve(0, component),
                curve(1, component),
                curve(2, component),
            ]
        })
        .collect()
}

/// §8.6.5.3's stages, with the adaptation `cie_to_srgb` applies between the matrix and the
/// device: the curves are the gamma decoding, and everything after it is linear.
fn cal_rgb_stages(white: [f32; 3], gamma: [f32; 3], matrix: &[f32; 9]) -> Stages {
    let mut columns = [[0.0f32; 3]; 3];
    for (column, values) in columns.iter_mut().enumerate() {
        for (axis, value) in values.iter_mut().enumerate() {
            *value = matrix
                .get(column.saturating_mul(3).saturating_add(axis))
                .copied()
                .unwrap_or(0.0);
        }
    }
    let input = curves(|axis, component| component.powf(gamma.get(axis).copied().unwrap_or(1.0)));
    let grid =
        corners(|decoded| xyz_d50_to_linear_srgb(adapt(cal_rgb_xyz(decoded, matrix), white, D50)));
    (
        input,
        2,
        grid,
        Inward::CalRgb {
            white,
            gamma,
            columns,
        },
    )
}

/// A profile's stages: a matrix profile's curves and corners, or a table profile's grid,
/// or `None` for a profile with no way in.
fn profile_stages(profile: &crate::icc::Profile) -> Option<Stages> {
    let inward = Inward::Profile(Arc::new(profile.clone()));
    if let Some(stages) = profile.matrix_stages() {
        let input = curves(|axis, component| stages.linear(axis, component));
        let grid = corners(|linear| xyz_d50_to_linear_srgb(stages.xyz(linear)));
        return Some((input, 2, grid, inward));
    }
    if !profile.is_bidirectional() {
        return None;
    }
    let side = RGB_TABLE_SIDE;
    let mut grid = Vec::with_capacity(side.pow(3));
    for third in 0..side {
        for second in 0..side {
            for first in 0..side {
                #[expect(clippy::cast_precision_loss, reason = "a grid index below the side")]
                let at = |index: usize| index as f32 / (side - 1) as f32;
                // With the compensation on, as every colour of the space reaches the screen
                // and as `to_device` is asked on the way in.
                let xyz = profile.to_xyz_with(
                    &[at(first), at(second), at(third)],
                    Rendering::compensating(),
                );
                grid.push(xyz_d50_to_linear_srgb(xyz));
            }
        }
    }
    Some((vec![[0.0f32; 3], [1.0f32; 3]], side, grid, inward))
}

/// How many samples [`inward_cube`] takes of the device's decoding, on each of the three
/// axes of the cube's input.
///
/// A thousand and twenty-four rather than the conversion out's 256, because this curve is
/// read *before* a stage whose slope is unbounded: the space's own encoding is an inverse
/// gamma, so an error of `e` in the linear value it is handed can come back as `e` raised to
/// a power below one. At 256 samples that worst case is a level of 255; at 1024 it is a
/// third of one, and the table costs twelve kilobytes.
pub(super) const INWARD_INPUT_SAMPLES: usize = 1024;

/// How many samples [`inward_cube`] takes of the space's own encoding, which is the cube's
/// output curve.
///
/// This is the one stage of the conversion *in* that cannot be resolved by a finer grid: a
/// `CalRGB` with `/Gamma 2.2` encodes a linear value as its 1/2.2 power, whose slope at zero
/// is infinite, so a uniformly sampled table's worst cell is its first one and the error
/// there falls only as the 1/2.2 power of the sample count. Measured by
/// `a_cube_into_a_gamma_space_is_within_a_level` over 200 000 colours, in levels of 255:
/// 1.20 at 8192 samples, **0.88 at 16 384**, and 0.88 at 65 536, where this curve has stopped
/// being the largest term. The table costs sixty-four kilobytes, once per group rather than
/// once per pixel, and it is a sixth of the 431 kilobytes the grid it replaces held.
const INWARD_OUTPUT_SAMPLES: usize = 16_384;

/// How many samples an axis of [`inward_cube`]'s grid takes where the space's three
/// components are not encoded alike.
///
/// Where they are alike the grid holds nothing but the linear stage and two samples an axis
/// reproduce it exactly. Where they are not, one of the three has to be carried into the grid
/// as its residue against the reference curve — see [`inward_cube`] — and the residue is a
/// power of at least one, smooth and with a bounded slope, so a grid of the side this crate
/// already samples a table profile at resolves it.
const INWARD_CURVED_SIDE: usize = 33;

/// The conversion *into* a blending colour space from the device's own three components, as
/// the three stages [`pdf_render::ColourCube`] carries (ISO 32000-2 §11.6.6, §10.3.1).
///
/// `decode` is the device's own half: the function that takes one of this processor's three
/// components to the quantity `light` is stated in — sRGB's decoding where that quantity is
/// linear light, the identity where the clause states its weights on the components
/// themselves, as §10.4.2.2 does. `light` is the linear stage: it takes those three
/// quantities and answers the space's own components before their encoding, unclamped, so
/// that interpolating it interpolates the map rather than a sampling of it. `encode` is that
/// encoding, one function per component, taking a linear value to the component the space
/// states — §8.6.5.3's gamma raised to its reciprocal, or a profile's tone curve inverted.
///
/// The three stages then land where the cube's own documentation puts them: the device's
/// decoding on the **input** curves, the linear map in the **grid**, and the encoding on the
/// **output** curve. That is the whole of this function's reason. A cube sampled with
/// identity curves puts every stage in the grid, and a grid cannot resolve a stage whose
/// slope is unbounded: ADR 1254 measured 4.66 levels of 255 for a `/Gamma 2.2` space at
/// thirty-three samples an axis, against 3.40 at sixty-five, which is a sampling that does
/// not converge.
///
/// **The output curve is one curve and the space has three components**, which is the one
/// place this needs an argument rather than a decomposition. Where the three encodings agree
/// — which is every space stating one gamma, and every profile whose three tone curves are
/// one curve — the output curve is that encoding and the grid holds the linear map alone.
/// Where they differ, the output curve is their **pointwise maximum** and the grid holds each
/// component's residue against it: the maximum of the three encodings is the one whose
/// inverse is flattest, so every residue is a function of slope at most one and the grid,
/// which is what carries them, is asked for nothing steep. ADR 1267.
pub(super) fn inward_cube(
    decode: &dyn Fn(f32) -> f32,
    light: &dyn Fn([f32; 3]) -> [f32; 3],
    encode: &dyn Fn(usize, f32) -> f32,
) -> Option<pdf_render::ColourCube> {
    #[expect(
        clippy::cast_precision_loss,
        reason = "an index below the sample count"
    )]
    let at = |index: usize, count: usize| index as f32 / count.saturating_sub(1) as f32;
    let output: Vec<f32> = (0..INWARD_OUTPUT_SAMPLES)
        .map(|index| {
            let linear = at(index, INWARD_OUTPUT_SAMPLES);
            let mut highest = encode(0, linear);
            for axis in 1..3 {
                highest = highest.max(encode(axis, linear));
            }
            highest
        })
        .collect();
    let alike = (0..INWARD_OUTPUT_SAMPLES).all(|index| {
        let linear = at(index, INWARD_OUTPUT_SAMPLES);
        let reference = output.get(index).copied().unwrap_or(0.0);
        (0..3).all(|axis| (encode(axis, linear) - reference).abs() <= f32::EPSILON)
    });
    let input: Vec<[f32; 3]> = (0..INWARD_INPUT_SAMPLES)
        .map(|index| [decode(at(index, INWARD_INPUT_SAMPLES)); 3])
        .collect();
    let (side, grid) = if alike {
        (2, corners(light))
    } else {
        let side = INWARD_CURVED_SIDE;
        let mut grid = Vec::with_capacity(side.pow(3));
        for third in 0..side {
            for second in 0..side {
                for first in 0..side {
                    let components = light([at(first, side), at(second, side), at(third, side)]);
                    grid.push(std::array::from_fn(|axis| {
                        let stated = encode(axis, channel(components[axis]));
                        inverted(&output, stated)
                    }));
                }
            }
        }
        (side, grid)
    };
    pdf_render::ColourCube::new(Arc::from(input), side, Arc::from(grid), Arc::from(output))
}

/// Where `value` sits in a monotone table of evenly spaced samples, as a position on
/// `0.0..=1.0`.
///
/// The inverse of the linear interpolation [`pdf_render::ColourCube`] applies to its output
/// curve, so that a grid sample written here comes back out of that curve as the value it was
/// derived from. Samples that repeat — a curve flat over a stretch — answer the first
/// position that produces the value, which is the only one an inverse can promise.
fn inverted(samples: &[f32], value: f32) -> f32 {
    let last = samples.len().saturating_sub(1);
    let above = samples.partition_point(|sample| *sample <= value);
    if above == 0 {
        return 0.0;
    }
    if above > last {
        return 1.0;
    }
    let below = above.saturating_sub(1);
    let (Some(&low), Some(&high)) = (samples.get(below), samples.get(above)) else {
        return 1.0;
    };
    // `high > value >= low` by the partition, so the span is positive.
    #[expect(
        clippy::cast_precision_loss,
        reason = "an index below the sample count"
    )]
    let position = (below as f32 + (value - low) / (high - low)) / last as f32;
    channel(position)
}

impl RgbRoute {
    /// The route into and out of `space`, or `None` where `space` is not a three-component
    /// space §11.3.4 lists, or is a profile with no way in.
    #[must_use]
    pub fn of(space: &ColourSpace) -> Option<Self> {
        let identity = space.rgb_identity()?;
        let (input, side, grid, inward) = match space {
            ColourSpace::CalRgb {
                white,
                black: _,
                gamma,
                matrix,
            } => cal_rgb_stages(*white, *gamma, matrix),
            ColourSpace::Icc { profile } => profile_stages(profile)?,
            _ => return None,
        };
        let output: Vec<f32> = (0..RGB_OUTPUT_SAMPLES)
            .map(|index| {
                #[expect(clippy::cast_precision_loss, reason = "an index below 1024")]
                let linear = index as f32 / (RGB_OUTPUT_SAMPLES - 1) as f32;
                gamma(linear)
            })
            .collect();
        let cube = pdf_render::ColourCube::new(
            Arc::from(input),
            side,
            Arc::from(grid),
            Arc::from(output),
        )?;
        Some(Self {
            cube,
            identity,
            inward,
        })
    }

    /// The conversion out of the space, for the display list to carry.
    #[must_use]
    pub fn cube(&self) -> &pdf_render::ColourCube {
        &self.cube
    }

    /// The conversion *into* the space from this processor's own three components, as the
    /// cube a backend interpolates (ISO 32000-2 §11.6.6, §10.3.1, §8.6.5.3, §8.6.5.5).
    ///
    /// [`RgbRoute::components_of_srgb`] is what this samples — §11.6.6's painting of a
    /// group's result into a parent compositing in this space — and it is carried as a cube
    /// for the same reason the conversion out is: the backend applies one lookup per pixel
    /// and never sees a colour space. What it is *not* is a grid over the whole conversion,
    /// which is [`inward_cube`]'s subject and ADR 1267's.
    ///
    /// `None` for a profile whose conversion in is a lookup table, which has no linear stage
    /// to separate, and for one whose matrix will not invert; the caller samples the whole
    /// conversion for those, as [`profile_stages`] does for the conversion out.
    #[must_use]
    pub fn into_cube(&self, rendering: Rendering) -> Option<pdf_render::ColourCube> {
        match &self.inward {
            Inward::CalRgb {
                white,
                gamma,
                columns,
            } => {
                // §8.6.5.3 run backwards, split at the one stage that is not linear: the
                // adaptation and the inverted `Matrix` are the grid's, and the reciprocal
                // gamma — with the clause's clamp of a component "falling outside that
                // range" — is the output curve's.
                let light = |linear: [f32; 3]| {
                    solve_three(columns, adapt(linear_srgb_to_xyz_d50(linear), D50, *white))
                        .unwrap_or([0.0; 3])
                };
                let encode = |axis: usize, value: f32| {
                    let clamped = channel(value);
                    match gamma.get(axis).copied() {
                        Some(exponent) if exponent > 0.0 => clamped.powf(1.0 / exponent),
                        _ => clamped,
                    }
                };
                inward_cube(&degamma, &light, &encode)
            }
            Inward::Profile(profile) => {
                // A matrix profile's PCS-to-device direction is the inverse of the matrix
                // followed by the inverse of each tone curve, which is the same split; a
                // table profile has neither stage apart and answers `None`.
                profile.matrix_light(linear_srgb_to_xyz_d50([1.0; 3]), rendering)?;
                let light = |linear: [f32; 3]| {
                    profile
                        .matrix_light(linear_srgb_to_xyz_d50(linear), rendering)
                        .unwrap_or([0.0; 3])
                };
                let encode =
                    |axis: usize, value: f32| channel(profile.matrix_component(axis, value));
                inward_cube(&degamma, &light, &encode)
            }
        }
    }

    /// Which space this is the route into.
    #[must_use]
    pub fn identity(&self) -> RgbIdentity {
        self.identity
    }

    /// The components a colour becomes inside the space: §11.6.6's conversion in, decided
    /// as the type's documentation says.
    #[must_use]
    pub fn components_of(
        &self,
        space: &ColourSpace,
        values: &[f32],
        rendering: Rendering,
    ) -> [f32; 3] {
        space.rgb_components_at(values, 0, rendering, self)
    }

    /// The components a colour this processor holds as device RGB becomes inside the space,
    /// from that colour's XYZ.
    ///
    /// §11.6.6's conversion of a *group's* result into a parent compositing in this space: the
    /// group's raster holds its result as sRGB — the device §10.3.2 has this processor assume
    /// — and that result is not a `DeviceRGB` graphics object, so §11.7.2's rule that such an
    /// object "shall be the CIE-based space of the nearest such ancestor" for compositing
    /// purposes does not reach it. It goes in as [`RgbRoute::components_of`] sends every other
    /// colour: through its XYZ (§10.3.1), which for sRGB is [`srgb_to_xyz_d50`].
    #[must_use]
    pub fn components_of_srgb(&self, colour: Color, rendering: Rendering) -> [f32; 3] {
        self.components_with_xyz(srgb_to_xyz_d50(colour), rendering)
    }

    /// The components whose colour has the D50 XYZ `xyz`.
    pub(super) fn components_with_xyz(&self, xyz: [f32; 3], rendering: Rendering) -> [f32; 3] {
        match &self.inward {
            Inward::CalRgb {
                white,
                gamma,
                columns,
            } => {
                // §8.6.5.3 run backwards: the adaptation `cie_to_srgb` applied is undone by
                // adapting from D50 onto the space's own white, the `Matrix` is inverted, and
                // each decoded component — "shall be in the range 0.0 to 1.0; component
                // values falling outside that range shall be adjusted to the nearest valid
                // value" — is clamped and raised to the reciprocal of its gamma.
                let decoded = solve_three(columns, adapt(xyz, D50, *white)).unwrap_or([0.0; 3]);
                let mut components = [0.0f32; 3];
                for ((component, value), exponent) in components.iter_mut().zip(decoded).zip(gamma)
                {
                    let clamped = channel(value);
                    *component = if *exponent > 0.0 {
                        clamped.powf(1.0 / exponent)
                    } else {
                        clamped
                    };
                }
                components
            }
            Inward::Profile(profile) => profile
                .to_device(xyz, rendering)
                .map_or([0.0; 3], |device| {
                    [channel(device[0]), channel(device[1]), channel(device[2])]
                }),
        }
    }

    /// §11.5.3's `Y` of a colour composited in this space, as one curve per component —
    /// or `None` where the space's `Y` is not a sum of one function of each component.
    ///
    /// > For CIE-based spaces, convert to the CIE 1931 XYZ space and use the Y component as
    /// > the luminosity. This produces a colorimetrically correct luminosity.
    ///
    /// For `CalRGB` the clause's EXAMPLE 1 writes the formula out: the `Y` entries of the
    /// `Matrix` weighting each gamma-decoded component, "using components of the Gamma and
    /// Matrix entries of the colour space dictionary" — the space's own XYZ, with no
    /// adaptation, which is why this reads the matrix rather than [`ColourSpace::cie_xyz_at`].
    /// A matrix profile's `Y` is the same shape, its tone curves weighted by the middle row
    /// of its matrix.
    ///
    /// **A table profile's `Y` is not a sum of three functions, and is sampled instead**
    /// (ADR 0851). Its conversion to the connection space is a lookup table, so no
    /// decomposition exists to write down — but the clause asks for one number of three
    /// components, and EXAMPLE 1's "[a]n analogous computation applies to other CIE-based
    /// colour spaces" is that number rather than a licence to leave the branch. So it is
    /// sampled at [`RGB_TABLE_SIDE`] points an axis, exactly as [`profile_stages`] samples
    /// the same profile's conversion *out*, and interpolated trilinearly by
    /// `pdf_render::Luminance`. **Without §8.6.5.9's black point compensation**, as the two
    /// separable shapes are: the clause asks for "the CIE 1931 XYZ" of the colour, and the
    /// compensation is a step toward a destination rather than part of a colour's XYZ.
    #[must_use]
    pub fn luminance(&self) -> Option<pdf_render::Luminance> {
        if let Inward::Profile(profile) = &self.inward
            && profile.matrix_stages().is_none()
        {
            let side = RGB_TABLE_SIDE;
            let mut samples = Vec::with_capacity(side.pow(3));
            for third in 0..side {
                for second in 0..side {
                    for first in 0..side {
                        #[expect(
                            clippy::cast_precision_loss,
                            reason = "a grid index below the side"
                        )]
                        let at = |index: usize| index as f32 / (side - 1) as f32;
                        let xyz = profile.to_xyz_with(
                            &[at(first), at(second), at(third)],
                            Rendering::without_black_point(),
                        );
                        samples.push(channel(xyz[1]));
                    }
                }
            }
            return pdf_render::Luminance::grid(side, Arc::from(samples));
        }
        let curves: [[f32; 3]; 256] = match &self.inward {
            Inward::CalRgb { gamma, columns, .. } => std::array::from_fn(|index| {
                #[expect(clippy::cast_precision_loss, reason = "an index below 256")]
                let component = index as f32 / 255.0;
                let mut share = [0.0f32; 3];
                for ((value, column), exponent) in share.iter_mut().zip(columns).zip(gamma) {
                    *value = column.get(1).copied().unwrap_or(0.0) * component.powf(*exponent);
                }
                share
            }),
            Inward::Profile(profile) => {
                let stages = profile.matrix_stages()?;
                std::array::from_fn(|index| {
                    #[expect(clippy::cast_precision_loss, reason = "an index below 256")]
                    let component = index as f32 / 255.0;
                    let mut share = [0.0f32; 3];
                    for (axis, value) in share.iter_mut().enumerate() {
                        let mut linear = [0.0f32; 3];
                        linear[axis] = stages.linear(axis, component);
                        *value = stages.xyz(linear)[1];
                    }
                    share
                })
            }
        };
        Some(pdf_render::Luminance::curves(Arc::new(curves)))
    }
}

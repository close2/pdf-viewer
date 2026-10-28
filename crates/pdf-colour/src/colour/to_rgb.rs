//! §8.6: a colour space's components converted to RGB, and the CIE route that does it.

use pdf_render::Color;

use super::cie::{
    D50, adapt, cal_rgb_xyz, cie_to_srgb, lab, lab_xyz, srgb_to_xyz_d50, xyz_d50_to_srgb,
};
use super::device_cmyk::{channel, cmyk};
use super::rgb_route::RgbRoute;
use super::{ColourSpace, MAX_DEPTH};
use crate::icc::Rendering;

impl ColourSpace {
    /// Converts a colour in this space to RGB.
    ///
    /// Black point compensation is applied where the space is an ICC profile, which is
    /// the `Default` behaviour ISO 32000-2 §8.6.5.9 leaves to the processor. Use
    /// [`Self::to_rgb_without_black_point`] where the document has asked for it off.
    #[must_use]
    pub fn to_rgb(&self, values: &[f32]) -> Color {
        self.to_rgb_under(values, Rendering::compensating())
    }

    /// Converts a colour without black point compensation.
    ///
    /// Required by ISO 32000-2 §8.6.5.9 in two cases: `/UseBlackPtComp` set to `OFF`, and
    /// a rendering intent of `AbsColorimetric`, where the entry "shall be treated as
    /// `OFF`" whatever it says. Absolute colorimetry means reproducing the source's
    /// actual measured colours, including a paper white that is not the display's white
    /// and a black that is not the display's black — compensating would defeat the
    /// intent's whole purpose.
    #[must_use]
    pub fn to_rgb_without_black_point(&self, values: &[f32]) -> Color {
        self.to_rgb_under(values, Rendering::without_black_point())
    }

    /// Converts a colour under the rendering parameters an object states.
    ///
    /// The general form of the two above, and what [`super::compositing::Compositing::paint`] calls: §8.6.5.8's
    /// intent decides which of a profile's `A2B` transforms the colour goes through and
    /// §8.6.5.9's entry decides the compensation, which [`Rendering`] carries together.
    #[must_use]
    pub fn to_rgb_under(&self, values: &[f32], rendering: Rendering) -> Color {
        self.to_rgb_at(values, 0, rendering)
    }

    /// [`RgbRoute::components_of`], carrying the recursion depth a nested space costs.
    ///
    /// The arms are §11.7.2's: a device space of three colourants is the group's space "for
    /// compositing purposes only", a colour already in the space keeps its components
    /// (§11.6.6's "not equivalent"), a special space is the colour it resolves to in its base
    /// or alternate, and everything else goes in from an XYZ — its own for a CIE-based
    /// colour, sRGB's for a device colour of another count.
    pub(super) fn rgb_components_at(
        &self,
        values: &[f32],
        depth: usize,
        rendering: Rendering,
        route: &RgbRoute,
    ) -> [f32; 3] {
        if depth > MAX_DEPTH {
            return [0.0; 3];
        }
        let at = |index: usize| channel(values.get(index).copied().unwrap_or(0.0));
        match self {
            Self::Rgb => [at(0), at(1), at(2)],
            Self::CalRgb { .. } | Self::Icc { .. }
                if self.rgb_identity() == Some(route.identity) =>
            {
                [at(0), at(1), at(2)]
            }
            Self::Indexed { base, .. } => base.rgb_components_at(
                &self.entry_of(values),
                depth.saturating_add(1),
                rendering,
                route,
            ),
            Self::Separation {
                alternate, tints, ..
            } => alternate.rgb_components_at(
                &tints.eval(values),
                depth.saturating_add(1),
                rendering,
                route,
            ),
            Self::Pattern { base } => base.as_ref().map_or([0.0; 3], |base| {
                base.rgb_components_at(values, depth.saturating_add(1), rendering, route)
            }),
            _ => match self.cie_xyz_at(values, depth, rendering) {
                Some(xyz) => route.components_with_xyz(xyz, rendering),
                None => route.components_with_xyz(
                    srgb_to_xyz_d50(self.to_rgb_at(values, depth, rendering)),
                    rendering,
                ),
            },
        }
    }

    /// §11.5.3's luminosity of a colour in a CIE-based space, or `None` for any other.
    ///
    /// > For CIE-based spaces, convert to the CIE 1931 XYZ space and use the Y component as
    /// > the luminosity. This produces a colorimetrically correct luminosity.
    ///
    /// The `Y` of `Self::cie_xyz_at`, clamped to the unit a mask value can hold — a
    /// `CalGray` component decoded by its gamma times a white point whose `Y` §8.6.5.2 makes
    /// 1.0, a profile's `Y` at D50, which is also 1.0 at white.
    ///
    /// **No mask this tree draws is weighed by it**: the owner chose §11.5.3's device branch for
    /// every mask group (A23, ADR 2552), and this is the clause's own branch, kept for the
    /// choice between the two that A23 leaves open.
    #[must_use]
    pub fn cie_luminance(&self, values: &[f32]) -> Option<f32> {
        self.cie_xyz_at(values, 0, Rendering::compensating())
            .map(|xyz| channel(xyz[1]))
    }

    /// The D50 XYZ a colour in a CIE-based space states, or `None` for a device or special
    /// space, which states no XYZ of its own.
    ///
    /// The same arithmetic [`Self::to_rgb_at`] runs for the four CIE-based families up to the
    /// one matrix that turns an XYZ into a pixel — `CalGray` and `CalRGB` adapted from their own
    /// white point onto D50 as [`cie_to_srgb`] does, `Lab` the same way from Table 64's own,
    /// a profile through its `A2B` with §8.6.5.9's compensation as asked — so that a colour
    /// converted *between* two CIE-based spaces (§10.3.1) takes exactly the route it would have
    /// taken to the screen, minus the screen.
    pub(super) fn cie_xyz_at(
        &self,
        values: &[f32],
        depth: usize,
        rendering: Rendering,
    ) -> Option<[f32; 3]> {
        if depth > MAX_DEPTH {
            return None;
        }
        let at = |index: usize| values.get(index).copied().unwrap_or(0.0);
        match self {
            Self::Icc { profile } => Some(profile.to_xyz_with(values, rendering)),
            Self::Lab { white, range, .. } => Some(adapt(
                lab_xyz(at(0), at(1), at(2), *range, *white),
                *white,
                D50,
            )),
            Self::CalGray {
                white,
                black: _,
                gamma,
            } => {
                let decoded = channel(at(0)).powf(*gamma);
                let xyz = [white[0] * decoded, white[1] * decoded, white[2] * decoded];
                Some(adapt(xyz, *white, D50))
            }
            Self::CalRgb {
                white,
                black: _,
                gamma,
                matrix,
            } => {
                let decoded = [
                    channel(at(0)).powf(gamma[0]),
                    channel(at(1)).powf(gamma[1]),
                    channel(at(2)).powf(gamma[2]),
                ];
                Some(adapt(cal_rgb_xyz(decoded, matrix), *white, D50))
            }
            Self::Indexed { base, .. } => {
                base.cie_xyz_at(&self.entry_of(values), depth.saturating_add(1), rendering)
            }
            Self::Separation {
                alternate, tints, ..
            } => alternate.cie_xyz_at(&tints.eval(values), depth.saturating_add(1), rendering),
            // §10.8.3's steps b) and c), which end in an XYZ; step d) is in `to_rgb_at`.
            Self::Simulated { process, spots, .. } => Some(Self::simulated_xyz(
                process.as_deref(),
                spots,
                values,
                depth,
                rendering,
            )),
            _ => None,
        }
    }

    pub(super) fn to_rgb_at(&self, values: &[f32], depth: usize, rendering: Rendering) -> Color {
        if depth > MAX_DEPTH {
            return Color::BLACK;
        }
        let at = |index: usize| values.get(index).copied().unwrap_or(0.0);

        match self {
            Self::Gray => {
                let g = channel(at(0));
                Color::rgb(g, g, g)
            }
            Self::Rgb => Color::rgb(channel(at(0)), channel(at(1)), channel(at(2))),
            Self::Cmyk => cmyk(at(0), at(1), at(2), at(3)),
            Self::Icc { profile } => profile.to_rgb_with(values, rendering),
            // `black` is read but not applied, as for the two spaces below.
            Self::Lab { white, range, .. } => lab(at(0), at(1), at(2), *range, *white),
            // `black` is read but not applied — `cie_to_srgb` carries the argument.
            Self::CalGray {
                white,
                black: _,
                gamma,
            } => {
                // §8.6.5.2: "the A component shall be first decoded by the gamma function,
                // and the result shall be multiplied by the components of the white point
                // to obtain the L, M and N components", which are also X, Y and Z because
                // a CalGray has no second transformation stage.
                let decoded = channel(at(0)).powf(*gamma);
                let xyz = [white[0] * decoded, white[1] * decoded, white[2] * decoded];
                cie_to_srgb(xyz, *white)
            }
            Self::CalRgb {
                white,
                black: _,
                gamma,
                matrix,
            } => {
                // §8.6.5.3: decode each component by its own gamma, then multiply the
                // three-element vector by `Matrix` to obtain XYZ. `Matrix` is given as
                // three XYZ triples, one per input component, so each triple is a column.
                let decoded = [
                    channel(at(0)).powf(gamma[0]),
                    channel(at(1)).powf(gamma[1]),
                    channel(at(2)).powf(gamma[2]),
                ];
                cie_to_srgb(cal_rgb_xyz(decoded, matrix), *white)
            }
            Self::Indexed { base, .. } => {
                base.to_rgb_at(&self.entry_of(values), depth.saturating_add(1), rendering)
            }
            Self::Separation {
                alternate, tints, ..
            } => {
                let converted = tints.eval(values);
                alternate.to_rgb_at(&converted, depth.saturating_add(1), rendering)
            }
            // §10.8.3 step d): "Convert the result to the actual device colour space and
            // output it." This device's is sRGB (ADR 0009).
            Self::Simulated { process, spots, .. } => xyz_d50_to_srgb(Self::simulated_xyz(
                process.as_deref(),
                spots,
                values,
                depth,
                rendering,
            )),
            // §8.6.6.4's `/All`, complemented for an additive device: a tint of 1.0 is every
            // colourant at maximum, which on a monitor is black.
            Self::AllColourants => {
                let grey = 1.0 - channel(values.first().copied().unwrap_or(1.0));
                Color::rgb(grey, grey, grey)
            }
            // §8.6.6.4's `/None`, which "shall have no effect on the current page". An alpha
            // of zero is that effect exactly: §11.3.6's compositing formulae leave the
            // backdrop untouched at zero alpha under every blend mode, so nothing downstream
            // needs to know this colour is special.
            Self::NoColourant { .. } => Color::TRANSPARENT,
            // A pattern has no colour of its own. Where it names an underlying space, an
            // uncoloured pattern's colour is in that; otherwise there is nothing to say.
            Self::Pattern { base } => base.as_ref().map_or(Color::BLACK, |base| {
                base.to_rgb_at(values, depth.saturating_add(1), rendering)
            }),
        }
    }

    /// The alpha [`Self::to_rgb_under`] gives every colour of this space, without converting
    /// one.
    ///
    /// A conversion's alpha is not a function of the components. It is §8.6.6.4's `/None`,
    /// which "shall have no effect on the current page" and is painted transparent, reached
    /// through the same three indirections [`Self::to_rgb_at`] follows — an `Indexed` base, a
    /// `Separation`'s alternate, a pattern's underlying space — under the same depth bound, and
    /// opaque everywhere else. A caller that wants only the alpha asks this rather than
    /// converting a colour it throws away: on a shading's ramp inside a four-component group
    /// that conversion was a tint transform run once a sample for nothing (ADR 2332).
    ///
    /// The match names every space rather than ending in a wildcard, so a new space is decided
    /// here when it is decided in [`Self::to_rgb_at`] — the one conversion of trap 6, read for
    /// its alpha — and `a_spaces_alpha_is_its_conversions_alpha` holds the two to each other.
    #[must_use]
    pub(crate) fn rgb_alpha(&self) -> f32 {
        self.rgb_alpha_at(0)
    }

    fn rgb_alpha_at(&self, depth: usize) -> f32 {
        if depth > MAX_DEPTH {
            return Color::BLACK.a;
        }
        match self {
            Self::Indexed { base, .. } | Self::Pattern { base: Some(base) } => {
                base.rgb_alpha_at(depth.saturating_add(1))
            }
            Self::Separation { alternate, .. } => alternate.rgb_alpha_at(depth.saturating_add(1)),
            Self::NoColourant { .. } => Color::TRANSPARENT.a,
            Self::Gray
            | Self::Rgb
            | Self::Cmyk
            | Self::Icc { .. }
            | Self::Lab { .. }
            | Self::CalGray { .. }
            | Self::CalRgb { .. }
            | Self::Simulated { .. }
            | Self::AllColourants
            | Self::Pattern { base: None } => Color::BLACK.a,
        }
    }
}

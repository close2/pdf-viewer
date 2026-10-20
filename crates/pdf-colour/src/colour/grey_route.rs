//! The way into and out of a one-component CIE-based blending space (§11.3.4, §11.6.6).

use std::sync::Arc;

use pdf_render::Color;

use super::ColourSpace;
use super::conversion::InkScale;
use super::device_cmyk::channel;
use super::rgb_route::inward_cube;

/// Which one-component space a [`GreyRoute`] is the route into and out of.
///
/// What [`super::compositing::Compositing`]'s key and §11.6.6's "not equivalent to the group colour space" both
/// need: two `CalGray` dictionaries stating one gamma and one white point are one space, and
/// a profile is the profile [`crate::icc::Profile::identity`] says it is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum GreyIdentity {
    /// §8.6.5.2's space, by the bits of its `Gamma` and `WhitePoint`.
    CalGray {
        /// `Gamma`, as `f32::to_bits`.
        gamma: u32,
        /// `WhitePoint`, as `f32::to_bits` apiece.
        white: [u32; 3],
    },
    /// A one-component `ICCBased` space, by its profile.
    Profile(u128),
}

/// How many samples [`GreyRoute`] takes of a one-component space's conversion out.
///
/// Two hundred and fifty-six, so that every component an eight-bit channel can hold lands
/// on a sample exactly and only a component recovered from under a partial alpha is
/// interpolated. A one-dimensional table this size costs three kilobytes, which is why it
/// needs no registry where a press needs [`Presses`].
const GREY_CURVE_SAMPLES: usize = 256;

/// The way into a `CalGray` or `ICCBased` 'GRAY' blending colour space, and the way out
/// (ISO 32000-2 §11.3.4, §11.6.6, §8.6.5.2, §8.6.5.5).
///
/// §11.6.6 has every painting operator inside such a group "convert source colours in a
/// colour space (that are not equivalent to the group colour space) to the group colour
/// space before compositing objects into the group", and §11.4.7 converts the composited
/// result out to the device at the end. The standard states the conversion *out* in full —
/// §8.6.5.2's gamma and white point, a profile's `A2B` — and states the conversion *in* for
/// no colour at all: a `CalGray` is defined from its component to XYZ and never the other
/// way, and §8.6.5.5 uses a profile's "to CIE" information only. So this tree takes the
/// conversion in to be the **inverse of the conversion out on the greys**, which is the
/// construction ADR 0263 chose for a press one dimension up: the space's own curve, sampled
/// ([`GreyRoute::curve`]), and a search over the same samples for the component whose device
/// colour has the grey a source colour has ([`GreyRoute::component_of`]). What that buys is
/// the property the clause's "equivalent" clause implies — an opaque device grey painted
/// into the group comes back out as itself, and a colour in the group's own space needs no
/// conversion at all.
///
/// A source colour's *grey* is §10.4.2.2's and §10.4.2.3's, through [`InkScale::grey_of`] —
/// the same function every `/Luminosity` mask and every `DeviceGray` group takes, so that
/// §11.6.6's one sentence has one conversion in this tree (trap 6; ADR 0790 records the
/// choice between this route and §10.3's, and `doc/todo/23` prices moving both at once).
///
/// **A curve with no inverse is not a route.** The search needs the sampled greys to be
/// monotone, which every gamma and every tone curve the standard admits is; a profile whose
/// curve is not has no component for a grey, and [`GreyRoute::of`] answers `None`, which
/// keeps the report the space had before this existed.
#[derive(Debug)]
pub struct GreyRoute {
    /// The conversion out, as the backends apply it.
    curve: pdf_render::GreyCurve,
    /// §10.4.2.2's grey of each sample of `curve`, non-decreasing, for the search in.
    greys: Vec<f32>,
    /// Which space this is the route into.
    pub(super) identity: GreyIdentity,
}

impl GreyRoute {
    /// The route into and out of `space`, or `None` where `space` is not a one-component
    /// space §11.3.4 lists, or its curve has no inverse.
    #[must_use]
    pub fn of(space: &ColourSpace) -> Option<Self> {
        let identity = space.grey_identity()?;
        let samples: Vec<[f32; 3]> = (0..GREY_CURVE_SAMPLES)
            .map(|index| {
                #[expect(clippy::cast_precision_loss, reason = "an index below 256")]
                let component = index as f32 / (GREY_CURVE_SAMPLES - 1) as f32;
                // The crate's ordinary conversion of a colour in this space, black point
                // compensation included: what the group's result becomes when it is
                // composited onto its parent is what a colour in the space becomes anywhere.
                let colour = space.to_rgb(&[component]);
                [colour.r, colour.g, colour.b]
            })
            .collect();
        let greys: Vec<f32> = samples
            .iter()
            .map(|sample| Color::rgb(sample[0], sample[1], sample[2]).grey_level())
            .collect();
        let monotone = greys.windows(2).all(|pair| pair[1] >= pair[0])
            && greys.last().copied().unwrap_or(0.0) > greys.first().copied().unwrap_or(0.0);
        if !monotone {
            return None;
        }
        let curve = pdf_render::GreyCurve::new(Arc::from(samples))?;
        Some(Self {
            curve,
            greys,
            identity,
        })
    }

    /// The conversion out of the space, for the display list to carry.
    #[must_use]
    pub fn curve(&self) -> &pdf_render::GreyCurve {
        &self.curve
    }

    /// The conversion *into* the space from this processor's own three components, as a cube
    /// whose three channels carry the one component (ISO 32000-2 §11.6.6, §10.4.2.2).
    ///
    /// [`GreyRoute::component_of`] asked of a `DeviceRGB` colour, which is what §11.6.6's
    /// painting of a group's result into a parent compositing in this space does, split at
    /// the stage that is not affine. §10.4.2.2's weights are stated on the components
    /// themselves rather than on linear light, so the input curves are the identity and the
    /// grid holds the weighted sum; the search that inverts the space's own curve is the
    /// output curve, and it is one curve for all three channels because a one-component
    /// space composites one number. ADR 1267.
    #[must_use]
    pub fn into_cube(&self) -> Option<pdf_render::ColourCube> {
        let light =
            |components: [f32; 3]| [InkScale::Unit.grey_of(&ColourSpace::Rgb, &components); 3];
        let encode = |_axis: usize, value: f32| self.component_with_grey(channel(value));
        inward_cube(&|value| value, &light, &encode)
    }

    /// Which space this is the route into.
    #[must_use]
    pub fn identity(&self) -> GreyIdentity {
        self.identity
    }

    /// The component a colour becomes inside the space: §11.6.6's conversion in.
    ///
    /// A colour already in the space keeps its component — "(that are not equivalent to the
    /// group colour space)" — and any other becomes the component whose device colour has
    /// its §10.4.2.2 or §10.4.2.3 grey, found in the sampled curve.
    #[must_use]
    pub fn component_of(&self, space: &ColourSpace, values: &[f32]) -> f32 {
        if space.grey_identity() == Some(self.identity) {
            return channel(values.first().copied().unwrap_or(0.0));
        }
        self.component_with_grey(InkScale::Unit.grey_of(space, values))
    }

    /// The component whose device colour has the grey `grey`, by search over the samples.
    ///
    /// Between two samples the answer is linear, which is exactly what
    /// [`pdf_render::GreyCurve::convert`] assumes on the way out, so the two are inverses
    /// of each other to the precision of the samples rather than of the curve.
    #[must_use]
    pub fn component_with_grey(&self, grey: f32) -> f32 {
        let last = self.greys.len().saturating_sub(1);
        // The first sample whose grey is above `grey`; the answer lies just below it.
        let above = self.greys.partition_point(|&sample| sample <= grey);
        if above == 0 {
            return 0.0;
        }
        if above > last {
            return 1.0;
        }
        let below = above.saturating_sub(1);
        let (Some(&low), Some(&high)) = (self.greys.get(below), self.greys.get(above)) else {
            return 1.0;
        };
        // `high > grey >= low` by the partition, so the span is positive.
        let fraction = (grey - low) / (high - low);
        #[expect(clippy::cast_precision_loss, reason = "sample indices below 256")]
        let component = (below as f32 + fraction) / last as f32;
        channel(component)
    }
}

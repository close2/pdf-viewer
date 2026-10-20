//! What one colour conversion needs beyond the colour: the space composited into, the black point, and the ink scale of a luminosity mask.

use std::sync::Arc;

use pdf_render::Color;
use pdf_syntax::{Dictionary, Document, Object};

use super::compositing::Compositing;
use super::spots::DeviceSpots;
use super::{ColourSpace, Reading, Separations};
use crate::black_generation::BlackGeneration;
use crate::icc::Rendering;

/// Everything one colour conversion needs beyond the colour: what the result is composited
/// into, and whether ISO 32000-2 §8.6.5.9's black point compensation applies.
///
/// The two travel together because a conversion needs both and neither belongs to the colour.
/// [`Compositing`] is a property of the *raster* being painted (ADR 0220); the black point is a
/// property of the **object** being painted, which is how §8.6.5.9 states it:
///
/// > If the current render intent of an object is AbsColorimetric then the value of
/// > UseBlackPtComp shall be treated as OFF .
///
/// §8.6.5.8 gives an object three routes to that intent, the third of which is Table 87's
/// `/Intent` on an image dictionary.
///
/// **This type is why the third route is obeyed at all.** The flag is paired with the target
/// that is threaded through `pdf_model::image`, `crate::shading` and `crate::mesh`, so an image
/// sample, a shading ramp and a mesh vertex receive the intent as a path's colour and a glyph's
/// do, and omitting it is a compile error rather than a habit: there is no [`Compositing::paint`]
/// call that can leave it out.
///
/// **And a third thing, which is not a property of the colour, the target or the object but of
/// the page**: §14.11.5's output intent, where the document states one this
/// tree reads. `ColourSpace::device_family` ranks it between §8.6.5.6's default and the device
/// space itself, so it decides what `/DeviceCMYK` *means* — and an image's, a shading's and a
/// mesh's colour spaces are parsed where their samples are converted, in `pdf_model::image`,
/// `crate::shading` and `crate::mesh`, after the interpreter has handed the work over; parsed
/// with no intent, an image in `DeviceCMYK` would draw through the assumed press beside a fill
/// drawn through the intent (ADR 1008). Carrying the intent in the same
/// value as the target and the black point is what makes the omission a type error rather than
/// a habit, exactly as pairing the flag with the target did for §8.6.5.9.
///
/// **And the flag is a pair, which is what makes §8.6.5.8's intent do something** (ADR 1032):
/// beyond the black point, [`Rendering`] carries the `A2B` transform they select as well, so an
/// image that states `/Intent /Perceptual` over a profile with an `A2B0` of its own converts
/// through that table and not through the colorimetric one beside it.
#[derive(Debug, Clone)]
pub struct Conversion {
    /// What the converted colour is composited into.
    into: Compositing,
    /// §8.6.5.8's intent and §8.6.5.9's black point compensation, as the object being painted
    /// states them.
    rendering: Rendering,
    /// The profile §14.11.5's output intent names for this page, if it names one this tree
    /// reads.
    ///
    /// A profile rather than a [`ColourSpace`] because Table 401 makes an intent's
    /// `/DestOutputProfile` "[a]n ICC profile stream" and nothing else, and because the
    /// caches keyed on a conversion — `shading::Cache`, `image::RasterCache` — need an
    /// identity to compare, which [`crate::icc::Profile::identity`] is.
    output_intent: Option<Arc<crate::icc::Profile>>,
    /// The profile of the `ICCBased` CMYK blending space §11.7.2 redefines `DeviceCMYK` to be
    /// inside the group being painted, carried for the output intent's reason: it decides what
    /// `/DeviceCMYK` *means* where an image's, a shading's or a mesh's space is parsed
    /// ([`Reading::within_cmyk_group`], ADR 2012).
    cmyk_group: Option<Arc<crate::icc::Profile>>,
    /// §10.4.2.4's black generation and undercolour removal, where the graphics state states a
    /// function.
    ///
    /// Here for the same reason the black point is: §11.7.5.3 lists the four together — "[t]he
    /// rendering intent, black-generation, undercolour-removal and black point compensation
    /// parameters control certain colour conversions" — and says they "may need to be applied
    /// earlier than the actual rendering of colour onto the page", which is exactly why an
    /// image's samples, a shading's ramp and a mesh's vertices need the state to travel with
    /// them. ADR 1207.
    black_generation: Option<Arc<BlackGeneration>>,
    /// Whether §10.8.3's separation simulation is in force for this page.
    ///
    /// Here for the same reason the output intent is: it decides what a `DeviceN` colour
    /// space *means*, and an image's, a shading's and a mesh's colour spaces are parsed where
    /// their samples are converted, after the interpreter has handed the work over. A reader
    /// who asks for the simulation and gets it on fills but not on images has been given two
    /// pages at once. ADR 1229.
    separations: Separations,
    /// The spot colourants §10.8.3's simulated device carries beside a target that is not a
    /// press, which an image's samples, a shading's ramp and a mesh's vertices must leave off the
    /// group's process components as a fill does (§11.7.3, ADR 1329). Empty on every conversion
    /// a backend draws unseparated.
    spots: DeviceSpots,
}

/// What distinguishes one [`Conversion`] from another, for the caches keyed on one.
///
/// Written out for [`Compositing`]'s reason: the intent is behind an `Arc`, and two `Arc`s of
/// one profile — one per interpretation of the same page — are one intent. Every derived trait
/// below goes through here, which keeps them agreeing with equality.
type ConversionKey<'a> = (
    &'a Compositing,
    Rendering,
    Option<u128>,
    Option<u128>,
    Option<usize>,
    Separations,
    &'a DeviceSpots,
);

impl Conversion {
    /// This value as the tuple every trait below is defined on.
    fn key(&self) -> ConversionKey<'_> {
        (
            &self.into,
            self.rendering,
            self.output_intent
                .as_ref()
                .map(|profile| profile.identity()),
            self.cmyk_group.as_ref().map(|profile| profile.identity()),
            self.black_generation
                .as_ref()
                .map(BlackGeneration::identity),
            self.separations,
            &self.spots,
        )
    }

    /// A conversion into `into`, compensating or not as §8.6.5.9's parameters decide, on a
    /// page with no output intent.
    ///
    /// The interpreter's two constructors — `Interpreter::conversion_under` and
    /// `Interpreter::image_conversion` — put the page's intent on afterwards through
    /// [`Self::under_output_intent`]; a conversion built anywhere else is for a colour that
    /// has no page, or a mask read for its coverage, and states no intent honestly.
    #[must_use]
    pub fn new(into: Compositing, rendering: Rendering) -> Self {
        Self {
            into,
            rendering,
            output_intent: None,
            cmyk_group: None,
            black_generation: None,
            separations: Separations::Alternate,
            spots: DeviceSpots(None),
        }
    }

    /// The same conversion on a page whose output intent is `intent`.
    ///
    /// `intent` is what `pdf_model::content`'s `output_intent_space` answered — an
    /// [`ColourSpace::Icc`] built from the `/DestOutputProfile`, or nothing. It is typed as a
    /// space because that is what `ColourSpace::device_family` substitutes for a device
    /// family, and what `pdf_model::content::transparency` hands `press_for_entry`; only its
    /// profile is kept here, and a space of any other variant is not an output intent by
    /// Table 401's definition and is not carried.
    #[must_use]
    pub fn under_output_intent(mut self, intent: Option<&ColourSpace>) -> Self {
        self.output_intent = match intent {
            Some(ColourSpace::Icc { profile }) => Some(Arc::clone(profile)),
            Some(_) | None => None,
        };
        self
    }

    /// What §14.11.5's output intent says this page's device colours mean, if it says.
    ///
    /// The argument [`ColourSpace::parse_with_output_intent`] takes, answered as a space
    /// because that function substitutes it for a device family. A refcount, not a copy.
    #[must_use]
    pub fn output_intent(&self) -> Option<ColourSpace> {
        self.output_intent.as_ref().map(|profile| ColourSpace::Icc {
            profile: Arc::clone(profile),
        })
    }

    /// The same conversion inside a group whose `ICCBased` CMYK blending space §11.7.2
    /// redefines `DeviceCMYK` to be, or outside every such group; a space of any other variant
    /// redefines nothing and is not carried. `pdf_model::content` is the only caller.
    #[must_use]
    pub fn within_cmyk_group(mut self, space: Option<&ColourSpace>) -> Self {
        self.cmyk_group = match space {
            Some(ColourSpace::Icc { profile }) => Some(Arc::clone(profile)),
            Some(_) | None => None,
        };
        self
    }

    /// The space §11.7.2 redefines `DeviceCMYK` to be for this conversion's colours, if any:
    /// the argument [`Reading::within_cmyk_group`] takes. A refcount, not a copy.
    #[must_use]
    pub fn cmyk_group(&self) -> Option<ColourSpace> {
        self.cmyk_group.as_ref().map(|profile| ColourSpace::Icc {
            profile: Arc::clone(profile),
        })
    }

    /// A conversion onto the device with compensation on.
    ///
    /// The pair a caller wants where the question is the *colour space* rather than the state:
    /// a mask's own samples, a `/Decode` array's bounds, an image whose colours are read only
    /// to be counted. `Compositing::Device` says the first half, and the second is
    /// `/UseBlackPtComp Default`, which §8.6.5.9 leaves to the processor and which this one
    /// compensates.
    #[must_use]
    pub fn device() -> Self {
        Self::new(Compositing::Device, Rendering::compensating())
    }

    /// What the converted colour is composited into.
    #[must_use]
    pub fn target(&self) -> &Compositing {
        &self.into
    }

    /// The same rendering parameters and the same page, composited into something else.
    #[must_use]
    pub fn into_target(&self, into: Compositing) -> Self {
        Self {
            into,
            rendering: self.rendering,
            output_intent: self.output_intent.clone(),
            cmyk_group: self.cmyk_group.clone(),
            black_generation: self.black_generation.clone(),
            separations: self.separations,
            spots: self.spots.clone(),
        }
    }

    /// The rendering parameters this conversion carries.
    #[must_use]
    pub fn rendering(&self) -> Rendering {
        self.rendering
    }

    /// The same conversion under §10.4.2.4's functions, where the graphics state states one.
    ///
    /// `pdf_model::content` is the only caller: Table 57's `/BG`, `/BG2`, `/UCR` and `/UCR2` are
    /// read there — from an `/ExtGState` and from a shading pattern's own (§11.6.7) — and this is
    /// how the pair reaches the three routes that convert after the interpreter has handed the
    /// work over.
    #[must_use]
    pub fn under_black_generation(mut self, generation: Option<Arc<BlackGeneration>>) -> Self {
        self.black_generation = generation;
        self
    }

    /// The same conversion under §10.8.3's separation simulation, or without it.
    ///
    /// `pdf_model::content` is the only caller: the request is a person's, it reaches the
    /// interpreter as a `ViewState` input, and this is how it reaches the three routes that
    /// parse a colour space after the interpreter has handed the work over.
    #[must_use]
    pub fn under_separations(mut self, separations: Separations) -> Self {
        self.separations = separations;
        self
    }

    /// Whether §10.8.3's separation simulation is in force for this page.
    #[must_use]
    pub fn separations(&self) -> Separations {
        self.separations
    }

    /// The same conversion on a run whose simulated device carries `spots` beside its target
    /// (`Compositing::paint_beside`). `pdf_model::content` is the only caller.
    #[must_use]
    pub fn beside(mut self, spots: DeviceSpots) -> Self {
        self.spots = spots;
        self
    }

    /// Parses a colour space as this conversion's page reads it: under its output intent,
    /// §11.7.2's redefinition of `DeviceCMYK` and §10.8.3's preference.
    ///
    /// The one reading for every route that parses a space after the interpreter has handed the
    /// work over — an image's, a shading's, a mesh's — so that a part of it cannot be left out at
    /// one of them (ADRs 1008, 1229, 2012).
    #[must_use]
    pub fn parse_space(
        &self,
        document: &Document,
        object: &Object,
        resources: &Dictionary,
    ) -> Option<ColourSpace> {
        let intent = self.output_intent();
        let cmyk_group = self.cmyk_group();
        ColourSpace::parse_under(
            document,
            object,
            resources,
            Reading::new(intent.as_ref())
                .within_cmyk_group(cmyk_group.as_ref())
                .under_separations(self.separations),
        )
    }

    /// §10.4.2.4's pair this conversion carries, if the state stated one.
    #[must_use]
    pub fn black_generation(&self) -> Option<&BlackGeneration> {
        self.black_generation.as_deref()
    }

    /// The colour `values` become, through [`Compositing::paint_beside`].
    #[must_use]
    pub fn paint(&self, space: &ColourSpace, values: &[f32]) -> Color {
        self.into.paint_beside(
            &self.spots,
            space,
            values,
            self.rendering,
            self.black_generation(),
        )
    }
}

impl PartialEq for Conversion {
    fn eq(&self, other: &Self) -> bool {
        self.key() == other.key()
    }
}

impl Eq for Conversion {}

impl PartialOrd for Conversion {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Conversion {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.key().cmp(&other.key())
    }
}

impl std::hash::Hash for Conversion {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.key().hash(state);
    }
}

/// How much ink one channel of a `/Luminosity` mask group's raster has to hold.
///
/// §11.5.3 composites the group *first* and takes §10.4.2.3's `1 − min(1, ink)` of the
/// result, so the quantity that has to survive the compositing is the ink itself — and a
/// rendered channel holds `0..=1` where an ink reaches 2.0. This is the divisor that makes it
/// fit: the group is painted in `1 − ink ÷ scale` and the mask reads `1 − min(1, scale × (1 −
/// channel))` back off it, which is the clause's own arithmetic with the `min` where the
/// clause puts it.
///
/// **There are exactly two values and the blending space picks one**, because §11.6.6 makes
/// the group's `/CS` "[t]he colour space into which colours shall be converted when painted
/// into the group":
///
/// - A **`DeviceGray`** group converts a colour by §10.4.2.3 on the way in, and that
///   conversion *is* the `min` — a grey level is `1 − min(1, ink)` and its own ink is one
///   minus that, so nothing painted into such a group can weigh more than [`Self::Unit`].
/// - A **`DeviceCMYK`** group keeps four components through the compositing, and the largest
///   ink four clamped components can weigh is `0.3 + 0.59 + 0.11 + 1.0` — [`Self::Double`],
///   which registration black `/BC [1 1 1 1]` reaches exactly.
///
/// A colour arriving from any *other* space weighs at most one unit in either: §10.4.2.4's
/// black generation cancels out of §10.4.2.3's weights, so an RGB or CIE-based colour taken
/// into `DeviceCMYK` weighs `1 − (0.3 R + 0.59 G + 0.11 B)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum InkScale {
    /// One unit of ink, which is a `DeviceGray` group and costs the channel nothing.
    Unit,
    /// Two units, which is a `DeviceCMYK` group: the ink of registration black.
    Double,
}

impl InkScale {
    /// The divisor itself.
    #[must_use]
    pub fn factor(self) -> f32 {
        match self {
            Self::Unit => 1.0,
            Self::Double => 2.0,
        }
    }

    /// What a colour is painted as inside a group of this scale.
    ///
    /// `1 − ink ÷ scale`, which is [`ColourSpace::luminosity`] at [`Self::Unit`] and holds a
    /// whole unit of excess ink at [`Self::Double`]. The clamp is not dead code at
    /// [`Self::Unit`]: that is where §11.6.6's conversion into a `DeviceGray` group applies
    /// §10.4.2.3's `min`, and a `k` operator inside such a group is the case that needs it.
    #[must_use]
    pub fn grey_of(self, space: &ColourSpace, values: &[f32]) -> f32 {
        1.0 - (space.ink(values) / self.factor()).min(1.0)
    }

    /// The mask value §11.5.3 derives from a composited channel of this scale.
    ///
    /// The inverse of [`Self::grey_of`] with the clause's `min` applied after the
    /// compositing rather than before it, which is the whole point of the scale. Written
    /// beside its inverse so the two cannot drift; `a_scaled_channel_carries_the_clauses_own_grey`
    /// is the arithmetic that pins them together.
    #[must_use]
    pub fn mask_value(self, channel: f32) -> f32 {
        1.0 - (self.factor() * (1.0 - channel)).min(1.0)
    }
}

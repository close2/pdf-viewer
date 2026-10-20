//! Colour spaces, resolved to RGB.
//!
//! Every colour in a PDF is a list of numbers whose meaning depends on the space it was
//! set in. This module is where that meaning is applied, so that nothing below the
//! interpreter ever sees a colour space — see [`pdf_render::Color`] on why that boundary
//! matters.
//!
//! # What is exact and what is not
//!
//! `DeviceGray`, `DeviceRGB`, `Indexed`, `Separation` and `DeviceN` are exact: the first
//! three are definitional and the last two are defined *by* a tint transform function,
//! which this crate evaluates.
//!
//! `DeviceCMYK` is the one space with no exact answer, and this comment used to say it "uses
//! the naive conversion" while the code interpolated the sixteen corners of the ink cube —
//! see `CMYK_CORNERS`, and ADRs 0009 and 0042 for why that is a *documented choice* between
//! two things §10.4.2.1 ranks rather than an approximation of something defined. `ICCBased`
//! falls back to the device space with the same component count when the profile cannot be
//! parsed — the specification explicitly permits that, and the alternative is refusing the
//! document.
//!
//! `Lab`, `CalGray` and `CalRGB` are converted properly, through CIE XYZ. They are the
//! three spaces the specification defines *in CIE terms*, so there is an answer to derive
//! and no excuse for approximating it.
//!
//! # One route from XYZ to the screen
//!
//! Everything CIE-based — `Lab`, `CalGray`, `CalRGB` and every ICC profile — arrives at
//! [`xyz_d50_to_srgb`], and nothing else turns an XYZ into a pixel. The same rule as
//! `to_rgb`, one level down: three separate `DeviceCMYK` conversions once disagreed in this
//! tree without anything looking wrong, and an XYZ matrix copied into a second place would
//! fail the same way and be just as invisible.

mod cie;
mod compositing;
mod conversion;
mod device_cmyk;
mod entries;
mod grey_route;
mod ink_search;
mod parsing;
mod press;
mod queries;
mod rgb_route;
mod separation;
mod spots;
mod to_cmyk;
mod to_rgb;

pub use cie::cie_to_srgb;
pub(crate) use cie::{D50, lab_to_xyz, xyz_d50_to_srgb, xyz_to_lab};
pub use compositing::{Compositing, Plane};
pub use conversion::{Conversion, InkScale};
pub use grey_route::{GreyIdentity, GreyRoute};
pub(crate) use ink_search::solve_three;
pub use press::{
    MAX_PRESSES, Press, PressIdentity, Presses, assumed_press, device_cmyk_blending_space,
    device_ink_press, press_for_profile, presses_cached, presses_sampled,
};
pub use rgb_route::{RgbIdentity, RgbRoute};
pub use spots::{DeviceSpots, PROCESS_COLOURANTS, separation_conversions};

use std::sync::Arc;

use pdf_syntax::Name;

use crate::function::Function;

/// How deep a chain of colour space references may nest.
///
/// `Indexed` names a base space, which may itself be `Separation`, whose alternate may be
/// `ICCBased`. Real chains are two or three deep; a longer one is a cycle.
const MAX_DEPTH: usize = 8;

/// A colour space, resolved far enough to convert colours.
#[derive(Debug, Clone)]
pub enum ColourSpace {
    /// One component, from black to white.
    Gray,
    /// Three components, red, green and blue.
    Rgb,
    /// Four components, cyan, magenta, yellow and black.
    Cmyk,
    /// Three components in the CIE L*a*b* space: ISO 32000-2 §8.6.5.4, Table 64.
    Lab {
        /// The diffuse white point, as CIE 1931 XYZ: the `X_W`, `Y_W` and `Z_W` the clause's
        /// second transformation stage multiplies by, and the white [`cie_to_srgb`] adapts
        /// onto D50.
        white: [f32; 3],
        /// The diffuse black point, as CIE 1931 XYZ. Read, and deliberately not applied —
        /// [`cie_to_srgb`] has the argument.
        black: [f32; 3],
        /// The `a` and `b` axis bounds, as `[a_min, a_max, b_min, b_max]`.
        range: [f32; 4],
    },
    /// One component in a calibrated grey space: ISO 32000-2 §8.6.5.2, Table 62.
    CalGray {
        /// The diffuse white point, as CIE 1931 XYZ.
        white: [f32; 3],
        /// The diffuse black point, as CIE 1931 XYZ. Read, and deliberately not applied —
        /// [`cie_to_srgb`] has the argument.
        black: [f32; 3],
        /// The exponent decoding `A` into luminance.
        gamma: f32,
    },
    /// Three components in a calibrated RGB space: ISO 32000-2 §8.6.5.3, Table 63.
    CalRgb {
        /// The diffuse white point, as CIE 1931 XYZ.
        white: [f32; 3],
        /// The diffuse black point, as CIE 1931 XYZ. Read, and deliberately not applied —
        /// [`cie_to_srgb`] has the argument.
        black: [f32; 3],
        /// The exponents decoding `A`, `B` and `C`.
        gamma: [f32; 3],
        /// `[XA YA ZA XB YB ZB XC YC ZC]` — the decoded components' XYZ contributions,
        /// stored in the specification's own order, which is one *column* per triple.
        matrix: [f32; 9],
    },
    /// One component, an index into a table of colours in a base space.
    Indexed {
        /// The space the table's entries are in.
        base: Box<ColourSpace>,
        /// The table, as consecutive component values in the base space.
        lookup: Vec<f32>,
        /// The largest valid index.
        high: usize,
    },
    /// One or more tint components, converted into the alternate colour space of ISO 32000-2
    /// §8.6.6.4 and §8.6.6.5.
    Separation {
        /// How many tint components the space takes.
        inputs: usize,
        /// The colourant each tint component names, in the order the space states them: a
        /// `Separation`'s one `name` or a `DeviceN`'s `names` array.
        ///
        /// Carried because a colourant is what a separation is *of*. §8.6.6.4 has the reader
        /// "determine whether the device has an available colourant corresponding to the name
        /// of the requested space", and under §10.8.3's simulated device the answer is yes for
        /// every name `pdf_model::colourants` gave a plane to — which [`Compositing::paint`]
        /// cannot tell without the name. ADR 1311.
        names: Arc<[Name]>,
        /// The space the tints are converted into.
        ///
        /// §8.6.6.5 calls it "the alternate colour space of that component", and which space
        /// that is depends on which component: the `alternateSpace` parameter where the whole
        /// space reverts through its tint transform, and Table 71's process `/ColorSpace`
        /// where every component is a process one — see [`Tints`].
        alternate: Box<ColourSpace>,
        /// What turns the tints into the alternate space's components.
        tints: Tints,
    },
    /// ISO 32000-2 §10.8.3's separation simulation of an `NChannel` space that has a spot
    /// colourant: each colourant rendered as its own separation and the separations combined
    /// by that clause's four steps, rather than the whole space reverting through one tint
    /// transform.
    ///
    /// Built only under [`Separations::Simulated`], which is a host's answer to §10.8.3's own
    /// condition; without it such a space is a [`Self::Separation`]. ADR 1229.
    Simulated {
        /// The space's `names` array, one entry per tint component, for the reason
        /// [`Self::Separation`] carries it.
        names: Arc<[Name]>,
        /// Table 71's process colour space, carrying every process component's tint at once,
        /// where the space has process components.
        ///
        /// One separation rather than one per process component, and that is §8.6.6.5's own
        /// sentence read closely: a component "not present on the output device shall use the
        /// alternate colour space of **that component**", and the alternate of every process
        /// component is the *same* space — Table 71's `/ColorSpace`. A colour space is what
        /// states what its own colourants do together, so evaluating the process components
        /// individually *into* it and reading it once is the per-component evaluation, which
        /// is [`Tints::Process`]'s reading one clause over (ADR 1103). Each spot colourant, by
        /// contrast, has an alternate space of its own, and §10.8.3 is what combines those.
        process: Option<Box<ColourSpace>>,
        /// One entry per component of the space's `names` array, in that order: the
        /// `Separation` colour space Table 70's `/Colorants` holds for that spot colourant —
        /// "the value shall be an array defining a Separation colour space for that colourant"
        /// — or `None` where the component is a process one, carried by `process`, or `/None`,
        /// which "shall never be painted on the page".
        spots: Vec<Option<ColourSpace>>,
    },
    /// The `/All` colourant of ISO 32000-2 §8.6.6.4, which marks every colourant at once.
    ///
    /// > When outputting to an additive device, such as a computer monitor, the subtractive
    /// > tint values of the All colourant shall be complemented by subtracting from 1 before
    /// > applying to all available colourants.
    ///
    /// So on this device a tint of `t` is the grey `1 − t` in all three components: full ink
    /// is black and no ink is white. It is not a `Separation` with a grey alternate, because
    /// the clause requires the alternate space and the tint transform to be *ignored* —
    /// whatever the file provides, and even where they are unreadable.
    AllColourants,
    /// A `Separation` or `DeviceN` space that marks nothing. ISO 32000-2 §8.6.6.4:
    ///
    /// > The special colourant name None shall not produce any visible output. Painting
    /// > operations in a Separation space with this colourant name shall have no effect on
    /// > the current page.
    ///
    /// §8.6.6.5 extends it to a `DeviceN` whose component names are *all* `None`, which
    /// "shall always discard its output … it shall never revert to the alternate colour
    /// space". A `DeviceN` with only *some* `None` components is an ordinary
    /// [`Self::Separation`] here, because the same clause says those components "shall be
    /// passed to the tint transformation function" once the space reverts — which on an
    /// additive device it always does.
    NoColourant {
        /// How many tint components the space takes, so that `sc`/`scn` still parse.
        inputs: usize,
    },
    /// A colour space defined by an embedded ICC profile.
    Icc {
        /// The parsed profile.
        ///
        /// Shared rather than owned, because a space is *cloned* wherever it is selected —
        /// [`Self::device_family`] answers with a copy of the output intent for every `k`
        /// and every `cs` naming a device family, and the interpreter keeps a copy in the
        /// graphics state — and a parsed press profile is hundreds of kilobytes of lookup
        /// table. Measured by ADR 1008 on 4000 `k` operators under `issue20513.pdf`'s
        /// 718 KB CMYK intent: 1.80 G instructions with the copy against 27.7 M for the same
        /// page with no intent, 95% of them in `memcpy`; a refcount makes the clone a pointer
        /// and the profile is parsed and held once (ADR 1008).
        profile: Arc<crate::icc::Profile>,
    },
    /// A pattern space, which carries no colour of its own.
    ///
    /// It may name an *underlying* space, and that is not decoration: an uncoloured
    /// tiling pattern is a stencil, and the colour poured through it is given in this
    /// space rather than in any of the pattern's own.
    Pattern {
        /// The space an uncoloured pattern's colour is given in, if one was named.
        base: Option<Box<ColourSpace>>,
    },
}

/// What turns a `Separation` or `DeviceN` space's tints into its alternate space's components
/// (ISO 32000-2 §8.6.6.4, §8.6.6.5).
///
/// Two routes, and the standard ranks them by which components the space has. The tint
/// transform is what §8.6.6.4 and §8.6.6.5 require of a processor that has none of the named
/// colourants — "PDF processors shall be able to approximate the colourants if they are not
/// available on the current output device, such as a display" — and it is the whole answer for
/// every space but one. The exception is §8.6.6.5's `NChannel` subtype, which asks for the
/// components to be taken apart:
///
/// > For NChannel colour spaces, the components shall be evaluated individually; that is, only
/// > the ones not present on the output device shall use the alternate colour space of that
/// > component.
///
/// On a display no component is present, so every one of them takes "the alternate colour
/// space of that component" — a spot colourant its own `/Colorants` `Separation`, and a
/// process component the process dictionary's `/ColorSpace`. **Combining the two is what the
/// clause does not state**, and NOTE 3 hands that to the processor: "PDF processors can use
/// their own blending algorithms for on-screen viewing and composite printing, rather than
/// being required to use a specified tint transformation function." Table 72's `/Solidities`,
/// `/PrintingOrder` and `/DotGain` parameterise such an algorithm — how far one ink covers the
/// ones beneath it, the order they are laid down in, and how each spreads — but they describe
/// inks on paper and the clause says outright that "PDF processors need not use this
/// information". So a space with a spot colourant keeps [`Self::Transform`], which is the
/// route both paragraphs agree on.
///
/// Where an `NChannel` space has **no** spot colourant the combination does not arise, the
/// clause is complete on its own terms, and [`Self::Process`] is that reading. ADR 1103.
#[derive(Debug, Clone)]
pub enum Tints {
    /// §8.6.6.4's and §8.6.6.5's `tintTransform`: `n` tints in, the alternate space's `m`
    /// components out.
    Transform(Box<Function>),
    /// §8.6.6.5's process dictionary, for an `NChannel` space whose every component is a
    /// process one: no function is evaluated, because the tints *are* the process space's
    /// components.
    ///
    /// One entry per component of the process `/ColorSpace`, in Table 71's own order —
    /// "[a]n array of component names that correspond, in order, to the components of the
    /// process colour space specified in ColorSpace" — saying which tint supplies it, or
    /// `None` where the `names` array omits that colourant. An omitted one is `0.0`, which
    /// is a value §8.6.4.4 defines rather than one chosen here: "0.0 shall denote the
    /// complete absence of a process colourant". [`ColourSpace::nchannel_process`] admits an
    /// omission only where the process space is the `DeviceCMYK` that sentence is about, and
    /// which §8.6.6.5 gives the permission to: "[f]or a CMYK colour space, a subset of the
    /// components may be present, and they may appear in any order in the names array."
    #[expect(
        clippy::doc_markdown,
        reason = "the comment quotes §8.6.6.5, Table 71 and §8.6.4.4 verbatim, and a \
                  quotation is not marked up"
    )]
    Process(Vec<Option<usize>>),
}

impl Tints {
    /// The alternate space's components for one colour in the space these tints belong to.
    #[must_use]
    pub fn eval(&self, values: &[f32]) -> Vec<f32> {
        match self {
            Self::Transform(transform) => transform.eval(values),
            Self::Process(from) => from
                .iter()
                .map(|tint| {
                    tint.and_then(|index| values.get(index).copied())
                        .unwrap_or(0.0)
                })
                .collect(),
        }
    }
}

/// Whether ISO 32000-2 §10.8.3's separation simulation is in force for a colour space being
/// read.
///
/// §10.8.3's condition is a *reader's* request, not anything a document states:
///
/// > If it is important for the colours of the display for a PDF, on a device that normally
/// > would not be used to produce separations, to more closely match those produced when
/// > using separations, then a simulation of the separation process can be performed for the
/// > output to the non-separation device.
///
/// So the choice belongs to the host, and [`Self::Alternate`] is the default because the
/// condition is not met until somebody says it is. §10.8.1 puts the decision in the same
/// place — "Whether separations are produced is up to the processing software" — and §10.8.2
/// states the default route as the expected one for a display: "Alternate colour spaces are
/// supplied for DeviceN and Separation colour spaces so that files prepared for generation of
/// separations can be displayed on other devices".
///
/// What the choice changes is which reading of §8.6.6.5 an `NChannel` space gets, and only
/// that: every other space is the same space under both. ADR 1229.
///
/// **What it costs when it is `Alternate`**, which is the answer almost every page gets: the
/// off path is the path that was here before, with one `Copy` comparison per `DeviceN` space
/// parsed and a two-word [`Reading`] travelling where a one-word intent did. Measured under
/// callgrind on `examples/callgrind_interpret`'s fifty interpretations of page 101 of the
/// standard, in one sitting against the same tree with this enum's route removed:
/// **1 253 064 426 instructions against 1 252 937 239, +0.010%** — about 2500 instructions a
/// page, and none of them per pixel.
#[expect(
    clippy::doc_markdown,
    reason = "the comment quotes §10.8.1, §10.8.2 and §10.8.3 verbatim, and a quotation is not \
              marked up"
)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Separations {
    /// §8.6.6.4's and §8.6.6.5's alternate colour space and tint transform, which is what
    /// those clauses require of a device with none of the named colourants.
    #[default]
    Alternate,
    /// §10.8.3's four steps: each colourant rendered as its own separation, the separations
    /// converted to flat XYZ against a white matte, multiply-blended, and the result
    /// converted to the device's space.
    Simulated,
}

/// What a colour space is read *under*: the page's output intent, the space §11.7.2 redefines
/// `DeviceCMYK` to be inside the group being painted, and whether §10.8.3's separation
/// simulation is in force.
///
/// The two travel together for [`Conversion`]'s reason one level down — a colour space is
/// parsed at four places in this tree (an operator's `cs`, an image's `/ColorSpace`, a
/// shading's, and a `/Default` standing in for a device family), and a parameter that only
/// some of them pass is a parameter some page silently loses. Bundling them makes adding a
/// third a change to this type rather than to seven signatures.
#[derive(Debug, Clone, Copy, Default)]
pub struct Reading<'a> {
    /// The space §14.11.5's `/DestOutputProfile` describes for this page, where the document
    /// states one this tree reads.
    intent: Option<&'a ColourSpace>,
    /// The `ICCBased` CMYK blending space of the nearest isolated group or page that states one,
    /// which `DeviceCMYK` is inside it. See [`Reading::within_cmyk_group`].
    cmyk_group: Option<&'a ColourSpace>,
    /// Whether §10.8.3's simulation is in force.
    separations: Separations,
}

impl<'a> Reading<'a> {
    /// A reading under `intent`, with no separation simulation.
    #[must_use]
    pub fn new(intent: Option<&'a ColourSpace>) -> Self {
        Self {
            intent,
            cmyk_group: None,
            separations: Separations::Alternate,
        }
    }

    /// The same reading inside a group whose blending colour space `DeviceCMYK` is redefined to,
    /// or outside every such group.
    ///
    /// ISO 32000-2 §11.7.2:
    ///
    /// > If an isolated transparency group or page has an ICCBased 'CMYK' colour space ,
    /// > DeviceCMYK shall be redefined within the transparency group to be the same as the
    /// > blending colour space and references to the process colourants Cyan , Magenta , Yellow
    /// > and Black are defined to be references to the corresponding colourants in the blending
    /// > colour space, even where the actual or simulated output device is not CMYK.
    ///
    /// `space` is that blending space, which `pdf_model::content` finds for the group being
    /// painted, nearest first. It outranks §14.11.5's intent — informational, "and PDF
    /// processors are free to disregard it" — and is outranked by §8.6.5.6's default, which the
    /// same clause names as what takes a device colour out of the device rules: "unless remapped
    /// to default CIE-based colour spaces". ADR 2012.
    #[must_use]
    pub fn within_cmyk_group(mut self, space: Option<&'a ColourSpace>) -> Self {
        self.cmyk_group = space;
        self
    }

    /// The space §11.7.2 redefines `DeviceCMYK` to be under this reading, if any.
    #[must_use]
    pub fn cmyk_group(&self) -> Option<&'a ColourSpace> {
        self.cmyk_group
    }

    /// The same reading with `separations` deciding §10.8.3.
    #[must_use]
    pub fn under_separations(mut self, separations: Separations) -> Self {
        self.separations = separations;
        self
    }

    /// The output intent this reading carries.
    #[must_use]
    pub fn output_intent(&self) -> Option<&'a ColourSpace> {
        self.intent
    }

    /// Whether §10.8.3's simulation is in force.
    #[must_use]
    pub fn separations(&self) -> Separations {
        self.separations
    }
}

#[cfg(test)]
#[expect(
    clippy::float_cmp,
    reason = "test code: these conversions are exact by definition, so an approximate \
              comparison would hide the very drift the test exists to catch"
)]
mod tests {
    use pdf_syntax::{Dictionary, Document, Name, Object};

    use pdf_render::Color;

    use super::{ColourSpace, GreyRoute, InkScale, RgbRoute, Tints};
    use crate::icc::Rendering;

    /// The assumed press's grid, for the tests that search against it directly.
    fn assumed() -> pdf_render::BlendingSpace {
        super::assumed_press().blending_space()
    }

    /// [`super::xyz_to_lab`] is [`super::lab_to_xyz`] run backwards.
    #[test]
    fn a_lab_round_trips_through_xyz() {
        for lightness in [0.0f32, 5.0, 50.0, 100.0] {
            for a in [-100.0f32, 0.0, 60.0] {
                for b in [-80.0f32, 0.0, 90.0] {
                    let back = super::xyz_to_lab(super::lab_to_xyz(lightness, a, b));
                    for (got, want) in back.iter().zip([lightness, a, b]) {
                        assert!(
                            (got - want).abs() < 0.05,
                            "L*a*b* ({lightness}, {a}, {b}) came back as {back:?}"
                        );
                    }
                }
            }
        }
    }

    /// [`super::srgb_to_xyz_d50`] is [`super::xyz_d50_to_srgb`] run backwards, to a tenth of
    /// a level over the cube, and D50 white is the display's white.
    #[test]
    #[expect(
        clippy::cast_precision_loss,
        reason = "a grid index below 125, exact in f32"
    )]
    fn srgb_to_xyz_is_the_inverse_of_xyz_to_srgb() {
        let white = super::xyz_d50_to_srgb(super::D50);
        for channel in [white.r, white.g, white.b] {
            assert!((channel - 1.0).abs() < 1e-3, "D50 is white: {white:?}");
        }
        for step in 0..125usize {
            let colour = Color::rgb(
                (step % 5) as f32 / 4.0,
                (step / 5 % 5) as f32 / 4.0,
                (step / 25) as f32 / 4.0,
            );
            let back = super::xyz_d50_to_srgb(super::cie::srgb_to_xyz_d50(colour));
            for (got, want) in [back.r, back.g, back.b]
                .iter()
                .zip([colour.r, colour.g, colour.b])
            {
                assert!(
                    (got - want).abs() * 255.0 < 0.1,
                    "{colour:?} came back as {back:?}"
                );
            }
        }
    }

    /// A press sampled from a bi-directional profile converts *in* through the profile's own
    /// "from CIE" table (§8.6.5.5, §10.3.1; ADR 0796), and one without keeps the search.
    ///
    /// The expected inks are the profile's own answer to the XYZ a colour states — for a
    /// device grey, sRGB's decoding of it (IEC 61966-2-1) through this module's one matrix;
    /// for a `Lab` colour, §8.6.5.4's XYZ directly, with no screen in between.
    #[test]
    fn a_bidirectional_profiles_press_converts_in_through_its_own_table() {
        let profile = crate::icc::Profile::parse(&crate::icc::fixtures::two_way_cmyk_profile())
            .expect("the fixture parses");
        let press = super::press_for_profile(&profile, Rendering::compensating()).expect("a press");
        assert!(press.converts_in_by_profile());

        let grey = ColourSpace::Rgb.to_cmyk(&[0.5, 0.5, 0.5], Rendering::compensating(), &press);
        let want = profile
            .to_device(
                super::cie::srgb_to_xyz_d50(Color::grey(0.5)),
                Rendering::compensating(),
            )
            .expect("a from-CIE table");
        assert_eq!(grey, want, "a device colour goes in through sRGB's XYZ");
        // And that answer is the table's rule on the compensated XYZ rather than anything the
        // search would find. Which compensation, derived: the fixture is a CMYK profile
        // carrying a "from CIE" table, so ISO/CD 18619 (2013) section 4.2.3 calls it
        // output-capable and takes the connection space's black through that table — which
        // this fixture answers with the three chromatic inks at full and no black ink, a
        // colour its own "to CIE" table then evaluates as paper white. A profile whose two
        // tables contradict each other that way is what the draft's clamp is for: L\* comes
        // out at 100 and is cut to 50, whose relative luminance is ((50 + 16) / 116)³ =
        // 0.184 19, so section 4.2.6's scale is 1 / (1 − 0.184 19) = 1.225 78 and its offset
        // is −0.225 78 of the white point. Undoing that on sRGB's 0.214 04 of D50 gives
        // (0.214 04 + 0.225 78) / 1.225 78 = 0.358 81.
        let level = (0.214_04 + 0.225_776) / 1.225_776;
        let stated = 1.0 - 0.964_2 * level * 32768.0 / 65535.0;
        assert!(
            (grey[0] - stated).abs() < 1e-3,
            "cyan is the table's own arithmetic: {} against {stated}",
            grey[0]
        );

        let lab = ColourSpace::Lab {
            white: super::D50,
            black: [0.0; 3],
            range: [-100.0, 100.0, -100.0, 100.0],
        };
        let from_lab = lab.to_cmyk(&[50.0, 20.0, -30.0], Rendering::compensating(), &press);
        let want = profile
            .to_device(
                super::lab_to_xyz(50.0, 20.0, -30.0),
                Rendering::compensating(),
            )
            .expect("a from-CIE table");
        assert_eq!(
            from_lab, want,
            "a CIE-based colour goes in from its own XYZ"
        );

        let one_way = crate::icc::Profile::parse(&crate::icc::fixtures::one_way_cmyk_profile())
            .expect("the fixture parses");
        let press = super::press_for_profile(&one_way, Rendering::compensating()).expect("a press");
        assert!(
            !press.converts_in_by_profile(),
            "a profile without the table keeps the right inverse of its grid"
        );
    }

    /// A space's initial colour is the one ISO 32000-2 §8.6.8 gives it, which is often not
    /// black.
    ///
    /// The cases below are the ones a reader would otherwise assume are black: `DeviceCMYK`
    /// starts at `[0 0 0 1]`, which is black through the K channel rather than through
    /// zeroes, an `Indexed` space starts at whatever its table's entry 0 holds, and a
    /// `Pattern` space paints nothing at all until `scn` names a pattern. A `Separation`
    /// starts at *full* ink rather than none, which is the fifth case and needs a tint
    /// transform to build; `initial_colour` carries the clause's sentence for it.
    #[test]
    fn a_space_starts_at_the_colour_the_clause_gives_it() {
        assert_eq!(ColourSpace::Gray.initial_colour(), vec![0.0]);
        assert_eq!(ColourSpace::Rgb.initial_colour(), vec![0.0, 0.0, 0.0]);
        assert_eq!(ColourSpace::Cmyk.initial_colour(), vec![0.0, 0.0, 0.0, 1.0]);
        assert_eq!(
            ColourSpace::Indexed {
                base: Box::new(ColourSpace::Rgb),
                lookup: vec![1.0, 0.0, 0.0],
                high: 0,
            }
            .initial_colour(),
            vec![0.0],
            "an Indexed space starts at entry 0, whatever colour that is"
        );
        // A pattern paints nothing until `scn` names one, so it has no components at all.
        assert!(
            ColourSpace::Pattern { base: None }
                .initial_colour()
                .is_empty()
        );
    }

    /// §8.6.5.1's initial colour is held to the space's range — "unless the range of valid
    /// values for a given component does not include 0.0, in which case the nearest valid value
    /// shall be substituted" — and the range is the one the conversion uses, however the file
    /// wrote it.
    ///
    /// A `Lab` `/Range` stated backwards panicked here under `f32::clamp`, on a `cs` alone,
    /// while the conversion beside it took the bounds in either order; an `ICCBased` space's
    /// Table 65 `/Range` is taken over the profile's (ADR 1098) and was not consulted at all.
    #[test]
    fn an_initial_colour_is_held_to_the_range_however_it_is_written() {
        let lab = |range: [f32; 4]| ColourSpace::Lab {
            white: super::D50,
            black: [0.0; 3],
            range,
        };
        assert_eq!(
            lab([-128.0, 127.0, -128.0, 127.0]).initial_colour(),
            vec![0.0; 3],
            "zero inside the range is the initial colour"
        );
        assert_eq!(
            lab([10.0, 50.0, -100.0, -20.0]).initial_colour(),
            vec![0.0, 10.0, -20.0],
            "zero outside the range is its nearest bound"
        );
        assert_eq!(
            lab([50.0, 10.0, -20.0, -100.0]).initial_colour(),
            vec![0.0, 10.0, -20.0],
            "a range written backwards spans the same interval, as the conversion reads it"
        );
        let backwards = lab([50.0, 10.0, -20.0, -100.0]);
        assert_eq!(
            backwards.to_rgb(&backwards.initial_colour()),
            lab([10.0, 50.0, -100.0, -20.0]).to_rgb(&[0.0, 10.0, -20.0]),
            "and it draws as the forward range draws"
        );

        let bytes: &[u8] = include_bytes!("../../../data/icc/sRGB2014.icc");
        let profile = crate::icc::Profile::parse(bytes).expect("the shipped profile parses");
        let restated = ColourSpace::Icc {
            profile: std::sync::Arc::new(profile.with_range(&[
                (0.25, 1.0),
                (0.0, 1.0),
                (-1.0, -0.5),
            ])),
        };
        assert_eq!(restated.initial_colour(), vec![0.25, 0.0, -0.5]);
    }

    /// §8.6.8's initial colour is the *selected* family's: a `/DeviceCMYK cs` starts at
    /// `[0.0 0.0 0.0 1.0]` whether a `/DefaultCMYK` or an output intent stands in for the family,
    /// passed unchanged as §8.6.5.6 passes every value of the family (ADR 1755).
    ///
    /// The press is `one_way_cmyk_profile`, whose paper is D50 and whose black ink alone is a
    /// tenth of it, so the zeros an `ICCBased` space's own initial colour would be draw the
    /// paper — the whole range of 255 from the black the family starts at.
    #[test]
    fn a_device_family_starts_at_its_own_initial_colour_whatever_stands_in_for_it() {
        use std::fmt::Write as _;

        let bytes = crate::icc::fixtures::one_way_cmyk_profile();
        let profile = crate::icc::Profile::parse(&bytes).expect("the fixture parses");
        let press = ColourSpace::Icc {
            profile: std::sync::Arc::new(profile),
        };
        let mut hex = String::new();
        for byte in &bytes {
            let _ = write!(hex, "{byte:02X}");
        }
        let source = format!(
            "%PDF-1.7\n1 0 obj\n<< /N 4 /Filter /ASCIIHexDecode /Length {} >>\nstream\n{hex}>\n\
             endstream\nendobj\n2 0 obj\n<< /ColorSpace << /DefaultCMYK [/ICCBased 1 0 R] \
             /CS0 [/DeviceCMYK] /CS1 /DeviceCMYK /CS2 [/ICCBased 1 0 R] >> >>\nendobj\n\
             trailer\n<< /Size 3 >>\n",
            hex.len().saturating_add(1)
        );
        let document = Document::open(source.into_bytes()).expect("a document");
        let resources = document
            .get(pdf_syntax::ObjectId {
                number: 2,
                generation: 0,
            })
            .as_dict()
            .cloned()
            .expect("a resource dictionary");
        let selection = |name: &str| Object::Name(Name::new(name.as_bytes().to_vec()));
        let level = |colour: Color| {
            let [r, g, b] = [colour.r, colour.g, colour.b].map(|c| (c * 255.0).round());
            (r, g, b)
        };

        // §14.11.5's output intent standing in for the family.
        let device = selection("DeviceCMYK");
        let under_intent = ColourSpace::parse_under(
            &document,
            &device,
            &Dictionary::new(),
            super::Reading::new(Some(&press)),
        )
        .expect("the family parses");
        assert!(matches!(under_intent, ColourSpace::Icc { .. }));
        let initial = under_intent.initial_colour_of(&document, &device, &Dictionary::new());
        assert_eq!(initial, vec![0.0, 0.0, 0.0, 1.0]);
        let started = level(under_intent.to_rgb(&initial));
        let paper = level(under_intent.to_rgb(&under_intent.initial_colour()));
        assert_eq!(
            paper,
            (255.0, 255.0, 255.0),
            "no ink is the profile's paper"
        );
        assert!(
            started.0 < 64.0 && started.1 < 64.0 && started.2 < 64.0,
            "the family's black is the press's black ink, got {started:?}"
        );

        // §8.6.5.6's default, by every route a device family can be selected.
        for name in ["DeviceCMYK", "CS0", "CS1"] {
            let object = selection(name);
            let space = ColourSpace::parse(&document, &object, &resources).expect("parses");
            assert!(
                matches!(space, ColourSpace::Icc { .. }),
                "{name}: the default"
            );
            assert_eq!(
                space.initial_colour_of(&document, &object, &resources),
                vec![0.0, 0.0, 0.0, 1.0],
                "{name}: the family's initial colour, passed unchanged"
            );
        }

        // A profile selected as itself is its own family, and starts at its own zeros.
        let own = selection("CS2");
        let space = ColourSpace::parse(&document, &own, &resources).expect("parses");
        assert_eq!(
            space.initial_colour_of(&document, &own, &resources),
            vec![0.0; 4]
        );
    }

    /// ISO 32000-2 §10.8.3's multiply is taken in the matte's own white, so white is its
    /// identity.
    ///
    /// Table 136's Multiply is `B(cb, cs) = cb × cs`, and its NOTE 3 states what the unit is:
    /// "multiplying any colour with black produces black while multiplying with white leaves
    /// the original colour unchanged". Step b) makes the matte "all white", so a separation
    /// carrying no colourant is that white and must fall out of step c) exactly. Built here
    /// rather than in `colour_paths.rs` because it is the *arithmetic* that is under test:
    /// three separations, two of them white, against the one that is not, with no document and
    /// no rasteriser between.
    ///
    /// The planted defect it is calibrated against is the one this reading is about —
    /// multiplying the flat XYZ values without dividing by the matte's white. That scales the
    /// answer by D50 once per extra separation, which here is `0.9642 × 0.9642 = 0.9297` of X
    /// against `1.0` of Y, so a grey stops being grey and the assertion below fails on every
    /// axis at once.
    #[test]
    fn the_white_matte_is_the_identity_of_the_multiply() {
        // A separation carrying no colourant at all: every component of its subtractive
        // alternate is §8.6.4.4's "complete absence of a process colourant", which is the
        // matte's white whatever tint it is handed.
        let white = ColourSpace::Separation {
            inputs: 1,
            names: std::sync::Arc::from([Name::new(b"Paper".to_vec())]),
            alternate: Box::new(ColourSpace::Cmyk),
            tints: Tints::Process(vec![None, None, None, None]),
        };
        // And one that carries black, so that a tint of 0.0 is the lightest colour it can
        // achieve and 1.0 the darkest, which is §8.6.6.4's own convention.
        let ink = ColourSpace::Separation {
            inputs: 1,
            names: std::sync::Arc::from([Name::new(b"Black".to_vec())]),
            alternate: Box::new(ColourSpace::Cmyk),
            tints: Tints::Process(vec![None, None, None, Some(0)]),
        };
        let alone = ColourSpace::Simulated {
            names: std::sync::Arc::from([Name::new(b"Ink".to_vec())]),
            process: None,
            spots: vec![Some(ink.clone())],
        };
        let with_white = ColourSpace::Simulated {
            names: [b"Ink".as_slice(), b"Paper", b"Card"]
                .map(|name| Name::new(name.to_vec()))
                .into(),
            process: None,
            spots: vec![Some(ink), Some(white.clone()), Some(white)],
        };
        // Compared as the eight bits a pixel holds, because the two routes differ by the
        // rounding of one XYZ round trip — 1.3e-4 of a component, a fortieth of a level —
        // while the defect this is calibrated against was planted and measured: the matte
        // comes out (245, 255, 207) rather than white, 48 levels of blue.
        let levels = |colour: Color| {
            [
                (colour.r * 255.0).round(),
                (colour.g * 255.0).round(),
                (colour.b * 255.0).round(),
            ]
        };
        for tint in [0.0_f32, 0.25, 0.5, 1.0] {
            assert_eq!(
                levels(with_white.to_rgb(&[tint, 0.0, 0.0])),
                levels(alone.to_rgb(&[tint])),
                "two white separations changed the colour at tint {tint}"
            );
        }
    }

    /// §8.6.5.1's `CalCMYK`: a family the standard withdrew and still says what to do with.
    ///
    /// > A PDF reader shall ignore CalCMYK colour space attributes and render colours
    /// > specified in this family as if they had been specified using DeviceCMYK
    ///
    /// So the dictionary beside the name is not a parse failure and not an approximation —
    /// it is *ignored*, by the clause's own instruction, and what is left is `DeviceCMYK`.
    /// No corpus document writes one, which is why this rule waited for a reading of the
    /// family rather than for a page; without it such a file reported an unsupported colour
    /// space, which is a refusal where the standard states an answer.
    #[test]
    fn a_calcmyk_space_is_device_cmyk_and_its_dictionary_is_ignored() {
        let document = Document::open(
            b"%PDF-1.7\n1 0 obj\n<< /WhitePoint [0.9505 1 1.089] >>\nendobj\n\
              trailer\n<< /Size 2 /Root 1 0 R >>\n"
                .to_vec(),
        )
        .expect("a document with one dictionary in it");
        let space = ColourSpace::parse(
            &document,
            &Object::Array(vec![
                Object::Name(Name::new(b"CalCMYK".as_slice())),
                Object::Dictionary(Dictionary::new()),
            ]),
            &Dictionary::new(),
        )
        .expect("§8.6.5.1 states what a CalCMYK space is");
        assert_eq!(space.components(), 4);
        assert_eq!(
            space.to_rgb(&[0.0, 0.0, 0.0, 1.0]),
            ColourSpace::Cmyk.to_rgb(&[0.0, 0.0, 0.0, 1.0]),
            "a CalCMYK colour is a DeviceCMYK colour"
        );
    }

    /// `DeviceGray` and `DeviceRGB` pass straight through, with no gamma applied.
    ///
    /// ISO 32000-2 §8.6.4.3 defines a `DeviceRGB` component as the intensity of one of the
    /// device's own primaries, and §8.6.5.7 NOTE 3 says PDF carries nothing describing that
    /// device's calibration. Applying any curve here would be asserting a calibration the
    /// specification says is not in the file; the identity is what "device colour" means.
    ///
    /// So `0.5 g` is 128, not the 188 a linear-to-sRGB encoding would give.
    #[test]
    fn grey_and_rgb_pass_through_unchanged() {
        assert_eq!(ColourSpace::Gray.to_rgb(&[0.5]).r, 0.5);
        let rgb = ColourSpace::Rgb.to_rgb(&[0.1, 0.2, 0.3]);
        assert_eq!((rgb.r, rgb.g, rgb.b), (0.1, 0.2, 0.3));
    }

    /// The table a backend is handed is this crate's own `DeviceCMYK` conversion.
    ///
    /// §11.4.7 converts a page composited in its blending space to the device's at the end,
    /// and that happens in a backend, so the conversion travels as sixteen corners. This is
    /// what keeps the two from becoming a second `DeviceCMYK` conversion — the failure this
    /// module's own header records having had once — by checking them against each other over
    /// the cube rather than at its corners, where a wrong interpolation would still agree.
    #[test]
    fn a_backends_table_is_this_crates_own_conversion() {
        let table = super::device_cmyk_blending_space();
        let mut worst = 0.0f32;
        let steps = [0.0, 0.125, 0.375, 0.5, 0.625, 0.875, 1.0];
        for c in steps {
            for m in steps {
                for y in steps {
                    for k in steps {
                        let ours = ColourSpace::Cmyk.to_rgb(&[c, m, y, k]);
                        let theirs = table.convert(c, m, y, k);
                        for (mine, other) in [ours.r, ours.g, ours.b].into_iter().zip(theirs) {
                            worst = worst.max((mine - other).abs());
                        }
                    }
                }
            }
        }
        assert!(
            worst < 1e-6,
            "the display list's table and `cmyk` are one conversion: worst gap {worst}"
        );
    }

    /// §10.4.2.4's conversion into `DeviceCMYK`, and the two things it round-trips through.
    ///
    /// The clause's own two steps with the nominal `BG(k) = k` and `UCR(k) = k`. Three claims,
    /// each of which decides something ADRs 0262 and 0263 rest on: §10.4.2.5's classic
    /// conversion back is the identity on the result, which is why the standard's own pair
    /// costs an opaque mark nothing; this tree's ink cube is **not**, which is why the two
    /// branches cannot be composed; and the gap is the visible one ADR 0262's picture has —
    /// `1 0 0 rg` coming back as the red process inks print.
    #[test]
    fn the_conversion_into_ink_round_trips_through_the_classic_formula_and_not_the_cube() {
        // §10.4.2.4 on pure red: c = 0, m = 1, y = 1, k = min = 0.
        let red = super::ink_search::rgb_to_cmyk(Color::rgb(1.0, 0.0, 0.0));
        assert_eq!(red, [0.0, 1.0, 1.0, 0.0]);
        // §10.4.2.5 back: red = 1 − min(1, cyan + black) = 1, green = 1 − 1 = 0, blue = 0.
        let classic = |components: [f32; 4]| {
            [
                1.0 - (components[0] + components[3]).min(1.0),
                1.0 - (components[1] + components[3]).min(1.0),
                1.0 - (components[2] + components[3]).min(1.0),
            ]
        };
        assert_eq!(
            classic(red),
            [1.0, 0.0, 0.0],
            "the standard's pair is exact"
        );

        // §10.4.2.3's grey, which the same conversion has to produce: c = m = y = 0.
        let grey = super::ink_search::rgb_to_cmyk(Color::grey(0.25));
        assert_eq!(grey, [0.0, 0.0, 0.0, 0.75]);
        assert_eq!(classic(grey), [0.25, 0.25, 0.25]);

        // And the cube is a different answer, which is what ADR 0262 refused to ship: process
        // red rather than the red a monitor emits.
        let through_the_cube = ColourSpace::Cmyk.to_rgb(&red);
        assert!(
            (through_the_cube.r - 237.0 / 255.0).abs() < 1e-6
                && (through_the_cube.g - 28.0 / 255.0).abs() < 1e-6,
            "pure red taken into ink by §10.4.2.4 and back through the cube is the red \
             corner, {through_the_cube:?}"
        );
    }

    /// [`super::rgb_to_ink`] is a right inverse of [`super::cmyk`], which is ADR 0263's claim.
    ///
    /// Three populations, and each says something the others cannot. **The cube's own image**
    /// is the claim itself: a colour the assumed inks can make comes back to itself, so an
    /// opaque mark on a page composited in ink is the colour the file states. **Named
    /// colours** are what a page is mostly made of — paper, black text, greys — and they are
    /// the ones ADR 0262's picture moved. And **the sRGB primaries** are the honest cost:
    /// they are outside the press's gamut and come back on its boundary.
    ///
    /// The old route is put back in the same test, which is what makes the numbers a
    /// difference this round made: §10.4.2.4's separation of the same colours, taken back
    /// through the cube, is 35 levels out on black and 36 on red.
    #[test]
    fn the_conversion_into_ink_is_a_right_inverse_of_the_conversion_out() {
        let round_trip = |colour: Color| {
            let ink = super::ink_search::rgb_to_ink(&super::assumed_press(), colour);
            super::device_cmyk::cmyk(ink[0], ink[1], ink[2], ink[3])
        };
        let gap = |a: Color, b: Color| {
            255.0
                * (a.r - b.r)
                    .abs()
                    .max((a.g - b.g).abs())
                    .max((a.b - b.b).abs())
        };

        // Every colour the cube can make, sampled over its own domain rather than over sRGB:
        // these have a preimage by construction, so the inverse has to find one.
        let mut worst: f32 = 0.0;
        for index in 0..6usize * 6 * 6 * 6 {
            let at = |axis: u32| {
                #[expect(clippy::cast_precision_loss, reason = "a digit in 0..6, exact in f32")]
                let digit = (index / 6usize.pow(axis) % 6) as f32;
                digit / 5.0
            };
            let made = super::device_cmyk::cmyk(at(0), at(1), at(2), at(3));
            worst = worst.max(gap(round_trip(made), made));
        }
        assert!(
            worst < 3.4,
            "a colour the inks can make comes back to itself: worst {worst} of 255"
        );
        // The bound is where it is because that is what the construction reaches, not because
        // 3.4 is a target. The colours that fall short of half a level are dark saturated inks
        // whose feasible band of black is narrower than `INK_LADDER`'s rungs, and the second
        // assertion is the one that prices `ink_table`: searching every colour instead of
        // reading the table reaches **3.12** where the table reaches 3.32, so a grid of 17
        // costs two tenths of a level at the gamut's own boundary and nothing anywhere else.
        assert!(
            worst > 3.3,
            "and the bound is the construction's own reach rather than a round number: {worst}"
        );
        let mut searched: f32 = 0.0;
        for index in 0..6usize * 6 * 6 * 6 {
            let at = |axis: u32| {
                #[expect(clippy::cast_precision_loss, reason = "a digit in 0..6, exact in f32")]
                let digit = (index / 6usize.pow(axis) % 6) as f32;
                digit / 5.0
            };
            let made = super::device_cmyk::cmyk(at(0), at(1), at(2), at(3));
            let ink = super::ink_search::search_ink(&assumed(), [made.r, made.g, made.b]);
            searched = searched.max(gap(
                super::device_cmyk::cmyk(ink[0], ink[1], ink[2], ink[3]),
                made,
            ));
        }
        assert!(
            (3.1..3.2).contains(&searched),
            "the search alone reaches 3.12 of 255 on the same colours: {searched}"
        );

        for (name, colour) in [
            ("paper", Color::rgb(1.0, 1.0, 1.0)),
            ("black", Color::rgb(0.0, 0.0, 0.0)),
            ("half grey", Color::grey(0.5)),
            ("dark grey", Color::grey(0.125)),
            ("light grey", Color::grey(0.9)),
        ] {
            let gap = gap(round_trip(colour), colour);
            assert!(gap < 1.0, "{name} comes back to itself: {gap} of 255");
        }

        // §10.4.2.4 on the same two, back through the cube: `0 g` becomes `#231F20`.
        let classic = |colour: Color| {
            let ink = super::ink_search::rgb_to_cmyk(colour);
            super::device_cmyk::cmyk(ink[0], ink[1], ink[2], ink[3])
        };
        let black = classic(Color::rgb(0.0, 0.0, 0.0));
        assert!(
            (gap(black, Color::rgb(0.0, 0.0, 0.0)) - 35.0).abs() < 0.5,
            "the route ADR 0262 refused puts black 35 levels out: {black:?}"
        );

        // And the cost of the choice, which is a gamut and not an error: sRGB's primaries are
        // outside the press's, so they land on its boundary — pure red at the red corner.
        let red = round_trip(Color::rgb(1.0, 0.0, 0.0));
        assert!(
            (red.r - 237.0 / 255.0).abs() < 2.0 / 255.0
                && (red.g - 28.0 / 255.0).abs() < 2.0 / 255.0,
            "pure red is outside the inks' gamut and lands on the red corner: {red:?}"
        );
    }

    /// §10.4.2.3's ink and grey level, on the four colours the clause's own formula fixes.
    ///
    /// `gray = 1.0 − min(1.0, 0.3 × cyan + 0.59 × magenta + 0.11 × yellow + black)`, which is
    /// what §11.5.3's EXAMPLE 2 prints for a `/Luminosity` soft mask. The last row is the one
    /// that needs the `min` and the one a rendered channel cannot hold: registration black
    /// weighs 2.0 and every mixture of it is decided before the clamp, not after.
    #[test]
    fn cmyk_ink_and_grey_are_the_clauses_own_arithmetic() {
        let ink = |c, m, y, k| ColourSpace::Cmyk.ink(&[c, m, y, k]);
        let grey = |c, m, y, k| ColourSpace::Cmyk.luminosity(&[c, m, y, k]);

        assert_eq!(ink(0.0, 0.0, 0.0, 0.0), 0.0, "no ink at all");
        assert_eq!(grey(0.0, 0.0, 0.0, 0.0), 1.0, "and so, white");
        assert_eq!(ink(0.0, 0.0, 0.0, 1.0), 1.0, "process black");
        assert_eq!(grey(0.0, 0.0, 0.0, 1.0), 0.0, "which masks everything away");
        assert!(
            (ink(1.0, 0.0, 0.0, 0.0) - 0.3).abs() < 1e-6,
            "cyan alone weighs 0.3"
        );
        assert!(
            (grey(1.0, 0.0, 0.0, 0.0) - 0.7).abs() < 1e-6,
            "and leaves 0.7"
        );
        assert_eq!(ink(1.0, 1.0, 1.0, 1.0), 2.0, "registration black");
        assert_eq!(grey(1.0, 1.0, 1.0, 1.0), 0.0, "clamped by the clause's min");
    }

    /// An `NChannel` space's process components are the group's own, with no function between.
    ///
    /// §8.6.6.5 requires the components of such a space to be taken apart — "[f]or NChannel
    /// colour spaces, the components shall be evaluated individually; that is, only the ones
    /// not present on the output device shall use the alternate colour space of that
    /// component" — and says what a process component's value then is: "[t]he values
    /// associated with the process components shall be stored in their natural form … since
    /// they shall be interpreted directly as process values by consumers making use of the
    /// process dictionary." Inside a page or group §11.4.7 composites in four components, the
    /// process space *is* what is being composited in, so "directly" is literal: the tints
    /// reach [`Compositing::Subtractive`]'s four components unconverted, and a component the
    /// `names` array omits is §8.6.4.4's "complete absence of a process colourant". ADR 1103
    /// built the route; ADR 1193 is why it stops where it does.
    #[expect(
        clippy::doc_markdown,
        reason = "the comment quotes §8.6.6.5 and §8.6.4.4 verbatim, and a quotation is not \
                  marked up"
    )]
    #[test]
    fn an_nchannel_process_space_is_the_four_components_a_cmyk_group_composites() {
        let press = super::assumed_press();
        let all_four = ColourSpace::Separation {
            inputs: 4,
            names: [b"Cyan".as_slice(), b"Magenta", b"Yellow", b"Black"]
                .map(|name| Name::new(name.to_vec()))
                .into(),
            alternate: Box::new(ColourSpace::Cmyk),
            tints: Tints::Process(vec![Some(0), Some(1), Some(2), Some(3)]),
        };
        assert_eq!(
            all_four.to_cmyk(&[0.1, 0.2, 0.3, 0.4], Rendering::compensating(), &press),
            [0.1, 0.2, 0.3, 0.4],
            "the tints are the group's components, in Table 71's order"
        );

        // `/Cyan` and `/Black` named, `/Magenta` and `/Yellow` left out, which §8.6.6.5 admits
        // for a CMYK process space alone.
        let subset = ColourSpace::Separation {
            inputs: 2,
            names: [b"Cyan".as_slice(), b"Black"]
                .map(|name| Name::new(name.to_vec()))
                .into(),
            alternate: Box::new(ColourSpace::Cmyk),
            tints: Tints::Process(vec![Some(0), None, None, Some(1)]),
        };
        assert_eq!(
            subset.to_cmyk(&[0.6, 0.9], Rendering::compensating(), &press),
            [0.6, 0.0, 0.0, 0.9],
            "an unnamed process component is absent rather than invented"
        );
    }

    /// §10.4.2.1's ranking, priced where §10.4.2.3 states a conversion this tree departs from.
    ///
    /// The clause is explicit — "the CMYK colour value equivalent to a specific gray level
    /// shall be" cyan, magenta and yellow at 0.0 with `black = 1.0 − grey` — and §10.4.2.1
    /// ranks that whole family below §10.3's route for a processor that follows profiles,
    /// which is the branch this tree is on. A grey painted into a `DeviceCMYK` group is
    /// therefore separated by [`super::rgb_to_ink`]'s search against the press instead, and
    /// this test is that decision's price rather than a check of it (ADR 1194).
    ///
    /// Two numbers, and they measure different things. The **components** are far apart: a
    /// tenth-grey carries 0.60 of cyan here where the clause states 0.0, and every grey but
    /// black and white carries chromatic ink the clause puts at zero — which is what §11.3.4's
    /// per-component blending inside such a group reads. The **pixel** is not apart at all:
    /// [`super::rgb_to_ink`] is a right inverse of the press's cube (ADR 0263), so an opaque
    /// grey mark composited in ink and converted back out is the grey the file stated, to
    /// inside one level of an eight-bit channel.
    #[test]
    fn a_grey_is_separated_by_the_press_rather_than_by_the_clauses_nominal_black() {
        let press = super::assumed_press();
        let mut worst_component = 0.0f32;
        let mut worst_pixel = 0.0f32;
        for step in 0..=20u8 {
            let level = f32::from(step) / 20.0;
            let ours = ColourSpace::Gray.to_cmyk(&[level], Rendering::compensating(), &press);
            // §10.4.2.3: "cyan = 0.0 magenta = 0.0 yellow = 0.0 black = 1.0 - grey".
            let clause = [0.0, 0.0, 0.0, 1.0 - level];
            for (ink, stated) in ours.iter().zip(clause) {
                worst_component = worst_component.max((ink - stated).abs());
            }
            let back = ColourSpace::Cmyk.to_rgb(&ours);
            for channel in [back.r, back.g, back.b] {
                worst_pixel = worst_pixel.max((channel - level).abs());
            }
        }
        assert!(
            worst_component > 0.5,
            "the two separations of a grey differ by {worst_component} in a component"
        );
        assert!(
            worst_pixel < 1.0 / 255.0,
            "and the pixel is the grey the file stated, worst {worst_pixel}"
        );
    }

    /// A grey and an RGB colour weigh the same ink whichever device space states them.
    ///
    /// This is the result that lets a mask group be painted in one number without converting
    /// each colour into the group's space first, and it is arithmetic rather than a
    /// convention. §10.4.2.3 sends a grey `g` to `(0, 0, 0, 1 − g)`, whose ink is `1 − g`.
    /// §10.4.2.4 sends an RGB colour to `c = 1 − red`, `m = 1 − green`, `y = 1 − blue`,
    /// `k = min(c, m, y)`, with the black generated and then removed from the other three —
    /// and because §10.4.2.3's three weights sum to 1.0, every `k` term cancels:
    /// `0.3(c − k) + 0.59(m − k) + 0.11(y − k) + k = 0.3c + 0.59m + 0.11y`. So the ink of an
    /// RGB colour is `1 − (0.3 R + 0.59 G + 0.11 B)` whatever the black-generation function
    /// did, and §10.4.2.2 gives that same grey level directly.
    #[test]
    fn a_grey_and_an_rgb_colour_weigh_the_same_ink_in_either_device_space() {
        for level in [0.0_f32, 0.25, 0.5, 1.0] {
            let grey = ColourSpace::Gray.luminosity(&[level]);
            assert!(
                (grey - level).abs() < 1e-6,
                "a grey of {level} is its own luminosity, and got {grey}"
            );
            let through_cmyk = ColourSpace::Cmyk.luminosity(&[0.0, 0.0, 0.0, 1.0 - level]);
            assert!(
                (through_cmyk - level).abs() < 1e-6,
                "and the same after §10.4.2.3 sends it to (0, 0, 0, 1 − g)"
            );
        }

        let rgb = ColourSpace::Rgb.luminosity(&[0.2, 0.7, 0.4]);
        let expected = 0.3_f32.mul_add(0.2, 0.59_f32.mul_add(0.7, 0.11 * 0.4));
        assert!(
            (rgb - expected).abs() < 1e-6,
            "§10.4.2.2's own formula, and got {rgb}"
        );
        let (c, m, y) = (1.0 - 0.2_f32, 1.0 - 0.7_f32, 1.0 - 0.4_f32);
        let k = c.min(m).min(y);
        let through_cmyk = ColourSpace::Cmyk.luminosity(&[c - k, m - k, y - k, k]);
        assert!(
            (through_cmyk - expected).abs() < 1e-6,
            "and §10.4.2.4's black generation cancels out of it, leaving {through_cmyk}"
        );
    }

    /// A `DeviceCMYK` colour's luminosity is *not* the grey level of the RGB it renders as.
    ///
    /// The one space where the two routes differ, and therefore the reason a raster inside a
    /// mask group has to be produced in the group's own quantity rather than converted after
    /// the fact (ADR 0220). Process black is the case a reader can check: `CMYK_CORNERS` puts
    /// it at `(35, 31, 32)`, whose §10.4.2.2 grey level is 32 of 255, against the clause's 0.
    /// [`GreyRoute::component_with_grey`] is the inverse of the curve it was sampled from.
    ///
    /// §11.6.6's conversion into a calibrated grey space is this tree's inverse of §8.6.5.2's
    /// conversion out, on the greys — so a component sent out through the curve and brought
    /// back by the search is itself, to the precision of the samples, over the whole range
    /// and for a gamma that makes the curve visibly non-linear. And a `DeviceGray` value
    /// painted into the space comes back out as itself, which is the property that lets an
    /// opaque device mark survive the group (ADR 0792).
    #[test]
    fn a_calibrated_greys_route_in_is_the_inverse_of_its_curve_out() {
        let space = ColourSpace::CalGray {
            white: [0.9505, 1.0, 1.089],
            black: [0.0; 3],
            gamma: 2.2,
        };
        let route = GreyRoute::of(&space).expect("a gamma curve has an inverse");
        assert_eq!(route.identity(), space.grey_identity().expect("a CalGray"));
        for step in 0..=255u8 {
            let component = f32::from(step) / 255.0;
            let out = route.curve().convert(component);
            let grey = Color::rgb(out[0], out[1], out[2]).grey_level();
            let back = route.component_with_grey(grey);
            assert!(
                (back - component).abs() <= 1.0 / 255.0,
                "component {component} leaves as grey {grey} and comes back as {back}"
            );
            // The same component, stated in the space's own terms, is not converted at all.
            let own = route.component_of(&space, &[component]);
            assert!((own - component).abs() < 1e-6, "{own} for {component}");
            // A device grey of the same level goes in through the inverse and out through
            // the curve as itself.
            let device = route.component_of(&ColourSpace::Gray, &[component]);
            let shown = route.curve().convert(device);
            assert!(
                (shown[0] - component).abs() <= 1.5 / 255.0,
                "device grey {component} shows as {} through the group",
                shown[0]
            );
        }
        assert!(
            GreyRoute::of(&ColourSpace::Gray).is_none(),
            "device grey needs no route"
        );
        assert!(
            GreyRoute::of(&ColourSpace::Rgb).is_none(),
            "and three components are not one"
        );
    }

    #[test]
    fn a_cmyk_colours_luminosity_is_not_the_grey_of_its_pixel() {
        let process_black = [0.0, 0.0, 0.0, 1.0];
        let rendered = ColourSpace::Cmyk.to_rgb(&process_black).grey_level();
        assert_eq!(bytes(Color::grey(rendered)).0, 32);
        assert_eq!(ColourSpace::Cmyk.luminosity(&process_black), 0.0);

        // A grey and an RGB colour are the two that do survive, which is what makes the
        // scaled channel exact for them and is the other half of the same statement.
        for (space, values) in [
            (ColourSpace::Gray, vec![0.25_f32]),
            (ColourSpace::Rgb, vec![0.2, 0.7, 0.4]),
        ] {
            let through_the_pixel = space.to_rgb(&values).grey_level();
            assert!(
                (space.luminosity(&values) - through_the_pixel).abs() < 1e-6,
                "{space:?} keeps its luminosity through a raster"
            );
        }
    }

    /// A scaled channel carries §10.4.2.3's grey, with the clause's `min` after the mixing.
    ///
    /// [`InkScale::grey_of`] and [`InkScale::mask_value`] are inverse over the ink each scale
    /// admits, so a colour painted into a mask group and read straight back is the luminosity
    /// [`ColourSpace::luminosity`] gives it — the property the whole construction rests on.
    /// And the second half is the one that is *not* a round trip: registration black over a
    /// half-covered white mark is `1 − min(1, ½·0 + ½·2) = 0` at [`InkScale::Double`], where
    /// clamping each colour first gives 0.5.
    #[test]
    fn a_scaled_channel_carries_the_clauses_own_grey() {
        for scale in [InkScale::Unit, InkScale::Double] {
            for values in [
                vec![0.0_f32, 0.0, 0.0, 0.0],
                vec![0.0, 0.0, 0.0, 1.0],
                vec![1.0, 0.0, 0.0, 0.0],
                vec![1.0, 1.0, 1.0, 1.0],
            ] {
                let channel = scale.grey_of(&ColourSpace::Cmyk, &values);
                assert!(
                    (0.0..=1.0).contains(&channel),
                    "a rendered channel holds 0..=1, and {values:?} gave {channel}"
                );
                let read_back = scale.mask_value(channel);
                assert!(
                    (read_back - ColourSpace::Cmyk.luminosity(&values)).abs() < 1e-6,
                    "{scale:?} on {values:?} read back {read_back}"
                );
            }
        }

        // Half coverage of white artwork over registration black: the clause composites the
        // ink and clamps once, which `Double` can express and `Unit` cannot.
        let white = InkScale::Double.grey_of(&ColourSpace::Cmyk, &[0.0, 0.0, 0.0, 0.0]);
        let backdrop = InkScale::Double.grey_of(&ColourSpace::Cmyk, &[1.0, 1.0, 1.0, 1.0]);
        let half = 0.5_f32.mul_add(white, 0.5 * backdrop);
        assert!(
            InkScale::Double.mask_value(half).abs() < 1e-6,
            "1 − min(1, ½·0 + ½·2) is 0, and got {}",
            InkScale::Double.mask_value(half)
        );
        assert!(
            (InkScale::Unit.mask_value(0.5_f32.mul_add(1.0, 0.5 * 0.0)) - 0.5).abs() < 1e-6,
            "where clamping each colour first gives 0.5"
        );
    }

    /// The eight-bit sRGB a colour converts to, for comparing against measured output.
    fn bytes(colour: Color) -> (u8, u8, u8) {
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "clamped to 0.0..=1.0 before scaling, so the result is a valid byte"
        )]
        let byte = |value: f32| (value.clamp(0.0, 1.0) * 255.0).round() as u8;
        (byte(colour.r), byte(colour.g), byte(colour.b))
    }

    /// The pure inks must land on the published appearances of the process inks.
    ///
    /// §8.6.4.4 defines no conversion and **§10.4.2.5 defines one**, which §10.4.2.1 offers
    /// to "a less-capable PDF processor" as a "crude approximation". So this pins a
    /// deliberate choice between two ranked answers rather than a derivation — see
    /// `CMYK_CORNERS` for the choice and for the three sources that outrank it. What the
    /// test defends is that the choice stays made: these sixteen values are the whole of it,
    /// and a conversion that moves any of them has stopped assuming the press we said we
    /// were assuming.
    #[test]
    fn the_process_inks_are_the_published_ink_appearances() {
        let cmyk = |c, m, y, k| bytes(ColourSpace::Cmyk.to_rgb(&[c, m, y, k]));

        assert_eq!(cmyk(0.0, 0.0, 0.0, 0.0), (255, 255, 255), "paper");
        assert_eq!(cmyk(1.0, 0.0, 0.0, 0.0), (0, 173, 239), "process cyan");
        assert_eq!(cmyk(0.0, 1.0, 0.0, 0.0), (236, 0, 140), "process magenta");
        assert_eq!(cmyk(0.0, 0.0, 1.0, 0.0), (255, 242, 0), "process yellow");
        assert_eq!(cmyk(0.0, 1.0, 1.0, 0.0), (237, 28, 36), "red");
        assert_eq!(cmyk(1.0, 0.0, 1.0, 0.0), (0, 166, 80), "green");
        assert_eq!(cmyk(1.0, 1.0, 0.0, 0.0), (46, 49, 146), "blue");
        assert_eq!(cmyk(1.0, 1.0, 1.0, 1.0), (0, 0, 0), "registration");

        // The one that catches a regression to the naive formula fastest: 100% black ink
        // is a very dark grey, not the absence of light. The naive conversion says
        // (0,0,0), which no press produces and no other viewer shows.
        assert_eq!(cmyk(0.0, 0.0, 0.0, 1.0), (35, 31, 32), "process black");
    }

    /// Interior values interpolate between the corners rather than jumping between them.
    #[test]
    fn intermediate_inks_interpolate() {
        let cmyk = |c, m, y, k| bytes(ColourSpace::Cmyk.to_rgb(&[c, m, y, k]));
        // These follow from the corners by multilinear interpolation and nothing else —
        // 0.5 cyan is exactly halfway between paper and process cyan. They are here to
        // catch a change of interpolation, which the corners alone cannot see.
        assert_eq!(cmyk(0.5, 0.0, 0.0, 0.0), (128, 214, 247));
        assert_eq!(cmyk(0.0, 0.0, 0.0, 0.25), (200, 199, 199));
        assert_eq!(cmyk(0.25, 0.25, 0.25, 0.0), (191, 178, 174));
    }

    /// Out-of-range components must clamp rather than produce colours outside the cube.
    #[test]
    fn components_outside_their_range_are_clamped() {
        let over = ColourSpace::Rgb.to_rgb(&[5.0, -5.0, f32::NAN]);
        assert_eq!(over.r, 1.0);
        assert_eq!(over.g, 0.0);
        assert!(over.b.is_finite(), "a NaN component must not escape");
    }

    /// L*a*b* is converted properly rather than approximated, so the anchors must land.
    #[test]
    fn lab_maps_its_anchor_colours() {
        let range = [-100.0, 100.0, -100.0, 100.0];
        let white = super::cie::lab(100.0, 0.0, 0.0, range, super::D50);
        assert!(
            white.r > 0.99 && white.g > 0.99 && white.b > 0.99,
            "{white:?}"
        );
        let black = super::cie::lab(0.0, 0.0, 0.0, range, super::D50);
        assert!(
            black.r < 0.01 && black.g < 0.01 && black.b < 0.01,
            "{black:?}"
        );
        // A strongly positive a* axis is red.
        let red = super::cie::lab(54.0, 81.0, 70.0, range, super::D50);
        assert!(red.r > red.g && red.r > red.b, "{red:?}");
    }

    /// A `Lab` space is read under the white point Table 64 makes it state, and §8.6.5.4's own
    /// EXAMPLE is the fixture: the D65 space it writes, run through the parser and the
    /// conversion (ADR 1712).
    ///
    /// The clause's second stage makes a colour's XYZ a multiple of that white, and ADR 0012's
    /// Bradford adaptation takes it onto D50, so the space's own white is the display's white
    /// and its neutrals stay neutral — but a chromatic colour is a different colour under D65
    /// than the same three numbers under D50, by more than a level of 255, which is the
    /// difference reading every `Lab` space as D50 had erased.
    #[test]
    fn a_lab_space_is_read_under_its_own_white_point() {
        let space_of = |dictionary: &str| {
            let source = format!(
                "%PDF-1.7\n1 0 obj\n[/Lab {dictionary}]\nendobj\n\
                 trailer\n<< /Size 2 /Root 1 0 R >>\n"
            );
            let document = Document::open(source.into_bytes()).expect("a document");
            let object = document.get(pdf_syntax::ObjectId {
                number: 1,
                generation: 0,
            });
            ColourSpace::parse(&document, &object, &Dictionary::new()).expect("a Lab space")
        };
        let example = space_of("<</WhitePoint [0.9505 1.00 1.0890] /Range [-128 127 -128 127] >>");
        let ColourSpace::Lab {
            white,
            black,
            range,
        } = &example
        else {
            panic!("§8.6.5.4's EXAMPLE is a Lab space: {example:?}");
        };
        assert_eq!(*white, [0.9505, 1.0, 1.089]);
        assert_eq!(*black, [0.0; 3], "Table 64's default BlackPoint");
        assert_eq!(*range, [-128.0, 127.0, -128.0, 127.0]);
        assert_eq!(example.component_range(1), (-128.0, 127.0));

        let d50 = space_of("<</WhitePoint [0.9642 1 0.8249] /Range [-128 127 -128 127] >>");
        let level = |colour: Color| [colour.r, colour.g, colour.b].map(|c| (c * 255.0).round());
        for neutral in [[100.0, 0.0, 0.0], [50.0, 0.0, 0.0], [0.0, 0.0, 0.0]] {
            let one = level(example.to_rgb(&neutral));
            assert!(
                (one[0] - one[1]).abs() <= 1.0 && (one[1] - one[2]).abs() <= 1.0,
                "L* {} with no chroma is neutral under D65: {one:?}",
                neutral[0]
            );
            assert_eq!(one, level(d50.to_rgb(&neutral)), "{neutral:?}");
        }
        for chromatic in [[50.0, 40.0, -40.0], [60.0, -30.0, 30.0]] {
            let one = level(example.to_rgb(&chromatic));
            let other = level(d50.to_rgb(&chromatic));
            assert!(
                one.iter().zip(&other).any(|(x, y)| (x - y).abs() > 1.0),
                "{chromatic:?} under D65 is {one:?}, and under D50 {other:?}"
            );
        }
        assert_eq!(
            example.cie_xyz_at(&[60.0, -30.0, 30.0], 0, Rendering::compensating()),
            Some(super::cie::adapt(
                super::cie::lab_xyz(60.0, -30.0, 30.0, *range, *white),
                *white,
                super::D50
            )),
            "the XYZ handed to another CIE-based space is the one the screen gets"
        );

        let unstated = space_of("<< >>");
        assert!(
            matches!(unstated, ColourSpace::Lab { white, .. } if white == super::D50),
            "a dictionary without its required WhitePoint is read as the connection space's"
        );
    }

    /// The folded D50→sRGB matrix must equal adapting to D65 and then converting.
    ///
    /// [`super::xyz_d50_to_srgb`] carries one matrix where the derivation has two: a
    /// Bradford adaptation from the connection space's D50 onto sRGB's D65, then the
    /// XYZ-to-linear-sRGB matrix IEC 61966-2-1 defines. A folded constant is unreadable and
    /// unfalsifiable on its own — this recomputes it from the two published matrices, so a
    /// typo in any of the nine numbers, or in either Bradford constant, fails here.
    ///
    /// The bound is 1e-3 because the published D50-adapted matrix was computed with
    /// D50 = [0.96422, 1.0, 0.82521], four digits finer than the value ICC.1 states for the
    /// connection space's illuminant and this module therefore uses.
    #[test]
    fn a_folded_matrix_equals_adapting_then_converting() {
        // IEC 61966-2-1: XYZ (D65) to linear sRGB.
        #[rustfmt::skip]
        const SRGB_FROM_XYZ_D65: [f32; 9] = [
             3.240_454_2, -1.537_138_5, -0.498_531_4,
            -0.969_266,    1.876_010_8,  0.041_556,
             0.055_643_4, -0.204_025_9,  1.057_225_2,
        ];
        #[rustfmt::skip]
        const FOLDED: [f32; 9] = [
             3.134_136, -1.617_036, -0.490_662,
            -0.978_755,  1.916_142,  0.033_454,
             0.071_95,  -0.228_988,  1.405_386,
        ];
        // sRGB's own white point, from the same standard.
        const D65: [f32; 3] = [0.950_47, 1.0, 1.088_83];

        // Recover each column of the composed matrix by pushing a basis vector through the
        // two stages, which needs no matrix multiplication of its own to get wrong.
        for column in 0..3 {
            let mut basis = [0.0f32; 3];
            if let Some(entry) = basis.get_mut(column) {
                *entry = 1.0;
            }
            let adapted = super::cie::adapt(basis, super::D50, D65);
            let converted = super::cie::transform(&SRGB_FROM_XYZ_D65, adapted);
            for (row, value) in converted.iter().enumerate() {
                let folded = FOLDED
                    .get(row.saturating_mul(3).saturating_add(column))
                    .copied()
                    .unwrap_or_default();
                assert!(
                    (value - folded).abs() < 1e-3,
                    "row {row} column {column}: derived {value}, folded {folded}"
                );
            }
        }
    }

    /// `CalGray` decodes to a luminance, and a luminance is not an sRGB value.
    ///
    /// ISO 32000-2 §8.6.5.2 makes `A` a CIE quantity: `A^Gamma` scaled by the white point
    /// *is* the XYZ, with no second stage. So with `Gamma 1` the component is linear
    /// luminance, and writing it into an sRGB raster unchanged — which this space's device
    /// equivalent would do — renders every value far too dark.
    ///
    /// The expected bytes are the sRGB encoding of the luminance and nothing else:
    /// `1.055 × 0.35^(1/2.4) − 0.055 = 0.626`, which is 160 of 255.
    #[test]
    fn calgray_is_a_luminance_and_must_be_encoded_for_the_display() {
        let space = ColourSpace::CalGray {
            white: [1.0, 1.0, 1.0],
            black: [0.0, 0.0, 0.0],
            gamma: 1.0,
        };
        assert_eq!(bytes(space.to_rgb(&[0.35])), (160, 160, 160));
        assert_eq!(bytes(space.to_rgb(&[0.75])), (225, 225, 225));
        assert_eq!(bytes(space.to_rgb(&[0.10])), (89, 89, 89));
        // The ends are fixed points whatever the encoding does between them.
        assert_eq!(bytes(space.to_rgb(&[1.0])), (255, 255, 255));
        assert_eq!(bytes(space.to_rgb(&[0.0])), (0, 0, 0));
    }

    /// `Gamma` is applied to the component before the white point scales it.
    ///
    /// §8.6.5.2's EXAMPLE 2 is exactly this space, and it exists because a display's
    /// transfer function is roughly `2.2`: decoding by it and re-encoding for sRGB very
    /// nearly cancels, which is the case a device-equivalent shortcut happens to get right
    /// and the reason the shortcut survived.
    #[test]
    fn calgray_applies_its_gamma_before_the_white_point() {
        let space = ColourSpace::CalGray {
            white: [0.950_5, 1.0, 1.089_0],
            black: [0.0, 0.0, 0.0],
            gamma: 2.222,
        };
        // 0.5^2.222 = 0.2143 luminance, encoded back to 0.5031 — within a level of the
        // component itself, and nowhere near the 89 the Gamma-1 space above gives at 0.35.
        assert_eq!(bytes(space.to_rgb(&[0.5])), (128, 128, 128));
        assert_eq!(bytes(space.to_rgb(&[1.0])), (255, 255, 255));
    }

    /// A `CalRGB` stating sRGB's own parameters must return the component it was given.
    ///
    /// This is the strongest check available for the chain as a whole, because it closes a
    /// loop through every stage — the three gammas, `Matrix`, the Bradford adaptation onto
    /// D50, the folded matrix back out, and the sRGB encoding — using only constants
    /// published in IEC 61966-2-1. Any stage that is wrong, transposed or applied in the
    /// wrong order breaks the identity, and none of them can break it in a compensating way
    /// because the input is not symmetric in its three components.
    ///
    /// `Matrix` is sRGB's primaries as XYZ (D65), in the specification's column order.
    #[test]
    fn calrgb_stating_srgbs_own_parameters_is_the_identity() {
        #[rustfmt::skip]
        let space = ColourSpace::CalRgb {
            white: [0.950_47, 1.0, 1.088_83],
            black: [0.0, 0.0, 0.0],
            gamma: [2.2, 2.2, 2.2],
            matrix: [
                0.412_456, 0.212_673, 0.019_334,
                0.357_576, 0.715_152, 0.119_192,
                0.180_437, 0.072_175, 0.950_304,
            ],
        };
        for input in [[0.2, 0.1, 0.05], [0.5, 0.25, 0.125], [0.9, 0.45, 0.225]] {
            let expected = bytes(ColourSpace::Rgb.to_rgb(&input.map(|value: f32| {
                // The component sRGB *encodes* the same linear light this CalRGB decodes
                // with its 2.2 gamma, which is what makes the two comparable at all.
                super::cie::gamma(value.powf(2.2))
            })));
            assert_eq!(bytes(space.to_rgb(&input)), expected, "{input:?}");
        }
    }

    /// A `CalRGB` stating one gamma, for the cube into a blending colour space.
    fn gamma_space(exponent: [f32; 3]) -> ColourSpace {
        #[rustfmt::skip]
        let matrix = [
            0.412_456, 0.212_673, 0.019_334,
            0.357_576, 0.715_152, 0.119_192,
            0.180_437, 0.072_175, 0.950_304,
        ];
        ColourSpace::CalRgb {
            white: [0.950_47, 1.0, 1.088_83],
            black: [0.0, 0.0, 0.0],
            gamma: exponent,
            matrix,
        }
    }

    /// Three device components from a counter, spread over the unit cube.
    ///
    /// A multiplicative congruential generator rather than a crate: the population has to be
    /// the same on every machine and in every session for the levels below to mean anything.
    fn scattered(index: u32) -> [f32; 3] {
        let mut state = index.wrapping_mul(2_654_435_761).wrapping_add(1);
        std::array::from_fn(|_| {
            state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            #[expect(clippy::cast_precision_loss, reason = "a sample of a 32-bit state")]
            {
                (state >> 8) as f32 / 16_777_216.0
            }
        })
    }

    /// The worst a route's cube departs from evaluating its conversion in, in levels of 255.
    fn worst_inward(route: &RgbRoute, cube: &pdf_render::ColourCube) -> f32 {
        let mut worst = 0.0f32;
        for index in 0..200_000u32 {
            let device = scattered(index);
            let exact = route.components_of_srgb(
                Color::rgb(device[0], device[1], device[2]),
                Rendering::default(),
            );
            let through = cube.convert(device);
            for (interpolated, stated) in through.iter().zip(exact) {
                worst = worst.max((interpolated - stated).abs() * 255.0);
            }
        }
        println!("worst {worst} levels of 255 over 200 000 colours");
        worst
    }

    /// The cube into a `CalRGB` with a gamma is within a level of 255 of the conversion.
    ///
    /// ISO 32000-2 §11.6.6's conversion of a group's result into a parent compositing in the
    /// space, which `pdf_model::content::transparency` carries to the backends as a cube. The
    /// number is what ADR 1267 is about: sampling the whole conversion on a grid with identity
    /// curves put 4.66 levels here at thirty-three samples an axis and 3.40 at sixty-five
    /// (ADR 1254), because the last stage is an inverse gamma whose slope at zero is
    /// unbounded. Splitting the stages — the device's decoding on the input curves, the
    /// linear map in the grid, the space's encoding on the output curve — leaves only the
    /// output curve's own sampling, which is what this measures.
    ///
    /// Calibrated by raising `INWARD_OUTPUT_SAMPLES` to the 33 the grid had: the worst rises
    /// to 21 levels, which is the stage this test is about being put back where it was.
    #[test]
    fn a_cube_into_a_gamma_space_is_within_a_level() {
        let space = gamma_space([2.2, 2.2, 2.2]);
        let route = RgbRoute::of(&space).expect("a CalRGB is a route");
        let cube = route
            .into_cube(Rendering::default())
            .expect("a CalRGB decomposes");
        // Two samples an axis: the grid holds the linear stage and nothing else.
        assert_eq!(cube.side(), 2, "a shared gamma leaves the grid linear");
        let worst = worst_inward(&route, &cube);
        assert!(worst < 1.0, "worst departure {worst} levels of 255");
    }

    /// A linear `CalRGB`'s cube is the conversion exactly, to the arithmetic's own precision.
    ///
    /// With `/Gamma 1` the encoding is the identity and every stage of the conversion in is
    /// linear, so two samples an axis reproduce it — the property `own_space_conversion`
    /// already has for the conversion out, now held in the other direction.
    #[test]
    fn a_cube_into_a_linear_space_is_the_conversion() {
        let space = gamma_space([1.0, 1.0, 1.0]);
        let route = RgbRoute::of(&space).expect("a CalRGB is a route");
        let cube = route
            .into_cube(Rendering::default())
            .expect("a CalRGB decomposes");
        let worst = worst_inward(&route, &cube);
        assert!(worst < 0.1, "worst departure {worst} levels of 255");
    }

    /// The cube into a `CalGray` parent is the conversion, where sampling the whole of it
    /// was 1.70 levels of 255 out.
    ///
    /// §11.6.6's conversion of a group's result into a parent compositing in a one-component
    /// space: §10.4.2.2's weighted sum, then the space's own curve inverted. The sum is affine
    /// on the components themselves, so the grid holds it exactly and only the inverted curve
    /// is sampled — where the construction this replaced interpolated the *composition* over
    /// a grid whose cells span a thirty-second of the range.
    #[test]
    fn a_cube_into_a_calibrated_grey_parent_is_within_a_level() {
        let space = ColourSpace::CalGray {
            white: super::D50,
            black: [0.0, 0.0, 0.0],
            gamma: 2.2,
        };
        let route = GreyRoute::of(&space).expect("a CalGray is a route");
        let cube = route.into_cube().expect("a one-component space decomposes");
        assert_eq!(cube.side(), 2, "one component leaves the grid affine");
        let exact = |device: [f32; 3]| route.component_of(&ColourSpace::Rgb, &device);
        let mut worst = 0.0f32;
        let mut sampled = 0.0f32;
        let side = 33usize;
        #[expect(clippy::cast_precision_loss, reason = "a grid index below the side")]
        let at = |index: usize| index as f32 / side.saturating_sub(1) as f32;
        let mut grid = Vec::with_capacity(side.pow(3));
        for blue in 0..side {
            for green in 0..side {
                for red in 0..side {
                    grid.push([exact([at(red), at(green), at(blue)]); 3]);
                }
            }
        }
        let whole = pdf_render::ColourCube::new(
            std::sync::Arc::from([[0.0f32; 3], [1.0f32; 3]]),
            side,
            std::sync::Arc::from(grid),
            std::sync::Arc::from([0.0f32, 1.0]),
        )
        .expect("a grid of the side it was built at is a cube");
        for index in 0..200_000u32 {
            let device = scattered(index);
            let stated = exact(device);
            worst = worst.max((cube.convert(device)[0] - stated).abs() * 255.0);
            sampled = sampled.max((whole.convert(device)[0] - stated).abs() * 255.0);
        }
        println!("grey: decomposed {worst}, sampled whole {sampled}, levels of 255");
        assert!(worst < 1.0, "worst departure {worst} levels of 255");
    }

    /// The cube the conversion in had before ADR 1267: the whole of it on a grid.
    ///
    /// Kept as a test helper rather than as code, because it is what the measurement below
    /// is *against* — a construction with identity curves on both sides, which puts every
    /// stage of the conversion into the grid.
    fn sampled_whole(route: &RgbRoute, side: usize) -> pdf_render::ColourCube {
        #[expect(clippy::cast_precision_loss, reason = "a grid index below the side")]
        let at = |index: usize| index as f32 / side.saturating_sub(1) as f32;
        let mut grid = Vec::with_capacity(side.pow(3));
        for blue in 0..side {
            for green in 0..side {
                for red in 0..side {
                    grid.push(route.components_of_srgb(
                        Color::rgb(at(red), at(green), at(blue)),
                        Rendering::default(),
                    ));
                }
            }
        }
        pdf_render::ColourCube::new(
            std::sync::Arc::from([[0.0f32; 3], [1.0f32; 3]]),
            side,
            std::sync::Arc::from(grid),
            std::sync::Arc::from([0.0f32, 1.0]),
        )
        .expect("a grid of the side it was built at is a cube")
    }

    /// The cube into a space whose three components are encoded differently beats sampling
    /// the conversion whole, which is the construction it replaced.
    ///
    /// The output curve is one curve and the space states three encodings, so two of them
    /// ride in the grid as residues against the third (see `inward_cube`) and the grid is no
    /// longer exact. What that leaves is measured here against what it replaced rather than
    /// asserted to be small: sampling the whole conversion at the same side puts 6.32 levels
    /// of 255 here and the residues put 3.96, and the gap is the decomposition's.
    ///
    /// A space of one gamma has no residue at all and is the test above.
    #[test]
    fn a_cube_of_three_encodings_beats_sampling_the_conversion_whole() {
        let space = gamma_space([1.8, 2.2, 2.4]);
        let route = RgbRoute::of(&space).expect("a CalRGB is a route");
        let cube = route
            .into_cube(Rendering::default())
            .expect("a CalRGB decomposes");
        assert!(cube.side() > 2, "three encodings need a curved grid");
        let decomposed = worst_inward(&route, &cube);
        let whole = worst_inward(&route, &sampled_whole(&route, cube.side()));
        assert!(
            decomposed < whole,
            "decomposed {decomposed} levels against {whole} sampled whole"
        );
    }

    /// `Matrix` holds one XYZ column per input component, not one row.
    ///
    /// §8.6.5.3 writes it as `[X_A Y_A Z_A X_B Y_B Z_B X_C Y_C Z_C]` — the *first*
    /// component's contribution to all three axes comes first. Reading the nine numbers
    /// row-major transposes the space, and on a near-symmetric matrix such as sRGB's the
    /// error is a small colour shift rather than an obvious one. This matrix is deliberately
    /// far from symmetric: it sends `A` to pure Z and `C` to pure X, so a transposed read
    /// swaps blue and red outright.
    #[test]
    fn calrgb_reads_its_matrix_one_column_per_component() {
        #[rustfmt::skip]
        let space = ColourSpace::CalRgb {
            white: super::D50,
            black: [0.0, 0.0, 0.0],
            gamma: [1.0, 1.0, 1.0],
            matrix: [
                0.0, 0.0, 1.0,   // A contributes only Z
                0.0, 1.0, 0.0,   // B contributes only Y
                1.0, 0.0, 0.0,   // C contributes only X
            ],
        };
        let from_a = space.to_rgb(&[1.0, 0.0, 0.0]);
        assert!(
            from_a.b > from_a.r,
            "A drives Z, which is blue, but got {from_a:?}"
        );
        let from_c = space.to_rgb(&[0.0, 0.0, 1.0]);
        assert!(
            from_c.r > from_c.b,
            "C drives X, which is red, but got {from_c:?}"
        );
    }

    /// A Cal space's `BlackPoint` does not move its colours, whatever it states.
    ///
    /// This pins a *choice* rather than a derivation — `cie_to_srgb` carries the whole
    /// argument for it, and ISO 32000-2 §8.6.5.9 is what makes it a choice at all. What the
    /// test defends is that the choice stays made: a stretch reintroduced here would move
    /// every colour in every document that raises its black point, and nothing about the
    /// resulting page would look wrong.
    ///
    /// The second black point is `calrgb.pdf` page 14's, which Table 63 permits and no
    /// stretch is defined on: its Y span is zero and its Z span negative.
    ///
    /// **The `CalRGB` half is the corpus's own A/B.** `calrgb.pdf` pages 1, 5, 11 and 12 state one
    /// space in `/WhitePoint`, `/Gamma` and `/Matrix` and differ only in `/BlackPoint` — `[0 0 0]`,
    /// `[1 1 1]`, `[8 8 8]`, `[50 50 50]` — and four renderers produce one raster from the four.
    /// The values below are those four pages, so a stretch reintroduced here fails this test on the
    /// same input the oracle would fail on.
    #[test]
    fn a_cal_spaces_black_point_does_not_move_its_colours() {
        let grey = |black| {
            bytes(
                ColourSpace::CalGray {
                    white: [1.0, 1.0, 1.0],
                    black,
                    gamma: 1.0,
                }
                .to_rgb(&[0.35]),
            )
        };
        assert_eq!(grey([0.0, 0.0, 0.0]), (160, 160, 160));
        assert_eq!(grey([0.7, 0.7, 0.7]), (160, 160, 160));
        assert_eq!(grey([0.2, 1.0, 1.7]), (160, 160, 160));

        // And refusing compensation cannot change what was never compensated, which is what
        // makes `/UseBlackPtComp OFF` a no-op here rather than a second behaviour.
        let space = ColourSpace::CalGray {
            white: [1.0, 1.0, 1.0],
            black: [0.7, 0.7, 0.7],
            gamma: 1.0,
        };
        assert_eq!(
            bytes(space.to_rgb_without_black_point(&[0.35])),
            bytes(space.to_rgb(&[0.35]))
        );

        // `calrgb.pdf` pages 1, 5, 11 and 12: one identity `CalRGB` under four black points,
        // on the swatch the oracle's own table reads, `A B C = 0.75 0.00 0.00`.
        let swatch = |black| {
            bytes(
                ColourSpace::CalRgb {
                    white: [1.0, 1.0, 1.0],
                    black,
                    gamma: [1.0, 1.0, 1.0],
                    matrix: [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0],
                }
                .to_rgb(&[0.75, 0.0, 0.0]),
            )
        };
        let stated = swatch([0.0, 0.0, 0.0]);
        assert_eq!(swatch([1.0, 1.0, 1.0]), stated);
        assert_eq!(swatch([8.0, 8.0, 8.0]), stated);
        assert_eq!(swatch([50.0, 50.0, 50.0]), stated);
    }

    #[test]
    fn an_indexed_space_reads_its_table() {
        let space = ColourSpace::Indexed {
            base: Box::new(ColourSpace::Rgb),
            lookup: vec![1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0],
            high: 2,
        };
        assert_eq!(space.components(), 1, "an index is one component");
        let second = space.to_rgb(&[1.0]);
        assert_eq!((second.r, second.g, second.b), (0.0, 1.0, 0.0));
        // An index past the end clamps rather than reading past the table.
        let clamped = space.to_rgb(&[99.0]);
        assert_eq!((clamped.r, clamped.g, clamped.b), (0.0, 0.0, 1.0));
    }
}

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

use pdf_render::Color;
use pdf_syntax::{Dictionary, Document, Object};

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
    /// Three components in the CIE L*a*b* space, with the given axis ranges.
    Lab {
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
    /// One or more tint components, converted by a function into an alternate space.
    Separation {
        /// How many tint components the space takes.
        inputs: usize,
        /// The space the tint transform produces.
        alternate: Box<ColourSpace>,
        /// The tint transform.
        transform: Box<Function>,
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
        profile: Box<crate::icc::Profile>,
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

impl ColourSpace {
    /// Resolves a colour space object.
    ///
    /// Returns `None` for a space this crate does not implement, so the caller reports it
    /// rather than guessing a colour.
    #[must_use]
    pub fn parse(document: &Document, object: &Object, resources: &Dictionary) -> Option<Self> {
        Self::parse_at(document, object, resources, 0)
    }

    fn parse_at(
        document: &Document,
        object: &Object,
        resources: &Dictionary,
        depth: usize,
    ) -> Option<Self> {
        if depth > MAX_DEPTH {
            return None;
        }
        let resolved = document.resolve(object);

        if let Some(name) = resolved.as_name() {
            let name = String::from_utf8_lossy(name.as_bytes()).into_owned();
            return Self::by_name(document, &name, resources, depth);
        }

        let items = resolved.as_array()?;
        let family = items
            .first()
            .map(|item| document.resolve(item))
            .and_then(|item| item.as_name().map(|n| n.as_bytes().to_vec()))?;

        match family.as_slice() {
            b"DeviceGray" | b"G" => Some(Self::Gray),
            b"DeviceRGB" | b"RGB" => Some(Self::Rgb),
            b"DeviceCMYK" | b"CMYK" => Some(Self::Cmyk),
            b"CalGray" => Some(Self::parse_cal_gray(document, items.get(1))),
            b"CalRGB" => Some(Self::parse_cal_rgb(document, items.get(1))),
            // §8.6.5.1: "A PDF reader shall ignore CalCMYK colour space attributes and
            // render colours specified in this family as if they had been specified using
            // DeviceCMYK." The family was withdrawn before it was ever completed — NOTE 1
            // says its definition "has been completely removed" — so there is nothing to
            // calibrate against and the clause states the whole of what a reader does with
            // one. No corpus document writes it; without this arm such a file would report
            // an unsupported colour space, which is a *refusal* where the standard states
            // an answer.
            #[expect(
                clippy::match_same_arms,
                reason = "the same answer for a different reason: `DeviceCMYK` is a device \
                          space and `CalCMYK` is a withdrawn family the clause redirects to \
                          it, and merging the arms would put one comment over two rules"
            )]
            b"CalCMYK" => Some(Self::Cmyk),
            b"Pattern" => Some(Self::Pattern {
                base: items
                    .get(1)
                    .and_then(|item| {
                        Self::parse_at(document, item, resources, depth.saturating_add(1))
                    })
                    .map(Box::new),
            }),
            b"Lab" => {
                let dict = items.get(1).map(|item| document.resolve(item));
                let dict = dict.as_ref().and_then(Object::as_dict);
                let range = dict
                    .and_then(|dict| {
                        let array = document.get_key(dict, "Range");
                        let values: Vec<f32> = array
                            .as_array()?
                            .iter()
                            .filter_map(|item| document.resolve(item).as_number().map(narrow))
                            .collect();
                        <[f32; 4]>::try_from(values.as_slice()).ok()
                    })
                    .unwrap_or([-100.0, 100.0, -100.0, 100.0]);
                Some(Self::Lab { range })
            }
            b"ICCBased" => {
                let stream = items.get(1).map(|item| document.resolve(item))?;
                let stream = stream.as_stream()?;

                // The profile itself is the document's own statement of what its numbers
                // mean, so it wins over everything below.
                if let Some(data) = document.decoded_stream_data(stream)
                    && let Some(profile) = crate::icc::Profile::parse(&data)
                {
                    return Some(Self::Icc {
                        profile: Box::new(profile),
                    });
                }

                // `/Alternate` is the producer's own statement of what to use when the
                // profile cannot be applied. Preferring it over a guess from `/N` is what
                // the specification asks for and is free: a document saying its profile
                // stands in for `Lab` or a `Separation` gets that, rather than whichever
                // device space happens to have the same component count.
                if let Some(alternate) = stream.dict.get("Alternate")
                    && let Some(space) =
                        Self::parse_at(document, alternate, resources, depth.saturating_add(1))
                {
                    return Some(space);
                }

                // Failing that, the device space with the same component count — which is
                // the fallback the specification itself describes.
                match document.get_key(&stream.dict, "N").as_integer() {
                    Some(1) => Some(Self::Gray),
                    Some(4) => Some(Self::Cmyk),
                    // Three is the common case, and an absent or nonsensical `/N` is far
                    // more likely to be RGB than anything else.
                    _ => Some(Self::Rgb),
                }
            }
            b"Indexed" | b"I" => Self::parse_indexed(document, items, resources, depth),
            b"Separation" | b"DeviceN" => {
                let is_separation = family.as_slice() == b"Separation";
                let names = colourant_names(document, items.get(1), is_separation);
                let inputs = if is_separation { 1 } else { names.len() };

                // §8.6.6.4's two special names are decided *before* the alternate space and
                // the tint transform, because the clause requires both to be ignored: "A PDF
                // processor shall support Separation colour spaces with the colourant names
                // All and None on all devices, even if the devices are not capable of
                // supporting any others. When processing Separation spaces with either of
                // these colourant names PDF processors shall ignore the alternateSpace and
                // tintTransform parameters … although valid values shall still be provided."
                // Reading them first is what makes that true even where they are unreadable.
                if is_separation {
                    match names.first().map(Vec::as_slice) {
                        Some(b"All") => return Some(Self::AllColourants),
                        Some(b"None") => return Some(Self::NoColourant { inputs: 1 }),
                        _ => {}
                    }
                } else if inputs > 0 && names.iter().all(|name| name.as_slice() == b"None") {
                    // §8.6.6.5: "A DeviceN colour space whose component colourant names are
                    // all None shall always discard its output, just the same as a Separation
                    // colour space for None; it shall never revert to the alternate colour
                    // space."
                    return Some(Self::NoColourant { inputs });
                }

                if inputs == 0 {
                    // §8.6.6.5's `names` array decides how many operands `scn` takes, so an
                    // empty or missing one leaves the space undefined rather than degenerate.
                    return None;
                }
                let alternate =
                    Self::parse_at(document, items.get(2)?, resources, depth.saturating_add(1))?;
                let transform = Function::parse(document, items.get(3)?).ok()?;
                Some(Self::Separation {
                    inputs,
                    alternate: Box::new(alternate),
                    transform: Box::new(transform),
                })
            }
            _ => None,
        }
    }

    /// Reads a `CalGray` dictionary: ISO 32000-2 §8.6.5.2, Table 62.
    fn parse_cal_gray(document: &Document, dictionary: Option<&Object>) -> Self {
        let dict = dictionary.map(|item| document.resolve(item));
        let dict = dict.as_ref().and_then(Object::as_dict);
        Self::CalGray {
            white: white_point(document, dict),
            black: numbers(document, dict, "BlackPoint").unwrap_or([0.0, 0.0, 0.0]),
            // Table 62: "G shall be positive". A non-positive exponent is not a gamma, and
            // `powf` would answer with an infinity rather than a colour.
            gamma: dict
                .map(|dict| document.get_key(dict, "Gamma"))
                .and_then(|value| value.as_number())
                .map(narrow)
                .filter(|gamma| *gamma > 0.0)
                .unwrap_or(1.0),
        }
    }

    /// Reads a `CalRGB` dictionary: ISO 32000-2 §8.6.5.3, Table 63.
    fn parse_cal_rgb(document: &Document, dictionary: Option<&Object>) -> Self {
        let dict = dictionary.map(|item| document.resolve(item));
        let dict = dict.as_ref().and_then(Object::as_dict);
        let gamma = numbers(document, dict, "Gamma").unwrap_or([1.0, 1.0, 1.0]);
        Self::CalRgb {
            white: white_point(document, dict),
            black: numbers(document, dict, "BlackPoint").unwrap_or([0.0, 0.0, 0.0]),
            gamma: gamma.map(|value| if value > 0.0 { value } else { 1.0 }),
            matrix: numbers(document, dict, "Matrix")
                .unwrap_or([1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0]),
        }
    }

    /// Reads an `Indexed` space: a base space, a maximum index, and a palette.
    fn parse_indexed(
        document: &Document,
        items: &[Object],
        resources: &Dictionary,
        depth: usize,
    ) -> Option<Self> {
        let base = Self::parse_at(document, items.get(1)?, resources, depth.saturating_add(1))?;
        let high = usize::try_from(
            items
                .get(2)
                .map(|item| document.resolve(item))
                .and_then(|item| item.as_integer())?,
        )
        .ok()?;
        let table = items.get(3).map(|item| document.resolve(item))?;
        let bytes = match &table {
            Object::String(bytes) => bytes.clone(),
            Object::Stream(stream) => document.decoded_stream_data(stream)?,
            _ => return None,
        };
        // §8.6.6.3: "Each byte shall be an unsigned integer in the range 0 to 255 that shall
        // be scaled to the range of the corresponding colour component in the base colour
        // space; that is, 0 corresponds to the minimum value in the range for that
        // component, and 255 corresponds to the maximum."
        //
        // Dividing by 255 is that scaling only where the base's components run 0 to 1, which
        // is every space but `Lab`. `issue2761.pdf` indexes into a `Lab` base and drew a
        // black square where four renderers draw a pale grey gradient: its lightest entry is
        // L = 253, which is 0.99 as a fraction and 99 out of 100 as a lightness.
        let components = base.components().max(1);
        let scaled: Vec<f32> = bytes
            .chunks(components)
            .flat_map(|entry| {
                entry.iter().enumerate().map(|(component, byte)| {
                    let (low, high) = base.component_range(component);
                    low + f32::from(*byte) * (high - low) / 255.0
                })
            })
            .collect();
        Some(Self::Indexed {
            base: Box::new(base),
            lookup: scaled,
            high,
        })
    }

    /// Table 88's default `/Decode` pair for one component of an image in this space.
    ///
    /// §8.9.5.2's table gives every family the full range of its components — which is
    /// [`Self::component_range`] — with one exception the table's own NOTE 2 names: an
    /// `Indexed` space's default is `[0 2^n − 1]`, so that "component values that index a
    /// colour table are passed through unchanged" rather than being scaled to 0.0..=1.0.
    pub(crate) fn default_decode(&self, component: usize, bits: u32) -> (f32, f32) {
        match self {
            Self::Indexed { .. } => {
                let max = (1u32 << bits.min(16)).saturating_sub(1);
                #[expect(
                    clippy::cast_precision_loss,
                    reason = "at most 65535, which f32 represents exactly"
                )]
                let high = max as f32;
                (0.0, high)
            }
            _ => self.component_range(component),
        }
    }

    /// The range one component of a colour in this space may take.
    ///
    /// Every family's components run from 0.0 to 1.0 except two. `Lab`'s lightness is a
    /// percentage and its two chromatic axes take their bounds from the space's own `/Range`
    /// (§8.6.5.4, Table 65). An `Indexed` space's one component is an index, which §8.6.6.3
    /// bounds by `hival`: "if the value is greater than hival, it shall be clipped".
    ///
    /// Two callers, both about a *range* rather than a value: [`Self::parse_indexed`] scales
    /// a colour table's bytes onto the base space's components, and [`crate::image`] clamps
    /// what a `/Decode` array produces.
    pub(crate) fn component_range(&self, component: usize) -> (f32, f32) {
        match self {
            Self::Indexed { high, .. } => {
                #[expect(
                    clippy::cast_precision_loss,
                    reason = "an index beyond f32's exact integers cannot address a table \
                              this crate built"
                )]
                let top = *high as f32;
                (0.0, top)
            }
            Self::Lab { range } => match component {
                0 => (0.0, 100.0),
                other => {
                    let at = other.saturating_sub(1).saturating_mul(2);
                    (
                        range.get(at).copied().unwrap_or(-100.0),
                        range.get(at.saturating_add(1)).copied().unwrap_or(100.0),
                    )
                }
            },
            _ => (0.0, 1.0),
        }
    }

    /// Resolves a space named directly, looking it up in the resources if need be.
    fn by_name(
        document: &Document,
        name: &str,
        resources: &Dictionary,
        depth: usize,
    ) -> Option<Self> {
        // Choosing a device space is a request to use the *default* space standing in for
        // it, where the resources name one. ISO 32000-2 §8.6.5.6: "If such an entry is
        // present, its value shall be used as the colour space for the operation currently
        // being performed." This is how a producer says "my DeviceCMYK means this press",
        // and ignoring it renders those documents in the wrong colours entirely.
        let default = match name {
            "DeviceGray" | "G" | "CalGray" => Some("DefaultGray"),
            "DeviceRGB" | "RGB" | "CalRGB" => Some("DefaultRGB"),
            "DeviceCMYK" | "CMYK" => Some("DefaultCMYK"),
            _ => None,
        };
        if let Some(default) = default
            && let Some(space) = Self::named_default(document, default, resources, depth)
        {
            return Some(space);
        }

        match name {
            "DeviceGray" | "G" | "CalGray" => return Some(Self::Gray),
            "DeviceRGB" | "RGB" | "CalRGB" => return Some(Self::Rgb),
            "DeviceCMYK" | "CMYK" => return Some(Self::Cmyk),
            // A bare `/Pattern` names no underlying space; the caller falls back on the
            // operand count when one is needed.
            "Pattern" => return Some(Self::Pattern { base: None }),
            _ => {}
        }
        // Anything else is a name in the page's `/ColorSpace` resource dictionary.
        let table = document.get_key(resources, "ColorSpace");
        let table = table.as_dict()?;
        let entry = table.get(name)?;
        Self::parse_at(document, entry, resources, depth.saturating_add(1))
    }

    /// Resolves a `/DefaultGray`, `/DefaultRGB` or `/DefaultCMYK` entry, if present.
    ///
    /// A default that resolves back to the device space it replaces would recurse for
    /// ever, so the lookup is bounded by the same depth limit as everything else here.
    fn named_default(
        document: &Document,
        key: &str,
        resources: &Dictionary,
        depth: usize,
    ) -> Option<Self> {
        if depth > MAX_DEPTH {
            return None;
        }
        let table = document.get_key(resources, "ColorSpace");
        let entry = table.as_dict()?.get(key)?;
        Self::parse_at(document, entry, resources, depth.saturating_add(1))
    }

    /// How many numbers a colour in this space takes.
    #[must_use]
    pub fn components(&self) -> usize {
        match self {
            Self::Gray | Self::Indexed { .. } | Self::CalGray { .. } | Self::AllColourants => 1,
            Self::Icc { profile } => profile.channels(),
            // A pattern is named, not given as components; where an uncoloured one takes
            // a colour, that colour belongs to the underlying space.
            Self::Pattern { base } => base.as_ref().map_or(1, |base| base.components()),
            Self::Rgb | Self::Lab { .. } | Self::CalRgb { .. } => 3,
            Self::Cmyk => 4,
            Self::Separation { inputs, .. } | Self::NoColourant { inputs } => *inputs,
        }
    }

    /// The colour a space starts in, when `cs` or `CS` selects it.
    ///
    /// ISO 32000-2 §8.6.8, of the `CS` operator: it "shall also set the current stroking
    /// colour to its initial value, which depends on the colour space", and then gives one
    /// per family. They are not all black, which is what makes this worth a function rather
    /// than a constant: a `Separation` starts at *full* ink, and an `Indexed` space starts at
    /// whatever its table's entry 0 happens to be.
    ///
    /// Returned as components in this space rather than as a colour, so that the caller
    /// converts them by the same route as any other colour — there is exactly one of those.
    #[must_use]
    pub fn initial_colour(&self) -> Vec<f32> {
        match self {
            // "In a DeviceGray, DeviceRGB, CalGray, or CalRGB colour space, the initial
            // colour shall have all components equal to 0.0."
            Self::Gray | Self::Rgb | Self::CalGray { .. } | Self::CalRgb { .. } => {
                vec![0.0; self.components()]
            }
            // "In a DeviceCMYK colour space, the initial colour shall be [0.0 0.0 0.0 1.0]."
            Self::Cmyk => vec![0.0, 0.0, 0.0, 1.0],
            // "In a Lab or ICCBased colour space, the initial colour shall have all
            // components equal to 0.0 unless that falls outside the intervals specified by
            // the space's Range entry, in which case the nearest valid value shall be
            // substituted." Lab's `a` and `b` carry that range here; an ICCBased space's
            // `/Range` is not read, and zero is inside it for every profile the corpus has.
            Self::Lab { range } => vec![
                0.0,
                0.0_f32.clamp(range[0], range[1]),
                0.0_f32.clamp(range[2], range[3]),
            ],
            Self::Icc { .. } => vec![0.0; self.components()],
            // "In an Indexed colour space, the initial colour value shall be 0."
            Self::Indexed { .. } => vec![0.0],
            // "In a Separation or DeviceN colour space, the initial tint value shall be 1.0
            // for all colourants." `/All` and `/None` are Separation spaces too, so they
            // start at full ink like any other — which for `/All` is black.
            Self::Separation { inputs, .. } | Self::NoColourant { inputs } => vec![1.0; *inputs],
            Self::AllColourants => vec![1.0],
            // A pattern has no colour of its own until `scn` names one.
            Self::Pattern { .. } => Vec::new(),
        }
    }

    /// Converts a colour in this space to RGB.
    ///
    /// Black point compensation is applied where the space is an ICC profile, which is
    /// the `Default` behaviour ISO 32000-2 §8.6.5.9 leaves to the processor. Use
    /// [`Self::to_rgb_without_black_point`] where the document has asked for it off.
    #[must_use]
    pub fn to_rgb(&self, values: &[f32]) -> Color {
        self.to_rgb_at(values, 0, true)
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
        self.to_rgb_at(values, 0, false)
    }

    fn to_rgb_at(&self, values: &[f32], depth: usize, black_point: bool) -> Color {
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
            Self::Icc { profile } => profile.to_rgb_with(values, black_point),
            Self::Lab { range } => lab(at(0), at(1), at(2), *range),
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
                let mut xyz = [0.0f32; 3];
                for (column, input) in decoded.iter().enumerate() {
                    for (axis, output) in xyz.iter_mut().enumerate() {
                        let entry = matrix
                            .get(column.saturating_mul(3).saturating_add(axis))
                            .copied()
                            .unwrap_or(0.0);
                        *output += entry * input;
                    }
                }
                cie_to_srgb(xyz, *white)
            }
            Self::Indexed { base, lookup, high } => {
                let components = base.components();
                let raw = at(0);
                let index = if raw.is_nan() || raw <= 0.0 {
                    0
                } else {
                    #[expect(
                        clippy::cast_possible_truncation,
                        clippy::cast_sign_loss,
                        reason = "guarded finite and positive; a float-to-integer cast \
                                  saturates rather than wrapping, and the result is \
                                  clamped to `high` immediately below"
                    )]
                    let rounded = raw.round() as usize;
                    rounded
                };
                let index = index.min(*high);
                let start = index.saturating_mul(components);
                let slice: Vec<f32> = (0..components)
                    .map(|offset| {
                        lookup
                            .get(start.saturating_add(offset))
                            .copied()
                            .unwrap_or(0.0)
                    })
                    .collect();
                base.to_rgb_at(&slice, depth.saturating_add(1), black_point)
            }
            Self::Separation {
                alternate,
                transform,
                ..
            } => {
                let converted = transform.eval(values);
                alternate.to_rgb_at(&converted, depth.saturating_add(1), black_point)
            }
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
                base.to_rgb_at(values, depth.saturating_add(1), black_point)
            }),
        }
    }
}

/// Clamps a colour component to `0.0..=1.0`, treating `NaN` as zero.
///
/// `f32::clamp` *propagates* `NaN` rather than removing it, so clamping alone is not
/// enough. A `NaN` reaching a rasteriser is not a wrong colour, it is an undefined one:
/// it survives premultiplication and blending and can leave a pixel with no value at all.
/// Functions are the likely source, since a malformed one can produce anything.
fn channel(value: f32) -> f32 {
    if value.is_nan() {
        return 0.0;
    }
    value.clamp(0.0, 1.0)
}

/// The sixteen corners of the CMYK cube, as sRGB, indexed by the bits `c m y k`.
///
/// # The standard states two answers and ranks them, and this is the higher one
///
/// This comment used to open "ISO 32000-2 gives **no** conversion from `DeviceCMYK` to any
/// other space", on the evidence of §8.6.4.4, which says of the components only that they
/// "shall represent the concentrations of these process colourants". That was true of the
/// clause it cited and false of the standard. **§10.4.2.5 states a conversion outright** —
/// each additive component is one minus the sum of its complementary ink and the black,
/// clamped at one — and §10.4.2.1 says what it is for: "a less-capable PDF processor **may** choose to use
/// the algorithms specified in the following subclauses 10.4.2.2 through 10.4.2.5. These
/// algorithms are, however, very simple and as perceived by a human viewer they produce only
/// crude approximations of the original colours." An ICC-enabled processor "should always
/// follow the provisions and recommendations provided in 10.3".
///
/// So the question is not unanswered; it is answered twice, in a stated order, and this tree
/// is on the higher branch. §10.3.2 is the sentence that authorises what this table *is*:
///
/// > A PDF processor should establish CIE-based colour specifications for device colour
/// > spaces ( DeviceGray , DeviceRGB , or DeviceCMYK ), and thus implicitly remap device
/// > colour spaces into CIEbased colour spaces, when those device colour spaces do not
/// > match that of the raster output device.
///
/// A display's native space is not CMYK, so the remapping is required of us, and §10.3.1
/// puts the choice of destination "beyond the scope of this document" while its NOTE lists
/// "assumptions made by the PDF processor software" among the ways it may be made. Assuming
/// standard process inks *is* such an assumption, made in the one place the clause leaves for
/// it.
///
/// What the specification does state is *which press to ask first*, and it names three
/// sources — `/DefaultCMYK` (§8.6.5.6, "shall be used"), an output intent's
/// `/DestOutputProfile` (§14.11.5, §8.6.5.7 NOTE 3), and an `ICCBased` space naming the
/// profile directly. All three are implemented and all three win over this table, which is
/// reached only when the document names no press at all.
///
/// The press assumed is standard process inks, at their published sRGB appearances:
/// `#00AEEF` cyan, `#EC008C` magenta, `#FFF200` yellow, `#231F20` black, with the
/// overprints that follow from them. Written as eight-bit values because that is the
/// precision at which ink appearances are published.
///
/// # And the crude answer was measured rather than dismissed
///
/// §10.4.2.5's formula is off by up to 115 of 255 at these corners and renders process
/// magenta as `#FF00FF`, a colour no ink produces — which is the difference between an answer
/// about subtractive ink and one about additive light. That is an argument; the measurement
/// is the evidence. Over the whole oracle, switching this table for §10.4.2.5's formula moves
/// **802 agreeing and 88 contradicted pages to 800 and 90**. So the standard's own lower
/// branch is worse here than its higher one, which is what §10.4.2.1 says to expect of it.
///
/// Other readers land within a level of these numbers. That is evidence that assuming
/// standard process inks is the conventional reading of §10.3.2's licence — not evidence that
/// these numbers are correct, because the clause states no destination for them to be correct
/// against.
#[rustfmt::skip]
const CMYK_CORNERS: [[u8; 3]; 16] = [
    //  R    G    B      c m y k
    [255, 255, 255], // 0 0 0 0  paper
    [  0, 173, 239], // 1 0 0 0  process cyan
    [236,   0, 140], // 0 1 0 0  process magenta
    [ 46,  49, 146], // 1 1 0 0  blue
    [255, 242,   0], // 0 0 1 0  process yellow
    [  0, 166,  80], // 1 0 1 0  green
    [237,  28,  36], // 0 1 1 0  red
    [ 54,  54,  57], // 1 1 1 0  three-colour black
    [ 35,  31,  32], // 0 0 0 1  process black
    [  0,  15,  36], // 1 0 0 1
    [ 36,   0,   0], // 0 1 0 1
    [  0,   0,   2], // 1 1 0 1
    [ 28,  26,   0], // 0 0 1 1
    [  0,  19,   0], // 1 0 1 1
    [ 34,   0,   0], // 0 1 1 1
    [  0,   0,   0], // 1 1 1 1  registration
];

/// Converts `DeviceCMYK` to sRGB by interpolating between the cube's corners.
///
/// Multilinear: each of the four inks contributes independently, which is what makes the
/// result exact at the sixteen corners and continuous everywhere between them. It is the
/// interpolation an ICC lookup table uses over its own grid, so the fallback behaves like
/// the profiles it stands in for rather than like a different kind of thing.
///
/// Interior points land within one level of what other readers produce, which are
/// themselves up to 53 levels apart from each other there — so this agrees with all of
/// them more closely than they agree among themselves.
fn cmyk(c: f32, m: f32, y: f32, k: f32) -> Color {
    let (c, m, y, k) = (channel(c), channel(m), channel(y), channel(k));
    let weights = [1.0 - c, c];
    let weights_m = [1.0 - m, m];
    let weights_y = [1.0 - y, y];
    let weights_k = [1.0 - k, k];

    let mut rgb = [0.0f32; 3];
    for (index, corner) in CMYK_CORNERS.iter().enumerate() {
        // The index's bits select which side of each axis this corner sits on, in the
        // order c, m, y, k from the least significant bit.
        let weight = weights[index & 1]
            * weights_m[(index >> 1) & 1]
            * weights_y[(index >> 2) & 1]
            * weights_k[(index >> 3) & 1];
        if weight == 0.0 {
            continue;
        }
        for (channel, value) in rgb.iter_mut().zip(corner.iter()) {
            *channel += weight * f32::from(*value) / 255.0;
        }
    }
    Color::rgb(channel(rgb[0]), channel(rgb[1]), channel(rgb[2]))
}

/// The inverse of the L*a*b* companding function.
fn expand(value: f32) -> f32 {
    if value >= 6.0 / 29.0 {
        value * value * value
    } else {
        108.0 / 841.0 * (value - 4.0 / 29.0)
    }
}

/// Converts CIE L*a*b* to D50 XYZ, for callers outside this module.
#[expect(
    clippy::many_single_char_names,
    reason = "a*, b*, and the intermediate m, l, n are the formula's own names"
)]
pub(crate) fn lab_to_xyz(lightness: f32, a: f32, b: f32) -> [f32; 3] {
    let m = (lightness.clamp(0.0, 100.0) + 16.0) / 116.0;
    let l = m + a / 500.0;
    let n = m - b / 200.0;
    [0.964_2 * expand(l), expand(m), 0.824_9 * expand(n)]
}

/// The CIE D50 white point, which is the ICC profile connection space's own.
///
/// Every CIE-based colour in this module is adapted to it before the one matrix that turns
/// an XYZ into a pixel, so it is the single hinge the whole colour path turns on.
pub(crate) const D50: [f32; 3] = [0.964_2, 1.0, 0.824_9];

/// The Bradford cone response matrix, and its inverse.
///
/// ISO 32000-2 §10.3.1 says conversion from a CIE-based source to the destination "shall be
/// performed based on ISO 15076-1:2010 (ICC.1:2010)", and that standard's media-relative
/// colorimetric intent adapts the source's white point onto the connection space's D50.
/// Bradford is the transform ICC's own `chad` tag carries, so this is the adaptation the
/// referenced standard describes rather than a choice made here.
///
/// Both matrices are the published constants, row-major.
#[rustfmt::skip]
const BRADFORD: [f32; 9] = [
     0.895_1,  0.266_4, -0.161_4,
    -0.750_2,  1.713_5,  0.036_7,
     0.038_9, -0.068_5,  1.029_6,
];

/// The inverse of [`BRADFORD`], row-major.
#[rustfmt::skip]
const BRADFORD_INVERSE: [f32; 9] = [
     0.986_992_9, -0.147_054_3,  0.159_962_7,
     0.432_305_3,  0.518_360_3,  0.049_291_2,
    -0.008_528_7,  0.040_042_8,  0.968_486_7,
];

/// Multiplies a row-major 3×3 matrix by a column vector.
fn transform(matrix: &[f32; 9], vector: [f32; 3]) -> [f32; 3] {
    let mut out = [0.0f32; 3];
    for (row, output) in out.iter_mut().enumerate() {
        for (column, input) in vector.iter().enumerate() {
            let entry = matrix
                .get(row.saturating_mul(3).saturating_add(column))
                .copied()
                .unwrap_or(0.0);
            *output += entry * input;
        }
    }
    out
}

/// Takes a CIE-based colour from its own XYZ to sRGB: adapt the white point, then convert.
///
/// `white` is the source space's own diffuse white, which the adaptation maps onto the
/// connection space's D50. Both `CalGray` and `CalRGB` end here, which is why the two share
/// every step after their own decoding stage.
///
/// # `BlackPoint` is read and deliberately not applied
///
/// A Cal space's `BlackPoint` is its *source's* diffuse shadow — §8.6.5.3 says outright that
/// it "is limited by the dynamic range of the input device" and "varies with exposure,
/// system response, and artistic intent". Stretching the range so that shadow lands on the
/// display's black is black point compensation, and ISO 32000-2 §8.6.5.9 makes that a
/// processor decision: `ON` means "according to the provisions in ISO 18619", `OFF` means
/// none, and `Default` — which is what every document in the corpus leaves it at — is
/// "left to the PDF processor to determine".
///
/// So this is a choice, not a derivation, and the choice is to reproduce the colorimetry the
/// space states. Two things decided it:
///
/// - The stretch is **undefined on input the specification permits**. Table 63 requires only
///   that the three numbers be non-negative — nothing puts the black below the white.
///   `calrgb.pdf` page 14 states `BlackPoint [0.2 1.0 1.7]` against `WhitePoint [1 1 1]`, so
///   the Y axis has zero span and the Z axis a negative one. A construction that has to be
///   guarded into doing nothing on two of three axes is not the construction the clause
///   means, and what it does on the third is arbitrary.
/// - The quantity is **not the one `icc.rs` compensates**, which is why that path keeps its
///   compensation and this one has none. There the black point is *measured* from the
///   profile — the darkest colour the device can actually reach — and aligning it is what
///   `PDF20_AN001-BPC` argues for. A source's stated shadow is a different quantity that
///   happens to share a name.
///
/// The cost, stated plainly: a document raising its `BlackPoint` gets shadows at the
/// lightness it states rather than stretched down to the display's black. `calgray.pdf`
/// page 3 and `calrgb.pdf` page 14 are the corpus's only examples, and both are files
/// written to probe this entry rather than to display anything.
///
/// All three reference renderers do the same, which is evidence that this is how §8.6.5.2
/// and §8.6.5.3 are commonly read — not the reason for the choice, which is above.
fn cie_to_srgb(xyz: [f32; 3], white: [f32; 3]) -> Color {
    xyz_d50_to_srgb(adapt(xyz, white, D50))
}

/// Chromatically adapts an XYZ from one white point onto another.
///
/// Scaling the three cone responses is the von Kries construction; Bradford is which cone
/// responses. `from` maps exactly onto `to`, which is the property everything here relies
/// on: a `CalGray` states its colours as multiples of its own white, so adapting turns them
/// into the same multiples of D50 and the grey stays grey.
fn adapt(xyz: [f32; 3], from: [f32; 3], to: [f32; 3]) -> [f32; 3] {
    let source = transform(&BRADFORD, from);
    let destination = transform(&BRADFORD, to);
    let mut cone = transform(&BRADFORD, xyz);
    for (axis, value) in cone.iter_mut().enumerate() {
        // A white point with a zero or negative cone response is not a white point; leaving
        // the axis alone is the only answer that cannot produce an infinity.
        if source[axis].abs() > 1e-9 {
            *value *= destination[axis] / source[axis];
        }
    }
    transform(&BRADFORD_INVERSE, cone)
}

/// Converts a D50 XYZ to sRGB.
///
/// The only place in this crate where an XYZ becomes a pixel. `Lab`, `CalGray`, `CalRGB`
/// and every ICC profile arrive here, and the module documentation says why that matters.
#[expect(
    clippy::many_single_char_names,
    reason = "X, Y, Z and R, G, B are the colour spaces' own axis names"
)]
pub(crate) fn xyz_d50_to_srgb(xyz: [f32; 3]) -> Color {
    let (x, y, z) = (xyz[0], xyz[1], xyz[2]);
    // XYZ (D50) to linear sRGB: the sRGB primaries' matrix with a Bradford adaptation from
    // D50 to sRGB's own D65 white already folded in, which is why it is not the matrix
    // IEC 61966-2-1 prints. `a_folded_matrix_equals_adapting_then_converting` derives it.
    let r = 3.134_136 * x - 1.617_036 * y - 0.490_662 * z;
    let g = -0.978_755 * x + 1.916_142 * y + 0.033_454 * z;
    let b = 0.071_95 * x - 0.228_988 * y + 1.405_386 * z;
    Color::rgb(gamma(r), gamma(g), gamma(b))
}

/// Converts CIE L*a*b* to sRGB through XYZ, using the D50 white point PDF specifies.
#[expect(
    clippy::many_single_char_names,
    reason = "L*, a*, b*, X, Y and Z are the colour space's own names for its axes; \
              renaming them would make this harder to check against the formulae"
)]
fn lab(lightness: f32, a: f32, b: f32, range: [f32; 4]) -> Color {
    let bound = |value: f32, low: f32, high: f32| {
        if value.is_nan() {
            low
        } else {
            value.clamp(low.min(high), low.max(high))
        }
    };
    let lightness = bound(lightness, 0.0, 100.0);
    let a = bound(a, range[0], range[1]);
    let b = bound(b, range[2], range[3]);

    let m = (lightness + 16.0) / 116.0;
    let l = m + a / 500.0;
    let n = m - b / 200.0;

    // PDF's default white point for Lab is D50, which is already the connection space's,
    // so no adaptation stands between this and the matrix.
    xyz_d50_to_srgb([D50[0] * expand(l), D50[1] * expand(m), D50[2] * expand(n)])
}

/// Reads the colourant names of a `Separation` or `DeviceN` space.
///
/// ISO 32000-2 §8.6.6.4 gives a `Separation` one `name` object; §8.6.6.5 gives a `DeviceN`
/// an array of them, whose length is also how many operands `scn` takes. The returned vector
/// therefore keeps the array's *arity*: an entry that is not a name becomes empty rather than
/// disappearing, so a malformed name cannot silently change the number of components.
fn colourant_names(document: &Document, object: Option<&Object>, single: bool) -> Vec<Vec<u8>> {
    let name_of = |item: &Object| {
        document
            .resolve(item)
            .as_name()
            .map(|name| name.as_bytes().to_vec())
            .unwrap_or_default()
    };
    let Some(object) = object else {
        return Vec::new();
    };
    if single {
        return vec![name_of(object)];
    }
    document
        .resolve(object)
        .as_array()
        .map(|items| items.iter().map(name_of).collect())
        .unwrap_or_default()
}

/// Reads a fixed-length array of numbers from a colour space dictionary.
fn numbers<const N: usize>(
    document: &Document,
    dict: Option<&Dictionary>,
    key: &'static str,
) -> Option<[f32; N]> {
    let array = document.get_key(dict?, key);
    let values: Vec<f32> = array
        .as_array()?
        .iter()
        .filter_map(|item| document.resolve(item).as_number().map(narrow))
        .collect();
    <[f32; N]>::try_from(values.as_slice()).ok()
}

/// Reads a `WhitePoint` entry, which Tables 62 and 63 both make required.
///
/// A dictionary without one is not a CIE-based space at all, and there is nothing in the
/// file to recover the intent from. D50 is the substitute because it is the connection
/// space's own white, which makes the adaptation stage vanish and leaves the space's
/// `Gamma` and `Matrix` — the parts the document *did* state — doing exactly what they say.
fn white_point(document: &Document, dict: Option<&Dictionary>) -> [f32; 3] {
    numbers(document, dict, "WhitePoint")
        // "The numbers X_W and Z_W shall be positive, and Y_W shall be equal to 1.0."
        // A white point violating that would divide the adaptation by zero or invert it.
        .filter(|white| white[0] > 0.0 && white[1] > 0.0 && white[2] > 0.0)
        .unwrap_or(D50)
}

/// The sRGB transfer function.
fn gamma(value: f32) -> f32 {
    let value = channel(value);
    if value <= 0.003_130_8 {
        channel(value * 12.92)
    } else {
        channel(1.055 * value.powf(1.0 / 2.4) - 0.055)
    }
}

fn narrow(value: f64) -> f32 {
    #[expect(
        clippy::cast_possible_truncation,
        reason = "a colour bound outside f32's range is not a bound"
    )]
    {
        value as f32
    }
}

#[cfg(test)]
#[expect(
    clippy::float_cmp,
    reason = "test code: these conversions are exact by definition, so an approximate \
              comparison would hide the very drift the test exists to catch"
)]
mod tests {
    use pdf_syntax::{Dictionary, Document, Object};

    use super::ColourSpace;

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
                Object::Name(pdf_syntax::Name::new(b"CalCMYK".as_slice())),
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

    /// The eight-bit sRGB a colour converts to, for comparing against measured output.
    fn bytes(colour: pdf_render::Color) -> (u8, u8, u8) {
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
        let white = super::lab(100.0, 0.0, 0.0, range);
        assert!(
            white.r > 0.99 && white.g > 0.99 && white.b > 0.99,
            "{white:?}"
        );
        let black = super::lab(0.0, 0.0, 0.0, range);
        assert!(
            black.r < 0.01 && black.g < 0.01 && black.b < 0.01,
            "{black:?}"
        );
        // A strongly positive a* axis is red.
        let red = super::lab(54.0, 81.0, 70.0, range);
        assert!(red.r > red.g && red.r > red.b, "{red:?}");
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
    /// D50 = [0.96422, 1.0, 0.82521], four digits finer than the value PDF states for `Lab`
    /// and this module therefore uses.
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
            let adapted = super::adapt(basis, super::D50, D65);
            let converted = super::transform(&SRGB_FROM_XYZ_D65, adapted);
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
                super::gamma(value.powf(2.2))
            })));
            assert_eq!(bytes(space.to_rgb(&input)), expected, "{input:?}");
        }
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

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
//! `DeviceCMYK` uses the naive conversion, which is what the specification describes for a
//! device with no calibration. `CalRGB` and `CalGray` are treated as their device
//! equivalents, and `ICCBased` as the device space with the same component count — the
//! specification explicitly permits the latter as a fallback, and the alternative is an
//! ICC engine.
//!
//! `Lab` is converted properly, because it is not close to anything else and a wrong
//! answer there is a visibly wrong colour rather than a slightly-off one.

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
            b"DeviceGray" | b"G" | b"CalGray" => Some(Self::Gray),
            b"DeviceRGB" | b"RGB" | b"CalRGB" => Some(Self::Rgb),
            b"DeviceCMYK" | b"CMYK" => Some(Self::Cmyk),
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
                let inputs = if is_separation {
                    1
                } else {
                    items
                        .get(1)
                        .map(|item| document.resolve(item))
                        .and_then(|item| item.as_array().map(<[Object]>::len))?
                };
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
        Some(Self::Indexed {
            base: Box::new(base),
            lookup: bytes.iter().map(|byte| f32::from(*byte) / 255.0).collect(),
            high,
        })
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
            Self::Gray | Self::Indexed { .. } => 1,
            Self::Icc { profile } => profile.channels(),
            // A pattern is named, not given as components; where an uncoloured one takes
            // a colour, that colour belongs to the underlying space.
            Self::Pattern { base } => base.as_ref().map_or(1, |base| base.components()),
            Self::Rgb | Self::Lab { .. } => 3,
            Self::Cmyk => 4,
            Self::Separation { inputs, .. } => *inputs,
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
/// # This is a choice, because the specification does not make one
///
/// ISO 32000-2 §8.6.4.4 defines `DeviceCMYK` components as "concentrations of process
/// colourants" and gives **no** conversion to any other space. §8.6.5.7 NOTE 3 says
/// outright that nothing in PDF describes the output device's calibration. The spec is not
/// silent by omission here — it is telling us the question has no answer in the abstract:
/// what a `DeviceCMYK` colour looks like is a property of a press, and a press is not
/// something a PDF describes.
///
/// So this table cannot be derived, and nothing here should be read as claiming it was.
/// What the specification does say is *which* press to ask, and it names three sources in
/// order — `/DefaultCMYK` (§8.6.5.6, "shall be used"), an output intent's
/// `/DestOutputProfile` (§14.11.5, §8.6.5.7 NOTE 3), and an `ICCBased` space naming the
/// profile directly. All three are implemented and all three win over this table. It is
/// reached only when the document names no press at all, and then some press must be
/// assumed to put anything on screen.
///
/// The press assumed is standard process inks, at their published sRGB appearances:
/// `#00AEEF` cyan, `#EC008C` magenta, `#FFF200` yellow, `#231F20` black, with the
/// overprints that follow from them. Written as eight-bit values because that is the
/// precision at which ink appearances are published.
///
/// The naive `1 - min(1, c + k)` this replaced is a different matter: it is off by up to
/// 115 of 255 at these corners and renders process magenta as `#FF00FF`, a colour no ink
/// produces. That formula is not a coarser answer to the question — it is an answer to a
/// question about additive light, asked of subtractive ink.
///
/// Other readers land within a level of these numbers. That is evidence that assuming
/// standard process inks is the conventional reading of an unspecified case — not evidence
/// that these numbers are correct, because for an unspecified case there is nothing for
/// them to be correct against.
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

    // PDF's default white point for Lab is D50.
    let (xw, yw, zw) = (0.964_2, 1.0, 0.824_9);
    let (x, y, z) = (xw * expand(l), yw * expand(m), zw * expand(n));

    // XYZ (D50) to linear sRGB, Bradford-adapted.
    let r = 3.134_136 * x - 1.617_036 * y - 0.490_662 * z;
    let g = -0.978_755 * x + 1.916_142 * y + 0.033_454 * z;
    let bl = 0.071_95 * x - 0.228_988 * y + 1.405_386 * z;

    Color::rgb(gamma(r), gamma(g), gamma(bl))
}

/// The sRGB transfer function, for callers outside this module.
pub(crate) fn srgb_gamma(value: f32) -> f32 {
    gamma(value)
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
    use super::ColourSpace;

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
    /// The specification defines no `DeviceCMYK` conversion at all, so this pins a
    /// deliberate choice rather than a derivation — see `CMYK_CORNERS` for why the choice
    /// exists and what overrides it. What the test defends is that the choice stays made:
    /// these sixteen values are the whole of it, and a conversion that moves any of them
    /// has stopped assuming the press we said we were assuming.
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

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
            // An ICC profile is described by how many components it takes; without an ICC
            // engine the device space with that many components is the specification's own
            // stated fallback.
            b"ICCBased" => {
                let stream = items.get(1).map(|item| document.resolve(item))?;
                let stream = stream.as_stream()?;
                match document.get_key(&stream.dict, "N").as_integer() {
                    Some(1) => Some(Self::Gray),
                    Some(4) => Some(Self::Cmyk),
                    // Three is the common case, and an absent or nonsensical `/N` is far
                    // more likely to be RGB than anything else.
                    _ => Some(Self::Rgb),
                }
            }
            b"Indexed" | b"I" => {
                let base =
                    Self::parse_at(document, items.get(1)?, resources, depth.saturating_add(1))?;
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
                let lookup = bytes.iter().map(|byte| f32::from(*byte) / 255.0).collect();
                Some(Self::Indexed {
                    base: Box::new(base),
                    lookup,
                    high,
                })
            }
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

    /// Resolves a space named directly, looking it up in the resources if need be.
    fn by_name(
        document: &Document,
        name: &str,
        resources: &Dictionary,
        depth: usize,
    ) -> Option<Self> {
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

    /// How many numbers a colour in this space takes.
    #[must_use]
    pub fn components(&self) -> usize {
        match self {
            Self::Gray | Self::Indexed { .. } => 1,
            // A pattern is named, not given as components; where an uncoloured one takes
            // a colour, that colour belongs to the underlying space.
            Self::Pattern { base } => base.as_ref().map_or(1, |base| base.components()),
            Self::Rgb | Self::Lab { .. } => 3,
            Self::Cmyk => 4,
            Self::Separation { inputs, .. } => *inputs,
        }
    }

    /// Converts a colour in this space to RGB.
    #[must_use]
    pub fn to_rgb(&self, values: &[f32]) -> Color {
        self.to_rgb_at(values, 0)
    }

    fn to_rgb_at(&self, values: &[f32], depth: usize) -> Color {
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
                base.to_rgb_at(&slice, depth.saturating_add(1))
            }
            Self::Separation {
                alternate,
                transform,
                ..
            } => {
                let converted = transform.eval(values);
                alternate.to_rgb_at(&converted, depth.saturating_add(1))
            }
            // A pattern has no colour of its own. Where it names an underlying space, an
            // uncoloured pattern's colour is in that; otherwise there is nothing to say.
            Self::Pattern { base } => base.as_ref().map_or(Color::BLACK, |base| {
                base.to_rgb_at(values, depth.saturating_add(1))
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

/// The specification's uncalibrated CMYK to RGB conversion.
fn cmyk(c: f32, m: f32, y: f32, k: f32) -> Color {
    let (c, m, y, k) = (channel(c), channel(m), channel(y), channel(k));
    Color::rgb(
        channel(1.0 - (c + k).min(1.0)),
        channel(1.0 - (m + k).min(1.0)),
        channel(1.0 - (y + k).min(1.0)),
    )
}

/// The inverse of the L*a*b* companding function.
fn expand(value: f32) -> f32 {
    if value >= 6.0 / 29.0 {
        value * value * value
    } else {
        108.0 / 841.0 * (value - 4.0 / 29.0)
    }
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

    #[test]
    fn device_spaces_convert_as_the_specification_defines() {
        assert_eq!(ColourSpace::Gray.to_rgb(&[0.5]).r, 0.5);
        let rgb = ColourSpace::Rgb.to_rgb(&[0.1, 0.2, 0.3]);
        assert_eq!((rgb.r, rgb.g, rgb.b), (0.1, 0.2, 0.3));
        // Full black ink is black; no ink at all is white.
        let black = ColourSpace::Cmyk.to_rgb(&[0.0, 0.0, 0.0, 1.0]);
        assert_eq!((black.r, black.g, black.b), (0.0, 0.0, 0.0));
        let white = ColourSpace::Cmyk.to_rgb(&[0.0, 0.0, 0.0, 0.0]);
        assert_eq!((white.r, white.g, white.b), (1.0, 1.0, 1.0));
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

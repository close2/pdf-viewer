//! Decoding image `XObject`s into RGBA8 samples.
//!
//! Everything is normalised to straight-alpha RGBA8, so neither rasteriser backend needs
//! to know about PDF colour spaces or bit depths — the same reason colours are resolved
//! before they reach a backend.
//!
//! # Where the samples come from
//!
//! Three routes, and which one a stream takes is decided by the codec left on the end of
//! its filter chain:
//!
//! - No codec: the bytes *are* samples, and [`unpack`] reads them at the declared depth.
//! - `DCTDecode`: `zune-jpeg`, in this process.
//! - `JBIG2Decode` and `JPXDecode`: [`pdf_sandbox`], which by default decodes them in a
//!   confined worker process. The samples come back in the form the filter is defined to
//!   deliver — packed bits for JBIG2, eight-bit components for JPEG 2000 — and everything
//!   after that is the same code every other image goes through.
//!
//! # Two things about the JPEG 2000 route that are easy to get backwards
//!
//! ISO 32000-2 §7.4.9 inverts the usual relationship between an image dictionary and its
//! data. `/ColorSpace` is *optional*, and where it is absent the codestream's own colour
//! space governs; where it is present the codestream's is ignored. `/BitsPerComponent` is
//! ignored either way. And `/Decode` is ignored unless the image is a mask. A reader that
//! treats the dictionary as authoritative for all three, as it is for every other filter,
//! renders JPEG 2000 images in the wrong colours and cannot tell.
//!
//! An image this module cannot decode returns an error naming why, and the interpreter
//! reports it. Drawing a grey box in its place would be worse: the page would look
//! finished and be wrong.

use std::sync::Arc;

use pdf_render::Image;
use pdf_sandbox::{Decoded, Request};
use pdf_syntax::{Dictionary, Document, ImageStream, Object, Stream};

/// Largest image this will decode, in samples.
///
/// 2^28 samples is a gigabyte of RGBA. Image dimensions come from the document, so an
/// unbounded allocation here is a denial of service with a two-line file.
const MAX_SAMPLES: u64 = 1 << 28;

/// Why an image could not be decoded.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum ImageError {
    /// The filter is an image codec this crate does not implement.
    #[error("{filter} is not implemented")]
    UnsupportedFilter {
        /// The filter name.
        filter: String,
    },
    /// The colour space is one this crate cannot convert.
    #[error("colour space {space} is not supported")]
    UnsupportedColourSpace {
        /// A description of the space.
        space: String,
    },
    /// The bit depth is one this crate does not unpack.
    #[error("{bits} bits per component is not supported")]
    UnsupportedDepth {
        /// The declared depth.
        bits: u32,
    },
    /// The image is larger than [`MAX_SAMPLES`].
    #[error("image of {samples} samples exceeds the limit of {MAX_SAMPLES}")]
    TooLarge {
        /// How many samples were requested.
        samples: u64,
    },
    /// The dictionary is missing something required, or the data is short.
    #[error("malformed image: {detail}")]
    Malformed {
        /// What was wrong.
        detail: String,
    },
    /// A codec that runs outside this process could not decode the image.
    ///
    /// Carries the sandbox's own words, which distinguish the two cases that matter: the
    /// decoder refused the data, or the worker itself could not be reached or died. The
    /// second is a deployment problem and reads like one.
    #[error("{detail}")]
    Sandboxed {
        /// What the sandbox reported.
        detail: String,
    },
}

/// How an image's samples are interpreted.
#[derive(Debug, Clone)]
enum ColourSpace {
    /// One component per sample.
    Gray,
    /// Three components per sample.
    Rgb,
    /// Four components per sample, subtractive.
    Cmyk,
    /// A stencil mask: one bit per sample, painted in the current fill colour.
    Mask,
    /// Any other space, converted sample by sample through the colour module.
    ///
    /// A CIE-based space is *not* its device equivalent. ISO 32000-2 §8.6.5.2 and §8.6.5.3
    /// define `CalGray` and `CalRGB` in CIE terms, so a `Gamma 1` grey is a linear luminance
    /// and writing it into an sRGB raster unchanged renders the image far too dark.
    /// `Indexed`, `Separation` and `DeviceN` are not device spaces either: each is a function
    /// of its sample rather than a colour. Converting per sample costs what the `DeviceCMYK`
    /// arm already costs and buys the same thing: one answer for a colour, whether it reached
    /// the page as a fill or as an image.
    Resolved(crate::colour::ColourSpace),
}

impl ColourSpace {
    fn components(&self) -> usize {
        match self {
            Self::Gray | Self::Mask => 1,
            Self::Rgb => 3,
            Self::Cmyk => 4,
            Self::Resolved(space) => space.components(),
        }
    }

    /// Table 88's default `/Decode` pair for one component, at this bit depth.
    ///
    /// Every device space's components run from 0.0 to 1.0, so their default pair is that
    /// range; the two spaces where it is not — `Lab`, whose lightness is a percentage and
    /// whose chromatic axes take the space's own `/Range`, and `Indexed`, whose default
    /// passes an index through unchanged — answer for themselves.
    fn default_decode(&self, component: usize, bits: u32) -> (f32, f32) {
        match self {
            Self::Resolved(space) => space.default_decode(component, bits),
            _ => (0.0, 1.0),
        }
    }

    /// The range a decoded value of one component is permitted to take.
    ///
    /// §8.9.5.2's closing sentence: "If an output value is not permitted for a component, it
    /// shall be adjusted to the nearest allowed value." A `/Decode` array may state any two
    /// numbers — the clause says so one paragraph earlier — so the map's output is clamped
    /// here rather than trusted.
    fn permitted(&self, component: usize) -> (f32, f32) {
        match self {
            Self::Resolved(space) => space.component_range(component),
            _ => (0.0, 1.0),
        }
    }
}

/// §8.9.5.2's `/Decode` array: what an integer sample means as a colour component.
///
/// > Samples with a value of 0 shall be mapped to D min … those with intermediate values
/// > shall be mapped linearly between D min and D max
///
/// Held as one table per component rather than as the clause's formula. Every input the
/// formula takes — the pair, the bit depth, the component's permitted range — is fixed by
/// the image dictionary before a sample is read, and its domain has at most 2^n points,
/// where n is 1 or 8 here. So the map is evaluated at most 256 times per component per
/// image instead of once per component per *pixel*, and the unpacker's arms become a lookup
/// that no longer has to know what a `/Decode` array is.
struct Decode {
    /// `values[component][sample]`, in the colour space's own component values.
    values: Vec<Vec<f32>>,
    /// [`Self::values`] quantised, because a device space's components *are* eight-bit
    /// channels and the arms that produce one would otherwise clamp, scale and round per
    /// pixel. 256 entries per component against a photograph's millions, and the benchmark
    /// that says it earns its place: interpreting `issue19971.pdf`'s 2500x1364 `DeviceRGB`
    /// photograph costs 162.40 G instructions with this table and 166.35 G by quantising per
    /// pixel, against 161.54 G before `/Decode` was applied at all. So it takes the whole
    /// clause's cost on the corpus's largest image from +2.98% to +0.54%, and to nothing
    /// measurable on a page carrying no image.
    channels: Vec<Vec<u8>>,
}

impl Decode {
    /// Reads `/Decode`, or Table 88's default for `space` where the dictionary states none.
    fn read(document: &Document, dict: &Dictionary, space: &ColourSpace, bits: u32) -> Self {
        let stated: Vec<f32> = document
            .get_key(dict, "Decode")
            .as_array()
            .map(|items| {
                items
                    .iter()
                    .map(|item| document.resolve(item))
                    .filter_map(|item| item.as_number())
                    .map(|value| {
                        #[expect(
                            clippy::cast_possible_truncation,
                            reason = "a component value beyond f32 is clamped to its \
                                      permitted range either way"
                        )]
                        {
                            value as f32
                        }
                    })
                    .collect()
            })
            .unwrap_or_default();

        // 2^n − 1, the largest sample the depth can carry, and the formula's x max.
        let max = (1u32 << bits.min(16)).saturating_sub(1);
        let span = f32::from(u16::try_from(max.max(1)).unwrap_or(u16::MAX));

        let mut values = Vec::with_capacity(space.components());
        for component in 0..space.components() {
            let at = component.saturating_mul(2);
            // A `/Decode` array "shall contain one pair of numbers for each component"; one
            // that does not is honoured as far as it goes, since the alternative is
            // discarding a pair the producer did state.
            let (low, high) = match (stated.get(at), stated.get(at.saturating_add(1))) {
                (Some(low), Some(high)) => (*low, *high),
                _ => space.default_decode(component, bits),
            };
            let (floor, ceiling) = space.permitted(component);
            let table = (0..=max)
                .map(|raw| {
                    let sample = f32::from(u16::try_from(raw).unwrap_or(u16::MAX));
                    // D min + x × (D max − D min) ÷ (2^n − 1), with the multiplication
                    // before the division: an `Indexed` space's default pair is
                    // `[0 2^n − 1]`, which the clause's NOTE 2 says "ensures that component
                    // values … are passed through unchanged", and `raw ÷ 255 × 255` is
                    // 253.99998 for a sample of 254 while `raw × 255 ÷ 255` is 254.
                    let value = (high - low).mul_add(sample, low * span) / span;
                    value.clamp(floor.min(ceiling), ceiling.max(floor))
                })
                .collect::<Vec<f32>>();
            values.push(table);
        }

        let channels = values
            .iter()
            .map(|table| table.iter().copied().map(channel).collect())
            .collect();
        Self { values, channels }
    }

    /// What sample `raw` of component `component` means.
    fn value(&self, component: usize, raw: usize) -> f32 {
        self.values
            .get(component)
            .and_then(|table| table.get(raw))
            .copied()
            .unwrap_or(0.0)
    }

    /// The same, quantised to an eight-bit channel.
    fn channel(&self, component: usize, raw: usize) -> u8 {
        self.channels
            .get(component)
            .and_then(|table| table.get(raw))
            .copied()
            .unwrap_or(0)
    }

    /// Whether the map is the identity on eight-bit device channels.
    ///
    /// The one question the `DCTDecode` route asks, because that route has already turned
    /// the codestream into eight-bit channels and only has to know whether to remap them.
    fn is_identity(&self) -> bool {
        self.channels.iter().all(|table| {
            table.len() == 256
                && table
                    .iter()
                    .enumerate()
                    .all(|(raw, value)| u8::try_from(raw).is_ok_and(|raw| raw == *value))
        })
    }
}

/// Decodes an image `XObject`.
///
/// `fill` is the current fill colour, used for a stencil mask, which paints the *current*
/// colour through its set bits rather than carrying colour of its own.
///
/// # Errors
///
/// See [`ImageError`].
pub fn decode(
    document: &Document,
    stream: &Stream,
    fill: pdf_render::Color,
) -> Result<Image, ImageError> {
    let dict = &stream.dict;

    let width = positive_integer(document, dict, "Width")?;
    let height = positive_integer(document, dict, "Height")?;

    let samples = u64::from(width).saturating_mul(u64::from(height));
    if samples > MAX_SAMPLES {
        return Err(ImageError::TooLarge { samples });
    }

    // An image mask is one bit per sample, painted in the fill colour, and carries no
    // colour space of its own.
    let is_mask = matches!(document.get_key(dict, "ImageMask"), Object::Boolean(true));

    // §8.9.6.3 and §8.9.6.4: `/Mask` is either a second image naming the areas of this one
    // that are painted, or a range of colours that are not. Read before the samples, because
    // the colour-key form is a test *on* the samples and has to travel into the unpacker.
    let mask = mask_entry(document, dict);
    let colour_key = match &mask {
        MaskEntry::ColourKey(ranges) => Some(ranges.as_slice()),
        _ => None,
    };

    // Every filter before the codec has run; the codec, if there is one, has not.
    let source = document
        .image_stream(stream)
        .ok_or_else(|| ImageError::Malformed {
            detail: "stream did not decode".to_owned(),
        })?;

    let (rgba, opacity_came_with_the_samples) = match source.codec.as_deref() {
        Some(b"DCTDecode" | b"DCT") => {
            let mut rgba = decode_jpeg(&source.data, width, height)?;
            apply_decode_to_channels(document, dict, &mut rgba);
            (rgba, false)
        }
        Some(b"JBIG2Decode") => (
            decode_jbig2(document, dict, &source, width, height, is_mask, fill)?,
            false,
        ),
        Some(b"JPXDecode") => decode_jpx(document, dict, &source, width, height, is_mask, fill)?,
        Some(b"CCITTFaxDecode" | b"CCF") => (
            decode_ccitt(document, dict, &source, width, height, is_mask, fill)?,
            false,
        ),
        Some(other) => {
            return Err(ImageError::UnsupportedFilter {
                filter: String::from_utf8_lossy(other).into_owned(),
            });
        }
        None => {
            let bits = if is_mask {
                1
            } else {
                u32::try_from(
                    document
                        .get_key(dict, "BitsPerComponent")
                        .as_integer()
                        .unwrap_or(8),
                )
                .unwrap_or(8)
            };
            let space = if is_mask {
                ColourSpace::Mask
            } else {
                colour_space(document, dict)?
            };
            let decode = Decode::read(document, dict, &space, bits);
            (
                unpack(
                    &source.data,
                    width,
                    height,
                    &Samples {
                        bits,
                        space: &space,
                        decode: &decode,
                        colour_key,
                        fill,
                    },
                )?,
                false,
            )
        }
    };

    let image = Image {
        width,
        height,
        data: Arc::from(rgba.as_slice()),
        // §8.9.5.3, and Table 87's default of false. The entry is a hint about what to do
        // when the image is magnified, so it travels with the samples and the backends
        // decide what to make of it.
        interpolate: matches!(document.get_key(dict, "Interpolate"), Object::Boolean(true)),
    };
    let image = if opacity_came_with_the_samples {
        // §7.4.9 and §11.6.5.2: a non-zero `/SMaskInData` means the opacity travelled with
        // the image samples, `/SMask` "shall not be present", and the embedded mask
        // overrides any that is. Applying one on top would multiply two alphas together.
        image
    } else {
        // Applied last so a soft mask cannot resurrect an inconsistent buffer.
        apply_soft_mask(document, dict, image)
    };

    // §11.6.4.3 makes the two mutually exclusive — an `/SMask` "shall override any explicit
    // or colour key mask" — so `mask_entry` has already returned [`MaskEntry::Overridden`]
    // for anything reached here after a soft mask was applied, and this arm runs only where
    // there was none. The sequence is therefore an ordering of two things that never both
    // happen, kept in the order the clauses rank them.
    match &mask {
        MaskEntry::Explicit(stencil) => apply_explicit_mask(document, &image, stencil),
        _ => Ok(image),
    }
}

/// Reads a required positive integer.
fn positive_integer(
    document: &Document,
    dict: &Dictionary,
    key: &'static str,
) -> Result<u32, ImageError> {
    document
        .get_key(dict, key)
        .as_integer()
        .and_then(|value| u32::try_from(value).ok())
        .filter(|value| *value > 0)
        .ok_or(ImageError::Malformed {
            detail: format!("missing or invalid /{key}"),
        })
}

/// Determines the colour space, reduced to what the sample unpacker handles.
fn colour_space(document: &Document, dict: &Dictionary) -> Result<ColourSpace, ImageError> {
    let space = document.get_key(dict, "ColorSpace");
    match &space {
        Object::Name(name) => match name.as_bytes() {
            b"DeviceGray" | b"G" | b"CalGray" => Ok(ColourSpace::Gray),
            b"DeviceRGB" | b"RGB" | b"CalRGB" => Ok(ColourSpace::Rgb),
            b"DeviceCMYK" | b"CMYK" => Ok(ColourSpace::Cmyk),
            other => Err(ImageError::UnsupportedColourSpace {
                space: String::from_utf8_lossy(other).into_owned(),
            }),
        },
        Object::Array(items) => {
            let family = items
                .first()
                .map(|item| document.resolve(item))
                .and_then(|item| item.as_name().map(|name| name.as_bytes().to_vec()))
                .unwrap_or_default();
            match family.as_slice() {
                // An ICC profile's component count tells us how to unpack even though the
                // profile itself is not applied — which is an approximation, and the
                // honest one: the alternative is refusing most real images.
                b"ICCBased" => {
                    let count = items
                        .get(1)
                        .map(|item| document.resolve(item))
                        .and_then(|item| item.as_dict().cloned())
                        .and_then(|inner| document.get_key(&inner, "N").as_integer())
                        .unwrap_or(3);
                    match count {
                        1 => Ok(ColourSpace::Gray),
                        4 => Ok(ColourSpace::Cmyk),
                        _ => Ok(ColourSpace::Rgb),
                    }
                }
                // §8.9.5.1 Table 87, of `/ColorSpace`: "it can be any type of colour space
                // except Pattern". A pattern carries no colour of its own, so a sample in
                // one names nothing that could be unpacked.
                b"Pattern" => Err(ImageError::UnsupportedColourSpace {
                    space: "Pattern, which Table 87 excludes".to_owned(),
                }),
                // Everything else the colour module reads, converted per sample: `Indexed`,
                // `Separation`, `DeviceN`, `Lab` and the two Cal spaces. Each is a function
                // of its samples rather than a colour, so there is nothing to approximate
                // and one place — `ColourSpace::to_rgb` — that decides what they mean.
                _ => crate::colour::ColourSpace::parse(
                    document,
                    &space,
                    // An image's colour space is written out in full: `/CS` naming a
                    // resource is an inline image's spelling and `crate::inline_image` has
                    // already resolved it, so there is no resource dictionary left to need.
                    &Dictionary::new(),
                )
                .map(ColourSpace::Resolved)
                .ok_or_else(|| ImageError::UnsupportedColourSpace {
                    space: String::from_utf8_lossy(&family).into_owned(),
                }),
            }
        }
        _ => Err(ImageError::UnsupportedColourSpace {
            space: "absent".to_owned(),
        }),
    }
}

/// How a row of raw bytes becomes colour: the layout, and what a value means.
///
/// Grouped rather than passed one at a time because all five are settled by the image
/// dictionary before a byte is read, and because the four routes that reach [`unpack`]
/// differ only in where the bytes came from.
struct Samples<'a> {
    /// Bits per component.
    bits: u32,
    /// What a component means.
    space: &'a ColourSpace,
    /// What a sample means, per component (§8.9.5.2).
    decode: &'a Decode,
    /// §8.9.6.4's colour-key ranges, one per component, in raw sample values.
    ///
    /// The clause's test is on the samples "before decoding with the Decode array", which is
    /// why this travels into the unpacker rather than filtering the RGBA it produces: after
    /// conversion the component values are gone, and for every space but the device ones
    /// they were never in the raster to begin with.
    colour_key: Option<&'a [(u32, u32)]>,
    /// The current fill colour, which a stencil paints through the bits that mark the page.
    fill: pdf_render::Color,
}

/// Unpacks raw samples into RGBA8.
fn unpack(data: &[u8], width: u32, height: u32, samples: &Samples) -> Result<Vec<u8>, ImageError> {
    let &Samples {
        bits,
        space,
        decode,
        colour_key,
        fill: _,
    } = samples;
    if !matches!(bits, 1 | 8) {
        // 2, 4 and 16 are legal and do occur. Refusing them is honest; guessing would
        // shift every sample.
        return Err(ImageError::UnsupportedDepth { bits });
    }

    let components = space.components();
    // A space taking one component has at most 2^bits colours, so every one of them can be
    // converted once instead of once per sample. This is exact rather than an approximation
    // — the samples are integers and the table holds every value one can take — and it is
    // what keeps an `Indexed` image whose base is an ICC profile, or a `Separation` whose
    // tint transform is a PostScript program, off the per-sample path.
    //
    // Measured over the sixteen corpus documents that draw a space this arm reaches:
    // 1.03 s of rendering without it, 330 ms with. `issue9940.pdf` is the extreme — 620 ms
    // to 42 ms — because its images are `Indexed` over a `DeviceN` whose tint transform is a
    // PostScript calculator, which was being run once per sample rather than once per entry
    // of a 256-entry table.
    let palette = match space {
        ColourSpace::Resolved(resolved) if resolved.components() == 1 => {
            Some(palette(resolved, bits, decode))
        }
        _ => None,
    };
    let width_usize = width as usize;
    let height_usize = height as usize;
    // Each row starts on a byte boundary.
    let row_bits = width_usize
        .saturating_mul(components)
        .saturating_mul(bits as usize);
    let row_bytes = row_bits.saturating_add(7) / 8;

    let mut out = Vec::with_capacity(width_usize.saturating_mul(height_usize).saturating_mul(4));

    for y in 0..height_usize {
        let row = data
            .get(y.saturating_mul(row_bytes)..y.saturating_mul(row_bytes).saturating_add(row_bytes))
            .unwrap_or_default();

        for x in 0..width_usize {
            // §8.9.6.4: "An image sample shall be masked (not painted) if all of its colour
            // components before decoding … fall within the specified ranges". A masked
            // sample keeps its position and loses its opacity; nothing else about it is
            // read, which is why this stands before the conversion rather than after it.
            if colour_key.is_some_and(|ranges| colour_key_masks(row, x, bits, components, ranges)) {
                out.extend_from_slice(&[0, 0, 0, 0]);
                continue;
            }
            out.extend_from_slice(&sample_rgba(samples, palette.as_deref(), row, x));
        }
    }

    Ok(out)
}

/// Whether §8.9.6.4's ranges cover every component of one sample.
///
/// The values compared are the raw integers the filter delivered, which is what the clause
/// means by "before decoding": `min i ≤ c i ≤ max i for all 1 ≤ i ≤ n`. `ranges` has one
/// entry per component, which [`mask_entry`] checked against the colour space before the
/// samples were read.
fn colour_key_masks(
    row: &[u8],
    x: usize,
    bits: u32,
    components: usize,
    ranges: &[(u32, u32)],
) -> bool {
    let at = x.saturating_mul(components);
    ranges.iter().enumerate().all(|(offset, (low, high))| {
        let index = at.saturating_add(offset);
        let raw = if bits == 1 {
            u32::from(sample_bit(row, index))
        } else {
            u32::from(row.get(index).copied().unwrap_or(0))
        };
        raw >= *low && raw <= *high
    })
}

/// One pixel, as straight-alpha RGBA8.
///
/// Split out of [`unpack`] so that the loop above is the *layout* — rows, byte boundaries,
/// bit packing — and this is the colour, which is the only part that depends on the space.
fn sample_rgba(
    samples: &Samples,
    palette: Option<&[pdf_render::Color]>,
    row: &[u8],
    x: usize,
) -> [u8; 4] {
    let &Samples {
        bits,
        space,
        decode,
        colour_key: _,
        fill,
    } = samples;
    let opaque =
        |colour: pdf_render::Color| [channel(colour.r), channel(colour.g), channel(colour.b), 255];
    match (space, bits) {
        (ColourSpace::Mask, _) => {
            // §8.9.6.2: a sample decoding to 0 marks the page, so the default `[0 1]` paints
            // through the *clear* bits and `[1 0]` reverses that. Both are the same lookup.
            let bit = usize::from(sample_bit(row, x));
            let paint = decode.value(0, bit) < 0.5;
            if paint {
                [
                    channel(fill.r),
                    channel(fill.g),
                    channel(fill.b),
                    channel(fill.a),
                ]
            } else {
                [0, 0, 0, 0]
            }
        }
        (ColourSpace::Gray, 1) => {
            let value = decode.channel(0, usize::from(sample_bit(row, x)));
            [value, value, value, 255]
        }
        (ColourSpace::Gray, _) => {
            let value = decode.channel(0, usize::from(row.get(x).copied().unwrap_or(0)));
            [value, value, value, 255]
        }
        (ColourSpace::Rgb, _) => {
            let at = x.saturating_mul(3);
            let read = |component: usize| {
                decode.channel(
                    component,
                    usize::from(row.get(at.saturating_add(component)).copied().unwrap_or(0)),
                )
            };
            [read(0), read(1), read(2), 255]
        }
        (ColourSpace::Cmyk, _) => {
            let at = x.saturating_mul(4);
            let read = |offset: usize| {
                decode.value(
                    offset,
                    usize::from(row.get(at.saturating_add(offset)).copied().unwrap_or(0)),
                )
            };
            // The *same* conversion a `k` operator or an `scn` in DeviceCMYK gets. Having a
            // second one here is how the same colour came to render differently depending on
            // whether it was drawn as a fill or as an image, which is exactly the bug this
            // crate should not have.
            opaque(crate::colour::ColourSpace::Cmyk.to_rgb(&[read(0), read(1), read(2), read(3)]))
        }
        (ColourSpace::Resolved(_), _) if palette.is_some() => {
            let sample = if bits == 1 {
                usize::from(sample_bit(row, x))
            } else {
                usize::from(row.get(x).copied().unwrap_or(0))
            };
            // The table already carries `/Decode`'s map, so the sample is the index and
            // nothing else happens to it here.
            opaque(
                palette
                    .and_then(|table| table.get(sample))
                    .copied()
                    .unwrap_or(pdf_render::Color::BLACK),
            )
        }
        (ColourSpace::Resolved(resolved), _) => {
            opaque(resolved_sample(resolved, row, x, bits, decode))
        }
    }
}

/// One pixel of a space the colour module resolves, converted through it.
///
/// Reached only for a space taking more than one component; the one-component spaces are a
/// table lookup, built by [`palette`].
fn resolved_sample(
    space: &crate::colour::ColourSpace,
    row: &[u8],
    x: usize,
    bits: u32,
    decode: &Decode,
) -> pdf_render::Color {
    let count = space.components();
    let at = x.saturating_mul(count);
    let values: Vec<f32> = (0..count)
        .map(|component| {
            let index = at.saturating_add(component);
            let raw = if bits == 1 {
                // One bit per *component*, so the components of a pixel are adjacent bits
                // rather than adjacent bytes.
                usize::from(sample_bit(row, index))
            } else {
                usize::from(row.get(index).copied().unwrap_or(0))
            };
            decode.value(component, raw)
        })
        .collect();
    space.to_rgb(&values)
}

/// Every colour a one-component space can produce at this bit depth, in sample order.
///
/// The `/Decode` map is baked in, so the table is indexed by the raw sample and the
/// per-pixel path does nothing but index it.
fn palette(
    space: &crate::colour::ColourSpace,
    bits: u32,
    decode: &Decode,
) -> Vec<pdf_render::Color> {
    let max = (1u32 << bits.min(8)).saturating_sub(1);
    (0..=max)
        .map(|raw| space.to_rgb(&[decode.value(0, raw as usize)]))
        .collect()
}

/// Reads bit `index` of a packed row, most significant bit first.
fn sample_bit(row: &[u8], index: usize) -> bool {
    let byte = row.get(index / 8).copied().unwrap_or(0);
    let shift = 7u32.saturating_sub(u32::try_from(index % 8).unwrap_or(0));
    (byte >> shift) & 1 == 1
}

fn channel(value: f32) -> u8 {
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "clamped to 0.0..=1.0 and scaled, so the conversion is exact"
    )]
    {
        (value.clamp(0.0, 1.0) * 255.0).round() as u8
    }
}

/// Decodes a JBIG2 image through the sandbox.
///
/// ISO 32000-2 §7.4.7. The stream carries the segments for one page; `/JBIG2Globals` in the
/// filter's own `/DecodeParms` (Table 12) carries the page-0 segments, which several images
/// may share — a scanned book's symbol dictionary is written once and referenced by every
/// page, which is most of why JBIG2 compresses text so well.
///
/// What comes back is packed one bit per pixel in `DeviceGray`'s sense, so it feeds
/// [`unpack`] exactly as any other 1-bit image does.
fn decode_jbig2(
    document: &Document,
    dict: &Dictionary,
    source: &ImageStream,
    width: u32,
    height: u32,
    is_mask: bool,
    fill: pdf_render::Color,
) -> Result<Vec<u8>, ImageError> {
    // A globals stream that will not decode is reported rather than skipped: without its
    // symbol dictionary the page would decode to blank or to nothing, and "the image is
    // empty" is a far worse answer than "the image needs a stream I could not read".
    let globals = match source
        .parms
        .as_ref()
        .map(|parms| document.get_key(parms, "JBIG2Globals"))
    {
        Some(Object::Stream(stream)) => Some(document.decoded_stream_data(&stream).ok_or_else(
            || ImageError::Malformed {
                detail: "/JBIG2Globals did not decode".to_owned(),
            },
        )?),
        _ => None,
    };

    let decoded = pdf_sandbox::decode(&Request::Jbig2 {
        data: &source.data,
        globals: globals.as_deref().unwrap_or_default(),
    })
    .map_err(|error| ImageError::Sandboxed {
        detail: error.to_string(),
    })?;
    let Decoded::Bilevel(bilevel) = decoded else {
        return Err(ImageError::Malformed {
            detail: "JBIG2 did not decode to bilevel samples".to_owned(),
        });
    };

    // The page information segment and the image dictionary both state the size. They must
    // agree, because the display list carries the dictionary's and the samples carry the
    // segment's.
    if (bilevel.width, bilevel.height) != (width, height) {
        return Err(ImageError::Malformed {
            detail: format!(
                "JBIG2 page is {}x{} but the dictionary says {width}x{height}",
                bilevel.width, bilevel.height
            ),
        });
    }

    let space = if is_mask {
        ColourSpace::Mask
    } else {
        colour_space(document, dict)?
    };
    let decode = Decode::read(document, dict, &space, 1);
    unpack(
        &bilevel.rows,
        width,
        height,
        &Samples {
            bits: 1,
            space: &space,
            decode: &decode,
            // §8.9.6.4's ranges are refused for a JBIG2 or CCITT image rather than applied
            // here; `mask_entry` says why, and reports it.
            colour_key: None,
            fill,
        },
    )
}

/// Decodes a CCITT fax-encoded image through the sandbox.
///
/// ISO 32000-2 §7.4.6. Everything the decoder needs is in Table 11's `/DecodeParms`, and
/// every entry there has a default, so this function's whole job is to turn a dictionary that
/// may be absent into a complete description — and to refuse the two cases where the
/// dictionary says something this cannot honour rather than quietly doing something else.
///
/// # The two refusals, and why they are refusals
///
/// **`/DamagedRowsBeforeError` above zero.** Table 11 defines it as the number of damaged rows
/// tolerated before an error, where tolerating one means "locating its end in the encoded data
/// by searching for an `EndOfLine` pattern and then substituting decoded data from the previous
/// row". That is error *concealment*, the decoder underneath has none, and a document that
/// asks for it is a document that expects damage — so drawing what came out anyway would be
/// drawing an image whose producer said in advance it might be wrong.
///
/// **`/Columns` disagreeing with `/Width`.** The filter delivers rows of `/Columns` samples
/// padded to a byte boundary; §8.9.5.1 says the image is `/Width` samples wide. Where the two
/// differ the row stride the unpacker assumes is not the stride the filter produced, and
/// nothing in ISO 32000-2 says which of the two statements wins. Reported rather than guessed.
fn decode_ccitt(
    document: &Document,
    dict: &Dictionary,
    source: &ImageStream,
    width: u32,
    height: u32,
    is_mask: bool,
    fill: pdf_render::Color,
) -> Result<Vec<u8>, ImageError> {
    let parms = source.parms.as_ref();
    let integer = |key: &str, default: i64| -> i64 {
        parms
            .map(|parms| document.get_key(parms, key))
            .and_then(|value| value.as_integer())
            .unwrap_or(default)
    };
    let flag = |key: &str, default: bool| -> bool {
        match parms.map(|parms| document.get_key(parms, key)) {
            Some(Object::Boolean(value)) => value,
            _ => default,
        }
    };

    let damaged_rows = integer("DamagedRowsBeforeError", 0);
    if damaged_rows > 0 {
        return Err(ImageError::UnsupportedFilter {
            filter: format!("CCITTFaxDecode with /DamagedRowsBeforeError {damaged_rows}"),
        });
    }

    let columns = u32::try_from(integer("Columns", 1728)).unwrap_or(0);
    if columns != width {
        return Err(ImageError::Malformed {
            detail: format!("CCITTFaxDecode /Columns {columns} is not the image's width {width}"),
        });
    }

    let parameters = pdf_sandbox::CcittParameters {
        k: i32::try_from(integer("K", 0)).unwrap_or(0),
        columns,
        // Table 11: a zero or absent `/Rows` means the height "is not predetermined", so the
        // image dictionary's `/Height` is the only statement of it left.
        rows: match u32::try_from(integer("Rows", 0)).unwrap_or(0) {
            0 => height,
            rows => rows,
        },
        end_of_line: flag("EndOfLine", false),
        encoded_byte_align: flag("EncodedByteAlign", false),
        end_of_block: flag("EndOfBlock", true),
        black_is_1: flag("BlackIs1", false),
    };

    let decoded = pdf_sandbox::decode(&Request::Ccitt {
        data: &source.data,
        parameters,
    })
    .map_err(|error| ImageError::Sandboxed {
        detail: error.to_string(),
    })?;
    let Decoded::Bilevel(bilevel) = decoded else {
        return Err(ImageError::Malformed {
            detail: "CCITTFaxDecode did not decode to bilevel samples".to_owned(),
        });
    };
    if (bilevel.width, bilevel.height) != (width, height) {
        return Err(ImageError::Malformed {
            detail: format!(
                "CCITTFaxDecode produced {}x{} but the dictionary says {width}x{height}",
                bilevel.width, bilevel.height
            ),
        });
    }

    let space = if is_mask {
        ColourSpace::Mask
    } else {
        colour_space(document, dict)?
    };
    let decode = Decode::read(document, dict, &space, 1);
    unpack(
        &bilevel.rows,
        width,
        height,
        &Samples {
            bits: 1,
            space: &space,
            decode: &decode,
            // §8.9.6.4's ranges are refused for a JBIG2 or CCITT image rather than applied
            // here; `mask_entry` says why, and reports it.
            colour_key: None,
            fill,
        },
    )
}

/// Decodes a JPEG 2000 image through the sandbox.
///
/// ISO 32000-2 §7.4.9. Returns the samples and whether an opacity channel came with them,
/// which decides whether an `/SMask` may still be applied.
fn decode_jpx(
    document: &Document,
    dict: &Dictionary,
    source: &ImageStream,
    width: u32,
    height: u32,
    is_mask: bool,
    fill: pdf_render::Color,
) -> Result<(Vec<u8>, bool), ImageError> {
    // Resolved before the request, because whether the codestream's own palette should be
    // applied depends on it: §7.4.9 gives `/ColorSpace` precedence over every colour
    // specification in the JPEG 2000 data, a palette included.
    let declared = document.get_key(dict, "ColorSpace");
    let declared_space = if matches!(declared, Object::Null) {
        None
    } else {
        Some(
            crate::colour::ColourSpace::parse(document, &declared, &Dictionary::new()).ok_or_else(
                || ImageError::UnsupportedColourSpace {
                    space: space_name(&declared),
                },
            )?,
        )
    };
    let indices = matches!(
        declared_space,
        Some(crate::colour::ColourSpace::Indexed { .. })
    );

    let decoded = pdf_sandbox::decode(&Request::Jpx {
        data: &source.data,
        indices,
    })
    .map_err(|error| ImageError::Sandboxed {
        detail: error.to_string(),
    })?;
    let Decoded::Raster(raster) = decoded else {
        return Err(ImageError::Malformed {
            detail: "JPEG 2000 did not decode to component samples".to_owned(),
        });
    };

    // §7.4.9: "Width and Height shall match the corresponding width and height values in
    // the JPEG 2000 data."
    if (raster.width, raster.height) != (width, height) {
        return Err(ImageError::Malformed {
            detail: format!(
                "JPEG 2000 image is {}x{} but the dictionary says {width}x{height}",
                raster.width, raster.height
            ),
        });
    }

    // §8.9.5.1 Table 87, `/SMaskInData`: code 1 and 2 both mean the samples carry opacity,
    // and 2 additionally means the colour components were multiplied by it. Absent or 0
    // means ignore any that is there. (This cited §8.9.5.4 until the eleventh session, which
    // is *alternate images* — a real clause, and not this one. The citation checker holds
    // every clause number to one the standard has, and cannot tell that from the right one.)
    let smask_in_data = document
        .get_key(dict, "SMaskInData")
        .as_integer()
        .unwrap_or(0);
    let use_opacity = smask_in_data != 0 && raster.has_opacity;
    let premultiplied = smask_in_data == 2;

    if is_mask {
        // §7.4.9: "If ImageMask is true, the JPEG 2000 data shall provide a single colour
        // channel with 1-bit samples." Those samples arrive scaled to eight bits, so the
        // two values are 0 and 255 and the threshold between them is anywhere in between.
        let samples: Vec<u8> = raster
            .data
            .chunks(raster.channels())
            .map(|pixel| u8::from(pixel.first().is_some_and(|value| *value >= 128)))
            .collect();
        let packed = pack_bits(&samples, width, height);
        return Ok((
            unpack(
                &packed,
                width,
                height,
                &Samples {
                    bits: 1,
                    space: &ColourSpace::Mask,
                    decode: &Decode::read(document, dict, &ColourSpace::Mask, 1),
                    // A stencil has no colour components for §8.9.6.4 to range over.
                    colour_key: None,
                    fill,
                },
            )?,
            use_opacity,
        ));
    }

    let space = match declared_space {
        Some(space) => space,
        None => codestream_colour_space(&raster)?,
    };
    if space.components() != usize::from(raster.components) {
        return Err(ImageError::Malformed {
            detail: format!(
                "the colour space takes {} components but the codestream has {}",
                space.components(),
                raster.components
            ),
        });
    }

    Ok((
        jpx_samples_to_rgba(&raster, &space, use_opacity, premultiplied),
        use_opacity,
    ))
}

/// Converts decoded JPEG 2000 samples into straight-alpha RGBA8.
fn jpx_samples_to_rgba(
    raster: &pdf_sandbox::Raster,
    space: &crate::colour::ColourSpace,
    use_opacity: bool,
    premultiplied: bool,
) -> Vec<u8> {
    // An `Indexed` space takes an *index*, not a fraction: `to_rgb` rounds its input and
    // looks it up. Every other space takes components in 0..1.
    let scale = if matches!(space, crate::colour::ColourSpace::Indexed { .. }) {
        1.0
    } else {
        1.0 / 255.0
    };

    let channels = raster.channels();
    let components = usize::from(raster.components);
    let pixels = (raster.width as usize).saturating_mul(raster.height as usize);
    let mut out = Vec::with_capacity(pixels.saturating_mul(4));
    let mut values = vec![0f32; components];
    for pixel in raster.data.chunks(channels) {
        let alpha = if use_opacity {
            pixel.get(components).copied().unwrap_or(255)
        } else {
            255
        };
        for (slot, sample) in values.iter_mut().zip(pixel.iter()) {
            *slot = f32::from(*sample) * scale;
        }
        if premultiplied && alpha != 0 {
            // Straight alpha is what `Image` documents and what both backends expect, so the
            // multiplication the producer did has to be undone here rather than compensated
            // for later. A zero alpha leaves the components alone: the colour under a fully
            // transparent pixel is not recoverable and not visible.
            let opacity = f32::from(alpha) / 255.0;
            for slot in &mut values {
                *slot /= opacity;
            }
        }
        let colour = space.to_rgb(&values);
        out.extend_from_slice(&[
            channel(colour.r),
            channel(colour.g),
            channel(colour.b),
            alpha,
        ]);
    }
    out
}

/// Chooses the colour space a JPEG 2000 codestream says its samples are in.
///
/// Reached only when the image dictionary has no `/ColorSpace`; where it has one, §7.4.9
/// gives it the last word — "any colour space specifications in the JPEG 2000 data shall be
/// ignored". Where the codestream's own answer cannot be used — an ICC profile this crate
/// cannot evaluate, or a space it does not recognise — the same clause names the fallback:
/// `DeviceGray`, `DeviceRGB` or `DeviceCMYK` according to whether there are 1, 3 or 4
/// ordinary channels.
fn codestream_colour_space(
    raster: &pdf_sandbox::Raster,
) -> Result<crate::colour::ColourSpace, ImageError> {
    let by_channel_count = || match raster.components {
        1 => Ok(crate::colour::ColourSpace::Gray),
        3 => Ok(crate::colour::ColourSpace::Rgb),
        4 => Ok(crate::colour::ColourSpace::Cmyk),
        other => Err(ImageError::UnsupportedColourSpace {
            space: format!("{other} JPEG 2000 channels"),
        }),
    };

    match &raster.colour {
        pdf_sandbox::Colour::Gray => Ok(crate::colour::ColourSpace::Gray),
        pdf_sandbox::Colour::Rgb => Ok(crate::colour::ColourSpace::Rgb),
        pdf_sandbox::Colour::Cmyk => Ok(crate::colour::ColourSpace::Cmyk),
        pdf_sandbox::Colour::Icc(profile) => {
            crate::icc::Profile::parse(profile).map_or_else(by_channel_count, |profile| {
                Ok(crate::colour::ColourSpace::Icc {
                    profile: Box::new(profile),
                })
            })
        }
        pdf_sandbox::Colour::Unknown => by_channel_count(),
    }
}

/// Names a colour space object for a report.
///
/// Its family, not its contents: a report saying `Indexed` is what a reader needs, and one
/// that printed the whole lookup table would bury it.
fn space_name(space: &Object) -> String {
    match space {
        Object::Name(name) => String::from_utf8_lossy(name.as_bytes()).into_owned(),
        Object::Array(items) => items.first().and_then(Object::as_name).map_or_else(
            || "an empty array".to_owned(),
            |name| String::from_utf8_lossy(name.as_bytes()).into_owned(),
        ),
        other => format!("{other:?}"),
    }
}

/// Packs one-bit samples into rows, most significant bit first.
///
/// The inverse of what [`unpack`] reads, so that a codec delivering a byte per pixel can
/// still take the ordinary 1-bit path rather than growing a parallel one.
fn pack_bits(samples: &[u8], width: u32, height: u32) -> Vec<u8> {
    let row_bytes = (width as usize).saturating_add(7) / 8;
    let mut packed = vec![0u8; row_bytes.saturating_mul(height as usize)];
    for y in 0..(height as usize) {
        for x in 0..(width as usize) {
            let index = y.saturating_mul(width as usize).saturating_add(x);
            if samples.get(index).copied().unwrap_or(0) != 0 {
                let at = y.saturating_mul(row_bytes).saturating_add(x / 8);
                if let Some(byte) = packed.get_mut(at) {
                    *byte |= 0x80u8 >> (x % 8);
                }
            }
        }
    }
    packed
}

/// Decodes a baseline JPEG.
fn decode_jpeg(data: &[u8], width: u32, height: u32) -> Result<Vec<u8>, ImageError> {
    // `ZCursor` is the reader `zune-jpeg` wants; a bare slice does not implement its
    // trait because the decoder needs to seek.
    let mut decoder =
        zune_jpeg::JpegDecoder::new(zune_jpeg::zune_core::bytestream::ZCursor::new(data));
    decoder
        .decode_headers()
        .map_err(|e| ImageError::Malformed {
            detail: format!("JPEG headers: {e}"),
        })?;

    let info = decoder.info().ok_or_else(|| ImageError::Malformed {
        detail: "JPEG has no frame".to_owned(),
    })?;
    let components = decoder
        .output_colorspace()
        .map_or(3, |space| space.num_components());
    let pixels = decoder.decode().map_err(|e| ImageError::Malformed {
        detail: format!("JPEG data: {e}"),
    })?;
    let count = usize::from(info.width).saturating_mul(usize::from(info.height));

    // The dictionary and the JPEG both state the dimensions; they must agree, because the
    // display list carries the dictionary's and the samples carry the JPEG's.
    if u32::from(info.width) != width || u32::from(info.height) != height {
        return Err(ImageError::Malformed {
            detail: format!(
                "JPEG is {}x{} but the dictionary says {width}x{height}",
                info.width, info.height
            ),
        });
    }

    // Filled with 255 so that alpha is already set and the loops below touch three bytes per
    // pixel instead of four.
    //
    // This loop is written for speed, and the benchmark that says it had to be is worth
    // recording: on `22060_A1_01_Plans.pdf`, `callgrind` put the *previous* version of it at
    // 6.89 G instructions, 38% of the whole run and nearly twice what `zune-jpeg` spent
    // actually decoding the JPEG. The cost was per pixel and structural — a `match` on the
    // component count inside the loop, three bounds-checked `get`s with `unwrap_or`,
    // saturating index arithmetic, and an `extend_from_slice` that re-checks capacity every
    // time. Pairing two `chunks_exact` iterators removes all four: the component count is
    // decided once, the chunk lengths are known to the compiler, and nothing is indexed.
    let mut out = vec![255u8; count.saturating_mul(4)];
    match components {
        1 => {
            for (destination, source) in out.chunks_exact_mut(4).zip(pixels.iter()) {
                if let Some(rgb) = destination.get_mut(..3) {
                    rgb.fill(*source);
                }
            }
        }
        3 => {
            for (destination, source) in out.chunks_exact_mut(4).zip(pixels.chunks_exact(3)) {
                if let Some(rgb) = destination.get_mut(..3) {
                    rgb.copy_from_slice(source);
                }
            }
        }
        _ => {
            return Err(ImageError::UnsupportedColourSpace {
                space: format!("{components}-component JPEG"),
            });
        }
    }

    Ok(out)
}

/// Applies §8.9.5.2's map to a raster that is already eight-bit RGB.
///
/// The `DCTDecode` route reaches this rather than [`unpack`], because `zune-jpeg` delivers
/// components rather than packed samples and the decoder's own colour transform has already
/// run. So the map is applied afterwards, over channels — which is the same arithmetic on
/// the same domain, since a JPEG component *is* an integer in 0 to 255.
///
/// `/Decode [1 0 1 0 1 0]` on a `DCTDecode` image is the one form of this the corpus states
/// (`issue7406.pdf`), and it was being dropped: this route never consulted the entry at all.
/// A greyscale JPEG's single component is written to all three channels by [`decode_jpeg`],
/// so the first pair governs each of them.
fn apply_decode_to_channels(document: &Document, dict: &Dictionary, rgba: &mut [u8]) {
    if document.get_key(dict, "Decode").as_array().is_none() {
        return;
    }
    // The components a JPEG delivers are device channels whatever `/ColorSpace` names, which
    // is the approximation this route already takes; `decode_jpeg` refuses anything but one
    // or three of them, and both become RGB here.
    let decode = Decode::read(document, dict, &ColourSpace::Rgb, 8);
    if decode.is_identity() {
        return;
    }
    let grey =
        matches!(document.get_key(dict, "Decode").as_array(), Some(items) if items.len() < 6);
    for pixel in rgba.chunks_exact_mut(4) {
        for (component, channel) in pixel.iter_mut().take(3).enumerate() {
            let pair = if grey { 0 } else { component };
            *channel = decode.channel(pair, usize::from(*channel));
        }
    }
}

/// Largest grid a mask and its image are combined on, in samples, where that is larger than
/// the image itself.
///
/// §8.9.6.3's two rasters "need not have the same resolution" and §11.6.5.2 Table 143 says
/// the same of a soft mask's, so combining either with its image means choosing a grid —
/// [`combine_on_the_finer_grid`] takes the finer of the two, which discards nothing either of
/// them carries and costs the product of the two larger dimensions. That product is what a
/// document controls, and 2^24 samples is 64 MB of RGBA: room for any real pair, and short of
/// the gigabyte a 2×2 image with a 34862×4332 mask would ask for. `issue16263.pdf` writes
/// exactly that pair as an `/SMask`.
///
/// See [`combined_grid`] for why the bound is on the *growth* rather than on the total.
const MAX_MASK_GRID: u64 = 1 << 24;

/// The grid a mask and its image would be combined on, if it is one this will allocate.
///
/// The bound is on how much the combination costs *beyond the image*, and it has to be: a
/// mask on the image's own grid costs exactly what the image costs, which [`MAX_SAMPLES`] has
/// already admitted, so a flat limit would refuse a pair no larger than the picture being
/// drawn. `issue19517.pdf` is that case — a 12608×16806 scan with a mask of the same
/// dimensions — and it was reported for its mask's size for as long as this rule was the flat
/// one, which said nothing true about the mask. What a document *can* do with two dimensions
/// it controls is make the product of the two larger ones enormous while both rasters stay
/// small, and that is what [`MAX_MASK_GRID`] refuses.
fn combined_grid(width: u32, height: u32, mask_width: u32, mask_height: u32) -> Result<u64, u64> {
    let grid = u64::from(width.max(mask_width)).saturating_mul(u64::from(height.max(mask_height)));
    let image = u64::from(width).saturating_mul(u64::from(height));
    if grid > MAX_MASK_GRID.max(image) {
        return Err(grid);
    }
    Ok(grid)
}

/// What an image's `/Mask` entry holds, once read.
///
/// ISO 32000-2 §8.9.6.1 lists it twice, because one key spells two mechanisms: `/Mask` is
/// either a second image saying *where* this one is painted (§8.9.6.3) or a set of ranges
/// saying *which colours* are not (§8.9.6.4). They share nothing but the key, which is why
/// this is read once, before the samples, and decided here rather than at three call sites.
enum MaskEntry {
    /// No `/Mask`.
    Absent,
    /// Present, and superseded by the document's own precedence rule.
    ///
    /// §11.6.4.3, of an image's `/SMask`: "This mask, if present, shall override any explicit
    /// or colour key mask specified by the image dictionary's Mask entry", and of a non-zero
    /// `/SMaskInData`: "the embedded soft-mask shall override any explicit or colour key
    /// mask". So a `/Mask` beside either is not a gap and nothing is owed — reporting it
    /// would name a key the file itself has told us not to read.
    Overridden,
    /// §8.9.6.4: one inclusive range per colour component, in raw sample values.
    ColourKey(Vec<(u32, u32)>),
    /// §8.9.6.3: an image mask naming the areas of the base image that are painted.
    Explicit(Arc<Stream>),
    /// Present, and not applied. Carries the words the interpreter reports.
    ///
    /// A `/Mask` that is silently dropped paints a region the document says is not part of
    /// the image, and `colorkeymask.pdf` is the standing example: a red band all four
    /// reference renderers hide, drawn by us with nothing reported until this existed.
    Unusable(String),
}

/// Reads `/Mask`, deciding which of §8.9.6's two mechanisms it names and whether it applies.
fn mask_entry(document: &Document, dict: &Dictionary) -> MaskEntry {
    let mask = document.get_key(dict, "Mask");
    if matches!(mask, Object::Null) {
        return MaskEntry::Absent;
    }
    // §11.6.4.3 orders the three mechanisms an image dictionary can carry, and `/Mask` is
    // last of them. Checked before anything is read out of it, so that a file writing both
    // cannot have the loser applied.
    let overridden = document.get_key(dict, "SMask").as_stream().is_some()
        || document
            .get_key(dict, "SMaskInData")
            .as_integer()
            .is_some_and(|value| value != 0);
    if overridden {
        return MaskEntry::Overridden;
    }
    match &mask {
        Object::Null => MaskEntry::Absent,
        Object::Array(items) => colour_key_entry(document, dict, items),
        _ => mask.as_stream().map_or_else(
            || MaskEntry::Unusable("/Mask is neither an image mask nor a range array".to_owned()),
            |stream| explicit_entry(document, dict, stream),
        ),
    }
}

/// Reads §8.9.6.4's range array against the image it masks.
///
/// > For colour key masking, the value of the Mask entry shall be an array of 2 × 𝑛
/// > integers, [min1max1 …min𝑛max𝑛] , where n is the number of colour components in the
/// > image's colour space. Each integer shall be in the range 0 to 2 BitsPerComponent - 1,
/// > representing colour values before decoding with the Decode array.
///
/// Both "shall"s are checked rather than assumed, because an array of the wrong length would
/// otherwise mask by whichever components happened to line up.
///
/// The refusal that is not about malformed input is the filtered one. The test is on the
/// samples a filter delivers, and this crate sees those only where they reach [`unpack`]; a
/// `DCTDecode` or `JPXDecode` image has become RGBA before then, and the clause's own NOTE 2
/// says of exactly that pair that lossy coding "can lead to slight changes in the colour
/// values of image samples, possibly causing samples that were intended to be masked to be
/// unexpectedly painted". The two bilevel codecs are refused with them for one reason rather
/// than four: a colour key over one-bit samples is a stencil written the long way, no corpus
/// document writes one, and a rule with three exceptions is worse than a rule.
fn colour_key_entry(document: &Document, dict: &Dictionary, items: &[Object]) -> MaskEntry {
    if matches!(document.get_key(dict, "ImageMask"), Object::Boolean(true)) {
        return MaskEntry::Unusable(
            "colour-key /Mask on an image mask, which has no colour components".to_owned(),
        );
    }
    if let Some(codec) = image_codec(document, dict) {
        return MaskEntry::Unusable(format!("colour-key /Mask on a {codec} image"));
    }
    let Ok(space) = colour_space(document, dict) else {
        return MaskEntry::Unusable(
            "colour-key /Mask on an image whose colour space this cannot read".to_owned(),
        );
    };

    let values: Vec<i64> = items
        .iter()
        .map(|item| document.resolve(item))
        .filter_map(|item| item.as_integer())
        .collect();
    let components = space.components();
    if values.len() != components.saturating_mul(2) {
        return MaskEntry::Unusable(format!(
            "colour-key /Mask has {} entries against a {components}-component image",
            values.len()
        ));
    }

    let bits = u32::try_from(
        document
            .get_key(dict, "BitsPerComponent")
            .as_integer()
            .unwrap_or(8),
    )
    .unwrap_or(8);
    let highest = i64::from(1u32.checked_shl(bits).unwrap_or(u32::MAX).saturating_sub(1));
    let mut ranges = Vec::with_capacity(components);
    for pair in values.chunks_exact(2) {
        let (Some(&low), Some(&high)) = (pair.first(), pair.get(1)) else {
            continue;
        };
        if low < 0 || high > highest {
            return MaskEntry::Unusable(format!(
                "colour-key /Mask range {low}..{high} is outside 0..{highest} at {bits} bits \
                 per component"
            ));
        }
        // A reversed pair is not malformed: `min i ≤ c i ≤ max i` is then satisfied by no
        // sample, so the range masks nothing, which is a statement a file may make.
        ranges.push((
            u32::try_from(low).unwrap_or(0),
            u32::try_from(high).unwrap_or(0),
        ));
    }
    MaskEntry::ColourKey(ranges)
}

/// Reads §8.9.6.3's explicit mask against the image it masks.
///
/// Two conditions decide whether it applies, and only the first is the clause's:
///
/// - **It has to be a stencil.** The clause admits "an image mask, as described in
///   subclause 8.9.6.2", and §8.9.6.2 defines one as an image `XObject` whose `/ImageMask`
///   entry is true. Table 87 says the same of the entry. A stream that carries a colour space and
///   says nothing about `/ImageMask` is therefore outside what `/Mask` is defined to hold,
///   and it is reported rather than interpreted.
///
///   `issue6621.pdf` is why that sentence is a rule rather than a formality. Its `/Mask` is
///   a one-bit `DeviceGray` image with no `/ImageMask`, and the stencil reading was written,
///   run, and thrown away on the evidence of the page: §8.9.6.2's "a sample value of 0 shall
///   mark the page" made the seal's *background* the painted part, so the page came out
///   blank where three renderers draw a court seal. The reading those three use is
///   §11.6.5.2's — luminosity as opacity, so white paints — which is a different clause
///   about a different key, and adopting it here would invert every stencil whose author
///   merely forgot `/ImageMask`. Refusing draws the image with its rectangle showing and
///   says so, which is wrong in a way a reader can see.
/// - **The combined grid has to fit.** See [`MAX_MASK_GRID`].
fn explicit_entry(document: &Document, dict: &Dictionary, stream: &Arc<Stream>) -> MaskEntry {
    let mask_dict = &stream.dict;
    let dimension = |dict: &Dictionary, key| {
        document
            .get_key(dict, key)
            .as_integer()
            .and_then(|value| u32::try_from(value).ok())
            .unwrap_or(0)
    };
    let (mask_width, mask_height) = (
        dimension(mask_dict, "Width"),
        dimension(mask_dict, "Height"),
    );
    let (width, height) = (dimension(dict, "Width"), dimension(dict, "Height"));

    if !matches!(
        document.get_key(mask_dict, "ImageMask"),
        Object::Boolean(true)
    ) {
        return MaskEntry::Unusable(format!(
            "/Mask is a {mask_width}x{mask_height} image that is not an image mask"
        ));
    }

    if let Err(grid) = combined_grid(width, height, mask_width, mask_height) {
        return MaskEntry::Unusable(format!(
            "/Mask is {mask_width}x{mask_height} against a {width}x{height} image, needing a \
             grid of {grid} samples"
        ));
    }
    MaskEntry::Explicit(Arc::clone(stream))
}

/// Names the image codec at the end of a `/Filter` chain, if there is one.
///
/// Read from the dictionary rather than from a decoded stream, because the callers that need
/// it are deciding whether to *report* something and must not pay for a decode to do it.
fn image_codec(document: &Document, dict: &Dictionary) -> Option<String> {
    let filter = document.get_key(dict, "Filter");
    let last = match &filter {
        Object::Name(name) => Some(name.as_bytes().to_vec()),
        Object::Array(items) => items
            .last()
            .map(|item| document.resolve(item))
            .and_then(|item| item.as_name().map(|name| name.as_bytes().to_vec())),
        _ => None,
    }?;
    matches!(
        last.as_slice(),
        b"DCTDecode" | b"DCT" | b"JPXDecode" | b"JBIG2Decode" | b"CCITTFaxDecode" | b"CCF"
    )
    .then(|| String::from_utf8_lossy(&last).into_owned())
}

/// Names a `/Mask` this crate cannot apply, for the caller to report.
///
/// Asked of the dictionary alone, so that the interpreter's report and this module's
/// behaviour cannot drift apart: every reason returned here is a reason [`decode`] will not
/// have applied the mask, and there is no other. The one case not covered is a mask whose
/// *data* will not decode, which [`decode`] turns into an error naming it — an image that
/// cannot be masked at all is refused rather than painted through a mask nobody could read.
#[must_use]
pub fn unapplied_mask(document: &Document, dict: &Dictionary) -> Option<String> {
    match mask_entry(document, dict) {
        MaskEntry::Unusable(reason) => Some(reason),
        MaskEntry::Absent
        | MaskEntry::Overridden
        | MaskEntry::ColourKey(_)
        | MaskEntry::Explicit(_) => None,
    }
}

/// Whether this image carries a mask of its own, which supersedes the graphics state's.
///
/// ISO 32000-2 §11.6.4.3, having listed the three ways a soft mask reaches a compositing
/// operation:
///
/// > Either form of mask in the image dictionary shall override, for this image object
/// > only, the current soft mask in the graphics state.
///
/// So an image with an `/SMask`, an `/SMaskInData` or a `/Mask` is painted through that and
/// *not* through the mask a `gs` left in force — and the state's mask is untouched for
/// everything drawn after it. Asked of the keys rather than of the decoded image, because
/// the clause is about what the dictionary says: a `/Mask` this crate cannot apply still
/// overrides, and is reported by [`unapplied_mask`] rather than quietly replaced by a
/// different mask than the file named.
#[must_use]
pub fn overrides_graphics_state_mask(document: &Document, dict: &Dictionary) -> bool {
    !matches!(document.get_key(dict, "SMask"), Object::Null)
        || document
            .get_key(dict, "SMaskInData")
            .as_integer()
            .is_some_and(|value| value != 0)
        || !matches!(document.get_key(dict, "Mask"), Object::Null)
}

/// Combines an image with a mask that need not share its resolution, on the finer grid.
///
/// `sample` is asked of each base colour and the four bytes of the mask sample above it, for
/// the colour to write and the opacity to scale the base pixel's own alpha by. Both of the
/// standard's per-image masks reduce to an opacity: a stencil (§8.9.6.3) answers all or
/// nothing, a soft-mask image (§11.6.5.2) answers its grey level. Multiplying rather than
/// replacing is §11.3.7's `α = shape × opacity` read through §11.6.4.1, which names the two as
/// separate sources; it also means the answer does not depend on which mask is applied first.
/// The colour passes through unchanged except where §11.6.5.2's `/Matte` has to be undone,
/// which is the one thing a mask says about a base *colour* rather than about its alpha.
///
/// # Which grid the two are combined on
///
/// §8.9.6.3, of an explicit mask:
///
/// > The base image and the image mask need not have the same resolution ( Width and Height
/// > values), but since all images shall be defined on the unit square in user space, their
/// > boundaries on the page will coincide; that is, they will overlay each other.
///
/// §11.6.5.2 Table 143 says it of a soft mask's `/Width` in different words — "independent of
/// it. Both images shall be mapped to the unit square in user space (as are all images),
/// regardless of whether the samples coincide individually" — and both leave the sampling to
/// the device. Correctly, the two are combined at *output* resolution, which this crate does
/// not know: it holds one raster per image and hands it to a rasteriser that may draw it at
/// any scale. The choice made here is the finer of the two grids in each axis, sampled
/// nearest-neighbour, and its merit is that it discards nothing either raster carries:
/// `issue4246.pdf` masks a 50×40 image with a 1000×800 stencil, and combining on the image's
/// grid would throw away 399 of every 400 mask samples; `smaskdim.pdf` gives a 2×2 image a
/// 76×102 soft mask, where the image's grid would leave four samples of a rounded rectangle.
/// What it costs is memory, which [`MAX_MASK_GRID`] bounds, and the difference from a
/// device-resolution composite where the page is drawn larger than both.
///
/// Nearest-neighbour is a choice for the stencil and a compromise for the soft mask. A
/// stencil's samples are two values with no meaningful average, and §8.9.5.3 leaves a
/// magnified image unsmoothed unless `/Interpolate` asks otherwise, which is the same answer
/// the base image gets. A soft mask carries continuous values, so its magnified edges are
/// stepped where a device-resolution composite would have interpolated them — visible only
/// where the mask is *coarser* than the grid, which is the case this function is not asked
/// about: it is the image that gets magnified in every corpus pair.
fn combine_on_the_finer_grid(
    image: &Image,
    mask: &Image,
    sample: impl Fn([u8; 3], &[u8]) -> ([u8; 3], u8),
) -> Image {
    let width = image.width.max(mask.width);
    let height = image.height.max(mask.height);
    let (width_usize, height_usize) = (width as usize, height as usize);
    let mut data = Vec::with_capacity(width_usize.saturating_mul(height_usize).saturating_mul(4));

    // `scale` maps a column or row of the combined grid onto one of a source grid. Integer
    // arithmetic throughout: both grids are at least one sample and the divisor is the
    // combined extent, so nothing here can divide by zero or leave the source's range.
    let scale = |along: usize, source: u32, combined: usize| {
        along
            .saturating_mul(source as usize)
            .checked_div(combined)
            .unwrap_or(0)
    };

    for y in 0..height_usize {
        let image_row = scale(y, image.height, height_usize).saturating_mul(image.width as usize);
        let mask_row = scale(y, mask.height, height_usize).saturating_mul(mask.width as usize);
        for x in 0..width_usize {
            let at = image_row
                .saturating_add(scale(x, image.width, width_usize))
                .saturating_mul(4);
            let mask_at = mask_row
                .saturating_add(scale(x, mask.width, width_usize))
                .saturating_mul(4);
            let pixel = image
                .data
                .get(at..at.saturating_add(4))
                .unwrap_or(&[0, 0, 0, 0]);
            let above = mask
                .data
                .get(mask_at..mask_at.saturating_add(4))
                .unwrap_or(&[0, 0, 0, 0]);
            let colour = [
                pixel.first().copied().unwrap_or(0),
                pixel.get(1).copied().unwrap_or(0),
                pixel.get(2).copied().unwrap_or(0),
            ];
            let (colour, opacity) = sample(colour, above);
            let own = u16::from(pixel.get(3).copied().unwrap_or(0));
            // Rounded rather than truncated, so an opaque pixel under a fully opaque mask
            // stays opaque: 255 × 255 / 255 is exact either way, but 255 × 254 / 255 is not.
            let combined = own
                .saturating_mul(u16::from(opacity))
                .saturating_add(127)
                .checked_div(255)
                .unwrap_or(0);
            data.extend_from_slice(&[
                colour[0],
                colour[1],
                colour[2],
                u8::try_from(combined).unwrap_or(u8::MAX),
            ]);
        }
    }

    Image {
        width,
        height,
        data: Arc::from(data.as_slice()),
        interpolate: image.interpolate,
    }
}

/// Applies §8.9.6.3's explicit mask: where the stencil does not mark, the image is not drawn.
fn apply_explicit_mask(
    document: &Document,
    image: &Image,
    stream: &Stream,
) -> Result<Image, ImageError> {
    let stencil = decode(document, stream, pdf_render::Color::BLACK).map_err(|error| {
        ImageError::Malformed {
            detail: format!("/Mask did not decode: {error}"),
        }
    })?;
    // The stencil came back from `decode` as the fill colour where its samples mark the page
    // and as nothing where they do not, so alpha is where the answer already is —
    // §8.9.6.2's rule about which bit marks, and any `/Decode [1 0]` reversing it, were both
    // applied there. This is the whole reason the mask goes through the ordinary image route
    // rather than a private one: `issue4379.pdf`'s stencil is `CCITTFaxDecode`d.
    //
    // The sense is the clause's — the mask indicates which places on the page are painted and
    // which are masked out — so a sample that marks paints and one that does not is left
    // unchanged.
    Ok(combine_on_the_finer_grid(
        image,
        &stencil,
        |colour, sample| {
            let marks = sample.get(3).is_some_and(|alpha| *alpha != 0);
            (colour, if marks { u8::MAX } else { 0 })
        },
    ))
}

/// What an image's `/SMask` entry holds, once read.
///
/// The same shape as [`MaskEntry`] and for the same reason: what the entry means has to be
/// decided once, so that what the interpreter reports and what [`decode`] does cannot drift
/// apart. Unlike `/Mask` there is only one mechanism here — §11.6.5.2's soft-mask image —
/// and the variants are whether it can be used.
enum SoftMaskEntry {
    /// No `/SMask`, or one that is not a stream.
    Absent,
    /// §11.6.5.2: a `DeviceGray` image whose samples are this image's opacity.
    ///
    /// `matte` is Table 144's colour, in the raster's own components, when the image's samples
    /// are pre-blended with one and this crate can undo it. `owed` names a requirement of the
    /// clause that applying the mask does not satisfy, and there is exactly one: a `/Matte`
    /// this crate cannot undo, because inverting it "shall precede the colour conversion" and
    /// the conversion has happened by the time an image reaches here. It is reported *beside*
    /// the mask rather than instead of it, which is the second place in this tree where a
    /// report accompanies drawing (the first is `/NeedAppearances`), and it needs the same
    /// argument. Here it is: the mask itself is fully specified and applying it is right,
    /// while the pre-blending is a defect in the colours. Refusing the mask because of the
    /// matte would draw an opaque rectangle whose edges are *entirely* the matte colour —
    /// where α is 0, `c′ = m` — which is worse on the page and no more honest.
    Image {
        stream: Arc<Stream>,
        matte: Option<[u8; 3]>,
        owed: Option<String>,
    },
    /// Present, and not applied. Carries the words the interpreter reports.
    Unusable(String),
}

/// Reads `/SMask` against the image it masks, deciding whether §11.6.5.2's mask applies.
///
/// Table 143 restricts a soft-mask image dictionary, and three of its restrictions decide
/// whether this crate can use one at all:
///
/// - **`/Width` and `/Height` are "independent of" the parent image's** unless `/Matte` is
///   present, and "[b]oth images shall be mapped to the unit square in user space (as are all
///   images), regardless of whether the samples coincide individually". So a mask of another
///   size still applies; [`combine_on_the_finer_grid`] is where the choice that costs is
///   made, and [`MAX_MASK_GRID`] is the one pair it refuses — `issue16263.pdf` gives a 2×2
///   image a 34862×4332 mask, 151 million samples for two distinct colours.
/// - **`/ColorSpace` is "Required; shall be `DeviceGray`"**, and this is not pedantry: the
///   mask is decoded by the ordinary image route, so a mask in some other space arrives as a
///   colour, and there is no clause saying which of its components is the opacity. §11.5.3's
///   luminosity is a rule about a *transparency group*'s colour, not about this key.
/// - **`/ImageMask` "[s]hall be false or absent"**, which matters more than it looks: a
///   stencil decodes to the fill colour and an alpha, carrying no grey level at all, so
///   reading its first component as opacity would make every such image fully transparent.
///   Refusing draws the image opaque, which is wrong in a way a reader can see.
///
/// A `/Matte` is the fourth thing read here and the only one that does not decide whether the
/// mask applies — see [`matte_colour`], which decides whether the pre-blending it announces
/// can be undone in the raster this crate holds.
fn soft_mask_entry(document: &Document, dict: &Dictionary) -> SoftMaskEntry {
    let smask = document.get_key(dict, "SMask");
    let Some(mask) = smask.as_stream() else {
        return SoftMaskEntry::Absent;
    };
    let dimension = |dict: &Dictionary, key| {
        document
            .get_key(dict, key)
            .as_integer()
            .and_then(|value| u32::try_from(value).ok())
            .unwrap_or(0)
    };
    let (mask_width, mask_height) = (
        dimension(&mask.dict, "Width"),
        dimension(&mask.dict, "Height"),
    );
    let (width, height) = (dimension(dict, "Width"), dimension(dict, "Height"));

    if matches!(
        document.get_key(&mask.dict, "ImageMask"),
        Object::Boolean(true)
    ) {
        return SoftMaskEntry::Unusable(format!(
            "/SMask is a {mask_width}x{mask_height} image mask, which carries no opacity"
        ));
    }
    if document.get_key(&mask.dict, "SMask").as_stream().is_some() {
        // Table 143 again: an `/SMask` inside an `/SMask` "[s]hall be absent". Applied to the
        // mask it would change what the mask says, which is not a thing the clause defines.
        return SoftMaskEntry::Unusable("/SMask carries a soft mask of its own".to_owned());
    }
    match colour_space(document, &mask.dict) {
        Ok(space) if space.components() == 1 => {}
        Ok(space) => {
            return SoftMaskEntry::Unusable(format!(
                "/SMask has a {}-component colour space where Table 143 requires DeviceGray",
                space.components()
            ));
        }
        Err(error) => {
            return SoftMaskEntry::Unusable(format!("/SMask colour space: {error}"));
        }
    }

    if let Err(grid) = combined_grid(width, height, mask_width, mask_height) {
        return SoftMaskEntry::Unusable(format!(
            "/SMask is {mask_width}x{mask_height} against a {width}x{height} image, needing a \
             grid of {grid} samples"
        ));
    }
    let (matte, owed) = match matte_colour(document, dict, &mask.dict) {
        Matte::Absent => (None, None),
        Matte::Colour(colour) => (Some(colour), None),
        Matte::Unreadable(reason) => (None, Some(reason)),
    };
    SoftMaskEntry::Image {
        stream: Arc::clone(mask),
        matte,
        owed,
    }
}

/// Table 144's `/Matte`, once read against the image whose samples it was blended into.
enum Matte {
    /// No `/Matte`: the image's samples are its colours.
    Absent,
    /// The matte colour, in the components the parent image's raster carries.
    Colour([u8; 3]),
    /// Present, and not undone. Carries the words the interpreter reports.
    Unreadable(String),
}

/// Reads `/Matte` from a soft-mask image dictionary, in the parent image's own components.
///
/// Table 144 gives the entry `n` numbers in the *parent* image's colour space, and §11.6.5.2
/// says what they mean: its samples are `c′ = m + α × (c - m)`, a weighted average of the
/// colour and the matte, so a processor "may sometimes need to invert the formula" to get `c`
/// back. Two of the clause's sentences decide how much of that can be done here — the
/// computation belongs to the parent image's own colour space, and:
///
/// > If a colour conversion is required, inversion of the pre-blending shall precede the
/// > colour conversion.
///
/// This crate holds one RGBA raster per image, so the conversion has already happened by the
/// time a mask is applied — and the inversion is exact afterwards only where the conversion
/// was the identity on components, which is the two device spaces `DeviceGray` and
/// `DeviceRGB`. In any other space the raster's bytes are a *function* of the pre-blended
/// components rather than the components, and dividing them by α computes something the
/// clause does not describe. Those are reported instead, which is a gap this has never been
/// asked for: `issue13931.pdf` is the corpus's only `/Matte` and its parent image is
/// `DeviceRGB`.
fn matte_colour(document: &Document, dict: &Dictionary, mask_dict: &Dictionary) -> Matte {
    let matte = document.get_key(mask_dict, "Matte");
    let Object::Array(items) = &matte else {
        return Matte::Absent;
    };
    #[expect(
        clippy::cast_possible_truncation,
        reason = "a colour component, which `channel` clamps to 0.0..=1.0 in any case"
    )]
    let components: Vec<f32> = items
        .iter()
        .map(|item| document.resolve(item))
        .filter_map(|item| item.as_number().map(|value| value as f32))
        .collect();
    // A component is a colour value in the parent's space, so 0..1 for both device spaces
    // this can invert; anything outside that is clamped by `channel` rather than refused,
    // which is what §8.9.5.2 does with an out-of-range sample.
    match (colour_space(document, dict), components.as_slice()) {
        (Ok(ColourSpace::Gray), [grey]) => {
            let grey = channel(*grey);
            Matte::Colour([grey, grey, grey])
        }
        (Ok(ColourSpace::Rgb), [red, green, blue]) => {
            Matte::Colour([channel(*red), channel(*green), channel(*blue)])
        }
        (Ok(space), _) => Matte::Unreadable(format!(
            "/SMask has a /Matte of {} components against a {}-component image, or one whose \
             pre-blending cannot be undone after conversion",
            components.len(),
            space.components()
        )),
        (Err(error), _) => Matte::Unreadable(format!("/SMask has a /Matte and {error}")),
    }
}

/// Undoes §11.6.5.2's pre-blending for one component: `c = m + (c′ - m) / α`.
///
/// Integer arithmetic: the numerator is at most 255 × 255, the divisor is the mask sample, and
/// the quotient truncates toward zero — under one part in 255 of the restored component, and
/// exact at both ends of the range, which is where a mistake would show. Where α is 0 the clause's NOTE says the inverse divides by zero and "an arbitrary value for
/// c can be chosen", because a fully transparent sample cannot affect the output — the matte
/// colour is the value that costs nothing to justify. The clamp is the clause's too: "[t]he
/// resulting c value shall lie within the range of colour component values for the image
/// colour space".
fn unblend(value: u8, matte: u8, alpha: u8) -> u8 {
    if alpha == 0 {
        return matte;
    }
    let scaled = i32::from(value)
        .saturating_sub(i32::from(matte))
        .saturating_mul(255)
        .checked_div(i32::from(alpha))
        .unwrap_or(0);
    u8::try_from(scaled.saturating_add(i32::from(matte)).clamp(0, 255)).unwrap_or(u8::MAX)
}

/// Names what §11.6.5.2 asks of an `/SMask` and this crate does not do, for the caller to
/// report.
///
/// Asked of the dictionary alone, so that the report and the behaviour cannot drift apart —
/// the same contract as [`unapplied_mask`], and the same one case it does not cover: a mask
/// whose data will not decode leaves the image opaque, because an image visibly present and
/// slightly wrong beats one dropped entirely. Unlike that function it also reports where the
/// mask *is* applied, for the one requirement that is not about the mask — see
/// [`SoftMaskEntry::Image`].
///
/// Reading it costs no decode, which is what makes it safe to ask of every image: the
/// 34862×4332 mask above cost 19 seconds and 600 MB when this question was answered by
/// decoding first and comparing afterwards.
#[must_use]
pub fn unapplied_soft_mask(document: &Document, dict: &Dictionary) -> Option<String> {
    match soft_mask_entry(document, dict) {
        SoftMaskEntry::Unusable(reason)
        | SoftMaskEntry::Image {
            owed: Some(reason), ..
        } => Some(reason),
        SoftMaskEntry::Absent | SoftMaskEntry::Image { owed: None, .. } => None,
    }
}

/// Applies §11.6.5.2's soft mask: each of its samples is the image's opacity there.
///
/// A soft mask that cannot be read leaves the image opaque rather than failing it: an
/// opaque image is visibly present and slightly wrong, whereas dropping it loses content
/// entirely.
fn apply_soft_mask(document: &Document, dict: &Dictionary, image: Image) -> Image {
    let SoftMaskEntry::Image {
        stream: mask_stream,
        matte,
        ..
    } = soft_mask_entry(document, dict)
    else {
        return image;
    };
    let Ok(mask) = decode(document, &mask_stream, pdf_render::Color::BLACK) else {
        return image;
    };
    combine_on_the_finer_grid(&image, &mask, |colour, sample| {
        // Table 143 required `DeviceGray` and `soft_mask_entry` checked it, so the three
        // colour channels of a mask sample hold one value and the first of them is it.
        let opacity = sample.first().copied().unwrap_or(0);
        let colour = match matte {
            None => colour,
            Some(matte) => [
                unblend(colour[0], matte[0], opacity),
                unblend(colour[1], matte[1], opacity),
                unblend(colour[2], matte[2], opacity),
            ],
        };
        (colour, opacity)
    })
}

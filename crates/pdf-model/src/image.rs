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
        Some(b"DCTDecode" | b"DCT") => (decode_jpeg(&source.data, width, height)?, false),
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
            (
                unpack(
                    &source.data,
                    width,
                    height,
                    &Samples {
                        bits,
                        space: &space,
                        invert: decode_inverts(document, dict),
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

    // The explicit mask comes after the soft mask, and the order is not arbitrary: it may
    // change the raster's dimensions, and `apply_soft_mask` compares its own mask against
    // them. The two are separate mechanisms — §8.9.6.3 is the opaque imaging model's cut-out
    // and §11.6.5.2's is a per-sample alpha — so an image carrying both gets both.
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

/// Returns `true` if `/Decode` inverts the samples.
///
/// Only the fully-inverted case is recognised, which is the one that occurs: `[1 0]` on a
/// mask or a greyscale image. An arbitrary decode array is a per-component linear map and
/// is not applied.
fn decode_inverts(document: &Document, dict: &Dictionary) -> bool {
    let decode = document.get_key(dict, "Decode");
    let Some(items) = decode.as_array() else {
        return false;
    };
    let values: Vec<f64> = items
        .iter()
        .map(|item| document.resolve(item))
        .filter_map(|item| item.as_number())
        .collect();
    matches!(values.first(), Some(first) if *first > 0.5)
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
    /// Whether `/Decode` reverses the component range (§8.9.5.2).
    invert: bool,
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
        invert,
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
            Some(palette(resolved, bits, invert))
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
        invert,
        colour_key: _,
        fill,
    } = samples;
    let opaque =
        |colour: pdf_render::Color| [channel(colour.r), channel(colour.g), channel(colour.b), 255];
    match (space, bits) {
        (ColourSpace::Mask, _) => {
            let bit = sample_bit(row, x);
            // A set bit means "do not paint" unless `/Decode [1 0]` inverts it.
            let paint = if invert { bit } else { !bit };
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
            // A set bit is white unless `/Decode [1 0]` inverts the range.
            let value = if sample_bit(row, x) ^ invert { 255 } else { 0 };
            [value, value, value, 255]
        }
        (ColourSpace::Gray, _) => {
            let value = maybe_invert(row.get(x).copied().unwrap_or(0), invert);
            [value, value, value, 255]
        }
        (ColourSpace::Rgb, _) => {
            let at = x.saturating_mul(3);
            [
                maybe_invert(row.get(at).copied().unwrap_or(0), invert),
                maybe_invert(row.get(at.saturating_add(1)).copied().unwrap_or(0), invert),
                maybe_invert(row.get(at.saturating_add(2)).copied().unwrap_or(0), invert),
                255,
            ]
        }
        (ColourSpace::Cmyk, _) => {
            let at = x.saturating_mul(4);
            let read = |offset: usize| {
                f32::from(maybe_invert(
                    row.get(at.saturating_add(offset)).copied().unwrap_or(0),
                    invert,
                )) / 255.0
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
            // The table already carries `/Decode`'s inversion, so the sample is the index
            // and nothing else happens to it here.
            opaque(
                palette
                    .and_then(|table| table.get(sample))
                    .copied()
                    .unwrap_or(pdf_render::Color::BLACK),
            )
        }
        (ColourSpace::Resolved(resolved), _) => {
            opaque(resolved_sample(resolved, row, x, bits, invert))
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
    invert: bool,
) -> pdf_render::Color {
    let count = space.components();
    let at = x.saturating_mul(count);
    // §8.9.5.2 Table 88: every space's default `/Decode` maps a sample onto the full range
    // of its component, which is 0.0 to 1.0 for every space that gets here.
    let values: Vec<f32> = (0..count)
        .map(|offset| {
            let index = at.saturating_add(offset);
            if bits == 1 {
                // One bit per *component*, so the components of a pixel are adjacent bits
                // rather than adjacent bytes.
                f32::from(sample_bit(row, index) ^ invert)
            } else {
                f32::from(maybe_invert(row.get(index).copied().unwrap_or(0), invert)) / 255.0
            }
        })
        .collect();
    space.to_rgb(&values)
}

/// Every colour a one-component space can produce at this bit depth, in sample order.
///
/// `invert` is baked in, so the table is indexed by the raw sample. §8.9.5.2 Table 88 decides
/// what a sample *means* before the space sees it: an `Indexed` space's default `/Decode` is
/// `[0 2^n - 1]`, which passes an index through unchanged, and every other space's maps the
/// sample onto 0.0 to 1.0.
fn palette(space: &crate::colour::ColourSpace, bits: u32, invert: bool) -> Vec<pdf_render::Color> {
    let max = (1u32 << bits.min(8)).saturating_sub(1);
    let indexed = matches!(space, crate::colour::ColourSpace::Indexed { .. });
    (0..=max)
        .map(|raw| {
            let sample = if invert { max.saturating_sub(raw) } else { raw };
            #[expect(
                clippy::cast_precision_loss,
                reason = "a sample is at most 255, which f32 represents exactly"
            )]
            let value = sample as f32;
            #[expect(
                clippy::cast_precision_loss,
                reason = "the maximum is at most 255, which f32 represents exactly"
            )]
            let full = max.max(1) as f32;
            space.to_rgb(&[if indexed { value } else { value / full }])
        })
        .collect()
}

/// Reads bit `index` of a packed row, most significant bit first.
fn sample_bit(row: &[u8], index: usize) -> bool {
    let byte = row.get(index / 8).copied().unwrap_or(0);
    let shift = 7u32.saturating_sub(u32::try_from(index % 8).unwrap_or(0));
    (byte >> shift) & 1 == 1
}

fn maybe_invert(value: u8, invert: bool) -> u8 {
    if invert {
        255u8.saturating_sub(value)
    } else {
        value
    }
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
    unpack(
        &bilevel.rows,
        width,
        height,
        &Samples {
            bits: 1,
            space: &space,
            invert: decode_inverts(document, dict),
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
    unpack(
        &bilevel.rows,
        width,
        height,
        &Samples {
            bits: 1,
            space: &space,
            invert: decode_inverts(document, dict),
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
                    invert: decode_inverts(document, dict),
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

/// Largest grid a `/Mask` and its image are combined on, in samples.
///
/// §8.9.6.3's two rasters "need not have the same resolution", so combining them means
/// choosing a grid — [`apply_explicit_mask`] takes the finer of the two, which discards
/// nothing either of them carries and costs the product of the two larger dimensions. That
/// product is what a document controls, and 2^24 samples is 64 MB of RGBA: room for any real
/// pair, and short of the gigabyte a 2×2 image with a 34862×4332 mask would ask for.
/// `issue16263.pdf` writes exactly that pair as an `/SMask`.
const MAX_MASK_GRID: u64 = 1 << 24;

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

    let grid = u64::from(width.max(mask_width)).saturating_mul(u64::from(height.max(mask_height)));
    if grid > MAX_MASK_GRID {
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

/// Applies §8.9.6.3's explicit mask: where the stencil does not mark, the image is not drawn.
///
/// # Which grid the two are combined on
///
/// > The base image and the image mask need not have the same resolution ( Width and Height
/// > values), but since all images shall be defined on the unit square in user space, their
/// > boundaries on the page will coincide; that is, they will overlay each other.
///
/// So the clause states the geometry and leaves the sampling to the device: correctly, the
/// two are combined at *output* resolution, which this crate does not know — it holds one
/// raster per image and hands it to a rasteriser that may draw it at any scale. The choice
/// made here is the finer of the two grids in each axis, sampled nearest-neighbour, and its
/// merit is that it discards nothing either raster carries: `issue4246.pdf` masks a 50×40
/// image with a 1000×800 stencil, and combining on the image's grid would throw away 399 of
/// every 400 mask samples. What it costs is memory, which [`MAX_MASK_GRID`] bounds, and the
/// difference from a device-resolution composite where the page is drawn larger than both.
///
/// Nearest-neighbour is not an approximation of a filter here — a stencil's samples are two
/// values with no meaningful average, and §8.9.5.3 leaves a magnified image unsmoothed
/// unless `/Interpolate` asks otherwise, which is the same answer for the base image.
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
    let width = image.width.max(stencil.width);
    let height = image.height.max(stencil.height);
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
        let stencil_row =
            scale(y, stencil.height, height_usize).saturating_mul(stencil.width as usize);
        for x in 0..width_usize {
            let at = image_row
                .saturating_add(scale(x, image.width, width_usize))
                .saturating_mul(4);
            let stencil_at = stencil_row
                .saturating_add(scale(x, stencil.width, width_usize))
                .saturating_mul(4);
            let pixel = image
                .data
                .get(at..at.saturating_add(4))
                .unwrap_or(&[0, 0, 0, 0]);
            let marks = stencil
                .data
                .get(stencil_at..stencil_at.saturating_add(4))
                .is_some_and(|sample| sample.get(3).is_some_and(|alpha| *alpha != 0));
            data.extend_from_slice(&[
                pixel.first().copied().unwrap_or(0),
                pixel.get(1).copied().unwrap_or(0),
                pixel.get(2).copied().unwrap_or(0),
                if marks {
                    pixel.get(3).copied().unwrap_or(0)
                } else {
                    0
                },
            ]);
        }
    }

    Ok(Image {
        width,
        height,
        data: Arc::from(data.as_slice()),
        interpolate: image.interpolate,
    })
}

/// Names an `/SMask` this crate cannot apply, for the caller to report.
///
/// A soft mask is not required to have the image's dimensions. ISO 32000-2 §11.6.5.2
/// Table 143, of a mask's `/Width`:
///
/// > If a Matte entry (see "Table 144 - Additional entry in a soft-mask image dictionary")
/// > is present, shall be the same as the Width value of the parent image; otherwise
/// > independent of it. Both images shall be mapped to the unit square in user space (as
/// > are all images), regardless of whether the samples coincide individually.
///
/// So the two grids are mapped onto the same square and combined at whatever resolution the
/// output has. This crate has one raster per image and no idea what resolution the page will
/// be drawn at, so combining them means choosing a grid: the image's loses the mask's detail
/// wherever the mask is finer, and the mask's costs its whole area — `issue16263.pdf` gives a
/// 2×2 image a 34862×4332 mask, which is 604 MB of RGBA for two distinct colours. Neither is
/// a decision to take from inside an image decoder, so the mask is left unapplied and named.
///
/// What that costs is on the page: `issue16263.pdf` draws black bars where the mask should
/// have cut them to overline strokes. Doing it properly means compositing an image and its
/// mask at *device* resolution, which is a display-list question rather than this one.
pub fn unapplied_soft_mask(document: &Document, dict: &Dictionary) -> Option<String> {
    let smask = document.get_key(dict, "SMask");
    let mask = smask.as_stream()?;

    let dimension = |key| {
        document
            .get_key(&mask.dict, key)
            .as_integer()
            .and_then(|value| u32::try_from(value).ok())
            .unwrap_or(0)
    };
    let (mask_width, mask_height) = (dimension("Width"), dimension("Height"));
    let width = document.get_key(dict, "Width").as_integer().unwrap_or(0);
    let height = document.get_key(dict, "Height").as_integer().unwrap_or(0);

    (i64::from(mask_width) != width || i64::from(mask_height) != height)
        .then(|| format!("/SMask is {mask_width}x{mask_height} against a {width}x{height} image"))
}

/// Applies an `/SMask` alpha channel, if the image has one and it decodes.
///
/// A soft mask that cannot be read leaves the image opaque rather than failing it: an
/// opaque image is visibly present and slightly wrong, whereas dropping it loses content
/// entirely.
fn apply_soft_mask(document: &Document, dict: &Dictionary, image: Image) -> Image {
    let smask = document.get_key(dict, "SMask");
    let Some(mask_stream) = smask.as_stream() else {
        return image;
    };

    // Only a mask sample-for-sample with the image is applied — see [`unapplied_soft_mask`]
    // for why that is a gap rather than a rule, and for what is reported instead.
    //
    // Asked of the *dictionary*, before decoding: `issue16263.pdf` gives a 2×2 image a
    // 34862×4332 mask, and decoding first to compare afterwards spent 19 seconds and 600 MB
    // producing a raster that was discarded on the next line.
    if unapplied_soft_mask(document, dict).is_some() {
        return image;
    }

    let Ok(mask) = decode(document, mask_stream, pdf_render::Color::BLACK) else {
        return image;
    };
    if mask.width != image.width || mask.height != image.height {
        return image;
    }

    let mut data = image.data.to_vec();
    for (pixel, mask_pixel) in data.chunks_exact_mut(4).zip(mask.data.chunks_exact(4)) {
        // The mask is greyscale, so any colour channel carries its value.
        if let (Some(alpha), Some(value)) = (pixel.get_mut(3), mask_pixel.first()) {
            *alpha = *value;
        }
    }

    Image {
        width: image.width,
        height: image.height,
        data: Arc::from(data.as_slice()),
        interpolate: image.interpolate,
    }
}

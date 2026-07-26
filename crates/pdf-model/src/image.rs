//! Decoding image `XObject`s into RGBA8 samples.
//!
//! Everything is normalised to straight-alpha RGBA8, so neither rasteriser backend needs
//! to know about PDF colour spaces or bit depths — the same reason colours are resolved
//! before they reach a backend.
//!
//! # What is refused, and why that matters here more than elsewhere
//!
//! `JBIG2Decode` and `JPXDecode` are not implemented and will not be until the sandbox
//! exists. Neither has a memory-safe implementation, and JBIG2 in particular is the
//! codec behind the FORCEDENTRY zero-click exploit. Wrapping a C library for them
//! *unsandboxed* would undo the main security argument for writing this project in Rust
//! at all.
//!
//! An image this module cannot decode returns an error naming why, and the interpreter
//! reports it. Drawing a grey box in its place would be worse: the page would look
//! finished and be wrong.

use std::sync::Arc;

use pdf_render::Image;
use pdf_syntax::{Dictionary, Document, Object, Stream};

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
}

/// How an image's samples are interpreted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ColourSpace {
    /// One component per sample.
    Gray,
    /// Three components per sample.
    Rgb,
    /// Four components per sample, subtractive.
    Cmyk,
    /// A stencil mask: one bit per sample, painted in the current fill colour.
    Mask,
}

impl ColourSpace {
    fn components(self) -> usize {
        match self {
            Self::Gray | Self::Mask => 1,
            Self::Rgb => 3,
            Self::Cmyk => 4,
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

    // The last filter decides how the samples arrive. `decoded_stream_data` already ran
    // the non-image filters, so a remaining image codec means it stopped there.
    let final_filter = image_filter(document, dict);

    let rgba = match final_filter.as_deref() {
        Some(b"DCTDecode" | b"DCT") => decode_jpeg(stream, width, height)?,
        Some(other) => {
            return Err(ImageError::UnsupportedFilter {
                filter: String::from_utf8_lossy(other).into_owned(),
            });
        }
        None => {
            let data =
                document
                    .decoded_stream_data(stream)
                    .ok_or_else(|| ImageError::Malformed {
                        detail: "stream did not decode".to_owned(),
                    })?;
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
            unpack(
                &data,
                width,
                height,
                bits,
                space,
                decode_inverts(document, dict),
                fill,
            )?
        }
    };

    let image = Image {
        width,
        height,
        data: Arc::from(rgba.as_slice()),
    };
    // Applied last so a soft mask cannot resurrect an inconsistent buffer.
    let image = apply_soft_mask(document, dict, image);
    Ok(image)
}

/// Returns the image codec left on the stream, if any.
///
/// A chain such as `[/FlateDecode /DCTDecode]` is unusual but legal; only the last entry
/// can be an image codec.
fn image_filter(document: &Document, dict: &Dictionary) -> Option<Vec<u8>> {
    let filter = document.get_key(dict, "Filter");
    let last = match filter {
        Object::Name(name) => name.as_bytes().to_vec(),
        Object::Array(items) => items
            .last()
            .map(|item| document.resolve(item))
            .and_then(|item| item.as_name().map(|name| name.as_bytes().to_vec()))?,
        _ => return None,
    };

    matches!(
        last.as_slice(),
        b"DCTDecode" | b"DCT" | b"JPXDecode" | b"JBIG2Decode" | b"CCITTFaxDecode" | b"CCF"
    )
    .then_some(last)
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
                b"CalRGB" => Ok(ColourSpace::Rgb),
                b"CalGray" => Ok(ColourSpace::Gray),
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
                other => Err(ImageError::UnsupportedColourSpace {
                    space: String::from_utf8_lossy(other).into_owned(),
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

/// Unpacks raw samples into RGBA8.
fn unpack(
    data: &[u8],
    width: u32,
    height: u32,
    bits: u32,
    space: ColourSpace,
    invert: bool,
    fill: pdf_render::Color,
) -> Result<Vec<u8>, ImageError> {
    if !matches!(bits, 1 | 8) {
        // 2, 4 and 16 are legal and do occur. Refusing them is honest; guessing would
        // shift every sample.
        return Err(ImageError::UnsupportedDepth { bits });
    }

    let components = space.components();
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
            match (space, bits) {
                (ColourSpace::Mask, _) => {
                    let bit = sample_bit(row, x);
                    // A set bit means "do not paint" unless `/Decode [1 0]` inverts it.
                    let paint = if invert { bit } else { !bit };
                    if paint {
                        out.extend_from_slice(&[
                            channel(fill.r),
                            channel(fill.g),
                            channel(fill.b),
                            channel(fill.a),
                        ]);
                    } else {
                        out.extend_from_slice(&[0, 0, 0, 0]);
                    }
                }
                (ColourSpace::Gray, 1) => {
                    // A set bit is white unless `/Decode [1 0]` inverts the range.
                    let value = if sample_bit(row, x) ^ invert { 255 } else { 0 };
                    out.extend_from_slice(&[value, value, value, 255]);
                }
                (ColourSpace::Gray, _) => {
                    let value = maybe_invert(row.get(x).copied().unwrap_or(0), invert);
                    out.extend_from_slice(&[value, value, value, 255]);
                }
                (ColourSpace::Rgb, _) => {
                    let at = x.saturating_mul(3);
                    out.extend_from_slice(&[
                        maybe_invert(row.get(at).copied().unwrap_or(0), invert),
                        maybe_invert(row.get(at.saturating_add(1)).copied().unwrap_or(0), invert),
                        maybe_invert(row.get(at.saturating_add(2)).copied().unwrap_or(0), invert),
                        255,
                    ]);
                }
                (ColourSpace::Cmyk, _) => {
                    let at = x.saturating_mul(4);
                    let read = |offset: usize| {
                        f32::from(maybe_invert(
                            row.get(at.saturating_add(offset)).copied().unwrap_or(0),
                            invert,
                        )) / 255.0
                    };
                    // The *same* conversion a `k` operator or an `scn` in DeviceCMYK gets.
                    // Having a second one here is how the same colour came to render
                    // differently depending on whether it was drawn as a fill or as an
                    // image, which is exactly the bug this crate should not have.
                    let colour = crate::colour::ColourSpace::Cmyk.to_rgb(&[
                        read(0),
                        read(1),
                        read(2),
                        read(3),
                    ]);
                    let byte = |value: f32| {
                        #[expect(
                            clippy::cast_possible_truncation,
                            clippy::cast_sign_loss,
                            reason = "a colour component is clamped to 0.0..=1.0 upstream"
                        )]
                        {
                            (value.clamp(0.0, 1.0) * 255.0).round() as u8
                        }
                    };
                    out.extend_from_slice(&[byte(colour.r), byte(colour.g), byte(colour.b), 255]);
                }
            }
        }
    }

    Ok(out)
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

/// Decodes a baseline JPEG.
fn decode_jpeg(stream: &Stream, width: u32, height: u32) -> Result<Vec<u8>, ImageError> {
    // The raw stream bytes: `DCTDecode` is the last filter, so nothing else has consumed
    // them. A chain with an earlier filter would need that filter run first, which is why
    // only the single-filter case reaches here.
    // `ZCursor` is the reader `zune-jpeg` wants; a bare slice does not implement its
    // trait because the decoder needs to seek.
    let mut decoder =
        zune_jpeg::JpegDecoder::new(zune_jpeg::zune_core::bytestream::ZCursor::new(&stream.data));
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

    let mut out = Vec::with_capacity(count.saturating_mul(4));
    for index in 0..count {
        let at = index.saturating_mul(components);
        match components {
            1 => {
                let value = pixels.get(at).copied().unwrap_or(0);
                out.extend_from_slice(&[value, value, value, 255]);
            }
            3 => out.extend_from_slice(&[
                pixels.get(at).copied().unwrap_or(0),
                pixels.get(at.saturating_add(1)).copied().unwrap_or(0),
                pixels.get(at.saturating_add(2)).copied().unwrap_or(0),
                255,
            ]),
            _ => {
                return Err(ImageError::UnsupportedColourSpace {
                    space: format!("{components}-component JPEG"),
                });
            }
        }
    }

    Ok(out)
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

    let Ok(mask) = decode(document, mask_stream, pdf_render::Color::BLACK) else {
        return image;
    };
    // Only a mask matching the image's dimensions is applied; scaling one is resampling,
    // and doing it badly would show as a halo.
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
    }
}

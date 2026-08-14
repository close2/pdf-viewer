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
use pdf_syntax::{Dictionary, Document, ImageStream, Object, ObjectId, Stream};
use rayon::iter::ParallelIterator as _;
use rayon::slice::ParallelSliceMut as _;

use crate::colour::Compositing;

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
    /// Reduces a resolved space to the arm the unpacker can take fastest.
    ///
    /// The three device families are eight-bit channels and nothing else, so they get an arm
    /// that reads bytes; every other space is a *function* of its samples and takes the one
    /// that converts. This is a choice about speed and never about meaning — the two routes
    /// agree by construction, because `crate::colour::ColourSpace::to_rgb` is the identity on
    /// a device space's components (§8.6.4, and the test that pins it).
    ///
    /// **They stop agreeing inside a `/Luminosity` mask group**, which is why the reduction
    /// asks what the samples are being composited into. There a sample becomes §10.4.2.3's
    /// ink rather than a colour (`crate::colour::Compositing`), and that is the identity on
    /// no device space at all: a grey `g` painted into a `DeviceCMYK` group is `(1 + g) ÷ 2`
    /// on the channel. So the fast arms are not taken and every sample goes through the one
    /// conversion, memoised by [`Conversion`] or tabulated by [`palette`] as it would be for
    /// any other space. That costs a mask group's images the fast path and nothing else pays
    /// (ADR 0220).
    ///
    /// A page §11.4.7 composites in `DeviceCMYK` is the same argument again: a sample becomes
    /// one half of that space's four components, which is the identity on no device space
    /// either — a `DeviceRGB` sample goes through §10.4.2.4 and a `DeviceCMYK` one is split
    /// between the two rasters. So those images take the converting arm as a mask group's do,
    /// and only a page that states such a space pays for it (ADR 0262).
    fn reduced(space: crate::colour::ColourSpace, into: Compositing) -> Self {
        match (space, into) {
            (space, Compositing::Luminosity(_) | Compositing::Subtractive(..)) => {
                Self::Resolved(space)
            }
            (crate::colour::ColourSpace::Gray, _) => Self::Gray,
            (crate::colour::ColourSpace::Rgb, _) => Self::Rgb,
            (crate::colour::ColourSpace::Cmyk, _) => Self::Cmyk,
            (other, _) => Self::Resolved(other),
        }
    }

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

/// An image `XObject` decoded as far as a grid the *file* states can take it.
///
/// Two of the standard's rasters share a unit square without sharing a grid — §8.9.6.3's
/// explicit mask and §11.6.5.2's soft-mask image — and where their refinement is small enough
/// to build, [`combine_on_the_finer_grid`] builds it and the answer is one raster. Where it is
/// not, §10.7.4's answer is the device's own grid, and this carries the two parts to whoever
/// knows it.
#[derive(Debug)]
pub enum Parts {
    /// One raster, with every mask the dictionary states already in it.
    Complete(Image),
    /// The base raster, and the soft mask that belongs with it at device resolution.
    Masked {
        /// The image's own samples, on the grid the file states.
        base: Image,
        /// §11.6.5.2's mask, still packed.
        opacity: SoftMaskAtDeviceScale,
    },
}

impl Parts {
    /// What a display list carries, with `over_base` given the graphics state's say first.
    ///
    /// The interpreter has one thing to add to an image's samples that the image dictionary
    /// knows nothing about: §10.5's transfer function, which belongs to the graphics state the
    /// image is drawn under rather than to the image. It is applied to the base raster here,
    /// **before** the mask is attached, which is the same order the eager route uses and is
    /// available to it for the same reason — a transfer maps colour components and a mask
    /// scales alpha, so neither can see the other's work.
    #[must_use]
    pub fn source(self, over_base: impl FnOnce(Image) -> Image) -> pdf_render::ImageSource {
        match self {
            Self::Complete(image) => pdf_render::ImageSource::Decoded(over_base(image)),
            Self::Masked { base, opacity } => {
                pdf_render::ImageSource::AtDeviceScale(opacity.over(over_base(base)))
            }
        }
    }
}

/// Decodes an image `XObject` to one raster on the grid the file states.
///
/// `fill` is the current fill colour, used for a stencil mask, which paints the *current*
/// colour through its set bits rather than carrying colour of its own.
///
/// A mask [`decode_parts`] would have deferred is resolved here at the base image's **own**
/// grid, which is the finest one a caller that wants a single raster can be given without
/// knowing a device. Every caller of this wants exactly that: a thumbnail is a small picture by
/// construction (§12.3.4), and a stencil painted through a pattern becomes a soft mask whose
/// grid is the stencil's.
///
/// # Errors
///
/// See [`ImageError`].
pub fn decode(
    document: &Document,
    stream: &Stream,
    resources: &Dictionary,
    fill: pdf_render::Color,
    into: Compositing,
) -> Result<Image, ImageError> {
    let mut masks = MaskCache::default();
    Ok(
        match decode_parts(document, stream, resources, fill, into, &mut masks)? {
            Parts::Complete(image) => image,
            Parts::Masked { base, opacity } => {
                let grid = pdf_render::Grid {
                    width: base.width,
                    height: base.height,
                };
                opacity.over(base).samples(grid)
            }
        },
    )
}

/// Decodes an image `XObject`, leaving a mask the device must place where it is.
///
/// The route the content interpreter takes, because a display list is rasterised at a scale the
/// interpreter does not know. [`decode`] is the same work with the parts put back together.
///
/// `masks` is the caller's memo of masks already read; see [`MaskCache`] for what it is worth
/// and why the key is sound.
///
/// # Errors
///
/// See [`ImageError`].
pub fn decode_parts(
    document: &Document,
    stream: &Stream,
    resources: &Dictionary,
    fill: pdf_render::Color,
    into: Compositing,
    masks: &mut MaskCache,
) -> Result<Parts, ImageError> {
    let dict = &stream.dict;
    let at = Dictionaries {
        document,
        dict,
        resources,
    };

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
    let mask = mask_entry(document, dict, resources);
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

    let (rgba, opacity_came_with_the_samples) = samples_of(
        at,
        &source,
        (width, height),
        is_mask,
        fill,
        colour_key,
        into,
    )?;

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
        // A mask whose grid the finer of the two cannot hold leaves this function in two
        // parts, for the device to put together. Everything below is about one raster.
        if let Some(opacity) = masks.read(document, dict, resources) {
            return Ok(Parts::Masked {
                base: image,
                opacity,
            });
        }
        // Applied last so a soft mask cannot resurrect an inconsistent buffer.
        apply_soft_mask(document, dict, resources, image)
    };

    // §11.6.4.3 makes the two mutually exclusive — an `/SMask` "shall override any explicit
    // or colour key mask" — so `mask_entry` has already returned [`MaskEntry::Overridden`]
    // for anything reached here after a soft mask was applied, and this arm runs only where
    // there was none. The sequence is therefore an ordering of two things that never both
    // happen, kept in the order the clauses rank them.
    match &mask {
        MaskEntry::Explicit(stencil) => {
            apply_explicit_mask(document, &image, resources, stencil).map(Parts::Complete)
        }
        _ => Ok(Parts::Complete(image)),
    }
}

/// The image's samples as straight-alpha RGBA8, and whether the opacity came with them.
///
/// One arm per route from bytes to colour: §7.4.8's `DCTDecode`, §7.4.7's `JBIG2Decode`,
/// §7.4.9's `JPXDecode`, §7.4.6's `CCITTFaxDecode`, and [`unpack`] for the four filters that
/// leave samples behind rather than a codestream. Split out of [`decode_parts`] because the
/// route a stream takes is a question about its filter chain and nothing else, while
/// everything around it is about masks.
///
/// The second half of the answer is §11.6.5.2's `/SMaskInData`, which only `JPXDecode` can
/// carry: an opacity that arrived inside the codestream is already in the alpha channel, and
/// applying `/SMask` on top of it would multiply two alphas together.
///
/// # Errors
///
/// See [`ImageError`].
fn samples_of(
    at: Dictionaries,
    source: &ImageStream,
    (width, height): (u32, u32),
    is_mask: bool,
    fill: pdf_render::Color,
    colour_key: Option<&[(u32, u32)]>,
    into: Compositing,
) -> Result<(Vec<u8>, bool), ImageError> {
    let Dictionaries {
        document,
        dict,
        resources,
    } = at;
    match source.codec.as_deref() {
        Some(b"DCTDecode" | b"DCT") => {
            let (mut rgba, components) = decode_jpeg(&source.data, width, height)?;
            apply_decode_to_channels(document, dict, components, &mut rgba);
            convert_channels(at, is_mask, components, &mut rgba, into)?;
            Ok((rgba, false))
        }
        Some(b"JBIG2Decode") => Ok((
            decode_jbig2(at, source, width, height, is_mask, fill, into)?,
            false,
        )),
        Some(b"JPXDecode") => decode_jpx(at, source, width, height, is_mask, fill, into),
        Some(b"CCITTFaxDecode" | b"CCF") => Ok((
            decode_ccitt(at, source, width, height, is_mask, fill, into)?,
            false,
        )),
        Some(other) => Err(ImageError::UnsupportedFilter {
            filter: String::from_utf8_lossy(other).into_owned(),
        }),
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
                colour_space(document, dict, resources, into)?
            };
            let decode = Decode::read(document, dict, &space, bits);
            let rgba = unpack(
                &source.data,
                width,
                height,
                &Samples {
                    bits,
                    space: &space,
                    decode: &decode,
                    colour_key,
                    fill,
                    into,
                },
            )?;
            Ok((rgba, false))
        }
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

/// An image dictionary, the document holding it, and the resources it was named from.
///
/// One argument rather than three because the three are never apart. Table 87's entries are
/// in the dictionary, the objects they reference are in the document, and — this is the one
/// that was missing until the twenty-fifth session — §8.6.5.6's `/DefaultGray`, `/DefaultRGB`
/// and `/DefaultCMYK` are in the resource dictionary the image was *drawn* from, which is a
/// property of the `Do` rather than of the image.
#[derive(Clone, Copy)]
struct Dictionaries<'a> {
    /// The document, for resolving anything indirect.
    document: &'a Document,
    /// The image's own dictionary.
    dict: &'a Dictionary,
    /// The resources in force where the image was drawn.
    resources: &'a Dictionary,
}

/// Determines the colour space, reduced to what the sample unpacker handles.
fn colour_space(
    document: &Document,
    dict: &Dictionary,
    resources: &Dictionary,
    into: Compositing,
) -> Result<ColourSpace, ImageError> {
    let space = document.get_key(dict, "ColorSpace");
    // §8.9.5.1 Table 87, of `/ColorSpace`: "it can be any type of colour space except
    // Pattern". A pattern carries no colour of its own, so a sample in one names nothing
    // that could be unpacked, and the refusal has to come before the parse because a
    // `/Pattern` array *is* a colour space the colour module reads.
    let family = match &space {
        Object::Name(name) => name.as_bytes().to_vec(),
        Object::Array(items) => items
            .first()
            .map(|item| document.resolve(item))
            .and_then(|item| item.as_name().map(|name| name.as_bytes().to_vec()))
            .unwrap_or_default(),
        _ => {
            return Err(ImageError::UnsupportedColourSpace {
                space: "absent".to_owned(),
            });
        }
    };
    if family == b"Pattern" || family == b"P" {
        return Err(ImageError::UnsupportedColourSpace {
            space: "Pattern, which Table 87 excludes".to_owned(),
        });
    }

    // Everything else goes through the one function that decides what a colour space *is*,
    // against the resources the image is drawn from. Two clauses want it that way and this
    // route was reaching neither. §8.6.5.6: the remapping through `/DefaultGray`,
    // `/DefaultRGB` or `/DefaultCMYK` applies to "a colour space given as an entry in an
    // image XObject, inline image, or shading dictionary", and an empty resource dictionary
    // used to be passed here, so a device space named by an image was never remapped.
    // §8.6.5.5: an `ICCBased` space's profile is the document's own statement of what its
    // numbers mean, and this function used to reduce such a space to a device one by its
    // `/N` — so the same colour rendered differently depending on whether it reached the
    // page as a fill or as an image, which is exactly trap 6's defect one level up.
    let resolved =
        crate::colour::ColourSpace::parse(document, &space, resources).ok_or_else(|| {
            ImageError::UnsupportedColourSpace {
                space: String::from_utf8_lossy(&family).into_owned(),
            }
        })?;
    Ok(ColourSpace::reduced(resolved, into))
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
    /// What the samples are being composited into (`crate::colour::Compositing`).
    ///
    /// A sample is a colour like any other, and §11.6.5.1's mask group composites in a
    /// quantity that is not the device's — so a raster that ignored this would be the one
    /// thing in such a group painted in the wrong units (ADR 0220).
    into: Compositing,
}

/// Unpacks raw samples into RGBA8.
fn unpack(data: &[u8], width: u32, height: u32, samples: &Samples) -> Result<Vec<u8>, ImageError> {
    let &Samples {
        bits,
        space,
        decode,
        colour_key,
        fill: _,
        into,
    } = samples;
    // Table 87 names five, and a value it does not name says nothing about how the bytes are
    // packed — so it is refused rather than rounded to a depth that would shift every sample.
    if !matches!(bits, 1 | 2 | 4 | 8 | 16) {
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
    // Only up to eight bits: a 16-bit table is 65 536 conversions, which is more work than
    // the image itself for anything smaller than a quarter-megapixel, and the per-sample arm
    // below already memoises exactly. No corpus image reaches that combination.
    let palette = match space {
        ColourSpace::Resolved(resolved) if resolved.components() == 1 && bits <= 8 => {
            Some(palette(resolved, bits, decode, into))
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

    // Only the arm that converts per sample needs one, and only where a tuple fits a key.
    // The memo's key packs eight bits per component, so it is exact only at or below that
    // depth; see `resolved_sample`.
    let mut cache = matches!(space, ColourSpace::Resolved(_))
        .then(|| palette.is_none() && components <= 4 && bits <= 8)
        .filter(|fits| *fits)
        .map(|_| Conversion::for_pixels(width_usize.saturating_mul(height_usize)));

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
            out.extend_from_slice(&sample_rgba(
                samples,
                palette.as_deref(),
                &mut cache,
                row,
                x,
            ));
        }
    }

    Ok(out)
}

/// Converts a three-component raster in place, optionally on more than one thread.
///
/// **A band's memo is its own, and that is what makes the split exact**: [`Conversion`]
/// memoises a pure function of a sample tuple, so two bands meeting the same tuple convert it
/// twice and agree. Nothing in the result depends on how the raster was divided, which is the
/// property `a_band_boundary_changes_no_pixel` exists to check — and it is the difference
/// between this split and the rasteriser's, where a curve clipped by a strip's edge is
/// re-parameterised and the pixels *do* move (ADR 0138).
///
/// The table is sized from the band rather than from the image, because one proportioned to the
/// whole would be allocated once per thread.
fn convert_three(
    space: &crate::colour::ColourSpace,
    rgba: &mut [u8],
    band: Option<usize>,
    into: Compositing,
) {
    let convert = |chunk: &mut [u8], slots: usize| {
        let mut cache = Conversion::for_pixels(slots.max(1));
        for pixel in chunk.chunks_exact_mut(4) {
            let Some(rgb) = pixel.get_mut(..3) else {
                continue;
            };
            let read = |index: usize| rgb.get(index).copied().unwrap_or(0);
            let key = (1u64 << 32)
                | (u64::from(read(0)) << 16)
                | (u64::from(read(1)) << 8)
                | u64::from(read(2));
            let colour = if let Some(colour) = cache.get(key) {
                colour
            } else {
                let colour = into.paint(
                    space,
                    &[
                        f32::from(read(0)) / 255.0,
                        f32::from(read(1)) / 255.0,
                        f32::from(read(2)) / 255.0,
                    ],
                    true,
                );
                cache.put(key, colour);
                colour
            };
            rgb.copy_from_slice(&[channel(colour.r), channel(colour.g), channel(colour.b)]);
        }
    };
    match band {
        Some(band) => rgba
            .par_chunks_mut(band.saturating_mul(4).max(4))
            .for_each(|chunk| convert(chunk, band)),
        None => convert(rgba, rgba.len() / 4),
    }
}

/// Converts a four-component raster in place, on the same terms as [`convert_three`].
///
/// The four bytes of a pixel are the four components — `decode_jpeg` writes them there and
/// nothing has read them as colour yet — so the alpha byte is restored here, after the fourth
/// component has been consumed. Everything else is [`convert_three`]'s argument, including
/// why a band boundary changes no pixel: the memo is of a pure function of one sample tuple.
fn convert_four(
    space: &crate::colour::ColourSpace,
    rgba: &mut [u8],
    band: Option<usize>,
    into: Compositing,
) {
    let convert = |chunk: &mut [u8], slots: usize| {
        let mut cache = Conversion::for_pixels(slots.max(1));
        for pixel in chunk.chunks_exact_mut(4) {
            let read = |index: usize| pixel.get(index).copied().unwrap_or(0);
            // A tag of its own, so a four-component tuple cannot collide with a
            // three-component one in a memo shared by neither.
            let key = (1u64 << 33)
                | (u64::from(read(0)) << 24)
                | (u64::from(read(1)) << 16)
                | (u64::from(read(2)) << 8)
                | u64::from(read(3));
            let colour = if let Some(colour) = cache.get(key) {
                colour
            } else {
                let colour = into.paint(
                    space,
                    &[
                        f32::from(read(0)) / 255.0,
                        f32::from(read(1)) / 255.0,
                        f32::from(read(2)) / 255.0,
                        f32::from(read(3)) / 255.0,
                    ],
                    true,
                );
                cache.put(key, colour);
                colour
            };
            pixel.copy_from_slice(&[channel(colour.r), channel(colour.g), channel(colour.b), 255]);
        }
    };
    match band {
        Some(band) => rgba
            .par_chunks_mut(band.saturating_mul(4).max(4))
            .for_each(|chunk| convert(chunk, band)),
        None => convert(rgba, rgba.len() / 4),
    }
}

/// How many pixels one parallel band of a colour conversion covers, or `None` for one thread.
///
/// **Both numbers here are measured and the measurement is a wall clock**, because a parallel
/// change makes the instruction count go *up* while the page appears sooner — the distinction
/// session 162 had to make for the strips, and the reason both are quoted below.
///
/// `issue19971.pdf`'s 2500×1364 `ICCBased` photograph, interpreted whole, on 24 cores:
///
/// | bands | median clock | instructions |
/// |---|---|---|
/// | serial | ~110 ms | 1 085 M |
/// | 4 | ~85 ms | 1 206 M |
/// | **8** | **~57 ms** | **1 365 M** |
/// | 24 (one per core) | ~55 ms | 1 605 M |
///
/// So the cap is 8: it is the whole of the clock and two thirds of the extra processor time.
/// What the extra buys nothing is a [`Conversion`] table per band — each is allocated and
/// zeroed, and a table proportioned to a 24th of the image collides no less than one
/// proportioned to an eighth.
///
/// Below [`PARALLEL_PIXELS`] the split is refused for the same reason: a small image would pay a
/// table per thread to save a few hundred conversions.
fn band_pixels(pixels: usize) -> Option<usize> {
    if pixels < PARALLEL_PIXELS {
        return None;
    }
    let bands = rayon::current_num_threads().clamp(1, MAX_BANDS);
    (bands >= 2).then(|| pixels.div_ceil(bands))
}

/// The smallest image worth colour-managing on more than one thread.
///
/// A quarter of a megapixel, which is where the two cross on this machine. The corpus gate —
/// 974 documents whose images are mostly far smaller — is 2.4 s either side of it, which is the
/// check that the threshold is not paid for by everything else.
const PARALLEL_PIXELS: usize = 1 << 18;

/// The most bands one image is divided into. See [`band_pixels`] for the measurement.
const MAX_BANDS: usize = 8;

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
        let raw = raw_sample(row, index, bits);
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
    cache: &mut Option<Conversion>,
    row: &[u8],
    x: usize,
) -> [u8; 4] {
    let &Samples {
        bits,
        space,
        decode,
        colour_key: _,
        fill,
        into,
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
        (ColourSpace::Gray, _) => {
            let value = decode.channel(0, index_of(row, x, bits));
            [value, value, value, 255]
        }
        (ColourSpace::Rgb, _) => {
            let at = x.saturating_mul(3);
            let read = |component: usize| {
                decode.channel(component, index_of(row, at.saturating_add(component), bits))
            };
            [read(0), read(1), read(2), 255]
        }
        (ColourSpace::Cmyk, _) => {
            let at = x.saturating_mul(4);
            let read = |offset: usize| {
                decode.value(offset, index_of(row, at.saturating_add(offset), bits))
            };
            // The *same* conversion a `k` operator or an `scn` in DeviceCMYK gets. Having a
            // second one here is how the same colour came to render differently depending on
            // whether it was drawn as a fill or as an image, which is exactly the bug this
            // crate should not have.
            opaque(crate::colour::ColourSpace::Cmyk.to_rgb(&[read(0), read(1), read(2), read(3)]))
        }
        (ColourSpace::Resolved(_), _) if palette.is_some() => {
            let sample = index_of(row, x, bits);
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
            opaque(resolved_sample(resolved, row, x, bits, decode, cache, into))
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
    cache: &mut Option<Conversion>,
    into: Compositing,
) -> pdf_render::Color {
    let count = space.components();
    let at = x.saturating_mul(count);
    // The raw samples, before `/Decode`, which is what makes them a key: the map from a
    // tuple of samples to a colour is fixed for the whole image. Eight bits per component
    // and at most four components is 32 bits, which the tag then sits above — so at sixteen
    // bits per component the key cannot hold the samples and the caller supplies no cache at
    // all rather than a lossy one.
    //
    // **The tag is not decoration and this line had lost it**, which the
    // three-hundred-and-eighty-third session found. `key` was seeded with the tag and then
    // shifted left eight bits per component, so four components pushed it out of the word
    // entirely and an all-zero sample tuple keyed on 0 — which is what [`Conversion`]'s slots
    // hold before anything is put in one, so `get` answered a *hit* out of an empty table and
    // every such pixel came back `Color::BLACK`. **Four components exactly**, because three
    // leave the tag at bit 56 and a wider key is never built: a `DeviceN` of four colourants
    // at no tint, and, since this round, every `DeviceCMYK` image inside a `/Luminosity` mask
    // group, where zero ink is white. The bytes are packed into the low half now and the tag
    // stays where the comment always said it was.
    let mut packed = 0u32;
    let values: Vec<f32> = (0..count)
        .map(|component| {
            // One sample per *component*, so at depths below eight the components of a
            // pixel are adjacent groups of bits rather than adjacent bytes.
            let raw = index_of(row, at.saturating_add(component), bits);
            #[expect(
                clippy::cast_possible_truncation,
                reason = "masked to one byte before the cast"
            )]
            {
                packed = (packed << 8) | ((raw & 0xFF) as u32);
            }
            decode.value(component, raw)
        })
        .collect();
    let key = (1u64 << 32) | u64::from(packed);
    if let Some(cache) = cache.as_ref()
        && let Some(colour) = cache.get(key)
    {
        return colour;
    }
    let colour = into.paint(space, &values, true);
    if let Some(cache) = cache.as_mut() {
        cache.put(key, colour);
    }
    colour
}

/// Every colour a one-component space can produce at this bit depth, in sample order.
///
/// The `/Decode` map is baked in, so the table is indexed by the raw sample and the
/// per-pixel path does nothing but index it.
fn palette(
    space: &crate::colour::ColourSpace,
    bits: u32,
    decode: &Decode,
    into: Compositing,
) -> Vec<pdf_render::Color> {
    let max = (1u32 << bits.min(8)).saturating_sub(1);
    debug_assert!(bits <= 8, "the caller keeps a 16-bit image off this path");
    (0..=max)
        .map(|raw| into.paint(space, &[decode.value(0, raw as usize)], true))
        .collect()
}

/// A memo of what a sample tuple converts to, for one image.
///
/// **Exact, not an approximation**: the key is the raw sample bytes, so a hit returns the
/// colour the conversion would have returned for those very samples. Nothing here rounds,
/// interpolates or resamples.
///
/// It exists because converting an image through a real colour space costs transcendental
/// functions per pixel. Callgrind on `issue19971.pdf`'s 2500x1364 `ICCBased` photograph
/// puts 36% of the page in `libm` — an ICC profile's tone curves and the sRGB encode are
/// both powers — and applying §8.6.5.5 without a memo took the whole corpus gate from 1.7 s
/// to 11.4 s. A one-component space never gets here: [`palette`] converts each of its at
/// most 256 possible samples once, which is the same idea where an exact table fits.
///
/// Direct-mapped rather than a hash map, and small enough to stay in cache, because an
/// image's colours are *spatially* clustered: neighbouring pixels are usually the same
/// colour or a near one, so a fixed table with no chaining and no growth answers most of
/// them. A collision costs one conversion, which is what the code did before, so the worst
/// case is the old cost plus a bounded probe.
struct Conversion {
    /// Packed sample tuple, with bit 32 set where the entry is occupied.
    keys: Vec<u64>,
    /// What that tuple converted to.
    values: Vec<pdf_render::Color>,
}

impl Conversion {
    /// Smallest and largest table, in entries.
    ///
    /// Sized from the image rather than fixed, because both ends cost. A 2^18-entry table
    /// is 2.9 MB to allocate and zero, which a 16x16 icon should not pay; a 2^12-entry one
    /// collides constantly on a photograph. Measured on `issue19971.pdf`'s 2500x1364
    /// `ICCBased` photograph and on the whole corpus gate, which is the population of small
    /// images: 2^14 gives 1.68 G instructions on the photograph and 1.9 s on the gate, 2^16
    /// gives 1.36 G and 1.8 s, and a fixed 2^18 gives 1.05 G and 1.9 s — the gate paying
    /// back what the photograph saves. Sizing by the image takes both.
    const MIN_SLOTS: usize = 1 << 12;
    const MAX_SLOTS: usize = 1 << 18;

    /// A table proportioned to an image of `pixels` samples.
    ///
    /// A quarter of the pixel count, rounded up to a power of two: an image whose colours
    /// are all distinct cannot be helped by any table, and one with structure repeats long
    /// before a quarter of its pixels.
    fn for_pixels(pixels: usize) -> Self {
        let wanted = pixels.next_power_of_two() / 4;
        let slots = wanted.clamp(Self::MIN_SLOTS, Self::MAX_SLOTS);
        Self {
            keys: vec![0; slots],
            values: vec![pdf_render::Color::BLACK; slots],
        }
    }

    /// Where a tuple lives, if it lives anywhere.
    ///
    /// Knuth's multiplicative hash on the packed tuple: the samples of one pixel are
    /// adjacent bytes, so the low bits alone would put every shade of one hue in one slot.
    fn slot(&self, key: u64) -> usize {
        let mixed = key.wrapping_mul(0x9E37_79B9_7F4A_7C15);
        // Shifted down to 24 bits before the mask, so a table smaller than 2^24 entries is
        // indexed by mixed bits rather than by the tuple's own low bytes.
        usize::try_from(mixed >> 40).unwrap_or(0) & (self.keys.len().saturating_sub(1))
    }

    fn get(&self, key: u64) -> Option<pdf_render::Color> {
        let at = self.slot(key);
        (self.keys.get(at) == Some(&key))
            .then(|| self.values.get(at).copied())
            .flatten()
    }

    fn put(&mut self, key: u64, colour: pdf_render::Color) {
        let at = self.slot(key);
        if let Some(slot) = self.keys.get_mut(at) {
            *slot = key;
        }
        if let Some(slot) = self.values.get_mut(at) {
            *slot = colour;
        }
    }
}

/// Reads bit `index` of a packed row, most significant bit first.
fn sample_bit(row: &[u8], index: usize) -> bool {
    let byte = row.get(index / 8).copied().unwrap_or(0);
    let shift = 7u32.saturating_sub(u32::try_from(index % 8).unwrap_or(0));
    (byte >> shift) & 1 == 1
}

/// [`raw_sample`] as a `usize`, which is what `Decode`'s tables are indexed by.
fn index_of(row: &[u8], index: usize, bits: u32) -> usize {
    raw_sample(row, index, bits) as usize
}

/// The `index`-th component of a row, at any of §8.9.5.1 Table 87's five bit depths.
///
/// > The value shall be 1 , 2 , 4 , 8 , or (from PDF 1.5) 16 .
///
/// Samples are packed continuously across a row with no padding between them, and a row
/// starts on a byte boundary — which is why the caller computes `row_bytes` and this reads
/// only within one row. The three sub-byte depths divide a byte exactly, so a sample never
/// straddles one; 16 bits is two bytes, most significant first, as every multi-byte integer
/// in a PDF is.
fn raw_sample(row: &[u8], index: usize, bits: u32) -> u32 {
    let byte = |at: usize| u32::from(row.get(at).copied().unwrap_or(0));
    match bits {
        1 => u32::from(sample_bit(row, index)),
        16 => {
            let at = index.saturating_mul(2);
            (byte(at) << 8) | byte(at.saturating_add(1))
        }
        8 => byte(index),
        // 2 and 4: `per` samples to a byte, most significant first.
        _ => {
            let per = usize::try_from(8u32.checked_div(bits).unwrap_or(8))
                .unwrap_or(8)
                .max(1);
            let at = index.checked_div(per).unwrap_or(0);
            let within = u32::try_from(index.checked_rem(per).unwrap_or(0)).unwrap_or(0);
            let shift = 8u32
                .saturating_sub(bits)
                .saturating_sub(within.saturating_mul(bits));
            (byte(at) >> shift) & (1u32 << bits).saturating_sub(1)
        }
    }
}

fn channel(value: f32) -> u8 {
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "clamped to 0.0..=1.0 and scaled, so the conversion is exact"
    )]
    {
        // `+ 0.5` and a truncating cast rather than `.round()`, which is the *same* answer
        // on this domain — the value is clamped non-negative, where round-half-away-from-zero
        // and round-half-up agree — and does not call `roundf`. This runs once per component
        // per pixel and callgrind put the library call at 10.7% of interpreting one
        // 2500x1364 photograph, 60 instructions a pixel to round three numbers.
        (value.clamp(0.0, 1.0) * 255.0 + 0.5) as u8
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
    at: Dictionaries,
    source: &ImageStream,
    width: u32,
    height: u32,
    is_mask: bool,
    fill: pdf_render::Color,
    into: Compositing,
) -> Result<Vec<u8>, ImageError> {
    let Dictionaries {
        document,
        dict,
        resources,
    } = at;
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
        colour_space(document, dict, resources, into)?
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
            into,
        },
    )
}

/// Decodes a CCITT fax-encoded image through the sandbox.
///
/// ISO 32000-2 §7.4.6. Everything the decoder needs is in the stream's `/DecodeParms` (Table
/// 5), whose CCITT entries Table 11 defines, and
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
    at: Dictionaries,
    source: &ImageStream,
    width: u32,
    height: u32,
    is_mask: bool,
    fill: pdf_render::Color,
    into: Compositing,
) -> Result<Vec<u8>, ImageError> {
    let Dictionaries {
        document,
        dict,
        resources,
    } = at;
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
        colour_space(document, dict, resources, into)?
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
            into,
        },
    )
}

/// Decodes a JPEG 2000 image through the sandbox.
///
/// ISO 32000-2 §7.4.9. Returns the samples and whether an opacity channel came with them,
/// which decides whether an `/SMask` may still be applied.
fn decode_jpx(
    at: Dictionaries,
    source: &ImageStream,
    width: u32,
    height: u32,
    is_mask: bool,
    fill: pdf_render::Color,
    into: Compositing,
) -> Result<(Vec<u8>, bool), ImageError> {
    let Dictionaries {
        document,
        dict,
        resources,
    } = at;
    // Resolved before the request, because whether the codestream's own palette should be
    // applied depends on it: §7.4.9 gives `/ColorSpace` precedence over every colour
    // specification in the JPEG 2000 data, a palette included.
    let declared = document.get_key(dict, "ColorSpace");
    let declared_space = if matches!(declared, Object::Null) {
        None
    } else {
        Some(
            crate::colour::ColourSpace::parse(document, &declared, resources).ok_or_else(|| {
                ImageError::UnsupportedColourSpace {
                    space: space_name(&declared),
                }
            })?,
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
                    // A stencil carries no colour, so nothing here depends on what the
                    // samples are composited into; the fill has been redirected already.
                    into: Compositing::Device,
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
        jpx_samples_to_rgba(&raster, &space, use_opacity, premultiplied, into),
        use_opacity,
    ))
}

/// Converts decoded JPEG 2000 samples into straight-alpha RGBA8.
fn jpx_samples_to_rgba(
    raster: &pdf_sandbox::Raster,
    space: &crate::colour::ColourSpace,
    use_opacity: bool,
    premultiplied: bool,
    into: Compositing,
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
        let colour = into.paint(space, &values, true);
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

/// Undoes Adobe's APP14 transform 2, turning four YCCK channels into the four CMYK ones.
///
/// # Why the colour space is chosen here rather than left to the decoder
///
/// `zune-jpeg` converts to RGB by default, and for a four-component codestream that means it
/// applies a **CMYK to RGB conversion of its own** — `blinn_8x8(c, k)`, which is
/// `(1 − C)(1 − K)` on samples it assumes are stored inverted, the convention a *standalone*
/// Adobe CMYK JPEG follows. Two things are wrong with letting that happen. It is a second
/// route from colour to pixels, which trap 6 in `doc/HANDOVER.md` forbids outright —
/// `ColourSpace::to_rgb` is the only place a colour becomes RGB — and inside a PDF the
/// inversion is not the marker's to state. §8.9.5.2's `/Decode` array says what a sample
/// means, its Table 87 default for `DeviceCMYK` is `[0 1 0 1 0 1 0 1]`, and §7.4.8 defers to
/// Adobe Technical Note #5116 for *markers* and the YCbCr/YCCK colour transform, not for the
/// polarity of a sample.
///
/// `cmykjpeg.pdf` is the corpus witness and it settles it: an Adobe-marked four-component
/// JPEG with no `/Decode`, whose samples are ordinary CMYK. Read as inverted, its sky comes
/// out **black**; read as the clause states, it is the photograph all four references draw.
/// So a CMYK codestream is asked for as CMYK and converted where every other colour is.
/// Adobe's APP14 transform 2, in place: four YCCK channels become the four CMYK ones.
///
/// §7.4.8 hands a `DCTDecode` codestream's syntax to ISO/IEC 10918, and the four-component
/// transform is Adobe's extension to it (APP14, `transform = 2`): the first three channels carry
/// a JFIF luminance-chrominance transform of what would otherwise be the first three *inverted*
/// CMYK channels, and the fourth carries K unchanged. So undoing it is JFIF's own YCbCr → RGB
/// followed by the inversion the convention already assumes:
///
/// ```text
/// C = 255 − R    M = 255 − G    Y = 255 − B    K = K
/// ```
///
/// **The inversion is not undone here, and that is the point.** An Adobe four-component JPEG
/// stores CMYK inverted whichever transform it uses, and a PDF that means the ordinary reading
/// says so with `/Decode [1 0 1 0 1 0 1 0]` — §8.9.5.2's entry, applied by
/// [`apply_decode_to_channels`] one step later. A decoder that un-inverted here would undo the
/// file's `/Decode` twice. What this function owes is only that transform 2 and transform 0
/// deliver the *same convention*, which is what libjpeg's `ycck_cmyk_convert` also does.
///
/// **Why not ask `zune-jpeg` for CMYK directly**: it has no `YCCK → CMYK` conversion. Its two
/// YCCK arms both go to RGB and composite the black channel in on the way, which throws away the
/// component §8.9.5.1 needs `/ColorSpace` to interpret. Asking for `YCCK` out takes the raw four.
///
/// The witness is outside this repository — a 92-page commercial catalogue whose every page is one
/// such image, and which drew nothing at all until this (`doc/todo/28`).
fn ycck_to_cmyk(pixels: &mut [u8]) {
    for pixel in pixels.chunks_exact_mut(4) {
        let (Some(y), Some(cb), Some(cr)) = (
            pixel.first().copied(),
            pixel.get(1).copied(),
            pixel.get(2).copied(),
        ) else {
            continue;
        };
        let (y, cb, cr) = (f32::from(y), f32::from(cb) - 128.0, f32::from(cr) - 128.0);
        // ITU-T T.871's inverse, which is the one JFIF states and the one every JPEG decoder
        // implements; the clamp is the standard's own, since the transform's range exceeds a byte.
        let clamp = |value: f32| {
            #[expect(
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                reason = "clamped to 0..=255 on the line above, which is what makes it a byte"
            )]
            let byte = value.clamp(0.0, 255.0).round() as u8;
            255_u8.saturating_sub(byte)
        };
        let red = clamp(1.402_f32.mul_add(cr, y));
        let green = clamp((-0.714_136_f32).mul_add(cr, (-0.344_136_f32).mul_add(cb, y)));
        let blue = clamp(1.772_f32.mul_add(cb, y));
        if let Some(three) = pixel.get_mut(..3) {
            three.copy_from_slice(&[red, green, blue]);
        }
    }
}

fn decode_jpeg(data: &[u8], width: u32, height: u32) -> Result<(Vec<u8>, usize), ImageError> {
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
    // **Four components stay four**, whichever of the two ways a codestream spells them.
    //
    // `zune-jpeg`'s default output is RGB, and its own conversions for both four-component inputs
    // composite the black channel away — which is exactly what §8.9.5.1 must not have: the
    // dictionary's `/ColorSpace` is what interprets the components, and a decoder that had
    // already turned them into three has answered the clause's question for it.
    //
    // `CMYK` is Adobe's APP14 transform 0 and needs only asking. `YCCK` is transform 2, where the
    // first three channels are a luminance-chrominance transform of the other three and the
    // fourth is carried alongside — and asking for `YCCK` out gets the four *raw* channels, which
    // is why the conversion below is here rather than in the decoder.
    //
    // **The frame's component count decides this, and the APP14 transform code does not** — which
    // is what this condition got wrong until the four-hundred-and-thirtieth session. §7.4.8 says
    // where the number comes from:
    //
    // > The values of these parameters, which include the dimensions of the image and the number
    // > of components per sample, are entirely under the control of the encoder and shall be
    // > stored in the encoded data.
    //
    // and Table 13 states the transform in terms of that number rather than the other way round —
    // "If the image has four components, CMYK values shall be transformed to YCbCrK before
    // encoding and from YCbCrK to CMYK after decoding" — so transform 0, "No transformation",
    // says only that nothing was applied. `zune-jpeg` maps it to `CMYK` at the marker and
    // *defers* the correction to a three-component `RGB` until it reads the frame, so
    // `input_colorspace()` is provisional here and `info.components` is not. Asking a
    // three-component codestream for four components made every such image an
    // `Unimplemented colorspace mapping from RGB to CMYK` — 21 images over four documents of a
    // 4000-document web sample, whole photographs lost (ADR 0266).
    let input = decoder.input_colorspace();
    let four = info.components == 4
        && matches!(
            input,
            Some(
                zune_jpeg::zune_core::colorspace::ColorSpace::CMYK
                    | zune_jpeg::zune_core::colorspace::ColorSpace::YCCK
            )
        );
    if let Some(space) = input.filter(|_| four) {
        decoder.set_options(
            zune_jpeg::zune_core::options::DecoderOptions::default().jpeg_set_out_colorspace(space),
        );
    }

    let components = decoder
        .output_colorspace()
        .map_or(3, |space| space.num_components());
    let mut pixels = decoder.decode().map_err(|e| ImageError::Malformed {
        detail: format!("JPEG data: {e}"),
    })?;
    // Gated on `four` for the same reason: a *three*-component frame whose marker says transform 2
    // is read as `YCbCr` by the decoder, and running the four-channel conversion over three
    // channels would walk a `chunks_exact_mut(4)` across pixel boundaries rather than refuse.
    if four && input == Some(zune_jpeg::zune_core::colorspace::ColorSpace::YCCK) {
        ycck_to_cmyk(&mut pixels);
    }
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
        // Four components stay four: they are `/ColorSpace`'s to interpret, and
        // `convert_channels` is where they become pixels. The alpha byte carries `k` until
        // then, which is why that conversion restores it rather than leaving it alone.
        4 => {
            for (destination, source) in out.chunks_exact_mut(4).zip(pixels.chunks_exact(4)) {
                destination.copy_from_slice(source);
            }
        }
        _ => {
            return Err(ImageError::UnsupportedColourSpace {
                space: format!("{components}-component JPEG"),
            });
        }
    }

    Ok((out, components))
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
fn apply_decode_to_channels(
    document: &Document,
    dict: &Dictionary,
    components: usize,
    rgba: &mut [u8],
) {
    if document.get_key(dict, "Decode").as_array().is_none() {
        return;
    }
    // One or three components are device channels whatever `/ColorSpace` names, which is the
    // approximation this route takes and which both become RGB here; four are CMYK, and the
    // clause's default ranges are the same `[0 1]` per component either way. The space is
    // named rather than resolved because `Decode::read` wants it only for those defaults and
    // for how many pairs to expect.
    let space = if components == 4 {
        ColourSpace::Cmyk
    } else {
        ColourSpace::Rgb
    };
    let decode = Decode::read(document, dict, &space, 8);
    if decode.is_identity() {
        return;
    }
    let grey =
        matches!(document.get_key(dict, "Decode").as_array(), Some(items) if items.len() < 6);
    // A greyscale JPEG's one component was written to all three channels, so three is right
    // for one component as well as for three; four components are still four channels.
    let touched = if components == 4 { 4 } else { 3 };
    for pixel in rgba.chunks_exact_mut(4) {
        for (component, channel) in pixel.iter_mut().take(touched).enumerate() {
            let pair = if grey { 0 } else { component };
            *channel = decode.channel(pair, usize::from(*channel));
        }
    }
}

/// Converts a `DCTDecode` raster from the space its dictionary names into device RGB.
///
/// The other four routes decide what a sample means in [`unpack`], through
/// `crate::colour::ColourSpace::to_rgb`; this one had `zune-jpeg`'s components written
/// straight into the raster as though `/ColorSpace` said `DeviceGray` or `DeviceRGB`. For
/// most JPEGs it does — but §8.6.5.5's `ICCBased` and §8.6.5.6's `/DefaultRGB` both put a
/// space there that means something else, and a page cannot tell you which it got.
///
/// A JPEG's components are eight-bit integers, so the conversion is a lookup for a
/// one-component space and a per-pixel call for three. Nothing runs at all where the space
/// is the device one the decoder already delivered, which is every corpus JPEG but a
/// handful.
fn convert_channels(
    at: Dictionaries,
    is_mask: bool,
    components: usize,
    rgba: &mut [u8],
    into: Compositing,
) -> Result<(), ImageError> {
    if is_mask {
        return Ok(());
    }
    // An unreadable space is reported by the ordinary route rather than twice.
    let Ok(space) = colour_space(at.document, at.dict, at.resources, into) else {
        return Ok(());
    };
    let space = match space {
        // Grey and RGB are what the decoder already produced. `DeviceCMYK` is *not*:
        // `decode_jpeg` hands over the four components untouched, on purpose, so that
        // §8.6.4.4's four numbers become a pixel where every other colour does. Inside a
        // mask group none of the three is what the decoder produced — `ColourSpace::reduced`
        // does not reduce there — so this early return is unreachable and the arms below
        // convert every sample, which is what the group is composited in.
        ColourSpace::Gray | ColourSpace::Rgb | ColourSpace::Mask => return Ok(()),
        ColourSpace::Cmyk => crate::colour::ColourSpace::Cmyk,
        ColourSpace::Resolved(space) => space,
    };
    // The raster's channel count and the space's need not match, and where they do not it is
    // usually not a defect: a greyscale JPEG is delivered as three equal channels, so a
    // one-component space reads the first of them. What cannot be reconciled is a space of
    // three or four components against a codestream of a different number, which is the
    // dictionary and the codestream contradicting each other.
    match (space.components(), components) {
        (1, _) => {
            // The same table `palette` builds, for the same reason: 256 possible samples
            // against a photograph's millions.
            let table: Vec<[u8; 3]> = (0..=255u8)
                .map(|value| {
                    let colour = into.paint(&space, &[f32::from(value) / 255.0], true);
                    [channel(colour.r), channel(colour.g), channel(colour.b)]
                })
                .collect();
            for pixel in rgba.chunks_exact_mut(4) {
                let grey = usize::from(pixel.first().copied().unwrap_or(0));
                if let Some(rgb) = pixel.get_mut(..3)
                    && let Some(converted) = table.get(grey)
                {
                    rgb.copy_from_slice(converted);
                }
            }
            Ok(())
        }
        (3, 3) => {
            convert_three(&space, rgba, band_pixels(rgba.len() / 4), into);
            Ok(())
        }
        (4, 4) => {
            convert_four(&space, rgba, band_pixels(rgba.len() / 4), into);
            Ok(())
        }
        (wanted, got) => Err(ImageError::UnsupportedColourSpace {
            space: format!("a {wanted}-component space on a JPEG of {got} components"),
        }),
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
fn mask_entry(document: &Document, dict: &Dictionary, resources: &Dictionary) -> MaskEntry {
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
        Object::Array(items) => colour_key_entry(document, dict, resources, items),
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
fn colour_key_entry(
    document: &Document,
    dict: &Dictionary,
    resources: &Dictionary,
    items: &[Object],
) -> MaskEntry {
    if matches!(document.get_key(dict, "ImageMask"), Object::Boolean(true)) {
        return MaskEntry::Unusable(
            "colour-key /Mask on an image mask, which has no colour components".to_owned(),
        );
    }
    if let Some(codec) = image_codec(document, dict) {
        return MaskEntry::Unusable(format!("colour-key /Mask on a {codec} image"));
    }
    // Asked for its component count and nothing else, so what the samples are composited
    // into does not enter: `Compositing::Device` is the question rather than an assumption.
    let Ok(space) = colour_space(document, dict, resources, Compositing::Device) else {
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
pub fn unapplied_mask(
    document: &Document,
    dict: &Dictionary,
    resources: &Dictionary,
) -> Option<String> {
    match mask_entry(document, dict, resources) {
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
    resources: &Dictionary,
    stream: &Stream,
) -> Result<Image, ImageError> {
    let stencil = decode(
        document,
        stream,
        resources,
        pdf_render::Color::BLACK,
        // A stencil is read for where it marks, so it carries no colour to redirect.
        Compositing::Device,
    )
    .map_err(|error| ImageError::Malformed {
        detail: format!("/Mask did not decode: {error}"),
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
    /// §11.6.5.2, on a grid whose refinement with the image's is too large to build.
    ///
    /// The mask is carried to the backend beside the image instead, and the two are combined
    /// where the device scale is known — which is what §10.7.4 asks for in the first place.
    /// See [`device_scaled_soft_mask`] for what makes a mask eligible.
    AtDeviceScale,
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
fn soft_mask_entry(
    document: &Document,
    dict: &Dictionary,
    resources: &Dictionary,
) -> SoftMaskEntry {
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
    match colour_space(document, &mask.dict, resources, Compositing::Device) {
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
        // The refinement of the two grids is too large to build, which is where §10.7.4's
        // answer — combine at device resolution — stops being an improvement and becomes the
        // only answer. It needs the mask's samples readable at a chosen grid; see
        // [`device_scaled_soft_mask`] for the three things that decides.
        if eligible_for_the_device_scale(document, &mask.dict, resources)
            && matches!(
                matte_colour(document, dict, resources, &mask.dict),
                Matte::Absent
            )
        {
            return SoftMaskEntry::AtDeviceScale;
        }
        return SoftMaskEntry::Unusable(format!(
            "/SMask is {mask_width}x{mask_height} against a {width}x{height} image, needing a \
             grid of {grid} samples"
        ));
    }
    let (matte, owed) = match matte_colour(document, dict, resources, &mask.dict) {
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
fn matte_colour(
    document: &Document,
    dict: &Dictionary,
    resources: &Dictionary,
    mask_dict: &Dictionary,
) -> Matte {
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
    match (
        colour_space(document, dict, resources, Compositing::Device),
        components.as_slice(),
    ) {
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
pub fn unapplied_soft_mask(
    document: &Document,
    dict: &Dictionary,
    resources: &Dictionary,
) -> Option<String> {
    match soft_mask_entry(document, dict, resources) {
        SoftMaskEntry::Unusable(reason)
        | SoftMaskEntry::Image {
            owed: Some(reason), ..
        } => Some(reason),
        // Nothing is owed for a mask carried to the backend: §11.6.5.2 is met there, at the
        // resolution §10.7.4 states, and a report would name a gap that has been closed.
        SoftMaskEntry::Absent
        | SoftMaskEntry::AtDeviceScale
        | SoftMaskEntry::Image { owed: None, .. } => None,
    }
}

/// Whether a soft mask's samples can be read at a grid chosen by the device.
///
/// Three things decide it, and each is the clause's own restriction rather than a convenience:
///
/// - **The stream carries no image codec.** [`unpack`]'s samples are packed rows in the file's
///   own bytes, so any grid can be read out of them by indexing; a `DCTDecode` or `JPXDecode`
///   codestream has to be decoded before a sample has a position at all, and decoding it whole
///   is the cost this route exists to avoid. JPEG 2000 is the one format that *can* decode at a
///   chosen resolution, and asking it to is still owed — `doc/todo/24`.
/// - **The colour space is `DeviceGray`.** Table 143 requires it — "Required; shall be
///   `DeviceGray`" — and it is what makes a sample's opacity a lookup rather than a colour
///   conversion. `soft_mask_entry` tolerates any one-component space for the ordinary route,
///   which decodes through the whole colour module; this route reads a byte.
/// - **The depth is one Table 87 names.** The same five [`unpack`] admits, for the same
///   reason: a depth the standard does not name says nothing about how the bytes are packed.
fn eligible_for_the_device_scale(
    document: &Document,
    mask_dict: &Dictionary,
    resources: &Dictionary,
) -> bool {
    if image_codec(document, mask_dict).is_some() {
        return false;
    }
    if !matches!(
        colour_space(document, mask_dict, resources, Compositing::Device),
        Ok(ColourSpace::Gray)
    ) {
        return false;
    }
    matches!(
        document
            .get_key(mask_dict, "BitsPerComponent")
            .as_integer()
            .unwrap_or(8),
        1 | 2 | 4 | 8 | 16
    )
}

/// §11.6.5.2's soft-mask image, kept in the file's own packed samples.
///
/// The alternative is one byte of RGBA per sample per channel, which is what
/// [`combine_on_the_finer_grid`] would need and what `issue16263.pdf`'s 34862×4332 mask makes
/// impossible: 604 MB for two distinct colours. Packed, the same mask is the 19 MB its
/// `FlateDecode` stream inflates to, and a raster of any grid can be read out of it.
///
/// Everything the read needs is settled before a sample is touched — the layout, and §8.9.5.2's
/// map from an integer sample to a component value — so this holds no document and no lifetime,
/// which is what lets it travel in a display list. That is not incidental: `Document` caches
/// what it parses behind `RefCell`, so it is not `Sync`, and a display list is drawn on every
/// core.
#[derive(Clone)]
pub struct SoftMaskAtDeviceScale {
    /// The stream's bytes with every non-image filter applied, packed as Table 87 describes.
    data: Arc<[u8]>,
    /// The grid the file states.
    width: u32,
    /// The grid the file states.
    height: u32,
    /// Bits per sample: 1, 2, 4, 8 or 16.
    bits: u32,
    /// §8.9.5.2's map from a raw sample to an opacity, as one table.
    ///
    /// Shared because [`MaskCache`] hands the same mask to every command that draws it.
    decode: Arc<Decode>,
}

impl std::fmt::Debug for SoftMaskAtDeviceScale {
    /// The shape of the mask, never its samples: this exists to hold a raster too large to
    /// print, and a display list is printed by `Command`'s own derive.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SoftMaskAtDeviceScale")
            .field("width", &self.width)
            .field("height", &self.height)
            .field("bits", &self.bits)
            .finish_non_exhaustive()
    }
}

impl SoftMaskAtDeviceScale {
    /// This mask over `base`, as one raster a backend produces at the grid it draws at.
    ///
    /// The two rasters travel together from here on, which is the whole of what the display
    /// list needed to be told: §8.9.6.3 and Table 143 both put an image and its mask on the
    /// same unit square without putting them on the same grid, so nothing but the device can
    /// say where a sample of one lies in the other.
    #[must_use]
    pub fn over(self, base: Image) -> pdf_render::DeferredImage {
        pdf_render::DeferredImage::new(Arc::new(MaskedAtDeviceScale { base, mask: self }))
    }

    /// The mask's samples as a grey raster, on a grid no finer than `grid`.
    ///
    /// ISO 32000-2 §10.7.4, of a sampled image drawn at a lower resolution than its own:
    ///
    /// > The position of the centre of such a pixel -in other words, the point whose
    /// > coordinate values have fractional parts of one-half -shall be mapped back into
    /// > source space to determine how to colour the pixel. There shall not be averaging over
    /// > the pixel area. If the resolution of the source image is higher than that of device
    /// > space, some source samples might not be used.
    ///
    /// That is the rule this follows, centre and all: output sample `i` of `cells` reads
    /// source sample `⌊(2i + 1) × samples ÷ 2 cells⌋`. It is a **departure from the departure**
    /// [`pdf_render::Image::area_averaged`] records — this renderer averages a reduced image
    /// where the clause says not to, on ADR 0025's argument — and the difference is deliberate:
    /// that argument rests on a measured witness whose thin features vanish, and averaging here
    /// would mean decoding every one of a mask's samples, which is the cost this whole route
    /// exists to avoid. What it costs is a mask feature thinner than a device pixel, which is
    /// exactly what the clause's last sentence says happens.
    fn raster(&self, grid: pdf_render::Grid) -> Image {
        let cells = self.cells(grid);
        let row_bytes = (self.width as usize)
            .saturating_mul(self.bits as usize)
            .saturating_add(7)
            / 8;
        let mut data = Vec::with_capacity(
            (cells.width as usize)
                .saturating_mul(cells.height as usize)
                .saturating_mul(4),
        );
        for down in 0..u64::from(cells.height) {
            let y = centre(down, u64::from(cells.height), u64::from(self.height));
            let from = y.saturating_mul(row_bytes);
            let row = self
                .data
                .get(from..from.saturating_add(row_bytes))
                .unwrap_or_default();
            for across in 0..u64::from(cells.width) {
                let x = centre(across, u64::from(cells.width), u64::from(self.width));
                let grey = self.decode.channel(0, index_of(row, x, self.bits));
                data.extend_from_slice(&[grey, grey, grey, u8::MAX]);
            }
        }
        Image {
            width: cells.width,
            height: cells.height,
            data: Arc::from(data.as_slice()),
            // Table 143 lists `/Interpolate` as "Optional" and the entry is about magnifying
            // an image's own samples; what a backend does with the *combined* raster is
            // decided by the base image's entry, which is where §8.9.5.3's hint was stated.
            interpolate: false,
        }
    }

    /// The grid this will actually produce: no finer than asked, than stated, or than fits.
    ///
    /// The third bound is the one a document controls. A backend asks for the device pixels the
    /// image covers, which a magnification the user chooses can make arbitrarily large, so the
    /// same [`MAX_MASK_GRID`] that refused the eager combination bounds this one — halving both
    /// axes until the product fits, so the raster stays the shape of the request.
    fn cells(&self, grid: pdf_render::Grid) -> pdf_render::Grid {
        let mut cells = pdf_render::Grid {
            width: grid.width.min(self.width).max(1),
            height: grid.height.min(self.height).max(1),
        };
        while u64::from(cells.width).saturating_mul(u64::from(cells.height)) > MAX_MASK_GRID
            && (cells.width > 1 || cells.height > 1)
        {
            cells.width = (cells.width / 2).max(1);
            cells.height = (cells.height / 2).max(1);
        }
        cells
    }
}

/// §10.7.4's centre of output cell `index` of `cells`, mapped back into a source of `samples`.
///
/// Integer arithmetic, and the last sample is never passed: `cells` is at most `samples`, so
/// `(2 × index + 1) × samples ÷ (2 × cells)` is below `samples` for every `index` below `cells`.
fn centre(index: u64, cells: u64, samples: u64) -> usize {
    let numerator = index
        .saturating_mul(2)
        .saturating_add(1)
        .saturating_mul(samples);
    let at = numerator
        .checked_div(cells.saturating_mul(2))
        .unwrap_or(0)
        .min(samples.saturating_sub(1));
    usize::try_from(at).unwrap_or(0)
}

/// An image and the soft mask that belongs with it, combined where the device scale is known.
#[derive(Debug)]
struct MaskedAtDeviceScale {
    base: Image,
    mask: SoftMaskAtDeviceScale,
}

impl pdf_render::ImageAtDeviceScale for MaskedAtDeviceScale {
    /// §11.6.5.2's mask applied to the base image, both resolved onto the device's grid.
    ///
    /// The combination itself is [`combine_on_the_finer_grid`]'s, unchanged: once the mask has
    /// been read at a grid the device can use, the two rasters are an ordinary pair and the one
    /// rule this crate has for combining a pair applies. What is different is only which grid
    /// the pair is on, which is the whole of what deferring bought.
    ///
    /// No `/Matte`: Table 143 makes the mask's `/Width` and `/Height` "the same as the ... value
    /// of the parent image" wherever one is present, so a pair whose grids differ enough to
    /// reach this route cannot have one, and `soft_mask_entry` checks it rather than assuming.
    fn samples(&self, grid: pdf_render::Grid) -> Image {
        let mask = self.mask.raster(grid);
        combine_on_the_finer_grid(&self.base, &mask, |colour, sample| {
            (colour, sample.first().copied().unwrap_or(0))
        })
    }
}

/// Soft masks already read for the device to place, keyed by the object that states them.
///
/// # Why this exists, with the measurement that asked for it
///
/// The same reason [`crate::shading::Cache`] does, one clause over: a page draws the same
/// `XObject` many times and the samples do not depend on where. `issue16263.pdf` runs `Do` on
/// one image **55 times**, and its mask's `FlateDecode` stream inflates to 18.9 MB — so
/// reading it per painting operation put **750 MB** through one 960×540 page against 15.6 MB
/// before this route existed, and 27 MB with the cache. Nothing about the answer changes:
/// every input is the mask object's own — its packed bytes, its `/BitsPerComponent`, its
/// `/Decode` — which is what makes the object's identity a sound key here where a shading
/// needed its colour space in the key as well.
///
/// A mask with no object number of its own — one written directly into the image dictionary —
/// is not cached and is read each time, which is exact rather than approximately right.
#[derive(Debug, Default)]
pub struct MaskCache {
    read: std::collections::BTreeMap<ObjectId, SoftMaskAtDeviceScale>,
}

impl MaskCache {
    /// Reads a mask, reusing an earlier read of the same object.
    fn read(
        &mut self,
        document: &Document,
        dict: &Dictionary,
        resources: &Dictionary,
    ) -> Option<SoftMaskAtDeviceScale> {
        let key = dict.get("SMask").and_then(Object::as_reference);
        if let Some(id) = key
            && let Some(mask) = self.read.get(&id)
        {
            return Some(mask.clone());
        }
        let mask = device_scaled_soft_mask(document, dict, resources)?;
        if let Some(id) = key {
            self.read.insert(id, mask.clone());
        }
        Some(mask)
    }
}

/// Reads a soft mask this crate will combine at device resolution, or `None`.
///
/// `None` where the entry is not that kind of mask, and where its stream will not decode — the
/// second being the one case [`unapplied_soft_mask`] does not cover and does not here either:
/// an image visibly present and opaque beats one dropped entirely, which is the same choice
/// [`apply_soft_mask`] makes for the same reason.
fn device_scaled_soft_mask(
    document: &Document,
    dict: &Dictionary,
    resources: &Dictionary,
) -> Option<SoftMaskAtDeviceScale> {
    if !matches!(
        soft_mask_entry(document, dict, resources),
        SoftMaskEntry::AtDeviceScale
    ) {
        return None;
    }
    let smask = document.get_key(dict, "SMask");
    let mask = smask.as_stream()?;
    let mask_dict = &mask.dict;
    let dimension = |key| {
        document
            .get_key(mask_dict, key)
            .as_integer()
            .and_then(|value| u32::try_from(value).ok())
            .filter(|value| *value > 0)
    };
    let (width, height) = (dimension("Width")?, dimension("Height")?);
    let bits = u32::try_from(
        document
            .get_key(mask_dict, "BitsPerComponent")
            .as_integer()
            .unwrap_or(8),
    )
    .ok()?;
    // `eligible_for_the_device_scale` established that the space is `DeviceGray`, which is the
    // one this reads a byte out of.
    let space = ColourSpace::Gray;
    let decode = Decode::read(document, mask_dict, &space, bits);
    let source = document.image_stream(mask)?;
    Some(SoftMaskAtDeviceScale {
        data: source.data,
        width,
        height,
        bits,
        decode: Arc::new(decode),
    })
}

/// Applies §11.6.5.2's soft mask: each of its samples is the image's opacity there.
///
/// A soft mask that cannot be read leaves the image opaque rather than failing it: an
/// opaque image is visibly present and slightly wrong, whereas dropping it loses content
/// entirely.
fn apply_soft_mask(
    document: &Document,
    dict: &Dictionary,
    resources: &Dictionary,
    image: Image,
) -> Image {
    let SoftMaskEntry::Image {
        stream: mask_stream,
        matte,
        ..
    } = soft_mask_entry(document, dict, resources)
    else {
        return image;
    };
    let Ok(mask) = decode(
        document,
        &mask_stream,
        resources,
        pdf_render::Color::BLACK,
        // §11.6.5.2's mask is read for its one channel of opacity, not for colour.
        Compositing::Device,
    ) else {
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

#[cfg(test)]
mod tests {
    use super::{Conversion, convert_three};
    use crate::colour::ColourSpace;

    /// A calibrated space whose conversion is neither the identity nor a device one.
    ///
    /// `CalRGB` with the sRGB primaries and a gamma of 2.2, which puts every sample through
    /// [`crate::colour::cie_to_srgb`] and so through the transcendental functions the memo
    /// exists to avoid — the same shape as the `ICCBased` photograph this split was measured on,
    /// without needing a profile in a test.
    fn calibrated() -> ColourSpace {
        ColourSpace::CalRgb {
            white: [0.9505, 1.0, 1.089],
            black: [0.0; 3],
            gamma: [2.2; 3],
            matrix: [
                0.4124, 0.2126, 0.0193, 0.3576, 0.7152, 0.1192, 0.1805, 0.0722, 0.9505,
            ],
        }
    }

    /// A photograph-shaped raster: bands of repeated colour, with a run that crosses every cut.
    fn raster(pixels: usize) -> Vec<u8> {
        (0..pixels)
            .flat_map(|index| {
                #[expect(
                    clippy::cast_possible_truncation,
                    reason = "a sample value, deliberately wrapped to repeat"
                )]
                let value = (index % 37) as u8;
                [value, value.wrapping_mul(3), value.wrapping_add(200), 255]
            })
            .collect()
    }

    /// **The picture does not depend on how the raster was divided.**
    ///
    /// The property the parallel split rests on, checked the way `render-cpu`'s
    /// `strip_parallelism` checks its own: run the same conversion at several band sizes and
    /// demand the bytes are identical. It holds here for a reason that does *not* hold there —
    /// [`Conversion`] memoises a pure function of a sample tuple, so a band boundary changes
    /// which conversions are *repeated* and never which answer is given, while a curve clipped
    /// by a strip's edge is genuinely a different curve (ADR 0138).
    ///
    /// Band sizes 1 and 3 are the interesting ones: they make the memo miss almost every time,
    /// which is exactly the case a wrong key would show up in.
    #[test]
    fn a_band_boundary_changes_no_pixel() {
        let space = calibrated();
        let pixels = 5_000;
        let mut serial = raster(pixels);
        convert_three(
            &space,
            &mut serial,
            None,
            crate::colour::Compositing::Device,
        );
        // A conversion that changed nothing would pass every comparison below.
        assert_ne!(serial, raster(pixels), "the space converts nothing");

        for band in [1, 3, 64, 999, pixels, pixels * 2] {
            let mut split = raster(pixels);
            convert_three(
                &space,
                &mut split,
                Some(band),
                crate::colour::Compositing::Device,
            );
            assert_eq!(split, serial, "band of {band} pixels");
        }
    }

    /// Adobe's APP14 transform 2, arithmetic first — §7.4.8's codestream through ISO/IEC 10918.
    ///
    /// Three values whose answers are exact and need no reference: white, black, and a colour
    /// whose YCbCr is a whole number in every channel. What is being checked is that the four
    /// channels stay four and that the transform is JFIF's inverse followed by the inversion an
    /// Adobe four-component JPEG already assumes — the same convention transform 0 delivers, so
    /// that a file's own `/Decode [1 0 1 0 1 0 1 0]` undoes exactly one of them.
    ///
    /// **No corpus document carries a YCCK JPEG**, which is why this is arithmetic rather than a
    /// picture. The witness is a 92-page commercial catalogue outside this repository whose every
    /// page is one such image and which drew nothing at all until this (`doc/todo/28`); on its
    /// first page ours is 0.0113 from `poppler` where `mupdf` is 0.0384, so the picture is
    /// checked — it is simply not checkable *here*.
    #[test]
    fn a_ycck_codestreams_four_channels_stay_four() {
        // Y = 255, Cb = Cr = 128 is white: R = G = B = 255, so C = M = Y = 0.
        let mut white = [255, 128, 128, 17];
        super::ycck_to_cmyk(&mut white);
        assert_eq!(white, [0, 0, 0, 17], "K passes through untouched");

        // Y = 0 is black: R = G = B = 0, so C = M = Y = 255.
        let mut black = [0, 128, 128, 200];
        super::ycck_to_cmyk(&mut black);
        assert_eq!(black, [255, 255, 255, 200]);

        // And a chrominance that separates the channels, so that a transform which dropped Cb or
        // Cr would fail rather than pass by symmetry. Y = 128, Cb = 128, Cr = 255: red is
        // 128 + 1.402 × 127 = 306, clamped to 255, so C is 0; blue is 128 + 1.772 × 0 = 128.
        let mut red = [128, 128, 255, 0];
        super::ycck_to_cmyk(&mut red);
        assert_eq!(red[0], 0, "cyan: the red channel saturated");
        assert!(
            red[1] > 200,
            "magenta: green fell a long way, got {}",
            red[1]
        );
        assert_eq!(red[2], 127, "yellow: blue is unmoved at 128");
        assert_eq!(red[3], 0);
    }

    /// §10.7.4's centre rule, which is what maps a device pixel back to a mask sample.
    ///
    /// > The position of the centre of such a pixel -in other words, the point whose
    /// > coordinate values have fractional parts of one-half -shall be mapped back into
    /// > source space to determine how to colour the pixel.
    ///
    /// Three properties, and the first is the one a corner rule fails: a grid that is not
    /// reduced at all reads every sample once and in order, so nothing shifts when a mask
    /// happens to be the size the device wants. The second is the rule itself at a four-fold
    /// reduction — cell `i` of 2 over 8 samples reads the sample at `(2i + 1) x 8 / 4`, which
    /// is 2 and 6, the centres of the two halves rather than their corners. The third is that
    /// no index ever leaves the source, which is what a bounds check would otherwise have to
    /// do once per sample.
    #[test]
    fn a_device_cell_reads_the_sample_under_its_centre() {
        for index in 0..8u64 {
            assert_eq!(
                super::centre(index, 8, 8),
                usize::try_from(index).expect("a literal under eight"),
                "the identity"
            );
        }
        assert_eq!((super::centre(0, 2, 8), super::centre(1, 2, 8)), (2, 6));
        assert_eq!(
            (
                super::centre(0, 3, 7),
                super::centre(1, 3, 7),
                super::centre(2, 3, 7)
            ),
            (1, 3, 5),
            "an odd reduction that divides nothing evenly"
        );
        for cells in 1..=32u64 {
            for index in 0..cells {
                assert!(super::centre(index, cells, 32) < 32);
            }
        }
    }

    /// The memo answers with what the conversion would have answered, and nothing else.
    ///
    /// Direct-mapped with no chaining, so a collision must *miss* rather than return the
    /// occupant — a table that answered the wrong colour would be invisible on a photograph and
    /// obvious on nothing.
    #[test]
    fn the_memo_answers_only_for_the_key_it_holds() {
        let mut cache = Conversion::for_pixels(64);
        let colour = pdf_render::Color {
            r: 0.25,
            g: 0.5,
            b: 0.75,
            a: 1.0,
        };
        cache.put(7, colour);
        assert_eq!(cache.get(7), Some(colour));
        assert_eq!(cache.get(8), None, "a key the table does not hold");
    }
}

//! `JPXDecode` (ISO 32000-2 §7.4.9), decoded by the sandboxed worker.
//!
//! The image dictionary's `/ColorSpace` is optional here and, where absent, the codestream's own
//! colour specification governs; `/BitsPerComponent` is ignored either way, and `/Decode` follows
//! `/ColorSpace`'s absence rather than the filter (ADR 0468).

use std::sync::Arc;

use pdf_sandbox::{Decoded, Request};
use pdf_syntax::{Dictionary, Document, ImageStream, Object, Stream};

use crate::colour::{Conversion, ExtendedEncoding, ExtendedSrgb};

use super::ImageError;
use super::combine::alpha_on_grid;
use super::convert::channel;
use super::device_scale::SoftMaskAtDeviceScale;
use super::dispatch::{SamplesOnGrid, positive_integer};
use super::space::{ColourSpace, Decode, Dictionaries, space_name};
use super::unpack::{Painting, Prematte, Samples, pack_bits, unpack};

/// Decodes a JPEG 2000 image through the sandbox.
///
/// ISO 32000-2 §7.4.9. Returns the samples, the grid they are on, and whether an opacity
/// channel came with them, which decides whether an `/SMask` may still be applied.
///
/// The grid is the codec's answer rather than the dictionary's, because a codestream over
/// the confined worker's budget comes back at one of its own reduced resolution levels
/// (§7.4.9 NOTE 3) — `issue19517.pdf`'s 12608×16806 scan decodes at 3152×4202. The
/// dictionary's `/Width` and `/Height` are still checked, against the grid the codestream
/// *states*, which the worker reports beside the raster for exactly that purpose.
pub(super) fn decode_jpx(
    at: Dictionaries,
    source: &ImageStream,
    (width, height): (u32, u32),
    painting: Painting,
) -> Result<SamplesOnGrid, ImageError> {
    let Dictionaries {
        document,
        dict,
        resources,
    } = at;
    // Resolved before the request: §7.4.9 gives `/ColorSpace` precedence over every colour
    // specification in the JPEG 2000 data, a palette included, so it decides what is decoded.
    let declared = document.get_key(dict, "ColorSpace");
    let declared_space = if matches!(declared, Object::Null) {
        None
    } else {
        // Under the page's output intent, as [`colour_space`] parses every other image's:
        // the codestream's colour specification is what §7.4.9 lets `/ColorSpace` override,
        // and a `/DeviceCMYK` stated here means what it means on this page.
        Some(
            painting
                .into
                .parse_space(document, &declared, resources)
                .ok_or_else(|| ImageError::UnsupportedColourSpace {
                    space: space_name(&declared),
                })?,
        )
    };
    let indices = matches!(
        declared_space,
        Some(crate::colour::ColourSpace::Indexed { .. })
    );

    let choice = jpx_colour_choice(&source.data, declared_space.is_some());
    let decoded = pdf_sandbox::decode(&Request::Jpx {
        data: &choice.data,
        indices,
    })
    .map_err(|error| ImageError::Sandboxed {
        detail: error.to_string(),
    })?;
    let Decoded::Raster(mut raster) = decoded else {
        return Err(ImageError::Malformed {
            detail: "JPEG 2000 did not decode to component samples".to_owned(),
        });
    };

    // §7.4.9: "Width and Height shall match the corresponding width and height values in
    // the JPEG 2000 data." The values *in the data* are what the codestream states, which
    // the worker reports beside the raster — a decode at a reduced resolution level changes
    // the raster without changing the data's statement, and comparing against the raster
    // would fail exactly the files the reduction exists to draw.
    if (raster.stated_width, raster.stated_height) != (width, height) {
        return Err(ImageError::Malformed {
            detail: format!(
                "JPEG 2000 data states {}x{} but the dictionary says {width}x{height}",
                raster.stated_width, raster.stated_height
            ),
        });
    }

    // §8.9.5.1 Table 87, `/SMaskInData`: code 1 and 2 both mean the samples carry opacity,
    // and 2 additionally means the colour components were multiplied by it. Absent or 0
    // means ignore any that is there.
    let smask_in_data = document
        .get_key(dict, "SMaskInData")
        .as_integer()
        .unwrap_or(0);
    let opacity_channel = JpxOpacity {
        use_opacity: smask_in_data != 0 && raster.has_opacity,
        premultiplied: smask_in_data == 2,
    };

    if painting.is_mask {
        return jpx_stencil(document, dict, &raster, painting.fill, opacity_channel);
    }

    let stated_by_the_dictionary = declared_space.is_some();
    let extended = extended_on(choice.extended, &raster);
    let space = match (declared_space.or(choice.space), extended) {
        (Some(space), _) => space,
        (None, Some(_)) => ExtendedSrgb::connection_space(),
        (None, None) => codestream_colour_space(&raster)?,
    };

    read_the_supposed_opacity_channel(
        &mut raster,
        &space,
        stated_by_the_dictionary && smask_in_data == 0,
    )?;

    let reading = match extended {
        Some(extended) => JpxReading::Extended(extended),
        None => JpxReading::Mapped(jpx_decode(
            document,
            dict,
            &space,
            stated_by_the_dictionary,
            u32::from(raster.precision),
        )),
    };
    let KeyedSamples {
        ranges: colour_key,
        shortfall,
    } = colour_key_in_the_rasters_domain(painting.colour_key, &raster);
    // §11.6.5.2's inversion pairs the mask's samples with the parent's by position, and
    // Table 143 makes the two grids the dictionary's. A codestream decoded at one of its reduced
    // levels is on another, and the mask is carried onto it over the same footprints
    // ([`alpha_on_grid`], ADR 1324), so the pre-blending is undone there too.
    let grid = (raster.width, raster.height);
    let carried = matte_alpha_on(painting.matte, (width, height), grid);
    let on_grid = (painting.matte.zip(carried.as_deref())).map(|(m, alpha)| m.carried(alpha));
    Ok(SamplesOnGrid {
        black_generation_bypassed: painting.into.bypasses_black_generation(&space),
        rgba: jpx_samples_to_rgba(
            &raster,
            &space,
            &reading,
            opacity_channel,
            colour_key,
            painting.into,
            on_grid.as_ref().or(painting.matte),
        ),
        grid,
        opacity_included: opacity_channel.use_opacity,
        stencil_opacity: None,
        shortfall,
        mask_in_alpha: false,
    })
}

/// A `JPXDecode` image's samples as the confined decoder delivered them, before any colour is
/// made of them: what a writer that must carry the image re-encodes (ADR 1333).
///
/// [`decode`] turns these into an eight-bit RGBA raster for a page; a writer that clears part of
/// the image and keeps the rest (§12.5.6.23's redaction) needs the step before that, because
/// every sample outside the part it clears is to be carried as the file had it. So this stops
/// where [`decode`]'s own JPEG 2000 route starts to convert: the components at the precision
/// Table 87 leaves to the processor — "\[t]he bit depth is determined by the PDF processor in the
/// process of decoding the JPEG 2000 image" — the colour space §7.4.9's precedence names, the
/// `/Decode` pairs a reader maps the samples through, and `/SMaskInData`'s opacity channel apart.
///
/// [`decode`]: super::decode
#[derive(Debug, Clone)]
pub struct JpxSamples {
    /// Columns of the raster, which [`jpx_samples`] has held to the dictionary's `/Width`.
    pub width: u32,
    /// Rows of the raster, held to the dictionary's `/Height`.
    pub height: u32,
    /// Colour components per pixel.
    pub components: usize,
    /// Bits per sample of the widest channel, 1 to 16, which sizes every sample's field: one
    /// byte to eight, two big-endian bytes above.
    pub precision: u8,
    /// Each colour component's own depth, which its integers are at within that field — all
    /// [`Self::precision`] unless §7.4.9's "\[t]he colour components in an image may have
    /// different numbers of bits per sample" applies (ADR 1371).
    pub depths: Vec<u8>,
    /// The opacity channel's own depth, where [`Self::opacity`] is carried.
    pub opacity_depth: u8,
    /// The colour components, interleaved, each at its own depth in [`Self::precision`]'s
    /// field.
    pub colour: Vec<u8>,
    /// Table 87's opacity channel at [`Self::precision`], where `/SMaskInData` is 1 or 2 and the
    /// codestream carries one; `None` where a reader applies none.
    pub opacity: Option<Vec<u8>>,
    /// Whether the colour components were multiplied by that opacity (`/SMaskInData` 2).
    pub premultiplied: bool,
    /// Which colour space the components are in.
    pub space: JpxSpace,
    /// The §8.9.5.2 `/Decode` pair a reader maps each component through at [`Self::precision`]:
    /// the dictionary's where it states a colour space and a pair, Table 88's default otherwise.
    pub decode: Vec<(f32, f32)>,
}

/// The samples a caller of [`jpx_samples`] that states no budget of its own asks for: the most
/// the ordinary confined worker decodes a codestream to at full resolution.
pub const ORDINARY_JPX_SAMPLES: u64 = pdf_sandbox::ORDINARY_SAMPLES;

/// The colour space a `JPXDecode` image's samples are in, by §7.4.9's precedence.
#[derive(Debug, Clone, PartialEq)]
pub enum JpxSpace {
    /// The image dictionary states one, and "any colour space specifications in the JPEG 2000
    /// data shall be ignored" — so the dictionary's entry describes the samples as it stands.
    Stated,
    /// The codestream's, one component.
    Gray,
    /// The codestream's, three components.
    Rgb,
    /// The codestream's, four components.
    Cmyk,
    /// The codestream's own ICC profile, which this crate can read.
    Icc(Vec<u8>),
    /// The codestream's enumerated CIE Lab, T.801 M.11.7.4.1, whose samples are §8.6.5.4's `Lab`
    /// over a `Range` of `[-128 127 -128 127]` under the white point of the illuminant its `IL`
    /// field names (ADRs 1383, 1713).
    Lab {
        /// That white point, as CIE 1931 XYZ: [`crate::jpeg2000::Illuminant::white_point`].
        white: [f32; 3],
    },
}

/// Decodes a `JPXDecode` image's samples through the confined decoder without converting them.
///
/// The steps are [`decode`]'s own up to the colour: §7.4.9's grid check against the data's
/// statement, the same reading of an opacity channel, and the same fallback where the
/// codestream's colour specification cannot be used. What differs is the request: [`decode`]
/// lets a codestream over the decoder's budget come back at a reduced resolution level, which
/// §7.4.9 NOTE 3 permits a viewer, and a raster on that grid is not the image's own. So this
/// asks for the full resolution within `samples` — pixels times channels — and a codestream
/// larger than that is refused by the decoder rather than reduced
/// (`pdf_sandbox::Request::JpxWhole`, ADR 1333).
///
/// # Errors
///
/// See [`ImageError`].
///
/// [`decode`]: super::decode
pub fn jpx_samples(
    document: &Document,
    dict: &Dictionary,
    resources: &Dictionary,
    (data, samples): (&[u8], u64),
) -> Result<JpxSamples, ImageError> {
    let width = positive_integer(document, dict, "Width")?;
    let height = positive_integer(document, dict, "Height")?;
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
    let choice = jpx_colour_choice(data, declared_space.is_some());
    let decoded = pdf_sandbox::decode(&Request::JpxWhole {
        data: &choice.data,
        indices,
        samples,
    })
    .map_err(|error| ImageError::Sandboxed {
        detail: error.to_string(),
    })?;
    let Decoded::Raster(mut raster) = decoded else {
        return Err(ImageError::Malformed {
            detail: "JPEG 2000 did not decode to component samples".to_owned(),
        });
    };
    if (raster.stated_width, raster.stated_height) != (width, height) {
        return Err(ImageError::Malformed {
            detail: format!(
                "JPEG 2000 data states {}x{} but the dictionary says {width}x{height}",
                raster.stated_width, raster.stated_height
            ),
        });
    }
    let smask_in_data = document
        .get_key(dict, "SMaskInData")
        .as_integer()
        .unwrap_or(0);
    let stated = declared_space.is_some();
    // A writer carries these samples under a §8.6 colour space, and PIMA 7667's encodings are
    // none of them: equation 5's offset and the odd extension of equations 6a to 6c put
    // components outside every range a `/Decode` pair into `CalRGB`, `Lab` or an ICC profile's
    // curves can carry. Writing them as the device space of their channel count would recolour
    // an image this crate draws as its codestream states, so the samples are refused by name
    // (ADR 2292).
    if let Some(extended) = extended_on(choice.extended, &raster) {
        return Err(ImageError::UnsupportedColourSpace {
            space: format!(
                "JPEG 2000 {}, which no PDF colour space restates for its samples to be carried in",
                extended_name(extended.encoding())
            ),
        });
    }
    let space = match declared_space.or(choice.space) {
        Some(space) => space,
        None => codestream_colour_space(&raster)?,
    };
    read_the_supposed_opacity_channel(&mut raster, &space, stated && smask_in_data == 0)?;
    let jpx_space = if stated {
        JpxSpace::Stated
    } else {
        match (&space, &raster.colour) {
            (crate::colour::ColourSpace::Lab { white, .. }, _) => JpxSpace::Lab { white: *white },
            (crate::colour::ColourSpace::Icc { .. }, pdf_sandbox::Colour::Icc(profile)) => {
                JpxSpace::Icc(profile.clone())
            }
            (crate::colour::ColourSpace::Rgb, _) => JpxSpace::Rgb,
            (crate::colour::ColourSpace::Cmyk, _) => JpxSpace::Cmyk,
            _ => JpxSpace::Gray,
        }
    };
    let components = usize::from(raster.components);
    let decode = jpx_decode_pairs(
        document,
        stated.then_some(dict),
        &space,
        (components, u32::from(raster.precision)),
    );
    let opacity = smask_in_data != 0 && raster.has_opacity;
    let (colour, opacity_channel) = split_opacity(&raster, raster.has_opacity);
    Ok(JpxSamples {
        width: raster.width,
        height: raster.height,
        components,
        precision: raster.precision,
        depths: (0..components)
            .map(|channel| raster.depth_of(channel))
            .collect(),
        opacity_depth: raster.depth_of(components),
        colour,
        opacity: opacity.then_some(opacity_channel).flatten(),
        premultiplied: opacity && smask_in_data == 2,
        space: jpx_space,
        decode,
    })
}

/// The §8.9.5.2 pair a reader maps each of a `JPXDecode` image's components through: the
/// dictionary's `/Decode` where `dict` is given — the dictionary states the colour space, so
/// its entries describe the samples — and Table 88's default for `space` otherwise.
fn jpx_decode_pairs(
    document: &Document,
    dict: Option<&Dictionary>,
    space: &crate::colour::ColourSpace,
    (components, precision): (usize, u32),
) -> Vec<(f32, f32)> {
    let pairs: Vec<f32> = dict
        .and_then(|dict| {
            document
                .get_key(dict, "Decode")
                .as_array()
                .map(<[Object]>::to_vec)
        })
        .map(|items| {
            items
                .iter()
                .filter_map(|item| document.resolve(item).as_number())
                .map(|value| {
                    #[expect(
                        clippy::cast_possible_truncation,
                        reason = "a component value beyond f32 is clamped to its permitted \
                                  range by every reader either way"
                    )]
                    {
                        value as f32
                    }
                })
                .collect()
        })
        .unwrap_or_default();
    (0..components)
        .map(|component| {
            let at = component.saturating_mul(2);
            match (pairs.get(at), pairs.get(at.saturating_add(1))) {
                (Some(low), Some(high)) => (*low, *high),
                _ => space.default_decode(component, precision),
            }
        })
        .collect()
}

/// A raster's colour components and, where `has_opacity`, its last channel apart.
fn split_opacity(raster: &pdf_sandbox::Raster, has_opacity: bool) -> (Vec<u8>, Option<Vec<u8>>) {
    if !has_opacity {
        return (raster.data.clone(), None);
    }
    let width = if raster.precision > 8 { 2 } else { 1 };
    let colour_bytes = usize::from(raster.components).saturating_mul(width);
    let pixel_bytes = colour_bytes.saturating_add(width);
    let mut colour = Vec::with_capacity(raster.data.len());
    let mut opacity = Vec::with_capacity(raster.data.len().checked_div(pixel_bytes).unwrap_or(0));
    for pixel in raster.data.chunks_exact(pixel_bytes) {
        let (components, alpha) = pixel.split_at(colour_bytes);
        colour.extend_from_slice(components);
        opacity.extend_from_slice(alpha);
    }
    (colour, Some(opacity))
}

/// A `/Matte`'d mask's samples on the grid a `JPXDecode` parent decoded to, where that is not the
/// stated one — §7.4.9 NOTE 3's reduced level — or `None` where no carrying is needed.
fn matte_alpha_on(
    matte: Option<&Prematte>,
    stated: (u32, u32),
    grid: (u32, u32),
) -> Option<Vec<u8>> {
    matte
        .filter(|_| grid != stated)
        .map(|matte| alpha_on_grid(matte.alpha, stated, grid))
}

/// Reads a codestream's supposed opacity channel as colour where ISO 32000-2 §7.4.9 says to.
///
/// §7.4.9, of a dictionary that states its own `/ColorSpace`:
///
/// > If present, it shall determine how the image samples are interpreted, and the colour
/// > space specifications in the JPEG 2000 data shall be ignored. The number of ordinary
/// > colour channels in the JPEG 2000 data shall match the number of components in the
/// > colour space
///
/// *Which* of a codestream's channels is an opacity channel is read off those same colour
/// space specifications — the JP2 channel-definition box sits beside them — so a bare
/// codestream, which carries no boxes at all, has both synthesised for it by the codec. A
/// fourth channel beside a synthesised three-component space is then taken for opacity, and
/// the count the sentence above binds is one short. The clause sets that reading aside, and
/// Table 87 makes it moot from the other side: with `/SMaskInData` 0 or absent, encoded
/// soft-mask information is ignored, so nothing is lost by reading the channel as colour.
///
/// Narrow on purpose, which is trap 11's rule about a condition rather than a count: a file
/// that *states* an opacity channel by writing a non-zero `/SMaskInData` is believed, and so
/// is one whose ordinary channels already match the declared space. Only where taking the
/// supposed opacity channel for an ordinary one makes the clause's sentence true is the
/// codec's guess set aside — which is what `stated_without_opacity` is.
///
/// # Errors
///
/// Where the channel count still disagrees with the colour space's, which is the sentence
/// above unsatisfiable either way round.
fn read_the_supposed_opacity_channel(
    raster: &mut pdf_sandbox::Raster,
    space: &crate::colour::ColourSpace,
    stated_without_opacity: bool,
) -> Result<(), ImageError> {
    if stated_without_opacity
        && raster.has_opacity
        && space.components() != usize::from(raster.components)
        && space.components() == raster.channels()
    {
        raster.components = raster.components.saturating_add(1);
        raster.has_opacity = false;
    }
    if space.components() != usize::from(raster.components) {
        return Err(ImageError::Malformed {
            detail: format!(
                "the colour space takes {} components but the codestream has {}",
                space.components(),
                raster.components
            ),
        });
    }
    Ok(())
}

/// §8.9.6.4's ranges where the raster can hold them, and a sentence where it cannot.
///
/// The ranges were bounded at parse time by the depth [`jpx_sample_domain`] read out of this
/// codestream's headers, and the samples arrive at the depth the confined decoder read out of
/// the same field. The two are one number in every codestream either can read, so a range above
/// what the raster can hold says the two readings parted — and a comparison in a domain the
/// samples are not in is what §8.9.6.4 is least served by. The image is still painted, because
/// the mask is the only part that cannot be answered, and the sentence travels beside it rather
/// than instead of it (ADR 1242).
fn colour_key_in_the_rasters_domain<'a>(
    colour_key: Option<&'a [(u32, u32)]>,
    raster: &pdf_sandbox::Raster,
) -> KeyedSamples<'a> {
    match colour_key {
        Some(ranges) if ranges.iter().any(|(_, high)| *high > raster.max_sample()) => {
            KeyedSamples {
                ranges: None,
                shortfall: Some(format!(
                    "colour-key /Mask ranges bounded by the depth this codestream's headers \
                     state, against samples the decoder delivered at {} bits; the ranges are \
                     not compared and the image is painted whole (§8.9.6.4)",
                    raster.precision
                )),
            }
        }
        other => KeyedSamples {
            ranges: other,
            shortfall: None,
        },
    }
}

/// What [`colour_key_in_the_rasters_domain`] answers: the ranges to apply, and the sentence
/// where they were dropped.
struct KeyedSamples<'a> {
    /// §8.9.6.4's ranges, where the raster's samples are in the domain they are bounded by.
    ranges: Option<&'a [(u32, u32)]>,
    /// See [`Parts::shortfall`].
    ///
    /// [`Parts::shortfall`]: super::Parts::shortfall
    shortfall: Option<String>,
}

/// A JPEG 2000 image whose dictionary makes it a stencil mask.
///
/// ISO 32000-2 §7.4.9's last bullet, of a dictionary whose `/ImageMask` is true:
///
/// > the JPEG 2000 data shall provide a single colour channel with 1-bit samples
///
/// Those samples arrive scaled to eight bits, so the two values are 0 and 255 and the
/// threshold between them is anywhere in between. The raster's own grid throughout, which for
/// a reduced decode is not the dictionary's.
///
/// This is the one place the filter's `/Decode` is read whatever `/ColorSpace` says, which is
/// [`jpx_decode`]'s bullet read from its other end — its closing clause exempts a stencil.
///
/// # And an opacity channel beside it
///
/// A non-zero `/SMaskInData` puts §11.6.5.2's opacity in the codestream, and §7.4.9 says how
/// many: "there shall be only one opacity channel in the JPEG 2000 data and it shall apply to all
/// colour channels". It is read into [`SamplesOnGrid::stencil_opacity`], a plane of its own, and
/// not into the alpha channel, because §11.6.4.2 has already given that channel the stencil's
/// shape — "\[f]or image masks (8.9.6.2, "Stencil masking"), the shape shall be 1.0 for painted
/// areas and 0.0 for masked areas" — and the pair is carried apart for the reason an `/SMask`'s
/// is (ADR 1218, ADR 1279).
///
/// Table 87's code 2 says the data stream's colour channels were premultiplied with the opacity
/// channel, so a stencil sample is compared with the midpoint scaled by its own opacity: `c × a > m × a`
/// is the undone comparison `c > m`, in integers. Where the opacity is zero the product is zero
/// whatever the stencil said, the sample is unrecoverable, and it is read as masked — which marks
/// nothing either way, since its opacity is zero too.
fn jpx_stencil(
    document: &Document,
    dict: &Dictionary,
    raster: &pdf_sandbox::Raster,
    fill: pdf_render::Color,
    opacity_channel: JpxOpacity,
) -> Result<SamplesOnGrid, ImageError> {
    let JpxOpacity {
        use_opacity,
        premultiplied,
    } = opacity_channel;
    let channels = raster.channels();
    let components = usize::from(raster.components);
    let pixels = (raster.width as usize).saturating_mul(raster.height as usize);
    let highest = raster.max_sample();
    let midpoint = highest / 2;
    let opacity_of = |pixel: usize| {
        raster
            .sample(pixel.saturating_mul(channels).saturating_add(components))
            .unwrap_or(highest)
    };
    let samples: Vec<u8> = (0..pixels)
        .map(|pixel| {
            let at = pixel.saturating_mul(channels);
            u8::from(raster.sample(at).is_some_and(|value| {
                if use_opacity && premultiplied {
                    u64::from(value).saturating_mul(u64::from(highest))
                        > u64::from(midpoint).saturating_mul(u64::from(opacity_of(pixel)))
                } else {
                    value > midpoint
                }
            }))
        })
        .collect();
    let stencil_opacity = use_opacity.then(|| {
        SoftMaskAtDeviceScale::grey_plane(
            raster.width,
            raster.height,
            (0..pixels)
                .map(|pixel| scaled_to_byte(opacity_of(pixel), highest))
                .collect(),
        )
    });
    let packed = pack_bits(&samples, raster.width, raster.height);
    Ok(SamplesOnGrid {
        black_generation_bypassed: false,
        rgba: unpack(
            &packed,
            raster.width,
            raster.height,
            &Samples {
                bits: 1,
                space: &ColourSpace::Mask,
                decode: &Decode::read(document, dict, &ColourSpace::Mask, 1),
                // A stencil has no colour components for §8.9.6.4 to range over.
                colour_key: None,
                fill,
                // A stencil carries no colour, so nothing here depends on what the samples are
                // composited into; the fill has been redirected already.
                into: &Conversion::device(),
                // Nor is there a component for Table 144's matte colour to have been blended
                // into: §8.9.6.2's stencil states where the page is marked and nothing else.
                matte: None,
            },
        )?,
        grid: (raster.width, raster.height),
        opacity_included: use_opacity,
        stencil_opacity,
        shortfall: None,
        mask_in_alpha: false,
    })
}

/// §8.9.5.2's map for a JPEG 2000 image, or Table 88's defaults where the clause silences it.
///
/// ISO 32000-2 §7.4.9, the last-but-one of the bullets that constrain an image dictionary:
///
/// > If ColorSpace is absent, then the Decode array shall be ignored unless ImageMask is true
///
/// The condition is `/ColorSpace`'s **absence** and nothing else, and Table 87's own `/Decode`
/// row states it the same way round. So a dictionary that *does* state its colour space states
/// how its samples map into that space too, exactly as it would under any other filter — this
/// is the one entry of the three §7.4.9 rearranges that the codestream does **not** take over,
/// because a codestream says what its samples are and never what a producer meant them to
/// mean. `stated` is that condition, answered by the caller, which has already resolved the
/// entry.
///
/// Where the array is absent Table 88's defaults apply and the map is the identity on every
/// device space, so building it unconditionally costs a table per component and says which
/// sentence is being obeyed. It also carries a space's own *units*, which the flat division by
/// 255 this replaced could not: an `Indexed` space's default is `[0 2^n − 1]`, so an index is
/// passed through rather than turned into a fraction.
///
/// The depth is the raster's own, which Table 87 makes this processor's to determine — "\[t]he
/// bit depth is determined by the PDF processor in the process of decoding the JPEG 2000
/// image" — and which `pdf_sandbox` answers with the codestream's wherever its components
/// agree on one (ADR 1242). §8.9.5.2's map is linear in the sample, so the pair still lands
/// D min at sample 0 and D max at the largest sample, which is what the pair means.
fn jpx_decode(
    document: &Document,
    dict: &Dictionary,
    space: &crate::colour::ColourSpace,
    stated: bool,
    bits: u32,
) -> Decode {
    let mapped = ColourSpace::Resolved(space.clone());
    if stated {
        Decode::read(document, dict, &mapped, bits)
    } else {
        Decode::from_pairs(&[], &mapped, bits)
    }
}

/// What `/SMaskInData` said about the opacity channel that came with the samples.
///
/// Two booleans that are read together and mean nothing apart: Table 87 gives code 1 and code 2
/// the same answer to the first question and different answers to the second.
#[derive(Clone, Copy)]
struct JpxOpacity {
    /// Whether the channel after the colour components is opacity (`/SMaskInData` 1 or 2).
    use_opacity: bool,
    /// Whether the colour components were multiplied by it (`/SMaskInData` 2).
    premultiplied: bool,
}

/// Converts decoded JPEG 2000 samples into straight-alpha RGBA8.
///
/// `colour_key` is §8.9.6.4's ranges, one per colour component, applied here for the reason
/// `jpeg_colour_key` gives on the other lossy filter: the test is on the components "before
/// decoding", which are the samples this loop reads, and the conversion below replaces them.
///
/// **The samples are read at the raster's own precision**, which for a codestream of more than
/// eight bits a component is the codestream's (`pdf_sandbox::Raster::precision`, ADR 1242). So
/// the range test is the comparison §8.9.6.4 states — its integers are in the domain the file
/// wrote them in — and the eight-bit raster this returns is reached afterwards, through
/// §8.9.5.2's map. The opacity channel is scaled here instead, because RGBA8 is what an
/// `Image` carries and an alpha has no `/Decode` entry to carry it.
fn jpx_samples_to_rgba(
    raster: &pdf_sandbox::Raster,
    space: &crate::colour::ColourSpace,
    reading: &JpxReading,
    opacity_channel: JpxOpacity,
    colour_key: Option<&[(u32, u32)]>,
    into: &Conversion,
    matte: Option<&Prematte>,
) -> Vec<u8> {
    let JpxOpacity {
        use_opacity,
        premultiplied,
    } = opacity_channel;
    let channels = raster.channels();
    let components = usize::from(raster.components);
    let pixels = (raster.width as usize).saturating_mul(raster.height as usize);
    let highest = raster.max_sample();
    let mut out = Vec::with_capacity(pixels.saturating_mul(4));
    let mut values = vec![0f32; components];
    for pixel in 0..pixels {
        let base = pixel.saturating_mul(channels);
        let sample = |channel: usize| raster.sample(base.saturating_add(channel));
        // §8.9.6.4: "[s]amples in the image that fall within this range shall not be painted".
        // The same answer `unpack` gives a masked sample — its position, and no opacity at all —
        // taken before the map below, because that is what turns a sample into a colour.
        if colour_key.is_some_and(|ranges| {
            ranges.iter().enumerate().all(|(component, (low, high))| {
                sample(component).is_some_and(|raw| raw >= *low && raw <= *high)
            })
        }) {
            out.extend_from_slice(&[0, 0, 0, 0]);
            continue;
        }
        let alpha = if use_opacity {
            sample(components).map_or(255, |raw| scaled_to_byte(raw, highest))
        } else {
            255
        };
        match reading {
            // The map carries the space's own units with it, which is what the flat division by
            // 255 this replaced could not: Table 88 gives an `Indexed` space `[0 2^n − 1]`, so an
            // index is passed through rather than turned into a fraction, and a `Lab` space's
            // lightness runs to 100.
            JpxReading::Mapped(decode) => {
                for (component, slot) in values.iter_mut().enumerate() {
                    *slot = decode.value(component, sample(component).unwrap_or(0) as usize);
                }
            }
            // PIMA 7667's own equations take the integers to the encoding's components; §7.4.9
            // ignores `/Decode` where the dictionary states no `/ColorSpace`, so no map stands
            // between them.
            JpxReading::Extended(extended) => {
                let codes = [0, 1, 2].map(|component| sample(component).unwrap_or(0));
                for (slot, component) in values.iter_mut().zip(extended.components(codes)) {
                    *slot = component;
                }
            }
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
        // §11.6.5.2's inversion, before the space is consulted and after §8.9.5.2's map, which
        // is where the clause puts it. §7.4.9's own premultiplication above is a different
        // entry with a different weight — `/SMaskInData`'s opacity channel, not `/SMask`'s
        // samples — and the two are undone in the order the raster carries them.
        if let Some(matte) = matte {
            matte.restore(&mut values, pixel, &|component| match reading {
                JpxReading::Mapped(_) => space.component_range(component),
                JpxReading::Extended(extended) => extended.component_range(component),
            });
        }
        let colour = match reading {
            JpxReading::Mapped(_) => into.paint(space, &values),
            JpxReading::Extended(extended) => {
                let three =
                    [0, 1, 2].map(|component| values.get(component).copied().unwrap_or(0.0));
                into.paint(space, &extended.lab_d50(three))
            }
        };
        out.extend_from_slice(&[
            channel(colour.r),
            channel(colour.g),
            channel(colour.b),
            alpha,
        ]);
    }
    out
}

/// How a `JPXDecode` image's decoded integers become the values its colour space is painted with.
enum JpxReading {
    /// §8.9.5.2's map into a space of §8.6's: the dictionary's, or the codestream's where the
    /// codec's answer or this crate's own describes it as one.
    Mapped(Decode),
    /// PIMA 7667's e-sRGB or e-sYCC, which no §8.6 space describes: the encoding's own equations
    /// take the integers to its components, and [`ExtendedSrgb::lab_d50`] takes those to the
    /// connection space's L\*a\*b\*, which [`ExtendedSrgb::connection_space`] paints (ADR 2292).
    Extended(ExtendedSrgb),
}

/// A sample of `highest`-bounded domain as the byte an RGBA8 alpha channel takes.
///
/// An opacity has no `/Decode` entry to carry it into the raster's eight bits the way
/// §8.9.5.2's map carries a colour component, so the one linear map the domain admits is
/// applied here: 0 stays 0 and `highest` becomes 255.
fn scaled_to_byte(raw: u32, highest: u32) -> u8 {
    if highest == 0 {
        return 255;
    }
    #[expect(
        clippy::cast_precision_loss,
        reason = "both are at most 65535, which every f32 represents exactly"
    )]
    let fraction = raw as f32 / highest as f32;
    channel(fraction)
}

/// The JPEG 2000 data the confined decoder is handed, and the space its samples are read in
/// where that is this crate's to state rather than the codec's.
struct JpxColourChoice<'a> {
    /// The data, with every colour specification box the choice sets aside made a Free box.
    data: std::borrow::Cow<'a, [u8]>,
    /// The space, where the kept specification is one the codec does not describe in a way this
    /// crate draws: T.801's enumerated CIE Lab, which the codec hands back as ICC-encoded
    /// L\*a\*b\* under an abstract profile ISO 32000-2 §8.6.5.5 admits for no colour space.
    space: Option<crate::colour::ColourSpace>,
    /// PIMA 7667's encoding, where the kept specification is e-sRGB or e-sYCC: every `colr` box
    /// is then a Free box, so the codec hands back the integers untouched, and this crate reads
    /// them by the encoding's own equations ([`JpxReading::Extended`]).
    extended: Option<ExtendedEncoding>,
}

/// The enumerated colour spaces the codec draws as their definitions state, besides CIE Lab.
///
/// Part 1 Table I-10's sRGB (16) and greyscale (17), and T.801 Table M.25's CMYK (12), sYCC (18)
/// and ROMM-RGB (21). The last two are the codec's conversions — sYCC's matrix and a ROMM-RGB
/// profile — whose defining texts (IEC 61966-2-1 Amd. 1, PIMA 7666) this project does not hold;
/// `tests/jpx_enumerated_spaces.rs` pins what the held texts do fix, that a colour of no chroma
/// or of equal components is a neutral one. Two more T.801 Table M.25 lists are drawn by this
/// crate instead, from the held PIMA 7667 (A348): e-sRGB ([`E_SRGB`]) and e-sYCC ([`E_SYCC`]),
/// which [`jpx_drawn`] reads. CIE Jab (19) depends on the unheld CIE Publication 131, whose
/// purchase the owner declines, and takes §7.4.9's fallback (ADRs 1383, 2187, 2292).
const JPX_ENUMERATED_DRAWN: [u32; 5] = [12, 16, 17, 18, 21];

/// T.801 Table M.25's 20, e-sRGB, which it hands to PIMA 7667.
const E_SRGB: u32 = 20;

/// T.801 Table M.25's 24, e-sYCC, PIMA 7667's Annex B.
const E_SYCC: u32 = 24;

/// What an `EnumCS` of PIMA 7667's is called, for a sentence about it.
fn extended_name(encoding: ExtendedEncoding) -> &'static str {
    match encoding {
        ExtendedEncoding::Rgb => "enumerated colour space 20 (e-sRGB, PIMA 7667)",
        ExtendedEncoding::Ycc => "enumerated colour space 24 (e-sYCC, PIMA 7667 Annex B)",
    }
}

/// The encoding a raster's integers are read by, where the choice kept one of PIMA 7667's and
/// the codec delivered three colour components at a depth the encoding is defined at.
///
/// The depth is the raster's because the integers are at it; [`jpx_drawn`] read the same field
/// from the headers to rank the specification, and a codec answering otherwise leaves §7.4.9's
/// fallback rather than an equation evaluated at a depth it was not written for.
fn extended_on(
    encoding: Option<ExtendedEncoding>,
    raster: &pdf_sandbox::Raster,
) -> Option<ExtendedSrgb> {
    encoding
        .filter(|_| raster.components == 3)
        .and_then(|encoding| ExtendedSrgb::new(encoding, raster.precision))
}

/// The one depth a codestream's first three components share, unsigned, or `None`.
///
/// PIMA 7667's equations take three components of one depth — e-sRGB's `n` and e-sYCC's `m` are
/// each one number for the three channels — and a signed component is no integer the equations
/// read.
fn shared_colour_depth(headers: &crate::jpeg2000::Headers<'_>) -> Option<u8> {
    let depths = headers.component_depths();
    let colour = depths.get(..3)?;
    let first = colour.first()?;
    colour
        .iter()
        .all(|depth| depth.bits == first.bits && !depth.signed && !first.signed)
        .then_some(first.bits)
}

/// Which of a `JPXDecode` image's colour specifications its samples are read through.
///
/// ISO 32000-2 §7.4.9 decides it in three sentences, and the codec knows none of them:
///
/// > If present, it shall determine how the image samples are interpreted, and the colour space
/// > specifications in the JPEG 2000 data shall be ignored.
///
/// > If multiple colour space specifications are given in the JPEG 2000 data, a PDF processor
/// > should attempt to use the one with the highest precedence and best approximation value. If
/// > the colour space is given by an unsupported ICC profile, the next lower colour space, in
/// > terms of precedence and approximation value, shall be used. If no supported colour space is
/// > found, the colour space used shall be DeviceGray , DeviceRGB , or DeviceCMYK , depending on
/// > the whether the number of ordinary channels in the JPEG 2000 data is 1, 3, or 4.
///
/// The codec reads the first `colr` box, as part 1's I.5.3.3 tells a JP2 reader to, converts
/// some enumerated spaces before its samples leave it, and refuses the image outright over a
/// space it does not know. So the choice is made here, from [`crate::jpeg2000::Headers`], and
/// handed over as the data itself: every specification set aside becomes a Free box (T.801
/// M.11.20) and the kept one is the first the codec meets. A dictionary stating a space sets
/// all of them aside; otherwise they are ranked by precedence, then by approximation (1 best,
/// 4 worst, and part 1's 0 — no statement — after those), then in file order, and the first this
/// crate draws is kept. Where none is, none is kept, and the codec's own fallback is the one the
/// third sentence names. ADR 1383.
fn jpx_colour_choice(data: &[u8], declared: bool) -> JpxColourChoice<'_> {
    let borrowed = || JpxColourChoice {
        data: std::borrow::Cow::Borrowed(data),
        space: None,
        extended: None,
    };
    let Ok(headers) = crate::jpeg2000::Headers::parse(data) else {
        return borrowed();
    };
    if headers.colour.is_empty() {
        return borrowed();
    }
    if declared {
        return JpxColourChoice {
            data: std::borrow::Cow::Owned(headers.keeping_colour(data, None, None)),
            space: None,
            extended: None,
        };
    }
    let mut ranked: Vec<(usize, &crate::jpeg2000::ColourSpecification<'_>)> =
        headers.colour.iter().enumerate().collect();
    ranked.sort_by_key(|(index, colour)| {
        let approximation = if colour.approximation == 0 {
            u8::MAX
        } else {
            colour.approximation
        };
        (std::cmp::Reverse(colour.precedence), approximation, *index)
    });
    let kept = ranked.into_iter().find_map(|(index, colour)| {
        jpx_drawn(colour, &headers).map(|drawn| (index, colour, drawn))
    });
    let Some((index, colour, drawn)) = kept else {
        return JpxColourChoice {
            data: std::borrow::Cow::Owned(headers.keeping_colour(data, None, None)),
            space: None,
            extended: None,
        };
    };
    if let JpxDrawn::Extended(encoding) = drawn {
        return JpxColourChoice {
            data: std::borrow::Cow::Owned(headers.keeping_colour(data, None, None)),
            space: None,
            extended: Some(encoding),
        };
    }
    // T.801 M.11.7.3.2 lays the Any ICC method's field out as part 1's restricted one, which
    // is the only one the codec reads a profile from.
    let method = (colour.method == crate::jpeg2000::ColourSpecification::ANY_ICC)
        .then_some(crate::jpeg2000::ColourSpecification::RESTRICTED_ICC);
    let data = if index == 0 && method.is_none() {
        std::borrow::Cow::Borrowed(data)
    } else {
        std::borrow::Cow::Owned(headers.keeping_colour(data, Some(index), method))
    };
    let space = match drawn {
        JpxDrawn::ByTheCodec | JpxDrawn::Extended(_) => None,
        JpxDrawn::As(space) => Some(space),
    };
    JpxColourChoice {
        data,
        space,
        extended: None,
    }
}

/// How this crate draws a colour specification it draws at all.
enum JpxDrawn {
    /// Through the codec's own answer: its gray, RGB or CMYK, or the profile it hands back.
    ByTheCodec,
    /// In a space of this crate's, which the codec's answer does not describe.
    As(crate::colour::ColourSpace),
    /// By PIMA 7667's equations, from integers the codec is asked not to interpret.
    Extended(ExtendedEncoding),
}

/// Whether this crate draws one colour specification, and in which space where that is its own.
///
/// `None` for a specification it does not draw. Enumerated CIE Lab is drawn as §8.6.5.4's `Lab`:
/// the codec applies T.801's Equation M-18 with `EP`'s ranges and offsets, or M.11.7.4.1's
/// defaults, and returns lightness over 0 to 100 and a\* and b\* offset by 128, which is `Lab`'s
/// default `Decode` over a `Range` of `[-128 127 -128 127]`. The codec reads no `IL`, and the
/// space carries it instead: the white point of the illuminant the field names, which the `Lab`
/// adapts onto D50 as every CIE-based space is adapted. A code neither T.801 nor T.4 defines names
/// no white point and is not drawn, which leaves §7.4.9's fallback (ADR 1713).
///
/// e-sRGB and e-sYCC are drawn where the codestream's first three components share a depth
/// PIMA 7667 defines the encoding at — section 4.1's 10, 12 and 16 bits for e-sRGB, any depth for
/// Annex B's e-sYCC — and the next specification is tried where they do not (ADR 2292).
fn jpx_drawn(
    colour: &crate::jpeg2000::ColourSpecification<'_>,
    headers: &crate::jpeg2000::Headers<'_>,
) -> Option<JpxDrawn> {
    match (colour.method, colour.enumerated, colour.profile) {
        (_, Some(code @ (E_SRGB | E_SYCC)), _) => {
            let encoding = if code == E_SRGB {
                ExtendedEncoding::Rgb
            } else {
                ExtendedEncoding::Ycc
            };
            shared_colour_depth(headers)
                .and_then(|bits| ExtendedSrgb::new(encoding, bits))
                .map(|_| JpxDrawn::Extended(encoding))
        }
        (_, Some(crate::jpeg2000::ColourSpecification::CIELAB), _) => colour
            .cielab_illuminant()
            .and_then(crate::jpeg2000::Illuminant::white_point)
            .map(|white| {
                JpxDrawn::As(crate::colour::ColourSpace::Lab {
                    white,
                    black: [0.0; 3],
                    range: [-128.0, 127.0, -128.0, 127.0],
                })
            }),
        (_, Some(space), _) => JPX_ENUMERATED_DRAWN
            .contains(&space)
            .then_some(JpxDrawn::ByTheCodec),
        (_, None, Some(profile)) => {
            crate::icc::Profile::parse(profile).map(|_| JpxDrawn::ByTheCodec)
        }
        _ => None,
    }
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
                    profile: Arc::new(profile),
                })
            })
        }
        pdf_sandbox::Colour::Unknown => by_channel_count(),
    }
}

/// The bits a JPEG 2000 image's samples reach this crate at, or a sentence saying why no
/// single domain exists.
///
/// ISO 32000-2 Table 87 makes the depth this processor's to determine — "\[t]he bit depth is
/// determined by the PDF processor in the process of decoding the JPEG 2000 image" — and what
/// it determines is the codestream's own, so that §8.9.6.4's ranges are compared in the domain
/// their integers were written in (ADR 1242). `pdf_sandbox::decode` reads the same `Ssiz`
/// precision off the same codestream and hands its samples back at it, which is what makes this
/// answer the raster's as well as the file's.
///
/// Reads the codestream's headers and no samples at all, which is what [`crate::jpeg2000`]
/// exists for — the confined decoder is not started to answer a question about the mask.
///
/// Two cases have no single domain and each is refused by name rather than approximated: a
/// component whose samples are signed, which §8.9.6.4's non-negative integers cannot range
/// over, and components that disagree on a depth, which §8.9.5.2's "can have different values
/// per colour component" allows and which leaves no one number for the ranges to be bounded by.
/// More than sixteen bits is neither this crate's nor the clause's limit but
/// `pdf_sandbox::Raster`'s, and it is named as that.
pub(super) fn jpx_sample_domain(
    document: &Document,
    stream: &Stream,
    space: &ColourSpace,
) -> Result<u32, String> {
    // §7.4.9 gives the dictionary's space precedence over the codestream's own, which is why
    // `decode_jpx` asks the worker for unscaled indices here; the domain is then the table's,
    // which §8.6.6.3 caps at 255 whatever precision the codestream declares.
    if matches!(
        space,
        ColourSpace::Resolved(crate::colour::ColourSpace::Indexed { .. })
    ) {
        return Ok(JPX_INDEX_BITS);
    }
    let Some(source) = document.image_stream(stream) else {
        return Err(
            "colour-key /Mask on a JPXDecode image whose data this could not read".to_owned(),
        );
    };
    let depths = crate::jpeg2000::Headers::parse(&source.data)
        .map(|headers| headers.component_depths())
        .unwrap_or_default();
    let Some(first) = depths.first() else {
        return Err(
            "colour-key /Mask on a JPXDecode image whose data states no bit depth, \
                    which §8.9.6.4's ranges are bounded by"
                .to_owned(),
        );
    };
    if !first.signed
        && (1..=JPX_MAX_SAMPLE_BITS).contains(&u32::from(first.bits))
        && depths
            .iter()
            .all(|depth| depth.bits == first.bits && !depth.signed)
    {
        return Ok(u32::from(first.bits));
    }
    let found = depths
        .iter()
        .map(|depth| {
            let sign = if depth.signed { "signed" } else { "unsigned" };
            format!("{} {sign}", depth.bits)
        })
        .collect::<Vec<_>>()
        .join(", ");
    Err(format!(
        "colour-key /Mask on a JPXDecode image whose components are {found} bits, which is \
         either signed, wider than the {JPX_MAX_SAMPLE_BITS} bits a sample this decodes to, or \
         more than one domain for §8.9.6.4's ranges to be bounded by"
    ))
}

/// The domain a JPEG 2000 image whose dictionary states an `Indexed` space ranges over.
///
/// §7.4.9 gives that dictionary precedence over the codestream's own space, so the decoder
/// hands the indices back unscaled and §8.6.6.3 caps `hival` at 255 whatever precision the
/// codestream declares.
const JPX_INDEX_BITS: u32 = 8;

/// The widest sample `pdf_sandbox::Raster` carries, which is this tree's limit and not a clause's.
const JPX_MAX_SAMPLE_BITS: u32 = 16;

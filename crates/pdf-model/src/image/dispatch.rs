//! From an image dictionary to straight-alpha samples: which route a stream takes, and what
//! is decoded beside it.
//!
//! The route is decided by the codec left on the end of the filter chain (ISO 32000-2 §7.4):
//! no codec is [`super::unpack`]'s, `DCTDecode` is [`super::dct`]'s, and `JBIG2Decode`,
//! `CCITTFaxDecode` and `JPXDecode` are handed to the sandbox by [`super::jbig2`],
//! [`super::ccitt`] and [`super::jpx`]. A soft mask's `/Matte` (§11.6.5.2) is read here before
//! the samples are, because the colour it undoes is the samples' own.

use std::sync::Arc;

use pdf_render::{Image, SampleAlpha};
use pdf_syntax::{Dictionary, Document, ImageStream, Object, Stream};
use rayon::iter::IndexedParallelIterator as _;
use rayon::iter::ParallelIterator as _;
use rayon::slice::ParallelSlice as _;
use rayon::slice::ParallelSliceMut as _;

use crate::colour::Conversion;

use super::cache::MaskCache;
use super::ccitt::{ccitt_bilevel, ccitt_shortfall, decode_ccitt};
use super::convert::PARALLEL_PIXELS;
use super::dct::{DecodedJpeg, dct_frame, decode_dct, decode_jpeg, samples_of_frame};
use super::device_scale::SoftMaskAtDeviceScale;
use super::jbig2::{decode_jbig2, jbig2_bilevel};
use super::jpx::decode_jpx;
use super::mask::{MaskEntry, apply_explicit_mask, contradicted_frame, mask_entry};
use super::soft_mask::{
    MaskPlane, SoftMaskEntry, Softened, apply_soft_mask, eager_soft_mask, prematte_alpha,
    soft_mask_entry, stated_grid,
};
use super::space::{ColourSpace, Decode, Dictionaries, colour_space};
use super::unpack::{
    MatteTable, Painting, Prematte, Samples, matted_eight_bit_device_tables, unpack,
    unpack_matted_eight_bit_device,
};
use super::{ImageError, InFlight, MAX_SAMPLES, Parts, Picture, sample_alpha};

/// [`decode_parts`], told whether anything else of the page is being decoded beside it.
///
/// [`decode_parts`]: super::decode_parts
pub(super) fn decode_parts_in(
    document: &Document,
    stream: &Stream,
    (resources, fill, into): (&Dictionary, pdf_render::Color, &Conversion),
    masks: &mut MaskCache,
    in_flight: InFlight,
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
    let mask = mask_entry(document, stream, resources);
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

    // §11.6.5.2's pre-blending is undone here rather than after the raster exists, because
    // the clause says where: "inversion of the pre-blending shall precede the colour
    // conversion". So the matte and the mask's samples travel into the route that turns
    // samples into colour, and the inversion happens in the parent image's own components.
    // Table 143 gives a `/Matte`'d mask the parent's own grid — `matte_colour` refuses one
    // that states another — so the two pair by position. ADR 1268.
    //
    // **The mask the eager route multiplies in is decoded once, and where it is large, beside the
    // image's own samples** (ADR 1469). [`eager_soft_mask`] reads the dictionary's grid; where that
    // is the grid read here, it is the same entry, the same stream and the same `decode` call that
    // [`prematte_alpha`] makes, so a `/Matte`'s opacity is read off it rather than off a second
    // decode — the clause's ordering puts that decode before the samples, so it cannot run beside
    // them — and an image with no `/Matte` starts it on the pool before its own samples. What is
    // made here answers [`apply_soft_mask`] only, and only if the routing below reaches it; a route
    // that does not has decoded a mask for nothing, which is all a wrong guess costs. Where the two
    // grids differ the mask is decoded for the inversion and again for the opacity, as before.
    let entry = soft_mask_entry(document, dict, resources, (width, height));
    let one_grid = stated_grid(document, dict) == (width, height);
    let (frame, premultiplied, eager_mask) = matte_before_samples(
        at,
        (&source, is_mask, in_flight),
        &entry,
        (width, height),
        one_grid,
    );
    let prematte = premultiplied.as_ref().map(Prematte::of);

    let painting = Painting {
        is_mask,
        fill,
        colour_key,
        into,
        matte: prematte.as_ref(),
    };
    let (decoded, eager_mask) = samples_beside_the_mask(
        at,
        (&source, (width, height), in_flight, frame),
        painting,
        (&entry, one_grid, eager_mask),
    );
    let SamplesOnGrid {
        rgba,
        grid: (raster_width, raster_height),
        opacity_included: opacity_came_with_the_samples,
        stencil_opacity,
        shortfall,
        mask_in_alpha,
    } = decoded?;
    // §7.4.8's disagreement is read off the grid the codec has just built on, rather than off a
    // second walk of the frame header: the arm that decoded the frame is the one that knows it.
    let contradiction = contradicted_frame(&source, (raster_width, raster_height), (width, height));

    let image = Image {
        // The grid the samples are actually on, which is the dictionary's unless a
        // self-describing codestream stated another. A JPEG 2000 codestream over the worker's
        // budget comes back at a reduced resolution level (§7.4.9 NOTE 3), and a JPEG frame
        // carries its own dimensions (§7.4.8) that a malformed dictionary can contradict; an
        // image occupies the unit square whatever its resolution (§8.9.5.1), so the raster's
        // own grid is the only honest one to build at — declaring the dictionary's would
        // shear every row.
        width: raster_width,
        height: raster_height,
        data: Arc::from(rgba.as_slice()),
        // §8.9.5.3, and Table 87's default of false. The entry is a hint about what to do
        // when the image is magnified, so it travels with the samples and the backends
        // decide what to make of it.
        interpolate: matches!(document.get_key(dict, "Interpolate"), Object::Boolean(true)),
        // What the alpha channel is made of before any soft mask is §11.6.4.2's shape on
        // every route: a stencil's painted areas; the rectangle, less what a colour key
        // range — applied in the unpacker — removed, which is the clause's "further modified
        // by an explicit or colour key mask"; and a filter that stopped short of the grid
        // left rows unpainted, which is shape 0 as well. Only a soft mask changes it, and
        // each of the three places below that applies one says so.
        sample_alpha: SampleAlpha::Shape,
    };
    // A mask's own shortfall joins the base's, named for the entry it came through; the base's
    // takes precedence because a report names one thing and the picture is the thing.
    let (picture, mask_shortfall) = soft_masked(
        at,
        image,
        is_mask,
        (opacity_came_with_the_samples, stencil_opacity),
        masks,
        (eager_mask, mask_in_alpha),
    );
    let mut shortfall =
        shortfall.or_else(|| mask_shortfall.map(|detail| format!("its /SMask: {detail}")));
    let image = match picture {
        Picture::Complete(image) => image,
        Picture::Masked { .. } => {
            return Ok(Parts {
                picture,
                shortfall,
                contradiction,
            });
        }
    };

    // §11.6.4.3 makes the two mutually exclusive — an `/SMask` "shall override any explicit
    // or colour key mask" — so `mask_entry` has already returned [`MaskEntry::Overridden`]
    // for anything reached here after a soft mask was applied, and this arm runs only where
    // there was none. The sequence is therefore an ordering of two things that never both
    // happen, kept in the order the clauses rank them.
    let picture = match &mask {
        MaskEntry::Explicit(stencil) => {
            let (image, mask_shortfall) =
                apply_explicit_mask(document, &image, resources, stencil)?;
            shortfall =
                shortfall.or_else(|| mask_shortfall.map(|detail| format!("its /Mask: {detail}")));
            Picture::Complete(image)
        }
        _ => Picture::Complete(image),
    };
    Ok(Parts {
        picture,
        shortfall,
        contradiction,
    })
}

/// Table 144's `/Matte` in the parent's components, the mask's samples on the parent's grid, and
/// whether those samples are the eager route's own plane: what [`Prematte`] borrows.
pub(super) type Premultiplied = (Vec<f32>, Arc<[u8]>, bool);

/// A `DCTDecode` frame [`matte_before_samples`] decoded beside its `/Matte`'s mask, and its
/// codec's answer — or `None` where it did not, and the samples are decoded as they always were.
type FrameBeside = Option<Result<DecodedJpeg, ImageError>>;

/// [`eager_soft_mask`]'s answer, where [`decode_parts`] asked it before the routing reached
/// [`apply_soft_mask`] (ADR 1469).
///
/// [`decode_parts`]: super::decode_parts
pub(super) enum EagerMask {
    /// Not asked: [`apply_soft_mask`] asks it, if the routing reaches it.
    Unasked,
    /// Asked, and this is what it answered — `None` being a mask that would not decode.
    Asked(Option<MaskPlane>),
    /// Asked, and its opacity is already the samples' alpha: the `/Matte`'s inversion read this
    /// plane on the raster's own grid and wrote each `α` beside the colour it restored, which is
    /// the byte [`opacity_multiplied_in_place`] would have written (ADR 1519).
    ///
    /// [`opacity_multiplied_in_place`]: super::combine::opacity_multiplied_in_place
    InTheSamples(MaskPlane),
}

/// §11.6.5.2's `/Matte` and the opacity its inversion reads, where the parent states one, and
/// the eager route's mask where the decode that read the opacity was that mask's — with the
/// parent's `DCTDecode` frame where that was decoded beside the mask.
///
/// `entry` is the parent's soft-mask entry on `grid`, the grid [`decode_parts`] read, and
/// `one_grid` whether that is the dictionary's grid [`eager_soft_mask`] reads: only then is the
/// entry, the stream and the `decode` call the eager route's own, and only then is its answer
/// handed on rather than asked again.
///
/// **A `DCTDecode` frame is decoded beside its `/Matte`'s mask.** The clause orders the inversion
/// before the colour conversion — "inversion of the pre-blending shall precede the colour
/// conversion" — and the inversion reads the mask; the codec reads neither, and decoding the frame
/// is most of what such an image costs. So where the parent is a `DCTDecode` image of at least
/// [`PARALLEL_PIXELS`] samples on the eager route's grid, the frame ([`dct_frame`]) is decoded on
/// this thread and the mask on the pool, and the inversion runs on the pair after both, where it
/// ran before. On `issue13931.pdf` the mask (its inflation and its plane) was 6 ms of the turn
/// spent before the frame's 9.5 ms began (ADR 1481). The frame comes back as the first part, and
/// `None` there means the caller decodes the samples as it always did.
///
/// [`decode_parts`]: super::decode_parts
fn matte_before_samples(
    at: Dictionaries,
    (source, is_mask, in_flight): (&ImageStream, bool, InFlight),
    entry: &SoftMaskEntry,
    grid: (u32, u32),
    one_grid: bool,
) -> (FrameBeside, Option<Premultiplied>, EagerMask) {
    let SoftMaskEntry::Image {
        stream,
        matte: Some(matte),
        ..
    } = entry
    else {
        return (None, None, EagerMask::Unasked);
    };
    if !one_grid {
        let alpha = prematte_alpha(at.document, stream, grid);
        return (
            None,
            alpha.map(|alpha| (matte.clone(), alpha, false)),
            EagerMask::Unasked,
        );
    }
    let beside_the_frame = !is_mask
        && matches!(source.codec.as_deref(), Some(b"DCTDecode" | b"DCT"))
        && u64::from(grid.0).saturating_mul(u64::from(grid.1)) >= PARALLEL_PIXELS as u64;
    let (frame, mask) = if beside_the_frame {
        let (frame, mask) = beside(
            || dct_frame(at, source, in_flight),
            || eager_soft_mask(at.document, at.dict, at.resources),
        );
        (Some(frame), mask.flatten())
    } else {
        (None, eager_soft_mask(at.document, at.dict, at.resources))
    };
    // The inversion may write the opacity beside the colour where its `α` is the very plane the
    // eager route would multiply in — the mask's own, on the raster's grid — which is
    // [`Prematte::opacity`]'s condition (ADR 1519).
    let premultiplied = mask.as_ref().map(|mask| {
        let alpha = mask.on_grid(grid);
        let opacity = Arc::ptr_eq(&mask.opacity, &alpha);
        (matte.clone(), alpha, opacity)
    });
    (frame, premultiplied, EagerMask::Asked(mask))
}

/// [`samples_of`]'s answer, with the eager route's mask decoded beside it on the pool where that
/// pays — or, where [`matte_before_samples`] already decoded the `DCTDecode` frame beside its
/// mask, that frame made the samples.
///
/// Where it pays is four conditions: [`matte_before_samples`] has not asked the mask already; the
/// dictionary's grid is the one read, so the entry is the eager route's (`one_grid`); the image is
/// at least [`PARALLEL_PIXELS`] samples, where a pool task costs nothing beside the decode; and
/// the codec is not `JPXDecode`, whose `/SMaskInData` would carry the opacity itself (ADR 1469).
/// Measured alone, pinned: `22060_A1_01_Plans.pdf`'s turn 94.4 → 80.0 ms, `issue13931.pdf`'s
/// 66.8 → 55.8; it costs no instruction, only moves the mask's decode off the image's thread.
fn samples_beside_the_mask(
    at: Dictionaries,
    (source, grid, in_flight, frame): (&ImageStream, (u32, u32), InFlight, FrameBeside),
    painting: Painting,
    (entry, one_grid, eager_mask): (&SoftMaskEntry, bool, EagerMask),
) -> (Result<SamplesOnGrid, ImageError>, EagerMask) {
    if let Some(frame) = frame {
        let samples = frame.and_then(|frame| samples_of_frame(at, frame, grid, painting));
        return (samples, eager_mask);
    }
    let worth_it = matches!(eager_mask, EagerMask::Unasked)
        && one_grid
        && u64::from(grid.0).saturating_mul(u64::from(grid.1)) >= PARALLEL_PIXELS as u64
        && !matches!(source.codec.as_deref(), Some(b"JPXDecode"))
        && matches!(entry, SoftMaskEntry::Image { .. });
    if !worth_it {
        return (
            samples_of(at, source, grid, painting, in_flight),
            eager_mask,
        );
    }
    let (decoded, mask) = beside(
        || samples_of(at, source, grid, painting, in_flight),
        || eager_soft_mask(at.document, at.dict, at.resources),
    );
    (decoded, mask.map_or(EagerMask::Unasked, EagerMask::Asked))
}

/// §11.6.5.2's soft mask on a decoded raster: multiplied in, or carried beside it for the device.
///
/// `in_data` is what [`samples_of`] said about `/SMaskInData` — whether the opacity came with the
/// samples, and for a stencil the plane it was read into — and `stencil` whether the raster is
/// §8.9.6.2's. The shortfall is the mask's own filter's, where the eager route decoded it.
/// `eager_mask` is [`eager_soft_mask`]'s answer where [`decode_parts`] already has it, and
/// `mask_in_alpha` [`SamplesOnGrid::mask_in_alpha`]: that mask's opacity is the samples' alpha
/// already, written by the `/Matte`'s inversion (ADR 1519).
///
/// [`decode_parts`]: super::decode_parts
fn soft_masked(
    at: Dictionaries,
    image: Image,
    stencil: bool,
    in_data: (bool, Option<SoftMaskAtDeviceScale>),
    masks: &mut MaskCache,
    (eager_mask, mask_in_alpha): (EagerMask, bool),
) -> (Picture, Option<String>) {
    let eager_mask = match eager_mask {
        EagerMask::Asked(Some(mask)) if mask_in_alpha => EagerMask::InTheSamples(mask),
        other => other,
    };
    let Dictionaries {
        document,
        dict,
        resources,
    } = at;
    let mut image = image;
    let (opacity_came_with_the_samples, stencil_opacity) = in_data;
    if opacity_came_with_the_samples {
        // §7.4.9 and §11.6.5.2: a non-zero `/SMaskInData` means the opacity travelled with
        // the image samples, `/SMask` "shall not be present", and the embedded mask
        // overrides any that is. Applying one on top would multiply two alphas together.
        image.sample_alpha = sample_alpha(stencil, true);
        // A stencil's embedded opacity was read as a plane of its own (`jpx_stencil`), so the
        // pair goes to the device as an `/SMask`'s does and the shape stays askable (ADR 1279).
        let picture = match stencil_opacity {
            Some(opacity) => Picture::Masked {
                base: image,
                opacity,
            },
            None => Picture::Complete(image),
        };
        return (picture, None);
    }
    // A mask whose grid the finer of the two cannot hold leaves this function in two
    // parts, for the device to put together. Everything below is about one raster.
    // The routing is asked about the raster that exists — the decoded grid, not the
    // dictionary's statement — which is what sends a reduced JPEG 2000 base under a
    // full-size mask here rather than into an eager combination on a grid neither raster is
    // on.
    if let Some(opacity) = masks.read(document, dict, resources, (image.width, image.height)) {
        image.sample_alpha = sample_alpha(stencil, true);
        return (
            Picture::Masked {
                base: image,
                opacity,
            },
            None,
        );
    }
    // Applied last so a soft mask cannot resurrect an inconsistent buffer.
    let Softened {
        mut image,
        shortfall,
        applied,
        apart,
    } = apply_soft_mask(document, dict, resources, image, (stencil, eager_mask));
    image.sample_alpha = sample_alpha(stencil, applied);
    let picture = match apart {
        Some(opacity) => Picture::Masked {
            base: image,
            opacity,
        },
        None => Picture::Complete(image),
    };
    (picture, shortfall)
}

/// One decode route's answer: samples, the grid they are on, and whether opacity came along.
pub(super) struct SamplesOnGrid {
    /// Straight-alpha RGBA8, row by row on [`Self::grid`].
    pub(super) rgba: Vec<u8>,
    /// The grid the samples are actually on, which is the dictionary's except where a
    /// self-describing codestream states another; see [`samples_of`].
    pub(super) grid: (u32, u32),
    /// §11.6.5.2's `/SMaskInData`: the opacity arrived inside the codestream, so no `/SMask`
    /// may be applied on top of it. It is in the alpha channel, except for a stencil's, which
    /// is [`Self::stencil_opacity`].
    pub(super) opacity_included: bool,
    /// A §8.9.6.2 stencil's `/SMaskInData` opacity, read as a plane of its own rather than
    /// multiplied into [`Self::rgba`]'s alpha, which stays the stencil's shape.
    ///
    /// §11.6.4.2 makes a stencil's painted areas its shape and §11.6.4.3 makes an opacity that
    /// arrived with the samples the other quantity, so the pair is carried to the device the
    /// way an `/SMask`'s is (ADR 1218, ADR 1279). `None` on every route but [`jpx_stencil`]'s.
    ///
    /// [`jpx_stencil`]: super::jpx::jpx_stencil
    pub(super) stencil_opacity: Option<SoftMaskAtDeviceScale>,
    /// The filter's sentence where it stopped short of the grid on damaged data; see
    /// [`Parts::shortfall`]. Only the `CCITTFaxDecode` arm produces one.
    pub(super) shortfall: Option<String>,
    /// §11.6.5.2's soft mask is already multiplied in: the `/Matte`'s inversion wrote its plane
    /// as the alpha ([`Prematte::opacity`]), so [`apply_soft_mask`] has nothing left to multiply.
    /// True only on [`samples_of_frame`]'s route, where [`invert_matte_in_place`] wrote it.
    pub(super) mask_in_alpha: bool,
}

/// The image's samples as straight-alpha RGBA8, the grid they are on, and whether the
/// opacity came with them.
///
/// One arm per route from bytes to colour: §7.4.8's `DCTDecode`, §7.4.7's `JBIG2Decode`,
/// §7.4.9's `JPXDecode`, §7.4.6's `CCITTFaxDecode`, and [`unpack`] for the four filters that
/// leave samples behind rather than a codestream. Split out of [`decode_parts`] because the
/// route a stream takes is a question about its filter chain and nothing else, while
/// everything around it is about masks.
///
/// The grid is the dictionary's for every arm but two, and both are codestreams that state
/// their own: a `JPXDecode` codestream over the confined worker's budget is decoded at a
/// reduced resolution level (§7.4.9 NOTE 3), and a `DCTDecode` frame carries the dimensions
/// §7.4.8 puts in the encoded data, which a malformed file can contradict. Both answer with
/// the grid the codec produced rather than letting the caller assume the dictionary's.
///
/// The third part of the answer is §11.6.5.2's `/SMaskInData`, which only `JPXDecode` can
/// carry: an opacity that arrived inside the codestream is already in the alpha channel, and
/// applying `/SMask` on top of it would multiply two alphas together.
///
/// # Errors
///
/// See [`ImageError`].
///
/// [`decode_parts`]: super::decode_parts
fn samples_of(
    at: Dictionaries,
    source: &ImageStream,
    (width, height): (u32, u32),
    painting: Painting,
    in_flight: InFlight,
) -> Result<SamplesOnGrid, ImageError> {
    let Dictionaries {
        document,
        dict,
        resources,
    } = at;
    match source.codec.as_deref() {
        Some(b"DCTDecode" | b"DCT") => decode_dct(at, source, (width, height), painting, in_flight),
        Some(b"JBIG2Decode") => {
            let (rgba, shortfall) = decode_jbig2(at, source, (width, height), painting)?;
            Ok(SamplesOnGrid {
                rgba,
                grid: (width, height),
                opacity_included: false,
                stencil_opacity: None,
                shortfall,
                mask_in_alpha: false,
            })
        }
        Some(b"JPXDecode") => decode_jpx(at, source, (width, height), painting),
        Some(b"CCITTFaxDecode" | b"CCF") => {
            let (rgba, shortfall) = decode_ccitt(at, source, (width, height), painting)?;
            Ok(SamplesOnGrid {
                rgba,
                grid: (width, height),
                opacity_included: false,
                stencil_opacity: None,
                shortfall,
                mask_in_alpha: false,
            })
        }
        Some(other) => Err(ImageError::UnsupportedFilter {
            filter: String::from_utf8_lossy(other).into_owned(),
        }),
        None => {
            // §8.9.5.1 Table 87 makes `/BitsPerComponent` "Required except for image masks and
            // images that use the JPXDecode filter", and its *value* is validated in [`unpack`],
            // which refuses anything but `1 2 4 8 16` by name. What is repaired rather than
            // refused is its **absence** on an image that reaches this arm — raw samples, or ones
            // a non-codec filter already decoded. For `RunLengthDecode` the table fixes the
            // answer: that filter "shall always deliver 8-bit samples", so 8 is the only depth it
            // can mean. For `LZWDecode`/`FlateDecode` the depth is genuinely the entry's, and 8 is
            // then a repair of a malformed dictionary to its modal depth rather than a derivation
            // — a deliberate choice over refusing, because no document exercises it:
            // `examples/required_entry_census` counts 0 of the 2997 image dictionaries over 963
            // pdf.js documents missing a required `/BitsPerComponent`. A stated value of another
            // type is repaired the same way, for the same reason.
            let bits = if painting.is_mask {
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
            let space = if painting.is_mask {
                ColourSpace::Mask
            } else {
                colour_space(document, dict, resources, painting.into)?
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
                    colour_key: painting.colour_key,
                    fill: painting.fill,
                    into: painting.into,
                    matte: painting.matte,
                },
            )?;
            Ok(SamplesOnGrid {
                rgba,
                grid: (width, height),
                opacity_included: false,
                stencil_opacity: None,
                shortfall: None,
                mask_in_alpha: false,
            })
        }
    }
}

/// Undoes §11.6.5.2's pre-blending on a `DCTDecode` raster, through the route every other
/// filter's samples take.
///
/// The clause states one inversion, `c = m + (c′ - m) ÷ α` in the parent image's own components,
/// and puts it before the colour conversion — "inversion of the pre-blending shall precede the
/// colour conversion". This route holds the frame's components as eight-bit samples until
/// [`convert_channels`] reads them, which is exactly the input [`unpack`] takes; so the samples
/// are repacked into rows and handed to it, and [`Prematte::restore`] is the one place Table
/// 144's formula is computed for both routes. A matte undone from a `DCTDecode` frame and from
/// the same samples behind `FlateDecode` is therefore the same bytes by construction. ADR 1268.
///
/// The outer `Err` is the colour space's own refusal, in the words [`convert_channels`] gives it.
/// The inner `Err` is the sentence for [`SamplesOnGrid::shortfall`] where the inversion could not
/// be done: §7.4.8 puts a JPEG's dimensions in the data and Table 143 pairs a `/Matte`'d mask with
/// the dictionary's, so where the two disagree there is no pairing to invert by. Said rather
/// than guessed at. ADR 1268. The inner `Ok` says whether the soft mask's opacity was written as
/// the alpha in the same pass ([`Prematte::opacity`], ADR 1519).
///
/// [`convert_channels`]: super::convert::convert_channels
pub(super) fn matte_through_unpack(
    at: Dictionaries,
    painting: &Painting,
    frame: (&mut Vec<u8>, usize),
    grid: (u32, u32),
    stated: (u32, u32),
) -> Result<Result<bool, String>, ImageError> {
    if grid != stated {
        return Ok(Err(format!(
            "the /Matte could not be undone: the codestream is {}x{} where the dictionary says \
             {}x{}",
            grid.0, grid.1, stated.0, stated.1
        )));
    }
    let (rgba, components) = frame;
    let space = colour_space(at.document, at.dict, at.resources, painting.into)?;
    let wanted = space.components();
    // A greyscale frame is delivered as three equal channels, so a one-component space reads the
    // first; three or four components against a frame of the other number is the dictionary and
    // the codestream contradicting each other, which [`convert_channels`] refuses in the same words.
    if wanted > 1 && wanted != components {
        return Err(ImageError::UnsupportedColourSpace {
            space: format!("a {wanted}-component space on a JPEG of {components} components"),
        });
    }
    let decode = Decode::read(at.document, at.dict, &space, 8);
    let samples = Samples {
        bits: 8,
        space: &space,
        decode: &decode,
        colour_key: None,
        fill: painting.fill,
        into: painting.into,
        matte: painting.matte,
    };
    let (width, height) = (grid.0 as usize, grid.1 as usize);
    // Where the inversion is tabulated the frame's own four-byte pixels are read where they lie:
    // the components are their first bytes, which is the layout the repacked rows below would
    // hold, `stride` apart instead of `wanted`. And where the frame covers the grid each pixel is
    // written back over itself, since an output pixel reads its own input pixel and nothing else
    // — so no second raster is allocated and faulted in beside the frame (ADR 1481).
    if let Some(tables) = matted_eight_bit_device_tables(&samples, width, height) {
        let (alpha, opacity) = painting
            .matte
            .map_or((&[][..], false), |matte| (matte.alpha, matte.opacity));
        if invert_matte_in_place(rgba, (width, height), &tables, (alpha, opacity)) {
            return Ok(Ok(opacity));
        }
        *rgba = unpack_matted_eight_bit_device(
            rgba,
            (width, height),
            (width.saturating_mul(4), 4),
            &tables,
            alpha,
        );
        return Ok(Ok(false));
    }
    let raw: Vec<u8> = rgba
        .chunks_exact(4)
        .flat_map(|pixel| pixel.iter().take(wanted).copied())
        .collect();
    *rgba = unpack(&raw, grid.0, grid.1, &samples)?;
    Ok(Ok(false))
}

/// [`unpack_matted_eight_bit_device`] over a frame of four-byte pixels that covers its grid, each
/// pixel's answer written over the pixel it is read from; `false`, and nothing touched, where the
/// frame or the mask does not cover the grid, which that function then answers.
///
/// The same bytes by construction: a pixel is `[t₀[α, c₀], t₁[α, c₁], t₂[α, c₂], 255]` (or the
/// grey table's entry three times) in both, read from the same four bytes before they are written,
/// and with every sample and mask value present neither function meets the rows' short ends.
/// `the_matte_table_route_unpacks_the_per_sample_routes_pixels` holds the three to one answer.
///
/// `opacity` writes `α` where the alpha byte is 255 otherwise: [`Prematte::opacity`]'s case, the
/// soft mask's multiplication taken in the same pass, to the byte it would have written
/// (`a_matte_inverted_with_its_opacity_is_the_inversion_then_the_multiplication`). On
/// `issue13931.pdf` the multiplication was a second pass over 51 MB (ADR 1481 section 6, ADR 1519).
pub(super) fn invert_matte_in_place(
    rgba: &mut [u8],
    (width, height): (usize, usize),
    tables: &[MatteTable],
    (alpha, opacity): (&[u8], bool),
) -> bool {
    let pixels = width.saturating_mul(height);
    if width == 0 || rgba.len() != pixels.saturating_mul(4) || alpha.len() < pixels {
        return false;
    }
    let entry = |table: &MatteTable, alpha: u8, sample: u8| {
        table[usize::from(alpha) << 8 | usize::from(sample)]
    };
    // `α | cover` is the alpha byte: 255 where the multiplication comes after, and `α` itself
    // where it is taken here — one walk either way, the choice made once rather than per pixel.
    let cover = if opacity { 0 } else { u8::MAX };
    let invert = |(pixels, alphas): (&mut [[u8; 4]], &[u8])| match tables {
        [grey] => {
            for (pixel, &alpha) in pixels.iter_mut().zip(alphas) {
                let level = entry(grey, alpha, pixel[0]);
                *pixel = [level, level, level, alpha | cover];
            }
        }
        [red, green, blue] => {
            for (pixel, &alpha) in pixels.iter_mut().zip(alphas) {
                *pixel = [
                    entry(red, alpha, pixel[0]),
                    entry(green, alpha, pixel[1]),
                    entry(blue, alpha, pixel[2]),
                    alpha | cover,
                ];
            }
        }
        _ => {}
    };
    // Rows divide across the pool for [`unpack_matted_eight_bit_device`]'s reason: no pixel reads
    // another.
    let (frame, _) = rgba.as_chunks_mut::<4>();
    frame
        .par_chunks_mut(width)
        .zip(alpha.par_chunks(width))
        .for_each(invert);
    true
}

/// The samples a `DCTDecode`, `CCITTFaxDecode` or `JBIG2Decode` filter delivers, packed as
/// §8.9.5.2 lays out any image's samples: what a writer that must carry the image re-encodes
/// (ADR 1371).
///
/// Table 87 makes a codec's output the image's sample data like any other filter's: "a
/// CCITTFaxDecode or JBIG2Decode filter shall always deliver 1-bit samples, a RunLengthDecode or
/// DCTDecode filter shall always deliver 8-bit samples", and `/BitsPerComponent` "shall be
/// consistent with the size of the data samples that the filter delivers". So these are the
/// integers the dictionary's `/ColorSpace`, `/Decode`, colour key and a mask's `/Matte` describe,
/// before any of them is applied — the step [`decode`] takes before it makes colour of them.
///
/// [`decode`]: super::decode
#[derive(Debug, Clone)]
#[expect(
    clippy::doc_markdown,
    reason = "the comment quotes Table 87 verbatim, and a quotation is not marked up"
)]
pub struct FilterSamples {
    /// Columns the filter delivered: the codestream's own for `DCTDecode`, whose dimensions
    /// §7.4.8 puts in the encoded data, and the dictionary's for the two bilevel filters.
    pub width: u32,
    /// Rows the filter delivered, on the same terms.
    pub height: u32,
    /// Colour components per sample: the frame's for `DCTDecode`, one for the bilevel filters.
    pub components: usize,
    /// Bits per component: eight for `DCTDecode`, one for the bilevel filters.
    pub bits: u8,
    /// The samples, left to right and top to bottom, each row filled to a byte boundary.
    pub data: Vec<u8>,
    /// Where the filter stopped before the grid was whole, or concealed damaged rows as Table 11
    /// asks, the sentence saying so: rows that are the worker's padding or the row above, and
    /// not what the file coded.
    pub shortfall: Option<String>,
}

/// Runs a `DCTDecode`, `CCITTFaxDecode` or `JBIG2Decode` image's codec as a filter and returns
/// the samples it delivers, unconverted (ADR 1371).
///
/// The codecs are the ones [`decode`] runs, under the same isolation: `DCTDecode` in this process
/// with §7.4.8 Table 13's colour transform undone as the filter's own step, the two bilevel ones
/// through the confined worker. Nothing after the filter is applied — no `/Decode`, no colour
/// conversion, no mask — so each integer is the one the file's entries were written against.
///
/// # Errors
///
/// See [`ImageError`]; an image behind no codec, or behind `JPXDecode`, is
/// [`ImageError::UnsupportedFilter`], because a codec-free image's stream already holds its
/// samples and a JPEG 2000 image's are [`jpx_samples`]'.
///
/// [`decode`]: super::decode
/// [`jpx_samples`]: super::jpx::jpx_samples
pub fn filter_samples(document: &Document, stream: &Stream) -> Result<FilterSamples, ImageError> {
    let dict = &stream.dict;
    let width = positive_integer(document, dict, "Width")?;
    let height = positive_integer(document, dict, "Height")?;
    let samples = u64::from(width).saturating_mul(u64::from(height));
    if samples > MAX_SAMPLES {
        return Err(ImageError::TooLarge { samples });
    }
    let source = document
        .image_stream(stream)
        .ok_or_else(|| ImageError::Malformed {
            detail: "stream did not decode".to_owned(),
        })?;
    match source.codec.as_deref() {
        Some(b"DCTDecode" | b"DCT") => {
            let stated = source
                .parms
                .as_ref()
                .map(|parms| document.get_key(parms, "ColorTransform"))
                .and_then(|value| value.as_integer());
            let DecodedJpeg {
                samples,
                stated_components: components,
                grid,
                ..
            } = decode_jpeg(&source.data, stated, InFlight::AmongOthers)?;
            // [`DecodedJpeg::samples`] holds four bytes a pixel whatever the frame's count; the
            // frame's own components are the first `components` of them, a grey frame's one
            // component being delivered three times over.
            let data = samples
                .chunks_exact(4)
                .flat_map(|pixel| pixel.iter().take(components).copied())
                .collect();
            Ok(FilterSamples {
                width: grid.0,
                height: grid.1,
                components,
                bits: 8,
                data,
                shortfall: None,
            })
        }
        Some(b"CCITTFaxDecode" | b"CCF") => {
            let (bilevel, damaged_rows) = ccitt_bilevel(document, &source, (width, height))?;
            let shortfall = ccitt_shortfall(&bilevel, height, damaged_rows);
            Ok(FilterSamples {
                width,
                height,
                components: 1,
                bits: 1,
                data: bilevel.rows,
                shortfall,
            })
        }
        Some(b"JBIG2Decode") => {
            let bilevel = jbig2_bilevel(document, &source, (width, height))?;
            Ok(FilterSamples {
                width,
                height,
                components: 1,
                bits: 1,
                data: bilevel.rows,
                shortfall: bilevel.stopped_by,
            })
        }
        other => Err(ImageError::UnsupportedFilter {
            filter: other.map_or_else(
                || "an image behind no codec".to_owned(),
                |name| String::from_utf8_lossy(name).into_owned(),
            ),
        }),
    }
}

/// Reads a required positive dimension, Table 87's `/Width` or `/Height`.
///
/// The refusal names what the entry *is* — absent, a value of another type, or a number that
/// is no sample count — rather than which of those it might be: `issue4575.pdf` writes
/// `/Width /Height` and no `/Height` at all, and a sentence reading "missing or invalid" was
/// true of it twice over without saying either thing.
pub(super) fn positive_integer(
    document: &Document,
    dict: &Dictionary,
    key: &'static str,
) -> Result<u32, ImageError> {
    if let Some(count) =
        crate::integer_entry::dimension(document, dict, key).filter(|count| *count > 0)
    {
        return Ok(count);
    }
    let detail = match &document.get_key(dict, key) {
        Object::Null => format!("no /{key}, which Table 87 requires"),
        Object::Integer(written) => format!("/{key} is {written}, which counts no samples"),
        Object::Real(written) => format!("/{key} is {written}, which counts no samples"),
        Object::Name(name) => format!(
            "/{key} is the name /{}, where Table 87 requires an integer",
            String::from_utf8_lossy(name.as_bytes())
        ),
        other => format!(
            "/{key} is {} {}, where Table 87 requires an integer",
            article(other.type_name()),
            other.type_name()
        ),
    };
    Err(ImageError::Malformed { detail })
}

/// The indefinite article a type name takes, for a sentence that reads as one.
const fn article(word: &str) -> &'static str {
    match word.as_bytes().first() {
        Some(b'a' | b'e' | b'i' | b'o' | b'u') => "an",
        _ => "a",
    }
}

/// `here` on this thread and `there` beside it on rayon's pool, and both answers.
///
/// `there` waits in a slot a pool task takes it from; if this thread finishes `here` first and
/// finds it still waiting, it takes it back and runs it itself, so it never waits on work no
/// thread has begun — [`DecodesAhead`]'s rule for a queued decode, for the same reason. What it
/// can still wait for is a pool thread to reach the empty task, which on a pool whose every
/// thread is decoding is the length of what they are decoding. The second answer is `None` only
/// if `there` unwound, which the scope has already re-raised by then.
///
/// [`DecodesAhead`]: super::DecodesAhead
fn beside<A, B: Send, There: FnOnce() -> B + Send>(
    here: impl FnOnce() -> A,
    there: There,
) -> (A, Option<B>) {
    let waiting = std::sync::Mutex::new(Some(there));
    let answered = std::sync::Mutex::new(None);
    let take = || {
        waiting
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .take()
    };
    let answer = |job: Option<There>| {
        if let Some(job) = job {
            let made = job();
            *answered
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(made);
        }
    };
    let first = rayon::in_place_scope(|scope| {
        scope.spawn(|_| answer(take()));
        let first = here();
        answer(take());
        first
    });
    let second = answered
        .into_inner()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    (first, second)
}

#[cfg(test)]
mod tests {
    /// [`beside`] answers with both closures' results, whichever thread ran the second.
    ///
    /// [`beside`]: super::beside
    #[test]
    fn beside_answers_both() {
        let (here, there) = super::beside(|| 6 * 7, || "beside");
        assert_eq!((here, there), (42, Some("beside")));
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(1)
            .build()
            .expect("a pool");
        let (here, there) = pool.install(|| super::beside(|| 1, || 2));
        assert_eq!((here, there), (1, Some(2)));
    }
}

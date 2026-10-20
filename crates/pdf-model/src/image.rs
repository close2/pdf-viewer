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
//! - No codec: the bytes *are* samples, and `unpack` reads them at the declared depth.
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
//! ignored either way. A reader that treats the dictionary as authoritative for both, as it
//! is for every other filter, renders JPEG 2000 images in the wrong colours and cannot tell.
//!
//! **`/Decode` is the third entry and does *not* follow those two.** The clause's condition is
//! `/ColorSpace`'s **absence**, not the filter — its own sentence is quoted verbatim in
//! `decode_jpx`, where the entry is read. Where the dictionary states a colour space it
//! states the map into it as well, and dropping the array would turn a crawled catalogue cover's
//! `/Decode [1 0 1 0 1 0 1 0]` CMYK photograph into its own complement in silence (ADR 0468).
//!
//! An image this module cannot decode returns an error naming why, and the interpreter
//! reports it. Drawing a grey box in its place would be worse: the page would look
//! finished and be wrong.
//!
//! # The modules
//!
//! `dispatch` takes a stream to its route: `unpack` reads samples with no codec left, `dct` (with
//! `restart` and `cut`) decodes a `DCTDecode` frame, and `jbig2`, `ccitt` and `jpx` hand their
//! codecs to the sandbox. `space` holds the colour space and the `/Decode` map, `convert` the
//! conversion into the page's space. `mask`, `soft_mask`, `combine` and `device_scale` apply
//! §8.9.6's and §11.6.5.2's masks, and `cache` and `ahead` are what a page keeps of its images and
//! what it decodes before their `Do`. ADR 1912 is the split.

use pdf_render::{Image, SampleAlpha};
use pdf_syntax::{Dictionary, Document, Stream};

use crate::colour::Conversion;

mod ahead;
mod cache;
mod ccitt;
mod combine;
mod convert;
mod dct;
mod device_scale;
mod dispatch;
mod jbig2;
mod jpx;
mod mask;
mod soft_mask;
mod space;
#[cfg(test)]
mod tests;
mod unpack;

pub(crate) use ahead::{AHEAD_FLOOR, DecodesAhead};
pub use cache::{MaskCache, NamedStream, RasterCache, ShapeMasks, StreamIdentity};
pub use dct::{BandedDecodes, banded_decodes};
pub use device_scale::SoftMaskAtDeviceScale;
use dispatch::decode_parts_in;
pub use dispatch::{FilterSamples, filter_samples};
pub use jpx::{JpxSamples, JpxSpace, ORDINARY_JPX_SAMPLES, jpx_samples};
pub use mask::{
    ccitt_bound_below_its_height, overrides_graphics_state_mask, short_of_its_grid, unapplied_mask,
};
pub(crate) use mask::{decode_reporting_frame, decode_stencil_reporting_frame};
pub use soft_mask::unapplied_soft_mask;

/// Largest image this will decode, in samples.
///
/// 2^28 samples is a gigabyte of RGBA. Image dimensions come from the document, so an
/// unbounded allocation here is a denial of service with a two-line file.
const MAX_SAMPLES: u64 = 1 << 28;

/// Whether an image is decoded with nothing else of its page decoding beside it: the signal that
/// lets one `DCTDecode` frame with no restart interval divide its own decode across the pool
/// (`cut`, ADR 1481).
///
/// The cut's entropy pass is serial and pays for itself only where the pool would otherwise idle;
/// on a page whose other images ADR 1321 is already decoding ahead it was measured a loss. So the
/// one caller placed to know answers — [`RasterCache::parts`], which knows whether the page offered
/// its images to be decoded ahead and runs on the interpreter's own thread — and every other caller
/// says [`InFlight::AmongOthers`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum InFlight {
    /// No other image of the page is being decoded ahead, and this decode runs outside rayon's
    /// pool of two threads or more, whose threads would otherwise idle.
    Alone,
    /// Other decodes may be running beside it, or the caller cannot say.
    AmongOthers,
}

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
    /// The image is larger than `MAX_SAMPLES`.
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

/// An image `XObject` decoded as far as a grid the *file* states can take it.
///
/// Two of the standard's rasters share a unit square without sharing a grid — §8.9.6.3's
/// explicit mask and §11.6.5.2's soft-mask image — and where their refinement is small enough
/// to build, `combine_on_the_finer_grid` builds it and the answer is one raster. Where it is
/// not, §10.7.4's answer is the device's own grid, and this carries the two parts to whoever
/// knows it.
/// Cloning one shares its samples rather than copying them — an [`Image`]'s data and a
/// [`SoftMaskAtDeviceScale`]'s are both behind an [`Arc`] — which is what makes [`RasterCache`]
/// able to answer a second `Do` with the first one's raster.
///
/// [`Arc`]: std::sync::Arc
#[derive(Debug, Clone)]
pub enum Picture {
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

/// Which of ISO 32000-2 §11.3.7.2's two quantities a decoded raster's alpha channel carries.
///
/// The vocabulary is [`pdf_render::SampleAlpha`], which the raster carries as a field; what
/// lives here is the *decision*, because this is where the masks are applied. Only two facts
/// fix it: whether the image is §8.9.6.2's stencil, and whether a soft mask was multiplied in.
///
/// An explicit or colour-key mask leaves the alpha shape, and §11.6.4.3 makes a soft mask
/// override either — "shall override any explicit or colour key mask" — so no shape mask is
/// ever multiplied in beside a soft one; and an unmasked image's alpha is 1.0 everywhere, so
/// under a soft mask the product is the mask alone. What is left carrying shape beside a soft
/// mask is a stencil, whose painted areas *are* its alpha, and that is the product.
fn sample_alpha(stencil: bool, soft_masked: bool) -> SampleAlpha {
    match (stencil, soft_masked) {
        (_, false) => SampleAlpha::Shape,
        (false, true) => SampleAlpha::Opacity,
        (true, true) => SampleAlpha::Both,
    }
}

/// What a decode delivers: the picture, and beside it what the filter said where it stopped
/// short of the grid.
///
/// The two travel together because they are answered together and remembered together:
/// [`RasterCache`] holds both, so a second `Do` of a damaged image says what the first said
/// (trap 5, `tests/image_reuse.rs`). The report is *in the value* rather than noted at decode
/// time for exactly that reason — a note made on the miss would be lost on the hit.
#[derive(Debug, Clone)]
pub struct Parts {
    /// The picture, in one raster or two.
    pub picture: Picture,
    /// The filter's own sentence where it stopped on damaged data before the grid was full,
    /// worded for the report, or `None` where it reached the grid or stopped where the clause
    /// lets it. The rows it did deliver are in [`Self::picture`]; the rest are blank.
    ///
    /// Only `CCITTFaxDecode` produces one today — §7.4.6 forbids the filter any "error
    /// correction or resynchronization", so the rows before the damage are exactly the
    /// filter's output (ADR 0794). It is a report beside the drawing and not a refusal, which
    /// is the pair `doc/HANDOVER.md` sets: suppressing the drawing throws away the scan lines
    /// the file carries, and suppressing the report makes a page whose lower half this program
    /// left blank indistinguishable from one the producer left blank.
    pub shortfall: Option<String>,
    /// The sentence for a `DCTDecode` frame whose dimensions the dictionary contradicts, or
    /// `None` where they agree or the codec is another one; `image::mask::contradicted_frame` has
    /// the reading. Carried with the raster for [`Self::shortfall`]'s reason: the decode is the
    /// walk that reads the frame header, and a `Do` answered from the cache says it too.
    pub contradiction: Option<String>,
}

impl Picture {
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

/// Confirms this *build* can reach the program that decodes §7.4.6, §7.4.7 and §7.4.9's images.
///
/// `CCITTFaxDecode`, `JBIG2Decode` and `JPXDecode` are decoded by [`pdf_sandbox`]'s separate
/// worker program, and a build without it decodes none of the three. Every other route through
/// this module is unaffected, which is what makes the shortfall hard to see: the page still
/// draws, the document still opens, and only the images those three filters carry are missing.
///
/// **This exists because a measurement cannot tell that shortfall from a result.** The
/// accessibility census reads nine of `issue5481.pdf`'s structure elements as having no place at
/// all when the worker is absent — §14.8.3.3 derives an element's rectangle from what its marked
/// content *drew*, and a `JPXDecode` image that was refused drew nothing — so the census's own
/// ratchet moved with the build rather than with the tree, and passed or failed accordingly
/// (ADR 0557, `doc/traps/instruments-and-reports.md` traps 10 and 16). A gate whose numbers
/// depend on a decoded image calls this before it measures; a host may call it to say at startup
/// what it would otherwise say one image at a time.
///
/// Nothing here is about *this* process's confinement, which is [`pdf_sandbox`]'s to report and
/// is not a reason to refuse: an unconfined worker decodes the same images.
///
/// # Errors
///
/// [`ImageError::Sandboxed`], carrying the sandbox's own sentence — which names whether the
/// worker was missing, was built from another tree, or would not start.
pub fn sandboxed_decoder() -> Result<(), ImageError> {
    pdf_sandbox::Sandbox::shared()
        .confinement()
        .map(|_confinement| ())
        .map_err(|error| ImageError::Sandboxed {
            detail: error.to_string(),
        })
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
    into: &Conversion,
) -> Result<Flattened, ImageError> {
    decode_reporting_frame(document, stream, resources, fill, into).map(|(flattened, _)| flattened)
}

/// [`decode`]'s answer: one raster, and what the filter said beside it.
///
/// The same pair as [`Parts`], with the mask put back into the raster. A caller that wants the
/// picture alone still has to take the sentence and say what it did with it, which is the point
/// of the pair not being two return values.
#[derive(Debug, Clone)]
pub struct Flattened {
    /// The raster, every mask the dictionary states already in it.
    pub image: Image,
    /// See [`Parts::shortfall`].
    pub shortfall: Option<String>,
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
    into: &Conversion,
    masks: &mut MaskCache,
) -> Result<Parts, ImageError> {
    decode_parts_in(
        document,
        stream,
        (resources, fill, into),
        masks,
        InFlight::AmongOthers,
    )
}

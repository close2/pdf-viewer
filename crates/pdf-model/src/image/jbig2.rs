//! `JBIG2Decode` (ISO 32000-2 §7.4.7), decoded by the sandboxed worker and delivered as packed
//! bits, which [`super::unpack`] then reads as any one-bit image.

use pdf_sandbox::{Decoded, Request};
use pdf_syntax::{Document, ImageStream, Object};

use super::ImageError;
use super::space::{ColourSpace, Decode, Dictionaries, colour_space};
use super::unpack::{Painting, Samples, unpack};

/// Decodes a JBIG2 image through the sandbox.
///
/// ISO 32000-2 §7.4.7. The stream carries the segments for one page; `/JBIG2Globals` in the
/// filter's own `/DecodeParms` (Table 12) carries the page-0 segments, which several images
/// may share — a scanned book's symbol dictionary is written once and referenced by every
/// page, which is most of why JBIG2 compresses text so well.
///
/// What comes back is packed one bit per pixel in `DeviceGray`'s sense, so it feeds
/// [`unpack`] exactly as any other 1-bit image does.
///
/// **A stream that ends inside a segment is drawn from the segments it does carry**, and the second
/// half of the answer is the sentence the worker composed about it — see
/// `pdf_sandbox::decode::shortfall_past_the_last_segment`. Unlike §7.4.6's damaged fax (ADR 0794)
/// there is nothing to leave unpainted: 14492's page information segment states a default pixel
/// value for every pixel of the page before any region is drawn onto it, so the grid is whole and
/// what is short is the regions that reached it. ADR 0823.
pub(super) fn decode_jbig2(
    at: Dictionaries,
    source: &ImageStream,
    (width, height): (u32, u32),
    painting: Painting,
) -> Result<(Vec<u8>, Option<String>), ImageError> {
    let Dictionaries {
        document,
        dict,
        resources,
    } = at;
    let bilevel = jbig2_bilevel(document, source, (width, height))?;
    let space = if painting.is_mask {
        ColourSpace::Mask
    } else {
        colour_space(document, dict, resources, painting.into)?
    };
    let decode = Decode::read(document, dict, &space, 1);
    let rgba = unpack(
        &bilevel.rows,
        width,
        height,
        &Samples {
            bits: 1,
            space: &space,
            decode: &decode,
            // Table 87: "a CCITTFaxDecode or JBIG2Decode filter shall always deliver 1-bit
            // samples", and those are the bits below, so §8.9.6.4's test is exact here.
            colour_key: painting.colour_key,
            fill: painting.fill,
            into: painting.into,
            matte: painting.matte,
        },
    )?;
    Ok((rgba, bilevel.stopped_by))
}

/// The one-bit rows a `JBIG2Decode` filter delivers (§7.4.7), read through the sandbox with the
/// filter's own `/JBIG2Globals`, and held to the dictionary's grid.
pub(super) fn jbig2_bilevel(
    document: &Document,
    source: &ImageStream,
    (width, height): (u32, u32),
) -> Result<pdf_sandbox::Bilevel, ImageError> {
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
    Ok(bilevel)
}

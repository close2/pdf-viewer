//! A cleared `JPXDecode` image whose components are deeper than Table 87's sixteen bits,
//! written back as JPEG 2000 (ADR 2470).
//!
//! §12.5.6.23 destroys "that portion of the image data" and leaves the rest as the file had it.
//! Every other cleared image is written under `FlateDecode`, whose samples Table 87 bounds:
//! "\[t]he value shall be 1 , 2 , 4 , 8 , or (from PDF 1.5) 16". §7.4.9 lets a JPEG 2000
//! component be deeper — "bits per sample shall be between 1 to 38 inclusive" — so such an
//! image's integers go back into the only filter that holds them, at each component's own depth,
//! through the tree's lossless writer, `pdf_jpx_write`. Its file is a JP2 file because §7.4.9 has
//! the filter "expect to read a full JPX file structure"; the JP2 colour box is the one JP2
//! requires, and the dictionary states the `/ColorSpace` that, by §7.4.9, governs instead.

use pdf_jpx_write::{Colour, Opacity, Plane};
use pdf_syntax::object::{Dictionary, Name};

use super::{ImageLayout, OwnRaster, reals};

/// The cleared samples, interleaved in fields of [`OwnRaster::bits`] (three bytes, big-endian),
/// and the opacity channel in the same fields where one is carried, as one JP2 file.
///
/// # Errors
///
/// A sentence where the grid is not the samples' or a colour count no JP2 enumerated space has
/// reached here, which the planning refuses first ([`super::images`]), or where the writer
/// refused the planes.
pub(super) fn encode(
    samples: &[u8],
    opacity: Option<&[u8]>,
    layout: ImageLayout,
    own: &OwnRaster,
) -> Result<Vec<u8>, String> {
    let colour = match layout.components {
        1 => Colour::Greyscale,
        3 => Colour::Srgb,
        other => {
            return Err(format!(
                "a JPEG 2000 image of {other} colour components, which no JP2 enumerated colour \
                 space has"
            ));
        }
    };
    let field = layout.bits / 8;
    let pixels = layout.width.saturating_mul(layout.height);
    let channel = |data: &[u8], index: usize, count: usize| -> Vec<u32> {
        (0..pixels)
            .map(|pixel| {
                let at = pixel
                    .saturating_mul(count)
                    .saturating_add(index)
                    .saturating_mul(field);
                data.get(at..at.saturating_add(field)).map_or(0, |bytes| {
                    bytes
                        .iter()
                        .fold(0u32, |value, byte| (value << 8) | u32::from(*byte))
                })
            })
            .collect()
    };
    let needed = pixels
        .saturating_mul(layout.components)
        .saturating_mul(field);
    if samples.len() < needed
        || opacity.is_some_and(|alpha| alpha.len() < pixels.saturating_mul(field))
    {
        return Err("the cleared samples are shorter than the image grid".to_owned());
    }
    let colours: Vec<Vec<u32>> = (0..layout.components)
        .map(|index| channel(samples, index, layout.components))
        .collect();
    let alpha = opacity.map(|data| channel(data, 0, 1));
    let planes: Vec<Plane<'_>> = colours
        .iter()
        .zip(&own.depths)
        .map(|(samples, depth)| Plane {
            depth: *depth,
            samples,
        })
        .collect();
    let kind = if own.premultiplied {
        Opacity::Premultiplied
    } else {
        Opacity::Plain
    };
    let opacity = alpha.as_ref().map(|samples| {
        (
            Plane {
                depth: own.opacity_depth,
                samples,
            },
            kind,
        )
    });
    let width = u32::try_from(layout.width).map_err(|_| "the image is too wide".to_owned())?;
    let height = u32::try_from(layout.height).map_err(|_| "the image is too tall".to_owned())?;
    pdf_jpx_write::jp2((width, height), colour, &planes, opacity)
        .map_err(|error| format!("the cleared samples were not written as JPEG 2000: {error}"))
}

/// What a `JPXDecode` image's dictionary states beyond what [`super::own_dictionary`] carried:
/// each component's `/Decode` pair at its own depth — §8.9.5.2 maps a `JPXDecode` component's
/// integers over the domain of the depth the JPEG 2000 data defines for it, which may differ per
/// component — and no `/BitsPerComponent`, which Table 87 does not require of a `JPXDecode`
/// image.
pub(super) fn describe(own: &OwnRaster, dict: &mut Dictionary) {
    let pairs: Vec<f32> = own
        .pairs
        .iter()
        .flat_map(|(low, high)| [*low, *high])
        .collect();
    dict.insert(Name::new(&b"Decode"[..]), reals(&pairs));
}

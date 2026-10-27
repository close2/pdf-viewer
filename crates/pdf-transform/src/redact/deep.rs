//! A cleared `JPXDecode` image whose components are deeper than Table 87's sixteen bits, or
//! signed, written back as JPEG 2000 (ADRs 2470, 2480, 2481).
//!
//! §12.5.6.23 destroys "that portion of the image data" and leaves the rest as the file had it.
//! Every other cleared image is written under `FlateDecode`, whose samples Table 87 bounds:
//! "\[t]he value shall be 1 , 2 , 4 , 8 , or (from PDF 1.5) 16". §7.4.9 lets a JPEG 2000
//! component be deeper — "bits per sample shall be between 1 to 38 inclusive" — so such an
//! image's integers go back into the only filter that holds them, at each component's own depth,
//! through the tree's lossless writer, `pdf_jpx_write`. A component whose `Ssiz` states a sign
//! goes the same way, because no other filter's samples can state one (ADR 2481). Its file is a
//! JP2 file for one or three colour components, whose colour boxes JP2 enumerates, and a JPX
//! file otherwise (ADR 2480), because §7.4.9 has the filter "expect to read a full JPX file
//! structure"; the colour specification box is the one the source file stated, and the
//! dictionary states the `/ColorSpace` that, by §7.4.9, governs instead.

use pdf_jpx_write::{ColourBox, Opacity, Plane, Space};
use pdf_model::jpeg2000::ColourSpecification;
use pdf_syntax::object::{Dictionary, Name};

use super::{ImageLayout, OwnRaster, reals};

/// A colour specification box of the source file, owned, as T.801 M.11.7.2 lays its fields out
/// (ADR 2480).
#[derive(Clone)]
pub(super) struct StatedColour {
    /// `METH`: 1 enumerated, 2 restricted ICC, 3 any ICC, 4 vendor colour.
    method: u8,
    /// `PREC`.
    precedence: i8,
    /// `APPROX`, 1 to 4: a JPX file may not state 0 (T.801 M.11.7.2).
    approximation: u8,
    /// `METHDAT`.
    data: Vec<u8>,
}

impl StatedColour {
    /// The box as the source stated it, or a sentence where its method is one T.801 Table M.22
    /// reserves, whose data a writer cannot show it preserves.
    ///
    /// `APPROX` 0, which a JP2 file sets and its readers ignore, is written as 1, the least that
    /// T.801 Table M.23 states, because a JPX file may not state 0.
    pub(super) fn from_header(stated: &ColourSpecification<'_>) -> Result<Self, String> {
        let data = match stated.method {
            1 => {
                let space = stated
                    .enumerated
                    .ok_or_else(|| "an enumerated colour specification with no space".to_owned())?;
                let mut data = space.to_be_bytes().to_vec();
                data.extend_from_slice(stated.parameters);
                data
            }
            2 | 3 => stated
                .profile
                .ok_or_else(|| "an ICC colour specification with no profile".to_owned())?
                .to_vec(),
            4 => stated.reserved.to_vec(),
            other => {
                return Err(format!(
                    "a colour specification of method {other}, which T.801 Table M.22 reserves"
                ));
            }
        };
        Ok(Self {
            method: stated.method,
            precedence: stated.precedence,
            approximation: stated.approximation.max(1),
            data,
        })
    }

    /// The writer's space for a file of `components` colour channels: JP2's two enumerations
    /// for one and three, enumerated 12 for the plain CMYK box, and the stated box otherwise.
    fn space(&self, components: usize) -> Space<'_> {
        match (components, self.method, self.data.as_slice()) {
            (1, ..) => Space::Greyscale,
            (3, ..) => Space::Srgb,
            (4, 1, [0, 0, 0, 12]) => Space::Cmyk,
            _ => Space::Stated(ColourBox {
                method: self.method,
                precedence: self.precedence,
                approximation: self.approximation,
                data: &self.data,
            }),
        }
    }
}

/// The cleared samples, interleaved in fields of [`OwnRaster::bits`] (three bytes, big-endian),
/// and the opacity channel in the same fields where one is carried, as one JP2 or JPX file.
///
/// # Errors
///
/// A sentence where the grid is not the samples', a colour count other than four has no stated
/// box to be described by, or the writer refused the planes.
pub(super) fn encode(
    samples: &[u8],
    opacity: Option<&[u8]>,
    layout: ImageLayout,
    own: &OwnRaster,
) -> Result<Vec<u8>, String> {
    let space = match &own.colour_box {
        Some(stated) => stated.space(layout.components),
        None => match layout.components {
            1 => Space::Greyscale,
            3 => Space::Srgb,
            4 => Space::Cmyk,
            other => {
                return Err(format!(
                    "a JPEG 2000 image of {other} colour components that states no colour \
                     specification box"
                ));
            }
        },
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
    // The channel is typed plain in the `cdef` box even where `/SMaskInData` 2 says the colour is
    // premultiplied: Table 87's entry is what states that, and this tree's decoder reads only
    // channel type 1 as opacity (ADR 2480).
    let kind = Opacity::Plain;
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
    pdf_jpx_write::file((width, height), space, &planes, opacity, &own.signed)
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

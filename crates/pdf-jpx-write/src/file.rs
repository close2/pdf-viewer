//! The JP2 file format of ISO/IEC 15444-1 Annex I around a codestream, because ISO 32000-2
//! §7.4.9 has the filter "expect to read a full JPX file structure" and JP2 is a subset of JPX.
//!
//! The boxes, in I.4's order: the signature (I.5.1), the file type naming the `jp2 ` brand
//! (I.5.2), the header superbox (I.5.3) holding the image header, the bits-per-component box
//! where the components' depths differ, the colour specification and, where an opacity channel
//! is carried, the channel definition, and then the contiguous codestream (I.5.4).

use crate::{Colour, Opacity, Plane};

/// A box of type `kind` around `content` (I.4): its length, which counts the eight bytes before
/// the content, as `LBox` — or as `XLBox` after an `LBox` of 1 where it passes 2^32 − 1.
#[expect(
    clippy::arithmetic_side_effects,
    reason = "a content length is far below 2^64 − 16"
)]
fn boxed(out: &mut Vec<u8>, kind: [u8; 4], content: &[u8]) {
    if let Ok(length) = u32::try_from(content.len() + 8) {
        out.extend_from_slice(&length.to_be_bytes());
        out.extend_from_slice(&kind);
    } else {
        out.extend_from_slice(&1u32.to_be_bytes());
        out.extend_from_slice(&kind);
        out.extend_from_slice(&(content.len() as u64 + 16).to_be_bytes());
    }
    out.extend_from_slice(content);
}

/// The JP2 file holding `codestream`, whose components are `planes`: the colour channels
/// first, and the opacity channel last where `opacity` names one.
#[expect(
    clippy::arithmetic_side_effects,
    clippy::cast_possible_truncation,
    reason = "the channel count is validated to fit `Csiz`, so it fits the two-byte `NC` and \
              `N`, each channel index fits `Cn`, and the sizes are small; every depth is \
              validated to be at least 1"
)]
pub(crate) fn write(
    (width, height): (u32, u32),
    planes: &[Plane<'_>],
    colour: Colour,
    opacity: Option<Opacity>,
    codestream: &[u8],
) -> Vec<u8> {
    let mut out = Vec::with_capacity(codestream.len() + 128);
    boxed(&mut out, *b"jP  ", &[0x0D, 0x0A, 0x87, 0x0A]);
    // I.5.2: the brand, minor version 0, and the compatibility list naming the brand.
    boxed(&mut out, *b"ftyp", b"jp2 \0\0\0\0jp2 ");
    let mut header = Vec::new();
    let uniform = planes.windows(2).all(|pair| pair[0].depth == pair[1].depth);
    // I.5.3.1: HEIGHT, WIDTH, NC, BPC (the shared Ssiz, or 255 where they differ), C = 7 for
    // this Recommendation's coding, UnkC 0 because the colour box states the space, IPR 0.
    let mut ihdr = Vec::with_capacity(14);
    ihdr.extend_from_slice(&height.to_be_bytes());
    ihdr.extend_from_slice(&width.to_be_bytes());
    ihdr.extend_from_slice(&(planes.len() as u16).to_be_bytes());
    let bpc = match planes.first() {
        Some(first) if uniform => first.depth - 1,
        _ => 0xFF,
    };
    ihdr.extend_from_slice(&[bpc, 7, 0, 0]);
    boxed(&mut header, *b"ihdr", &ihdr);
    if !uniform {
        // I.5.3.2: each component's depth − 1, unsigned.
        let depths: Vec<u8> = planes.iter().map(|plane| plane.depth - 1).collect();
        boxed(&mut header, *b"bpcc", &depths);
    }
    // I.5.3.3: METH 1, an enumerated space; PREC 0; APPROX 0; EnumCS.
    let enumerated: u32 = match colour {
        Colour::Srgb => 16,
        Colour::Greyscale => 17,
    };
    let mut colr = vec![1, 0, 0];
    colr.extend_from_slice(&enumerated.to_be_bytes());
    boxed(&mut header, *b"colr", &colr);
    if let Some(kind) = opacity {
        // I.5.3.6: each colour channel is its colour (Typ 0, Asoc its number), and the last
        // channel is opacity (Typ 1) or premultiplied opacity (Typ 2) over the whole image
        // (Asoc 0) — the one opacity channel ISO 32000-2 §7.4.9 allows.
        let colours = planes.len().saturating_sub(1);
        let mut cdef = Vec::with_capacity(2 + 6 * planes.len());
        cdef.extend_from_slice(&(planes.len() as u16).to_be_bytes());
        for channel in 0..colours {
            cdef.extend_from_slice(&(channel as u16).to_be_bytes());
            cdef.extend_from_slice(&0u16.to_be_bytes());
            cdef.extend_from_slice(&(channel as u16).saturating_add(1).to_be_bytes());
        }
        let typ: u16 = match kind {
            Opacity::Plain => 1,
            Opacity::Premultiplied => 2,
        };
        cdef.extend_from_slice(&(colours as u16).to_be_bytes());
        cdef.extend_from_slice(&typ.to_be_bytes());
        cdef.extend_from_slice(&0u16.to_be_bytes());
        boxed(&mut header, *b"cdef", &cdef);
    }
    boxed(&mut out, *b"jp2h", &header);
    boxed(&mut out, *b"jp2c", codestream);
    out
}

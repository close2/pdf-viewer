//! The JP2 file format of ISO/IEC 15444-1 Annex I around a codestream, because ISO 32000-2
//! §7.4.9 has the filter "expect to read a full JPX file structure" and JP2 is a subset of JPX.
//!
//! The boxes, in I.4's order: the signature (I.5.1), the file type naming the `jp2 ` brand
//! (I.5.2), the header superbox (I.5.3) holding the image header, the bits-per-component box
//! where the components' depths or signs differ, the colour specification and, where an opacity
//! channel is carried, the channel definition, and then the contiguous codestream (I.5.4).
//!
//! A space JP2 does not enumerate (four colour channels, or any other count) makes the file a
//! JPX file instead, ITU-T T.801 | ISO/IEC 15444-2 Annex M: the `jpx ` brand (M.8), the Reader
//! Requirements box that must follow the file type box (M.11.1), and the colour specification
//! box in its JPX form (M.11.7.2), whose approximation field may not be 0.

use crate::{Opacity, Plane, Space, is_signed};

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
    signed: &[bool],
    space: Space<'_>,
    opacity: Option<Opacity>,
    codestream: &[u8],
) -> Vec<u8> {
    let mut out = Vec::with_capacity(codestream.len() + 192);
    boxed(&mut out, *b"jP  ", &[0x0D, 0x0A, 0x87, 0x0A]);
    let jpx = space.is_jpx();
    if jpx {
        // M.8: the brand `jpx ` and at least one compatibility entry naming it. `jpxb` (M.9.2)
        // is not claimed: the baseline's colour list (M.9.2.4) has no CMYK and no vendor space.
        boxed(&mut out, *b"ftyp", b"jpx \0\0\0\0jpx ");
        boxed(&mut out, *b"rreq", &reader_requirements(space, opacity));
    } else {
        // I.5.2: the brand, minor version 0, and the compatibility list naming the brand.
        boxed(&mut out, *b"ftyp", b"jp2 \0\0\0\0jp2 ");
    }
    let mut header = Vec::new();
    let sign = |index: usize| -> u8 { if is_signed(signed, index) { 0x80 } else { 0 } };
    let uniform = planes
        .windows(2)
        .enumerate()
        .all(|(index, pair)| pair[0].depth == pair[1].depth && sign(index) == sign(index + 1));
    // I.5.3.1: HEIGHT, WIDTH, NC, BPC (the shared Ssiz, or 255 where they differ), C = 7 for
    // this Recommendation's coding, UnkC 0 because the colour box states the space, IPR 0.
    let mut ihdr = Vec::with_capacity(14);
    ihdr.extend_from_slice(&height.to_be_bytes());
    ihdr.extend_from_slice(&width.to_be_bytes());
    ihdr.extend_from_slice(&(planes.len() as u16).to_be_bytes());
    // Table I-6's BPC counts the sign bit like Table A-11's Ssiz: the high bit marks a signed
    // component.
    let bpc = match planes.first() {
        Some(first) if uniform => (first.depth - 1) | sign(0),
        _ => 0xFF,
    };
    ihdr.extend_from_slice(&[bpc, 7, 0, 0]);
    boxed(&mut header, *b"ihdr", &ihdr);
    if !uniform {
        // I.5.3.2: each component's depth − 1, with its sign bit.
        let depths: Vec<u8> = planes
            .iter()
            .enumerate()
            .map(|(index, plane)| (plane.depth - 1) | sign(index))
            .collect();
        boxed(&mut header, *b"bpcc", &depths);
    }
    boxed(&mut header, *b"colr", &space.colour_box());
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

/// The Reader Requirements box's contents, ITU-T T.801 M.11.1, for the file this module writes.
///
/// The file is one contiguous codestream with no extension, of the kind T.800 defines, with or
/// without an opacity channel, in one colour space. M.6.2.1 factors what a reader must support
/// into AND-separated columns of OR-separated options, and each feature's row of the table (its
/// compatibility mask) has a bit set for every column it can satisfy:
///
/// | bit | column | features that satisfy it (T.801 Table M.14) |
/// |---|---|---|
/// | 0 | the colour space | the one feature naming it, where T.801 Table M.14 has one |
/// | 1 | the codestream's coding | 1 (no extensions), 5 (T.800 JPEG 2000) |
/// | 2 | the codestream's placement | 12 (contiguous) |
/// | 3 | opacity | 8 (none), 9 (opacity), 10 (premultiplied opacity) |
///
/// A reader that supports each column reads the whole file, so the Fully Understand Aspects
/// mask and the Decode Completely mask are alike. A colour space with no feature of its own in
/// T.801 Table M.14 (a vendor colour specification, an enumerated space it does not list) is
/// left out of the expression: its colour specification box still states it, and no column would
/// otherwise be satisfiable by a flag the table does not have.
fn reader_requirements(space: Space<'_>, opacity: Option<Opacity>) -> Vec<u8> {
    const COLOUR: u8 = 0b0001;
    const CODING: u8 = 0b0010;
    const PLACEMENT: u8 = 0b0100;
    const OPACITY: u8 = 0b1000;
    let opacity_flag: u16 = match opacity {
        None => 8,
        Some(Opacity::Plain) => 9,
        Some(Opacity::Premultiplied) => 10,
    };
    let mut features: Vec<(u16, u8)> = vec![
        (1, CODING),
        (5, CODING),
        (12, PLACEMENT),
        (opacity_flag, OPACITY),
    ];
    let mut expression = CODING | PLACEMENT | OPACITY;
    if let Some(flag) = space.feature() {
        features.push((flag, COLOUR));
        expression |= COLOUR;
    }
    features.sort_unstable();
    // ML 1; FUAM; DCM; NSF; then each SF and its one-byte mask; NVF 0.
    let mut out = vec![1, expression, expression];
    // At most five features are listed (coding twice, placement, opacity, colour), so the count
    // fits `NSF`'s two bytes and the fallback is never taken.
    out.extend_from_slice(
        &u16::try_from(features.len())
            .unwrap_or(u16::MAX)
            .to_be_bytes(),
    );
    for (flag, mask) in features {
        out.extend_from_slice(&flag.to_be_bytes());
        out.push(mask);
    }
    out.extend_from_slice(&0u16.to_be_bytes());
    out
}

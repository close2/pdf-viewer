//! A producer's codestream, byte for byte, given the JP2 file structure ISO 32000-2 §7.4.9 has the
//! filter "expect to read a full JPX file structure".
//!
//! A bare ISO/IEC 15444-1 codestream (Annex A, from its `SOC` marker) holds every sample an image
//! has and none of the boxes of Annex I, so it is not a JP2 file and not JPX data at all; the
//! boxes are a description of the codestream, and its own `SIZ` segment (A.5.1) states every
//! field they need — the grid, the component count, each component's depth and sign. Nothing in
//! the codestream is decoded or written again, so the samples any decoder reads from the file are
//! the samples it read from the codestream (ADR 2695).
//!
//! The file is the one [`crate::jp2`] writes around its own codestream, so the boxes and their
//! order are `file`'s. Two shapes are refused rather than described: a codestream whose `Rsiz`
//! (ISO/IEC 15444-1 Table A.10) says it needs a capability beyond that part's — ISO/IEC
//! 15444-2's extensions or a later part's — which a JP2 file may not hold and ITU-T T.801
//! M.9.2.3's baseline does not admit; and colour channels of different depths, which a JP2 file can state in a
//! `bpcc` box but ISO 19005-2 section 6.2.8.3 and ISO 19005-4 section 6.2.7.3 do not admit.

use crate::{Colour, Error, Plane, Space, file};

/// `SOC`, the start of a codestream (A.4.1).
const START_OF_CODESTREAM: [u8; 2] = [0xFF, 0x4F];

/// `SIZ`, which A.5.1 makes the first marker segment after `SOC`.
const IMAGE_AND_TILE_SIZE: [u8; 2] = [0xFF, 0x51];

/// The bytes before a codestream's first component field: `SOC`, then `SIZ`'s marker, `Lsiz`,
/// `Rsiz`, the four grid fields, the four tile fields and `Csiz`.
const FIXED_FIELDS: usize = 2 + 2 + 2 + 2 + 4 * 4 + 4 * 4 + 2;

/// `Rsiz`'s bits that say a reader needs more than ISO/IEC 15444-1: bit 15 for ISO/IEC 15444-2's
/// extensions, bit 14 for the capabilities a `CAP` marker segment lists.
const BEYOND_PART_ONE: u16 = 0xC000;

/// The deepest component A.5.1's `Ssiz` states: 38 bits, which §7.4.9 also allows.
const DEEPEST: u8 = 38;

/// Whether `data` begins as a bare codestream does, with `SOC` and then `SIZ`.
#[must_use]
pub fn is_bare_codestream(data: &[u8]) -> bool {
    data.starts_with(&START_OF_CODESTREAM) && data.get(2..4) == Some(IMAGE_AND_TILE_SIZE.as_slice())
}

/// The fields a JP2 header states, read from a codestream's `SIZ`.
struct Size {
    /// `Xsiz` − `XOsiz`.
    width: u32,
    /// `Ysiz` − `YOsiz`.
    height: u32,
    /// Each component's depth, 1 to 38.
    depths: Vec<u8>,
    /// Each component's sign, from `Ssiz`'s high bit.
    signed: Vec<bool>,
}

/// A big-endian `u16` at `at`.
fn u16_at(data: &[u8], at: usize) -> Option<u16> {
    data.get(at..)?
        .first_chunk::<2>()
        .map(|b| u16::from_be_bytes(*b))
}

/// A big-endian `u32` at `at`.
fn u32_at(data: &[u8], at: usize) -> Option<u32> {
    data.get(at..)?
        .first_chunk::<4>()
        .map(|b| u32::from_be_bytes(*b))
}

/// The codestream's `SIZ`, read where it is whole.
///
/// # Errors
///
/// [`Error::NotACodestream`] where `SOC` and a whole `SIZ` do not open the data,
/// [`Error::BeyondPartOne`] where `Rsiz` asks for more than ISO/IEC 15444-1, [`Error::EmptyGrid`]
/// and [`Error::Components`] where the fields state no sample.
fn size(codestream: &[u8]) -> Result<Size, Error> {
    if !is_bare_codestream(codestream) {
        return Err(Error::NotACodestream);
    }
    let field = |at: usize| u32_at(codestream, at).ok_or(Error::NotACodestream);
    let capabilities = u16_at(codestream, 6).ok_or(Error::NotACodestream)?;
    if capabilities & BEYOND_PART_ONE != 0 {
        return Err(Error::BeyondPartOne(capabilities));
    }
    let (x, y, x_offset, y_offset) = (field(8)?, field(12)?, field(16)?, field(20)?);
    let (Some(width), Some(height)) = (x.checked_sub(x_offset), y.checked_sub(y_offset)) else {
        return Err(Error::EmptyGrid);
    };
    if width == 0 || height == 0 {
        return Err(Error::EmptyGrid);
    }
    let count = usize::from(u16_at(codestream, FIXED_FIELDS - 2).ok_or(Error::NotACodestream)?);
    if count == 0 {
        return Err(Error::Components(0));
    }
    let mut depths = Vec::with_capacity(count);
    let mut signed = Vec::with_capacity(count);
    for component in 0..count {
        let at = component
            .checked_mul(3)
            .and_then(|offset| offset.checked_add(FIXED_FIELDS))
            .ok_or(Error::NotACodestream)?;
        let ssiz = *codestream.get(at).ok_or(Error::NotACodestream)?;
        // ISO/IEC 15444-1 Table A.11: the low seven bits are the depth less one, the high bit the
        // sign.
        let depth = (ssiz & 0x7F).saturating_add(1);
        // ISO/IEC 15444-1 Table A.11 states no depth past 38, so a larger field is not a
        // codestream's.
        if depth > DEEPEST {
            return Err(Error::NotACodestream);
        }
        depths.push(depth);
        signed.push(ssiz & 0x80 != 0);
    }
    Ok(Size {
        width,
        height,
        depths,
        signed,
    })
}

/// A JP2 file (ISO/IEC 15444-1 Annex I) holding `codestream` unchanged, its colour specification
/// box enumerating `colour`.
///
/// The image header and, where the components' depths or signs differ, the bits-per-component
/// box are the codestream's own `SIZ` restated (I.5.3.1, I.5.3.2). The colour box is the one
/// field the codestream does not state: ISO 32000-2 §7.4.9 has a `/ColorSpace` the image
/// dictionary states override it, and a caller writes this file only beside one.
///
/// # Errors
///
/// [`Error::NotACodestream`], [`Error::BeyondPartOne`], [`Error::EmptyGrid`] and
/// [`Error::Components`] from the codestream's `SIZ`;
/// [`Error::ColourChannels`] where its component count is not `colour`'s; [`Error::ColourDepths`]
/// where its colour channels differ in depth.
pub fn around(codestream: &[u8], colour: Colour) -> Result<Vec<u8>, Error> {
    let size = size(codestream)?;
    if size.depths.len() != colour.channels() {
        return Err(Error::ColourChannels {
            colour,
            planes: size.depths.len(),
        });
    }
    if size.depths.windows(2).any(|pair| pair[0] != pair[1]) {
        return Err(Error::ColourDepths(size.depths));
    }
    // `file` reads a plane's depth and nothing else of it; the samples are the codestream's.
    let planes: Vec<Plane<'_>> = size
        .depths
        .iter()
        .map(|&depth| Plane {
            depth,
            samples: &[],
        })
        .collect();
    Ok(file::write(
        (size.width, size.height),
        &planes,
        &size.signed,
        Space::from(colour),
        None,
        codestream,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A `SIZ` for a `width` × `height` grid of the components `ssiz`, after `SOC`, with `rsiz`.
    fn header(rsiz: u16, width: u32, height: u32, ssiz: &[u8]) -> Vec<u8> {
        let mut out = vec![0xFF, 0x4F, 0xFF, 0x51];
        let length = u16::try_from(ssiz.len().saturating_mul(3).saturating_add(38)).expect("small");
        out.extend_from_slice(&length.to_be_bytes());
        out.extend_from_slice(&rsiz.to_be_bytes());
        for value in [width, height, 0, 0, width, height, 0, 0] {
            out.extend_from_slice(&value.to_be_bytes());
        }
        out.extend_from_slice(&u16::try_from(ssiz.len()).expect("small").to_be_bytes());
        for &s in ssiz {
            out.extend_from_slice(&[s, 1, 1]);
        }
        out
    }

    #[test]
    fn the_codestream_is_carried_byte_for_byte_after_the_header_it_states() {
        let codestream = header(0, 100, 64, &[7, 7, 7]);
        let file = around(&codestream, Colour::Srgb).expect("a three-channel codestream");
        assert!(
            file.ends_with(&codestream),
            "the codestream is the last box's payload"
        );
        let at = file
            .windows(4)
            .position(|window| window == b"ihdr")
            .expect("an image header");
        // HEIGHT, WIDTH, NC, BPC.
        assert_eq!(&file[at + 4..at + 8], &64u32.to_be_bytes());
        assert_eq!(&file[at + 8..at + 12], &100u32.to_be_bytes());
        assert_eq!(&file[at + 12..at + 14], &3u16.to_be_bytes());
        assert_eq!(file[at + 14], 7);
        assert!(!file.windows(4).any(|window| window == b"bpcc"));
    }

    #[test]
    fn a_signed_component_is_stated_signed() {
        let file = around(&header(0, 4, 4, &[0x87]), Colour::Greyscale).expect("one channel");
        let at = file.windows(4).position(|w| w == b"ihdr").expect("ihdr");
        assert_eq!(file[at + 14], 0x87);
    }

    #[test]
    fn what_is_not_a_part_one_codestream_of_the_colours_channels_is_refused() {
        assert_eq!(
            around(b"\0\0\0\x0cjP  ", Colour::Srgb),
            Err(Error::NotACodestream)
        );
        assert_eq!(
            around(&header(0, 4, 4, &[7, 7, 7])[..20], Colour::Srgb),
            Err(Error::NotACodestream)
        );
        assert_eq!(
            around(&header(0x8000, 4, 4, &[7, 7, 7]), Colour::Srgb),
            Err(Error::BeyondPartOne(0x8000))
        );
        assert_eq!(
            around(&header(0x4000, 4, 4, &[7, 7, 7]), Colour::Srgb),
            Err(Error::BeyondPartOne(0x4000))
        );
        assert_eq!(
            around(&header(0, 4, 4, &[7]), Colour::Srgb),
            Err(Error::ColourChannels {
                colour: Colour::Srgb,
                planes: 1
            })
        );
        assert_eq!(
            around(&header(0, 4, 4, &[7, 7, 15]), Colour::Srgb),
            Err(Error::ColourDepths(vec![8, 8, 16]))
        );
        assert_eq!(
            around(&header(0, 4, 4, &[38]), Colour::Greyscale),
            Err(Error::NotACodestream)
        );
        assert_eq!(
            around(&header(0, 0, 4, &[7]), Colour::Greyscale),
            Err(Error::EmptyGrid)
        );
    }
}

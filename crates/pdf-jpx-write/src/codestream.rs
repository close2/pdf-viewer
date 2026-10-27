//! The codestream of ISO/IEC 15444-1 Annex A: the main header, one tile-part and the packets in
//! it.
//!
//! What it states, and why each choice is the smallest one that is lossless:
//!
//! - **One tile** the size of the image (A.5.1's `XTsiz`, `YTsiz`), every component at full
//!   resolution (`XRsiz` = `YRsiz` = 1).
//! - **Zero decomposition levels with the reversible 5-3 filter** (A.6.1,
//!   ISO/IEC 15444-1 Table A.20): with no level there is no transform to apply, so the one band
//!   is the LL band and each coefficient is the sample less G.1.2's DC level shift of
//!   2^(depth − 1) — an integer the decoder adds back exactly.
//! - **No quantisation** (A.6.4, ISO/IEC 15444-1 Table A.28's `Sqcd` style 0), the reversible
//!   path's: the band's exponent `εb` is the deepest component's depth and two guard bits are
//!   stated, so E.1.1.1's `Mb = G + εb − 1` is one more bit plane than any coefficient's
//!   magnitude needs.
//! - **No multiple component transformation, one layer, LRCP order**, default precincts of
//!   2^15 (A.6.1), 64 × 64 code-blocks of style 0.

use crate::Plane;
use crate::tier1::{CodedBlock, encode_block};
use crate::tier2::packet;

/// Code-block side, 2^6 (A.6.1: `xcb` = `ycb` = 4 states 2^(4 + 2)).
const BLOCK: usize = 64;
/// The default precinct side at resolution level 0, 2^15 (A.6.1, `Scod` bit 0 clear).
const PRECINCT: usize = 1 << 15;
/// E.1.1.1's guard bits, stated in `Sqcd`.
const GUARD_BITS: u8 = 2;

/// Appends a marker segment: its code, its length (which counts itself), and its parameters
/// (A.1.4).
#[expect(
    clippy::arithmetic_side_effects,
    clippy::cast_possible_truncation,
    reason = "every segment here is far below 65 535 bytes: SIZ is 38 + 3 × 16 384 at the most"
)]
fn segment(out: &mut Vec<u8>, marker: u8, parameters: &[u8]) {
    out.extend_from_slice(&[0xFF, marker]);
    out.extend_from_slice(&((parameters.len() + 2) as u16).to_be_bytes());
    out.extend_from_slice(parameters);
}

/// The codestream for `planes` on a `width` × `height` grid, already validated by the caller.
#[expect(
    clippy::arithmetic_side_effects,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    reason = "a sample is validated below 2^24, so it is an `i32` unchanged; the component \
              count is validated to fit `Csiz` and the depth to fit `Ssiz` and `εb`; the \
              tile-part length is stated as 0 where it would pass 2^32 − 1"
)]
pub(crate) fn write(width: u32, height: u32, planes: &[Plane<'_>]) -> Vec<u8> {
    let mut out = vec![0xFF, 0x4F]; // SOC, A.4.1.
    // SIZ, A.5.1.
    let mut siz = Vec::with_capacity(36 + 3 * planes.len());
    siz.extend_from_slice(&0u16.to_be_bytes()); // Rsiz: this Recommendation's capabilities.
    for value in [width, height, 0, 0, width, height, 0, 0] {
        siz.extend_from_slice(&value.to_be_bytes());
    }
    siz.extend_from_slice(&(planes.len() as u16).to_be_bytes());
    for plane in planes {
        // Ssiz: depth − 1 with the sign bit clear, then XRsiz and YRsiz.
        siz.extend_from_slice(&[plane.depth - 1, 1, 1]);
    }
    segment(&mut out, 0x51, &siz);
    // COD, A.6.1: Scod 0; LRCP, one layer, no MCT; zero levels, 64 × 64 code-blocks, style 0,
    // the 5-3 reversible filter.
    segment(&mut out, 0x52, &[0, 0, 0, 1, 0, 0, 4, 4, 0, 1]);
    // QCD, A.6.4: no quantisation, two guard bits, and the one band's exponent.
    let deepest = planes.iter().map(|plane| plane.depth).max().unwrap_or(1);
    segment(&mut out, 0x5C, &[GUARD_BITS << 5, deepest << 3]);
    let most = u32::from(GUARD_BITS) + u32::from(deepest) - 1;

    let mut body = Vec::new();
    let (width_us, height_us) = (width as usize, height as usize);
    // B.12.1.1's LRCP order: one layer, one resolution, then component, then precinct.
    for plane in planes {
        let shift = 1i32 << (plane.depth - 1);
        for precinct_top in (0..height_us).step_by(PRECINCT) {
            for precinct_left in (0..width_us).step_by(PRECINCT) {
                let precinct_bottom = height_us.min(precinct_top + PRECINCT);
                let precinct_right = width_us.min(precinct_left + PRECINCT);
                let columns = (precinct_right - precinct_left).div_ceil(BLOCK);
                let mut blocks: Vec<CodedBlock> = Vec::new();
                for top in (precinct_top..precinct_bottom).step_by(BLOCK) {
                    for left in (precinct_left..precinct_right).step_by(BLOCK) {
                        let bottom = precinct_bottom.min(top + BLOCK);
                        let right = precinct_right.min(left + BLOCK);
                        let mut coefficients = Vec::with_capacity((bottom - top) * (right - left));
                        for row in top..bottom {
                            for col in left..right {
                                // Validated below 2^depth, and depth is at most 24, so the
                                // sample fits an `i32` and the shift cannot overflow it.
                                let sample = plane.samples[row * width_us + col] as i32;
                                coefficients.push(sample - shift);
                            }
                        }
                        blocks.push(encode_block(&coefficients, right - left, bottom - top));
                    }
                }
                body.extend_from_slice(&packet(&blocks, columns, most));
            }
        }
    }
    // SOT, A.4.2: tile 0, its length from the SOT marker to the end of its data, part 0 of 1.
    let length = 12 + 2 + body.len();
    let psot = u32::try_from(length).unwrap_or(0);
    let mut sot = Vec::with_capacity(8);
    sot.extend_from_slice(&0u16.to_be_bytes());
    sot.extend_from_slice(&psot.to_be_bytes());
    sot.extend_from_slice(&[0, 1]);
    segment(&mut out, 0x90, &sot);
    out.extend_from_slice(&[0xFF, 0x93]); // SOD, A.4.3.
    out.extend_from_slice(&body);
    out.extend_from_slice(&[0xFF, 0xD9]); // EOC, A.4.4.
    out
}

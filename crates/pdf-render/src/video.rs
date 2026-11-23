//! A decoded video picture's colour conversion, stated once for both backends: what a 4:2:0
//! picture's three planes and its stream's signal description mean as RGB (RFC 0015 section 6,
//! round 4; ADR 3529).
//!
//! `render-cpu` draws the conversion as the oracle and `render-raster` on the device, both from the
//! [`Conversion`] this module resolves, so the two compute one formula from one set of numbers and
//! a difference between them is a backend's arithmetic, never a reading (the same contract the
//! display list keeps for pages).
//!
//! # The reading, and where it comes from
//!
//! The stream states its signal in the sequence parameter set's VUI, ITU-T H.264 Annex E (E.1.1,
//! E.2.1): `video_full_range_flag`, `colour_primaries`, `transfer_characteristics`,
//! `matrix_coefficients` and `chroma_sample_loc_type_top_field`. ITU-T H.264 is not held in `doc/`;
//! everything below is cited from it and paraphrased, never quoted.
//!
//! - **Range** (Annex E, the equations under `video_full_range_flag`): with the flag 0, an 8-bit
//!   luma sample `Y` stands for `E'Y = (Y − 16) / 219` and a chroma sample `C` for
//!   `E'P = (C − 128) / 224`; with the flag 1, `E'Y = Y / 255` and `E'P = (C − 128) / 255`. The
//!   flag's absence means 0.
//! - **Matrix** (Table E-5): for every code that names a pair `K_R`, `K_B`,
//!   `E'R = E'Y + 2 (1 − K_R) E'PR`, `E'B = E'Y + 2 (1 − K_B) E'PB` and
//!   `E'G = (E'Y − K_R E'R − K_B E'B) / (1 − K_R − K_B)`. Codes 1 (`K_R` 0.2126, `K_B` 0.0722),
//!   4 (0.30, 0.11), 5 and 6 (0.299, 0.114), 7 (0.212, 0.087) and 9 (0.2627, 0.0593) are drawn.
//! - **Siting** (Figure E-1): `chroma_sample_loc_type` 0 to 5 places each chroma sample of a 4:2:0
//!   picture at one of six positions relative to the top-left luma sample of its two-by-two block,
//!   horizontally at 0 or ½ a luma sample and vertically at 0, ½ or 1. Its absence means 0.
//!
//! # What is a choice, and said to be one
//!
//! - **An unspecified matrix** — code 2, or a reserved code (3, 15 and above) — is a stream that
//!   states no matrix, and the standard then derives none. The choice made here is Rec. ITU-R BT.601's
//!   pair (0.299, 0.114) for a picture at most 576 lines tall and Rec. ITU-R BT.709's (0.2126,
//!   0.0722) for a taller one: the television standards those two picture sizes were produced under.
//!   [`Conversion::matrix_stated`] says which pictures took it (ADR 3529).
//! - **The filter between chroma samples** is not the standard's either: H.264 places the samples
//!   and leaves display upsampling to the display. Each output pixel takes the two-by-two linear
//!   interpolation of the four chroma samples nearest its own position, the edges held, with
//!   weights in quarters so that the interpolation is exact in integers on both backends.
//! - **Primaries and transfer** are carried and not applied: the converted `R'G'B'` is handed to
//!   the window as its own RGB, which is the same thing this program does with a page's
//!   `DeviceRGB`. Converting a BT.2020 stream onto an sRGB display is later work (ADR 3529).
//!
//! # What is refused, by name
//!
//! Code 0 (identity), which Annex E allows only for 4:4:4 and the decoder never yields; and codes 8
//! (`YCgCo`), 10 (BT.2020 constant luminance), 11 (`Y'D'zD'x`), 12 and 13 (chromaticity-derived)
//! and 14 (`ICtCp`), whose formulas are not the `K_R`/`K_B` pair. A siting code above 5 names no position.

/// What a stream says of its own signal: the VUI's codes, with Annex E's meaning of an absent one
/// already applied by the reader of the stream (`video_full_range_flag` 0, `colour_primaries`,
/// `transfer_characteristics` and `matrix_coefficients` 2, `chroma_sample_loc_type` 0).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Signal {
    /// `video_full_range_flag`.
    pub full_range: bool,
    /// `colour_primaries` (Table E-3), carried and not applied.
    pub colour_primaries: u8,
    /// `transfer_characteristics` (Table E-4), carried and not applied.
    pub transfer_characteristics: u8,
    /// `matrix_coefficients` (Table E-5).
    pub matrix_coefficients: u8,
    /// `chroma_sample_loc_type_top_field` (Figure E-1); a progressive picture has only this one.
    pub chroma_sample_loc_type: u8,
}

impl Default for Signal {
    /// A stream with no VUI, read by Annex E's inferences.
    fn default() -> Self {
        Self {
            full_range: false,
            colour_primaries: 2,
            transfer_characteristics: 2,
            matrix_coefficients: 2,
            chroma_sample_loc_type: 0,
        }
    }
}

/// One picture's three planes, 8-bit 4:2:0, each tight: luma `width × height`, each chroma plane
/// `⌈width / 2⌉ × ⌈height / 2⌉`.
#[derive(Debug, Clone, Copy)]
pub struct Planes<'a> {
    /// Luma width in samples.
    pub width: u32,
    /// Luma height in samples.
    pub height: u32,
    /// The luma plane.
    pub y: &'a [u8],
    /// The blue-difference plane.
    pub cb: &'a [u8],
    /// The red-difference plane.
    pub cr: &'a [u8],
}

impl Planes<'_> {
    /// The chroma planes' width and height.
    #[must_use]
    pub fn chroma_size(&self) -> (u32, u32) {
        (self.width.div_ceil(2), self.height.div_ceil(2))
    }

    /// Whether every plane is exactly as long as the size says, and the size is not empty.
    #[must_use]
    pub fn are_consistent(&self) -> bool {
        let (chroma_width, chroma_height) = self.chroma_size();
        let luma = u64::from(self.width).saturating_mul(u64::from(self.height));
        let chroma = u64::from(chroma_width).saturating_mul(u64::from(chroma_height));
        luma > 0
            && u64::try_from(self.y.len()).is_ok_and(|length| length == luma)
            && u64::try_from(self.cb.len()).is_ok_and(|length| length == chroma)
            && u64::try_from(self.cr.len()).is_ok_and(|length| length == chroma)
    }
}

/// Why a picture's signal has no conversion here.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum ConversionRefused {
    /// A `matrix_coefficients` code whose formula is not the `K_R`/`K_B` pair, named.
    #[error("matrix_coefficients {code} ({name}), which this program does not convert")]
    Matrix {
        /// The code.
        code: u8,
        /// Table E-5's name for it.
        name: &'static str,
    },
    /// A `chroma_sample_loc_type` past Figure E-1's six.
    #[error("chroma_sample_loc_type {0}, past the six Figure E-1 places")]
    Siting(u8),
    /// Planes whose lengths are not a 4:2:0 picture of the size they came with.
    #[error("planes that are not a 4:2:0 picture of {width} by {height}")]
    Planes {
        /// The stated luma width.
        width: u32,
        /// The stated luma height.
        height: u32,
    },
}

/// The conversion of one picture, resolved from its [`Signal`]: the numbers both backends compute
/// the one formula from.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Conversion {
    /// `K_R` of Table E-5's pair.
    pub kr: f64,
    /// `K_B` of the pair.
    pub kb: f64,
    /// `video_full_range_flag`.
    pub full_range: bool,
    /// The chroma samples' horizontal place, in half luma samples right of the block's left
    /// column: 0 or 1.
    pub site_x: u8,
    /// Their vertical place, in half luma samples below the block's top row: 0, 1 or 2.
    pub site_y: u8,
    /// Whether the stream stated the matrix; `false` where [`Conversion::for_signal`] chose one.
    pub matrix_stated: bool,
}

impl Conversion {
    /// The conversion `signal` states, for a picture `height` lines tall (which decides only the
    /// choice for an unspecified matrix).
    ///
    /// # Errors
    ///
    /// A matrix or siting this program does not convert, by name.
    pub fn for_signal(signal: Signal, height: u32) -> Result<Self, ConversionRefused> {
        let refused = |name| ConversionRefused::Matrix {
            code: signal.matrix_coefficients,
            name,
        };
        let (kr, kb, matrix_stated) = match signal.matrix_coefficients {
            1 => (0.2126, 0.0722, true),
            4 => (0.30, 0.11, true),
            5 | 6 => (0.299, 0.114, true),
            7 => (0.212, 0.087, true),
            9 => (0.2627, 0.0593, true),
            0 => return Err(refused("identity, for 4:4:4 only")),
            8 => return Err(refused("YCgCo")),
            10 => return Err(refused("BT.2020 constant luminance")),
            11 => return Err(refused("Y'D'zD'x")),
            12 => return Err(refused("chromaticity-derived non-constant luminance")),
            13 => return Err(refused("chromaticity-derived constant luminance")),
            14 => return Err(refused("ICtCp")),
            _ if height <= 576 => (0.299, 0.114, false),
            _ => (0.2126, 0.0722, false),
        };
        let (site_x, site_y) = match signal.chroma_sample_loc_type {
            0 => (0, 1),
            1 => (1, 1),
            2 => (0, 0),
            3 => (1, 0),
            4 => (0, 2),
            5 => (1, 2),
            other => return Err(ConversionRefused::Siting(other)),
        };
        Ok(Self {
            kr,
            kb,
            full_range: signal.full_range,
            site_x,
            site_y,
            matrix_stated,
        })
    }

    /// The formula as an affine map, in output levels: `[R, G, B]` before rounding is
    /// `luma · (Y − luma_offset) + cb_row · (C16b − 2048) + cr_row · (C16r − 2048)` for each
    /// component, where `C16` is the chroma sample interpolated in sixteenths (so 2048 is 128).
    ///
    /// This is the range's and the matrix's equations multiplied out, so that each backend
    /// computes one product per term from integers it holds exactly.
    #[must_use]
    pub fn affine(&self) -> Affine {
        let (luma_offset, luma_span, chroma_span) = if self.full_range {
            (0.0, 255.0, 255.0)
        } else {
            (16.0, 219.0, 224.0)
        };
        let kg = 1.0 - self.kr - self.kb;
        // E'P = C16 / (16 · chroma_span); each output is 255 times its E'.
        let chroma = 255.0 / (16.0 * chroma_span);
        let red_from_cr = 2.0 * (1.0 - self.kr);
        let blue_from_cb = 2.0 * (1.0 - self.kb);
        Affine {
            luma: 255.0 / luma_span,
            luma_offset,
            cb: [
                0.0,
                -self.kb * blue_from_cb / kg * chroma,
                blue_from_cb * chroma,
            ],
            cr: [
                red_from_cr * chroma,
                -self.kr * red_from_cr / kg * chroma,
                0.0,
            ],
        }
    }
}

/// [`Conversion::affine`]'s numbers.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Affine {
    /// Output levels per luma level, the same for the three components.
    pub luma: f64,
    /// The luma level that stands for black: 16, or 0 at full range.
    pub luma_offset: f64,
    /// `[R, G, B]` levels per sixteenth of a blue-difference level.
    pub cb: [f64; 3],
    /// `[R, G, B]` levels per sixteenth of a red-difference level.
    pub cr: [f64; 3],
}

/// The chroma sample at luma position `(x, y)` in sixteenths of a level: the two-by-two linear
/// interpolation of the nearest chroma samples at the conversion's siting, weights in quarters,
/// indices held at the plane's edges. `plane` is `chroma_width × chroma_height`.
///
/// The position of luma column `x` in chroma columns is `(2x − site_x) / 4`, a whole number of
/// quarters, which is why the weights are exact; the same for rows.
#[must_use]
#[expect(
    clippy::arithmetic_side_effects,
    reason = "a sample is at most 255 and the weights' products at most 16, so the sum is at most \
              4080; an index is a row of a plane whose length is a `usize`"
)]
pub fn chroma_sixteenths(
    plane: &[u8],
    chroma_width: u32,
    chroma_height: u32,
    conversion: &Conversion,
    x: u32,
    y: u32,
) -> u32 {
    let (left, right, across) = taps(x, conversion.site_x, chroma_width);
    let (top, bottom, down) = taps(y, conversion.site_y, chroma_height);
    let width = chroma_width as usize;
    let at = |column: u32, row: u32| -> u32 {
        plane
            .get(row as usize * width + column as usize)
            .map_or(0, |&sample| u32::from(sample))
    };
    let upper = (4 - across) * at(left, top) + across * at(right, top);
    let lower = (4 - across) * at(left, bottom) + across * at(right, bottom);
    (4 - down) * upper + down * lower
}

/// The two chroma indices either side of luma index `index` at siting `site` (half samples), and
/// the far one's weight in quarters.
#[expect(
    clippy::arithmetic_side_effects,
    reason = "a `u32` index doubled and a sum of one fit an `i64`"
)]
fn taps(index: u32, site: u8, count: u32) -> (u32, u32, u32) {
    let quarters = i64::from(index) * 2 - i64::from(site);
    let near = quarters.div_euclid(4);
    let weight = u32::try_from(quarters.rem_euclid(4)).unwrap_or(0);
    let last = i64::from(count.saturating_sub(1));
    let hold = |index: i64| u32::try_from(index.clamp(0, last)).unwrap_or(0);
    (hold(near), hold(near + 1), weight)
}

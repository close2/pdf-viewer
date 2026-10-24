//! PIMA 7667:2001's e-sRGB and its Annex B e-sYCC, decoded and taken to the connection space.
//!
//! ITU-T T.801 Table M.25 names both as JPEG 2000 enumerated colour spaces — e-sRGB is 20 and
//! e-sYCC, the YCC encoding over it, is 24 — and hands each to PIMA 7667, which the project holds
//! (`doc/questions/A348`; `doc/third-party-data.md` has its provenance). ISO 32000-2 §7.4.9 asks a
//! processor to support the JPX baseline's enumerated spaces, and T.801 M.9.2.4 lists these two
//! among them. ADR 2292 is the reading this module implements.
//!
//! Both encode the colour of an image on the standard's reference display (its section 4.3):
//! ITU-R BT.709-3's primaries (Table 1), a white of CIE illuminant D65's chromaticity (4.3.1), and
//! — because section 4.4.1 encodes what a flareless display emits — zero tristimulus at zero. So
//! decoding is the standard's own inverse, step for step:
//!
//! - **e-sRGB**, section 4.5. Equation 5 takes an n-bit integer to a nonlinear component by
//!   removing equation 4's offset, `2^(n−2) + 2^(n−3)`, and equation 3's scale, `255 × 2^(n−9)`;
//!   equations 6a to 6c invert the sRGB nonlinearity, which equations 2a to 2c extend *oddly*
//!   about zero, so a negative component is a colour outside sRGB's gamut rather than a clip; and
//!   equation 7 is the matrix from those linear components to tristimulus values scaled so that
//!   the display white's `Y` is 1.
//! - **e-sYCC**, Annex B.2. Equations B7 and B8 take an m-bit luma and two m-bit chromas to
//!   fractions, the chromas about the code `2^(m−1)`, and equation B9's matrix takes those to
//!   e-sRGB's nonlinear components, which then follow section 4.5. B10 requantises them to an
//!   n-bit e-sRGB integer and B11 is equation 4 again; that step is not taken here, because the
//!   next step on this route is equation 5, which divides it out — what skipping it changes is
//!   half an n-bit level of rounding, not the colour.
//!
//! **Which precisions are e-sRGB.** Section 4.1 defines three levels and names them e-sRGB10,
//! e-sRGB12 and e-sRGB16, and equations 3 to 5 are written for those `n`. A codestream stating
//! e-sRGB at any other depth states no encoding the standard defines, so [`ExtendedSrgb::new`]
//! answers `None` for it and §7.4.9's fallback draws it. Annex B places no such limit on e-sYCC's
//! `m`, so every depth from 1 to 16 bits — the most a decoded raster carries — is drawn.
//!
//! **The connection space** is the ICC's D50, reached as every other CIE-based colour in this
//! crate is: Bradford from the space's own white ([`REFERENCE_WHITE`]) onto D50. PIMA 7667's
//! informative Annex E.2 recommends the same linear Bradford adaptation for an e-sRGB input
//! profile and prints the composed matrix as its equation E3;
//! `the_composed_matrix_is_annex_e_s_e3` holds this module's composition to it.

use super::ColourSpace;
use super::cie::{D50, adapt, transform, xyz_to_lab};

/// PIMA 7667 section 4.3.1's reference display white, `X_W = 76.037`, `Y_W = 80.000` and
/// `Z_W = 87.125` in candelas per square metre, scaled by section 4.4.1's division by 80 so that
/// `Y` is 1.
const REFERENCE_WHITE: [f32; 3] = [76.037 / 80.0, 1.0, 87.125 / 80.0];

/// PIMA 7667 equation 7, linear e-sRGB to the image tristimulus values `X_N`, `Y_N`, `Z_N`,
/// row-major — IEC 61966-2-1's sRGB matrix, as the section says.
#[rustfmt::skip]
const LINEAR_TO_XYZ: [f32; 9] = [
    0.4124, 0.3576, 0.1805,
    0.2126, 0.7152, 0.0722,
    0.0193, 0.1192, 0.9505,
];

/// PIMA 7667 equation B9, e-sYCC's `Y′`, `Cb′`, `Cr′` to e-sRGB's nonlinear `R′`, `G′`, `B′`,
/// row-major and as printed.
///
/// The printed matrix is the one Annex B.2 says the transformation shall use, so it is carried as
/// printed rather than recomputed from equation B3 or inverted from B4, which differ from it in the
/// fourth place (the `0.0003` in the last row is B4's rounding showing through).
#[rustfmt::skip]
const YCC_TO_NONLINEAR: [f32; 9] = [
    1.0000,  0.0000,  2.804,
    1.0000, -0.6882, -1.4282,
    1.0000,  3.5440,  0.0003,
];

/// The depths PIMA 7667 section 4.1 defines an e-sRGB encoding at: e-sRGB10, e-sRGB12, e-sRGB16.
const E_SRGB_PRECISIONS: [u8; 3] = [10, 12, 16];

/// The deepest sample a decoded raster carries, and so the deepest e-sYCC this module reads.
const DEEPEST: u8 = 16;

/// Which of PIMA 7667's two encodings a codestream's three components are in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExtendedEncoding {
    /// e-sRGB, T.801 Table M.25's 20: three nonlinear RGB components, PIMA 7667 section 4.
    Rgb,
    /// e-sYCC, T.801 Table M.25's 24: luma and two chromas over e-sRGB, PIMA 7667 Annex B.
    Ycc,
}

/// One of PIMA 7667's encodings at one depth: what turns a pixel's three integers into a colour.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExtendedSrgb {
    /// Which encoding.
    encoding: ExtendedEncoding,
    /// The bits of each component: e-sRGB's `n` or e-sYCC's `m`.
    bits: u8,
}

impl ExtendedSrgb {
    /// The encoding at `bits` a component, or `None` where PIMA 7667 defines no such encoding:
    /// e-sRGB at a depth other than section 4.1's 10, 12 and 16 bits, or e-sYCC outside 1 to 16 bits.
    #[must_use]
    pub fn new(encoding: ExtendedEncoding, bits: u8) -> Option<Self> {
        let defined = match encoding {
            ExtendedEncoding::Rgb => E_SRGB_PRECISIONS.contains(&bits),
            ExtendedEncoding::Ycc => (1..=DEEPEST).contains(&bits),
        };
        defined.then_some(Self { encoding, bits })
    }

    /// Which encoding this is.
    #[must_use]
    pub fn encoding(self) -> ExtendedEncoding {
        self.encoding
    }

    /// The components a pixel's three integers state: e-sRGB's nonlinear `R′`, `G′`, `B′` by
    /// equation 5, or e-sYCC's `Y′`, `Cb′`, `Cr′` by equations B7 and B8.
    ///
    /// These are the values a colour component of this space *is* — black at zero in both, and
    /// e-sYCC's chromas centred on zero — so they are the ones §11.6.5.2's matte and Table 87's
    /// premultiplication are undone in before [`Self::lab_d50`] is asked.
    #[must_use]
    pub fn components(self, codes: [u32; 3]) -> [f32; 3] {
        let codes = codes.map(as_real);
        match self.encoding {
            ExtendedEncoding::Rgb => {
                let (offset, scale) = self.offset_and_scale();
                codes.map(|code| (code - offset) / scale)
            }
            ExtendedEncoding::Ycc => {
                let (top, middle) = self.top_and_middle();
                [
                    codes[0] / top,
                    (codes[1] - middle) / top,
                    (codes[2] - middle) / top,
                ]
            }
        }
    }

    /// The least and greatest value component `component` takes: what [`Self::components`]
    /// makes of the integers 0 and `2^bits − 1`.
    #[must_use]
    pub fn component_range(self, component: usize) -> (f32, f32) {
        let highest = 1u32
            .checked_shl(u32::from(self.bits))
            .unwrap_or(u32::MAX)
            .saturating_sub(1);
        let least = self.components([0; 3]);
        let greatest = self.components([highest; 3]);
        let at = component.min(2);
        (
            least.get(at).copied().unwrap_or(0.0),
            greatest.get(at).copied().unwrap_or(1.0),
        )
    }

    /// The linear e-sRGB components a colour states: equation B9 first for e-sYCC, then
    /// equations 6a to 6c.
    #[must_use]
    pub fn linear(self, components: [f32; 3]) -> [f32; 3] {
        let nonlinear = match self.encoding {
            ExtendedEncoding::Rgb => components,
            ExtendedEncoding::Ycc => transform(&YCC_TO_NONLINEAR, components),
        };
        nonlinear.map(linearised)
    }

    /// The image tristimulus values a colour states, relative to section 4.3.1's reference white: equation 7
    /// applied to [`Self::linear`].
    #[must_use]
    pub fn xyz(self, components: [f32; 3]) -> [f32; 3] {
        transform(&LINEAR_TO_XYZ, self.linear(components))
    }

    /// The colour's XYZ in the connection space: [`Self::xyz`] adapted from the reference white
    /// onto D50 by the Bradford route every CIE-based space in this crate takes.
    #[must_use]
    pub fn xyz_d50(self, components: [f32; 3]) -> [f32; 3] {
        adapt(self.xyz(components), REFERENCE_WHITE, D50)
    }

    /// The colour as the connection space's CIE L\*a\*b\*, which [`Self::connection_space`] paints.
    ///
    /// The colour is handed on as a §8.6.5.4 `Lab` under D50 rather than as an RGB, because a
    /// `Lab` reaches every destination this crate converts to — the screen, an output intent's
    /// press, a blending colour space — by the CIE route, and nothing on it clips a colour
    /// outside sRGB's gamut before the destination does. Section 4.3.4 says an e-sRGB colour shall
    /// have no luminance above the display white's or below zero, which is `Lab`'s lightness
    /// from 0 to 100, so every colour the standard admits passes unchanged; one that breaks the
    /// bound has its lightness held at it by §8.6.5.4's range, a choice ADR 2292 records.
    #[must_use]
    pub fn lab_d50(self, components: [f32; 3]) -> [f32; 3] {
        xyz_to_lab(self.xyz_d50(components))
    }

    /// The space [`Self::lab_d50`]'s values are painted in: §8.6.5.4's `Lab` under D50, with
    /// `a*` and `b*` unbounded so that no extended colour is pulled into a range it was never in.
    #[must_use]
    pub fn connection_space() -> ColourSpace {
        ColourSpace::Lab {
            white: D50,
            black: [0.0; 3],
            range: [f32::MIN, f32::MAX, f32::MIN, f32::MAX],
        }
    }

    /// Equation 4's offset and equation 3's scale for e-sRGB's `n`.
    fn offset_and_scale(self) -> (f32, f32) {
        let power = |exponent: i32| 2f32.powi(exponent);
        let bits = i32::from(self.bits);
        (
            power(bits.saturating_sub(2)) + power(bits.saturating_sub(3)),
            255.0 * power(bits.saturating_sub(9)),
        )
    }

    /// Equation B7's divisor `2^m − 1` and B8's chroma centre `2^(m−1)` for e-sYCC's `m`.
    fn top_and_middle(self) -> (f32, f32) {
        let bits = i32::from(self.bits);
        (2f32.powi(bits) - 1.0, 2f32.powi(bits.saturating_sub(1)))
    }
}

/// Equations 6a to 6c: one nonlinear e-sRGB component to its linear value.
///
/// The middle band, 6b, is closed at both ends — `−0.04045 ≤ R′ ≤ 0.04045` — and the outer two
/// are its complement, so the comparisons are strict.
fn linearised(nonlinear: f32) -> f32 {
    const KNEE: f32 = 0.040_45;
    if nonlinear > KNEE {
        ((nonlinear + 0.055) / 1.055).powf(2.4)
    } else if nonlinear < -KNEE {
        -((-nonlinear + 0.055) / 1.055).powf(2.4)
    } else {
        nonlinear / 12.92
    }
}

/// An integer of at most sixteen bits as a real.
fn as_real(code: u32) -> f32 {
    #[expect(
        clippy::cast_precision_loss,
        reason = "a decoded sample is at most 65535, which every f32 represents exactly"
    )]
    let real = code as f32;
    real
}

#[cfg(test)]
#[expect(
    clippy::indexing_slicing,
    reason = "test code: fixed three-element arrays indexed by constants"
)]
mod tests {
    use super::{
        D50, ExtendedEncoding, ExtendedSrgb, LINEAR_TO_XYZ, REFERENCE_WHITE, adapt, transform,
    };

    fn e_srgb(bits: u8) -> ExtendedSrgb {
        ExtendedSrgb::new(ExtendedEncoding::Rgb, bits)
            .unwrap_or_else(|| panic!("PIMA 7667 section 4.1 defines e-sRGB at {bits} bits"))
    }

    fn e_sycc(bits: u8) -> ExtendedSrgb {
        ExtendedSrgb::new(ExtendedEncoding::Ycc, bits)
            .unwrap_or_else(|| panic!("Annex B places no limit on {bits} bits"))
    }

    /// Section 4.1's three precisions are e-sRGB and no other depth is; e-sYCC takes any.
    #[test]
    fn only_the_three_defined_precisions_are_e_srgb() {
        for bits in 1..=16 {
            assert_eq!(
                ExtendedSrgb::new(ExtendedEncoding::Rgb, bits).is_some(),
                [10, 12, 16].contains(&bits),
                "{bits} bits"
            );
            assert!(ExtendedSrgb::new(ExtendedEncoding::Ycc, bits).is_some());
        }
        assert!(ExtendedSrgb::new(ExtendedEncoding::Ycc, 0).is_none());
        assert!(ExtendedSrgb::new(ExtendedEncoding::Ycc, 17).is_none());
    }

    /// Section 4.4.3's NOTE tabulates the neutral patches `Y` = k/99 at each precision; decoding
    /// each tabulated code gives a `Y` nearer the patch than either neighbouring code's.
    #[test]
    fn the_neutral_patch_table_decodes_to_its_own_luminances() {
        const TABLE: [(u32, [u32; 3]); 9] = [
            (0, [384, 1536, 24576]),
            (1, [435, 1741, 27856]),
            (3, [481, 1925, 30803]),
            (7, [534, 2137, 34199]),
            (14, [594, 2376, 38023]),
            (29, [679, 2714, 43426]),
            (59, [790, 3158, 50536]),
            (79, [846, 3383, 54126]),
            (99, [894, 3576, 57216]),
        ];
        for (patch, codes) in TABLE {
            let wanted = f64::from(patch) / 99.0;
            for (bits, code) in [10, 12, 16].into_iter().zip(codes) {
                let space = e_srgb(bits);
                let luminance = |code: u32| f64::from(space.xyz(space.components([code; 3]))[1]);
                let miss = |code: u32| (luminance(code) - wanted).abs();
                assert!(
                    miss(code) <= miss(code - 1).min(miss(code + 1)) + 1e-6,
                    "Y = {patch}/99 at {bits} bits: code {code} decodes to {}",
                    luminance(code)
                );
            }
        }
    }

    /// Annex A's A1 places an 8-bit sRGB value `v` at `v × 2^(n−9) + offset`, exactly; equation 5
    /// therefore takes that code back to `v / 255`, the sRGB component it was.
    #[test]
    fn an_srgb_value_placed_by_annex_a_decodes_to_itself() {
        for bits in [10, 12, 16] {
            let space = e_srgb(bits);
            let step = 1u32 << (bits - 9);
            let offset = (1u32 << (bits - 2)) + (1u32 << (bits - 3));
            for value in [0u16, 1, 50, 100, 200, 255] {
                let [component, ..] = space.components([u32::from(value) * step + offset; 3]);
                assert!(
                    (component - f32::from(value) / 255.0).abs() < 1e-6,
                    "{value} at {bits} bits gave {component}"
                );
            }
        }
    }

    /// An extended component, worked by hand from equations 5, 6a and 7: the 16-bit code 20000 is
    /// `R′` = (20000 − 24576) / 32640 = −0.140196, below 6a's knee, so `R` = −((0.140196 +
    /// 0.055) / 1.055)^2.4 = −0.017431; beside `G′` = `B′` = 1 (code 57216, linear 1), equation 7
    /// gives `X` = 0.4124 × −0.017431 + 0.3576 + 0.1805 = 0.530911, `Y` = 0.783694 and `Z` =
    /// 1.069364 — a colour sRGB cannot show, carried rather than clipped.
    #[test]
    fn an_extended_component_keeps_its_sign() {
        let space = e_srgb(16);
        let components = space.components([20000, 57216, 57216]);
        assert!((components[0] + 0.140_196).abs() < 1e-6, "{components:?}");
        let linear = space.linear(components);
        assert!((linear[0] + 0.017_431).abs() < 2e-6, "{linear:?}");
        let xyz = space.xyz(components);
        for (got, want) in xyz.iter().zip([0.530_911, 0.783_694, 1.069_364]) {
            assert!((got - want).abs() < 2e-6, "{xyz:?}");
        }
    }

    /// Equation 7's matrix takes equal linear components to the reference white, section 4.4.2's
    /// NOTE, to the four places it is printed at.
    #[test]
    fn equal_components_are_the_reference_white() {
        let white = transform(&LINEAR_TO_XYZ, [1.0; 3]);
        for (got, want) in white.iter().zip(REFERENCE_WHITE) {
            assert!((got - want).abs() < 1e-4, "{white:?}");
        }
    }

    /// Annex E.2's equation E3 is equation 7 composed with a linear Bradford adaptation onto D50,
    /// printed to three places; this module's composition agrees with it to a unit in the third.
    #[test]
    fn the_composed_matrix_is_annex_e_s_e3() {
        #[rustfmt::skip]
        const E3: [[f32; 3]; 3] = [
            [0.436, 0.385, 0.143],
            [0.223, 0.717, 0.061],
            [0.014, 0.097, 0.714],
        ];
        for (column, unit) in [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]]
            .into_iter()
            .enumerate()
        {
            let xyz = adapt(transform(&LINEAR_TO_XYZ, unit), REFERENCE_WHITE, D50);
            for (row, got) in xyz.iter().enumerate() {
                assert!(
                    (got - E3[row][column]).abs() <= 1e-3,
                    "E3 row {row} column {column}: {got}"
                );
            }
        }
    }

    /// Equation B9 worked by hand on the 8-bit e-sYCC pixel (150, 150, 150): B7 and B8 give
    /// `Y′` = 150/255 = 0.588235 and `Cb′` = `Cr′` = 22/255 = 0.086275, and B9 gives `R′` =
    /// 0.588235 + 2.804 × 0.086275 = 0.830149, `G′` = 0.588235 − 0.6882 × 0.086275 − 1.4282 ×
    /// 0.086275 = 0.405644 and `B′` = 0.588235 + 3.544 × 0.086275 + 0.0003 × 0.086275 = 0.894018.
    #[test]
    fn e_sycc_reaches_e_srgb_by_b9() {
        let space = e_sycc(8);
        let components = space.components([150, 150, 150]);
        let nonlinear = transform(&super::YCC_TO_NONLINEAR, components);
        for (got, want) in nonlinear.iter().zip([0.830_149, 0.405_644, 0.894_018]) {
            assert!((got - want).abs() < 2e-6, "{nonlinear:?}");
        }
    }

    /// A colour with no chroma is neutral, and at the luma of the reference white is that white.
    #[test]
    fn e_sycc_without_chroma_is_neutral() {
        let space = e_sycc(8);
        let white = space.xyz(space.components([255, 128, 128]));
        for (got, want) in white.iter().zip(REFERENCE_WHITE) {
            assert!((got - want).abs() < 1e-3, "{white:?}");
        }
    }

    /// The D50 reading of the reference white is D50, which is what makes a neutral e-sRGB a
    /// neutral `Lab` (a\* and b\* of zero).
    #[test]
    fn the_reference_white_is_d50_in_the_connection_space() {
        let space = e_srgb(16);
        let lab = space.lab_d50(space.components([57216; 3]));
        assert!((lab[0] - 100.0).abs() < 0.05, "{lab:?}");
        assert!(lab[1].abs() < 0.05 && lab[2].abs() < 0.05, "{lab:?}");
    }
}

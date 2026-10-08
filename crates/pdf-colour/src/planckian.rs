//! The white point of a Planckian radiator — the illuminant a colour temperature names.
//!
//! A JPEG 2000 CIE Lab box may state its illuminant as a colour temperature alone (ITU-T T.801
//! M.11.7.4.1, after ITU-T T.4 section E.6.7), and the CIE's International Lighting Vocabulary,
//! CIE S 017:2020 entry 17-23-067, defines a colour temperature as that of the Planckian radiator
//! whose radiation has the stimulus's chromaticity. So the temperature names one spectrum, and the
//! white point is that spectrum's tristimulus sum against the CIE 1931 2° colour-matching functions,
//! scaled to `Y` = 1 — which is how every other white point in `data/cie/PROVENANCE.md` is made.
//! A *correlated* colour temperature, the vocabulary's word for a source off the Planckian locus
//! (a daylight phase), is a different statement that neither T.4 nor T.801 makes (ADR 1713).
//!
//! The spectrum is Planck's law in the relative form `λ⁻⁵ / (exp(c₂ / λT) − 1)`, its constant the
//! SI's: `c₂ = hc / k` with the three exact values of the 2019 SI. CIE standard illuminant A is a
//! Planckian radiator, by the formula its dataset record names (ISO/CIE 11664-2 equation 1, not
//! held), and `the_radiator_at_illuminant_as_temperature_is_illuminant_a` holds this sum at 2856 K
//! to the CIE's own table.
//!
//! The functions are `data/cie/CIE_xyz_1931_2deg.csv`, compiled in and read into a table on the
//! first temperature asked for, never at startup — no document but one stating such a box asks.

use std::sync::OnceLock;

/// The CIE 1931 2° colour-matching functions at 1 nm from 360 to 830 nm, as the CIE publishes them
/// (CC BY-SA 4.0; `data/cie/PROVENANCE.md`).
const OBSERVER: &str = include_str!("../../../data/cie/CIE_xyz_1931_2deg.csv");

/// The second radiation constant, `hc / k`, in metre kelvins, from the 2019 SI's exact Planck
/// constant, speed of light and Boltzmann constant.
const SECOND_RADIATION_CONSTANT: f64 = 6.626_070_15e-34 * 299_792_458.0 / 1.380_649e-23;

/// The colour-matching functions as `(wavelength in metres, [x̄, ȳ, z̄])`, read once.
fn observer() -> &'static [(f64, [f64; 3])] {
    static TABLE: OnceLock<Vec<(f64, [f64; 3])>> = OnceLock::new();
    TABLE.get_or_init(|| {
        OBSERVER
            .lines()
            .filter_map(|line| {
                let mut fields = line.trim().split(',').map(str::parse::<f64>);
                let nanometres = fields.next()?.ok()?;
                let x = fields.next()?.ok()?;
                let y = fields.next()?.ok()?;
                let z = fields.next()?.ok()?;
                Some((nanometres * 1e-9, [x, y, z]))
            })
            .collect()
    })
}

/// The white point, as CIE 1931 XYZ with `Y` at 1.0, of a Planckian radiator at `kelvin`, or
/// `None` where the temperature is so low that no wavelength the observer sees carries energy.
#[must_use]
pub fn white_point(kelvin: f64) -> Option<[f32; 3]> {
    if !(kelvin.is_finite() && kelvin > 0.0) {
        return None;
    }
    let mut sum = [0.0f64; 3];
    for (wavelength, functions) in observer() {
        let exponent = SECOND_RADIATION_CONSTANT / (wavelength * kelvin);
        let radiance = wavelength.powi(-5) / exponent.exp_m1();
        if !radiance.is_finite() {
            continue;
        }
        for (axis, function) in functions.iter().enumerate() {
            sum[axis] += radiance * function;
        }
    }
    if !(sum[1].is_finite() && sum[1] > 0.0) {
        return None;
    }
    #[expect(
        clippy::cast_possible_truncation,
        reason = "a ratio near 1, which f32 holds to far better than a level of 255"
    )]
    let white = [(sum[0] / sum[1]) as f32, 1.0, (sum[2] / sum[1]) as f32];
    Some(white)
}

#[cfg(test)]
mod tests {
    use super::white_point;

    /// The observer table is the whole file: 471 wavelengths, 360 to 830 nm.
    #[test]
    fn the_observer_is_every_wavelength_the_file_states() {
        let table = super::observer();
        assert_eq!(table.len(), 471);
        assert!((table[0].0 - 360e-9).abs() < 1e-15);
        assert!((table[470].0 - 830e-9).abs() < 1e-15);
    }

    /// ISO/CIE 11664-2's illuminant A is a Planckian radiator at 2856 K on the present
    /// temperature scale, so the radiator at that temperature is illuminant A's white point,
    /// `[1.09850 1 0.35585]` by `pdf_model::jpeg2000::Illuminant::A`'s sum of the CIE's own table —
    /// to 2 × 10⁻⁴. The standard's own formula for A is not held, so the residue is bounded here
    /// rather than explained.
    #[test]
    fn the_radiator_at_illuminant_as_temperature_is_illuminant_a() {
        let white = white_point(2856.0).expect("a temperature");
        for (axis, want) in [1.098_5, 1.0, 0.355_85].into_iter().enumerate() {
            assert!(
                (white[axis] - want).abs() < 2e-4,
                "axis {axis}: {white:?} against illuminant A's"
            );
        }
    }

    /// T.801 M.11.7.4.1's own example temperature, 7500 K, is bluer than illuminant A — more `Z`
    /// than `X` — and a temperature with no energy in the visible band names no white.
    #[test]
    fn a_hotter_radiator_is_bluer_and_a_cold_one_is_no_white() {
        let white = white_point(7500.0).expect("a temperature");
        assert!(white[2] > white[0], "{white:?}");
        assert_eq!(white_point(0.0), None);
        assert_eq!(white_point(1.0), None);
    }
}

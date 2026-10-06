//! The figure of the earth a coordinate system names, and the quantities every projection's
//! formulas derive from it — Guidance Note 7-2 section 1.1.

use super::Refusal;

/// A reference ellipsoid: its semi-major axis and its inverse flattening.
///
/// Guidance Note 7-2 section 1.1 calls these the *primary* parameters and derives the rest —
/// flattening, eccentricity, the two radii of curvature — from them, which is what the methods
/// here do. An inverse flattening of zero is a sphere: ISO 19162 section 8.2.1 writes a sphere
/// that way, and so does the one census document stating ESRI's auxiliary sphere (ADR 1586).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Ellipsoid {
    /// The semi-major axis `a`, in metres.
    pub semi_major_axis: f64,
    /// `1/f`, or `0.0` for a sphere.
    pub inverse_flattening: f64,
}

impl Ellipsoid {
    /// An ellipsoid from its two primary parameters, refused where they describe no ellipsoid:
    /// an axis that is not a positive length, or a flattening of one or more (an inverse
    /// flattening at or below one, other than the sphere's zero).
    ///
    /// # Errors
    ///
    /// [`Refusal::Ellipsoid`] with the two numbers as stated.
    pub fn new(semi_major_axis: f64, inverse_flattening: f64) -> Result<Self, Refusal> {
        let axis_ok = semi_major_axis.is_finite() && semi_major_axis > 0.0;
        let flattening_ok = inverse_flattening == 0.0
            || (inverse_flattening.is_finite() && inverse_flattening > 1.0);
        if axis_ok && flattening_ok {
            Ok(Self {
                semi_major_axis,
                inverse_flattening,
            })
        } else {
            Err(Refusal::Ellipsoid {
                semi_major_axis,
                inverse_flattening,
            })
        }
    }

    /// The flattening `f`, zero for a sphere.
    #[must_use]
    pub fn flattening(&self) -> f64 {
        if self.inverse_flattening == 0.0 {
            0.0
        } else {
            1.0 / self.inverse_flattening
        }
    }

    /// The square of the eccentricity, `e² = 2f − f²`.
    #[must_use]
    pub fn eccentricity_squared(&self) -> f64 {
        let f = self.flattening();
        2.0 * f - f * f
    }

    /// The eccentricity `e`.
    #[must_use]
    pub fn eccentricity(&self) -> f64 {
        self.eccentricity_squared().sqrt()
    }

    /// The radius of curvature in the meridian, `ρ = a(1 − e²)/(1 − e² sin²φ)^(3/2)`.
    #[must_use]
    pub fn meridian_radius(&self, latitude: f64) -> f64 {
        let e2 = self.eccentricity_squared();
        let s = latitude.sin();
        self.semi_major_axis * (1.0 - e2) / (1.0 - e2 * s * s).powf(1.5)
    }

    /// The radius of curvature in the prime vertical, `ν = a/(1 − e² sin²φ)^(1/2)`.
    #[must_use]
    pub fn prime_vertical_radius(&self, latitude: f64) -> f64 {
        let e2 = self.eccentricity_squared();
        let s = latitude.sin();
        self.semi_major_axis / (1.0 - e2 * s * s).sqrt()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_derived_parameters_are_the_guidance_notes() {
        // Guidance Note 7-2 section 3.2.3.1's example: Airy 1830, a = 6377563.396 m,
        // 1/f = 299.32496, whence e² = 0.00667054.
        let airy = Ellipsoid::new(6_377_563.396, 299.324_96).expect("Airy 1830");
        assert!((airy.eccentricity_squared() - 0.006_670_54).abs() < 5e-9);
        // Section 3.3.1.1's: Bessel 1841 at 52°09′22.178″N, where ρO = 6374588.71 and νO = 6390710.613.
        let bessel = Ellipsoid::new(6_377_397.155, 299.152_81).expect("Bessel 1841");
        let origin = 0.910_296_727;
        assert!((bessel.meridian_radius(origin) - 6_374_588.71).abs() < 0.01);
        assert!((bessel.prime_vertical_radius(origin) - 6_390_710.613).abs() < 0.001);
    }

    #[test]
    fn a_sphere_has_no_eccentricity_and_a_flat_disc_is_refused() {
        let sphere = Ellipsoid::new(6_378_137.0, 0.0).expect("a sphere");
        assert!(sphere.eccentricity().abs() < f64::EPSILON);
        assert!(Ellipsoid::new(6_378_137.0, 1.0).is_err());
        assert!(Ellipsoid::new(-1.0, 298.0).is_err());
        assert!(Ellipsoid::new(f64::NAN, 298.0).is_err());
    }
}

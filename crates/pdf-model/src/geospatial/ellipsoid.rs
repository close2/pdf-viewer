//! The figure of the earth a coordinate system names, and the quantities every projection's
//! formulas derive from it — Guidance Note 7-2 section 1.1 — and the conversion between a
//! position on it and the earth-centred Cartesian coordinates a datum transformation works in,
//! section 4.1.1.

use super::Refusal;
use super::projection::LatLon;

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

    /// The geocentric Cartesian coordinates `X`, `Y`, `Z`, in metres, of a latitude, a longitude
    /// counted from Greenwich, both in radians, and a height above the ellipsoid — Guidance Note
    /// 7-2 section 4.1.1's forward conversion.
    #[must_use]
    pub fn geocentric(&self, position: LatLon, height: f64) -> [f64; 3] {
        let LatLon {
            latitude,
            longitude,
        } = position;
        let nu = self.prime_vertical_radius(latitude);
        let across = (nu + height) * latitude.cos();
        [
            across * longitude.cos(),
            across * longitude.sin(),
            ((1.0 - self.eccentricity_squared()) * nu + height) * latitude.sin(),
        ]
    }

    /// The latitude, the longitude counted from Greenwich and the height above the ellipsoid of
    /// geocentric Cartesian coordinates in metres — section 4.1.1's reverse conversion, by the
    /// formula the section gives for finding the latitude without iterating.
    #[must_use]
    #[expect(
        clippy::many_single_char_names,
        reason = "the formula is Guidance Note 7-2's, and its symbols are the names a reader checks \
                  it against"
    )]
    pub fn geodetic(&self, geocentric: [f64; 3]) -> (LatLon, f64) {
        let [x, y, z] = geocentric;
        let a = self.semi_major_axis;
        let e2 = self.eccentricity_squared();
        let epsilon = e2 / (1.0 - e2);
        let b = a * (1.0 - self.flattening());
        let p = x.hypot(y);
        let q = (z * a).atan2(p * b);
        let latitude = (z + epsilon * b * q.sin().powi(3)).atan2(p - e2 * a * q.cos().powi(3));
        let height = p / latitude.cos() - self.prime_vertical_radius(latitude);
        (
            LatLon {
                latitude,
                longitude: y.atan2(x),
            },
            height,
        )
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

    /// A latitude or longitude of degrees, minutes and seconds, in radians.
    fn dms(degrees: f64, minutes: f64, seconds: f64) -> f64 {
        (degrees + minutes / 60.0 + seconds / 3600.0).to_radians()
    }

    #[test]
    fn the_geocentric_conversion_reproduces_the_north_sea_example() {
        // Guidance Note 7-2 section 4.1.1's example on WGS 84: X = 3771793.968 m,
        // Y = 140253.342 m, Z = 5124304.349 m is 53°48′33.820″N 2°07′46.380″E at h = 73.0 m.
        let wgs84 = Ellipsoid::new(6_378_137.0, 298.257_223_563).expect("WGS 84");
        let (position, height) = wgs84.geodetic([3_771_793.968, 140_253.342, 5_124_304.349]);
        // Half the example's last printed digit: 0.0005″ of arc, and 0.05 m of height.
        let half = (0.0005f64 / 3600.0).to_radians();
        assert!((position.latitude - dms(53.0, 48.0, 33.820)).abs() < half);
        assert!((position.longitude - dms(2.0, 7.0, 46.380)).abs() < half);
        assert!((height - 73.0).abs() < 0.05);
        // The section's intermediate latitude, 0.939151101 rad.
        assert!((position.latitude - 0.939_151_101).abs() < 5e-10);
        // The forward conversion of the printed position comes back to the example's
        // coordinates within what 0.0005″ is on the ground, about 1.5 cm.
        let [x, y, z] = wgs84.geocentric(position, height);
        assert!((x - 3_771_793.968).abs() < 1e-6, "{x}");
        assert!((y - 140_253.342).abs() < 1e-6, "{y}");
        assert!((z - 5_124_304.349).abs() < 1e-6, "{z}");
        let printed = LatLon {
            latitude: dms(53.0, 48.0, 33.820),
            longitude: dms(2.0, 7.0, 46.380),
        };
        let [x, y, z] = wgs84.geocentric(printed, 73.0);
        assert!((x - 3_771_793.968).abs() < 0.03, "{x}");
        assert!((y - 140_253.342).abs() < 0.03, "{y}");
        assert!((z - 5_124_304.349).abs() < 0.03, "{z}");
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

//! The two azimuthal equidistant forms of Micronesia, Guidance Note 7-2 section 3.3.3.1's
//! Modified Azimuthal Equidistant (EPSG method 9832) and section 3.3.3.2's Guam Projection (9831).
//!
//! The first is the US National Geodetic Survey's oblique ellipsoidal form, which measures the
//! distance from the origin along a normal section rather than the geodesic ("for the distances
//! over which these projections are used (under 800km) this modification introduces no
//! significant error"); the second is a simplified form of the same over Guam alone, whose reverse
//! is three fixed steps of an iteration.

use super::cassini::{footpoint_latitude, meridian_distance};
use super::{LatLon, Projection};

/// Section 3.3.3.2's count of the reverse's steps: "The Guam projection uses three iterations,
/// which is satisfactory over the small area of application."
const GUAM_ITERATIONS: usize = 3;

impl Projection {
    /// Section 3.3.3.1's azimuth `α` and angular distance `s` of a point from the origin, `delta`
    /// being λ − λO.
    ///
    /// The section's distance `s` is the spherical sine rule's, `asin[sin(λ – λO) cos ψ / sin α]`,
    /// with its own case for `sin α = 0`, the origin's meridian: `asin(cos ϕO sin ψ – sin ϕO cos ψ)
    /// · SIGN(cos α)`. `α` is the `atan2` of `y = sin(λ – λO) cos ψ` and
    /// `x = cos ϕO sin ψ – sin ϕO cos ψ cos(λ – λO)` (the section's second argument times `cos ψ`),
    /// so `sin α = y / (x² + y²)^½` and the quotient is `(x² + y²)^½` exactly. That is evaluated
    /// here: on the meridian it is the section's case, `|x|`, and beside it the section's quotient
    /// is `0/0` in floating point — a longitude a nanodegree off the meridian moved a point 116 km
    /// south of the origin by half a kilometre (ADR 3885).
    fn normal_section(&self, latitude: f64, delta: f64) -> (f64, f64) {
        let e2 = self.ellipsoid.eccentricity_squared();
        let origin = Self::get(self.parameters.origin_latitude);
        let (sin_0, cos_0) = origin.sin_cos();
        let nu_0 = self.ellipsoid.prime_vertical_radius(origin);
        let nu = self.ellipsoid.prime_vertical_radius(latitude);
        let psi = ((1.0 - e2) * latitude.tan() + e2 * nu_0 * sin_0 / (nu * latitude.cos())).atan();
        let alpha = delta.sin().atan2(cos_0 * psi.tan() - sin_0 * delta.cos());
        let s = (delta.sin() * psi.cos())
            .hypot(cos_0 * psi.sin() - sin_0 * psi.cos() * delta.cos())
            .asin();
        (alpha, s)
    }

    /// Section 3.3.3.1's forward formulas, `delta` being λ − λO, less the false easting and
    /// northing.
    pub(super) fn modified_azimuthal_forward(&self, latitude: f64, delta: f64) -> (f64, f64) {
        let e2 = self.ellipsoid.eccentricity_squared();
        let e = self.ellipsoid.eccentricity();
        let origin = Self::get(self.parameters.origin_latitude);
        let (sin_0, cos_0) = origin.sin_cos();
        let nu_0 = self.ellipsoid.prime_vertical_radius(origin);
        let (alpha, s) = self.normal_section(latitude, delta);
        let root = (1.0 - e2).sqrt();
        let g = e * sin_0 / root;
        let h = e * cos_0 * alpha.cos() / root;
        let (h2, g2) = (h * h, g * g);
        let c = nu_0
            * s
            * ((1.0 - s.powi(2) * h2 * (1.0 - h2) / 6.0)
                + (s.powi(3) / 8.0) * g * h * (1.0 - 2.0 * h2)
                + (s.powi(4) / 120.0) * (h2 * (4.0 - 7.0 * h2) - 3.0 * g2 * (1.0 - 7.0 * h2))
                - (s.powi(5) / 48.0) * g * h);
        (c * alpha.sin(), c * alpha.cos())
    }

    /// Section 3.3.3.1's reverse formulas.
    ///
    /// The section's `ϕ = atan[(1 – e² K sin ϕO / sin Ψ′) tan Ψ′ / (1 – e²)]` is evaluated as
    /// `atan[(sin Ψ′ – e² K sin ϕO) / (cos Ψ′ (1 – e²))]`, the same quotient with `sin Ψ′`
    /// multiplied through, so a point whose `Ψ′` is the equator has no `0/0` to divide.
    pub(super) fn modified_azimuthal(&self, x: f64, y: f64) -> LatLon {
        let e2 = self.ellipsoid.eccentricity_squared();
        let origin = Self::get(self.parameters.origin_latitude);
        let (sin_0, cos_0) = origin.sin_cos();
        let nu_0 = self.ellipsoid.prime_vertical_radius(origin);
        let c = x.hypot(y);
        let alpha = x.atan2(y);
        let a = -e2 * cos_0.powi(2) * alpha.cos().powi(2) / (1.0 - e2);
        let b = 3.0 * e2 * (1.0 - a) * sin_0 * cos_0 * alpha.cos() / (1.0 - e2);
        let d = c / nu_0;
        let j = d - a * (1.0 + a) * d.powi(3) / 6.0 - b * (1.0 + 3.0 * a) * d.powi(4) / 24.0;
        let k = 1.0 - a * j * j / 2.0 - b * j.powi(3) / 6.0;
        let psi = (sin_0 * j.cos() + cos_0 * j.sin() * alpha.cos()).asin();
        LatLon {
            latitude: ((psi.sin() - e2 * k * sin_0) / (psi.cos() * (1.0 - e2))).atan(),
            longitude: Self::get(self.parameters.origin_longitude)
                + (alpha.sin() * j.sin() / psi.cos()).asin(),
        }
    }

    /// `(1 – e² sin² ϕ)^½`, which every formula of section 3.3.3.2 carries.
    fn guam_w(&self, latitude: f64) -> f64 {
        (1.0 - self.ellipsoid.eccentricity_squared() * latitude.sin().powi(2)).sqrt()
    }

    /// Section 3.3.3.2's forward formulas: `x = a (λ – λO) cos ϕ / (1 – e² sin² ϕ)^½`, and
    /// `N – FN = M – MO + x² tan ϕ (1 – e² sin² ϕ)^½ / 2a`.
    pub(super) fn guam_forward(&self, latitude: f64, delta: f64) -> (f64, f64) {
        let a = self.ellipsoid.semi_major_axis;
        let e2 = self.ellipsoid.eccentricity_squared();
        let w = self.guam_w(latitude);
        let x = a * delta * latitude.cos() / w;
        let m = meridian_distance(a, e2, latitude);
        let m_0 = meridian_distance(a, e2, Self::get(self.parameters.origin_latitude));
        (x, m - m_0 + x * x * latitude.tan() * w / (2.0 * a))
    }

    /// Section 3.3.3.2's reverse formulas: three times, the footpoint latitude of
    /// `M = MO + (N – FN) – (E – FE)² tan ϕ (1 – e² sin² ϕ)^½ / 2a`, `ϕ` being the latitude of
    /// natural origin the first time and the step before's after it; then
    /// `λ = λO + (E – FE) (1 – e² sin² ϕ)^½ / (a cos ϕ)`.
    pub(super) fn guam(&self, x: f64, y: f64) -> LatLon {
        let a = self.ellipsoid.semi_major_axis;
        let e2 = self.ellipsoid.eccentricity_squared();
        let origin = Self::get(self.parameters.origin_latitude);
        let m_0 = meridian_distance(a, e2, origin);
        let latitude = (0..GUAM_ITERATIONS).fold(origin, |latitude, _| {
            let m = m_0 + y - x * x * latitude.tan() * self.guam_w(latitude) / (2.0 * a);
            footpoint_latitude(a, e2, m)
        });
        LatLon {
            latitude,
            longitude: Self::get(self.parameters.origin_longitude)
                + x * self.guam_w(latitude) / (a * latitude.cos()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::{Method, Parameters};
    use super::*;
    use crate::geospatial::ellipsoid::Ellipsoid;

    /// Section 3.3.3.1's example, Guam 1963 / Yap Islands, on Clarke 1866, its origin
    /// 0.166621493 rad north.
    fn yap() -> Projection {
        let ellipsoid = Ellipsoid::new(6_378_206.4, 294.978_70).expect("Clarke 1866");
        let parameters = Parameters {
            origin_latitude: Some(0.166_621_493),
            origin_longitude: Some(2.411_499_514),
            ..Parameters::default()
        };
        Projection::new(Method::ModifiedAzimuthalEquidistant, ellipsoid, parameters).expect("it")
    }

    #[test]
    fn the_distance_is_the_sections_on_and_off_the_meridian() {
        let projection = yap();
        let e2 = projection.ellipsoid.eccentricity_squared();
        let origin: f64 = 0.166_621_493;
        let nu_0 = projection.ellipsoid.prime_vertical_radius(origin);
        let psi = |latitude: f64| {
            let nu = projection.ellipsoid.prime_vertical_radius(latitude);
            ((1.0 - e2) * latitude.tan() + e2 * nu_0 * origin.sin() / (nu * latitude.cos())).atan()
        };
        // The example's own `ψ`, `α` and `s`.
        let point = 0.167_490_973;
        assert!((psi(point) - 0.167_485_249).abs() < 1e-9);
        let (azimuth, distance) = projection.normal_section(point, 2.411_923_377 - 2.411_499_514);
        assert!(
            (azimuth - 0.450_640_866).abs() < 1e-5 && (distance - 0.000_959_566).abs() < 2e-9,
            "{azimuth}, {distance}"
        );
        // Off the meridian, where `sin α` is well away from zero, the section's quotient; and on
        // it, north and south of the origin, the section's case with `SIGN(cos α)`.
        for (latitude, delta) in [(point, 0.000_423_863), (0.20, 0.01), (0.12, -0.02)] {
            let p = psi(latitude);
            let alpha = f64::sin(delta).atan2(origin.cos() * p.tan() - origin.sin() * delta.cos());
            let printed = (delta.sin() * p.cos() / alpha.sin()).asin();
            let (azimuth, distance) = projection.normal_section(latitude, delta);
            assert!((azimuth - alpha).abs() < 1e-15, "{latitude}, {delta}");
            assert!((distance - printed).abs() < 1e-15, "{latitude}, {delta}");
        }
        for latitude in [0.20, 0.12] {
            let p = psi(latitude);
            let cos_alpha = (origin.cos() * p.tan() - origin.sin()).signum();
            let printed = (origin.cos() * p.sin() - origin.sin() * p.cos()).asin() * cos_alpha;
            let (azimuth, distance) = projection.normal_section(latitude, 0.0);
            assert!((azimuth.cos() - cos_alpha).abs() < 1e-15, "{latitude}");
            assert!((distance - printed).abs() < 1e-15, "{latitude}");
        }
    }
}

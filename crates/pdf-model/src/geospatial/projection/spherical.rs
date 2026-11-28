//! Lambert Azimuthal Equal Area (Spherical), EPSG method 1027, by the formulas Guidance Note 7-2
//! section 3.3.2.1 sends its reader to.
//!
//! The section prints none of its own: "See USGS Professional Paper 1395, "Map Projections - A
//! Working Manual" by John P. Snyder for formulas and example", and of the radius, "R is the
//! radius of the sphere [...] If the figure of the earth used is an ellipsoid rather than a sphere
//! then R should be calculated as the radius of the authalic sphere using the formula for RA". So
//! the formulas are Professional Paper 1395's for the sphere — equation 24-2's `k′` with 22-4 and
//! 22-5 forward, and 20-14 with 20-15's two-argument form back — and `R` is the ellipsoid's own
//! semi-major axis where it is a sphere, `RA` of Guidance Note 7-2's implementation notes where it
//! is not (ADR 3875).
//!
//! The paper also prints a polar aspect (24-3 to 24-6, 20-16 and 20-17); its formulas are the
//! oblique aspect's at φ1 = ±90°, the paper's `2R sin(π/4 − φ/2)` being `R k′ cos φ` there, so the
//! one evaluation serves every aspect.

use super::{LatLon, Projection};

impl Projection {
    /// `R`: the sphere's radius, or the authalic sphere's of an ellipsoid (Guidance Note 7-2's
    /// `RA = a [(1 − ((1 − e²) / 2e) ln((1 − e) / (1 + e))) / 2]^½`).
    fn spherical_radius(&self) -> f64 {
        let a = self.ellipsoid.semi_major_axis;
        let e = self.ellipsoid.eccentricity();
        if e == 0.0 {
            return a;
        }
        let e2 = e * e;
        a * ((1.0 - (1.0 - e2) / (2.0 * e) * ((1.0 - e) / (1.0 + e)).ln()) * 0.5).sqrt()
    }

    /// Professional Paper 1395's forward formulas, `delta` being λ − λ0, less the false easting
    /// and northing.
    ///
    /// The origin's antipode, where `k′` divides by zero, has no finite position, and the caller
    /// refuses the non-finite answer.
    pub(super) fn spherical_azimuthal_forward(&self, latitude: f64, delta: f64) -> (f64, f64) {
        let r = self.spherical_radius();
        let origin = Self::get(self.parameters.origin_latitude);
        let k = (2.0
            / (1.0 + origin.sin() * latitude.sin() + origin.cos() * latitude.cos() * delta.cos()))
        .sqrt();
        (
            r * k * latitude.cos() * delta.sin(),
            r * k * (origin.cos() * latitude.sin() - origin.sin() * latitude.cos() * delta.cos()),
        )
    }

    /// Professional Paper 1395's reverse formulas, from the easting and northing less the false
    /// ones.
    pub(super) fn spherical_azimuthal(&self, x: f64, y: f64) -> LatLon {
        let r = self.spherical_radius();
        let origin = Self::get(self.parameters.origin_latitude);
        let origin_longitude = Self::get(self.parameters.origin_longitude);
        let rho = x.hypot(y);
        if rho == 0.0 {
            // The origin itself, where equation 20-14's `/ ρ` has no value.
            return LatLon {
                latitude: origin,
                longitude: origin_longitude,
            };
        }
        // not a repair: an arithmetic guard on a ratio within ±1 inside the projection's disc,
        // not a file value.
        let c = 2.0 * (rho / (2.0 * r)).clamp(-1.0, 1.0).asin();
        // not a repair: as above.
        let latitude = (c.cos() * origin.sin() + y * c.sin() * origin.cos() / rho)
            .clamp(-1.0, 1.0)
            .asin();
        let longitude =
            (x * c.sin()).atan2(rho * origin.cos() * c.cos() - y * origin.sin() * c.sin());
        LatLon {
            latitude,
            longitude: origin_longitude + longitude,
        }
    }
}

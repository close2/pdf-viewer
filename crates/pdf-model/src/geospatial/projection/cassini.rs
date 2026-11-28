//! Cassini-Soldner, EPSG method 9806, by Guidance Note 7-2 section 3.2.2's formulas.
//!
//! The method is the transverse aspect of the equidistant cylindrical projection: true scale along
//! the central meridian, and the meridian distance `M` from the equator its northing. Both
//! directions are the section's series, which need no iteration.

use super::{LatLon, Projection};

/// Section 3.2.2's meridian distance `M` from the equator to a latitude, the series to its sixth
/// power of `e` as the section prints it, and section 3.1.4's for Bonne.
pub(super) fn meridian_distance(a: f64, e2: f64, latitude: f64) -> f64 {
    let (e4, e6) = (e2 * e2, e2 * e2 * e2);
    a * ((1.0 - e2 / 4.0 - 3.0 * e4 / 64.0 - 5.0 * e6 / 256.0) * latitude
        - (3.0 * e2 / 8.0 + 3.0 * e4 / 32.0 + 45.0 * e6 / 1024.0) * (2.0 * latitude).sin()
        + (15.0 * e4 / 256.0 + 45.0 * e6 / 1024.0) * (4.0 * latitude).sin()
        - (35.0 * e6 / 3072.0) * (6.0 * latitude).sin())
}

/// The latitude whose [`meridian_distance`] is `m`: section 3.2.2's footpoint `φ1` from
/// `µ1 = M1 / [a (1 − e²/4 − 3e⁴/64 − 5e⁶/256)]` and `e1 = [1 − (1 − e²)^0.5] / [1 + (1 − e²)^0.5]`,
/// which section 3.1.4 prints for Bonne too.
pub(super) fn footpoint_latitude(a: f64, e2: f64, m: f64) -> f64 {
    let (e4, e6) = (e2 * e2, e2 * e2 * e2);
    let mu1 = m / (a * (1.0 - e2 / 4.0 - 3.0 * e4 / 64.0 - 5.0 * e6 / 256.0));
    let root = (1.0 - e2).sqrt();
    let e1 = (1.0 - root) / (1.0 + root);
    mu1 + (3.0 * e1 / 2.0 - 27.0 * e1.powi(3) / 32.0) * (2.0 * mu1).sin()
        + (21.0 * e1.powi(2) / 16.0 - 55.0 * e1.powi(4) / 32.0) * (4.0 * mu1).sin()
        + (151.0 * e1.powi(3) / 96.0) * (6.0 * mu1).sin()
        + (1097.0 * e1.powi(4) / 512.0) * (8.0 * mu1).sin()
}

impl Projection {
    /// `MO`, the meridian distance of the latitude of natural origin.
    fn cassini_m0(&self) -> f64 {
        meridian_distance(
            self.ellipsoid.semi_major_axis,
            self.ellipsoid.eccentricity_squared(),
            Self::get(self.parameters.origin_latitude),
        )
    }

    /// Section 3.2.2's forward formulas: the easting and northing less the false ones.
    ///
    /// A point at a pole has no finite `tan φ`, and the caller refuses the non-finite answer.
    pub(super) fn cassini_forward(&self, latitude: f64, delta: f64) -> (f64, f64) {
        let a = self.ellipsoid.semi_major_axis;
        let e2 = self.ellipsoid.eccentricity_squared();
        let (sin, cos, tan) = (latitude.sin(), latitude.cos(), latitude.tan());
        let big_a = delta * cos;
        let t = tan * tan;
        let c = e2 * cos * cos / (1.0 - e2);
        let nu = a / (1.0 - e2 * sin * sin).sqrt();
        let x = meridian_distance(a, e2, latitude) - self.cassini_m0()
            + nu * tan * (big_a.powi(2) / 2.0 + (5.0 - t + 6.0 * c) * big_a.powi(4) / 24.0);
        let easting = nu
            * (big_a - t * big_a.powi(3) / 6.0 - (8.0 - t + 8.0 * c) * t * big_a.powi(5) / 120.0);
        (easting, x)
    }

    /// Section 3.2.2's reverse formulas, from the footpoint latitude `φ1` of the northing.
    pub(super) fn cassini(&self, x: f64, y: f64) -> LatLon {
        let a = self.ellipsoid.semi_major_axis;
        let e2 = self.ellipsoid.eccentricity_squared();
        let phi1 = footpoint_latitude(a, e2, self.cassini_m0() + y);
        let sin1 = phi1.sin();
        let w = 1.0 - e2 * sin1 * sin1;
        let nu1 = a / w.sqrt();
        let rho1 = a * (1.0 - e2) / w.powf(1.5);
        let tan1 = phi1.tan();
        let t1 = tan1 * tan1;
        let d = x / nu1;
        LatLon {
            latitude: phi1
                - (nu1 * tan1 / rho1) * (d.powi(2) / 2.0 - (1.0 + 3.0 * t1) * d.powi(4) / 24.0),
            longitude: Self::get(self.parameters.origin_longitude)
                + (d - t1 * d.powi(3) / 3.0 + (1.0 + 3.0 * t1) * t1 * d.powi(5) / 15.0)
                    / phi1.cos(),
        }
    }
}

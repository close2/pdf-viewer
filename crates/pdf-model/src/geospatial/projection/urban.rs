//! Colombia Urban, EPSG method 1052, by Guidance Note 7-2 section 3.4.3's formulas.
//!
//! The capital of each Colombian department maps its urban area on a plane through the origin at
//! the area's average height `hO`, so that survey distances need no reduction to the ellipsoid.
//! Neither direction iterates, and the reverse is the section's own closed form rather than an
//! exact inverse of the forward: its worked example returns to the printed latitude and longitude
//! to well inside a thousandth of a second.

use super::{LatLon, Projection};

impl Projection {
    /// `ρ`, the radius of curvature in the meridian, and `ν`, the one perpendicular to it, at a
    /// latitude (section 3.4.3).
    fn urban_radii(&self, latitude: f64) -> (f64, f64) {
        let a = self.ellipsoid.semi_major_axis;
        let e2 = self.ellipsoid.eccentricity_squared();
        let w = 1.0 - e2 * latitude.sin().powi(2);
        (a * (1.0 - e2) / w.powf(1.5), a / w.sqrt())
    }

    /// The section's `B = tan φO / 2ρOνO`, which both directions use.
    fn urban_b(&self) -> f64 {
        let latitude_0 = Self::get(self.parameters.origin_latitude);
        let (rho_0, nu_0) = self.urban_radii(latitude_0);
        latitude_0.tan() / (2.0 * rho_0 * nu_0)
    }

    /// Section 3.4.3's forward conversion, `delta` being λ − λO: the easting and northing less the
    /// false ones.
    pub(super) fn urban_forward(&self, latitude: f64, delta: f64) -> (f64, f64) {
        let latitude_0 = Self::get(self.parameters.origin_latitude);
        let height = Self::get(self.parameters.origin_height);
        let (rho_0, nu_0) = self.urban_radii(latitude_0);
        let (_, nu) = self.urban_radii(latitude);
        let (rho_m, _) = self.urban_radii(f64::midpoint(latitude_0, latitude));
        let big_a = 1.0 + height / nu_0;
        let big_g = 1.0 + height / rho_m;
        let cos = latitude.cos();
        (
            big_a * nu * cos * delta,
            big_g
                * rho_0
                * ((latitude - latitude_0) + self.urban_b() * delta * delta * nu * nu * cos * cos),
        )
    }

    /// Section 3.4.3's reverse conversion, from the easting and northing less the false ones.
    pub(super) fn urban_inverse(&self, x: f64, y: f64) -> LatLon {
        let p = &self.parameters;
        let (latitude_0, height) = (Self::get(p.origin_latitude), Self::get(p.origin_height));
        let a = self.ellipsoid.semi_major_axis;
        let e2 = self.ellipsoid.eccentricity_squared();
        let (rho_0, _) = self.urban_radii(latitude_0);
        let c = 1.0 + height / a;
        let d = rho_0 * (1.0 + height / (a * (1.0 - e2)));
        let latitude = latitude_0 + y / d - self.urban_b() * (x / c).powi(2);
        let (_, nu) = self.urban_radii(latitude);
        LatLon {
            latitude,
            longitude: Self::get(p.origin_longitude) + x / (c * nu * latitude.cos()),
        }
    }
}

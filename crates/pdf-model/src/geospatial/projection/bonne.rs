//! Bonne, EPSG methods 9827 and 9828, by Guidance Note 7-2 sections 3.1.4 and 3.1.4.1.
//!
//! An equal-area pseudoconic projection: each parallel is an arc of a circle about the centre of
//! the cone tangent at the origin's parallel, laid off at its true meridional distance, and each
//! point on it at its true distance along the parallel. The south-orientated form negates both
//! terms the false easting and northing are added to, which the caller does.

use std::f64::consts::FRAC_PI_2;

use super::cassini::{footpoint_latitude, meridian_distance};
use super::{LatLon, Projection};

impl Projection {
    /// Section 3.1.4's `m = cos ϕ / (1 – e² sin² ϕ)^0.5`.
    fn bonne_m(&self, latitude: f64) -> f64 {
        let e2 = self.ellipsoid.eccentricity_squared();
        latitude.cos() / (1.0 - e2 * latitude.sin().powi(2)).sqrt()
    }

    /// Section 3.1.4's `a mO / sin ϕO`, the radius of the origin's parallel on its cone.
    fn bonne_cone(&self) -> f64 {
        let origin = Self::get(self.parameters.origin_latitude);
        self.ellipsoid.semi_major_axis * self.bonne_m(origin) / origin.sin()
    }

    /// Section 3.1.4's forward formulas, `delta` being λ − λO, less the false easting and
    /// northing: `E = ρ sin T` and `N = a mO / sin ϕO − ρ cos T`.
    ///
    /// The section's `M` is the series section 3.2.2 also prints, to `e⁶`.
    pub(super) fn bonne_forward(&self, latitude: f64, delta: f64) -> (f64, f64) {
        let a = self.ellipsoid.semi_major_axis;
        let e2 = self.ellipsoid.eccentricity_squared();
        let cone = self.bonne_cone();
        let m_0 = meridian_distance(a, e2, Self::get(self.parameters.origin_latitude));
        let rho = cone + m_0 - meridian_distance(a, e2, latitude);
        // `ρ` is positive at every latitude for a ϕO north of the equator (negative south of
        // it), since `a mO / sin ϕO` exceeds the arc from ϕO to the pole; so `T` is finite, and
        // at a pole `m = 0` makes it zero, the pole one point whatever its longitude.
        let t = a * self.bonne_m(latitude) * delta / rho;
        (rho * t.sin(), cone - rho * t.cos())
    }

    /// Section 3.1.4's reverse formulas, from the easting and northing less the false ones.
    ///
    /// `ρ` takes the sign of ϕO, and so does the `atan2` that gives the longitude. At a pole
    /// `m = 0` and the section says to "use λ = λO"; a latitude within [`super::AT_THE_POLE`] of
    /// a pole is that pole.
    pub(super) fn bonne(&self, x: f64, y: f64) -> LatLon {
        let a = self.ellipsoid.semi_major_axis;
        let origin = Self::get(self.parameters.origin_latitude);
        let origin_longitude = Self::get(self.parameters.origin_longitude);
        let cone = self.bonne_cone();
        let rho = (x * x + (cone - y).powi(2)).sqrt().copysign(origin);
        let e2 = self.ellipsoid.eccentricity_squared();
        let m_0 = meridian_distance(a, e2, origin);
        let latitude = footpoint_latitude(a, e2, cone + m_0 - rho);
        if FRAC_PI_2 - latitude.abs() < super::AT_THE_POLE {
            return LatLon {
                latitude,
                longitude: origin_longitude,
            };
        }
        let angle = if origin < 0.0 {
            (-x).atan2(-(cone - y))
        } else {
            x.atan2(cone - y)
        };
        LatLon {
            latitude,
            longitude: origin_longitude + rho * angle / (a * self.bonne_m(latitude)),
        }
    }
}

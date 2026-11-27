//! Polar Stereographic, EPSG methods 9810, 9829 and 9830 — variants A, B and C — by Guidance Note
//! 7-2 section 3.3.1.2's formulas.
//!
//! The three variants are one projection defined three ways: A by the scale at the pole, B by the
//! parallel along which the scale is one, and C by that parallel with the false grid coordinates
//! at its crossing of the longitude of origin. In each the radius from the pole is a multiple of
//! `t`, so one constant per variant — the multiple, and for C the false origin's radius `ρF` —
//! carries the whole difference.
//!
//! The pole is the one the variant states: for A the latitude of natural origin, which the
//! section allows only as ±90°, and for B and C the hemisphere of the standard parallel, as the
//! section's note 2 says the Dataset defines it.

use std::f64::consts::{FRAC_PI_2, FRAC_PI_4};

use super::{LatLon, Method, Projection};

/// Section 3.3.1.2's constants for one projection.
struct Polar {
    /// The north pole case, rather than the south.
    north: bool,
    /// `ρ / t`: `2 a kO / √((1+e)^(1+e) (1−e)^(1−e))` for variant A, and `a mF / tF` for B and C.
    rho_per_t: f64,
    /// Variant C's `ρF`, the radius of the false origin; zero for A and B, whose false grid
    /// coordinates are the pole's.
    rho_f: f64,
}

impl Projection {
    /// Section 3.3.1.2's `t` at a latitude, for the north or the south pole case.
    fn polar_t(&self, latitude: f64, north: bool) -> f64 {
        let e = self.ellipsoid.eccentricity();
        let s = e * latitude.sin();
        let factor = ((1.0 + s) / (1.0 - s)).powf(e / 2.0);
        if north {
            (FRAC_PI_4 - latitude / 2.0).tan() * factor
        } else {
            (FRAC_PI_4 + latitude / 2.0).tan() / factor
        }
    }

    /// The constants of this projection's variant.
    fn polar_constants(&self) -> Polar {
        let a = self.ellipsoid.semi_major_axis;
        let e = self.ellipsoid.eccentricity();
        let e2 = self.ellipsoid.eccentricity_squared();
        let p = &self.parameters;
        if self.method == Method::PolarStereographicA {
            let k = ((1.0 + e).powf(1.0 + e) * (1.0 - e).powf(1.0 - e)).sqrt();
            return Polar {
                north: Self::get(p.origin_latitude) > 0.0,
                rho_per_t: 2.0 * a * Self::get(p.scale) / k,
                rho_f: 0.0,
            };
        }
        let parallel = Self::get(p.parallel_1);
        let north = parallel > 0.0;
        let m_f = parallel.cos() / (1.0 - e2 * parallel.sin().powi(2)).sqrt();
        let t_f = self.polar_t(parallel, north);
        Polar {
            north,
            rho_per_t: a * m_f / t_f,
            rho_f: if self.method == Method::PolarStereographicC {
                a * m_f
            } else {
                0.0
            },
        }
    }

    /// Section 3.3.1.2's forward formulas, less the false easting and northing (variant C's
    /// easting and northing at the false origin).
    ///
    /// The pole opposite the projection's has no finite `t`, and the caller refuses the
    /// non-finite answer.
    pub(super) fn polar_forward(&self, latitude: f64, delta: f64) -> (f64, f64) {
        let Polar {
            north,
            rho_per_t,
            rho_f,
        } = self.polar_constants();
        let rho = rho_per_t * self.polar_t(latitude, north);
        let towards = rho * delta.cos();
        let y = if north {
            rho_f - towards
        } else {
            towards - rho_f
        };
        (rho * delta.sin(), y)
    }

    /// Section 3.3.1.2's reverse formulas, from the easting and northing less the false ones.
    ///
    /// The longitude is `atan2` of the offset from the pole in every case, which is the
    /// section's `λO` where the easting is the false one on the origin's side of the pole, and
    /// at the pole itself, and the opposite meridian beyond it.
    pub(super) fn polar(&self, x: f64, y: f64) -> LatLon {
        let Polar {
            north,
            rho_per_t,
            rho_f,
        } = self.polar_constants();
        let from_pole = if north { rho_f - y } else { y + rho_f };
        let t = x.hypot(from_pole) / rho_per_t;
        let chi = if north {
            FRAC_PI_2 - 2.0 * t.atan()
        } else {
            2.0 * t.atan() - FRAC_PI_2
        };
        let e2 = self.ellipsoid.eccentricity_squared();
        let (e4, e6, e8) = (e2 * e2, e2.powi(3), e2.powi(4));
        let latitude = chi
            + (e2 / 2.0 + 5.0 * e4 / 24.0 + e6 / 12.0 + 13.0 * e8 / 360.0) * (2.0 * chi).sin()
            + (7.0 * e4 / 48.0 + 29.0 * e6 / 240.0 + 811.0 * e8 / 11520.0) * (4.0 * chi).sin()
            + (7.0 * e6 / 120.0 + 81.0 * e8 / 1120.0) * (6.0 * chi).sin()
            + (4279.0 * e8 / 161_280.0) * (8.0 * chi).sin();
        LatLon {
            latitude,
            longitude: Self::get(self.parameters.origin_longitude) + x.atan2(from_pole),
        }
    }
}

//! Lambert Azimuthal Equal Area, EPSG method 9820, by Guidance Note 7-2 section 3.3.2's formulas:
//! the oblique aspect, and the polar aspect where the section says the oblique formulas are
//! indeterminate, at a latitude of natural origin of ±90°.
//!
//! The ellipsoid is carried to the authalic sphere — the sphere of the same surface area, on which
//! the authalic latitude `β` is a latitude — and projected azimuthally from there; the reverse
//! returns through the series section 3.3.2 prints, which needs no iteration.

use std::f64::consts::FRAC_PI_2;

use super::{LatLon, Projection};

/// Section 3.3.2's constants at the origin: `qP`, `Rq`, `βO` and `D`.
struct Authalic {
    /// `qP`, `q` at the pole.
    q_pole: f64,
    /// `Rq`, the authalic sphere's radius.
    radius: f64,
    /// `βO`, the origin's authalic latitude.
    beta_0: f64,
    /// `D`, the oblique aspect's scale correction.
    d: f64,
}

impl Projection {
    /// Whether the latitude of natural origin is a pole, where section 3.3.2's polar aspect
    /// applies; `Some(true)` for the north pole.
    fn azimuthal_pole(&self) -> Option<bool> {
        let origin = Self::get(self.parameters.origin_latitude);
        ((origin.abs() - FRAC_PI_2).abs() < 1e-12).then_some(origin > 0.0)
    }

    /// The section's constants for this projection.
    fn authalic(&self) -> Authalic {
        let a = self.ellipsoid.semi_major_axis;
        let e2 = self.ellipsoid.eccentricity_squared();
        let origin = Self::get(self.parameters.origin_latitude);
        // `q` is section 3.1.3's `α`, the one formula, with its spherical limit.
        let q_pole = self.albers_alpha(FRAC_PI_2);
        let radius = a * (q_pole / 2.0).sqrt();
        // not a repair: the clamp keeps a rounding past one from becoming NaN, an arithmetic guard
        // not a file value.
        let beta_0 = (self.albers_alpha(origin) / q_pole).clamp(-1.0, 1.0).asin();
        let d =
            a * (origin.cos() / (1.0 - e2 * origin.sin().powi(2)).sqrt()) / (radius * beta_0.cos());
        Authalic {
            q_pole,
            radius,
            beta_0,
            d,
        }
    }

    /// The series section 3.3.2 gives from an authalic latitude `β'` back to a geodetic one.
    fn geodetic_of_authalic(&self, beta: f64) -> f64 {
        let e2 = self.ellipsoid.eccentricity_squared();
        let (e4, e6) = (e2 * e2, e2 * e2 * e2);
        beta + (e2 / 3.0 + 31.0 * e4 / 180.0 + 517.0 * e6 / 5040.0) * (2.0 * beta).sin()
            + (23.0 * e4 / 360.0 + 251.0 * e6 / 3780.0) * (4.0 * beta).sin()
            + (761.0 * e6 / 45360.0) * (6.0 * beta).sin()
    }

    /// Section 3.3.2's forward formulas, oblique or polar, less the false easting and northing.
    ///
    /// The antipode of the origin has no finite position (the oblique `B` divides by zero there),
    /// and the caller refuses the non-finite answer.
    pub(super) fn azimuthal_forward(&self, latitude: f64, delta: f64) -> (f64, f64) {
        let a = self.ellipsoid.semi_major_axis;
        let q = self.albers_alpha(latitude);
        let constants = self.authalic();
        if let Some(north) = self.azimuthal_pole() {
            let rho = if north {
                a * (constants.q_pole - q).sqrt()
            } else {
                a * (constants.q_pole + q).sqrt()
            };
            let northing = rho * delta.cos();
            (rho * delta.sin(), if north { -northing } else { northing })
        } else {
            let Authalic {
                q_pole,
                radius,
                beta_0,
                d,
            } = constants;
            // not a repair: an arithmetic guard on `q / qP`, which is within ±1 for every
            // latitude, not a file value.
            let beta = (q / q_pole).clamp(-1.0, 1.0).asin();
            let b = radius
                * (2.0
                    / (1.0 + beta_0.sin() * beta.sin() + beta_0.cos() * beta.cos() * delta.cos()))
                .sqrt();
            (
                b * d * beta.cos() * delta.sin(),
                (b / d) * (beta_0.cos() * beta.sin() - beta_0.sin() * beta.cos() * delta.cos()),
            )
        }
    }

    /// Section 3.3.2's reverse formulas, oblique or polar, from the easting and northing less
    /// the false ones.
    pub(super) fn azimuthal(&self, x: f64, y: f64) -> LatLon {
        let a = self.ellipsoid.semi_major_axis;
        let origin_longitude = Self::get(self.parameters.origin_longitude);
        let Authalic {
            q_pole,
            radius,
            beta_0,
            d,
        } = self.authalic();
        let (beta, longitude) = if let Some(north) = self.azimuthal_pole() {
            let rho = x.hypot(y);
            // not a repair: an arithmetic guard on a ratio the formulas keep within ±1 inside
            // the projection's disc, not a file value.
            let beta = (1.0 - rho * rho / (a * a * q_pole)).clamp(-1.0, 1.0).asin();
            if north {
                (beta, x.atan2(-y))
            } else {
                (-beta, x.atan2(y))
            }
        } else {
            let rho = (x / d).hypot(d * y);
            if rho == 0.0 {
                // The origin itself, where the formulas' `/ ρ` has no value.
                return LatLon {
                    latitude: Self::get(self.parameters.origin_latitude),
                    longitude: origin_longitude,
                };
            }
            // not a repair: arithmetic guards on ratios within ±1 inside the projection's
            // disc, not file values.
            let c = 2.0 * (rho / (2.0 * radius)).clamp(-1.0, 1.0).asin();
            let beta = (c.cos() * beta_0.sin() + d * y * c.sin() * beta_0.cos() / rho)
                .clamp(-1.0, 1.0)
                .asin();
            let longitude = (x * c.sin())
                .atan2(d * rho * beta_0.cos() * c.cos() - d * d * y * beta_0.sin() * c.sin());
            (beta, longitude)
        };
        LatLon {
            latitude: self.geodetic_of_authalic(beta),
            longitude: origin_longitude + longitude,
        }
    }
}

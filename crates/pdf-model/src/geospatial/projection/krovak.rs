//! Krovak, EPSG methods 9819, 1041, 1042 and 1043, by Guidance Note 7-2 sections 3.1.2.1 to
//! 3.1.2.4's formulas.
//!
//! The ellipsoid is carried to a Gaussian conformal sphere about the projection centre, the
//! sphere is turned so that the cone's axis is its pole, and the oblique conic projection of the
//! turned sphere gives the plane coordinates `Xp` and `Yp`: a southing and a westing. The North
//! Orientated variants negate both grid coordinates after the false ones are added (sections
//! 3.1.2.2 and 3.1.2.4); the Modified variants subtract section 3.1.2.3's polynomial in the plane
//! coordinates about the evaluation point first.
//!
//! [`Projection::forward`] and [`Projection::inverse`] carry a Krovak grid point as the westing
//! (or easting) first and the southing (or northing) second, the order the other methods' east-west
//! and north-south axes take; section 3.1.2.1 writes the southing `X` and the westing `Y`.

use std::f64::consts::FRAC_PI_4;

use super::{CONVERGED, LatLon, MAX_ITERATIONS, Method, Projection};
use crate::geospatial::Refusal;

/// Section 3.1.2.1's constants of the projection: `B`, `tO`, `n` and `rO`.
struct Cone {
    /// `B`, the longitude's scale onto the conformal sphere.
    b: f64,
    /// `tO`, the latitude's constant onto the sphere.
    t_0: f64,
    /// `n = sin φP`, the cone's constant.
    n: f64,
    /// `rO = kP A / tan φP`, the radius of the pseudo standard parallel.
    r_0: f64,
}

/// `D`, the pseudo-longitude on the turned sphere: section 3.1.2.1's `atan2(sin D1, cos D1)` of
/// the conformal latitude `U`, the scaled longitude `V`, the pseudo-latitude `T` and the cone
/// axis's co-latitude `αC`.
fn pseudo_longitude(alpha: f64, u: f64, v: f64, t: f64) -> f64 {
    let sin_d = u.cos() * v.sin() / t.cos();
    let cos_d = (alpha.cos() * t.sin() - u.sin()) / (alpha.sin() * t.cos());
    sin_d.atan2(cos_d)
}

impl Projection {
    /// For a Krovak method, whether it is north orientated; `None` for every other method.
    pub(super) fn krovak_orientation(&self) -> Option<bool> {
        match self.method {
            Method::Krovak | Method::KrovakModified => Some(false),
            Method::KrovakNorthOrientated | Method::KrovakModifiedNorthOrientated => Some(true),
            _ => None,
        }
    }

    /// Whether this is one of section 3.1.2.3's modified methods.
    fn krovak_modified(&self) -> bool {
        matches!(
            self.method,
            Method::KrovakModified | Method::KrovakModifiedNorthOrientated
        )
    }

    /// The section's constants for this projection.
    fn krovak_cone(&self) -> Cone {
        let p = &self.parameters;
        let a = self.ellipsoid.semi_major_axis;
        let e = self.ellipsoid.eccentricity();
        let e2 = self.ellipsoid.eccentricity_squared();
        let centre = Self::get(p.origin_latitude);
        let parallel = Self::get(p.parallel_1);
        let s = centre.sin();
        let big_a = a * (1.0 - e2).sqrt() / (1.0 - e2 * s * s);
        let b = (1.0 + e2 * centre.cos().powi(4) / (1.0 - e2)).sqrt();
        let gamma_0 = (s / b).asin();
        let t_0 = (FRAC_PI_4 + gamma_0 / 2.0).tan()
            * ((1.0 + e * s) / (1.0 - e * s)).powf(e * b / 2.0)
            / (FRAC_PI_4 + centre / 2.0).tan().powf(b);
        Cone {
            b,
            t_0,
            n: parallel.sin(),
            r_0: Self::get(p.scale) * big_a / parallel.tan(),
        }
    }

    /// Section 3.1.2.3's `dX` and `dY` of plane coordinates `Xr` and `Yr` about the evaluation
    /// point; the same polynomial serves both directions.
    fn krovak_correction(&self, xr: f64, yr: f64) -> (f64, f64) {
        let c = self.parameters.coefficients.map(Self::get);
        let (x2, y2) = (xr * xr, yr * yr);
        let dx = c[0] + c[2] * xr - c[3] * yr - 2.0 * c[5] * xr * yr
            + c[4] * (x2 - y2)
            + c[6] * xr * (x2 - 3.0 * y2)
            - c[7] * yr * (3.0 * x2 - y2)
            + 4.0 * c[8] * xr * yr * (x2 - y2)
            + c[9] * (x2 * x2 + y2 * y2 - 6.0 * x2 * y2);
        let dy = c[1]
            + c[2] * yr
            + c[3] * xr
            + 2.0 * c[4] * xr * yr
            + c[5] * (x2 - y2)
            + c[7] * xr * (x2 - 3.0 * y2)
            + c[6] * yr * (3.0 * x2 - y2)
            - 4.0 * c[9] * xr * yr * (x2 - y2)
            + c[8] * (x2 * x2 + y2 * y2 - 6.0 * x2 * y2);
        (dx, dy)
    }

    /// Sections 3.1.2.1 and 3.1.2.3's forward formulas, `delta` being λ − λO: the westing and the
    /// southing, the false ones included.
    ///
    /// `D` is the section's `atan2(sin D1, cos D1)`, which it gives for a point past ±90° of
    /// pseudo-longitude from the central line: within that range it is the `asin` the section
    /// prints first, and past it the `asin` would answer for the mirror point, so the one form
    /// serves both sides (ADR 3876).
    pub(super) fn krovak_forward(&self, latitude: f64, delta: f64) -> [f64; 2] {
        let p = &self.parameters;
        let e = self.ellipsoid.eccentricity();
        let alpha = Self::get(p.azimuth);
        let parallel = Self::get(p.parallel_1);
        let Cone { b, t_0, n, r_0 } = self.krovak_cone();
        let s = latitude.sin();
        let u = 2.0
            * ((t_0 * (latitude / 2.0 + FRAC_PI_4).tan().powf(b)
                / ((1.0 + e * s) / (1.0 - e * s)).powf(e * b / 2.0))
            .atan()
                - FRAC_PI_4);
        // Section 3.1.2.1's `V = B (λO − λ)`.
        let v = -b * delta;
        let t = (alpha.cos() * u.sin() + alpha.sin() * u.cos() * v.cos()).asin();
        let d = pseudo_longitude(alpha, u, v, t);
        let theta = n * d;
        let r =
            r_0 * (FRAC_PI_4 + parallel / 2.0).tan().powf(n) / (t / 2.0 + FRAC_PI_4).tan().powf(n);
        let (xp, yp) = (r * theta.cos(), r * theta.sin());
        let (dx, dy) = if self.krovak_modified() {
            let [x_0, y_0] = p.evaluation_point.map(Self::get);
            self.krovak_correction(xp - x_0, yp - y_0)
        } else {
            (0.0, 0.0)
        };
        // not a repair: the false westing and southing a system states, absent read as zero as
        // for every method (`Projection::forward`).
        let false_easting = p.false_easting.unwrap_or(0.0);
        let false_northing = p.false_northing.unwrap_or(0.0);
        [yp - dy + false_easting, xp - dx + false_northing]
    }

    /// Sections 3.1.2.1 and 3.1.2.3's reverse formulas, from the westing and the southing.
    pub(super) fn krovak_inverse(&self, westing: f64, southing: f64) -> Result<LatLon, Refusal> {
        let p = &self.parameters;
        let e = self.ellipsoid.eccentricity();
        let alpha = Self::get(p.azimuth);
        let parallel = Self::get(p.parallel_1);
        let Cone { b, t_0, n, r_0 } = self.krovak_cone();
        // not a repair: as in `krovak_forward`.
        let x = southing - p.false_northing.unwrap_or(0.0);
        let y = westing - p.false_easting.unwrap_or(0.0);
        let (xp, yp) = if self.krovak_modified() {
            let [x_0, y_0] = p.evaluation_point.map(Self::get);
            let (dx, dy) = self.krovak_correction(x - x_0, y - y_0);
            (x + dx, y + dy)
        } else {
            (x, y)
        };
        let r = xp.hypot(yp);
        let theta = yp.atan2(xp);
        let d = theta / parallel.sin();
        let t = 2.0
            * (((r_0 / r).powf(1.0 / n) * (FRAC_PI_4 + parallel / 2.0).tan()).atan() - FRAC_PI_4);
        let u = (alpha.cos() * t.sin() - alpha.sin() * t.cos() * d.cos()).asin();
        let v = (t.cos() * d.sin() / u.cos()).asin();
        let scaled = t_0.powf(-1.0 / b) * (u / 2.0 + FRAC_PI_4).tan().powf(1.0 / b);
        let mut latitude = u;
        for _ in 0..MAX_ITERATIONS {
            let s = latitude.sin();
            let next =
                2.0 * ((scaled * ((1.0 + e * s) / (1.0 - e * s)).powf(e / 2.0)).atan() - FRAC_PI_4);
            let done = (next - latitude).abs() < CONVERGED;
            latitude = next;
            if done {
                return Ok(LatLon {
                    latitude,
                    longitude: Self::get(p.origin_longitude) - v / b,
                });
            }
        }
        Err(Refusal::NoConvergence {
            method: self.method.name(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::pseudo_longitude;

    /// Section 3.1.2.1's two forms of `D` on both sides of the ±90° its first is "satisfactory"
    /// within: on the near side the printed `asin` is the `atan2`, and on the far side the `asin`
    /// is the mirror `π − D` of the point's own pseudo-longitude, which the `atan2` keeps
    /// (ADR 3876).
    #[test]
    fn the_pseudo_longitude_is_one_formula_on_both_sides_of_ninety_degrees() {
        let alpha = 0.528_627_763_f64;
        // The S-JTSK example's U and V, inside the range.
        let (u, v) = (0.875_596_949_f64, 0.139_422_687_f64);
        let t = (alpha.cos() * u.sin() + alpha.sin() * u.cos() * v.cos()).asin();
        let printed = (u.cos() * v.sin() / t.cos()).asin();
        assert!((pseudo_longitude(alpha, u, v, t) - printed).abs() < 1e-12);
        assert!((printed - 0.506_554_623).abs() < 1e-9, "{printed}");
        // A point on the far side of the cone's axis, more than 90° of pseudo-longitude away.
        let (u, v) = (1.4_f64, 2.5_f64);
        let t = (alpha.cos() * u.sin() + alpha.sin() * u.cos() * v.cos()).asin();
        let d = pseudo_longitude(alpha, u, v, t);
        let printed = (u.cos() * v.sin() / t.cos()).asin();
        assert!(d.abs() > std::f64::consts::FRAC_PI_2, "{d}");
        assert!(
            (printed - (std::f64::consts::PI - d)).abs() < 1e-12,
            "{printed} {d}"
        );
    }
}

//! Equal Earth, EPSG method 1078, by Guidance Note 7-2 section 3.4.4's formulas.
//!
//! A pseudocylindrical equal-area projection: the latitude is carried to the authalic sphere
//! (radius `Rq`, latitude `β`), then to the parametric angle `θ = asin(sin β √3 / 2)`, and the
//! northing is a polynomial in `θ` whose derivative is the easting's denominator. The reverse
//! inverts that polynomial by the section's Newton iteration and returns from `β` by the series
//! section 3.3.2 also prints, which needs no iteration.

use std::f64::consts::FRAC_PI_2;

use super::{CONVERGED, LatLon, MAX_ITERATIONS, Projection};
use crate::geospatial::Refusal;

/// The northing's polynomial in `θ`, `θ [1.340264 − 0.081106 θ² + θ⁶ (0.000893 + 0.003796 θ²)]`.
fn northing_polynomial(theta: f64) -> f64 {
    let t2 = theta * theta;
    theta * (1.340_264 - 0.081_106 * t2 + t2 * t2 * t2 * (0.000_893 + 0.003_796 * t2))
}

/// Its derivative in `θ`, which section 3.4.4 prints as the easting's denominator,
/// `1.340264 − 0.243318 θ² + θ⁶ (0.006251 + 0.034164 θ²)`.
fn derivative(theta: f64) -> f64 {
    let t2 = theta * theta;
    1.340_264 - 0.243_318 * t2 + t2 * t2 * t2 * (0.006_251 + 0.034_164 * t2)
}

impl Projection {
    /// Section 3.4.4's `Rq = a (qP / 2)^0.5`, the authalic sphere's radius, and `qP`.
    ///
    /// On a sphere `q` is `2 sin ϕ`, so `qP = 2`, `Rq = R` and `β = ϕ`: the section's spherical
    /// form is the same evaluation.
    fn equal_earth_sphere(&self) -> (f64, f64) {
        let q_pole = self.albers_alpha(FRAC_PI_2);
        (
            self.ellipsoid.semi_major_axis * (q_pole / 2.0).sqrt(),
            q_pole,
        )
    }

    /// Section 3.4.4's forward formulas, `delta` being λ − λO, less the false easting and
    /// northing.
    pub(super) fn equal_earth_forward(&self, latitude: f64, delta: f64) -> (f64, f64) {
        let (r_q, q_pole) = self.equal_earth_sphere();
        // not a repair: an arithmetic guard on `q / qP`, which is within ±1 for every
        // latitude, not a file value.
        let ratio = (self.albers_alpha(latitude) / q_pole).clamp(-1.0, 1.0);
        let beta = ratio.asin();
        let theta = (beta.sin() * 3f64.sqrt() / 2.0).asin();
        (
            r_q * 2.0 * delta * theta.cos() / (3f64.sqrt() * derivative(theta)),
            r_q * northing_polynomial(theta),
        )
    }

    /// Section 3.4.4's reverse formulas, from the easting and northing less the false ones.
    ///
    /// A northing past the pole's has no `θ` whose `asin{2 sin(θ) / √3}` is a latitude; the
    /// non-finite answer is refused by the caller.
    pub(super) fn equal_earth(&self, x: f64, y: f64) -> Result<LatLon, Refusal> {
        let (r_q, _) = self.equal_earth_sphere();
        let target = y / r_q;
        let mut theta = target;
        for _ in 0..MAX_ITERATIONS {
            let step = (northing_polynomial(theta) - target) / derivative(theta);
            theta -= step;
            if step.abs() < CONVERGED {
                let beta = (2.0 * theta.sin() / 3f64.sqrt()).asin();
                return Ok(LatLon {
                    latitude: self.geodetic_of_authalic(beta),
                    longitude: Self::get(self.parameters.origin_longitude)
                        + 3f64.sqrt() * x * derivative(theta) / (2.0 * r_q * theta.cos()),
                });
            }
        }
        Err(Refusal::NoConvergence {
            method: self.method.name(),
        })
    }
}

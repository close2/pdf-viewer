//! Tunisia Mining Grid, EPSG method 9816, by Guidance Note 7-2 section 3.4.2's formulas.
//!
//! The grid is the mining decree's tabulation of block corners: latitude and longitude, in grads
//! from Paris, linear in the northing and the easting in kilometres. The section derives its rates
//! from the decree's 2 km intervals — 0.02437 grads of longitude, and 0.02004 grads of latitude
//! south of the 360 km northing and 0.02003 north of it — and its end values (94 km at 5.68989
//! grads, 490 km at 10.51515, 40 km at 33.39, 860 km at 41.6039) are those rates from the origin to
//! the last digit, which the tests check.
//!
//! The section prints the direction from the grid first and calls the other "the reverse"; here,
//! as for every method, the forward direction is from latitude and longitude. Its line after the
//! longitude's formula says "where ϕ and ϕO are in grads relative to the Paris meridian", where the
//! formula above it is in λ and λO: it is read as naming those (ADR 3887).

use std::f64::consts::PI;

use super::{LatLon, Projection};

/// One grad in radians.
const GRAD: f64 = PI / 200.0;

/// The section's `0.012185` grads of longitude a kilometre of easting, in radians a metre.
const LONGITUDE_RATE: f64 = 0.012_185 * GRAD / 1000.0;

/// The section's `A = 0.010015` grads of latitude a kilometre of northing north of the false
/// origin, in radians a metre.
const NORTH_RATE: f64 = 0.010_015 * GRAD / 1000.0;

/// The section's `A = 0.01002` south of it, and on it.
const SOUTH_RATE: f64 = 0.010_02 * GRAD / 1000.0;

impl Projection {
    /// The section's `E = EF + [(λ – λO) / 0.012185]` and `N = NF + [(ϕ – ϕO) / A]`, `A` being
    /// 0.010015 "if ϕ > ϕO" and 0.01002 otherwise, less the false easting and northing.
    pub(super) fn tunisia_forward(&self, latitude: f64, delta: f64) -> (f64, f64) {
        let rise = latitude - Self::get(self.parameters.origin_latitude);
        let rate = if rise > 0.0 { NORTH_RATE } else { SOUTH_RATE };
        (delta / LONGITUDE_RATE, rise / rate)
    }

    /// The section's `ϕ = ϕO + [(N – NF) * A]` and `λ = λO + [(E – EF) * 0.012185]`, `A` being
    /// 0.010015 "if N > NF" and 0.01002 otherwise.
    pub(super) fn tunisia(&self, x: f64, y: f64) -> LatLon {
        let rate = if y > 0.0 { NORTH_RATE } else { SOUTH_RATE };
        LatLon {
            latitude: Self::get(self.parameters.origin_latitude) + y * rate,
            longitude: Self::get(self.parameters.origin_longitude) + x * LONGITUDE_RATE,
        }
    }
}

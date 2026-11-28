//! American Polyconic, EPSG method 9818, by Guidance Note 7-2 section 3.1.5's formulas.
//!
//! Each parallel is the arc of its own tangent cone, laid off along the central meridian at its
//! true meridional distance; the reverse is the section's Newton–Raphson iteration in latitude.
//!
//! Two of the reverse's printed lines are read rather than evaluated as they stand (ADR 3876).
//! The equator's case is printed "\[i]f MO = (N-FN)", where the forward formulas put the equator
//! at `N − FN = −MO`: the case is `A = 0`, where the iteration's own `A` vanishes, which is the
//! condition the section's source, USGS Professional Paper 1395 (equation 7-12's case `y = −M0`),
//! states. And the iteration's `H` is printed `H = Mn`, which makes `C H − 2 / sin 2ϕ'` a
//! difference of a length and a number: the step is Newton's on the numerator, whose derivative
//! takes `H` as `M′`, the derivative of `M / a` in latitude, as Professional Paper 1395's
//! equations 18-17 and 18-21 write it.

use super::{CONVERGED, LatLon, MAX_ITERATIONS, Projection};
use crate::geospatial::Refusal;

impl Projection {
    /// The meridional arc `M` from the equator to a latitude, section 3.1.5's series to `e⁶`.
    fn polyconic_arc(&self, latitude: f64) -> f64 {
        let a = self.ellipsoid.semi_major_axis;
        let [c0, c2, c4, c6] = self.polyconic_series();
        a * (c0 * latitude - c2 * (2.0 * latitude).sin() + c4 * (4.0 * latitude).sin()
            - c6 * (6.0 * latitude).sin())
    }

    /// The derivative of `M / a` in latitude, `M′` of Professional Paper 1395's equation 18-17.
    fn polyconic_arc_rate(&self, latitude: f64) -> f64 {
        let [c0, c2, c4, c6] = self.polyconic_series();
        c0 - 2.0 * c2 * (2.0 * latitude).cos() + 4.0 * c4 * (4.0 * latitude).cos()
            - 6.0 * c6 * (6.0 * latitude).cos()
    }

    /// The series' four coefficients in `e²`.
    fn polyconic_series(&self) -> [f64; 4] {
        let e2 = self.ellipsoid.eccentricity_squared();
        let (e4, e6) = (e2 * e2, e2 * e2 * e2);
        [
            1.0 - e2 / 4.0 - 3.0 * e4 / 64.0 - 5.0 * e6 / 256.0,
            3.0 * e2 / 8.0 + 3.0 * e4 / 32.0 + 45.0 * e6 / 1024.0,
            15.0 * e4 / 256.0 + 45.0 * e6 / 1024.0,
            35.0 * e6 / 3072.0,
        ]
    }

    /// Section 3.1.5's forward formulas, `delta` being λ − λO, less the false easting and
    /// northing.
    ///
    /// The equator's case, `E − FE = a(λ − λO)`, is the limit of the general case's `ν cot ϕ sin
    /// L` as ϕ goes to zero, so the two agree on both sides of it.
    pub(super) fn polyconic_forward(&self, latitude: f64, delta: f64) -> (f64, f64) {
        let a = self.ellipsoid.semi_major_axis;
        let m_0 = self.polyconic_arc(Self::get(self.parameters.origin_latitude));
        if latitude == 0.0 {
            return (a * delta, -m_0);
        }
        let e2 = self.ellipsoid.eccentricity_squared();
        let nu = a / (1.0 - e2 * latitude.sin().powi(2)).sqrt();
        let l = delta * latitude.sin();
        let cot = 1.0 / latitude.tan();
        (
            nu * cot * l.sin(),
            self.polyconic_arc(latitude) - m_0 + nu * cot * (1.0 - l.cos()),
        )
    }

    /// Section 3.1.5's reverse formulas, from the easting and northing less the false ones.
    pub(super) fn polyconic(&self, x: f64, y: f64) -> Result<LatLon, Refusal> {
        let a = self.ellipsoid.semi_major_axis;
        let e2 = self.ellipsoid.eccentricity_squared();
        let origin_longitude = Self::get(self.parameters.origin_longitude);
        let m_0 = self.polyconic_arc(Self::get(self.parameters.origin_latitude));
        let big_a = (m_0 + y) / a;
        if big_a == 0.0 {
            return Ok(LatLon {
                latitude: 0.0,
                longitude: origin_longitude + x / a,
            });
        }
        let big_b = big_a * big_a + (x * x) / (a * a);
        let c_of = |latitude: f64| (1.0 - e2 * latitude.sin().powi(2)).sqrt() * latitude.tan();
        let mut latitude = big_a;
        for _ in 0..MAX_ITERATIONS {
            let c = c_of(latitude);
            let j = self.polyconic_arc(latitude) / a;
            let h = self.polyconic_arc_rate(latitude);
            let twice = (2.0 * latitude).sin();
            let step = (big_a * (c * j + 1.0) - j - 0.5 * c * (j * j + big_b))
                / (e2 * twice * (j * j + big_b - 2.0 * big_a * j) / (4.0 * c)
                    + (big_a - j) * (c * h - 2.0 / twice)
                    - h);
            let next = latitude - step;
            let done = (next - latitude).abs() < CONVERGED;
            latitude = next;
            if done {
                return Ok(LatLon {
                    latitude,
                    longitude: origin_longitude + (x * c_of(latitude) / a).asin() / latitude.sin(),
                });
            }
        }
        Err(Refusal::NoConvergence {
            method: self.method.name(),
        })
    }
}

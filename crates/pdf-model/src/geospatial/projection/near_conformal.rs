//! Lambert Conic Near-Conformal, EPSG method 9817, by Guidance Note 7-2 section 3.1.1.6's
//! formulas: the Army Map Service's one-parallel Lambert, its meridian arc a series in the third
//! flattening `n` and its radius `M = kO (m + A m³)` truncated at the third order.
//!
//! The arc's five coefficients are the expansion of the rectifying radius `a (1 + n²/4 + n⁴/64)
//! / (1 + n)` and its sine terms to `n⁵`; their first terms are `3an/2`, `15an²/16`, `35an³/48` and
//! `315an⁴/512`, and the section's example prints them all to the digits it gives (A′ =
//! 111131.8633 m a degree, B′ = 16300.64407 m). The arc agrees with section 3.2.2's to a fraction of
//! a millimetre, which the tests check.

use std::f64::consts::PI;

use super::{CONVERGED, LatLon, MAX_ITERATIONS, Projection, Refusal};

/// The section's constants of the projection.
struct MeridianSeries {
    /// `A′`, metres a degree of latitude, and `B′` to `E′`, metres.
    coefficients: [f64; 5],
    /// `A = 1 / (6 ρO νO)`.
    big_a: f64,
    /// `rO = kO νO / tan ϕO`.
    r_0: f64,
    /// `sO`, the arc to the latitude of natural origin.
    s_0: f64,
}

impl MeridianSeries {
    /// `s = A′ ϕ – B′ sin 2ϕ + C′ sin 4ϕ – D′ sin 6ϕ + E′ sin 8ϕ`, ϕ in degrees in the first term and
    /// radians in the rest.
    fn s(&self, latitude: f64) -> f64 {
        let [a, b, c, d, e] = self.coefficients;
        a * latitude.to_degrees() - b * (2.0 * latitude).sin() + c * (4.0 * latitude).sin()
            - d * (6.0 * latitude).sin()
            + e * (8.0 * latitude).sin()
    }
}

impl Projection {
    /// The section's first constants, from the ellipsoid and the natural origin.
    fn near_conformal_arc(&self) -> MeridianSeries {
        let a = self.ellipsoid.semi_major_axis;
        let f = self.ellipsoid.flattening();
        let n = f / (2.0 - f);
        let [n2, n3, n4, n5] = [n.powi(2), n.powi(3), n.powi(4), n.powi(5)];
        let origin = Self::get(self.parameters.origin_latitude);
        let rho_0 = self.ellipsoid.meridian_radius(origin);
        let nu_0 = self.ellipsoid.prime_vertical_radius(origin);
        let mut arc = MeridianSeries {
            coefficients: [
                a * (1.0 - n + 5.0 * (n2 - n3) / 4.0 + 81.0 * (n4 - n5) / 64.0) * PI / 180.0,
                3.0 * a * (n - n2 + 7.0 * (n3 - n4) / 8.0 + 55.0 * n5 / 64.0) / 2.0,
                15.0 * a * (n2 - n3 + 3.0 * (n4 - n5) / 4.0) / 16.0,
                35.0 * a * (n3 - n4 + 11.0 * n5 / 16.0) / 48.0,
                315.0 * a * (n4 - n5) / 512.0,
            ],
            big_a: 1.0 / (6.0 * rho_0 * nu_0),
            r_0: Self::get(self.parameters.scale) * nu_0 / origin.tan(),
            s_0: 0.0,
        };
        arc.s_0 = arc.s(origin);
        arc
    }

    /// Section 3.1.1.6's forward formulas, `delta` being λ − λO, less the false easting and
    /// northing: `M = kO (m + A m³)` from `m = s – sO`, `r = rO – M`, `θ = (λ – λO) sin ϕO`, and
    /// `E = r sin θ`, `N = M + r sin θ tan(θ/2)`.
    pub(super) fn near_conformal_forward(&self, latitude: f64, delta: f64) -> (f64, f64) {
        let arc = self.near_conformal_arc();
        let k_0 = Self::get(self.parameters.scale);
        let m = arc.s(latitude) - arc.s_0;
        let big_m = k_0 * (m + arc.big_a * m.powi(3));
        let r = arc.r_0 - big_m;
        let theta = delta * Self::get(self.parameters.origin_latitude).sin();
        (
            r * theta.sin(),
            big_m + r * theta.sin() * (theta / 2.0).tan(),
        )
    }

    /// Section 3.1.1.6's reverse formulas, by the iteration it gives "\[i]f an exact solution is
    /// required", so that the reverse undoes the forward rather than the section's one-step
    /// solution, which it says is accurate to 0.001″ within five degrees of the origin.
    ///
    /// `θ′` takes `rO`'s sign as `r′` does. The section prints `θ′ = atan2{(E – FE), [rO – (N –
    /// FN)]}` and signs only `r′`; but `rO – (N – FN)` is `r cos θ` and `E – FE` is `r sin θ`, so
    /// south of the equator, where `r` is negative, the printed `atan2` answers `θ ± π` (ADR 3887).
    ///
    /// # Errors
    ///
    /// [`Refusal::NoConvergence`] where either iteration does not settle.
    pub(super) fn near_conformal(&self, x: f64, y: f64) -> Result<LatLon, Refusal> {
        let arc = self.near_conformal_arc();
        let k_0 = Self::get(self.parameters.scale);
        let origin = Self::get(self.parameters.origin_latitude);
        let sign = arc.r_0.signum();
        let theta = (sign * x).atan2(sign * (arc.r_0 - y));
        let r = sign * x.hypot(arc.r_0 - y);
        let big_m = arc.r_0 - r;
        // m′ = m′ – [M′ – kO m′ – kO A m′³] / [– kO – 3 kO A m′²], from m′ = M′.
        let m = self.settle(big_m, |m| {
            m - (big_m - k_0 * m - k_0 * arc.big_a * m.powi(3))
                / (-k_0 - 3.0 * k_0 * arc.big_a * m * m)
        })?;
        // ϕ′ = ϕ′ + [m′ + sO – s(ϕ′)] / A′ (π/180), from ϕ′ = ϕO + m′/A′ (π/180): the step divides
        // by the arc's mean derivative, `A′ (180/π)` metres a radian.
        let start = origin + (m / arc.coefficients[0]).to_radians();
        let latitude = self.settle(start, |latitude| {
            latitude + ((m + arc.s_0 - arc.s(latitude)) / arc.coefficients[0]).to_radians()
        })?;
        Ok(LatLon {
            latitude,
            longitude: Self::get(self.parameters.origin_longitude) + theta / origin.sin(),
        })
    }

    /// The fixed point of `step` from `start`, to [`CONVERGED`] in its own unit, or a refusal.
    fn settle(&self, start: f64, step: impl Fn(f64) -> f64) -> Result<f64, Refusal> {
        let mut value = start;
        for _ in 0..MAX_ITERATIONS {
            let next = step(value);
            if (next - value).abs() <= CONVERGED * value.abs().max(1.0) {
                return Ok(next);
            }
            value = next;
        }
        Err(Refusal::NoConvergence {
            method: self.method.name(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::super::cassini::meridian_distance;
    use super::super::{Method, Parameters};
    use super::*;
    use crate::geospatial::ellipsoid::Ellipsoid;

    /// Section 3.1.1.6's example, Deir ez Zor / Levant Zone, on Clarke 1880 (IGN).
    fn levant(origin: f64) -> Projection {
        let ellipsoid = Ellipsoid::new(6_378_249.2, 293.466_021_3).expect("Clarke 1880 (IGN)");
        let parameters = Parameters {
            origin_latitude: Some(origin.to_radians()),
            origin_longitude: Some((37.0_f64 + 21.0 / 60.0).to_radians()),
            scale: Some(0.999_625_6),
            false_easting: Some(300_000.0),
            false_northing: Some(300_000.0),
            ..Parameters::default()
        };
        Projection::new(Method::LambertConicNearConformal, ellipsoid, parameters).expect("it")
    }

    #[test]
    fn the_constants_are_the_examples() {
        let projection = levant(34.65);
        let arc = projection.near_conformal_arc();
        let within = |value: f64, printed: f64, unit: f64| {
            assert!((value - printed).abs() < unit, "{value} against {printed}");
        };
        within(arc.big_a, 4.106_749_4e-15, 1e-22);
        let [a, b, c, d, e] = arc.coefficients;
        within(a, 111_131.863_3, 1e-4);
        within(b, 16_300.644_07, 1e-5);
        within(c, 17.387_51, 1e-5);
        within(d, 0.023_08, 1e-5);
        within(e, 0.000_033, 1e-6);
        within(arc.s_0, 3_835_482.233, 1e-3);
        within(arc.r_0, 9_235_264.405, 1e-3);
        // The forward's intermediate values at 0.654874806 rad.
        let latitude = 0.654_874_806;
        let s = arc.s(latitude);
        within(s, 4_154_101.458, 2e-3);
        within(s - arc.s_0, 318_619.225, 2e-3);
        // The one-step reverse: m′ is the first Newton step from M′, and ϕ′ the arc's mean
        // derivative's estimate from it.
        let big_m = 318_632.717;
        let k_0 = 0.999_625_6;
        let m = big_m
            - (big_m - k_0 * big_m - k_0 * arc.big_a * big_m.powi(3))
                / (-k_0 - 3.0 * k_0 * arc.big_a * big_m.powi(2));
        within(m, 318_619.222, 2e-3);
        within(
            34.65_f64.to_radians() + (m / a).to_radians(),
            0.654_795_830,
            1e-9,
        );
    }

    #[test]
    fn the_arc_is_the_meridian_distance_of_section_3_2_2() {
        let projection = levant(34.65);
        let arc = projection.near_conformal_arc();
        let a = projection.ellipsoid.semi_major_axis;
        let e2 = projection.ellipsoid.eccentricity_squared();
        for degrees in [0.0_f64, 10.0, 30.0, 45.0, 60.0, 80.0] {
            let latitude = degrees.to_radians();
            let difference = arc.s(latitude) - meridian_distance(a, e2, latitude);
            assert!(difference.abs() < 1e-3, "{degrees}°: {difference} m");
        }
    }

    #[test]
    fn south_of_the_equator_the_angle_takes_the_cones_sign() {
        // The section's example mirrored: its `rO` is negative, and the reverse returns every
        // point on both sides of the central meridian.
        let projection = levant(-34.65);
        for (latitude, longitude) in [(-37.5_f64, 34.1_f64), (-32.0, 40.0), (-34.65, 37.35)] {
            let point = LatLon {
                latitude: latitude.to_radians(),
                longitude: longitude.to_radians(),
            };
            let grid = projection.forward(point).expect("a point");
            let back = projection
                .inverse(grid.easting, grid.northing)
                .expect("its grid point");
            assert!(
                (back.latitude - point.latitude).abs() < 1e-12
                    && (back.longitude - point.longitude).abs() < 1e-12,
                "{back:?} against {point:?}"
            );
        }
    }

    #[test]
    fn an_origin_on_the_equator_is_refused() {
        let ellipsoid = Ellipsoid::new(6_378_249.2, 293.466_021_3).expect("Clarke 1880 (IGN)");
        let parameters = Parameters {
            origin_latitude: Some(0.0),
            origin_longitude: Some(0.0),
            scale: Some(1.0),
            ..Parameters::default()
        };
        assert!(Projection::new(Method::LambertConicNearConformal, ellipsoid, parameters).is_err());
    }
}

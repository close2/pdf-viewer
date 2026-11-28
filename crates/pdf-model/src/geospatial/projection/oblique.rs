//! Hotine Oblique Mercator, variants A and B, EPSG methods 9812 and 9815, by Guidance Note 7-2
//! section 3.2.4.1's formulas.
//!
//! The method projects the ellipsoid conformally onto an aposphere and that onto a cylinder
//! touching it along the initial line, of azimuth αC through the projection centre (φC, λC). The
//! first coordinates computed, (u, v), are along and across that line from the natural origin,
//! where the line meets the aposphere's equator; the grid's easting and northing are (u, v)
//! rotated by γC, the angle from the rectified to the skew grid. The variants differ only in where
//! the false coordinates apply: variant A states them at the natural origin, variant B states
//! (EC, NC) at the projection centre, which is (uC, 0) — so both are an offset added after the
//! rotation, and [`super::Parameters::false_easting`] and `false_northing` carry either.
//!
//! **The azimuth of 90°, read as a choice (ADR 3871).** For variant B the section gives a special
//! case where αC is 90° (Hungary's EOV, the Swiss grid): `u = 0` where λ = λC, and otherwise
//! `u = A atan2{(S cos γO + V sin γO), cos[B (λ − λO)]} / B − |uC| SIGN(φC) SIGN(λC − λ)`. Read
//! literally, the last factor adds |uC| instead of subtracting it for every point east of λC,
//! where the `atan2` gives the same value as on the west side: a point a metre east of the Swiss
//! projection centre lands 20 000 km from it, while the section's own definition puts (EC, NC) at
//! the centre and the projection is continuous there. The general formula, which the section
//! states for every other azimuth, gives the continuous answer on both sides and agrees with the special
//! case's `u = 0` at λ = λC and with its formula west of it. So the general formula is evaluated
//! for every azimuth, the special case's `u = 0` is kept where λ = λC, and its `uC = A (λC − λO)`
//! is the one used at 90° (it equals the general formula's value there).

use std::f64::consts::{FRAC_PI_2, FRAC_PI_4};

use super::{LatLon, Method, Projection};

/// Section 3.2.4.1's constants of the projection, which both directions and both variants share.
struct Hotine {
    /// `B`, the aposphere's scale of longitude.
    b: f64,
    /// `A`, the aposphere's radius with the scale on the initial line applied.
    a: f64,
    /// `H = F tO^B`.
    h: f64,
    /// `γO`, the initial line's azimuth at the natural origin.
    gamma_0: f64,
    /// `λO`, the natural origin's longitude.
    lambda_0: f64,
    /// `uC`, the projection centre's distance along the initial line from the natural origin.
    u_c: f64,
}

/// Section 3.2.4.1's isometric function `t` of a latitude, as the forward case and `tO` state it.
fn isometric(e: f64, latitude: f64) -> f64 {
    let s = latitude.sin();
    (FRAC_PI_4 - latitude / 2.0).tan() / ((1.0 - e * s) / (1.0 + e * s)).powf(e / 2.0)
}

/// The section's SIGN, of a latitude or a longitude difference; zero takes the positive sign, as
/// the section's case of a centre on the equator has `D = 1` and so no term the sign multiplies.
fn sign(value: f64) -> f64 {
    if value < 0.0 { -1.0 } else { 1.0 }
}

impl Projection {
    /// Whether the azimuth of the initial line is the section's special case of 90 degrees.
    fn hotine_due_east(&self) -> bool {
        (Self::get(self.parameters.azimuth) - FRAC_PI_2).abs() < 1e-12
    }

    /// The constants, from the defining parameters (section 3.2.4.1).
    fn hotine(&self) -> Hotine {
        let p = &self.parameters;
        let (phi_c, lambda_c) = (Self::get(p.origin_latitude), Self::get(p.origin_longitude));
        let (alpha_c, k_c) = (Self::get(p.azimuth), Self::get(p.scale));
        let a = self.ellipsoid.semi_major_axis;
        let e = self.ellipsoid.eccentricity();
        let e2 = self.ellipsoid.eccentricity_squared();
        let (sin_c, cos_c) = (phi_c.sin(), phi_c.cos());
        let b = (1.0 + e2 * cos_c.powi(4) / (1.0 - e2)).sqrt();
        let big_a = a * b * k_c * (1.0 - e2).sqrt() / (1.0 - e2 * sin_c * sin_c);
        let t_0 = isometric(e, phi_c);
        let d = b * (1.0 - e2).sqrt() / (cos_c * (1.0 - e2 * sin_c * sin_c).sqrt());
        // The section's guard: where D < 1, D² is taken as 1, so that `(D² − 1)^0.5` is zero.
        let d2 = if d < 1.0 { 1.0 } else { d * d };
        let f = d + (d2 - 1.0).sqrt() * sign(phi_c);
        let h = f * t_0.powf(b);
        let g = (f - 1.0 / f) / 2.0;
        let gamma_0 = (alpha_c.sin() / d).asin();
        let lambda_0 = lambda_c - (g * gamma_0.tan()).asin() / b;
        let u_c = if self.hotine_due_east() {
            big_a * (lambda_c - lambda_0)
        } else {
            (big_a / b) * (d2 - 1.0).sqrt().atan2(alpha_c.cos()) * sign(phi_c)
        };
        Hotine {
            b,
            a: big_a,
            h,
            gamma_0,
            lambda_0,
            u_c,
        }
    }

    /// The offset variant B's `u` is measured from: zero for variant A, whose false coordinates
    /// are at the natural origin, and `|uC| SIGN(φC)` for variant B, whose are at the centre.
    fn hotine_u_offset(&self, constants: &Hotine) -> f64 {
        if self.method == Method::HotineObliqueMercatorB {
            constants.u_c.abs() * sign(Self::get(self.parameters.origin_latitude))
        } else {
            0.0
        }
    }

    /// Section 3.2.4.1's forward case, `delta` being λ − λC: the rectified skew coordinates less
    /// the false ones.
    ///
    /// A pole has `t = 0` and an infinite `Q`, and a point where `U` reaches ±1 an infinite `v`;
    /// both are non-finite answers the caller refuses.
    pub(super) fn hotine_forward(&self, latitude: f64, delta: f64) -> (f64, f64) {
        let constants = self.hotine();
        let Hotine {
            b,
            a,
            h,
            gamma_0,
            lambda_0,
            ..
        } = constants;
        let gamma_c = Self::get(self.parameters.rectified_grid_angle);
        let from_origin = delta + Self::get(self.parameters.origin_longitude) - lambda_0;
        let t = isometric(self.ellipsoid.eccentricity(), latitude);
        let q = h / t.powf(b);
        let s = (q - 1.0 / q) / 2.0;
        let big_t = f64::midpoint(q, 1.0 / q);
        let v_term = (b * from_origin).sin();
        let u_term = (-v_term * gamma_0.cos() + s * gamma_0.sin()) / big_t;
        let v = a * ((1.0 - u_term) / (1.0 + u_term)).ln() / (2.0 * b);
        let u = if self.method == Method::HotineObliqueMercatorB
            && self.hotine_due_east()
            && delta.abs() < f64::MIN_POSITIVE
        {
            0.0
        } else {
            a * (s * gamma_0.cos() + v_term * gamma_0.sin()).atan2((b * from_origin).cos()) / b
                - self.hotine_u_offset(&constants)
        };
        (
            v * gamma_c.cos() + u * gamma_c.sin(),
            u * gamma_c.cos() - v * gamma_c.sin(),
        )
    }

    /// Section 3.2.4.1's reverse case, from the easting and northing less the false ones.
    pub(super) fn hotine_inverse(&self, x: f64, y: f64) -> LatLon {
        let constants = self.hotine();
        let gamma_c = Self::get(self.parameters.rectified_grid_angle);
        let v = x * gamma_c.cos() - y * gamma_c.sin();
        let u = y * gamma_c.cos() + x * gamma_c.sin() + self.hotine_u_offset(&constants);
        let Hotine {
            b,
            a,
            h,
            gamma_0,
            lambda_0,
            ..
        } = constants;
        let q = (-(b * v / a)).exp();
        let s = (q - 1.0 / q) / 2.0;
        let big_t = f64::midpoint(q, 1.0 / q);
        let v_term = (b * u / a).sin();
        let u_term = (v_term * gamma_0.cos() + s * gamma_0.sin()) / big_t;
        let t = (h / ((1.0 + u_term) / (1.0 - u_term)).sqrt()).powf(1.0 / b);
        let chi = FRAC_PI_2 - 2.0 * t.atan();
        let e2 = self.ellipsoid.eccentricity_squared();
        let (e4, e6, e8) = (e2 * e2, e2 * e2 * e2, e2 * e2 * e2 * e2);
        let latitude = chi
            + (2.0 * chi).sin() * (e2 / 2.0 + 5.0 * e4 / 24.0 + e6 / 12.0 + 13.0 * e8 / 360.0)
            + (4.0 * chi).sin() * (7.0 * e4 / 48.0 + 29.0 * e6 / 240.0 + 811.0 * e8 / 11_520.0)
            + (6.0 * chi).sin() * (7.0 * e6 / 120.0 + 81.0 * e8 / 1120.0)
            + (8.0 * chi).sin() * (4279.0 * e8 / 161_280.0);
        LatLon {
            latitude,
            longitude: lambda_0
                - (s * gamma_0.cos() - v_term * gamma_0.sin()).atan2((b * u / a).cos()) / b,
        }
    }
}

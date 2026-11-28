//! Laborde Oblique Mercator, EPSG method 9813, by Guidance Note 7-2 section 3.2.4.2's formulas.
//!
//! Like Hotine's, a triple projection: the ellipsoid onto a conformal sphere (`B`, `C`, `ϕs`),
//! the sphere rotated so the initial line through the projection centre is its equator, and the
//! rotated sphere onto the plane by a Mercator whose complex coordinate `H` is corrected by the
//! cubic term `G H³` (ADR 3877).
//!
//! The section's longitudes are reduced to the Paris meridian. The method only ever takes the
//! difference `λ − λC`, so a system whose base counts longitude from Greenwich and states `λC`
//! from Greenwich (Tananarive / Laborde Grid, EPSG 8441) evaluates the same point as one that
//! counts both from Paris (EPSG 29701).
//!
//! The reverse solves `H + G H³ = H0` by the section's Newton step
//! `Hk+1 = (H0 + 2 G Hk³) / (3 G Hk² + 1)`, starting from `H0`. The section prints a first step
//! `H1 = H0 / (H0 + G H0³)`, which is not that step at `k = 0` and is near one rather than near
//! `H0`; ADR 3878 reads it, and the module's tests show both starts reach the same root.

use std::f64::consts::{FRAC_PI_2, FRAC_PI_4};

use super::{CONVERGED, LatLon, MAX_ITERATIONS, Projection};
use crate::geospatial::Refusal;

/// A complex number, the section's `H` and `G`, as a real and an imaginary part.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Complex {
    /// The real part.
    re: f64,
    /// The imaginary part.
    im: f64,
}

impl Complex {
    fn add(self, other: Self) -> Self {
        Self {
            re: self.re + other.re,
            im: self.im + other.im,
        }
    }

    fn sub(self, other: Self) -> Self {
        Self {
            re: self.re - other.re,
            im: self.im - other.im,
        }
    }

    fn mul(self, other: Self) -> Self {
        Self {
            re: self.re * other.re - self.im * other.im,
            im: self.re * other.im + self.im * other.re,
        }
    }

    fn div(self, other: Self) -> Self {
        let norm = other.re * other.re + other.im * other.im;
        Self {
            re: (self.re * other.re + self.im * other.im) / norm,
            im: (self.im * other.re - self.re * other.im) / norm,
        }
    }

    fn scale(self, factor: f64) -> Self {
        Self {
            re: self.re * factor,
            im: self.im * factor,
        }
    }

    fn modulus(self) -> f64 {
        self.re.hypot(self.im)
    }

    fn cube(self) -> Self {
        self.mul(self).mul(self)
    }
}

/// The constant term one, as a complex number.
const ONE: Complex = Complex { re: 1.0, im: 0.0 };

/// Section 3.2.4.2's constants of the projection.
struct Laborde {
    /// `B = {1 + [e² cos⁴ ϕC] / (1 – e²)}^0.5`.
    b: f64,
    /// `ϕs = asin[sin ϕC / B]`, the centre's latitude on the conformal sphere.
    phi_s: f64,
    /// `R = a kC (1 – e²)^0.5 / (1 – e² sin² ϕC)`.
    r: f64,
    /// `C`, the constant of the latitude's carriage onto the sphere.
    c: f64,
    /// `G = (1 – cos 2αC + i sin 2αC) / 12`.
    g: Complex,
}

/// The rotation's last step, common to both directions: from the rotated sphere's `(U, V, W)` to
/// a longitude and a latitude, `L′ = 2 atan(V / (U + d))` and `P′ = atan(W / d)` where
/// `d = (U² + V²)^0.5` is not zero, and `L′ = 0`, `P′ = sign(W) π/2` where it is.
fn spherical_angles(u: f64, v: f64, w: f64) -> (f64, f64) {
    let d = u.hypot(v);
    if d == 0.0 {
        (0.0, w.signum() * FRAC_PI_2)
    } else {
        (2.0 * (v / (u + d)).atan(), (w / d).atan())
    }
}

/// The Newton iteration for `H + G H³ = H0` from a first estimate, until the residual
/// `|H0 − Hk − G Hk³|` is within [`CONVERGED`]; `None` where it does not settle.
///
/// The section stops at `ABSOLUTE(REAL(H0 − Hk − G Hk³)) < 1E-11`; the modulus bounds the
/// imaginary part as well, and the tighter bound is the one every iteration here uses.
fn solve(h_0: Complex, g: Complex, start: Complex) -> Option<Complex> {
    let mut h = start;
    for _ in 0..MAX_ITERATIONS {
        if h_0.sub(h).sub(g.mul(h.cube())).modulus() < CONVERGED {
            return Some(h);
        }
        h = h_0
            .add(g.mul(h.cube()).scale(2.0))
            .div(g.mul(h.mul(h)).scale(3.0).add(ONE));
    }
    None
}

impl Projection {
    /// Section 3.2.4.2's constants, from the projection centre, the azimuth and the scale.
    fn laborde_constants(&self) -> Laborde {
        let p = &self.parameters;
        let a = self.ellipsoid.semi_major_axis;
        let e = self.ellipsoid.eccentricity();
        let e2 = e * e;
        let phi_c = Self::get(p.origin_latitude);
        let alpha_c = Self::get(p.azimuth);
        let b = (1.0 + e2 * phi_c.cos().powi(4) / (1.0 - e2)).sqrt();
        let phi_s = (phi_c.sin() / b).asin();
        let r = a * Self::get(p.scale) * (1.0 - e2).sqrt() / (1.0 - e2 * phi_c.sin().powi(2));
        let c = (FRAC_PI_4 + phi_s / 2.0).tan().ln() - b * self.laborde_isometric(phi_c);
        let g = Complex {
            re: 1.0 - (2.0 * alpha_c).cos(),
            im: (2.0 * alpha_c).sin(),
        }
        .scale(1.0 / 12.0);
        Laborde { b, phi_s, r, c, g }
    }

    /// `ln{tan(π/4 + ϕ/2) ([1 – e sin ϕ] / [1 + e sin ϕ])^(e/2)}`, the isometric latitude the
    /// section's `C` and `q` both take.
    fn laborde_isometric(&self, latitude: f64) -> f64 {
        let e = self.ellipsoid.eccentricity();
        let s = latitude.sin();
        (FRAC_PI_4 + latitude / 2.0).tan().ln() + e / 2.0 * ((1.0 - e * s) / (1.0 + e * s)).ln()
    }

    /// Section 3.2.4.2's forward case, `delta` being λ − λC, less the easting and northing at
    /// the projection centre.
    ///
    /// The point opposite the centre on the rotated sphere's equator has `U + d = 0` and no
    /// finite `L′`, and a pole of the rotated sphere has no finite `H`; the caller refuses the
    /// non-finite answer for both.
    pub(super) fn laborde_forward(&self, latitude: f64, delta: f64) -> (f64, f64) {
        let Laborde { b, phi_s, r, c, g } = self.laborde_constants();
        let l = b * delta;
        let q = c + b * self.laborde_isometric(latitude);
        let p = 2.0 * q.exp().atan() - FRAC_PI_2;
        let u = p.cos() * l.cos() * phi_s.cos() + p.sin() * phi_s.sin();
        let v = p.cos() * l.cos() * phi_s.sin() - p.sin() * phi_s.cos();
        let w = p.cos() * l.sin();
        let (l_prime, p_prime) = spherical_angles(u, v, w);
        let h = Complex {
            re: -l_prime,
            im: (FRAC_PI_4 + p_prime / 2.0).tan().ln(),
        };
        let projected = h.add(g.mul(h.cube()));
        (r * projected.im, r * projected.re)
    }

    /// Section 3.2.4.2's reverse case, from the easting and northing less those at the centre.
    pub(super) fn laborde(&self, x: f64, y: f64) -> Result<LatLon, Refusal> {
        let Laborde { b, phi_s, r, c, g } = self.laborde_constants();
        let h_0 = Complex {
            re: y / r,
            im: x / r,
        };
        let not_settled = || Refusal::NoConvergence {
            method: self.method.name(),
        };
        let h = solve(h_0, g, h_0).ok_or_else(not_settled)?;
        let l_prime = -h.re;
        let p_prime = 2.0 * h.im.exp().atan() - FRAC_PI_2;
        let u = p_prime.cos() * l_prime.cos() * phi_s.cos()
            + p_prime.cos() * l_prime.sin() * phi_s.sin();
        let v = p_prime.sin();
        let w = p_prime.cos() * l_prime.cos() * phi_s.sin()
            - p_prime.cos() * l_prime.sin() * phi_s.cos();
        let (l, p) = spherical_angles(u, v, w);
        let q_prime = ((FRAC_PI_4 + p / 2.0).tan().ln() - c) / b;
        let e = self.ellipsoid.eccentricity();
        let mut latitude = 2.0 * q_prime.exp().atan() - FRAC_PI_2;
        for _ in 0..MAX_ITERATIONS {
            let s = latitude.sin();
            let next = 2.0 * (((1.0 + e * s) / (1.0 - e * s)).powf(e / 2.0) * q_prime.exp()).atan()
                - FRAC_PI_2;
            let done = (next - latitude).abs() < CONVERGED;
            latitude = next;
            if done {
                return Ok(LatLon {
                    latitude,
                    longitude: Self::get(self.parameters.origin_longitude) + l / b,
                });
            }
        }
        Err(not_settled())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Section 3.2.4.2's example's `G` and `H0`, as printed.
    const G: Complex = Complex {
        re: 0.017_487_082,
        im: 0.051_075_588,
    };
    const H_0: Complex = Complex {
        re: 0.047_000_760,
        im: -0.033_290_167,
    };

    #[test]
    fn the_printed_first_step_and_the_newton_step_reach_the_same_root() {
        // The section's `H1 = H0 / (H0 + G H0³)`, printed as 0.999820949 − 0.000001503i.
        let printed = H_0.div(H_0.add(G.mul(H_0.cube())));
        assert!(
            (printed.re - 0.999_820_949).abs() < 1e-9 && (printed.im + 0.000_001_503).abs() < 1e-9,
            "{printed:?}"
        );
        // From H0 (the general step at k = 0) and from the printed H1, the root the example
        // prints, `Hk = 0.046992297 − 0.033284279i`.
        let from_h_0 = solve(H_0, G, H_0).expect("from H0");
        let from_printed = solve(H_0, G, printed).expect("from the printed H1");
        for root in [from_h_0, from_printed] {
            assert!(
                (root.re - 0.046_992_297).abs() < 1e-9 && (root.im + 0.033_284_279).abs() < 1e-9,
                "{root:?}"
            );
        }
        assert!(from_h_0.sub(from_printed).modulus() < 1e-14);
    }

    #[test]
    fn the_rotation_is_read_on_both_sides_of_d_equal_to_zero() {
        // `d = 0` is the rotated sphere's pole, `P′ = ±π/2`; just off it the general formulas
        // approach the same latitude.
        for w in [1.0, -1.0] {
            let (l, p) = spherical_angles(0.0, 0.0, w);
            assert_eq!((l, p), (0.0, w * FRAC_PI_2));
            let (_, near) = spherical_angles(1e-9, 1e-9, w);
            assert!((near - p).abs() < 1e-8, "{w}: {near}");
        }
    }
}

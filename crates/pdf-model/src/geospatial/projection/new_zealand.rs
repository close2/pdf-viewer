//! New Zealand Map Grid, EPSG method 9811.
//!
//! Guidance Note 7-2 section 3.4.1 prints no formulas: it describes the grid as a double
//! projection — a conformal projection of the ellipsoid, then a complex polynomial chosen by the
//! Cauchy-Riemann equations so that lines of constant scale follow the islands — and names Land
//! and Survey Technical Circular 1973/32 for the mathematics. The Dataset prints none either ("See
//! information source"). The formulas here are those of the Office of the Surveyor-General's
//! *Conversion between Latitude and Longitude and New Zealand Map Grid* (OSG Technical Report 4.2,
//! Land Information New Zealand, 21 November 2003), which states them and their coefficients for
//! NZGD49 and credits the projection to W. I. Reilly (1973); its three test points are the
//! method's tests (ADR 3887).
//!
//! The coefficients belong to the grid, not to the method's parameters: they are fitted to its
//! origin at 41°S 173°E on the International ellipsoid, which is NZGD49's. The parameters a system
//! states — the origin, the false easting and northing — are the ones the report's `φ0`, `λ0`, `E0`
//! and `N0` are, and the ellipsoid's semi-major axis its `a`.

use super::{LatLon, MAX_ITERATIONS, Projection, Refusal};

/// The report's `A1` to `A10`: `Δψ = Σ An Δφⁿ`, the isometric latitude's difference from the
/// origin's as a series in `Δφ`, the latitude's in units of 10⁵ arc-seconds.
const A: [f64; 10] = [
    0.639_917_507_3,
    -0.135_879_761_3,
    0.063_294_409,
    -0.025_268_53,
    0.011_787_9,
    -0.005_516_1,
    0.002_690_6,
    -0.001_333,
    0.000_67,
    -0.000_34,
];

/// The report's `B1` to `B6`, real and imaginary parts: `z = Σ Bn θⁿ`.
const B: [Complex; 6] = [
    Complex(0.755_785_322_8, 0.0),
    Complex(0.249_204_646, 0.003_371_507),
    Complex(-0.001_541_739, 0.041_058_560),
    Complex(-0.101_629_07, 0.017_276_09),
    Complex(-0.266_234_89, -0.362_492_18),
    Complex(-0.687_098_3, -1.165_196_7),
];

/// The report's `C1` to `C6`: the reverse's first approximation `θ0 = Σ Cn zⁿ`.
const C: [Complex; 6] = [
    Complex(1.323_127_043_9, 0.0),
    Complex(-0.577_245_789, -0.007_809_598),
    Complex(0.508_307_513, -0.112_208_952),
    Complex(-0.150_947_62, 0.182_006_02),
    Complex(1.014_181_79, 1.644_976_96),
    Complex(1.966_054_9, 2.512_764_5),
];

/// The report's `D1` to `D9`: `Δφ = Σ Dn Δψⁿ`.
const D: [f64; 9] = [
    1.562_701_424_3,
    0.518_540_639_8,
    -0.033_330_98,
    -0.105_290_6,
    -0.036_859_4,
    0.007_317,
    0.012_20,
    0.003_94,
    -0.001_3,
];

/// The unit of the report's `Δφ`: `Δφ = (φ – φ0) × 3600 × 10⁻⁵`, degrees to 10⁵ arc-seconds.
const DELTA_PHI_PER_DEGREE: f64 = 3600.0e-5;

/// Where the reverse's iteration has settled, in the unit of `θ`, radians of the conformal
/// sphere: 1e-15 is a hundredth of a micrometre on the grid.
const SETTLED: f64 = 1e-15;

/// A complex number, `re + i im`.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Complex(f64, f64);

impl Complex {
    /// The product of two.
    fn times(self, other: Self) -> Self {
        Self(
            self.0 * other.0 - self.1 * other.1,
            self.0 * other.1 + self.1 * other.0,
        )
    }

    /// The sum of two.
    fn plus(self, other: Self) -> Self {
        Self(self.0 + other.0, self.1 + other.1)
    }

    /// A real multiple.
    fn scaled(self, k: f64) -> Self {
        Self(self.0 * k, self.1 * k)
    }

    /// The quotient of two, refused (as `None`) where the divisor is zero.
    fn over(self, other: Self) -> Option<Self> {
        let norm = other.0 * other.0 + other.1 * other.1;
        (norm > 0.0).then(|| {
            Self(
                (self.0 * other.0 + self.1 * other.1) / norm,
                (self.1 * other.0 - self.0 * other.1) / norm,
            )
        })
    }
}

/// `Σ cn xⁿ` for `n` from 1, by Horner's rule.
fn real_series(coefficients: &[f64], x: f64) -> f64 {
    coefficients.iter().rev().fold(0.0, |sum, c| (sum + c) * x)
}

/// `Σ cn zⁿ` for `n` from 1, by Horner's rule.
fn complex_series(coefficients: &[Complex], z: Complex) -> Complex {
    coefficients
        .iter()
        .rev()
        .fold(Complex(0.0, 0.0), |sum, &c| sum.plus(c).times(z))
}

impl Projection {
    /// The report's section 2, `delta` being λ − λ0 in radians, less the false easting and
    /// northing: `θ = Δψ + iΔλ`, `z = Σ Bn θⁿ = x + iy`, and `E – E0 = y a`, `N – N0 = x a`.
    pub(super) fn nzmg_forward(&self, latitude: f64, delta: f64) -> (f64, f64) {
        let a = self.ellipsoid.semi_major_axis;
        let origin = Self::get(self.parameters.origin_latitude);
        let delta_phi = (latitude - origin).to_degrees() * DELTA_PHI_PER_DEGREE;
        let theta = Complex(real_series(&A, delta_phi), delta);
        let Complex(x, y) = complex_series(&B, theta);
        (y * a, x * a)
    }

    /// The report's section 3: `z = (N – N0)/a + i(E – E0)/a`, `θ0 = Σ Cn zⁿ`, then
    ///
    /// `θi+1 = [z + Σ (n – 1) Bn θiⁿ] / [Σ n Bn θiⁿ⁻¹]`,
    ///
    /// Newton's step on `Σ Bn θⁿ = z`, the first sum from `n = 2` and the second from 1. The report
    /// says two steps "will give millimetre accuracy"; this takes them until `θ` stops changing,
    /// so that the reverse undoes the forward, then `Δφ = Σ Dn Δψⁿ`.
    ///
    /// # Errors
    ///
    /// [`Refusal::NoConvergence`] where the iteration does not settle, and
    /// [`Refusal::OutsideDomain`] where its derivative vanishes.
    pub(super) fn nzmg(&self, x: f64, y: f64) -> Result<LatLon, Refusal> {
        let a = self.ellipsoid.semi_major_axis;
        let z = Complex(y / a, x / a);
        let mut theta = complex_series(&C, z);
        for _ in 0..MAX_ITERATIONS {
            let mut numerator = z;
            let mut denominator = Complex(0.0, 0.0);
            let mut power = Complex(1.0, 0.0);
            for (n, &b) in (1_u8..).zip(B.iter()) {
                let n = f64::from(n);
                // `power` is θⁿ⁻¹ here.
                denominator = denominator.plus(b.times(power).scaled(n));
                power = power.times(theta);
                numerator = numerator.plus(b.times(power).scaled(n - 1.0));
            }
            let next = numerator.over(denominator).ok_or_else(|| self.outside())?;
            let settled = (next.0 - theta.0).abs() < SETTLED && (next.1 - theta.1).abs() < SETTLED;
            theta = next;
            if settled {
                let delta_phi = real_series(&D, theta.0);
                return Ok(LatLon {
                    latitude: Self::get(self.parameters.origin_latitude)
                        + (delta_phi / DELTA_PHI_PER_DEGREE).to_radians(),
                    longitude: Self::get(self.parameters.origin_longitude) + theta.1,
                });
            }
        }
        Err(Refusal::NoConvergence {
            method: self.method.name(),
        })
    }
}

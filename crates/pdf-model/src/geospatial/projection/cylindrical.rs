//! The normal cylindrical methods of Guidance Note 7-2 sections 3.2.5 and 3.2.6: Equidistant
//! Cylindrical (EPSG 1028) and its spherical form (1029), and Lambert Cylindrical Equal Area
//! (9835) and its spherical form (9834).
//!
//! Section 3.2.6 prints no formulas for the ellipsoid. It says "See USGS Professional Paper 1395,
//! "Map Projections - A Working Manual" by John P. Snyder for formulas and example", so the
//! ellipsoidal equal-area formulas are that paper's normal aspect (equations 10-13 to 10-15 and
//! 3-12 forward, 10-26 and 10-27 back), and the latitude returns from the authalic one by the
//! paper's series 3-18, which section 3.3.2 prints too. The spherical forms take `R` as their
//! sections say: the authalic sphere's `RA` for the equal-area one, and the conformal sphere's
//! `RC` at ϕ1 for the equidistant one, where the figure is an ellipsoid.

use std::f64::consts::FRAC_PI_2;

use super::{LatLon, Projection};

/// Section 3.2.5's meridional arc `M / a` as a series in `e²`, to `e¹⁴`: row `j` holds the
/// coefficients of `e⁰` to `e¹⁴` multiplying ϕ (`j = 0`) or `sin 2jϕ`.
///
/// Each fraction is the section's printed one; every one equals the term of the same power in the
/// expansion of `(1 − e²) ∫ (1 − e² sin² ϕ)^(−3/2) dϕ`, the section's own equation (1).
const ARC: [[f64; 8]; 8] = [
    [
        1.0,
        -1.0 / 4.0,
        -3.0 / 64.0,
        -5.0 / 256.0,
        -175.0 / 16_384.0,
        -441.0 / 65_536.0,
        -4_851.0 / 1_048_576.0,
        -14_157.0 / 4_194_304.0,
    ],
    [
        0.0,
        -3.0 / 8.0,
        -3.0 / 32.0,
        -45.0 / 1_024.0,
        -105.0 / 4_096.0,
        -2_205.0 / 131_072.0,
        -6_237.0 / 524_288.0,
        -297_297.0 / 33_554_432.0,
    ],
    [
        0.0,
        0.0,
        15.0 / 256.0,
        45.0 / 1_024.0,
        525.0 / 16_384.0,
        1_575.0 / 65_536.0,
        155_925.0 / 8_388_608.0,
        495_495.0 / 33_554_432.0,
    ],
    [
        0.0,
        0.0,
        0.0,
        -35.0 / 3_072.0,
        -175.0 / 12_288.0,
        -3_675.0 / 262_144.0,
        -13_475.0 / 1_048_576.0,
        -385_385.0 / 33_554_432.0,
    ],
    [
        0.0,
        0.0,
        0.0,
        0.0,
        315.0 / 131_072.0,
        2_205.0 / 524_288.0,
        43_659.0 / 8_388_608.0,
        189_189.0 / 33_554_432.0,
    ],
    [
        0.0,
        0.0,
        0.0,
        0.0,
        0.0,
        -693.0 / 1_310_720.0,
        -6_237.0 / 5_242_880.0,
        -297_297.0 / 167_772_160.0,
    ],
    [
        0.0,
        0.0,
        0.0,
        0.0,
        0.0,
        0.0,
        1_001.0 / 8_388_608.0,
        11_011.0 / 33_554_432.0,
    ],
    [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, -6_435.0 / 234_881_024.0],
];

/// Section 3.2.5's reverse series in the third flattening `n`, to `n⁷`: row `j − 1` holds the
/// coefficients of `n⁰` to `n⁷` multiplying `sin 2jµ`, as the section prints them.
const FOOTPOINT: [[f64; 8]; 7] = [
    [
        0.0,
        3.0 / 2.0,
        0.0,
        -27.0 / 32.0,
        0.0,
        269.0 / 512.0,
        0.0,
        -6_607.0 / 24_576.0,
    ],
    [
        0.0,
        0.0,
        21.0 / 16.0,
        0.0,
        -55.0 / 32.0,
        0.0,
        6_759.0 / 4_096.0,
        0.0,
    ],
    [
        0.0,
        0.0,
        0.0,
        151.0 / 96.0,
        0.0,
        -417.0 / 128.0,
        0.0,
        87_963.0 / 20_480.0,
    ],
    [
        0.0,
        0.0,
        0.0,
        0.0,
        1_097.0 / 512.0,
        0.0,
        -15_543.0 / 2_560.0,
        0.0,
    ],
    [
        0.0,
        0.0,
        0.0,
        0.0,
        0.0,
        8_011.0 / 2_560.0,
        0.0,
        -69_119.0 / 6_144.0,
    ],
    [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 293_393.0 / 61_440.0, 0.0],
    [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 6_845_701.0 / 860_160.0],
];

/// A polynomial's value, its coefficients in ascending powers of `x`.
fn polynomial(coefficients: &[f64; 8], x: f64) -> f64 {
    coefficients.iter().rev().fold(0.0, |sum, c| sum * x + c)
}

/// The multiple `2j` of the angle in row `j` of a series.
const MULTIPLE: [f64; 8] = [0.0, 2.0, 4.0, 6.0, 8.0, 10.0, 12.0, 14.0];

impl Projection {
    /// The standard parallel ϕ1, required by [`Projection::new`].
    fn standard_parallel(&self) -> f64 {
        Self::get(self.parameters.parallel_1)
    }

    /// Professional Paper 1395's `k0 = cos ϕs / (1 − e² sin² ϕs)^½` (equation 10-13).
    fn cylindrical_scale(&self) -> f64 {
        let parallel = self.standard_parallel();
        let e2 = self.ellipsoid.eccentricity_squared();
        parallel.cos() / (1.0 - e2 * parallel.sin().powi(2)).sqrt()
    }

    /// The ellipsoidal equal-area forward formulas (equations 10-14 and 10-15), `delta` being
    /// λ − λ0, less the false easting and northing.
    pub(super) fn cylindrical_equal_area_forward(&self, latitude: f64, delta: f64) -> (f64, f64) {
        let a = self.ellipsoid.semi_major_axis;
        let k_0 = self.cylindrical_scale();
        (
            a * k_0 * delta,
            a * self.albers_alpha(latitude) / (2.0 * k_0),
        )
    }

    /// The ellipsoidal equal-area reverse formulas: `β = asin[2 y k0 / (a qp)]` (10-26), the
    /// latitude by series 3-18, and `λ = λ0 + x / (a k0)` (10-27).
    ///
    /// A northing past the pole's makes the `asin` argument exceed one, and the non-finite answer
    /// is refused by the caller.
    pub(super) fn cylindrical_equal_area(&self, x: f64, y: f64) -> LatLon {
        let a = self.ellipsoid.semi_major_axis;
        let k_0 = self.cylindrical_scale();
        let beta = (2.0 * y * k_0 / (a * self.albers_alpha(FRAC_PI_2))).asin();
        LatLon {
            latitude: self.geodetic_of_authalic(beta),
            longitude: Self::get(self.parameters.origin_longitude) + x / (a * k_0),
        }
    }

    /// Section 3.2.6.1's forward formulas, `E = FE + R (λ – λO) cos(ϕ1)` and
    /// `N = FN + R sin(ϕ) / cos(ϕ1)`, less the false easting and northing.
    pub(super) fn spherical_cylindrical_equal_area_forward(
        &self,
        latitude: f64,
        delta: f64,
    ) -> (f64, f64) {
        let r = self.spherical_radius();
        let parallel = self.standard_parallel();
        (
            r * delta * parallel.cos(),
            r * latitude.sin() / parallel.cos(),
        )
    }

    /// Section 3.2.6.1's reverse formulas, `ϕ = asin{[(N – FN) / R] cos(ϕ1)}` and
    /// `λ = λO + {[E – FE] / [R cos(ϕ1)]}`.
    pub(super) fn spherical_cylindrical_equal_area(&self, x: f64, y: f64) -> LatLon {
        let r = self.spherical_radius();
        let parallel = self.standard_parallel();
        LatLon {
            latitude: (y / r * parallel.cos()).asin(),
            longitude: Self::get(self.parameters.origin_longitude) + x / (r * parallel.cos()),
        }
    }

    /// Section 3.2.5's meridional arc `M` from the equator to a latitude, by its series.
    fn equidistant_arc(&self, latitude: f64) -> f64 {
        let e2 = self.ellipsoid.eccentricity_squared();
        let series = polynomial(&ARC[0], e2) * latitude
            + ARC
                .iter()
                .zip(MULTIPLE)
                .skip(1)
                .map(|(coefficients, multiple)| {
                    polynomial(coefficients, e2) * (multiple * latitude).sin()
                })
                .sum::<f64>();
        self.ellipsoid.semi_major_axis * series
    }

    /// Section 3.2.5's `ν1 cos ϕ1`, the radius of the standard parallel.
    fn parallel_radius(&self) -> f64 {
        let parallel = self.standard_parallel();
        let e2 = self.ellipsoid.eccentricity_squared();
        self.ellipsoid.semi_major_axis * parallel.cos() / (1.0 - e2 * parallel.sin().powi(2)).sqrt()
    }

    /// Section 3.2.5's forward formulas, `E = FE + ν1 cos ϕ1 (λ – λO)` and `N = FN + M`, less
    /// the false easting and northing.
    pub(super) fn equidistant_forward(&self, latitude: f64, delta: f64) -> (f64, f64) {
        (
            self.parallel_radius() * delta,
            self.equidistant_arc(latitude),
        )
    }

    /// Section 3.2.5's reverse formulas: `λ = λO + X / (ν1 cos ϕ1)`, and ϕ by the series in `n`
    /// from the rectifying latitude `µ = Y / [a (1 − e²/4 − …)]`, the arc series' first row.
    pub(super) fn equidistant(&self, x: f64, y: f64) -> LatLon {
        let e2 = self.ellipsoid.eccentricity_squared();
        let root = (1.0 - e2).sqrt();
        let n = (1.0 - root) / (1.0 + root);
        let mu = y / (self.ellipsoid.semi_major_axis * polynomial(&ARC[0], e2));
        let latitude = mu
            + FOOTPOINT
                .iter()
                .zip(MULTIPLE.iter().skip(1))
                .map(|(coefficients, multiple)| polynomial(coefficients, n) * (multiple * mu).sin())
                .sum::<f64>();
        LatLon {
            latitude,
            longitude: Self::get(self.parameters.origin_longitude) + x / self.parallel_radius(),
        }
    }

    /// Section 3.2.5.1's `R`: the sphere's radius, or on an ellipsoid the conformal sphere's at
    /// ϕ1, the implementation notes' `RC = a √(1 – e²) / (1 – e² sin² ϕ)`.
    fn equidistant_sphere_radius(&self) -> f64 {
        let parallel = self.standard_parallel();
        let e2 = self.ellipsoid.eccentricity_squared();
        self.ellipsoid.semi_major_axis * (1.0 - e2).sqrt() / (1.0 - e2 * parallel.sin().powi(2))
    }

    /// Section 3.2.5.1's forward formulas, `E = FE + R (λ – λO) cos(ϕ1)` and `N = FN + R ϕ`,
    /// less the false easting and northing.
    pub(super) fn spherical_equidistant_forward(&self, latitude: f64, delta: f64) -> (f64, f64) {
        let r = self.equidistant_sphere_radius();
        (r * delta * self.standard_parallel().cos(), r * latitude)
    }

    /// Section 3.2.5.1's reverse formulas, `ϕ = (N – FN) / R` and
    /// `λ = λO + {[E – FE] / [R cos(ϕ1)]}`.
    ///
    /// A northing past the pole's gives a latitude past it, which the caller refuses.
    pub(super) fn spherical_equidistant(&self, x: f64, y: f64) -> LatLon {
        let r = self.equidistant_sphere_radius();
        LatLon {
            latitude: y / r,
            longitude: Self::get(self.parameters.origin_longitude)
                + x / (r * self.standard_parallel().cos()),
        }
    }
}

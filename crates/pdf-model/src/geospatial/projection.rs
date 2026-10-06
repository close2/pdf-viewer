//! The inverse map projections: an easting and a northing back to a latitude and a longitude of
//! the same geographic system, each method's formulas as IOGP Guidance Note 7-2 states them.
//!
//! §12.10.4's projected system "specifies the algorithms and associated parameters used to
//! transform points between geographic coordinates and a two-dimensional (projected) coordinate
//! system", and names no algorithm itself: the WKT string or the EPSG code does, and the formulas
//! for each named method are IOGP's. So every method here cites the section of Guidance Note 7-2
//! whose reverse formulas it evaluates, and its tests are that section's own worked example
//! (ADR 1587). Only the *reverse* direction is built: §12.10 asks where a projected point is on
//! the earth, and never the other way.
//!
//! The method set is the census's, not a catalogue: the methods the documents of the crawl name,
//! from Transverse Mercator (85 documents) to the oblique stereographic (3). A method the census
//! found but this module does not carry — Hotine's oblique Mercator, Krovak, the vertical
//! perspective — is refused by its name, as is every other (ADR 1586).
//!
//! Angles in and out are radians and lengths metres; [`super::system`] converts the file's units
//! on the way in.

#![expect(
    clippy::many_single_char_names,
    reason = "the formulas are Guidance Note 7-2's, and its symbols are the names a reader checks \
              them against"
)]

use std::f64::consts::{FRAC_PI_2, FRAC_PI_4};

use super::Refusal;
use super::ellipsoid::Ellipsoid;

/// How many times an iterated latitude is refined before the answer is refused.
///
/// Guidance Note 7-2 expects its iterations to settle in three or four steps (section 3.1.1.1)
/// and prints four (section 3.2.3.1); a bound well past that is a guard against an input
/// outside the method's domain rather than a precision setting.
const MAX_ITERATIONS: usize = 32;

/// When an iterated latitude has stopped changing, in radians — 1e-14 rad is 64 nm on the earth,
/// far inside ADR 1587's budget, and above `f64`'s own resolution of a latitude near one radian.
const CONVERGED: f64 = 1e-14;

/// A projection method this module evaluates, with the section of Guidance Note 7-2 it follows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Method {
    /// Transverse Mercator, EPSG method 9807, by the JHS formulas Guidance Note 7-2 section
    /// 3.2.3.1 recommends.
    TransverseMercator,
    /// Lambert Conic Conformal (2SP), EPSG method 9802, Guidance Note 7-2 section 3.1.1.1.
    LambertConicConformal2Sp,
    /// Lambert Conic Conformal (1SP), EPSG method 9801, Guidance Note 7-2 section 3.1.1.2.
    LambertConicConformal1Sp,
    /// Mercator variant A, the one-standard-parallel form, EPSG method 9804, Guidance Note 7-2
    /// section 3.2.1.
    MercatorVariantA,
    /// Mercator variant B, the two-standard-parallel form, EPSG method 9805, Guidance Note 7-2
    /// section 3.2.1.
    MercatorVariantB,
    /// Popular Visualisation Pseudo-Mercator, EPSG method 1024, Guidance Note 7-2 section
    /// 3.2.1.2 — Web Mercator.
    PseudoMercator,
    /// Albers Equal Area, EPSG method 9822, Guidance Note 7-2 section 3.1.3.
    AlbersEqualArea,
    /// Oblique and Equatorial Stereographic, EPSG method 9809, Guidance Note 7-2 section 3.3.1.1.
    ObliqueStereographic,
}

impl Method {
    /// The method's name in Guidance Note 7-2, which is what a refusal or a host prints.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::TransverseMercator => "Transverse Mercator",
            Self::LambertConicConformal2Sp => "Lambert Conic Conformal (2SP)",
            Self::LambertConicConformal1Sp => "Lambert Conic Conformal (1SP)",
            Self::MercatorVariantA => "Mercator (variant A)",
            Self::MercatorVariantB => "Mercator (variant B)",
            Self::PseudoMercator => "Popular Visualisation Pseudo-Mercator",
            Self::AlbersEqualArea => "Albers Equal Area",
            Self::ObliqueStereographic => "Oblique Stereographic",
        }
    }
}

/// A method's defining parameters, in radians and metres, by the role each plays.
///
/// One struct for every method because the roles recur: an origin (natural or false, as the
/// method defines it), a scale factor, the false coordinates of that origin, and up to two
/// standard parallels. Which roles a method needs is [`Projection::new`]'s to check.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Parameters {
    /// The latitude of the natural origin, or of the false origin for the methods defined at
    /// one (the two-parallel Lambert and Albers).
    pub origin_latitude: Option<f64>,
    /// The longitude of the same origin.
    pub origin_longitude: Option<f64>,
    /// The scale factor at the natural origin.
    pub scale: Option<f64>,
    /// The false easting, or the easting at the false origin.
    pub false_easting: Option<f64>,
    /// The false northing, or the northing at the false origin.
    pub false_northing: Option<f64>,
    /// The latitude of the first standard parallel.
    pub parallel_1: Option<f64>,
    /// The latitude of the second standard parallel.
    pub parallel_2: Option<f64>,
}

/// A projection ready to evaluate: a method, its ellipsoid and its parameters, with the
/// constants the method's reverse formulas need computed once.
#[derive(Debug, Clone, PartialEq)]
pub struct Projection {
    /// The method.
    pub method: Method,
    /// The ellipsoid of the base geographic system.
    pub ellipsoid: Ellipsoid,
    /// The parameters as given.
    pub parameters: Parameters,
}

/// The multiples of the angle in the four terms of section 3.2.3.1's series: 2ξ, 4ξ, 6ξ, 8ξ.
const MULTIPLES: [f64; 4] = [2.0, 4.0, 6.0, 8.0];

/// A latitude and a longitude in radians.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LatLon {
    /// The latitude φ.
    pub latitude: f64,
    /// The longitude λ, east positive, wrapped into −π..π (Guidance Note 7-2 section 1.4).
    pub longitude: f64,
}

/// A required parameter, or a refusal naming it.
fn required(value: Option<f64>, method: Method, parameter: &'static str) -> Result<f64, Refusal> {
    value.ok_or(Refusal::MissingParameter {
        method: method.name(),
        parameter,
    })
}

/// `atanh` of a value the formulas keep inside −1..1 for every point on the ellipsoid.
fn atanh(x: f64) -> f64 {
    0.5 * ((1.0 + x) / (1.0 - x)).ln()
}

/// Guidance Note 7-2 section 1.4's wrap of a longitude into −π..π.
fn wrap(longitude: f64) -> f64 {
    let wrapped =
        (longitude + std::f64::consts::PI).rem_euclid(std::f64::consts::TAU) - std::f64::consts::PI;
    // `rem_euclid` maps +π to −π; a longitude stated as +180° keeps its sign.
    if wrapped <= -std::f64::consts::PI && longitude > 0.0 {
        std::f64::consts::PI
    } else {
        wrapped
    }
}

impl Projection {
    /// A projection, refused where the method's formulas need a parameter the system does not
    /// state.
    ///
    /// # Errors
    ///
    /// [`Refusal::MissingParameter`] naming the first parameter missing, and
    /// [`Refusal::Unsupported`] for the one case a method's section leaves open — the oblique
    /// stereographic centred south of the equator (see [`Self::inverse`]).
    pub fn new(
        method: Method,
        ellipsoid: Ellipsoid,
        parameters: Parameters,
    ) -> Result<Self, Refusal> {
        let p = &parameters;
        match method {
            Method::TransverseMercator
            | Method::ObliqueStereographic
            | Method::LambertConicConformal1Sp => {
                required(p.origin_latitude, method, "latitude of natural origin")?;
                required(p.origin_longitude, method, "longitude of natural origin")?;
                required(p.scale, method, "scale factor at natural origin")?;
            }
            Method::LambertConicConformal2Sp | Method::AlbersEqualArea => {
                required(p.origin_latitude, method, "latitude of false origin")?;
                required(p.origin_longitude, method, "longitude of false origin")?;
                required(p.parallel_1, method, "latitude of 1st standard parallel")?;
                required(p.parallel_2, method, "latitude of 2nd standard parallel")?;
            }
            Method::MercatorVariantA => {
                required(p.origin_longitude, method, "longitude of natural origin")?;
                required(p.scale, method, "scale factor at natural origin")?;
            }
            Method::MercatorVariantB => {
                required(p.origin_longitude, method, "longitude of natural origin")?;
                required(p.parallel_1, method, "latitude of 1st standard parallel")?;
            }
            Method::PseudoMercator => {
                required(p.origin_longitude, method, "longitude of natural origin")?;
            }
        }
        if method == Method::ObliqueStereographic && p.origin_latitude.is_some_and(|v| v < 0.0) {
            return Err(Refusal::Unsupported {
                method: method.name(),
                reason: "a stereographic projection centred south of the equator",
            });
        }
        Ok(Self {
            method,
            ellipsoid,
            parameters,
        })
    }

    /// The latitude and longitude of an easting and a northing, in metres.
    ///
    /// The false easting and northing default to zero where the system states neither: every
    /// method's formulas subtract them, and an absent offset is no offset. Every other
    /// parameter a method uses was required by [`Self::new`].
    ///
    /// # Errors
    ///
    /// [`Refusal::NoConvergence`] where an iterated latitude does not settle within
    /// [`MAX_ITERATIONS`], and [`Refusal::OutsideDomain`] where the point is one the method's
    /// formulas do not reach — a non-finite answer is refused, never returned.
    pub fn inverse(&self, easting: f64, northing: f64) -> Result<LatLon, Refusal> {
        let p = &self.parameters;
        let x = easting - p.false_easting.unwrap_or(0.0);
        let y = northing - p.false_northing.unwrap_or(0.0);
        let answer = match self.method {
            Method::TransverseMercator => self.transverse_mercator(x, y)?,
            Method::LambertConicConformal2Sp => self.lambert_2sp(x, y)?,
            Method::LambertConicConformal1Sp => self.lambert_1sp(x, y)?,
            Method::MercatorVariantA | Method::MercatorVariantB => self.mercator(x, y),
            Method::PseudoMercator => self.pseudo_mercator(x, y),
            Method::AlbersEqualArea => self.albers(x, y),
            Method::ObliqueStereographic => self.oblique_stereographic(x, y)?,
        };
        if answer.latitude.is_finite()
            && answer.longitude.is_finite()
            && answer.latitude.abs() <= FRAC_PI_2 + 1e-12
        {
            Ok(LatLon {
                latitude: answer.latitude.clamp(-FRAC_PI_2, FRAC_PI_2),
                longitude: wrap(answer.longitude),
            })
        } else {
            Err(Refusal::OutsideDomain {
                method: self.method.name(),
            })
        }
    }

    /// A parameter [`Self::new`] has already required.
    fn get(value: Option<f64>) -> f64 {
        // `new` refused every projection whose method's formulas use a parameter it lacks, so
        // the zero is never read; it is not a default for any parameter.
        value.unwrap_or(0.0)
    }

    /// Guidance Note 7-2 section 3.2.3.1, the JHS reverse formulas.
    fn transverse_mercator(&self, x: f64, y: f64) -> Result<LatLon, Refusal> {
        let p = &self.parameters;
        let (latitude_0, longitude_0, k0) = (
            Self::get(p.origin_latitude),
            Self::get(p.origin_longitude),
            Self::get(p.scale),
        );
        let a = self.ellipsoid.semi_major_axis;
        let f = self.ellipsoid.flattening();
        let e = self.ellipsoid.eccentricity();
        let n = f / (2.0 - f);
        let (n2, n3, n4) = (n * n, n * n * n, n * n * n * n);
        let b = a / (1.0 + n) * (1.0 + n2 / 4.0 + n4 / 64.0);

        // The meridional arc to the origin, MO, by the forward constants.
        let h = [
            n / 2.0 - 2.0 / 3.0 * n2 + 5.0 / 16.0 * n3 + 41.0 / 180.0 * n4,
            13.0 / 48.0 * n2 - 3.0 / 5.0 * n3 + 557.0 / 1440.0 * n4,
            61.0 / 240.0 * n3 - 103.0 / 140.0 * n4,
            49561.0 / 161_280.0 * n4,
        ];
        let m0 = if latitude_0 == 0.0 {
            0.0
        } else if (latitude_0.abs() - FRAC_PI_2).abs() < f64::EPSILON {
            b * latitude_0
        } else {
            let q0 = latitude_0.tan().asinh() - e * atanh(e * latitude_0.sin());
            // The section's own simplification: ξO0 = βO = atan(sinh QO).
            let xi_00 = q0.sinh().atan();
            let xi_0 = xi_00
                + MULTIPLES
                    .iter()
                    .zip(h)
                    .map(|(multiple, coefficient)| coefficient * (multiple * xi_00).sin())
                    .sum::<f64>();
            b * xi_0
        };

        let h_reverse = [
            n / 2.0 - 2.0 / 3.0 * n2 + 37.0 / 96.0 * n3 - 1.0 / 360.0 * n4,
            1.0 / 48.0 * n2 + 1.0 / 15.0 * n3 - 437.0 / 1440.0 * n4,
            17.0 / 480.0 * n3 - 37.0 / 840.0 * n4,
            4397.0 / 161_280.0 * n4,
        ];
        let eta_prime = x / (b * k0);
        let xi_prime = (y + k0 * m0) / (b * k0);
        let (mut xi_sum, mut eta_sum) = (0.0, 0.0);
        for (&twice, coefficient) in MULTIPLES.iter().zip(h_reverse) {
            xi_sum += coefficient * (twice * xi_prime).sin() * (twice * eta_prime).cosh();
            eta_sum += coefficient * (twice * xi_prime).cos() * (twice * eta_prime).sinh();
        }
        let xi_0 = xi_prime - xi_sum;
        let eta_0 = eta_prime - eta_sum;
        let beta = (xi_0.sin() / eta_0.cosh()).asin();
        let q_prime = beta.tan().asinh();
        let mut q = q_prime;
        let mut converged = false;
        for _ in 0..MAX_ITERATIONS {
            let next = q_prime + e * atanh(e * q.tanh());
            let done = (next - q).abs() < CONVERGED;
            q = next;
            if done {
                converged = true;
                break;
            }
        }
        if !converged {
            return Err(Refusal::NoConvergence {
                method: self.method.name(),
            });
        }
        Ok(LatLon {
            latitude: q.sinh().atan(),
            longitude: longitude_0 + (eta_0.tanh() / beta.cos()).asin(),
        })
    }

    /// `m` and `t` of Guidance Note 7-2 section 3.1.1.1, at one latitude.
    fn lambert_m_t(&self, latitude: f64) -> (f64, f64) {
        let e = self.ellipsoid.eccentricity();
        let e2 = self.ellipsoid.eccentricity_squared();
        let s = latitude.sin();
        let m = latitude.cos() / (1.0 - e2 * s * s).sqrt();
        let t = (FRAC_PI_4 - latitude / 2.0).tan() / ((1.0 - e * s) / (1.0 + e * s)).powf(e / 2.0);
        (m, t)
    }

    /// The common tail of both Lambert variants: θ′ and t′ to a latitude, by section 3.1.1.1's
    /// iteration, which the section expects to settle in three or four steps.
    fn lambert_latitude(&self, t_prime: f64) -> Result<f64, Refusal> {
        let e = self.ellipsoid.eccentricity();
        let mut latitude = FRAC_PI_2 - 2.0 * t_prime.atan();
        for _ in 0..MAX_ITERATIONS {
            let s = latitude.sin();
            let next =
                FRAC_PI_2 - 2.0 * (t_prime * ((1.0 - e * s) / (1.0 + e * s)).powf(e / 2.0)).atan();
            let done = (next - latitude).abs() < CONVERGED;
            latitude = next;
            if done {
                return Ok(latitude);
            }
        }
        Err(Refusal::NoConvergence {
            method: self.method.name(),
        })
    }

    /// Guidance Note 7-2 section 3.1.1.1's reverse formulas.
    ///
    /// The section gives r′ the sign of n; the same sign is applied to θ′'s two
    /// arguments, because the forward formulas make `E − EF = r sin θ` and `rF − (N − NF) =
    /// r cos θ` with `r` of n's sign, and an `atan2` of two arguments both negated is θ ± π.
    /// The northern cone (`n > 0`) is unchanged by it, and the section's worked example is one.
    fn lambert_2sp(&self, x: f64, y: f64) -> Result<LatLon, Refusal> {
        let p = &self.parameters;
        let a = self.ellipsoid.semi_major_axis;
        let (m1, t1) = self.lambert_m_t(Self::get(p.parallel_1));
        let (m2, t2) = self.lambert_m_t(Self::get(p.parallel_2));
        let (_, t_f) = self.lambert_m_t(Self::get(p.origin_latitude));
        let n = (m1.ln() - m2.ln()) / (t1.ln() - t2.ln());
        let big_f = m1 / (n * t1.powf(n));
        let r_f = a * big_f * t_f.powf(n);
        let sign = n.signum();
        let r_prime = sign * (x * x + (r_f - y).powi(2)).sqrt();
        let t_prime = (r_prime / (a * big_f)).powf(1.0 / n);
        let theta = (sign * x).atan2(sign * (r_f - y));
        Ok(LatLon {
            latitude: self.lambert_latitude(t_prime)?,
            longitude: theta / n + Self::get(p.origin_longitude),
        })
    }

    /// Guidance Note 7-2 section 3.1.1.2's reverse formulas, the 2SP tail with `n = sin φO`, the
    /// natural origin, and the scale factor.
    fn lambert_1sp(&self, x: f64, y: f64) -> Result<LatLon, Refusal> {
        let p = &self.parameters;
        let a = self.ellipsoid.semi_major_axis;
        let k0 = Self::get(p.scale);
        let latitude_0 = Self::get(p.origin_latitude);
        let (m0, t0) = self.lambert_m_t(latitude_0);
        let n = latitude_0.sin();
        let big_f = m0 / (n * t0.powf(n));
        let r0 = a * big_f * t0.powf(n) * k0;
        let sign = n.signum();
        let r_prime = sign * (x * x + (r0 - y).powi(2)).sqrt();
        let t_prime = (r_prime / (a * k0 * big_f)).powf(1.0 / n);
        let theta = (sign * x).atan2(sign * (r0 - y));
        Ok(LatLon {
            latitude: self.lambert_latitude(t_prime)?,
            longitude: theta / n + Self::get(p.origin_longitude),
        })
    }

    /// Guidance Note 7-2 section 3.2.1's reverse formulas for variants A and B: the series in χ
    /// for latitude, which needs no iteration.
    fn mercator(&self, x: f64, y: f64) -> LatLon {
        let p = &self.parameters;
        let a = self.ellipsoid.semi_major_axis;
        let e2 = self.ellipsoid.eccentricity_squared();
        let k0 = if self.method == Method::MercatorVariantB {
            // The section takes ϕ1 as the absolute value of the first standard parallel.
            let parallel = Self::get(p.parallel_1).abs();
            parallel.cos() / (1.0 - e2 * parallel.sin().powi(2)).sqrt()
        } else {
            Self::get(p.scale)
        };
        let t = (-y / (a * k0)).exp();
        let chi = FRAC_PI_2 - 2.0 * t.atan();
        let (e4, e6, e8) = (e2 * e2, e2 * e2 * e2, e2 * e2 * e2 * e2);
        let latitude = chi
            + (e2 / 2.0 + 5.0 * e4 / 24.0 + e6 / 12.0 + 13.0 * e8 / 360.0) * (2.0 * chi).sin()
            + (7.0 * e4 / 48.0 + 29.0 * e6 / 240.0 + 811.0 * e8 / 11520.0) * (4.0 * chi).sin()
            + (7.0 * e6 / 120.0 + 81.0 * e8 / 1120.0) * (6.0 * chi).sin()
            + (4279.0 * e8 / 161_280.0) * (8.0 * chi).sin();
        LatLon {
            latitude,
            longitude: x / (a * k0) + Self::get(p.origin_longitude),
        }
    }

    /// Guidance Note 7-2 section 3.2.1.2's reverse formulas: the sphere's, on the ellipsoid's
    /// semi-major axis, which is what makes the method a separate one.
    fn pseudo_mercator(&self, x: f64, y: f64) -> LatLon {
        let a = self.ellipsoid.semi_major_axis;
        let d = -y / a;
        LatLon {
            latitude: FRAC_PI_2 - 2.0 * d.exp().atan(),
            longitude: x / a + Self::get(self.parameters.origin_longitude),
        }
    }

    /// α of Guidance Note 7-2 section 3.1.3 at one latitude, with its spherical limit `2 sin φ`
    /// where the eccentricity is zero and the formula's `1/(2e)` has no value.
    fn albers_alpha(&self, latitude: f64) -> f64 {
        let e = self.ellipsoid.eccentricity();
        let e2 = self.ellipsoid.eccentricity_squared();
        let s = latitude.sin();
        if e == 0.0 {
            return 2.0 * s;
        }
        (1.0 - e2)
            * (s / (1.0 - e2 * s * s) - 1.0 / (2.0 * e) * ((1.0 - e * s) / (1.0 + e * s)).ln())
    }

    /// Guidance Note 7-2 section 3.1.3's reverse formulas: the authalic latitude and the series
    /// from it, which needs no iteration. θ takes n's sign for section 3.1.1.1's reason.
    fn albers(&self, x: f64, y: f64) -> LatLon {
        let p = &self.parameters;
        let a = self.ellipsoid.semi_major_axis;
        let e = self.ellipsoid.eccentricity();
        let e2 = self.ellipsoid.eccentricity_squared();
        let m = |latitude: f64| latitude.cos() / (1.0 - e2 * latitude.sin().powi(2)).sqrt();
        let (phi_1, phi_2) = (Self::get(p.parallel_1), Self::get(p.parallel_2));
        let (m1, m2) = (m(phi_1), m(phi_2));
        let (alpha_1, alpha_2) = (self.albers_alpha(phi_1), self.albers_alpha(phi_2));
        let alpha_0 = self.albers_alpha(Self::get(p.origin_latitude));
        let n = (m1 * m1 - m2 * m2) / (alpha_2 - alpha_1);
        let c = m1 * m1 + n * alpha_1;
        let rho_0 = a * (c - n * alpha_0).sqrt() / n;
        let rho = (x * x + (rho_0 - y).powi(2)).sqrt();
        let sign = n.signum();
        let theta = (sign * x).atan2(sign * (rho_0 - y));
        let alpha_prime = (c - rho * rho * n * n / (a * a)) / n;
        let denominator = if e == 0.0 {
            2.0
        } else {
            1.0 - (1.0 - e2) / (2.0 * e) * ((1.0 - e) / (1.0 + e)).ln()
        };
        let beta = (alpha_prime / denominator).clamp(-1.0, 1.0).asin();
        let (e4, e6) = (e2 * e2, e2 * e2 * e2);
        let latitude = beta
            + (e2 / 3.0 + 31.0 * e4 / 180.0 + 517.0 * e6 / 5040.0) * (2.0 * beta).sin()
            + (23.0 * e4 / 360.0 + 251.0 * e6 / 3780.0) * (4.0 * beta).sin()
            + (761.0 * e6 / 45360.0) * (6.0 * beta).sin();
        LatLon {
            latitude,
            longitude: Self::get(p.origin_longitude) + theta / n,
        }
    }

    /// Guidance Note 7-2 section 3.3.1.1's reverse formulas: through the conformal sphere to the
    /// isometric latitude, then the section's iteration back to the ellipsoid.
    fn oblique_stereographic(&self, x: f64, y: f64) -> Result<LatLon, Refusal> {
        let p = &self.parameters;
        let e = self.ellipsoid.eccentricity();
        let e2 = self.ellipsoid.eccentricity_squared();
        let (latitude_0, longitude_0, k0) = (
            Self::get(p.origin_latitude),
            Self::get(p.origin_longitude),
            Self::get(p.scale),
        );
        let s0 = latitude_0.sin();
        let r = (self.ellipsoid.meridian_radius(latitude_0)
            * self.ellipsoid.prime_vertical_radius(latitude_0))
        .sqrt();
        let n = (1.0 + e2 * latitude_0.cos().powi(4) / (1.0 - e2)).sqrt();
        let s1 = (1.0 + s0) / (1.0 - s0);
        let s2 = (1.0 - e * s0) / (1.0 + e * s0);
        let w1 = (s1 * s2.powf(e)).powf(n);
        let sin_chi_00 = (w1 - 1.0) / (w1 + 1.0);
        let c = (n + s0) * (1.0 - sin_chi_00) / ((n - s0) * (1.0 + sin_chi_00));
        let w2 = c * w1;
        let chi_0 = ((w2 - 1.0) / (w2 + 1.0)).asin();
        let big_lambda_0 = longitude_0;

        let g = 2.0 * r * k0 * (FRAC_PI_4 - chi_0 / 2.0).tan();
        let h = 4.0 * r * k0 * chi_0.tan() + g;
        let i = x.atan2(h + y);
        let j = x.atan2(g - y) - i;
        let chi = chi_0 + 2.0 * ((y - x * (j / 2.0).tan()) / (2.0 * r * k0)).atan();
        let big_lambda = j + 2.0 * i + big_lambda_0;
        let longitude = (big_lambda - big_lambda_0) / n + big_lambda_0;
        let sin_chi = chi.sin();
        let psi = 0.5 * ((1.0 + sin_chi) / (c * (1.0 - sin_chi))).ln() / n;

        let mut latitude = 2.0 * psi.exp().atan() - FRAC_PI_2;
        for _ in 0..MAX_ITERATIONS {
            let s = latitude.sin();
            let psi_i = ((latitude / 2.0 + FRAC_PI_4).tan()
                * ((1.0 - e * s) / (1.0 + e * s)).powf(e / 2.0))
            .ln();
            let next = latitude - (psi_i - psi) * latitude.cos() * (1.0 - e2 * s * s) / (1.0 - e2);
            let done = (next - latitude).abs() < CONVERGED;
            latitude = next;
            if done {
                return Ok(LatLon {
                    latitude,
                    longitude,
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
    use super::*;

    /// Degrees, minutes and seconds to radians, signed by the hemisphere.
    fn dms(degrees: f64, minutes: f64, seconds: f64) -> f64 {
        let sign = if degrees < 0.0 { -1.0 } else { 1.0 };
        sign * (degrees.abs() + minutes / 60.0 + seconds / 3600.0).to_radians()
    }

    /// ADR 1587's budget, applied to an example that prints seconds to `places` decimals: the
    /// answer is within half the last printed digit.
    fn assert_within(answer: f64, expected: f64, places: i32) {
        let half_digit = (0.5 * 10f64.powi(places).recip() / 3600.0).to_radians();
        assert!(
            (answer - expected).abs() <= half_digit,
            "{answer} against {expected}: off by {}\"",
            (answer - expected).abs().to_degrees() * 3600.0
        );
    }

    #[test]
    fn transverse_mercator_reproduces_the_british_national_grid_example() {
        // Guidance Note 7-2 section 3.2.3.1's example, OSGB 1936 / British National Grid, JHS
        // formulas: (577274.99 E, 69740.50 N) is 50°30′00.000″N 00°30′00.000″E.
        let projection = Projection::new(
            Method::TransverseMercator,
            Ellipsoid::new(6_377_563.396, 299.324_96).expect("Airy 1830"),
            Parameters {
                origin_latitude: Some(dms(49.0, 0.0, 0.0)),
                origin_longitude: Some(-dms(2.0, 0.0, 0.0)),
                scale: Some(0.999_601_271_7),
                false_easting: Some(400_000.0),
                false_northing: Some(-100_000.0),
                ..Parameters::default()
            },
        )
        .expect("every parameter stated");
        let answer = projection.inverse(577_274.99, 69_740.50).expect("a point");
        assert_within(answer.latitude, dms(50.0, 30.0, 0.0), 3);
        assert_within(answer.longitude, dms(0.0, 30.0, 0.0), 3);
    }

    #[test]
    fn lambert_2sp_reproduces_the_texas_south_central_example_in_us_survey_feet() {
        // Guidance Note 7-2 section 3.1.1.1's example, NAD27 / Texas South Central: the
        // example's grid is in US survey feet, so its numbers are converted to metres here as
        // `system` converts a file's, by the foot's definition of 1200/3937 m.
        let foot = 1200.0 / 3937.0;
        let projection = Projection::new(
            Method::LambertConicConformal2Sp,
            Ellipsoid::new(6_378_206.400, 294.978_70).expect("Clarke 1866"),
            Parameters {
                origin_latitude: Some(dms(27.0, 50.0, 0.0)),
                origin_longitude: Some(-dms(99.0, 0.0, 0.0)),
                parallel_1: Some(dms(28.0, 23.0, 0.0)),
                parallel_2: Some(dms(30.0, 17.0, 0.0)),
                false_easting: Some(2_000_000.0 * foot),
                false_northing: Some(0.0),
                ..Parameters::default()
            },
        )
        .expect("every parameter stated");
        let answer = projection
            .inverse(2_963_503.91 * foot, 254_759.80 * foot)
            .expect("a point");
        assert_within(answer.latitude, dms(28.0, 30.0, 0.0), 3);
        assert_within(answer.longitude, -dms(96.0, 0.0, 0.0), 3);
    }

    #[test]
    fn lambert_1sp_reproduces_the_jamaica_national_grid_example() {
        // Guidance Note 7-2 section 3.1.1.2's example, JAD69 / Jamaica National Grid:
        // (255966.58 E, 142493.51 N) is 17°55′55.80″N 76°56′37.26″W.
        let projection = Projection::new(
            Method::LambertConicConformal1Sp,
            Ellipsoid::new(6_378_206.400, 294.978_70).expect("Clarke 1866"),
            Parameters {
                origin_latitude: Some(dms(18.0, 0.0, 0.0)),
                origin_longitude: Some(-dms(77.0, 0.0, 0.0)),
                scale: Some(1.0),
                false_easting: Some(250_000.0),
                false_northing: Some(150_000.0),
                ..Parameters::default()
            },
        )
        .expect("every parameter stated");
        let answer = projection.inverse(255_966.58, 142_493.51).expect("a point");
        assert_within(answer.latitude, dms(17.0, 55.0, 55.80), 2);
        assert_within(answer.longitude, -dms(76.0, 56.0, 37.26), 2);
    }

    #[test]
    fn mercator_variant_a_reproduces_the_makassar_example() {
        // Guidance Note 7-2 section 3.2.1's first example, Makassar / NEIEZ: (5009726.58 E,
        // 569150.82 N) is 3°00′00.000″S 120°00′00.000″E.
        let projection = Projection::new(
            Method::MercatorVariantA,
            Ellipsoid::new(6_377_397.155, 299.152_81).expect("Bessel 1841"),
            Parameters {
                origin_longitude: Some(dms(110.0, 0.0, 0.0)),
                scale: Some(0.997),
                false_easting: Some(3_900_000.0),
                false_northing: Some(900_000.0),
                ..Parameters::default()
            },
        )
        .expect("every parameter stated");
        let answer = projection
            .inverse(5_009_726.58, 569_150.82)
            .expect("a point");
        assert_within(answer.latitude, -dms(3.0, 0.0, 0.0), 3);
        assert_within(answer.longitude, dms(120.0, 0.0, 0.0), 3);
    }

    #[test]
    fn mercator_variant_b_reproduces_the_caspian_sea_example() {
        // Guidance Note 7-2 section 3.2.1's second example, Pulkovo 1942 / Caspian Sea
        // Mercator: (165704.29 E, 5171848.07 N) is 53°00′00.000″N 53°00′00.000″E.
        let projection = Projection::new(
            Method::MercatorVariantB,
            Ellipsoid::new(6_378_245.0, 298.3).expect("Krassowsky 1940"),
            Parameters {
                origin_longitude: Some(dms(51.0, 0.0, 0.0)),
                parallel_1: Some(dms(42.0, 0.0, 0.0)),
                false_easting: Some(0.0),
                false_northing: Some(0.0),
                ..Parameters::default()
            },
        )
        .expect("every parameter stated");
        let answer = projection
            .inverse(165_704.29, 5_171_848.07)
            .expect("a point");
        assert_within(answer.latitude, dms(53.0, 0.0, 0.0), 3);
        assert_within(answer.longitude, dms(53.0, 0.0, 0.0), 3);
    }

    #[test]
    fn pseudo_mercator_reproduces_the_web_mercator_example() {
        // Guidance Note 7-2 section 3.2.1.2's reverse example, WGS 84 / Pseudo-Mercator, a
        // point ten kilometres north on the grid: (–11169055.58 E, 2810000.00 N) is 24°27′48.889″N
        // 100°20′00.000″W.
        let projection = Projection::new(
            Method::PseudoMercator,
            Ellipsoid::new(6_378_137.0, 298.257_223_6).expect("WGS 84"),
            Parameters {
                origin_longitude: Some(0.0),
                ..Parameters::default()
            },
        )
        .expect("every parameter stated");
        let answer = projection
            .inverse(-11_169_055.58, 2_810_000.00)
            .expect("a point");
        assert_within(answer.latitude, dms(24.0, 27.0, 48.889), 3);
        assert_within(answer.longitude, -dms(100.0, 20.0, 0.0), 3);
    }

    #[test]
    fn oblique_stereographic_reproduces_the_rd_new_example() {
        // Guidance Note 7-2 section 3.3.1.1's example, Amersfoort / RD New: (196105.28 E,
        // 557057.74 N) is 53°00′00.000″N 6°00′00.000″E.
        let projection = Projection::new(
            Method::ObliqueStereographic,
            Ellipsoid::new(6_377_397.155, 299.152_81).expect("Bessel 1841"),
            Parameters {
                origin_latitude: Some(dms(52.0, 9.0, 22.178)),
                origin_longitude: Some(dms(5.0, 23.0, 15.500)),
                scale: Some(0.999_907_9),
                false_easting: Some(155_000.0),
                false_northing: Some(463_000.0),
                ..Parameters::default()
            },
        )
        .expect("every parameter stated");
        let answer = projection.inverse(196_105.28, 557_057.74).expect("a point");
        assert_within(answer.latitude, dms(53.0, 0.0, 0.0), 3);
        assert_within(answer.longitude, dms(6.0, 0.0, 0.0), 3);
    }

    /// Guidance Note 7-2 section 3.1.3's forward formulas, which the section states and prints
    /// no example for: the Albers inverse is tested against them, at the parameters of
    /// ISO 32000-2 §12.10.4's own EXAMPLE 2.
    fn albers_forward(projection: &Projection, latitude: f64, longitude: f64) -> (f64, f64) {
        let p = &projection.parameters;
        let a = projection.ellipsoid.semi_major_axis;
        let e2 = projection.ellipsoid.eccentricity_squared();
        let m = |phi: f64| phi.cos() / (1.0 - e2 * phi.sin().powi(2)).sqrt();
        let (phi_1, phi_2) = (p.parallel_1.expect("φ1"), p.parallel_2.expect("φ2"));
        let (alpha, alpha_0) = (
            projection.albers_alpha(latitude),
            projection.albers_alpha(p.origin_latitude.expect("φO")),
        );
        let (alpha_1, alpha_2) = (
            projection.albers_alpha(phi_1),
            projection.albers_alpha(phi_2),
        );
        let n = (m(phi_1).powi(2) - m(phi_2).powi(2)) / (alpha_2 - alpha_1);
        let c = m(phi_1).powi(2) + n * alpha_1;
        let theta = n * (longitude - p.origin_longitude.expect("λO"));
        let rho = a * (c - n * alpha).sqrt() / n;
        let rho_0 = a * (c - n * alpha_0).sqrt() / n;
        (rho * theta.sin(), rho_0 - rho * theta.cos())
    }

    #[test]
    fn albers_inverts_the_sections_own_forward_formulas_at_the_clauses_example() {
        // §12.10.4's EXAMPLE 2: GRS 1980, standard parallels 20°N and 60°N, origin 40°N 96°W.
        let projection = Projection::new(
            Method::AlbersEqualArea,
            Ellipsoid::new(6_378_137.0, 298.257_222_101).expect("GRS 1980"),
            Parameters {
                origin_latitude: Some(40f64.to_radians()),
                origin_longitude: Some((-96f64).to_radians()),
                parallel_1: Some(20f64.to_radians()),
                parallel_2: Some(60f64.to_radians()),
                false_easting: Some(0.0),
                false_northing: Some(0.0),
                ..Parameters::default()
            },
        )
        .expect("every parameter stated");
        for (latitude, longitude) in [
            (35.0_f64, -75.0_f64),
            (50.0, -120.0),
            (25.5, -96.0),
            (65.0, -60.0),
        ] {
            let (x, y) = albers_forward(&projection, latitude.to_radians(), longitude.to_radians());
            let answer = projection.inverse(x, y).expect("a point");
            // The section's series for latitude is truncated at e⁶; ADR 1587's budget is a
            // thousandth of a second.
            assert_within(answer.latitude, latitude.to_radians(), 3);
            assert_within(answer.longitude, longitude.to_radians(), 3);
        }
    }

    #[test]
    fn a_southern_cone_inverts_its_own_forward_formulas() {
        // The sign `lambert_2sp` applies to θ′: a cone with both parallels south of the equator,
        // checked against Guidance Note 7-2 section 3.1.1.1's forward formulas.
        let projection = Projection::new(
            Method::LambertConicConformal2Sp,
            Ellipsoid::new(6_378_137.0, 298.257_222_101).expect("GRS 1980"),
            Parameters {
                origin_latitude: Some((-32f64).to_radians()),
                origin_longitude: Some(135f64.to_radians()),
                parallel_1: Some((-28f64).to_radians()),
                parallel_2: Some((-36f64).to_radians()),
                false_easting: Some(1_000_000.0),
                false_northing: Some(2_000_000.0),
                ..Parameters::default()
            },
        )
        .expect("every parameter stated");
        let (latitude, longitude) = ((-30f64).to_radians(), 140f64.to_radians());
        let a = projection.ellipsoid.semi_major_axis;
        let (m1, t1) = projection.lambert_m_t((-28f64).to_radians());
        let (m2, t2) = projection.lambert_m_t((-36f64).to_radians());
        let (_, t_f) = projection.lambert_m_t((-32f64).to_radians());
        let (_, t) = projection.lambert_m_t(latitude);
        let n = (m1.ln() - m2.ln()) / (t1.ln() - t2.ln());
        let big_f = m1 / (n * t1.powf(n));
        let (r, r_f) = (a * big_f * t.powf(n), a * big_f * t_f.powf(n));
        let theta = n * (longitude - 135f64.to_radians());
        let easting = 1_000_000.0 + r * theta.sin();
        let northing = 2_000_000.0 + r_f - r * theta.cos();
        let answer = projection.inverse(easting, northing).expect("a point");
        assert_within(answer.latitude, latitude, 4);
        assert_within(answer.longitude, longitude, 4);
    }

    #[test]
    fn a_method_without_its_parameters_is_refused_by_name() {
        let refused = Projection::new(
            Method::TransverseMercator,
            Ellipsoid::new(6_378_137.0, 298.257_223_563).expect("WGS 84"),
            Parameters {
                origin_longitude: Some(0.0),
                scale: Some(0.9996),
                ..Parameters::default()
            },
        );
        assert_eq!(
            refused,
            Err(Refusal::MissingParameter {
                method: "Transverse Mercator",
                parameter: "latitude of natural origin"
            })
        );
    }

    #[test]
    fn a_longitude_wraps_into_the_guidance_notes_range() {
        assert!((wrap(3.5) - (3.5 - std::f64::consts::TAU)).abs() < 1e-15);
        assert!((wrap(-3.5) - (std::f64::consts::TAU - 3.5)).abs() < 1e-15);
        assert!((wrap(std::f64::consts::PI) - std::f64::consts::PI).abs() < 1e-15);
        assert!((wrap(0.25) - 0.25).abs() < 1e-15);
    }
}

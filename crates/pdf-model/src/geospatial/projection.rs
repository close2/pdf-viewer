//! The map projections in both directions: a latitude and a longitude to an easting and a
//! northing of the same system, and back, each method's formulas as IOGP Guidance Note 7-2
//! states them.
//!
//! §12.10.4's projected system "specifies the algorithms and associated parameters used to
//! transform points between geographic coordinates and a two-dimensional (projected) coordinate
//! system", and names no algorithm itself: the WKT string or the EPSG code does, and the formulas
//! for each named method are IOGP's. So every method here cites the section of Guidance Note 7-2
//! whose formulas it evaluates, and its tests are that section's own worked example (ADRs 1587 and
//! 1672). The clause's *between* runs both ways and so does this module: [`Projection::inverse`]
//! is what a registration point or `/PCSM`'s position needs to become a latitude, and
//! [`Projection::forward`] is what a position needs to reach a projected `/DCS` — the system Table
//! 269 says "shall be used for the display of position values" — or to be carried back through
//! `/PCSM` onto the page.
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

/// How close to a pole a latitude is the pole, in radians — 1e-12 rad is 6 µm on the earth.
///
/// A conic method's far pole and both of Mercator's lie at infinity, and `f64`'s `tan` of the
/// nearest double to π/2 is a large finite number rather than infinity, so without this the
/// forward formulas would return a finite grid point for a place no grid holds.
const AT_THE_POLE: f64 = 1e-12;

/// The latitude poleward of which Guidance Note 7-2 section 3.2.1.2 says the Pseudo-Mercator's
/// northing formula fails and is not to be used: 88°.
const PSEUDO_MERCATOR_LIMIT: f64 = 88.0 * std::f64::consts::PI / 180.0;

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

/// A projection ready to evaluate: a method, its ellipsoid and its parameters.
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

/// An easting and a northing in metres, the false coordinates of the origin included.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GridPoint {
    /// The easting E.
    pub easting: f64,
    /// The northing N.
    pub northing: f64,
}

/// Section 3.2.3.1's JHS constants, which both directions of the Transverse Mercator share.
struct Jhs {
    /// The third flattening, `n = f / (2 − f)`.
    n: f64,
    /// `B`, the series' radius.
    b: f64,
    /// The forward series' coefficients h1 to h4.
    h: [f64; 4],
    /// The meridional arc from the equator to the origin, `MO`.
    m0: f64,
}

/// Section 3.3.1.1's conformal sphere, which both directions of the stereographic share.
struct ConformalSphere {
    /// Its radius, `R = (ρO νO)^0.5`.
    r: f64,
    /// `n`, the longitude's scale onto the sphere.
    n: f64,
    /// `c`, the latitude's constant onto the sphere.
    c: f64,
    /// The origin's conformal latitude, `χO`.
    chi_0: f64,
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
///
/// The section states it for a longitude's difference from the origin's (λ − λO), which is what
/// every forward formula takes, and the inverse applies it to the longitude it returns.
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

    /// The refusal for a point the method's formulas do not reach.
    fn outside(&self) -> Refusal {
        Refusal::OutsideDomain {
            method: self.method.name(),
        }
    }

    /// The easting and northing of a latitude and a longitude, in metres.
    ///
    /// The false easting and northing default to zero where the system states neither, as in
    /// [`Self::inverse`]; the longitude's difference from the origin's is wrapped into −π..π as
    /// section 1.4 says, so a map across the antimeridian projects as one piece.
    ///
    /// # Errors
    ///
    /// [`Refusal::OutsideDomain`] for a latitude past a pole, a non-finite input, and a point the
    /// method sends to infinity or folds onto another — Mercator's poles, a cone's far pole, the
    /// stereographic's antipode, a Transverse Mercator point more than 90° from the central
    /// meridian (whose formulas' `asin` answers for the mirror point instead), and the
    /// Pseudo-Mercator poleward of 88°, where section 3.2.1.2 says its formula is not to be used.
    pub fn forward(&self, point: LatLon) -> Result<GridPoint, Refusal> {
        let LatLon {
            latitude,
            longitude,
        } = point;
        if !latitude.is_finite() || !longitude.is_finite() || latitude.abs() > FRAC_PI_2 + 1e-12 {
            return Err(self.outside());
        }
        let latitude = latitude.clamp(-FRAC_PI_2, FRAC_PI_2);
        let p = &self.parameters;
        let delta = wrap(longitude - Self::get(p.origin_longitude));
        let (x, y) = match self.method {
            Method::TransverseMercator => self.transverse_mercator_forward(latitude, delta)?,
            Method::LambertConicConformal2Sp => {
                let (n, big_f, r_f) = self.lambert_2sp_cone();
                let a = self.ellipsoid.semi_major_axis;
                self.cone_forward(latitude, delta, n, a * big_f, r_f)?
            }
            Method::LambertConicConformal1Sp => {
                let (n, big_f, r0) = self.lambert_1sp_cone();
                let a = self.ellipsoid.semi_major_axis;
                let k0 = Self::get(p.scale);
                self.cone_forward(latitude, delta, n, a * big_f * k0, r0)?
            }
            Method::MercatorVariantA | Method::MercatorVariantB => {
                self.mercator_forward(latitude, delta)?
            }
            Method::PseudoMercator => self.pseudo_mercator_forward(latitude, delta)?,
            Method::AlbersEqualArea => self.albers_forward(latitude, delta),
            Method::ObliqueStereographic => self.oblique_stereographic_forward(latitude, delta)?,
        };
        let easting = x + p.false_easting.unwrap_or(0.0);
        let northing = y + p.false_northing.unwrap_or(0.0);
        if easting.is_finite() && northing.is_finite() {
            Ok(GridPoint { easting, northing })
        } else {
            Err(self.outside())
        }
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
    /// formulas do not reach — a non-finite answer is refused, never returned, and so is a
    /// Pseudo-Mercator latitude poleward of 88°, section 3.2.1.2's limit in this direction too.
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
        let within_method =
            self.method != Method::PseudoMercator || answer.latitude.abs() <= PSEUDO_MERCATOR_LIMIT;
        if answer.latitude.is_finite()
            && answer.longitude.is_finite()
            && answer.latitude.abs() <= FRAC_PI_2 + 1e-12
            && within_method
        {
            Ok(LatLon {
                latitude: answer.latitude.clamp(-FRAC_PI_2, FRAC_PI_2),
                longitude: wrap(answer.longitude),
            })
        } else {
            Err(self.outside())
        }
    }

    /// A parameter [`Self::new`] has already required.
    fn get(value: Option<f64>) -> f64 {
        // `new` refused every projection whose method's formulas use a parameter it lacks, so
        // the zero is never read; it is not a default for any parameter.
        value.unwrap_or(0.0)
    }

    /// Section 3.2.3.1's constants of the projection, which its forward and reverse formulas
    /// both begin from.
    fn jhs(&self) -> Jhs {
        let latitude_0 = Self::get(self.parameters.origin_latitude);
        let a = self.ellipsoid.semi_major_axis;
        let f = self.ellipsoid.flattening();
        let e = self.ellipsoid.eccentricity();
        let n = f / (2.0 - f);
        let (n2, n3, n4) = (n * n, n * n * n, n * n * n * n);
        let b = a / (1.0 + n) * (1.0 + n2 / 4.0 + n4 / 64.0);
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
        Jhs { n, b, h, m0 }
    }

    /// Guidance Note 7-2 section 3.2.3.1, the JHS forward formulas.
    ///
    /// A point more than 90° of longitude from the central meridian is refused: there
    /// `cos β sin(λ − λO)` takes the value of the point mirrored across the 90° meridian, and the
    /// formulas' `asin` returns that mirror's grid position rather than this point's.
    fn transverse_mercator_forward(
        &self,
        latitude: f64,
        delta: f64,
    ) -> Result<(f64, f64), Refusal> {
        if delta.abs() > FRAC_PI_2 {
            return Err(self.outside());
        }
        let k0 = Self::get(self.parameters.scale);
        let e = self.ellipsoid.eccentricity();
        let Jhs { b, h, m0, .. } = self.jhs();
        let q = latitude.tan().asinh() - e * atanh(e * latitude.sin());
        let beta = q.sinh().atan();
        let eta_0 = atanh(beta.cos() * delta.sin());
        // `sin β cosh η0` is at most one within 90° of the central meridian; the clamp keeps a
        // rounding past it from becoming a NaN.
        let xi_0 = (beta.sin() * eta_0.cosh()).clamp(-1.0, 1.0).asin();
        let (mut xi, mut eta) = (xi_0, eta_0);
        for (&multiple, coefficient) in MULTIPLES.iter().zip(h) {
            xi += coefficient * (multiple * xi_0).sin() * (multiple * eta_0).cosh();
            eta += coefficient * (multiple * xi_0).cos() * (multiple * eta_0).sinh();
        }
        Ok((k0 * b * eta, k0 * (b * xi - m0)))
    }

    /// Guidance Note 7-2 section 3.2.3.1, the JHS reverse formulas.
    fn transverse_mercator(&self, x: f64, y: f64) -> Result<LatLon, Refusal> {
        let p = &self.parameters;
        let (longitude_0, k0) = (Self::get(p.origin_longitude), Self::get(p.scale));
        let e = self.ellipsoid.eccentricity();
        let Jhs { n, b, m0, .. } = self.jhs();
        let (n2, n3, n4) = (n * n, n * n * n, n * n * n * n);
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

    /// Section 3.1.1.1's cone: `n`, `F` and `rF`, the radius of the false origin's parallel.
    fn lambert_2sp_cone(&self) -> (f64, f64, f64) {
        let p = &self.parameters;
        let a = self.ellipsoid.semi_major_axis;
        let (m1, t1) = self.lambert_m_t(Self::get(p.parallel_1));
        let (m2, t2) = self.lambert_m_t(Self::get(p.parallel_2));
        let (_, t_f) = self.lambert_m_t(Self::get(p.origin_latitude));
        let n = (m1.ln() - m2.ln()) / (t1.ln() - t2.ln());
        let big_f = m1 / (n * t1.powf(n));
        (n, big_f, a * big_f * t_f.powf(n))
    }

    /// Section 3.1.1.2's cone: `n = sin φO`, `F`, and `rO`, the radius of the natural origin's
    /// parallel with the scale factor applied.
    fn lambert_1sp_cone(&self) -> (f64, f64, f64) {
        let p = &self.parameters;
        let a = self.ellipsoid.semi_major_axis;
        let k0 = Self::get(p.scale);
        let latitude_0 = Self::get(p.origin_latitude);
        let (m0, t0) = self.lambert_m_t(latitude_0);
        let n = latitude_0.sin();
        let big_f = m0 / (n * t0.powf(n));
        (n, big_f, a * big_f * t0.powf(n) * k0)
    }

    /// The forward formulas both Lambert variants share — `E − EF = r sin θ`, `N − NF = rO −
    /// r cos θ`, with `r = scale · tⁿ` and `θ = n(λ − λO)` — where `scale` is `aF` for the 2SP
    /// variant and `aFkO` for the 1SP (sections 3.1.1.1 and 3.1.1.2).
    ///
    /// The pole on the far side from the cone's apex — the south pole of a northern cone — is
    /// where `t` is infinite, and is refused.
    fn cone_forward(
        &self,
        latitude: f64,
        delta: f64,
        n: f64,
        scale: f64,
        r_origin: f64,
    ) -> Result<(f64, f64), Refusal> {
        if latitude * n.signum() <= -(FRAC_PI_2 - AT_THE_POLE) {
            return Err(self.outside());
        }
        let (_, t) = self.lambert_m_t(latitude);
        let r = scale * t.powf(n);
        let theta = n * delta;
        Ok((r * theta.sin(), r_origin - r * theta.cos()))
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
        let a = self.ellipsoid.semi_major_axis;
        let (n, big_f, r_f) = self.lambert_2sp_cone();
        let sign = n.signum();
        let r_prime = sign * (x * x + (r_f - y).powi(2)).sqrt();
        let t_prime = (r_prime / (a * big_f)).powf(1.0 / n);
        let theta = (sign * x).atan2(sign * (r_f - y));
        Ok(LatLon {
            latitude: self.lambert_latitude(t_prime)?,
            longitude: theta / n + Self::get(self.parameters.origin_longitude),
        })
    }

    /// Guidance Note 7-2 section 3.1.1.2's reverse formulas, the 2SP tail with `n = sin φO`, the
    /// natural origin, and the scale factor.
    fn lambert_1sp(&self, x: f64, y: f64) -> Result<LatLon, Refusal> {
        let p = &self.parameters;
        let a = self.ellipsoid.semi_major_axis;
        let k0 = Self::get(p.scale);
        let (n, big_f, r0) = self.lambert_1sp_cone();
        let sign = n.signum();
        let r_prime = sign * (x * x + (r0 - y).powi(2)).sqrt();
        let t_prime = (r_prime / (a * k0 * big_f)).powf(1.0 / n);
        let theta = (sign * x).atan2(sign * (r0 - y));
        Ok(LatLon {
            latitude: self.lambert_latitude(t_prime)?,
            longitude: theta / n + Self::get(p.origin_longitude),
        })
    }

    /// Section 3.2.1's scale factor on the equator: stated for variant A, and for variant B
    /// derived from the standard parallel, which the section takes as its absolute value.
    fn mercator_scale(&self) -> f64 {
        let p = &self.parameters;
        if self.method == Method::MercatorVariantB {
            let e2 = self.ellipsoid.eccentricity_squared();
            let parallel = Self::get(p.parallel_1).abs();
            parallel.cos() / (1.0 - e2 * parallel.sin().powi(2)).sqrt()
        } else {
            Self::get(p.scale)
        }
    }

    /// Guidance Note 7-2 section 3.2.1's forward formulas for variants A and B; both poles are at
    /// infinite northing and are refused.
    fn mercator_forward(&self, latitude: f64, delta: f64) -> Result<(f64, f64), Refusal> {
        if latitude.abs() >= FRAC_PI_2 - AT_THE_POLE {
            return Err(self.outside());
        }
        let a = self.ellipsoid.semi_major_axis;
        let e = self.ellipsoid.eccentricity();
        let k0 = self.mercator_scale();
        let s = latitude.sin();
        let isometric = ((FRAC_PI_4 + latitude / 2.0).tan()
            * ((1.0 - e * s) / (1.0 + e * s)).powf(e / 2.0))
        .ln();
        Ok((a * k0 * delta, a * k0 * isometric))
    }

    /// Guidance Note 7-2 section 3.2.1's reverse formulas for variants A and B: the series in χ
    /// for latitude, which needs no iteration.
    fn mercator(&self, x: f64, y: f64) -> LatLon {
        let a = self.ellipsoid.semi_major_axis;
        let e2 = self.ellipsoid.eccentricity_squared();
        let k0 = self.mercator_scale();
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
            longitude: x / (a * k0) + Self::get(self.parameters.origin_longitude),
        }
    }

    /// Guidance Note 7-2 section 3.2.1.2's forward formulas: the sphere's, on the ellipsoid's
    /// semi-major axis, refused poleward of the section's 88°.
    fn pseudo_mercator_forward(&self, latitude: f64, delta: f64) -> Result<(f64, f64), Refusal> {
        if latitude.abs() > PSEUDO_MERCATOR_LIMIT {
            return Err(self.outside());
        }
        let a = self.ellipsoid.semi_major_axis;
        Ok((a * delta, a * (FRAC_PI_4 + latitude / 2.0).tan().ln()))
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

    /// Section 3.1.3's cone: `n`, `C` and `ρO`.
    fn albers_cone(&self) -> (f64, f64, f64) {
        let p = &self.parameters;
        let a = self.ellipsoid.semi_major_axis;
        let e2 = self.ellipsoid.eccentricity_squared();
        let m = |latitude: f64| latitude.cos() / (1.0 - e2 * latitude.sin().powi(2)).sqrt();
        let (phi_1, phi_2) = (Self::get(p.parallel_1), Self::get(p.parallel_2));
        let (m1, m2) = (m(phi_1), m(phi_2));
        let (alpha_1, alpha_2) = (self.albers_alpha(phi_1), self.albers_alpha(phi_2));
        let alpha_0 = self.albers_alpha(Self::get(p.origin_latitude));
        let n = (m1 * m1 - m2 * m2) / (alpha_2 - alpha_1);
        let c = m1 * m1 + n * alpha_1;
        (n, c, a * (c - n * alpha_0).sqrt() / n)
    }

    /// Guidance Note 7-2 section 3.1.3's forward formulas, which reach every point of the
    /// ellipsoid: an equal-area cone has no point at infinity.
    fn albers_forward(&self, latitude: f64, delta: f64) -> (f64, f64) {
        let a = self.ellipsoid.semi_major_axis;
        let (n, c, rho_0) = self.albers_cone();
        let rho = a * (c - n * self.albers_alpha(latitude)).sqrt() / n;
        let theta = n * delta;
        (rho * theta.sin(), rho_0 - rho * theta.cos())
    }

    /// Guidance Note 7-2 section 3.1.3's reverse formulas: the authalic latitude and the series
    /// from it, which needs no iteration. θ takes n's sign for section 3.1.1.1's reason.
    fn albers(&self, x: f64, y: f64) -> LatLon {
        let a = self.ellipsoid.semi_major_axis;
        let e = self.ellipsoid.eccentricity();
        let e2 = self.ellipsoid.eccentricity_squared();
        let (n, c, rho_0) = self.albers_cone();
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
            longitude: Self::get(self.parameters.origin_longitude) + theta / n,
        }
    }

    /// Section 3.3.1.1's conformal sphere at the projection's origin.
    fn conformal_sphere(&self) -> ConformalSphere {
        let e = self.ellipsoid.eccentricity();
        let e2 = self.ellipsoid.eccentricity_squared();
        let latitude_0 = Self::get(self.parameters.origin_latitude);
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
        ConformalSphere { r, n, c, chi_0 }
    }

    /// Guidance Note 7-2 section 3.3.1.1's forward formulas: onto the conformal sphere, then the
    /// sphere's stereographic projection from the origin's antipode, which is refused.
    ///
    /// `sin χ` is evaluated as `1 − 2/(w + 1)`, the section's `(w − 1)/(w + 1)` rearranged, so
    /// that a pole — where `w` is infinite or zero — gives ±1 rather than infinity over infinity.
    fn oblique_stereographic_forward(
        &self,
        latitude: f64,
        delta: f64,
    ) -> Result<(f64, f64), Refusal> {
        let e = self.ellipsoid.eccentricity();
        let k0 = Self::get(self.parameters.scale);
        let ConformalSphere { r, n, c, chi_0 } = self.conformal_sphere();
        let s = latitude.sin();
        let sa = (1.0 + s) / (1.0 - s);
        let sb = (1.0 - e * s) / (1.0 + e * s);
        let w = c * (sa * sb.powf(e)).powf(n);
        let chi = (1.0 - 2.0 / (w + 1.0)).clamp(-1.0, 1.0).asin();
        // Λ − ΛO, since the section's Λ is n(λ − ΛO) + ΛO.
        let lambda = n * delta;
        let b = 1.0 + chi.sin() * chi_0.sin() + chi.cos() * chi_0.cos() * lambda.cos();
        if b <= f64::EPSILON {
            return Err(self.outside());
        }
        let scale = 2.0 * r * k0 / b;
        Ok((
            scale * chi.cos() * lambda.sin(),
            scale * (chi.sin() * chi_0.cos() - chi.cos() * chi_0.sin() * lambda.cos()),
        ))
    }

    /// Guidance Note 7-2 section 3.3.1.1's reverse formulas: through the conformal sphere to the
    /// isometric latitude, then the section's iteration back to the ellipsoid.
    fn oblique_stereographic(&self, x: f64, y: f64) -> Result<LatLon, Refusal> {
        let e = self.ellipsoid.eccentricity();
        let e2 = self.ellipsoid.eccentricity_squared();
        let k0 = Self::get(self.parameters.scale);
        let big_lambda_0 = Self::get(self.parameters.origin_longitude);
        let ConformalSphere { r, n, c, chi_0 } = self.conformal_sphere();

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

    /// ADR 1672's budget for the forward direction: within 1.5 cm on the ground of the grid point
    /// an example prints, in `unit` metres — ADR 1587's half a thousandth of a second, as a
    /// distance, so that the two directions are held to one budget.
    fn assert_grid_within(answer: GridPoint, [easting, northing]: [f64; 2], unit: f64) {
        for (got, expected) in [(answer.easting, easting), (answer.northing, northing)] {
            let off = (got - expected * unit).abs();
            assert!(
                off <= GROUND_BUDGET,
                "{got} m against {expected}: off by {off} m"
            );
        }
    }

    /// ADR 1587's budget on the ground: 0.0005″ of latitude is 1.5 cm.
    const GROUND_BUDGET: f64 = 0.015;

    /// A projection whose parameters are all stated.
    fn projection(method: Method, ellipsoid: Ellipsoid, parameters: Parameters) -> Projection {
        Projection::new(method, ellipsoid, parameters).expect("every parameter stated")
    }

    /// The US survey foot, by its definition of 1200/3937 m, which is how `system` converts a
    /// file's feet.
    const FOOT: f64 = 1200.0 / 3937.0;

    /// Guidance Note 7-2 section 3.2.3.1's example, OSGB 1936 / British National Grid.
    fn british_national_grid() -> Projection {
        projection(
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
    }

    /// Guidance Note 7-2 section 3.1.1.1's example, NAD27 / Texas South Central, whose grid is
    /// in US survey feet.
    fn texas_south_central() -> Projection {
        projection(
            Method::LambertConicConformal2Sp,
            Ellipsoid::new(6_378_206.400, 294.978_70).expect("Clarke 1866"),
            Parameters {
                origin_latitude: Some(dms(27.0, 50.0, 0.0)),
                origin_longitude: Some(-dms(99.0, 0.0, 0.0)),
                parallel_1: Some(dms(28.0, 23.0, 0.0)),
                parallel_2: Some(dms(30.0, 17.0, 0.0)),
                false_easting: Some(2_000_000.0 * FOOT),
                false_northing: Some(0.0),
                ..Parameters::default()
            },
        )
    }

    /// Guidance Note 7-2 section 3.1.1.2's example, JAD69 / Jamaica National Grid.
    fn jamaica_national_grid() -> Projection {
        projection(
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
    }

    /// Guidance Note 7-2 section 3.2.1's first example, Makassar / NEIEZ.
    fn makassar() -> Projection {
        projection(
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
    }

    /// Guidance Note 7-2 section 3.2.1's second example, Pulkovo 1942 / Caspian Sea Mercator.
    fn caspian_sea() -> Projection {
        projection(
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
    }

    /// Guidance Note 7-2 section 3.2.1.2's example, WGS 84 / Pseudo-Mercator.
    fn web_mercator() -> Projection {
        projection(
            Method::PseudoMercator,
            Ellipsoid::new(6_378_137.0, 298.257_223_6).expect("WGS 84"),
            Parameters {
                origin_longitude: Some(0.0),
                ..Parameters::default()
            },
        )
    }

    /// Guidance Note 7-2 section 3.3.1.1's example, Amersfoort / RD New.
    fn rd_new() -> Projection {
        projection(
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
    }

    /// ISO 32000-2 §12.10.4's EXAMPLE 2: Albers on GRS 1980, standard parallels 20°N and 60°N,
    /// origin 40°N 96°W — the one Albers system this tree holds a text for, since section 3.1.3
    /// prints no example of its own.
    fn north_america_albers() -> Projection {
        projection(
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
    }

    /// A Lambert cone with both parallels south of the equator: the sign `lambert_2sp` applies.
    fn southern_cone() -> Projection {
        projection(
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
    }

    fn at(latitude: f64, longitude: f64) -> LatLon {
        LatLon {
            latitude,
            longitude,
        }
    }

    #[test]
    fn transverse_mercator_reproduces_the_british_national_grid_example() {
        // Section 3.2.3.1's example, JHS formulas: (577274.99 E, 69740.50 N) is
        // 50°30′00.000″N 00°30′00.000″E, in both directions.
        let projection = british_national_grid();
        let answer = projection.inverse(577_274.99, 69_740.50).expect("a point");
        assert_within(answer.latitude, dms(50.0, 30.0, 0.0), 3);
        assert_within(answer.longitude, dms(0.0, 30.0, 0.0), 3);
        let grid = projection
            .forward(at(dms(50.0, 30.0, 0.0), dms(0.0, 30.0, 0.0)))
            .expect("a grid point");
        assert_grid_within(grid, [577_274.99, 69_740.50], 1.0);
        // The example prints its constants and its forward intermediates, and they are the
        // tighter test: each to half its last printed digit. Its printed grid point is not
        // what its own η and ξ give — kO·B·η is 577274.984 m and kO(Bξ − MO) − 100000 is
        // 69740.492 m — so the printed easting and northing are held to the ground budget above
        // and the intermediates to their digits (ADR 1672).
        let Jhs { n, b, h, m0 } = projection.jhs();
        let close = |got: f64, expected: f64, half: f64| {
            assert!((got - expected).abs() <= half, "{got} against {expected}");
        };
        close(n, 0.001_673_22, 5e-9);
        close(b, 6_366_914.609, 5e-4);
        close(h[0], 0.000_834_745_2, 5e-11);
        close(h[1], 0.000_000_755_4, 5e-11);
        close(h[2], 1.184_87e-9, 5e-15);
        close(h[3], 2.408_64e-12, 5e-18);
        close(m0, 5_429_228.602, 5e-4);
        let k0 = 0.999_601_271_7;
        close(
            (grid.easting - 400_000.0) / (k0 * b),
            0.027_854_260_3,
            5e-11,
        );
        close(
            ((grid.northing + 100_000.0) / k0 + m0) / b,
            0.879_395_617_1,
            5e-11,
        );
    }

    #[test]
    fn lambert_2sp_reproduces_the_texas_south_central_example_in_us_survey_feet() {
        // Section 3.1.1.1's example: (2963503.91 ftUS E, 254759.80 ftUS N) is 28°30′00″N
        // 96°00′00″W.
        let projection = texas_south_central();
        let answer = projection
            .inverse(2_963_503.91 * FOOT, 254_759.80 * FOOT)
            .expect("a point");
        assert_within(answer.latitude, dms(28.0, 30.0, 0.0), 3);
        assert_within(answer.longitude, -dms(96.0, 0.0, 0.0), 3);
        let grid = projection
            .forward(at(dms(28.0, 30.0, 0.0), -dms(96.0, 0.0, 0.0)))
            .expect("a grid point");
        assert_grid_within(grid, [2_963_503.91, 254_759.80], FOOT);
    }

    #[test]
    fn lambert_1sp_reproduces_the_jamaica_national_grid_example() {
        // Section 3.1.1.2's example: (255966.58 E, 142493.51 N) is 17°55′55.80″N 76°56′37.26″W.
        let projection = jamaica_national_grid();
        let answer = projection.inverse(255_966.58, 142_493.51).expect("a point");
        assert_within(answer.latitude, dms(17.0, 55.0, 55.80), 2);
        assert_within(answer.longitude, -dms(76.0, 56.0, 37.26), 2);
        let grid = projection
            .forward(at(dms(17.0, 55.0, 55.80), -dms(76.0, 56.0, 37.26)))
            .expect("a grid point");
        assert_grid_within(grid, [255_966.58, 142_493.51], 1.0);
    }

    #[test]
    fn mercator_variant_a_reproduces_the_makassar_example() {
        // Section 3.2.1's first example: (5009726.58 E, 569150.82 N) is 3°00′00.000″S
        // 120°00′00.000″E.
        let projection = makassar();
        let answer = projection
            .inverse(5_009_726.58, 569_150.82)
            .expect("a point");
        assert_within(answer.latitude, -dms(3.0, 0.0, 0.0), 3);
        assert_within(answer.longitude, dms(120.0, 0.0, 0.0), 3);
        let grid = projection
            .forward(at(-dms(3.0, 0.0, 0.0), dms(120.0, 0.0, 0.0)))
            .expect("a grid point");
        assert_grid_within(grid, [5_009_726.58, 569_150.82], 1.0);
    }

    #[test]
    fn mercator_variant_b_reproduces_the_caspian_sea_example() {
        // Section 3.2.1's second example: (165704.29 E, 5171848.07 N) is 53°00′00.000″N
        // 53°00′00.000″E.
        let projection = caspian_sea();
        let answer = projection
            .inverse(165_704.29, 5_171_848.07)
            .expect("a point");
        assert_within(answer.latitude, dms(53.0, 0.0, 0.0), 3);
        assert_within(answer.longitude, dms(53.0, 0.0, 0.0), 3);
        let grid = projection
            .forward(at(dms(53.0, 0.0, 0.0), dms(53.0, 0.0, 0.0)))
            .expect("a grid point");
        assert_grid_within(grid, [165_704.29, 5_171_848.07], 1.0);
    }

    #[test]
    fn pseudo_mercator_reproduces_the_web_mercator_example() {
        // Section 3.2.1.2's example. Its forward point is printed in radians to nine places,
        // 0.425542460 rad N and −1.751147016 rad E, and is (–11169055.58 E, 2800000.00 N); its
        // reverse point is a point ten kilometres north on the grid, (–11169055.58 E,
        // 2810000.00 N), which is 24°27′48.889″N 100°20′00.000″W.
        let projection = web_mercator();
        let answer = projection
            .inverse(-11_169_055.58, 2_810_000.00)
            .expect("a point");
        assert_within(answer.latitude, dms(24.0, 27.0, 48.889), 3);
        assert_within(answer.longitude, -dms(100.0, 20.0, 0.0), 3);
        let grid = projection
            .forward(at(0.425_542_460, -1.751_147_016))
            .expect("a grid point");
        assert_grid_within(grid, [-11_169_055.58, 2_800_000.00], 1.0);
    }

    #[test]
    fn oblique_stereographic_reproduces_the_rd_new_example() {
        // Section 3.3.1.1's example: 53°N 6°E is (196105.283 E, 557057.739 N), and the reverse
        // calculation of (196105.28 E, 557057.74 N) is 53°00′00.000″N 6°00′00.000″E.
        let projection = rd_new();
        let answer = projection.inverse(196_105.28, 557_057.74).expect("a point");
        assert_within(answer.latitude, dms(53.0, 0.0, 0.0), 3);
        assert_within(answer.longitude, dms(6.0, 0.0, 0.0), 3);
        let grid = projection
            .forward(at(dms(53.0, 0.0, 0.0), dms(6.0, 0.0, 0.0)))
            .expect("a grid point");
        assert_grid_within(grid, [196_105.283, 557_057.739], 1.0);
    }

    #[test]
    fn every_method_returns_from_its_own_forward_within_the_budget() {
        // ADR 1672: the round trip closes within ADR 1587's half a thousandth of a second at each
        // example's point and at points around it — one degree either way, and for the
        // Transverse Mercator four degrees off the central meridian, the edge of the band within
        // which section 3.2.3.1 says its two formulas agree.
        let cases: [(Projection, f64, f64); 9] = [
            (british_national_grid(), 50.5, 0.5),
            (texas_south_central(), 28.5, -96.0),
            (
                jamaica_national_grid(),
                dms(17.0, 55.0, 55.80).to_degrees(),
                -76.94,
            ),
            (makassar(), -3.0, 120.0),
            (caspian_sea(), 53.0, 53.0),
            (web_mercator(), 24.38, -100.33),
            (rd_new(), 53.0, 6.0),
            (north_america_albers(), 35.0, -75.0),
            (southern_cone(), -30.0, 140.0),
        ];
        for (projection, latitude, longitude) in cases {
            for (d_latitude, d_longitude) in [(0.0, 0.0), (1.0, 1.0), (-1.0, -1.0), (0.5, -4.0)] {
                let point = at(
                    (latitude + d_latitude).to_radians(),
                    (longitude + d_longitude).to_radians(),
                );
                let grid = projection.forward(point).expect("a grid point");
                let back = projection
                    .inverse(grid.easting, grid.northing)
                    .expect("a point");
                assert_within(back.latitude, point.latitude, 3);
                assert_within(back.longitude, point.longitude, 3);
            }
        }
    }

    #[test]
    fn every_method_puts_its_origin_at_its_false_coordinates() {
        // Each section's forward formulas make the origin's own grid position the false easting
        // and northing: θ = 0 and r = rF on a cone, η = 0 and Bξ = MO on the Transverse Mercator,
        // Λ = ΛO and χ = χO on the conformal sphere, and the equator on a Mercator.
        for projection in [
            british_national_grid(),
            texas_south_central(),
            jamaica_national_grid(),
            makassar(),
            caspian_sea(),
            web_mercator(),
            rd_new(),
            north_america_albers(),
            southern_cone(),
        ] {
            let p = projection.parameters;
            let grid = projection
                .forward(at(
                    p.origin_latitude.unwrap_or(0.0),
                    p.origin_longitude.expect("every method states one"),
                ))
                .expect("the origin");
            assert!(
                (grid.easting - p.false_easting.unwrap_or(0.0)).abs() < 1e-6
                    && (grid.northing - p.false_northing.unwrap_or(0.0)).abs() < 1e-6,
                "{}: {grid:?}",
                projection.method.name()
            );
        }
    }

    #[test]
    fn a_point_no_grid_holds_is_refused_rather_than_answered() {
        let outside = |projection: &Projection, latitude: f64, longitude: f64| {
            projection.forward(at(latitude.to_radians(), longitude.to_radians()))
                == Err(Refusal::OutsideDomain {
                    method: projection.method.name(),
                })
        };
        // Mercator's poles, the northern cone's south pole, a Transverse Mercator point past 90°
        // of longitude, the Pseudo-Mercator past 88°, and a latitude past a pole.
        assert!(outside(&makassar(), 90.0, 120.0));
        assert!(outside(&caspian_sea(), -90.0, 51.0));
        assert!(outside(&texas_south_central(), -90.0, -99.0));
        assert!(outside(&southern_cone(), 90.0, 135.0));
        // The stereographic's antipode, on the equatorial sphere where the conformal sphere is
        // the sphere itself and the antipode's `B` is exactly zero.
        let equatorial = projection(
            Method::ObliqueStereographic,
            Ellipsoid::new(6_371_000.0, 0.0).expect("a sphere"),
            Parameters {
                origin_latitude: Some(0.0),
                origin_longitude: Some(0.0),
                scale: Some(1.0),
                ..Parameters::default()
            },
        );
        assert!(outside(&equatorial, 0.0, 180.0));
        assert!(equatorial.forward(at(0.0, 179f64.to_radians())).is_ok());
        assert!(outside(&british_national_grid(), 10.0, 100.0));
        assert!(outside(&web_mercator(), 88.5, 0.0));
        assert!(outside(&jamaica_national_grid(), 91.0, 0.0));
        // The near pole of a cone is its apex, a point at the cone's centre and no refusal.
        assert!(texas_south_central().forward(at(FRAC_PI_2, 0.0)).is_ok());
        // The inverse keeps the Pseudo-Mercator's limit too.
        let far_north = web_mercator()
            .forward(at(87.9f64.to_radians(), 0.0))
            .expect("inside the limit");
        assert!(
            web_mercator()
                .inverse(0.0, far_north.northing * 1.1)
                .is_err()
        );
    }

    #[test]
    fn a_longitude_across_the_antimeridian_projects_as_one_piece() {
        // Section 1.4: λ − λO is wrapped into −180°..180°, so a point at 179°W on a system whose
        // origin is at 179°E is two degrees east of it, not 358° west.
        let projection = projection(
            Method::TransverseMercator,
            Ellipsoid::new(6_378_137.0, 298.257_223_563).expect("WGS 84"),
            Parameters {
                origin_latitude: Some(0.0),
                origin_longitude: Some(179f64.to_radians()),
                scale: Some(0.9996),
                false_easting: Some(500_000.0),
                false_northing: Some(0.0),
                ..Parameters::default()
            },
        );
        let east = projection
            .forward(at(10f64.to_radians(), (-179f64).to_radians()))
            .expect("a grid point");
        let west = projection
            .forward(at(10f64.to_radians(), 177f64.to_radians()))
            .expect("a grid point");
        assert!((east.easting - 500_000.0 - (500_000.0 - west.easting)).abs() < 1e-6);
        assert!(east.easting > 500_000.0);
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

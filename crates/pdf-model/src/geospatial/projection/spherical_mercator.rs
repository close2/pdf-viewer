//! Mercator (Spherical), EPSG method 1026, by Guidance Note 7-2 section 3.2.1.1's formulas, and
//! the code it replaced, Mercator (1SP) (Spherical), 9841.
//!
//! The Dataset prints no formulas for 9841 ("See information source"), and records that 1026
//! replaced it with the reason "Removed scale factor as defining parameter for spherical
//! mercator". So 9841 is read as section 3.2.1.1's formulas with a scale factor `kO` multiplying
//! `R`, as section 3.2.1's variant A has it multiply `a`, and 1026, which states none, as `kO = 1`
//! (ADR 3886). The one 9841 system the table carries states `kO = 1`, where the readings agree.

use std::f64::consts::{FRAC_PI_2, FRAC_PI_4};

use super::{LatLon, PSEUDO_MERCATOR_LIMIT, Projection, Refusal};

impl Projection {
    /// `R kO`: section 3.2.1.1's `R`, the sphere's radius or, on an ellipsoid, "the radius of
    /// the conformal sphere at the projection origin at latitude ϕO", the implementation notes'
    /// `RC = a √(1 – e²) / (1 – e² sin² ϕO)`, times 9841's scale factor.
    fn spherical_mercator_radius(&self) -> f64 {
        let p = &self.parameters;
        let origin = Self::get(p.origin_latitude);
        let e2 = self.ellipsoid.eccentricity_squared();
        let radius =
            self.ellipsoid.semi_major_axis * (1.0 - e2).sqrt() / (1.0 - e2 * origin.sin().powi(2));
        // not a repair: 1026 has no scale factor among its parameters, and the reading above
        // states 9841's as multiplying `R`; absent, `R` stands alone.
        radius * p.scale.unwrap_or(1.0)
    }

    /// Section 3.2.1.1's forward formulas, `E = FE + R (λ – λO)` and
    /// `N = FN + R ln[tan(π/4 + ϕ/2)]`, less the false easting and northing, refused poleward of
    /// 88° where the section says the northing's formula "should not be used".
    pub(super) fn spherical_mercator_forward(
        &self,
        latitude: f64,
        delta: f64,
    ) -> Result<(f64, f64), Refusal> {
        if latitude.abs() > PSEUDO_MERCATOR_LIMIT {
            return Err(self.outside());
        }
        let r = self.spherical_mercator_radius();
        Ok((r * delta, r * (FRAC_PI_4 + latitude / 2.0).tan().ln()))
    }

    /// Section 3.2.1.1's reverse formulas, `D = (FN – N) / R`, `ϕ = π/2 – 2 atan(e^D)` and
    /// `λ = [(E – FE)/R] + λO`.
    pub(super) fn spherical_mercator(&self, x: f64, y: f64) -> LatLon {
        let r = self.spherical_mercator_radius();
        let d = -y / r;
        LatLon {
            latitude: FRAC_PI_2 - 2.0 * d.exp().atan(),
            longitude: x / r + Self::get(self.parameters.origin_longitude),
        }
    }
}

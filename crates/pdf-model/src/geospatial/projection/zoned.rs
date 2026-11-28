//! Transverse Mercator Zoned Grid System, EPSG method 9824, by Guidance Note 7-2 section 3.2.3.2.
//!
//! A family of Transverse Mercator zones sharing every parameter but the central meridian, which
//! each zone derives from its number, and the number prefixed to the easting in millions of
//! metres. The section prints its modification against an older series form of the Transverse
//! Mercator easting; the general formulas this module evaluates are section 3.2.3.1's, whose
//! easting the prefix is added to (ADR 3879).
//!
//! The section says how a longitude finds its zone, and nothing of how an easting does. An
//! easting is `Z × 10⁶ + FE + x` with `FE + x` inside a zone's million metres for every grid the
//! method is used for (UTM's runs 166 to 834 km), so the reverse reads the zone as the easting's
//! whole millions — a choice, written down in ADR 3879, and an easting whose millions are no zone
//! is refused.

use std::f64::consts::TAU;

use super::{GridPoint, LatLon, Method, Parameters, Projection};
use crate::geospatial::Refusal;

/// The easting's prefix: one zone is `10⁶` metres, section 3.2.3.2's `Z*10⁶`.
const ZONE_PREFIX: f64 = 1e6;

impl Projection {
    /// The number of zones, `360 / W`, which [`Projection::new`] required to be whole; the
    /// rounding takes off what converting the width to radians left.
    fn zone_count(&self) -> f64 {
        (TAU / Self::get(self.parameters.zone_width)).round()
    }

    /// Section 3.2.3.2's `Z = [INT((λ – λI) / W) mod (360 / W)] + 1`, its `mod` the section's
    /// `a − floor(a / b) × b`.
    ///
    /// `INT` is read as `floor`: it is the reading under which every longitude west of λI still
    /// lands in a zone of width W, where truncation toward zero would give zone 1 twice the
    /// width (ADR 3879).
    pub(crate) fn zone_number(&self, longitude: f64) -> f64 {
        let width = Self::get(self.parameters.zone_width);
        let count = self.zone_count();
        let whole = ((longitude - Self::get(self.parameters.initial_longitude)) / width).floor();
        whole - (whole / count).floor() * count + 1.0
    }

    /// The Transverse Mercator of zone `z`: section 3.2.3.2's `λO = λI + (Z W) – (W/2)` and the
    /// zoned system's other parameters.
    fn zone(&self, z: f64) -> Projection {
        let p = &self.parameters;
        let width = Self::get(p.zone_width);
        Projection {
            method: Method::TransverseMercator,
            ellipsoid: self.ellipsoid,
            parameters: Parameters {
                origin_longitude: Some(Self::get(p.initial_longitude) + z * width - width / 2.0),
                ..*p
            },
        }
    }

    /// The easting and northing of a point: its zone's Transverse Mercator, the zone number
    /// prefixed to the easting.
    pub(super) fn zoned_forward(
        &self,
        latitude: f64,
        longitude: f64,
    ) -> Result<GridPoint, Refusal> {
        let z = self.zone_number(longitude);
        let point = self.zone(z).forward(LatLon {
            latitude,
            longitude,
        })?;
        Ok(GridPoint {
            easting: z * ZONE_PREFIX + point.easting,
            northing: point.northing,
        })
    }

    /// The latitude and longitude of an easting and a northing whose whole millions name the zone.
    pub(super) fn zoned_inverse(&self, easting: f64, northing: f64) -> Result<LatLon, Refusal> {
        let z = (easting / ZONE_PREFIX).floor();
        if !(z >= 1.0 && z <= self.zone_count()) {
            return Err(Refusal::OutsideDomain {
                method: self.method.name(),
            });
        }
        self.zone(z).inverse(easting - z * ZONE_PREFIX, northing)
    }
}

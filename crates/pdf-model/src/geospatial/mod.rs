//! ISO 32000-2 §12.10's projection: a geospatial measure's projected coordinates turned into a
//! latitude and a longitude of the file's own geographic system.
//!
//! [`crate::measurement`] reads every table of §12.10 as data. What it cannot do alone is the
//! step §12.10.4's projected system exists for — the system "specifies the algorithms and
//! associated parameters used to transform points between geographic coordinates and a
//! two-dimensional (projected) coordinate system" — because the algorithm is named outside this
//! standard: by an EPSG code, or by a Well Known Text string whose format "is specified in ISO
//! 19162". This module is that step, built of the tree's own on the owner's answer
//! `doc/questions/A171`:
//!
//! - [`wkt`] is ISO 19162 section 6's string form;
//! - [`system`]'s reading says what a tree means — the older `PROJCS` form every census document
//!   carries, and ISO 19162's own `PROJCRS`;
//! - [`projection`] is each method the census found in both directions, each cited to IOGP
//!   Guidance Note 7-2's own section and tested on its worked example;
//! - [`registration`] is the affine map a registration determines between an object's unit
//!   square and the earth, and that map run backwards (ADRs 1593 and 1672);
//! - [`datum`] carries a position to a display system on another datum, through the parameters a
//!   file's `TOWGS84` states (ADR 1958);
//! - [`Ellipsoid`] is Guidance Note 7-2 section 1.1's shared quantities, and section 4.1.1's
//!   conversion to and from earth-centred coordinates.
//!
//! # The fork the census decided
//!
//! Table 270 lets a file name its system by `/EPSG` or by `/WKT`, and the first needs the IOGP's
//! registry carried into this tree while the second carries every parameter inline. The census
//! over the crawl (`examples/geospatial_census.rs`, ADR 1586) found that every system dictionary
//! stating `/EPSG` states `/WKT` beside it — not one document of the crawl or the corpora names a
//! system by code alone — so **no registry is carried**: a dictionary that states only a code is
//! refused by that code ([`Refusal::EpsgWithoutWkt`]), and one that states both is read from its
//! WKT.
//!
//! # What is not done, and why it is the clause's line
//!
//! **A datum is transformed where the file states how, and only there.** §12.10 asks where a point
//! is in the system the file names; the answer is a latitude on that system's own datum.
//! §12.10.2's `/DCS` names the system positions are displayed in, and [`Geospatial::display`]
//! reaches it — on the same datum as the same latitude, and on another through Guidance Note 7-2
//! section 4.2.3's Helmert transformation with the parameters each datum's `TOWGS84` states, on
//! the owner's answer `doc/questions/A436`. The parameters between two datums are in no clause and
//! no name, so a datum that states none is refused by both datums' names rather than guessed for
//! (ADR 1958).
//! **Between registration points the function is a choice, and it is named**: §12.10 states the
//! points and no function between them, so [`Geospatial::registration_geographic`] converts the
//! points the file states, [`Geospatial::geographic_position`] converts through `/PCSM`, the one
//! function the table does state, and [`Geospatial::registration_map`] is ADR 1593's reading
//! between the points — fitted in a projected system's plane (ADR 1918) — which
//! `crate::measurement::Viewport::page_position` runs backwards.
//!
//! **One sentence of Table 269 is departed from**, on the owner's answer `doc/questions/A271`: a
//! projected system's `/GPTS` whose every pair is shaped as degrees are read as its base system's
//! degrees rather than as the "eastings and northings" the table states, because every projected
//! map the census found writes them so (ADR 1918).

pub mod datum;
pub mod ellipsoid;
pub mod projection;
pub mod registration;
pub mod system;
pub mod wkt;

pub use datum::{DatumShift, Helmert};
pub use ellipsoid::Ellipsoid;
pub use projection::{GridPoint, LatLon, Method, Parameters, Projection};
pub use registration::AffineRegistration;
pub use system::{GeographicSystem, ProjectedSystem, ReferenceSystem};

use crate::measurement::{CoordinateSystem, Geospatial};

/// Why a position cannot be given, each naming what the file states or what this tree does not
/// carry — a refusal is said out loud and never replaced by a guess (ADR 1586).
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum Refusal {
    /// The dictionary states neither `/EPSG` nor `/WKT`, which §12.10.3 requires one of.
    #[error("the coordinate system states neither an EPSG code nor a WKT string")]
    NotStated,
    /// The dictionary names its system by EPSG code alone, and no registry is carried.
    #[error("EPSG {0} is stated without a WKT string, and this program carries no EPSG registry")]
    EpsgWithoutWkt(i64),
    /// The `/WKT` string is not Well Known Text.
    #[error("the WKT string is not Well Known Text: {0}")]
    Wkt(wkt::WktError),
    /// The outermost token is not a geographic or projected system.
    #[error("{0} is not a geographic or projected coordinate system")]
    NotASystem(String),
    /// A component a system requires is not stated.
    #[error("{of} states no {what}")]
    Missing {
        /// What is missing.
        what: &'static str,
        /// The token it is missing from.
        of: String,
    },
    /// A projection method this tree does not carry, by the name the file gives it.
    #[error("the projection method {0:?} is not one this program evaluates")]
    UnsupportedMethod(String),
    /// A bound CRS's transformation method this tree does not carry, by the name the file gives
    /// it: one outside the Helmert family of Guidance Note 7-2 sections 4.2.3 and 4.2.4, or one
    /// that reads a grid of shifts from a file (ADR 1958).
    #[error("the datum transformation method {0:?} is not one this program evaluates")]
    UnsupportedTransformation(String),
    /// A parameter a method's formulas need and the system does not state.
    #[error("{method} needs its {parameter}, which the system does not state")]
    MissingParameter {
        /// The method.
        method: &'static str,
        /// The parameter, by Guidance Note 7-2's name for it.
        parameter: &'static str,
    },
    /// A parameter whose meaning this reader does not know.
    #[error("{method} states a parameter {parameter:?} this program does not know")]
    UnknownParameter {
        /// The method, as named.
        method: String,
        /// The parameter, as named.
        parameter: String,
    },
    /// A combination a method's definition does not cover.
    #[error("{method}: {reason} is not a combination this program evaluates")]
    Unsupported {
        /// The method.
        method: &'static str,
        /// The combination.
        reason: &'static str,
    },
    /// An ellipsoid whose numbers describe none.
    #[error("an ellipsoid of axis {semi_major_axis} and inverse flattening {inverse_flattening}")]
    Ellipsoid {
        /// The semi-major axis as stated.
        semi_major_axis: f64,
        /// The inverse flattening as stated.
        inverse_flattening: f64,
    },
    /// A unit whose conversion factor is not a positive number.
    #[error("the unit {0:?} states no usable conversion factor")]
    Unit(String),
    /// An iterated latitude that did not settle.
    #[error("{method}'s latitude did not converge for this point")]
    NoConvergence {
        /// The method.
        method: &'static str,
    },
    /// `/DCS` names a system on another datum, and one of the two datums states no parameters
    /// that carry it to WGS 84 and is not WGS 84 itself, so no transformation between them is
    /// stated (ADR 1958).
    #[error(
        "the display system's datum {to:?} is not the file's own {from:?}, and the two strings \
         state no transformation between them"
    )]
    DatumTransformation {
        /// The datum of `/GCS`'s geographic system.
        from: String,
        /// The datum `/DCS` names.
        to: String,
    },
    /// A datum's `TOWGS84` clause is not the three, six or seven numbers OGC 01-009 section
    /// 7.3.18 lets it state.
    #[error(
        "the datum's TOWGS84 states {count} parameter(s), where three, six or seven numbers are a \
         transformation"
    )]
    DatumShift {
        /// How many attributes the clause states.
        count: usize,
    },
    /// A point the method's formulas do not reach.
    #[error("the point is outside the domain of {method}")]
    OutsideDomain {
        /// The method.
        method: &'static str,
    },
    /// `/DCS` names a projected system, whose position is an easting and a northing; a caller
    /// that writes only degrees has asked for a form the display system does not take, and
    /// [`Geospatial::display`] gives the easting and northing.
    #[error(
        "the display system is projected, so its position is an easting and a northing, not \
         degrees"
    )]
    DisplayIsProjected,
    /// The registration states too few points, or points on one line or one parallel, to
    /// determine an affine map between the unit square and the earth (ADR 1593).
    #[error(
        "the file's {0} registration point(s) do not span the map: an affine map needs three \
         that are not on one line"
    )]
    NoMap(usize),
    /// `/PCSM` maps two directions of the object onto one line of the projected plane, so no
    /// position on the plane has one point of the object.
    #[error("the /PCSM matrix is singular, so no position on the map has one point on the page")]
    SingularMatrix,
    /// The position is outside the map's neatline, where Table 269's `/Bounds` says the file's
    /// registration does not apply.
    #[error(
        "this position is outside the map's neatline, where the file's registration does not \
         apply (Table 269's /Bounds)"
    )]
    OutsideNeatline,
}

/// A position as Table 269's `/DCS` displays it: in the display system's own coordinates.
#[derive(Debug, Clone, PartialEq)]
pub enum Displayed {
    /// A latitude and a longitude of a geographic display system.
    Geographic(GeographicPosition),
    /// An easting and a northing of a projected display system, in its own linear unit.
    Projected {
        /// The easting.
        easting: f64,
        /// The northing.
        northing: f64,
        /// The unit both are counted in, as the system's string names it.
        unit: String,
    },
}

/// A position on the earth, in degrees of the file's own geographic system.
///
/// The longitude counts from that system's own prime meridian, east positive, within −180..180
/// (Guidance Note 7-2 section 1.4); for every system the census found, that meridian is
/// Greenwich.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GeographicPosition {
    /// The latitude, north positive.
    pub latitude: f64,
    /// The longitude, east positive.
    pub longitude: f64,
}

/// The projected system a `PROJCS` dictionary's `/WKT` states.
fn projected_system(system: &CoordinateSystem) -> Result<ProjectedSystem, Refusal> {
    match system.reference_system()? {
        ReferenceSystem::Projected(projected) => Ok(projected),
        ReferenceSystem::Geographic(_) => Err(Refusal::NotASystem(
            "a PROJCS dictionary whose WKT states a geographic system".to_owned(),
        )),
    }
}

impl CoordinateSystem {
    /// The system this dictionary states, read from its `/WKT`.
    ///
    /// # Errors
    ///
    /// [`Refusal::EpsgWithoutWkt`] where only a code is stated, [`Refusal::NotStated`] where
    /// neither is, and whatever [`system::from_wkt`] refuses of the string.
    pub fn reference_system(&self) -> Result<ReferenceSystem, Refusal> {
        match (&self.wkt, self.epsg) {
            (Some(text), _) => system::from_wkt(text),
            (None, Some(code)) => Err(Refusal::EpsgWithoutWkt(code)),
            (None, None) => Err(Refusal::NotStated),
        }
    }
}

impl ProjectedSystem {
    /// The easting and the northing, in this system's own unit, of a latitude and a longitude of
    /// its base geographic system — Guidance Note 7-2's forward formulas of the system's method.
    ///
    /// # Errors
    ///
    /// What [`Projection::forward`] refuses: a point the method sends to infinity or does not
    /// reach.
    pub fn grid(&self, position: GeographicPosition) -> Result<[f64; 2], Refusal> {
        let GridPoint { easting, northing } = self.projection.forward(LatLon {
            latitude: position.latitude.to_radians(),
            longitude: position.longitude.to_radians(),
        })?;
        Ok([easting / self.linear_unit, northing / self.linear_unit])
    }

    /// The latitude and longitude of an easting and a northing in this system's own unit.
    ///
    /// # Errors
    ///
    /// What [`Projection::inverse`] refuses.
    pub fn geographic(&self, easting: f64, northing: f64) -> Result<GeographicPosition, Refusal> {
        let LatLon {
            latitude,
            longitude,
        } = self
            .projection
            .inverse(easting * self.linear_unit, northing * self.linear_unit)?;
        Ok(GeographicPosition {
            latitude: latitude.to_degrees(),
            longitude: longitude.to_degrees(),
        })
    }
}

impl Geospatial {
    /// Each `/GPTS`–`/LPTS` registration pair with its geographic point as a latitude and a
    /// longitude.
    ///
    /// Where `/GCS` is geographic, Table 269 states the points as "degrees of latitude and
    /// longitude, respectively", and they are returned as the file states them. Where it is
    /// projected, the points are "eastings and northings", and each is carried through the
    /// system's inverse projection — except where every pair is shaped as degrees, and then they
    /// are the base system's degrees and are returned as the file states them (ADR 1918).
    ///
    /// # Errors
    ///
    /// [`Refusal::NotStated`] without a `/GCS`, and what [`CoordinateSystem::reference_system`]
    /// or the projection refuses.
    pub fn registration_geographic(&self) -> Result<Vec<(GeographicPosition, [f64; 2])>, Refusal> {
        let system = self.coordinate_system.as_ref().ok_or(Refusal::NotStated)?;
        if !system.projected {
            return Ok(self.registration_as_degrees());
        }
        Ok(self
            .registration_in_plane(&projected_system(system)?)?
            .into_iter()
            .map(|(position, _, local)| (position, local))
            .collect())
    }

    /// The affine map the registration determines between the object's unit square and the
    /// earth, with its departure from the file's points: in degrees where `/GCS` is geographic
    /// (ADR 1593), and in the projected system's plane where it is projected, a position between
    /// the points carried to the earth by the system's inverse projection (ADR 1918).
    ///
    /// # Errors
    ///
    /// [`Refusal::NoMap`] where the pairs determine no map of the plane, and what
    /// [`Self::registration_geographic`] or the projection refuses.
    pub fn registration_map(&self) -> Result<AffineRegistration, Refusal> {
        let system = self.coordinate_system.as_ref().ok_or(Refusal::NotStated)?;
        let count = self.registration().len();
        if !system.projected {
            return AffineRegistration::fit(&self.registration_as_degrees())
                .ok_or(Refusal::NoMap(count));
        }
        let projected = projected_system(system)?;
        let pairs = self.registration_in_plane(&projected)?;
        AffineRegistration::fit_in_plane(projected, &pairs)?.ok_or(Refusal::NoMap(count))
    }

    /// The registration pairs with `/GPTS` read as latitudes and longitudes, as Table 269 states
    /// them for a geographic system.
    fn registration_as_degrees(&self) -> Vec<(GeographicPosition, [f64; 2])> {
        self.registration()
            .into_iter()
            .map(|([latitude, longitude], local)| {
                (
                    GeographicPosition {
                        latitude,
                        longitude,
                    },
                    local,
                )
            })
            .collect()
    }

    /// A projected system's registration pairs, each with its position on the earth and in the
    /// system's plane.
    ///
    /// Table 269 states a projected system's `/GPTS` as "eastings and northings", and they are
    /// carried to the earth by the inverse projection. **A set whose every pair lies within ±90
    /// by ±180 is read as the base system's degrees instead**, and carried into the plane by the
    /// forward projection: the census found 158 documents stating a projected `/GCS`, every one
    /// of them writes its points that way and none states `/PCSM`, and read as eastings each map
    /// would lie within a few hundred metres of its grid's origin. That is a departure from the
    /// table's sentence, on the owner's answer `doc/questions/A271`, and its cost is a conforming
    /// file whose every easting and northing falls within 90 by 180 units of its grid's origin,
    /// read as degrees. What the reading rests on is NOTE 2 beside the sentence, that every
    /// projected system includes an underlying geographic one (ADR 1918).
    fn registration_in_plane(
        &self,
        projected: &ProjectedSystem,
    ) -> Result<Vec<registration::PlanePair>, Refusal> {
        let pairs = self.registration();
        let shaped_as_degrees = !pairs.is_empty()
            && pairs
                .iter()
                .all(|([a, b], _)| a.abs() <= 90.0 && b.abs() <= 180.0);
        pairs
            .into_iter()
            .map(|([first, second], local)| {
                if shaped_as_degrees {
                    let position = GeographicPosition {
                        latitude: first,
                        longitude: second,
                    };
                    Ok((position, projected.grid(position)?, local))
                } else {
                    Ok((projected.geographic(first, second)?, [first, second], local))
                }
            })
            .collect()
    }

    /// A position of [`Self::coordinate_system`]'s own geographic system, in the system Table
    /// 269's `/DCS` names — "\[a] projected or geographic coordinate system that shall be used for
    /// the display of position values".
    ///
    /// Without a `/DCS` the position is displayed as it is. With one, the position is carried to
    /// the display system's geographic system — the system itself, or a projected system's base —
    /// by `datum::transform`: on the same datum and ellipsoid as `/GCS`'s it is the same
    /// latitude, its longitude moved by the difference of the two prime meridians, and on another
    /// datum — the clause's own example, a 1927 datum displayed in WGS84 — it goes through the
    /// Helmert transformations the two strings' `TOWGS84` clauses state (ADR 1958). A projected
    /// `/DCS` then carries it through its forward projection to an easting and a northing.
    ///
    /// # Errors
    ///
    /// [`Refusal::DatumTransformation`] by both datums' names where the strings state no
    /// transformation between them, [`Refusal::DatumShift`] for a `TOWGS84` this reader cannot
    /// read, whatever reading either system's `/WKT` refuses, and what the display system's
    /// forward projection refuses.
    pub fn display(&self, position: GeographicPosition) -> Result<Displayed, Refusal> {
        let Some(display) = self.display_system.as_ref() else {
            return Ok(Displayed::Geographic(position));
        };
        let system = self.coordinate_system.as_ref().ok_or(Refusal::NotStated)?;
        if display.wkt.is_none() && display.epsg.is_some() && display.epsg == system.epsg {
            // The same code names the same system; nothing needs reading to know that.
            return Ok(Displayed::Geographic(position));
        }
        let (to, projected) = match display.reference_system()? {
            ReferenceSystem::Geographic(geographic) => (geographic, None),
            ReferenceSystem::Projected(projected) => (projected.base.clone(), Some(projected)),
        };
        let from = match system.reference_system()? {
            ReferenceSystem::Geographic(geographic) => geographic,
            ReferenceSystem::Projected(projected) => projected.base,
        };
        let shown = datum::transform(&from, &to, position)?;
        Ok(match projected {
            None => Displayed::Geographic(shown),
            Some(projected) => {
                let [easting, northing] = projected.grid(shown)?;
                Displayed::Projected {
                    easting,
                    northing,
                    unit: projected.linear_unit_name,
                }
            }
        })
    }

    /// [`Self::display`] for a caller that writes degrees: the display system's latitude and
    /// longitude, or [`Refusal::DisplayIsProjected`] where `/DCS` is a projected system, whose
    /// easting and northing [`Self::display`] gives.
    ///
    /// # Errors
    ///
    /// What [`Self::display`] refuses, and [`Refusal::DisplayIsProjected`].
    pub fn display_position(
        &self,
        position: GeographicPosition,
    ) -> Result<GeographicPosition, Refusal> {
        match self.display(position)? {
            Displayed::Geographic(shown) => Ok(shown),
            Displayed::Projected { .. } => Err(Refusal::DisplayIsProjected),
        }
    }

    /// A position in the object's own coordinates as a latitude and a longitude: `/PCSM` into
    /// the projected system ([`Self::projected_position`]), then that system's inverse
    /// projection.
    ///
    /// `None` where [`Self::projected_position`] is — no matrix, or a geographic `/GCS` that
    /// Table 269 says makes the matrix one that "should be ignored".
    #[must_use]
    pub fn geographic_position(
        &self,
        position: [f64; 3],
    ) -> Option<Result<GeographicPosition, Refusal>> {
        let [easting, northing, _] = self.projected_position(position)?;
        let system = self.coordinate_system.as_ref()?;
        Some(match system.reference_system() {
            Ok(ReferenceSystem::Projected(projected)) => projected.geographic(easting, northing),
            Ok(ReferenceSystem::Geographic(_)) => Err(Refusal::NotASystem(
                "a PROJCS dictionary whose WKT states a geographic system".to_owned(),
            )),
            Err(refusal) => Err(refusal),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A `/GCS` for ETRS89 / UTM zone 32N, the census's commonest, stated both ways as 41 crawl
    /// documents state it.
    fn utm_32n() -> CoordinateSystem {
        CoordinateSystem {
            projected: true,
            epsg: Some(25832),
            wkt: Some(r#"PROJCS["ETRS89_UTM_zone_32N",GEOGCS["GCS_ETRS_1989",DATUM["D_ETRS_1989",SPHEROID["GRS_1980",6378137,298.257222101]],PRIMEM["Greenwich",0],UNIT["Degree",0.017453292519943295]],PROJECTION["Transverse_Mercator"],PARAMETER["latitude_of_origin",0],PARAMETER["central_meridian",9],PARAMETER["scale_factor",0.9996],PARAMETER["false_easting",500000],PARAMETER["false_northing",0],UNIT["Meter",1]]"#.to_owned()),
        }
    }

    #[test]
    fn a_code_alone_is_refused_by_the_code_and_both_read_the_wkt() {
        let code_only = CoordinateSystem {
            projected: true,
            epsg: Some(25832),
            wkt: None,
        };
        assert_eq!(
            code_only.reference_system(),
            Err(Refusal::EpsgWithoutWkt(25832))
        );
        assert!(matches!(
            utm_32n().reference_system(),
            Ok(ReferenceSystem::Projected(_))
        ));
    }

    #[test]
    fn projected_registration_points_come_back_as_degrees() {
        let geospatial = Geospatial {
            coordinate_system: Some(utm_32n()),
            geographic_points: vec![[500_000.0, 0.0], [500_000.0, 5_000_000.0]],
            local_points: vec![[0.0, 0.0], [0.0, 1.0]],
            ..Geospatial::default()
        };
        let points = geospatial.registration_geographic().expect("degrees");
        assert_eq!(points.len(), 2);
        assert!(points[0].0.latitude.abs() < 1e-12);
        assert!((points[0].0.longitude - 9.0).abs() < 1e-12);
        // Five thousand kilometres up the central meridian, scaled by 0.9996, is about 45°N.
        assert!((points[1].0.latitude - 45.1).abs() < 0.1);
        assert!((points[1].0.longitude - 9.0).abs() < 1e-12);
        assert!(points[1].1[0].abs() < f64::EPSILON && (points[1].1[1] - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn projected_registration_points_shaped_as_degrees_are_the_base_systems_degrees() {
        // A census document's shape: a UTM /GCS whose /GPTS are its corners' latitudes and
        // longitudes (`poppler-101379-0.pdf` in the Tika corpus), read as the base system's
        // degrees (ADR 1918) rather than as eastings a few metres from the grid's origin.
        let geospatial = Geospatial {
            coordinate_system: Some(utm_32n()),
            geographic_points: vec![[43.042_427, 2.742_364], [43.036_308, 2.742_389]],
            local_points: vec![[0.0, 1.0], [0.0, 0.0]],
            ..Geospatial::default()
        };
        assert_eq!(
            geospatial.registration_geographic(),
            Ok(vec![
                (
                    GeographicPosition {
                        latitude: 43.042_427,
                        longitude: 2.742_364
                    },
                    [0.0, 1.0]
                ),
                (
                    GeographicPosition {
                        latitude: 43.036_308,
                        longitude: 2.742_389
                    },
                    [0.0, 0.0]
                ),
            ])
        );
        // One point inside the shape and one outside it is not the shape: both are eastings
        // and northings, as the table states them, so the first is 2.7 m north of the equator
        // and not at 43° N.
        let mixed = Geospatial {
            geographic_points: vec![[43.0, 2.7], [500_000.0, 4_000_000.0]],
            ..geospatial
        };
        let points = mixed.registration_geographic().expect("degrees");
        assert!(points[0].0.latitude.abs() < 0.001, "{points:?}");
        assert!((points[1].0.longitude - 9.0).abs() < 1e-9, "{points:?}");
    }

    #[test]
    fn a_projected_registration_is_fitted_in_the_plane_and_run_backwards() {
        // The drive's map: a UTM zone 32N system registered by a rectangle of the base system's
        // degrees, 47° N to 48° N and 8° E to 10° E, which is no rectangle in the plane.
        let geospatial = Geospatial {
            coordinate_system: Some(utm_32n()),
            geographic_points: vec![[47.0, 8.0], [48.0, 8.0], [48.0, 10.0], [47.0, 10.0]],
            local_points: vec![[0.0, 0.0], [0.0, 1.0], [1.0, 1.0], [1.0, 0.0]],
            ..Geospatial::default()
        };
        let map = geospatial.registration_map().expect("a map in the plane");
        let Ok(ReferenceSystem::Projected(projected)) = utm_32n().reference_system() else {
            panic!("a projected system");
        };
        // A least-squares fit passes through the mean of its points at the mean of their
        // positions in the square, so the middle of the square is the mean of the four corners'
        // eastings and northings, carried back through the inverse.
        let mut mean = [0.0, 0.0];
        for [latitude, longitude] in &geospatial.geographic_points {
            let [easting, northing] = projected
                .grid(GeographicPosition {
                    latitude: *latitude,
                    longitude: *longitude,
                })
                .expect("a grid point");
            mean[0] += easting / 4.0;
            mean[1] += northing / 4.0;
        }
        let expected = projected.geographic(mean[0], mean[1]).expect("a position");
        let middle = map.position([0.5, 0.5]).expect("a position");
        assert!(
            (middle.latitude - expected.latitude).abs() < 1e-12,
            "{middle:?}"
        );
        assert!(
            (middle.longitude - expected.longitude).abs() < 1e-12,
            "{middle:?}"
        );
        // Symmetric about the central meridian, so the middle is on it; and the plane's middle is
        // not the degrees' — the parallels of a transverse Mercator curve.
        assert!((middle.longitude - 9.0).abs() < 1e-9, "{middle:?}");
        assert!((middle.latitude - 47.5).abs() > 1e-4, "{middle:?}");
        // The corners are no affine map's in the plane, and the departure says how far, in
        // degrees: well under a hundredth of a degree for a map one degree tall.
        assert!(
            map.departure() > 0.0 && map.departure() < 0.01,
            "{}",
            map.departure()
        );
        // Run backwards, a position read comes back to the point that read it.
        for local in [[0.25, 0.75], [0.5, 0.5], [0.9, 0.1]] {
            let position = map.position(local).expect("a position");
            let back = map
                .local(position)
                .expect("a grid point")
                .expect("an invertible map");
            assert!((back[0] - local[0]).abs() < 1e-9 && (back[1] - local[1]).abs() < 1e-9);
        }
        // Too few points determine no map, by the same refusal as a geographic system's.
        let two = Geospatial {
            geographic_points: vec![[47.0, 8.0], [48.0, 8.0]],
            local_points: vec![[0.0, 0.0], [0.0, 1.0]],
            ..geospatial
        };
        assert_eq!(two.registration_map(), Err(Refusal::NoMap(2)));
    }

    #[test]
    fn geographic_registration_points_are_the_files_own_degrees() {
        let geospatial = Geospatial {
            coordinate_system: Some(CoordinateSystem {
                projected: false,
                epsg: Some(4326),
                wkt: None,
            }),
            geographic_points: vec![[48.2, 16.37]],
            local_points: vec![[0.5, 0.5]],
            ..Geospatial::default()
        };
        let points = geospatial.registration_geographic().expect("degrees");
        assert_eq!(
            points,
            vec![(
                GeographicPosition {
                    latitude: 48.2,
                    longitude: 16.37
                },
                [0.5, 0.5]
            )]
        );
    }

    #[test]
    fn a_display_system_on_the_same_datum_is_the_same_latitude_and_another_datum_is_refused() {
        let gcs_wgs84 = r#"GEOGCS["GCS_WGS_1984",DATUM["D_WGS_1984",SPHEROID["WGS_1984",6378137.0,298.257223563]],PRIMEM["Greenwich",0.0],UNIT["Degree",0.0174532925199433]]"#;
        let nad27 = r#"GEOGCS["GCS_North_American_1927",DATUM["D_North_American_1927",SPHEROID["Clarke_1866",6378206.4,294.9786982]],PRIMEM["Greenwich",0.0],UNIT["Degree",0.0174532925199433]]"#;
        let geographic = |wkt: &str| CoordinateSystem {
            projected: false,
            epsg: None,
            wkt: Some(wkt.to_owned()),
        };
        let position = GeographicPosition {
            latitude: 30.0,
            longitude: -97.0,
        };
        let same = Geospatial {
            coordinate_system: Some(geographic(gcs_wgs84)),
            display_system: Some(geographic(gcs_wgs84)),
            ..Geospatial::default()
        };
        assert_eq!(same.display_position(position), Ok(position));
        // §12.10.2's own illustration: a map on a 1927 datum displayed in WGS84.
        let shifted = Geospatial {
            coordinate_system: Some(geographic(nad27)),
            display_system: Some(geographic(gcs_wgs84)),
            ..Geospatial::default()
        };
        assert_eq!(
            shifted.display_position(position),
            Err(Refusal::DatumTransformation {
                from: "D_North_American_1927".to_owned(),
                to: "D_WGS_1984".to_owned()
            })
        );
        let none = Geospatial {
            coordinate_system: Some(geographic(nad27)),
            ..Geospatial::default()
        };
        assert_eq!(none.display_position(position), Ok(position));
    }

    #[test]
    fn a_display_system_on_another_datum_is_reached_through_the_parameters_the_file_states() {
        // §12.10.2's illustration, a map on a 1927 datum displayed in WGS84 "corresponding to
        // values reported by a GPS device", where the map's string states its datum's `TOWGS84`
        // (OGC 01-009 section 7.3.18): three translations, the rest zero.
        let nad27 = r#"GEOGCS["GCS_North_American_1927",DATUM["D_North_American_1927",SPHEROID["Clarke_1866",6378206.4,294.9786982],TOWGS84[-8,160,176]],PRIMEM["Greenwich",0.0],UNIT["Degree",0.0174532925199433]]"#;
        let wgs84 = r#"GEOGCS["GCS_WGS_1984",DATUM["D_WGS_1984",SPHEROID["WGS_1984",6378137.0,298.257223563]],PRIMEM["Greenwich",0.0],UNIT["Degree",0.0174532925199433]]"#;
        let system = |wkt: &str, projected: bool| CoordinateSystem {
            projected,
            epsg: None,
            wkt: Some(wkt.to_owned()),
        };
        let geospatial = Geospatial {
            coordinate_system: Some(system(nad27, false)),
            display_system: Some(system(wgs84, false)),
            ..Geospatial::default()
        };
        let position = GeographicPosition {
            latitude: 30.0,
            longitude: -97.0,
        };
        let shown = geospatial
            .display_position(position)
            .expect("a stated transformation");
        // Guidance Note 7-2 section 4.3.1's steps written out: height zero (section 4.1.4), to
        // geocentric on Clarke 1866 (section 4.1.1), the translation (section 4.2.4), and back on
        // WGS 84's ellipsoid.
        let clarke = Ellipsoid::new(6_378_206.4, 294.978_698_2).expect("Clarke 1866");
        let figure = Ellipsoid::new(6_378_137.0, 298.257_223_563).expect("WGS 84");
        let [x, y, z] = clarke.geocentric(
            LatLon {
                latitude: 30f64.to_radians(),
                longitude: (-97f64).to_radians(),
            },
            0.0,
        );
        let (expected, _) = figure.geodetic([x - 8.0, y + 160.0, z + 176.0]);
        assert!(
            (shown.latitude - expected.latitude.to_degrees()).abs() < 1e-12,
            "{shown:?}"
        );
        assert!(
            (shown.longitude - expected.longitude.to_degrees()).abs() < 1e-12,
            "{shown:?}"
        );
        assert!(
            (shown.latitude - 30.0).abs() > 1e-5,
            "the datum moved the latitude"
        );
        // A projected display system on WGS 84 — UTM zone 14N — is reached through the same
        // transformation and then its forward projection.
        let utm_14n = r#"PROJCS["WGS_1984_UTM_Zone_14N",GEOGCS["GCS_WGS_1984",DATUM["D_WGS_1984",SPHEROID["WGS_1984",6378137.0,298.257223563]],PRIMEM["Greenwich",0.0],UNIT["Degree",0.0174532925199433]],PROJECTION["Transverse_Mercator"],PARAMETER["False_Easting",500000.0],PARAMETER["False_Northing",0.0],PARAMETER["Central_Meridian",-99.0],PARAMETER["Scale_Factor",0.9996],PARAMETER["Latitude_Of_Origin",0.0],UNIT["Meter",1.0]]"#;
        let projected = Geospatial {
            display_system: Some(system(utm_14n, true)),
            ..geospatial.clone()
        };
        let Ok(Displayed::Projected {
            easting, northing, ..
        }) = projected.display(position)
        else {
            panic!("an easting and a northing");
        };
        let Ok(ReferenceSystem::Projected(utm)) = system::from_wkt(utm_14n) else {
            panic!("a projected system");
        };
        let [grid_easting, grid_northing] = utm.grid(shown).expect("a grid point");
        assert!((easting - grid_easting).abs() < 1e-6 && (northing - grid_northing).abs() < 1e-6);
        // The same map without its `TOWGS84` states no transformation, and is refused by both
        // datums' names, as before.
        let unstated = Geospatial {
            coordinate_system: Some(system(&nad27.replace(",TOWGS84[-8,160,176]", ""), false)),
            ..geospatial
        };
        assert_eq!(
            unstated.display_position(position),
            Err(Refusal::DatumTransformation {
                from: "D_North_American_1927".to_owned(),
                to: "D_WGS_1984".to_owned()
            })
        );
    }

    #[test]
    fn a_projected_display_system_on_the_same_datum_shows_an_easting_and_a_northing() {
        // Table 269's `/DCS` "shall be used for the display of position values" and may be a
        // projected system; on `/GCS`'s own datum it is reached by its forward projection.
        let etrs89 = r#"GEOGCS["GCS_ETRS_1989",DATUM["D_ETRS_1989",SPHEROID["GRS_1980",6378137,298.257222101]],PRIMEM["Greenwich",0],UNIT["Degree",0.017453292519943295]]"#;
        let utm_33n = utm_32n()
            .wkt
            .expect("a WKT")
            .replace("zone_32N", "zone_33N")
            .replace(r#""central_meridian",9"#, r#""central_meridian",15"#);
        let geospatial = Geospatial {
            coordinate_system: Some(CoordinateSystem {
                projected: false,
                epsg: None,
                wkt: Some(etrs89.to_owned()),
            }),
            display_system: Some(CoordinateSystem {
                projected: true,
                epsg: None,
                wkt: Some(utm_33n.clone()),
            }),
            ..Geospatial::default()
        };
        let on_the_meridian = GeographicPosition {
            latitude: 0.0,
            longitude: 15.0,
        };
        let Ok(Displayed::Projected {
            easting,
            northing,
            unit,
        }) = geospatial.display(on_the_meridian)
        else {
            panic!("an easting and a northing");
        };
        assert!((easting - 500_000.0).abs() < 1e-6 && northing.abs() < 1e-6);
        assert_eq!(unit, "Meter");
        // Off the meridian, the display system's own inverse returns the position given.
        let munich = GeographicPosition {
            latitude: 48.137,
            longitude: 11.575,
        };
        let Ok(Displayed::Projected {
            easting, northing, ..
        }) = geospatial.display(munich)
        else {
            panic!("an easting and a northing");
        };
        let Ok(ReferenceSystem::Projected(display)) = system::from_wkt(&utm_33n) else {
            panic!("a projected system");
        };
        let back = display.geographic(easting, northing).expect("a position");
        assert!((back.latitude - munich.latitude).abs() < 1e-9);
        assert!((back.longitude - munich.longitude).abs() < 1e-9);
        // A caller that writes degrees is told the display system is projected.
        assert_eq!(
            geospatial.display_position(munich),
            Err(Refusal::DisplayIsProjected)
        );
        // On another datum the forward projection is never reached.
        let nad27_utm = utm_33n
            .replace("D_ETRS_1989", "D_North_American_1927")
            .replace(
                r#""GRS_1980",6378137,298.257222101"#,
                r#""Clarke_1866",6378206.4,294.9786982"#,
            );
        let other = Geospatial {
            display_system: Some(CoordinateSystem {
                projected: true,
                epsg: None,
                wkt: Some(nad27_utm),
            }),
            ..geospatial
        };
        assert!(matches!(
            other.display(munich),
            Err(Refusal::DatumTransformation { .. })
        ));
    }

    #[test]
    fn every_method_closes_at_a_census_maps_registration_point() {
        // ADR 1672: one map per method the census's round trip found, its `/GCS` string as the
        // crawl document states it (a literal string's `\` before an ordinary character resolved
        // as §7.3.4.2 says, by dropping it) and its first `/GPTS` pair read as the base system's
        // degrees, as ADR 1918 reads them. The forward projection and the inverse return to the
        // point within ADR 1587's 0.0005″.
        let cases: [(&str, &str, Method, [f64; 2]); 7] = [
            (
                "0100299.pdf",
                r#"PROJCS["NAD_1983_StatePlane_New_York_Central_FIPS_3102_Feet",GEOGCS["GCS_North_American_1983",DATUM["D_North_American_1983",SPHEROID["GRS_1980",6378137.0,298.257222101]],PRIMEM["Greenwich",0.0],UNIT["Degree",0.0174532925199433]],PROJECTION["Transverse_Mercator"],PARAMETER["False_Easting",820208.333],PARAMETER["False_Northing",0.0],PARAMETER["Central_Meridian",-76.5833333333333],PARAMETER["Scale_Factor",0.9999375],PARAMETER["Latitude_Of_Origin",40.0],UNIT["Foot_US",0.3048006096012192]]"#,
                Method::TransverseMercator,
                [42.391_88, -75.512_55],
            ),
            (
                "0792430.pdf",
                r#"PROJCS["NAD_1983_StatePlane_Washington_North_FIPS_4601_Feet",GEOGCS["GCS_North_American_1983",DATUM["D_North_American_1983",SPHEROID["GRS_1980",6378137.0,298.257222101]],PRIMEM["Greenwich",0.0],UNIT["Degree",0.0174532925199433]],PROJECTION["Lambert_Conformal_Conic"],PARAMETER["False_Easting",1640416.666666667],PARAMETER["False_Northing",0.0],PARAMETER["Central_Meridian",-120.8333333333333],PARAMETER["Standard_Parallel_1",47.5],PARAMETER["Standard_Parallel_2",48.73333333333333],PARAMETER["Latitude_Of_Origin",47.0],UNIT["Foot_US",0.3048006096012192]]"#,
                Method::LambertConicConformal2Sp,
                [48.149_04, -122.183_57],
            ),
            (
                "1161520.pdf",
                r#"PROJCS["NAD_1983_HARN_WISCRS_Oneida_County_Feet",GEOGCS["GCS_North_American_1983_HARN",DATUM["D_North_American_1983_HARN",SPHEROID["GRS_1980",6378137.0,298.257222101]],PRIMEM["Greenwich",0.0],UNIT["Degree",0.0174532925199433]],PROJECTION["Lambert_Conformal_Conic"],PARAMETER["False_Easting",230000.0],PARAMETER["False_Northing",188936.744],PARAMETER["Central_Meridian",-89.54444444444444],PARAMETER["Standard_Parallel_1",45.70422377027778],PARAMETER["Scale_Factor",1.0000686968],PARAMETER["Latitude_Of_Origin",45.70422377027778],UNIT["Foot_US",0.3048006096012192]]"#,
                Method::LambertConicConformal1Sp,
                [45.613_72, -89.431_92],
            ),
            (
                "0423430.pdf",
                r#"PROJCS["WGS_1984_Web_Mercator ",GEOGCS["GCS_WGS_1984_Major_AuxSphere",DATUM["D_WGS_1984_Major_AuxSphere",SPHEROID["WGS_1984",6378137.0,0.0]],PRIMEM["Greenwich",0.0],UNIT["Degree",0.0174532925199433]],PROJECTION["Mercator"],PARAMETER["False_Easting",0.0],PARAMETER["False_Northing",0.0],PARAMETER["Central_Meridian",0.0],PARAMETER["Standard_Parallel_1",0.0], UNIT["Meter",1.0]]"#,
                Method::MercatorVariantB,
                [38.2415, -122.993],
            ),
            (
                "0300297.pdf",
                r#"PROJCS["GAM",GEOGCS["GCS_North_American_1983",DATUM["D_North_American_1983",SPHEROID["GRS_1980",6378137.0,298.257222101]],PRIMEM["Greenwich",0.0],UNIT["Degree",0.0174532925199433]],PROJECTION["Albers"],PARAMETER["False_Easting",4921250.0],PARAMETER["False_Northing",19685000.0],PARAMETER["Central_Meridian",-100.0],PARAMETER["Standard_Parallel_1",27.5],PARAMETER["Standard_Parallel_2",35.0],PARAMETER["Latitude_Of_Origin",31.25],UNIT["Foot_US",0.3048006096012192]]"#,
                Method::AlbersEqualArea,
                [29.471_54, -97.657_88],
            ),
            (
                "2514866.pdf",
                r#"PROJCS["Rijksdriehoekstelsel_New",GEOGCS["GCS_Amersfoort",DATUM["D_Amersfoort",SPHEROID["Bessel_1841",6377397.155,299.1528128]],PRIMEM["Greenwich",0.0],UNIT["Degree",0.0174532925199433]],PROJECTION["Double_Stereographic"],PARAMETER["False_Easting",155000.0],PARAMETER["False_Northing",463000.0],PARAMETER["Central_Meridian",5.38763888888889],PARAMETER["Scale_Factor",0.9999079],PARAMETER["Latitude_Of_Origin",52.15616055555555],UNIT["Meter",1.0]]"#,
                Method::ObliqueStereographic,
                [50.586_65, 3.032_52],
            ),
            (
                "6204117.pdf",
                r#"PROJCS["WGS_1984_Web_Mercator_Auxiliary_Sphere",GEOGCS["GCS_WGS_1984",DATUM["D_WGS_1984",SPHEROID["WGS_1984",6378137.0,298.257223563]],PRIMEM["Greenwich",0.0],UNIT["Degree",0.0174532925199433]],PROJECTION["Mercator_Auxiliary_Sphere"],PARAMETER["False_Easting",0.0],PARAMETER["False_Northing",0.0],PARAMETER["Central_Meridian",0.0],PARAMETER["Standard_Parallel_1",0.0],PARAMETER["Auxiliary_Sphere_Type",0.0],UNIT["Meter",1.0]]"#,
                Method::PseudoMercator,
                [-28.603, 21.7683],
            ),
        ];
        let budget = 0.0005 / 3600.0;
        for (file, wkt, method, [latitude, longitude]) in cases {
            let Ok(ReferenceSystem::Projected(projected)) = system::from_wkt(wkt) else {
                panic!("{file}: a projected system");
            };
            assert_eq!(projected.projection.method, method, "{file}");
            let position = GeographicPosition {
                latitude,
                longitude,
            };
            let [easting, northing] = projected.grid(position).expect("a grid point");
            let back = projected.geographic(easting, northing).expect("a position");
            assert!(
                (back.latitude - latitude).abs() <= budget
                    && (back.longitude - longitude).abs() <= budget,
                "{file}: {back:?}"
            );
        }
    }

    #[test]
    fn a_matrix_and_a_projection_together_answer_a_latitude() {
        // `/PCSM` placing the object's origin at (500000, 0) and one unit at one metre; the
        // origin is then the equator on the central meridian.
        let mut matrix = [0.0; 12];
        matrix[0] = 1.0;
        matrix[4] = 1.0;
        matrix[8] = 1.0;
        matrix[9] = 500_000.0;
        let geospatial = Geospatial {
            coordinate_system: Some(utm_32n()),
            projected_matrix: Some(matrix),
            ..Geospatial::default()
        };
        let position = geospatial
            .geographic_position([0.0, 0.0, 0.0])
            .expect("a matrix applies")
            .expect("a position");
        assert!(position.latitude.abs() < 1e-12);
        assert!((position.longitude - 9.0).abs() < 1e-12);
        // Without a matrix there is nothing to apply, which is `None` and not a refusal.
        let without = Geospatial {
            coordinate_system: Some(utm_32n()),
            ..Geospatial::default()
        };
        assert!(without.geographic_position([0.0, 0.0, 0.0]).is_none());
    }
}

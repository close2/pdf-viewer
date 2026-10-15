//! A position carried from one geodetic datum to another, where the file states how.
//!
//! §12.10.2's `/DCS` is the system "that shall be used for the display of position values", and the
//! clause's own illustration is a map on a 1927 datum displayed "in the WGS84 datum corresponding to
//! values reported by a GPS device" — a change of datum. ISO 32000-2 states no parameters for one,
//! and neither does a datum's name: the parameters between two datums are a survey's measurement.
//! **What this module carries is the method; the parameters are the file's** (ADR 1958). A WKT
//! string states them one of two ways, and [`super::system`] reads both into a [`DatumShift`]:
//!
//! - the older form's datum may carry a `TOWGS84` clause, which OGC 01-009 section 7.3.18 defines
//!   as up to seven Bursa Wolf parameters approximating the transformation from that datum to
//!   WGS 84 ([`Helmert::from_towgs84`]);
//! - ISO 19162 section 20's bound CRS states a source system, a target system and an abridged
//!   transformation from the first to the second, which section 20.2.3 writes in metres,
//!   arc-seconds and a scale relative to unity.
//!
//! A datum that states neither, and is not the datum the other system's statement reaches, is
//! refused by both datums' names. No grid of shifts, no registry of parameters and no epoch is
//! carried (`doc/questions/A436`).
//!
//! # The method, and where each step is the Guidance Note's
//!
//! Guidance Note 7-2 section 4.3.1 builds a transformation between two geographic systems as a
//! concatenation — geographic to geocentric, geocentric to geocentric, geocentric to geographic —
//! and its table for the two-dimensional domain names the steps of the Position Vector method one
//! by one:
//!
//! - a latitude and longitude are given a height above the ellipsoid, set to zero, which section
//!   4.1.4 says induces no error into the horizontal position while it is within a few kilometres
//!   of sea level (`ELLIPSOIDAL_HEIGHT`);
//! - they become earth-centred Cartesian coordinates on the source ellipsoid, the longitude first
//!   counted from Greenwich, as section 4.1.1 requires ([`super::Ellipsoid::geocentric`]);
//! - the seven-parameter Helmert transformation of section 4.2.3 carries them to the target datum
//!   ([`Helmert::apply`]);
//! - section 4.1.1's reverse conversion on the target ellipsoid gives the latitude and longitude,
//!   and the height is dropped (section 4.1.4).
//!
//! Each system's string states a transformation *out of* its own datum, so two systems meet on the
//! datum one of them reaches — WGS 84, for two `TOWGS84` clauses — in that datum's geocentric
//! coordinates: the source's parameters carry the point there, and the target's carry it back by
//! section 4.2.3's own reversal, the same formula with the signs of the seven parameters changed.
//!
//! # Two readings of OGC 01-009 this module rests on
//!
//! - **The rotation convention is the Position Vector one.** OGC 01-009 section 10.4's Bursa
//!   Wolf formula writes its matrix with `-ez` above the diagonal in the first row and `+ez` below
//!   it, which is Guidance Note 7-2 section 4.2.3's Position Vector matrix and not its Coordinate
//!   Frame one; the Note's warning that the two differ only in the sign of the rotations is why
//!   this is named rather than assumed. A bound CRS names its method, and a Coordinate Frame one
//!   is held here with its rotations negated, the Note's `rPV = −rCF`.
//! - **The units are metres, arc-seconds and parts per million**: OGC 01-009 section 12.3.27 gives
//!   the shifts in metres, the rotations in arc seconds and the scaling in parts per million, and
//!   section 10.4's formula printing the scale as `1+ppm*1000000` is read by that attribute, parts
//!   per million, as Guidance Note 7-2 Table 5 writes it: `M = 1 + dS·10⁻⁶`.

use super::GeographicPosition;
use super::Refusal;
use super::ellipsoid::Ellipsoid;
use super::projection::LatLon;
use super::system::GeographicSystem;

/// The height above the ellipsoid a two-dimensional position is given before it is made
/// geocentric: Guidance Note 7-2 section 4.1.4 says the assumed height is usually set to zero, and
/// that while it is within a few kilometres of sea level the horizontal position the later
/// geocentric transformation gives is unaffected.
const ELLIPSOIDAL_HEIGHT: f64 = 0.0;

/// Radians per second of arc: the unit OGC 01-009 section 12.3.27 gives a `TOWGS84` rotation in,
/// and ISO 19162 section 20.2.3 an abridged transformation's.
pub(crate) const ARC_SECOND: f64 = std::f64::consts::PI / (180.0 * 3600.0);

/// WGS 84's semi-major axis in metres and inverse flattening, the figure every `TOWGS84` clause's
/// target is on (Guidance Note 7-2 section 4.1.1's example states WGS 84's ellipsoid so).
const WGS84_FIGURE: (f64, f64) = (6_378_137.0, 298.257_223_563);

/// A seven-parameter Helmert transformation between two geocentric systems, in Guidance Note 7-2
/// section 4.2.3's Position Vector convention.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Helmert {
    /// `tX`, `tY`, `tZ`: the translation added to the point's position vector, in metres.
    pub translation: [f64; 3],
    /// `rX`, `rY`, `rZ`: the rotations about the source system's axes, in radians, positive as the
    /// Position Vector convention defines.
    pub rotation: [f64; 3],
    /// `dS`: the scale difference, the scale factor `M` less one.
    pub scale_difference: f64,
}

impl Helmert {
    /// The transformation a WKT1 `TOWGS84` clause states: `dx, dy, dz` in metres, `ex, ey, ez` in
    /// arc-seconds and `ppm` in parts per million (OGC 01-009 section 12.3.27).
    ///
    /// OGC 01-009 section 7.3.18 says that sometimes only the first three or six parameters are
    /// defined, and that the rest must then be zero, so three, six and seven numbers are each a
    /// transformation.
    ///
    /// # Errors
    ///
    /// [`Refusal::DatumShift`] with the count, for any other number of parameters or for one that
    /// is not finite.
    pub fn from_towgs84(values: &[f64]) -> Result<Self, Refusal> {
        let refused = || Refusal::DatumShift {
            count: values.len(),
        };
        if !matches!(values.len(), 3 | 6 | 7) || !values.iter().all(|value| value.is_finite()) {
            return Err(refused());
        }
        let at = |index: usize| values.get(index).copied().unwrap_or(0.0);
        Ok(Self {
            translation: [at(0), at(1), at(2)],
            rotation: [at(3) * ARC_SECOND, at(4) * ARC_SECOND, at(5) * ARC_SECOND],
            scale_difference: at(6) * 1e-6,
        })
    }

    /// The point's geocentric coordinates on the target system — section 4.2.3's Position Vector
    /// formula, `V_T = M · R · V_S + T` with `M = 1 + dS` and `R` the rotation matrix whose small
    /// angles the section writes out.
    #[must_use]
    pub fn apply(&self, source: [f64; 3]) -> [f64; 3] {
        let [x, y, z] = source;
        let [rx, ry, rz] = self.rotation;
        let [tx, ty, tz] = self.translation;
        let m = 1.0 + self.scale_difference;
        [
            m * (x - rz * y + ry * z) + tx,
            m * (rz * x + y - rx * z) + ty,
            m * (-ry * x + rx * y + z) + tz,
        ]
    }

    /// The transformation in the other direction, by the reversal section 4.2.3 says the EPSG
    /// Dataset documents for the method: the same formula with the signs of the translation, the
    /// rotations and the scale difference changed. The section bounds the misclosure of a round
    /// trip so made at two centimetres in practical circumstances; the exact reverse is, in the
    /// section's words, a different method (ADR 1958).
    #[must_use]
    pub fn reversed(&self) -> Self {
        let [tx, ty, tz] = self.translation;
        let [rx, ry, rz] = self.rotation;
        Self {
            translation: [-tx, -ty, -tz],
            rotation: [-rx, -ry, -rz],
            scale_difference: -self.scale_difference,
        }
    }
}

/// A transformation a coordinate system's string states out of its own datum: where it goes, and
/// the parameters that carry a position there.
#[derive(Debug, Clone, PartialEq)]
pub struct DatumShift {
    /// The datum it reaches, by the name the string gives it — `WGS 84` for a `TOWGS84` clause,
    /// the target system's datum for a bound CRS.
    pub datum: String,
    /// That datum's ellipsoid.
    pub ellipsoid: Ellipsoid,
    /// The transformation from the system's own datum to that one.
    pub helmert: Helmert,
}

impl DatumShift {
    /// What a `TOWGS84` clause states: a transformation into WGS 84 (OGC 01-009 section 7.3.18).
    ///
    /// # Errors
    ///
    /// What [`Helmert::from_towgs84`] refuses.
    pub fn to_wgs84(values: &[f64]) -> Result<Self, Refusal> {
        let (axis, inverse_flattening) = WGS84_FIGURE;
        Ok(Self {
            datum: "WGS 84".to_owned(),
            ellipsoid: Ellipsoid::new(axis, inverse_flattening)?,
            helmert: Helmert::from_towgs84(values)?,
        })
    }
}

/// A datum as it is compared: its name reduced to letters and digits, lower-cased, with ESRI's
/// `D_` prefix taken off — ESRI writes a datum `D_WGS_1984` where its ellipsoid is `WGS_1984`, and
/// the prefix is the spelling's and not the datum's — and its ellipsoid.
///
/// **WGS 84 is the one datum known by name**, because it is the one every `TOWGS84` clause is
/// stated into: the three spellings the census's strings and ISO 19162's examples write it —
/// `WGS_1984`, `WGS84`, `World Geodetic System 1984` — are one datum where the ellipsoid is WGS
/// 84's, `a` = 6378137 m and `1/f` = 298.257223563, so that a string naming the datum on another
/// figure is not taken for it (ADR 1958).
#[derive(Debug, Clone, PartialEq)]
struct Datum {
    /// The reduced name.
    name: String,
    /// The ellipsoid.
    ellipsoid: Ellipsoid,
}

impl Datum {
    /// The datum a name and an ellipsoid state.
    fn of(name: &str, ellipsoid: Ellipsoid) -> Self {
        let plain: String = name
            .chars()
            .filter(char::is_ascii_alphanumeric)
            .map(|c| c.to_ascii_lowercase())
            .collect();
        let plain = plain
            .strip_prefix('d')
            .filter(|_| name.starts_with("D_"))
            .map_or_else(|| plain.clone(), str::to_owned);
        let (axis, inverse_flattening) = WGS84_FIGURE;
        #[expect(
            clippy::float_cmp,
            reason = "WGS 84's semi-major axis is a defined whole number of metres"
        )]
        let on_wgs84_figure = ellipsoid.semi_major_axis == axis
            && (ellipsoid.inverse_flattening - inverse_flattening).abs() < 1e-6;
        let wgs84 = on_wgs84_figure
            && matches!(
                plain.as_str(),
                "wgs1984" | "wgs84" | "worldgeodeticsystem1984"
            );
        Self {
            name: if wgs84 { "wgs84".to_owned() } else { plain },
            ellipsoid,
        }
    }

    /// A system's own datum.
    fn of_system(system: &GeographicSystem) -> Self {
        Self::of(&system.datum, system.ellipsoid)
    }
}

/// Whether two geographic systems are on one datum, so that a latitude on one is the same latitude
/// on the other: the same datum by name and the same ellipsoid.
pub(crate) fn same_datum(from: &GeographicSystem, to: &GeographicSystem) -> bool {
    Datum::of_system(from) == Datum::of_system(to)
}

/// The datums a system's position can be carried to in geocentric coordinates, each with the
/// transformation that carries it: its own datum by none, and the datum its string's statement
/// reaches by the statement's parameters.
fn routes(system: &GeographicSystem) -> Vec<(Datum, Helmert)> {
    let mut routes = vec![(Datum::of_system(system), Helmert::default())];
    if let Some(Ok(shift)) = &system.datum_shift {
        routes.push((Datum::of(&shift.datum, shift.ellipsoid), shift.helmert));
    }
    routes
}

/// The transformation from `from`'s geocentric coordinates to `to`'s, where the two strings state
/// one: a datum both reach, `from`'s parameters into it and `to`'s reversed out of it.
fn meeting(from: &GeographicSystem, to: &GeographicSystem) -> Result<(Helmert, Helmert), Refusal> {
    let outward = routes(to);
    for (datum, into) in routes(from) {
        if let Some((_, out_of)) = outward.iter().find(|(other, _)| *other == datum) {
            return Ok((into, out_of.reversed()));
        }
    }
    // A statement this reader could not read is the reason where one was made; otherwise the two
    // strings stated nothing that joins them.
    for system in [from, to] {
        if let Some(Err(refusal)) = &system.datum_shift {
            return Err(refusal.clone());
        }
    }
    Err(Refusal::DatumTransformation {
        from: from.datum.clone(),
        to: to.datum.clone(),
    })
}

/// A latitude and longitude of `from`, in degrees, as a latitude and longitude of `to`, each
/// longitude counted from its own system's prime meridian.
///
/// On one datum ([`same_datum`]) the latitude is unchanged and the longitude moves by the
/// difference of the two prime meridians. On two, the position is carried as the module's
/// documentation states, through a datum the two strings' statements both reach.
///
/// # Errors
///
/// [`Refusal::DatumTransformation`] by both datums' names where the strings state nothing that
/// joins them, and the refusal of a statement this reader cannot read — a `TOWGS84` that is not
/// three, six or seven numbers, a bound CRS's method or parameter — where that statement was the
/// strings' only one.
pub(crate) fn transform(
    from: &GeographicSystem,
    to: &GeographicSystem,
    position: GeographicPosition,
) -> Result<GeographicPosition, Refusal> {
    let (latitude, greenwich) = if same_datum(from, to) {
        (position.latitude, position.longitude + from.prime_meridian)
    } else {
        let (into, out_of) = meeting(from, to)?;
        let source = LatLon {
            latitude: position.latitude.to_radians(),
            longitude: (position.longitude + from.prime_meridian).to_radians(),
        };
        let geocentric = from.ellipsoid.geocentric(source, ELLIPSOIDAL_HEIGHT);
        // Section 4.1.4: the height the reverse conversion gives is dropped.
        let (target, _height) = to.ellipsoid.geodetic(out_of.apply(into.apply(geocentric)));
        (target.latitude.to_degrees(), target.longitude.to_degrees())
    };
    let mut longitude = greenwich - to.prime_meridian;
    if longitude > 180.0 {
        longitude -= 360.0;
    } else if longitude < -180.0 {
        longitude += 360.0;
    }
    Ok(GeographicPosition {
        latitude,
        longitude,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geospatial::Ellipsoid;
    use crate::geospatial::system::{ReferenceSystem, from_wkt};

    /// A geographic system read from a WKT string.
    fn geographic(wkt: &str) -> GeographicSystem {
        match from_wkt(wkt) {
            Ok(ReferenceSystem::Geographic(system)) => system,
            other => panic!("a geographic system: {other:?}"),
        }
    }

    #[test]
    fn the_position_vector_example_is_reproduced() {
        // Guidance Note 7-2 section 4.2.3's example, WGS 72 to WGS 84: tZ = +4.5 m,
        // rZ = +0.554″ = 0.000002685868 rad, dS = +0.219 ppm; the WGS 72 point
        // (3657660.66, 255768.55, 5201382.11) m is (3657660.78, 255778.43, 5201387.75) m on
        // WGS 84, printed to the centimetre — so within one centimetre, the last printed digit:
        // the formula gives 3657660.774 m for the first, which the example prints as .78.
        let helmert = Helmert::from_towgs84(&[0.0, 0.0, 4.5, 0.0, 0.0, 0.554, 0.219])
            .expect("seven parameters");
        // The example's radians, to half their last printed digit.
        assert!((helmert.rotation[2] - 0.000_002_685_868).abs() < 5e-13);
        let [x, y, z] = helmert.apply([3_657_660.66, 255_768.55, 5_201_382.11]);
        assert!((x - 3_657_660.78).abs() < 0.01, "{x}");
        assert!((y - 255_778.43).abs() < 0.01, "{y}");
        assert!((z - 5_201_387.75).abs() < 0.01, "{z}");
        // Reversed by the sign change, the round trip closes within the section's two
        // centimetres; for these parameters it closes far closer.
        let [xs, ys, zs] = helmert.reversed().apply([x, y, z]);
        assert!((xs - 3_657_660.66).abs() < 0.02 && (ys - 255_768.55).abs() < 0.02);
        assert!((zs - 5_201_382.11).abs() < 0.02);
    }

    #[test]
    fn the_coordinate_frame_example_is_the_position_vector_with_the_rotation_negated() {
        // Section 4.2.3 states the same example in the Coordinate Frame convention with
        // rZ = −0.554″ and the same output. `TOWGS84` is the Position Vector convention (OGC
        // 01-009 section 10.4), so the Coordinate Frame parameters are stated here with their
        // rotation negated and give the same point: the convention is the sign, nothing else.
        let coordinate_frame = [0.0, 0.0, 4.5, -0.0, -0.0, -0.554, 0.219];
        let as_position_vector = [
            coordinate_frame[0],
            coordinate_frame[1],
            coordinate_frame[2],
            -coordinate_frame[3],
            -coordinate_frame[4],
            -coordinate_frame[5],
            coordinate_frame[6],
        ];
        let helmert = Helmert::from_towgs84(&as_position_vector).expect("seven parameters");
        let [x, y, _] = helmert.apply([3_657_660.66, 255_768.55, 5_201_382.11]);
        assert!((x - 3_657_660.78).abs() < 0.01 && (y - 255_778.43).abs() < 0.01);
        // And taken as Position Vector parameters unchanged, the point moves the other way.
        let wrong = Helmert::from_towgs84(&coordinate_frame).expect("seven parameters");
        assert!(
            (wrong.apply([3_657_660.66, 255_768.55, 5_201_382.11])[1] - 255_778.43).abs() > 1.0
        );
    }

    #[test]
    fn three_parameters_are_the_geocentric_translations_example() {
        // Section 4.2.4's example, WGS 84 to ED50 in the North Sea: tX = +84.87 m,
        // tY = +96.49 m, tZ = +116.95 m, which section 4.2.4 calls the Helmert transformation with
        // no rotation and no scale — and OGC 01-009 section 7.3.18 lets a `TOWGS84` state only
        // those three.
        let helmert = Helmert::from_towgs84(&[84.87, 96.49, 116.95]).expect("three parameters");
        let [x, y, z] = helmert.apply([3_771_793.97, 140_253.34, 5_124_304.35]);
        assert!((x - 3_771_878.84).abs() < 1e-6, "{x}");
        assert!((y - 140_349.83).abs() < 1e-6, "{y}");
        assert!((z - 5_124_421.30).abs() < 1e-6, "{z}");
        // Section 4.2.4: the method is exactly reversible by the sign change.
        let [xs, ys, zs] = helmert.reversed().apply([x, y, z]);
        assert!((xs - 3_771_793.97).abs() < 1e-6 && (ys - 140_253.34).abs() < 1e-6);
        assert!((zs - 5_124_304.35).abs() < 1e-6);
    }

    #[test]
    fn a_clause_of_another_length_is_refused_by_its_count() {
        assert!(Helmert::from_towgs84(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0]).is_ok());
        for values in [&[][..], &[1.0, 2.0], &[1.0; 4], &[1.0; 8]] {
            assert_eq!(
                Helmert::from_towgs84(values),
                Err(Refusal::DatumShift {
                    count: values.len()
                })
            );
        }
        assert!(Helmert::from_towgs84(&[f64::NAN, 0.0, 0.0]).is_err());
    }

    /// The North Sea point of section 4.1.1's example on WGS 84, as a geographic system's degrees.
    fn north_sea() -> GeographicPosition {
        GeographicPosition {
            latitude: 53.0 + 48.0 / 60.0 + 33.820 / 3600.0,
            longitude: 2.0 + 7.0 / 60.0 + 46.380 / 3600.0,
        }
    }

    const WGS84: &str = r#"GEOGCS["GCS_WGS_1984",DATUM["D_WGS_1984",SPHEROID["WGS_1984",6378137.0,298.257223563]],PRIMEM["Greenwich",0.0],UNIT["Degree",0.0174532925199433]]"#;

    /// ED50 with section 4.2.4's North Sea parameters stated the way `TOWGS84` states them: from
    /// ED50 *to* WGS 84, which is the example's WGS 84 to ED50 translation with its signs changed.
    const ED50: &str = r#"GEOGCS["ED50",DATUM["European_Datum_1950",SPHEROID["International 1924",6378388,297],TOWGS84[-84.87,-96.49,-116.95,0,0,0,0]],PRIMEM["Greenwich",0],UNIT["degree",0.0174532925199433]]"#;

    #[test]
    fn the_concatenation_reproduces_the_north_sea_translation_on_the_ground() {
        // Section 4.1.1's point, its height taken as zero as section 4.1.4 says a two-dimensional
        // position is, carried by section 4.2.4's translation to ED50 and back to latitude and
        // longitude on the International 1924 ellipsoid, then back to WGS 84.
        let (wgs84, ed50) = (geographic(WGS84), geographic(ED50));
        let shown = transform(&wgs84, &ed50, north_sea()).expect("both datums reach WGS 84");
        // The same steps written out from the Note's own formulas, step by step.
        let international = Ellipsoid::new(6_378_388.0, 297.0).expect("International 1924");
        let wgs84_figure = Ellipsoid::new(6_378_137.0, 298.257_223_563).expect("WGS 84");
        let source = LatLon {
            latitude: north_sea().latitude.to_radians(),
            longitude: north_sea().longitude.to_radians(),
        };
        let [x, y, z] = wgs84_figure.geocentric(source, 0.0);
        let (expected, _) = international.geodetic([x + 84.87, y + 96.49, z + 116.95]);
        assert!((shown.latitude - expected.latitude.to_degrees()).abs() < 1e-12);
        assert!((shown.longitude - expected.longitude.to_degrees()).abs() < 1e-12);
        // ED50 lies east and north of WGS 84 in the North Sea, by about a hundred metres: the
        // position on ED50 has the larger latitude and the larger longitude, a few seconds apart.
        let seconds = |degrees: f64| degrees * 3600.0;
        let north = seconds(shown.latitude - north_sea().latitude);
        let east = seconds(shown.longitude - north_sea().longitude);
        assert!(north > 1.0 && north < 6.0, "{north}″");
        assert!(east > 1.0 && east < 8.0, "{east}″");
        // And back: WGS 84's position returns, the translation being exactly reversible.
        let back = transform(&ed50, &wgs84, shown).expect("back to WGS 84");
        let budget = 0.0005 / 3600.0;
        assert!(
            (back.latitude - north_sea().latitude).abs() < budget,
            "{back:?}"
        );
        assert!(
            (back.longitude - north_sea().longitude).abs() < budget,
            "{back:?}"
        );
    }

    #[test]
    fn two_datums_each_stating_parameters_meet_in_wgs84() {
        // §12.10.2's own illustration, a 1927 datum shown on another: NAD27 and ED50 each state a
        // `TOWGS84`, neither is WGS 84, and the position goes through WGS 84 between them.
        let nad27 = geographic(
            r#"GEOGCS["NAD27",DATUM["North_American_Datum_1927",SPHEROID["Clarke 1866",6378206.4,294.9786982138982],TOWGS84[-8,160,176,0,0,0,0]],PRIMEM["Greenwich",0],UNIT["degree",0.0174532925199433]]"#,
        );
        let ed50 = geographic(ED50);
        let wgs84 = geographic(WGS84);
        let austin = GeographicPosition {
            latitude: 30.25,
            longitude: -97.75,
        };
        let direct = transform(&nad27, &ed50, austin).expect("both state parameters");
        let through = transform(
            &wgs84,
            &ed50,
            transform(&nad27, &wgs84, austin).expect("NAD27 reaches WGS 84"),
        )
        .expect("WGS 84 reaches ED50");
        // Stopping on WGS 84's ellipsoid drops the height the first step gave, a few metres, and
        // section 4.1.4 says a height that small moves no horizontal position: the two agree
        // within ADR 1587's half a thousandth of a second.
        let budget = 0.0005 / 3600.0;
        assert!(
            (direct.latitude - through.latitude).abs() < budget,
            "{direct:?} {through:?}"
        );
        assert!(
            (direct.longitude - through.longitude).abs() < budget,
            "{direct:?} {through:?}"
        );
        assert!(
            (direct.latitude - austin.latitude).abs() > 1e-4,
            "the datums differ"
        );
    }

    #[test]
    fn a_datum_stating_no_parameters_is_refused_by_both_names() {
        let nad27 = geographic(
            r#"GEOGCS["GCS_North_American_1927",DATUM["D_North_American_1927",SPHEROID["Clarke_1866",6378206.4,294.9786982]],PRIMEM["Greenwich",0.0],UNIT["Degree",0.0174532925199433]]"#,
        );
        let wgs84 = geographic(WGS84);
        let refused = Err(Refusal::DatumTransformation {
            from: "D_North_American_1927".to_owned(),
            to: "D_WGS_1984".to_owned(),
        });
        assert_eq!(transform(&nad27, &wgs84, north_sea()), refused);
        // WGS 84 named on a sphere is not WGS 84, so it is no meeting place either.
        let mut sphere = wgs84.clone();
        sphere.ellipsoid = Ellipsoid::new(6_378_137.0, 0.0).expect("a sphere");
        assert_ne!(Datum::of_system(&sphere), Datum::of_system(&wgs84));
        assert!(matches!(
            transform(&geographic(ED50), &sphere, north_sea()),
            Err(Refusal::DatumTransformation { .. })
        ));
        // A clause that states the wrong number of parameters is refused by its count, and only
        // when a transformation asks for it: the system itself still reads.
        let short =
            geographic(&ED50.replace("TOWGS84[-84.87,-96.49,-116.95,0,0,0,0]", "TOWGS84[1,2]"));
        assert_eq!(
            transform(&short, &wgs84, north_sea()),
            Err(Refusal::DatumShift { count: 2 })
        );
    }

    #[test]
    fn wgs84_by_any_of_its_spellings_is_the_same_place() {
        // ESRI's `D_WGS_1984`, the older form's `WGS_1984` and ISO 19162's `World Geodetic System
        // 1984` are three spellings of the one datum known by name, each on WGS 84's ellipsoid, so
        // they are one datum and a position on one is the same position on the others.
        let esri = geographic(WGS84);
        let ogc = geographic(
            r#"GEOGCS["WGS 84",DATUM["WGS_1984",SPHEROID["WGS 84",6378137,298.257223563]],PRIMEM["Greenwich",0],UNIT["degree",0.0174532925199433]]"#,
        );
        let iso = geographic(
            r#"GEOGCRS["WGS 84",DATUM["World Geodetic System 1984",ELLIPSOID["WGS 84",6378137,298.257223563]],CS[ellipsoidal,2],AXIS["latitude",north],AXIS["longitude",east],ANGLEUNIT["degree",0.0174532925199433]]"#,
        );
        assert!(same_datum(&esri, &ogc) && same_datum(&ogc, &iso));
        for (from, to) in [(&esri, &ogc), (&ogc, &iso), (&iso, &esri)] {
            let shown = transform(from, to, north_sea()).expect("WGS 84 to WGS 84");
            assert!(
                (shown.latitude - north_sea().latitude).abs() < 1e-9,
                "{shown:?}"
            );
            assert!(
                (shown.longitude - north_sea().longitude).abs() < 1e-9,
                "{shown:?}"
            );
        }
    }

    #[test]
    fn a_bound_crs_meets_a_display_system_on_its_target_datum_and_no_other() {
        // ISO 19162 section 20.1: a bound CRS's transformation goes to its target system, which is
        // not WGS 84 here. A display system on the target's datum is reached; one on WGS 84 is
        // not, because neither string states anything that joins NAD83 to WGS 84.
        let bound = geographic(
            r#"BOUNDCRS[SOURCECRS[GEODCRS["NAD27",DATUM["North American Datum 1927",ELLIPSOID["Clarke 1866",6378206.4,294.978698213]],CS[ellipsoidal,2],AXIS["latitude",north],AXIS["longitude",east],ANGLEUNIT["degree",0.0174532925199433]]],TARGETCRS[GEODCRS["NAD83",DATUM["North American Datum 1983",ELLIPSOID["GRS 1980",6378137,298.2572221]],CS[ellipsoidal,2],AXIS["latitude",north],AXIS["longitude",east],ANGLEUNIT["degree",0.0174532925199433]]],ABRIDGEDTRANSFORMATION["a stated shift",METHOD["Geocentric translations",ID["EPSG",1031]],PARAMETER["X-axis translation",-8],PARAMETER["Y-axis translation",160],PARAMETER["Z-axis translation",176]]]"#,
        );
        let nad83 = geographic(
            r#"GEOGCRS["NAD83",DATUM["North American Datum 1983",ELLIPSOID["GRS 1980",6378137,298.2572221]],CS[ellipsoidal,2],AXIS["latitude",north],AXIS["longitude",east],ANGLEUNIT["degree",0.0174532925199433]]"#,
        );
        let austin = GeographicPosition {
            latitude: 30.25,
            longitude: -97.75,
        };
        let shown = transform(&bound, &nad83, austin).expect("the target's datum");
        let clarke = Ellipsoid::new(6_378_206.4, 294.978_698_213).expect("Clarke 1866");
        let grs80 = Ellipsoid::new(6_378_137.0, 298.257_222_1).expect("GRS 1980");
        let [x, y, z] = clarke.geocentric(
            LatLon {
                latitude: austin.latitude.to_radians(),
                longitude: austin.longitude.to_radians(),
            },
            0.0,
        );
        let (expected, _) = grs80.geodetic([x - 8.0, y + 160.0, z + 176.0]);
        assert!((shown.latitude - expected.latitude.to_degrees()).abs() < 1e-12);
        assert!((shown.longitude - expected.longitude.to_degrees()).abs() < 1e-12);
        // And the other way: the display system's own statement reached backwards.
        let back = transform(&nad83, &bound, shown).expect("from the target's datum");
        assert!(
            (back.latitude - austin.latitude).abs() < 0.0005 / 3600.0,
            "{back:?}"
        );
        assert_eq!(
            transform(&bound, &geographic(WGS84), austin),
            Err(Refusal::DatumTransformation {
                from: "North American Datum 1927".to_owned(),
                to: "D_WGS_1984".to_owned(),
            })
        );
    }

    #[test]
    fn a_prime_meridian_is_counted_from_greenwich_before_the_earth_is_entered() {
        // Section 4.1.1: a system on another prime meridian is first taken to its Greenwich
        // equivalent. A copy of ED50 counting longitudes from Paris — ISO 19162 section 8.2.2's
        // EXAMPLE 1 states that meridian as 2.5969213 grad, 2.33722917° — gives the same point on
        // WGS 84 for a longitude that much smaller.
        let ed50 = geographic(ED50);
        let paris =
            geographic(&ED50.replace(r#"PRIMEM["Greenwich",0]"#, r#"PRIMEM["Paris",2.33722917]"#));
        let wgs84 = geographic(WGS84);
        let on_greenwich = GeographicPosition {
            latitude: 48.8,
            longitude: 2.5,
        };
        let on_paris = GeographicPosition {
            latitude: 48.8,
            longitude: 2.5 - 2.337_229_17,
        };
        let a = transform(&ed50, &wgs84, on_greenwich).expect("ED50 reaches WGS 84");
        let b = transform(&paris, &wgs84, on_paris).expect("so does its Paris copy");
        assert!(
            (a.latitude - b.latitude).abs() < 1e-12 && (a.longitude - b.longitude).abs() < 1e-12
        );
    }
}

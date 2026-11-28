//! ISO 32000-2 §12.10.3's numeric form: a coordinate system named by its EPSG code alone, resolved
//! through the EPSG Dataset this tree carries into the same [`ReferenceSystem`] a `/WKT` string
//! builds.
//!
//! §12.10.3, as Errata Collection 3 Issue #321 corrects it, lets a file describe a system "as a
//! numeric EPSG reference code" instead of WKT, and the code carries none of the system's numbers:
//! they are the registry's. The owner's answer `doc/questions/A1095` carries the registry, and ADR
//! 3821 says how:
//!
//! - `dataset` is the subset, written by `tools/epsg-table.py` from a pinned PROJ release's export
//!   of the EPSG Dataset — every geographic 2D and 3D system, every projected system over one, and
//!   their datums, ellipsoids, prime meridians, conversions, units and method names, deprecated
//!   codes included — as `static` tables sorted by code. Nothing is read or built until a code is
//!   looked up, so the launch path pays nothing for it (CLAUDE.md's "no parsed data at startup").
//! - The numbers are the Dataset's own, unchanged, as IOGP's terms of use (`LICENSE-EPSG`) require:
//!   a parameter the Dataset states in sexagesimal DMS stays so in the table and is converted here,
//!   where it is read (`sexagesimal_dms`).
//! - A system's datum transformation is not part of its code — the Dataset holds many
//!   transformations per datum and the code chooses none — so a system read from a code states
//!   none, as a WKT string with no `TOWGS84` states none (ADR 1958).
//!
//! # Where the table comes from, and how to make it again
//!
//! PROJ 9.9.0's release archive (`https://download.osgeo.org/proj/proj-9.9.0.tar.gz`, SHA-256
//! `791a0610547eeabb17006cfd49cdbd2034f3240f47ed5e88a1031811f4e2bcf3`) carries the EPSG Dataset
//! v13.102 as SQL in its `data/sql/`. `python3 -I -B tools/epsg-table.py --fetch` downloads and
//! checks it, and `python3 -I -B tools/epsg-table.py` writes `dataset.rs` from it; neither runs at
//! build time or at launch. The ignored test `the_table_is_what_the_generator_writes_from_the_pinned_release`
//! does both and fails where the file in the tree differs by a byte. PROJ's export holds seven
//! parameters a conversion, so Krovak Modified's evaluation point and C1 to C10 are read from
//! the registry's own records of its four conversions
//! (`https://apps.epsg.org/api/v1/Conversion/<code>/`), which `--fetch` saves and the generator
//! holds to pinned digests (ADR 3897). The Dataset is IOGP's, and
//! `LICENSE-EPSG` and `NOTICE` at the repository root carry its terms and the acknowledgement they
//! require.
//!
//! A code the table does not hold is refused by its number ([`Refusal::EpsgNotCarried`]), and a
//! projected system whose method this tree does not evaluate by its number and the method's name
//! ([`Refusal::EpsgMethod`]) — never by guessing a neighbouring system.

use super::projection::{
    Method as ProjectionMethod, Parameters, Projection, parallel_from_natural_origin,
};
use super::system::{GeographicSystem, ProjectedSystem, ReferenceSystem};
use super::{Ellipsoid, Refusal};

// The generated file is one item per line by design, so that a regeneration's diff is one line
// per changed record; rustfmt would wrap the long ones.
#[rustfmt::skip]
#[expect(
    clippy::unreadable_literal,
    clippy::excessive_precision,
    reason = "every number is the EPSG Dataset's text, unchanged as IOGP's terms require (ADR 3821): \
              a separator or a rounding would be a spelling of ours"
)]
mod dataset;

pub use dataset::VERSION;

/// The kind of quantity a unit measures.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum UnitKind {
    /// Metres per unit.
    Length,
    /// Radians per unit.
    Angle,
    /// A ratio per unit.
    Scale,
}

/// A unit of measure: code, kind, the factor to the kind's base unit where the Dataset states
/// one, and name.
struct Unit(u32, UnitKind, Option<f64>, &'static str);

/// An ellipsoid's second defining parameter, as the Dataset states it.
enum Figure {
    /// `1/f`, zero for a sphere.
    InverseFlattening(f64),
    /// The semi-minor axis, in the semi-major axis's unit.
    SemiMinorAxis(f64),
}

/// An ellipsoid: code, semi-major axis, its unit's code, and the second parameter.
struct Spheroid(u32, f64, u32, Figure);

/// A prime meridian: code, longitude from Greenwich, and its unit's code.
struct Meridian(u32, f64, u32);

/// A geodetic datum or datum ensemble: code, name, ellipsoid code, prime meridian code.
struct Datum(u32, &'static str, u32, u32);

/// A geographic 2D or 3D system: code, name, datum code, the angular unit's code. A 3D system's
/// third axis is a height, which §12.10's positions do not carry.
struct Geographic(u32, &'static str, u32, u32);

/// A projection method: code and the Dataset's name for it.
struct Method(u32, &'static str);

/// A parameter of a conversion: the parameter's code, its value, its unit's code.
struct Parameter(u32, f64, u32);

/// A conversion: code, method code, parameters.
struct Conversion(u32, u32, &'static [Parameter]);

/// A projected system: code, name, base geographic system's code, conversion code, the code of
/// its axes' unit.
struct Projected(u32, &'static str, u32, u32, u32);

/// The record of `table` whose code is `code`; every table is sorted by code.
fn find<T>(table: &'static [T], key: impl Fn(&T) -> u32, code: u32) -> Option<&'static T> {
    // not a repair: a lookup; a code the table does not hold is `None`, which every caller refuses
    // by its number.
    table
        .binary_search_by_key(&code, key)
        .ok()
        .map(|index| &table[index])
}

/// The record a code the generator wrote as a reference names. The generator writes a record for
/// every code another record names (the closure in `tools/epsg-table.py`), and
/// `every_reference_in_the_table_resolves` checks it, so a miss is a table this program was not
/// built with; it is refused by the code that named it rather than assumed.
fn referenced<T>(
    table: &'static [T],
    key: impl Fn(&T) -> u32,
    code: u32,
    from: u32,
) -> Result<&'static T, Refusal> {
    find(table, key, code).ok_or(Refusal::EpsgNotCarried(i64::from(from)))
}

/// EPSG unit 9110's sexagesimal DMS number, `DDD.MMSSsss`, as decimal degrees.
///
/// The Dataset states most parameters in this packed form, whose digits after the point are
/// minutes and seconds rather than a fraction of a degree. The nudge before the minutes are cut
/// keeps a value written `.30` — thirty minutes, which a binary fraction holds as 29.999… — on the
/// minute its digits say; it is far below the last digit any record states.
fn sexagesimal_dms(value: f64) -> f64 {
    let magnitude = value.abs();
    let degrees = magnitude.trunc();
    let minutes_and_seconds = (magnitude - degrees) * 100.0;
    let minutes = (minutes_and_seconds + 1e-8).trunc();
    let seconds = (minutes_and_seconds - minutes) * 100.0;
    (degrees + minutes / 60.0 + seconds / 3600.0).copysign(value)
}

/// The EPSG code of sexagesimal DMS.
const SEXAGESIMAL_DMS: u32 = 9110;

/// Unit 9108, degree minute second hemisphere: a way of *writing* degrees, which a number in a
/// PDF file cannot take; its value is degrees, so its factor is a degree's.
const DMS_HEMISPHERE: u32 = 9108;

/// A value in the unit `code` names, converted to the unit's kind's base: metres, radians, or a
/// ratio.
fn base_value(value: f64, code: u32, kind: UnitKind, from: u32) -> Result<f64, Refusal> {
    let unit = referenced(dataset::UNITS, |u| u.0, code, from)?;
    if unit.1 != kind {
        return Err(Refusal::Unit(unit.3.to_owned()));
    }
    match (unit.2, code) {
        (Some(factor), _) => Ok(value * factor),
        (None, SEXAGESIMAL_DMS) => Ok(sexagesimal_dms(value).to_radians()),
        (None, DMS_HEMISPHERE) => Ok(value.to_radians()),
        (None, _) => Err(Refusal::Unit(unit.3.to_owned())),
    }
}

/// A scale factor in the unit `code` names.
///
/// Four Transverse Mercator systems of the Dataset (the Fiji grids among them) state their scale
/// factor at the natural origin in unit 9001, the metre, where every other states it in a scale
/// unit. A scale factor is a ratio, and the metre's factor is one, so the number is read as the
/// ratio it states rather than refused: a choice, since the Dataset's record is self-inconsistent
/// and no clause decides it (ADR 3821).
fn scale_value(value: f64, code: u32, from: u32) -> Result<f64, Refusal> {
    let unit = referenced(dataset::UNITS, |u| u.0, code, from)?;
    match (unit.1, unit.2) {
        (UnitKind::Length, Some(factor)) if (factor - 1.0).abs() < f64::EPSILON => Ok(value),
        _ => base_value(value, code, UnitKind::Scale, from),
    }
}

/// The geographic or projected system an EPSG code names.
///
/// # Errors
///
/// [`Refusal::EpsgNotCarried`] for a code that names no geographic or projected system of the
/// carried Dataset, [`Refusal::EpsgMethod`] for a projected system whose method this program does
/// not evaluate, and what [`Ellipsoid::new`] and [`Projection::new`] refuse of the Dataset's
/// numbers.
pub fn system(code: i64) -> Result<ReferenceSystem, Refusal> {
    let not_carried = Refusal::EpsgNotCarried(code);
    let code = u32::try_from(code).map_err(|_| not_carried.clone())?;
    if let Some(record) = find(dataset::GEOGRAPHIC, |g| g.0, code) {
        return geographic(record).map(ReferenceSystem::Geographic);
    }
    let record = find(dataset::PROJECTED, |p| p.0, code).ok_or(not_carried)?;
    projected(record).map(ReferenceSystem::Projected)
}

/// A geographic system's record as the system it describes.
fn geographic(record: &Geographic) -> Result<GeographicSystem, Refusal> {
    let &Geographic(code, name, datum_code, angle_unit) = record;
    let datum = referenced(dataset::DATUMS, |d| d.0, datum_code, code)?;
    let spheroid = referenced(dataset::ELLIPSOIDS, |e| e.0, datum.2, code)?;
    let meridian = referenced(dataset::PRIME_MERIDIANS, |m| m.0, datum.3, code)?;

    let axis = base_value(spheroid.1, spheroid.2, UnitKind::Length, code)?;
    let inverse_flattening = match spheroid.3 {
        Figure::InverseFlattening(value) => value,
        // A semi-minor axis equal to the semi-major one is a sphere, which `Ellipsoid` writes as an
        // inverse flattening of zero; otherwise `1/f = a / (a − b)`, in the one unit both share.
        Figure::SemiMinorAxis(minor) => {
            let difference = spheroid.1 - minor;
            if difference > 0.0 {
                spheroid.1 / difference
            } else {
                0.0
            }
        }
    };
    Ok(GeographicSystem {
        name: name.to_owned(),
        datum: datum.1.to_owned(),
        ellipsoid: Ellipsoid::new(axis, inverse_flattening)?,
        prime_meridian: base_value(meridian.1, meridian.2, UnitKind::Angle, code)?.to_degrees(),
        angular_unit: base_value(1.0, angle_unit, UnitKind::Angle, code)?,
        datum_shift: None,
    })
}

/// The method of this program's an EPSG method code names, where it evaluates one.
fn method(code: u32) -> Option<ProjectionMethod> {
    // The method codes Guidance Note 7-2 prints beside each method's section.
    Some(match code {
        9807 => ProjectionMethod::TransverseMercator,
        9802 => ProjectionMethod::LambertConicConformal2Sp,
        9801 => ProjectionMethod::LambertConicConformal1Sp,
        9804 => ProjectionMethod::MercatorVariantA,
        9805 => ProjectionMethod::MercatorVariantB,
        1024 => ProjectionMethod::PseudoMercator,
        9822 => ProjectionMethod::AlbersEqualArea,
        9809 => ProjectionMethod::ObliqueStereographic,
        9806 => ProjectionMethod::CassiniSoldner,
        9820 => ProjectionMethod::LambertAzimuthalEqualArea,
        9810 => ProjectionMethod::PolarStereographicA,
        9829 => ProjectionMethod::PolarStereographicB,
        9830 => ProjectionMethod::PolarStereographicC,
        9808 => ProjectionMethod::TransverseMercatorSouthOrientated,
        9826 => ProjectionMethod::LambertConicConformalWestOrientated,
        9812 => ProjectionMethod::HotineObliqueMercatorA,
        9815 => ProjectionMethod::HotineObliqueMercatorB,
        1052 => ProjectionMethod::ColombiaUrban,
        1027 => ProjectionMethod::LambertAzimuthalEqualAreaSpherical,
        9818 => ProjectionMethod::AmericanPolyconic,
        1051 => ProjectionMethod::LambertConicConformal2SpMichigan,
        9819 => ProjectionMethod::Krovak,
        1041 => ProjectionMethod::KrovakNorthOrientated,
        1042 => ProjectionMethod::KrovakModified,
        1043 => ProjectionMethod::KrovakModifiedNorthOrientated,
        1078 => ProjectionMethod::EqualEarth,
        9835 => ProjectionMethod::LambertCylindricalEqualArea,
        9834 => ProjectionMethod::LambertCylindricalEqualAreaSpherical,
        // 9842 and 9823 are the codes 1028 and 1029 replaced; `projected` reads their
        // parameters as `parallel_from_natural_origin` says.
        1028 | 9842 => ProjectionMethod::EquidistantCylindrical,
        1029 | 9823 => ProjectionMethod::EquidistantCylindricalSpherical,
        9827 => ProjectionMethod::Bonne,
        9828 => ProjectionMethod::BonneSouthOrientated,
        9813 => ProjectionMethod::LabordeObliqueMercator,
        9824 => ProjectionMethod::TransverseMercatorZonedGridSystem,
        9833 => ProjectionMethod::HyperbolicCassiniSoldner,
        9832 => ProjectionMethod::ModifiedAzimuthalEquidistant,
        9831 => ProjectionMethod::GuamProjection,
        // 9841 is the code 1026 replaced, which also stated a scale factor (ADR 3886).
        1026 | 9841 => ProjectionMethod::MercatorSpherical,
        9803 => ProjectionMethod::LambertConicConformal2SpBelgium,
        9817 => ProjectionMethod::LambertConicNearConformal,
        9811 => ProjectionMethod::NewZealandMapGrid,
        9816 => ProjectionMethod::TunisiaMiningGrid,
        1111 => ProjectionMethod::TransverseMercator3D,
        _ => return None,
    })
}

/// A projected system's record as the system it describes.
fn projected(record: &Projected) -> Result<ProjectedSystem, Refusal> {
    let &Projected(code, name, base_code, conversion_code, unit_code) = record;
    let base = geographic(referenced(dataset::GEOGRAPHIC, |g| g.0, base_code, code)?)?;
    let conversion = referenced(dataset::CONVERSIONS, |c| c.0, conversion_code, code)?;
    let Some(method) = method(conversion.1) else {
        let named = referenced(dataset::METHODS, |m| m.0, conversion.1, code)?;
        return Err(Refusal::EpsgMethod {
            code: i64::from(code),
            method: named.1,
            method_code: i64::from(conversion.1),
        });
    };

    let mut parameters = Parameters::default();
    for &Parameter(parameter, value, unit) in conversion.2 {
        let angle = || base_value(value, unit, UnitKind::Angle, code);
        let length = || base_value(value, unit, UnitKind::Length, code);
        // The parameter codes Guidance Note 7-2 prints beside each method; a natural and a false
        // origin play one role, as `super::system`'s reading of a WKT string's identifiers has it.
        match parameter {
            // Hotine's projection centre (8811, 8812), scale on the initial line (8815) and
            // easting and northing at the centre (8816, 8817) play the same roles.
            8801 | 8811 | 8821 => parameters.origin_latitude = Some(angle()?),
            8802 | 8812 | 8822 | 8833 => parameters.origin_longitude = Some(angle()?),
            // Krovak's scale on the pseudo standard parallel (8819) is its one scale factor.
            8805 | 8815 | 8819 => parameters.scale = Some(scale_value(value, unit, code)?),
            8806 | 8816 | 8826 => parameters.false_easting = Some(length()?),
            8807 | 8817 | 8827 => parameters.false_northing = Some(length()?),
            // Krovak's co-latitude of the cone axis (1036) is the αC Hotine's azimuth is.
            8813 | 1036 => parameters.azimuth = Some(angle()?),
            8814 => parameters.rectified_grid_angle = Some(angle()?),
            1039 => parameters.origin_height = Some(length()?),
            // Polar Stereographic's latitude of standard parallel is its one parallel, and its
            // longitude of origin plays the natural origin's role,
            // and Krovak's pseudo standard parallel (8818) is its one parallel too.
            8823 | 8832 | 8818 => parameters.parallel_1 = Some(angle()?),
            1038 => parameters.ellipsoid_scale = Some(scale_value(value, unit, code)?),
            8617 => parameters.evaluation_point[0] = Some(length()?),
            8618 => parameters.evaluation_point[1] = Some(length()?),
            // Krovak Modified's C1 to C10 (1026 to 1035), each a coefficient of unity.
            1026..=1035 => {
                let slot = (1026..=1035)
                    .position(|known: u32| known == parameter)
                    .and_then(|index| parameters.coefficients.get_mut(index));
                if let Some(slot) = slot {
                    *slot = Some(scale_value(value, unit, code)?);
                }
            }
            8824 => parameters.parallel_2 = Some(angle()?),
            8830 => parameters.initial_longitude = Some(angle()?),
            8831 => parameters.zone_width = Some(angle()?),
            other => {
                // Every parameter of the methods built is one of the codes above; another is one
                // these formulas would ignore, and so evaluate a different system than the code's.
                return Err(Refusal::UnknownParameter {
                    method: method.name().to_owned(),
                    parameter: format!("EPSG parameter {other}"),
                });
            }
        }
    }
    if matches!(conversion.1, 9823 | 9842) {
        parallel_from_natural_origin(&mut parameters);
    }
    let unit = referenced(dataset::UNITS, |u| u.0, unit_code, code)?;
    let linear_unit = base_value(1.0, unit_code, UnitKind::Length, code)?;
    let ellipsoid = base.ellipsoid;
    Ok(ProjectedSystem {
        name: name.to_owned(),
        projection: Projection::new(method, ellipsoid, parameters)?,
        base,
        linear_unit,
        linear_unit_name: unit.3.to_owned(),
    })
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod method_tests;

#[cfg(test)]
mod one_system_tests;

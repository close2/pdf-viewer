//! What a Well Known Text tree means: a geographic system, or a projected one with its method
//! and parameters — ISO 19162's keywords and the older ones its Annex C carries.
//!
//! The census decides which of the two dialects this module must read, and the answer inverts
//! the expectation (ADR 1586): every WKT string of the crawl is the older form — `PROJCS`,
//! `GEOGCS`, `PROJECTION`, `PARAMETER` — which ISO 19162 Annex C documents as backward
//! compatibility, and none is the `PROJCRS` form of ISO 19162's own clauses 8 and 9. Both are
//! read, because §12.10.3 names ISO 19162 and a file conforming to that text writes the newer
//! form; the older form is read because it is what the world's files carry.
//!
//! # Two choices the texts leave open, both ADR 1587's
//!
//! - **The units of an older string's parameters.** ISO 19162 section C.3.4 says the older
//!   form's parameter units are ambiguous. This reader takes the convention every census
//!   document follows — an angle in the base system's angular unit, a length in the projected
//!   system's linear unit — because the census's State Plane systems state their false eastings
//!   in US survey feet beside `UNIT["Foot_US",…]`, and no reading of them in metres puts the
//!   origin where the system's name says it is. The newer form states a unit per parameter, and
//!   where one is omitted ISO 19162 section 9.3.4 makes a length metres and an angle degrees.
//! - **A method named for a family.** The older form names a projection rather than a method
//!   (section C.3.4 again), and two names in the census stand for two methods each:
//!   `Lambert_Conformal_Conic` is the two-parallel method where two distinct standard parallels
//!   are stated and the one-parallel method otherwise, and `Mercator` is variant B where a
//!   standard parallel is stated and variant A where a scale factor is. A combination neither
//!   method defines is refused by name rather than resolved.
//!
//! A datum's transformation to another is read where the string states one: the older form's
//! `TOWGS84` clause, the parameters that carry the datum to WGS 84 (OGC 01-009 section 7.3.18), and
//! ISO 19162 section 20's bound CRS, a system with an abridged transformation to a target system.
//! [`super::datum`] is what they are used for, and only a display system on another datum asks for
//! them (ADR 1958).

use super::Refusal;
use super::datum::{ARC_SECOND, DatumShift, Helmert};
use super::ellipsoid::Ellipsoid;
use super::projection::{Method, Parameters, Projection};
use super::wkt::{self, Attribute, Node};

/// A geographic coordinate system: an ellipsoid and the meridian longitudes count from.
#[derive(Debug, Clone, PartialEq)]
pub struct GeographicSystem {
    /// The system's name, as the string states it.
    pub name: String,
    /// Its datum's name, as the string states it.
    pub datum: String,
    /// The ellipsoid of its datum.
    pub ellipsoid: Ellipsoid,
    /// The prime meridian's longitude east of Greenwich, in degrees.
    pub prime_meridian: f64,
    /// Radians per unit of the system's angles.
    pub angular_unit: f64,
    /// The transformation out of its datum that the string states — a `TOWGS84` clause, or a
    /// bound CRS's abridged transformation — or `None` where it states none. A statement this
    /// reader cannot read is kept as its refusal rather than refusing the system, because only a
    /// change of datum asks for it.
    pub datum_shift: Option<Result<DatumShift, Refusal>>,
}

/// A projected coordinate system: a base geographic system, a projection from it, and the unit
/// its eastings and northings are counted in.
#[derive(Debug, Clone, PartialEq)]
pub struct ProjectedSystem {
    /// The system's name, as the string states it.
    pub name: String,
    /// The geographic system the projection is from.
    pub base: GeographicSystem,
    /// The projection, its parameters converted to radians and metres.
    pub projection: Projection,
    /// Metres per unit of the system's eastings and northings.
    pub linear_unit: f64,
    /// That unit's name as the string states it, or `metre` where ISO 19162 section 7.4.4's
    /// implied default applies — what a host writes beside an easting it displays.
    pub linear_unit_name: String,
}

/// The system a `/WKT` string states.
#[derive(Debug, Clone, PartialEq)]
pub enum ReferenceSystem {
    /// Latitudes and longitudes.
    Geographic(GeographicSystem),
    /// Eastings and northings.
    Projected(ProjectedSystem),
}

/// The keywords of a geographic system: the older `GEOGCS` and ISO 19162 section 8's own.
const GEOGRAPHIC: &[&str] = &[
    "GEOGCS",
    "GEOGCRS",
    "GEOGRAPHICCRS",
    "GEODCRS",
    "GEODETICCRS",
    "BASEGEOGCRS",
    "BASEGEODCRS",
];

/// The keywords of a projected system.
const PROJECTED: &[&str] = &["PROJCS", "PROJCRS", "PROJECTEDCRS"];

/// Reads a `/WKT` string as the system it states.
///
/// # Errors
///
/// A [`Refusal`] naming what the string does not state or what this reader does not carry: a
/// string that is not Well Known Text, a keyword that is not a geographic or projected system,
/// a component missing, a method by its name, a parameter by its role.
pub fn from_wkt(text: &str) -> Result<ReferenceSystem, Refusal> {
    let node = wkt::parse(text).map_err(Refusal::Wkt)?;
    if node.keyword == "BOUNDCRS" {
        bound(&node)
    } else {
        system(&node)
    }
}

/// A geographic or projected system, from its outermost token.
fn system(node: &Node) -> Result<ReferenceSystem, Refusal> {
    if PROJECTED.contains(&node.keyword.as_str()) {
        projected(node).map(ReferenceSystem::Projected)
    } else if GEOGRAPHIC.contains(&node.keyword.as_str()) {
        geographic(node).map(ReferenceSystem::Geographic)
    } else {
        Err(Refusal::NotASystem(node.keyword.clone()))
    }
}

/// ISO 19162 section 20.1's bound CRS: the source system, with the abridged transformation to the
/// target system kept on its geographic system — on a projected source's base system, which
/// section 20.1 says a transformation in the geodetic domain is deemed to operate on.
fn bound(node: &Node) -> Result<ReferenceSystem, Refusal> {
    let missing = |what: &'static str| Refusal::Missing {
        what,
        of: node.to_string(),
    };
    // Section 20.1: `SOURCECRS` and `TARGETCRS` each hold one full CRS description.
    let inner = |keyword: &str, what: &'static str| {
        node.child(&[keyword])
            .and_then(|holder| {
                holder
                    .attributes
                    .iter()
                    .find_map(|attribute| match attribute {
                        Attribute::Node(inner) => Some(inner),
                        _ => None,
                    })
            })
            .ok_or_else(|| missing(what))
    };
    let mut source = system(inner("SOURCECRS", "a source system")?)?;
    let target = match system(inner("TARGETCRS", "a target system")?)? {
        ReferenceSystem::Geographic(geographic) => geographic,
        ReferenceSystem::Projected(projected) => projected.base,
    };
    let shift = node
        .child(&["ABRIDGEDTRANSFORMATION"])
        .ok_or_else(|| missing("an abridged transformation"))
        .and_then(|transformation| abridged(transformation, &target));
    let base = match &mut source {
        ReferenceSystem::Geographic(geographic) => geographic,
        ReferenceSystem::Projected(projected) => &mut projected.base,
    };
    base.datum_shift = Some(shift);
    Ok(source)
}

/// The Helmert family's three methods as Guidance Note 7-2 sections 4.2.3 and 4.2.4 name them,
/// each by the EPSG codes the Note prints for its geocentric, two-dimensional and
/// three-dimensional forms (section 4.3.1's tables).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Shift {
    /// Geocentric translations, three parameters (section 4.2.4).
    Translations,
    /// The Position Vector transformation, seven (section 4.2.3).
    PositionVector,
    /// The Coordinate Frame rotation, seven, the rotations of the opposite sign (section 4.2.3).
    CoordinateFrame,
}

/// The method a bound CRS's `METHOD` names, by code or by name; `None` for any other — a grid's,
/// a ten-parameter one, a time-dependent one.
fn shift_method(name: &str, code: Option<i64>) -> Option<Shift> {
    if let Some(code) = code {
        return match code {
            1031 | 9603 | 1035 => Some(Shift::Translations),
            1033 | 9606 | 1037 => Some(Shift::PositionVector),
            1032 | 9607 | 1038 => Some(Shift::CoordinateFrame),
            _ => None,
        };
    }
    // The Note's names, with its domain in parentheses or without it. Table F.4 of ISO 19162 gives
    // `Bursa-Wolf` and `Helmert` as aliases of both seven-parameter methods, so neither alias
    // names one, and neither is read.
    let plain = normalised(name);
    let plain = ["geocentricdomain", "geog2ddomain", "geog3ddomain"]
        .iter()
        .find_map(|domain| plain.strip_suffix(domain))
        .unwrap_or(&plain);
    match plain {
        "geocentrictranslations" => Some(Shift::Translations),
        "positionvectortransformation" | "positionvector7paramtransformation" => {
            Some(Shift::PositionVector)
        }
        "coordinateframerotation" => Some(Shift::CoordinateFrame),
        _ => None,
    }
}

/// The parameter of the Helmert family a `PARAMETER` is, by its EPSG code (8605 to 8611, as ISO
/// 19162 Table F.4 lists them for the three methods) or its name: an index into
/// `[tX, tY, tZ, rX, rY, rZ, scale]`.
fn shift_parameter(name: &str, code: Option<i64>) -> Option<usize> {
    if let Some(code) = code {
        return code
            .checked_sub(8605)
            .and_then(|index| usize::try_from(index).ok())
            .filter(|index| *index < SHIFT_PARAMETERS.len());
    }
    [
        "xaxistranslation",
        "yaxistranslation",
        "zaxistranslation",
        "xaxisrotation",
        "yaxisrotation",
        "zaxisrotation",
        "scaledifference",
    ]
    .iter()
    .position(|known| *known == normalised(name))
}

/// The Helmert family's seven parameters by Guidance Note 7-2 Table 5's names, in the order the
/// EPSG codes 8605 to 8611 give them.
const SHIFT_PARAMETERS: [&str; 7] = [
    "X-axis translation",
    "Y-axis translation",
    "Z-axis translation",
    "X-axis rotation",
    "Y-axis rotation",
    "Z-axis rotation",
    "scale difference",
];

/// An abridged transformation of the Helmert family, from the source system's datum to `target`'s.
///
/// ISO 19162 section 20.2.3 fixes the units the parameters are written in: a length in metres, an
/// angle in arc-seconds, and a scale as a number relative to unity, so that 3.5 parts per million
/// is written 1.0000035 — whence the scale difference is the stated number less one. A
/// Coordinate Frame rotation is held with its rotations negated, Guidance Note 7-2 section 4.2.3's
/// `rPV = −rCF`.
fn abridged(transformation: &Node, target: &GeographicSystem) -> Result<DatumShift, Refusal> {
    let method_node = transformation
        .child(&["METHOD"])
        .ok_or_else(|| Refusal::Missing {
            what: "a method",
            of: transformation.to_string(),
        })?;
    let method_name = method_node.name().unwrap_or("").to_owned();
    let method = shift_method(&method_name, epsg_code(method_node))
        .ok_or_else(|| Refusal::UnsupportedTransformation(method_name.clone()))?;
    if transformation.child(&["PARAMETERFILE"]).is_some() {
        // Section 20.2.4's parameter file is a grid's, which no method of the family reads.
        return Err(Refusal::UnsupportedTransformation(method_name));
    }
    let mut values: [Option<f64>; 7] = [None; 7];
    for parameter in transformation.children("PARAMETER") {
        let name = parameter.name().unwrap_or("");
        let index = shift_parameter(name, epsg_code(parameter))
            .filter(|index| method != Shift::Translations || *index < 3)
            .ok_or_else(|| Refusal::UnknownParameter {
                method: method_name.clone(),
                parameter: name.to_owned(),
            })?;
        let value = parameter
            .number(1)
            .filter(|value| value.is_finite())
            .ok_or_else(|| Refusal::Missing {
                what: "a parameter's value",
                of: transformation.to_string(),
            })?;
        if let Some(slot) = values.get_mut(index) {
            *slot = Some(value);
        }
    }
    let needed = if method == Shift::Translations { 3 } else { 7 };
    let mut stated = [0.0; 7];
    for (index, (value, slot)) in values
        .iter()
        .zip(stated.iter_mut())
        .enumerate()
        .take(needed)
    {
        *slot = value.ok_or(Refusal::MissingParameter {
            method: "a Helmert transformation",
            parameter: SHIFT_PARAMETERS.get(index).copied().unwrap_or("parameter"),
        })?;
    }
    let sign = if method == Shift::CoordinateFrame {
        -1.0
    } else {
        1.0
    };
    let [tx, ty, tz, rx, ry, rz, scale] = stated;
    Ok(DatumShift {
        datum: target.datum.clone(),
        ellipsoid: target.ellipsoid,
        helmert: Helmert {
            translation: [tx, ty, tz],
            rotation: [rx, ry, rz].map(|angle| sign * angle * ARC_SECOND),
            scale_difference: if method == Shift::Translations {
                0.0
            } else {
                scale - 1.0
            },
        },
    })
}

/// A unit's conversion factor: the second attribute of `UNIT`, `ANGLEUNIT` or `LENGTHUNIT`.
fn unit_factor(node: Option<&Node>, default: f64) -> Result<f64, Refusal> {
    let Some(node) = node else {
        return Ok(default);
    };
    match node.number(1) {
        Some(factor) if factor.is_finite() && factor > 0.0 => Ok(factor),
        _ => Err(Refusal::Unit(node.name().unwrap_or("").to_owned())),
    }
}

/// A geographic system, from `GEOGCS` or ISO 19162 section 8's `GEOGCRS` and its kin.
fn geographic(node: &Node) -> Result<GeographicSystem, Refusal> {
    let missing = |what: &'static str| Refusal::Missing {
        what,
        of: node.to_string(),
    };
    let datum = node
        .child(&["DATUM", "GEODETICDATUM", "TRF", "ENSEMBLE"])
        .ok_or_else(|| missing("a datum"))?;
    let spheroid = datum
        .child(&["SPHEROID", "ELLIPSOID"])
        .ok_or_else(|| missing("an ellipsoid"))?;
    let (Some(axis), Some(inverse_flattening)) = (spheroid.number(1), spheroid.number(2)) else {
        return Err(missing("an ellipsoid's axis and flattening"));
    };
    // ISO 19162 section 8.2.1 lets the newer form state the axis in a unit of its own; the older
    // form required the semi-major axis in metres (section C.3.1).
    let axis_unit = unit_factor(spheroid.child(&["LENGTHUNIT", "UNIT"]), 1.0)?;
    let ellipsoid = Ellipsoid::new(axis * axis_unit, inverse_flattening)?;

    // The angular unit is the older form's `UNIT` or the newer's `ANGLEUNIT`, at the system's
    // own level; section 7.4.4's implied default is the degree, and section C.4.1 notes that
    // the older form's prime meridian is not always written.
    let degree = 1f64.to_radians();
    let angular_unit = unit_factor(node.child(&["ANGLEUNIT", "UNIT"]), degree)?;
    let prime_meridian = match node.child(&["PRIMEM", "PRIMEMERIDIAN"]) {
        None => 0.0,
        Some(primem) => {
            let value = primem
                .number(1)
                .ok_or_else(|| missing("a prime meridian's longitude"))?;
            let unit = unit_factor(primem.child(&["ANGLEUNIT", "UNIT"]), angular_unit)?;
            (value * unit).to_degrees()
        }
    };
    if node.keyword.starts_with("GEOD") || node.keyword == "BASEGEODCRS" {
        // A geodetic CRS may be geocentric: ISO 19162 section 8.3 gives it a Cartesian
        // coordinate system, whose coordinates are not latitudes at all.
        if let Some(cs) = node.child(&["CS"])
            && !matches!(cs.attributes.first(), Some(Attribute::Word(kind)) if kind == "ELLIPSOIDAL")
        {
            return Err(Refusal::NotASystem(format!(
                "{node} with a non-ellipsoidal CS"
            )));
        }
    }
    // OGC 01-009 section 7.3.18's `TOWGS84`, which ISO 19162 section C.3.3 documents inside the
    // older form's datum: its attributes are the parameters, each a number.
    let datum_shift = datum.child(&["TOWGS84"]).map(|clause| {
        let values: Vec<f64> = clause
            .attributes
            .iter()
            .map_while(|attribute| match attribute {
                Attribute::Number(value) => Some(*value),
                _ => None,
            })
            .collect();
        if values.len() == clause.attributes.len() {
            DatumShift::to_wgs84(&values)
        } else {
            Err(Refusal::DatumShift {
                count: clause.attributes.len(),
            })
        }
    });
    Ok(GeographicSystem {
        name: node.name().unwrap_or("").to_owned(),
        datum: datum.name().unwrap_or("").to_owned(),
        ellipsoid,
        prime_meridian,
        angular_unit,
        datum_shift,
    })
}

/// A parameter name reduced to its letters and digits, lower-cased: the older form's names are
/// written `Central_Meridian`, `central_meridian` and `Central Meridian` across the census, and
/// the spelling carries no meaning.
fn normalised(name: &str) -> String {
    name.chars()
        .filter(char::is_ascii_alphanumeric)
        .map(|c| c.to_ascii_lowercase())
        .collect()
}

/// The role one parameter plays, by its name or by ISO 19162 section 9.3.4's EPSG identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Role {
    /// An origin's latitude — natural or false, as the method defines it.
    OriginLatitude,
    /// An origin's longitude.
    OriginLongitude,
    /// The scale factor.
    Scale,
    /// The false easting.
    FalseEasting,
    /// The false northing.
    FalseNorthing,
    /// The first standard parallel.
    Parallel1,
    /// The second standard parallel.
    Parallel2,
    /// ESRI's auxiliary sphere type, which decides which sphere `Mercator_Auxiliary_Sphere`
    /// projects onto.
    AuxiliarySphere,
}

/// Which [`Role`] a parameter plays, or `None` for a name this reader does not know.
fn role(name: &str, code: Option<i64>) -> Option<Role> {
    // The EPSG Dataset's parameter codes, which Guidance Note 7-2 prints beside each method.
    if let Some(code) = code {
        return match code {
            8801 | 8821 => Some(Role::OriginLatitude),
            8802 | 8822 => Some(Role::OriginLongitude),
            8805 => Some(Role::Scale),
            8806 | 8826 => Some(Role::FalseEasting),
            8807 | 8827 => Some(Role::FalseNorthing),
            8823 => Some(Role::Parallel1),
            8824 => Some(Role::Parallel2),
            _ => None,
        };
    }
    match normalised(name).as_str() {
        "latitudeoforigin"
        | "latitudeofnaturalorigin"
        | "latitudeoffalseorigin"
        | "latitudeofcenter"
        | "latitudeofprojectioncentre" => Some(Role::OriginLatitude),
        "centralmeridian"
        | "longitudeoforigin"
        | "longitudeofnaturalorigin"
        | "longitudeoffalseorigin"
        | "longitudeofcenter" => Some(Role::OriginLongitude),
        "scalefactor" | "scalefactoratnaturalorigin" => Some(Role::Scale),
        "falseeasting" | "eastingatfalseorigin" => Some(Role::FalseEasting),
        "falsenorthing" | "northingatfalseorigin" => Some(Role::FalseNorthing),
        "standardparallel1" | "latitudeof1ststandardparallel" => Some(Role::Parallel1),
        "standardparallel2" | "latitudeof2ndstandardparallel" => Some(Role::Parallel2),
        "auxiliaryspheretype" => Some(Role::AuxiliarySphere),
        _ => None,
    }
}

/// The method a projection names, before its parameters decide between a family's two.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Named {
    /// A method named exactly.
    Exact(Method),
    /// `Lambert_Conformal_Conic`, either Lambert variant.
    LambertFamily,
    /// `Mercator`, either variant.
    MercatorFamily,
    /// ESRI's `Mercator_Auxiliary_Sphere`.
    AuxiliarySphere,
}

/// The method a name or an EPSG method code names.
fn named(name: &str, code: Option<i64>) -> Option<Named> {
    if let Some(code) = code {
        return match code {
            9807 => Some(Named::Exact(Method::TransverseMercator)),
            9802 => Some(Named::Exact(Method::LambertConicConformal2Sp)),
            9801 => Some(Named::Exact(Method::LambertConicConformal1Sp)),
            9804 => Some(Named::Exact(Method::MercatorVariantA)),
            9805 => Some(Named::Exact(Method::MercatorVariantB)),
            1024 => Some(Named::Exact(Method::PseudoMercator)),
            9822 => Some(Named::Exact(Method::AlbersEqualArea)),
            9809 => Some(Named::Exact(Method::ObliqueStereographic)),
            _ => None,
        };
    }
    Some(match normalised(name).as_str() {
        "transversemercator" | "gausskruger" | "gausskrueger" => {
            Named::Exact(Method::TransverseMercator)
        }
        "lambertconicconformal2sp" | "lambertconformalconic2sp" => {
            Named::Exact(Method::LambertConicConformal2Sp)
        }
        "lambertconicconformal1sp" | "lambertconformalconic1sp" => {
            Named::Exact(Method::LambertConicConformal1Sp)
        }
        "lambertconformalconic" | "lambertconicconformal" => Named::LambertFamily,
        "mercator1sp" | "mercatorvarianta" => Named::Exact(Method::MercatorVariantA),
        "mercator2sp" | "mercatorvariantb" => Named::Exact(Method::MercatorVariantB),
        "mercator" => Named::MercatorFamily,
        "popularvisualisationpseudomercator" => Named::Exact(Method::PseudoMercator),
        "mercatorauxiliarysphere" => Named::AuxiliarySphere,
        "albers" | "albersequalarea" | "albersconicequalarea" => {
            Named::Exact(Method::AlbersEqualArea)
        }
        "obliquestereographic" | "doublestereographic" => {
            Named::Exact(Method::ObliqueStereographic)
        }
        _ => return None,
    })
}

/// An `ID["EPSG",n]` or `AUTHORITY["EPSG","n"]` inside a token, as a number.
fn epsg_code(node: &Node) -> Option<i64> {
    let id = node.child(&["ID", "AUTHORITY"])?;
    if !id
        .name()
        .is_some_and(|authority| authority.eq_ignore_ascii_case("EPSG"))
    {
        return None;
    }
    match id.attributes.get(1)? {
        Attribute::Number(code) => {
            #[expect(
                clippy::cast_possible_truncation,
                reason = "an EPSG code is a small whole number; a fractional one names nothing"
            )]
            let whole = *code as i64;
            (code.fract() == 0.0).then_some(whole)
        }
        Attribute::Text(code) => code.trim().parse().ok(),
        _ => None,
    }
}

/// A projected system, from `PROJCS` or ISO 19162 section 9's `PROJCRS`.
fn projected(node: &Node) -> Result<ProjectedSystem, Refusal> {
    let missing = |what: &'static str| Refusal::Missing {
        what,
        of: node.to_string(),
    };
    let base_node = node
        .child(GEOGRAPHIC)
        .ok_or_else(|| missing("a base geographic system"))?;
    let base = geographic(base_node)?;
    let newer = node.keyword != "PROJCS";

    // The method and its parameters: in the older form `PROJECTION` and `PARAMETER` sit in the
    // system itself; in the newer, both sit inside `CONVERSION` (ISO 19162 section 9.3).
    let (holder, method_node) = if newer {
        let conversion = node
            .child(&["CONVERSION"])
            .ok_or_else(|| missing("a conversion"))?;
        let method = conversion
            .child(&["METHOD", "PROJECTION"])
            .ok_or_else(|| missing("a method"))?;
        (conversion, method)
    } else {
        let method = node
            .child(&["PROJECTION"])
            .ok_or_else(|| missing("a projection"))?;
        (node, method)
    };
    let method_name = method_node.name().unwrap_or("").to_owned();
    let family = named(&method_name, epsg_code(method_node))
        .ok_or_else(|| Refusal::UnsupportedMethod(method_name.clone()))?;

    // The linear unit: the older form's `UNIT` at the system's level, the newer form's
    // `LENGTHUNIT` there or on its first axis; section 7.4.4's implied default is the metre.
    let unit_node = node.child(&["LENGTHUNIT", "UNIT"]).or_else(|| {
        node.child(&["AXIS"])
            .and_then(|axis| axis.child(&["LENGTHUNIT"]))
    });
    let linear_unit = unit_factor(unit_node, 1.0)?;
    let linear_unit_name = unit_node.and_then(Node::name).unwrap_or("metre").to_owned();

    let mut parameters = Parameters::default();
    let mut auxiliary_sphere = None;
    for parameter in holder.children("PARAMETER") {
        let name = parameter.name().unwrap_or("");
        let Some(role) = role(name, epsg_code(parameter)) else {
            // A parameter whose meaning this reader does not know is not silently dropped: the
            // methods here are fully defined by the roles above, so an extra one is a method
            // this reader would evaluate wrongly.
            return Err(Refusal::UnknownParameter {
                method: method_name,
                parameter: name.to_owned(),
            });
        };
        let value = parameter
            .number(1)
            .ok_or_else(|| missing("a parameter's value"))?;
        let stated_unit = parameter.child(&["ANGLEUNIT", "LENGTHUNIT", "SCALEUNIT", "UNIT"]);
        let angle = |value: f64| -> Result<f64, Refusal> {
            let unit = if newer {
                unit_factor(stated_unit, 1f64.to_radians())?
            } else {
                base.angular_unit
            };
            Ok(value * unit)
        };
        let length = |value: f64| -> Result<f64, Refusal> {
            let unit = if newer {
                unit_factor(stated_unit, 1.0)?
            } else {
                linear_unit
            };
            Ok(value * unit)
        };
        match role {
            Role::OriginLatitude => parameters.origin_latitude = Some(angle(value)?),
            Role::OriginLongitude => parameters.origin_longitude = Some(angle(value)?),
            Role::Parallel1 => parameters.parallel_1 = Some(angle(value)?),
            Role::Parallel2 => parameters.parallel_2 = Some(angle(value)?),
            Role::FalseEasting => parameters.false_easting = Some(length(value)?),
            Role::FalseNorthing => parameters.false_northing = Some(length(value)?),
            Role::Scale => {
                parameters.scale = Some(value * unit_factor(stated_unit.filter(|_| newer), 1.0)?);
            }
            Role::AuxiliarySphere => auxiliary_sphere = Some(value),
        }
    }
    let method = resolve(family, &method_name, &mut parameters, auxiliary_sphere)?;
    let ellipsoid = if family == Named::AuxiliarySphere {
        // ESRI's type 0 projects onto the sphere of the ellipsoid's semi-major axis, which is
        // the Pseudo-Mercator's formula exactly (Guidance Note 7-2 section 3.2.1.2).
        Ellipsoid::new(base.ellipsoid.semi_major_axis, 0.0)?
    } else {
        base.ellipsoid
    };
    Ok(ProjectedSystem {
        name: node.name().unwrap_or("").to_owned(),
        projection: Projection::new(method, ellipsoid, parameters)?,
        base,
        linear_unit,
        linear_unit_name,
    })
}

/// The one method a family name and its parameters state, or a refusal naming the combination.
///
/// The comparisons are exact on purpose: they ask whether the file stated the same number twice
/// or stated one, which is a question about the file's text and not about a computed value.
#[expect(
    clippy::float_cmp,
    reason = "a comparison of two numbers as the file states them, not of computed values"
)]
fn resolve(
    family: Named,
    name: &str,
    parameters: &mut Parameters,
    auxiliary_sphere: Option<f64>,
) -> Result<Method, Refusal> {
    let conflict = |reason: &'static str| Refusal::Unsupported {
        method: "the older form's projection name",
        reason,
    };
    if auxiliary_sphere.is_some() && family != Named::AuxiliarySphere {
        return Err(Refusal::UnknownParameter {
            method: name.to_owned(),
            parameter: "Auxiliary_Sphere_Type".to_owned(),
        });
    }
    Ok(match family {
        Named::Exact(method) => method,
        Named::LambertFamily => match (parameters.parallel_1, parameters.parallel_2) {
            (Some(first), Some(second)) if first != second => {
                if parameters.scale.is_some_and(|scale| scale != 1.0) {
                    return Err(conflict(
                        "two standard parallels and a scale factor other than one",
                    ));
                }
                Method::LambertConicConformal2Sp
            }
            (Some(parallel), _) => {
                // One parallel: the 1SP method, whose natural origin is on that parallel.
                if parameters
                    .origin_latitude
                    .is_some_and(|origin| origin != parallel)
                {
                    return Err(conflict("one standard parallel and an origin off it"));
                }
                parameters.origin_latitude = Some(parallel);
                Method::LambertConicConformal1Sp
            }
            (None, _) => Method::LambertConicConformal1Sp,
        },
        Named::MercatorFamily => {
            if parameters.parallel_1.is_some() {
                if parameters.scale.is_some_and(|scale| scale != 1.0) {
                    return Err(conflict(
                        "a standard parallel and a scale factor other than one",
                    ));
                }
                Method::MercatorVariantB
            } else {
                Method::MercatorVariantA
            }
        }
        Named::AuxiliarySphere => {
            if auxiliary_sphere.unwrap_or(0.0) != 0.0 {
                return Err(conflict("an auxiliary sphere type other than 0"));
            }
            if parameters
                .parallel_1
                .is_some_and(|parallel| parallel != 0.0)
            {
                return Err(conflict(
                    "an auxiliary sphere with a standard parallel off the equator",
                ));
            }
            Method::PseudoMercator
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// ISO 32000-2 §12.10.4's EXAMPLE 1, as the clause prints it.
    const CLAUSE_EXAMPLE_1: &str = r#"GEOGCS["GCS_North_American_1983", DATUM[ "D_North_American_1983", SPHEROID["GRS_1980",6378137.0,298.257222101] ], PRIMEM["Greenwich",0.0], UNIT["Degree",0.0174532925199433] ]"#;

    #[test]
    fn the_clauses_geographic_example_is_nad83_on_grs_1980() {
        let Ok(ReferenceSystem::Geographic(system)) = from_wkt(CLAUSE_EXAMPLE_1) else {
            panic!("a geographic system");
        };
        assert_eq!(system.name, "GCS_North_American_1983");
        assert_eq!(
            system.ellipsoid.semi_major_axis.to_bits(),
            6_378_137f64.to_bits()
        );
        assert_eq!(
            system.ellipsoid.inverse_flattening.to_bits(),
            298.257_222_101f64.to_bits()
        );
        assert!(system.prime_meridian.abs() < f64::EPSILON);
        assert!((system.angular_unit - 1f64.to_radians()).abs() < 1e-17);
    }

    #[test]
    fn a_datums_towgs84_is_read_and_its_absence_is_none() {
        // OGC 01-009 section 7.4's example carries `TOWGS84[375,-111,431,0,0,0,0]` inside its
        // OSGB 1936 datum; the parameters are read as stated.
        let wkt = r#"GEOGCS["OSGB 1936",DATUM["OSGB_1936",SPHEROID["Airy 1830",6377563.396,299.3249646],TOWGS84[375,-111,431,0,0,0,0]],PRIMEM["Greenwich",0],UNIT["degree",0.0174532925199433]]"#;
        let Ok(ReferenceSystem::Geographic(system)) = from_wkt(wkt) else {
            panic!("a geographic system");
        };
        let Some(Ok(shift)) = system.datum_shift else {
            panic!("a stated transformation");
        };
        assert_eq!(shift.datum, "WGS 84");
        assert_eq!(
            shift.helmert,
            Helmert {
                translation: [375.0, -111.0, 431.0],
                rotation: [0.0; 3],
                scale_difference: 0.0,
            }
        );
        let Ok(ReferenceSystem::Geographic(none)) = from_wkt(CLAUSE_EXAMPLE_1) else {
            panic!("a geographic system");
        };
        assert_eq!(none.datum_shift, None);
        // A clause that is not numbers is kept as its refusal, and the system still reads.
        let Ok(ReferenceSystem::Geographic(odd)) =
            from_wkt(&wkt.replace("TOWGS84[375,", r#"TOWGS84["x","#))
        else {
            panic!("a geographic system");
        };
        assert_eq!(odd.datum_shift, Some(Err(Refusal::DatumShift { count: 7 })));
    }

    /// Three numbers as their bits, so that a test asks for the numbers the string states exactly.
    fn bits(values: [f64; 3]) -> [u64; 3] {
        values.map(f64::to_bits)
    }

    /// ISO 19162 section 20.3's EXAMPLE 1 source system, NAD27 in the newer form, wrapped in a
    /// bound CRS with the transformation given.
    fn bound_nad27(transformation: &str) -> String {
        format!(
            r#"BOUNDCRS[SOURCECRS[GEODCRS["NAD27",DATUM["North American Datum 1927",ELLIPSOID["Clarke 1866",6378206.4,294.978698213]],CS[ellipsoidal,2],AXIS["latitude",north],AXIS["longitude",east],ANGLEUNIT["degree",0.0174532925199433]]],TARGETCRS[GEODCRS["NAD83",DATUM["North American Datum 1983",ELLIPSOID["GRS 1980",6378137,298.2572221]],CS[ellipsoidal,2],AXIS["latitude",north],AXIS["longitude",east],ANGLEUNIT["degree",0.0174532925199433]]],{transformation}]"#
        )
    }

    #[test]
    fn a_bound_crs_is_its_source_with_the_transformation_kept() {
        // Section 20.3's EXAMPLE 1 itself: NADCON reads its shifts from two files, a grid, which
        // no method of the Helmert family does — so the source system reads and the
        // transformation is refused by its method's name, only when a change of datum asks.
        let nadcon = bound_nad27(
            r#"ABRIDGEDTRANSFORMATION["NAD27 to NAD83 Alaska",METHOD["NADCON",ID["EPSG",9613]],PARAMETERFILE["Latitude difference file","alaska.las"],PARAMETERFILE["Longitude difference file","alaska.los"]]"#,
        );
        let Ok(ReferenceSystem::Geographic(system)) = from_wkt(&nadcon) else {
            panic!("the source system");
        };
        assert_eq!(system.datum, "North American Datum 1927");
        assert_eq!(
            system.datum_shift,
            Some(Err(Refusal::UnsupportedTransformation("NADCON".to_owned())))
        );
        // Section 20.2.3's EXAMPLE, geocentric translations, carried to the target's datum.
        let translations = bound_nad27(
            r#"ABRIDGEDTRANSFORMATION["Tokyo to JGD2000 (GSI)",METHOD["Geocentric translations",ID["EPSG",1031]],PARAMETER["X-axis translation",-146.414],PARAMETER["Y-axis translation",507.337],PARAMETER["Z-axis translation",680.507]]"#,
        );
        let Ok(ReferenceSystem::Geographic(system)) = from_wkt(&translations) else {
            panic!("the source system");
        };
        let Some(Ok(shift)) = system.datum_shift else {
            panic!("a stated transformation");
        };
        assert_eq!(shift.datum, "North American Datum 1983");
        assert_eq!(
            bits(shift.helmert.translation),
            bits([-146.414, 507.337, 680.507])
        );
        assert_eq!(bits(shift.helmert.rotation), bits([0.0; 3]));
        // A translations method stating a rotation is a method this reader would evaluate wrongly.
        let rotated = translations.replace(
            r#"PARAMETER["Z-axis translation",680.507]"#,
            r#"PARAMETER["Z-axis translation",680.507],PARAMETER["Z-axis rotation",1]"#,
        );
        let Ok(ReferenceSystem::Geographic(system)) = from_wkt(&rotated) else {
            panic!("the source system");
        };
        assert!(matches!(
            system.datum_shift,
            Some(Err(Refusal::UnknownParameter { .. }))
        ));
    }

    #[test]
    fn a_coordinate_frame_bound_crs_is_the_towgs84_of_the_opposite_rotations() {
        // Section 20.3's EXAMPLE 3, Amersfoort to ETRS89 by the Coordinate Frame method, its
        // rotations in arc-seconds and its scale relative to unity (section 20.2.3). The same
        // transformation stated as a `TOWGS84`, which is the Position Vector convention (OGC 01-009
        // section 10.4), has the rotations negated — Guidance Note 7-2 section 4.2.3's
        // `rPV = −rCF` — and the scale in parts per million; the two are one Helmert.
        let wkt = r#"BOUNDCRS[SOURCECRS[GEOGCRS["Amersfoort",DATUM["Amersfoort",ELLIPSOID["Bessel 1841",6377397.155,299.1528128]],CS[ellipsoidal,2],AXIS["latitude",north],AXIS["longitude",east],ANGLEUNIT["degree",0.0174532925199433]]],TARGETCRS[GEOGCRS["ETRS89",DATUM["European Terrestrial Reference System 1989",ELLIPSOID["GRS 1980",6378137,298.257222101]],CS[ellipsoidal,2],AXIS["latitude",north],AXIS["longitude",east],ANGLEUNIT["degree",0.0174532925199433]]],ABRIDGEDTRANSFORMATION["Amersfoort to ETRS89 (3)",METHOD["Coordinate Frame",ID["EPSG",1032]],PARAMETER["X-axis translation",565.2369,ID["EPSG",8605]],PARAMETER["Y-axis translation",50.0087,ID["EPSG",8606]],PARAMETER["Z-axis translation",465.658,ID["EPSG",8607]],PARAMETER["X-axis rotation",0.407,ID["EPSG",8608]],PARAMETER["Y-axis rotation",-0.351,ID["EPSG",8609]],PARAMETER["Z-axis rotation",1.870,ID["EPSG",8610]],PARAMETER["Scale difference",1.000004812,ID["EPSG",8611]]]]"#;
        let Ok(ReferenceSystem::Geographic(system)) = from_wkt(wkt) else {
            panic!("the source system");
        };
        let Some(Ok(bound)) = system.datum_shift else {
            panic!("a stated transformation");
        };
        let towgs84 =
            Helmert::from_towgs84(&[565.2369, 50.0087, 465.658, -0.407, 0.351, -1.870, 4.812])
                .expect("seven parameters");
        assert_eq!(bits(bound.helmert.translation), bits(towgs84.translation));
        for (a, b) in bound.helmert.rotation.iter().zip(towgs84.rotation) {
            assert!((a - b).abs() < 1e-18, "{a} {b}");
        }
        assert!((bound.helmert.scale_difference - towgs84.scale_difference).abs() < 1e-15);
        // Without a code the method's name decides, and the Note's own name with its domain reads.
        let named = wkt.replace(
            r#"METHOD["Coordinate Frame",ID["EPSG",1032]]"#,
            r#"METHOD["Coordinate Frame rotation (geog2D domain)"]"#,
        );
        let Ok(ReferenceSystem::Geographic(system)) = from_wkt(&named) else {
            panic!("the source system");
        };
        assert_eq!(system.datum_shift, Some(Ok(bound.clone())));
        // `Helmert` names both conventions in ISO 19162's Table F.4, so it names neither.
        let ambiguous = wkt.replace(
            r#"METHOD["Coordinate Frame",ID["EPSG",1032]]"#,
            r#"METHOD["Helmert"]"#,
        );
        let Ok(ReferenceSystem::Geographic(system)) = from_wkt(&ambiguous) else {
            panic!("the source system");
        };
        assert_eq!(
            system.datum_shift,
            Some(Err(Refusal::UnsupportedTransformation(
                "Helmert".to_owned()
            )))
        );
        // A seven-parameter method missing one is refused by the parameter's name.
        let short = wkt.replace(
            r#",PARAMETER["Scale difference",1.000004812,ID["EPSG",8611]]"#,
            "",
        );
        let Ok(ReferenceSystem::Geographic(system)) = from_wkt(&short) else {
            panic!("the source system");
        };
        assert!(matches!(
            system.datum_shift,
            Some(Err(Refusal::MissingParameter {
                parameter: "scale difference",
                ..
            }))
        ));
    }

    #[test]
    fn a_projected_bound_crs_keeps_the_transformation_on_its_base_system() {
        // Section 20.1: a transformation in the geodetic domain bound to a projected source is
        // deemed to operate on the projected system's base system.
        let wkt = r#"BOUNDCRS[SOURCECRS[PROJCRS["NAD27 / Texas South Central",BASEGEOGCRS["NAD27",DATUM["North American Datum 1927",ELLIPSOID["Clarke 1866",20925832.164,294.97869821,LENGTHUNIT["US survey foot",0.304800609601219]]],PRIMEM["Greenwich",0]],CONVERSION["Texas South Central SPCS27",METHOD["Lambert Conic Conformal (2SP)",ID["EPSG",9802]],PARAMETER["Latitude of false origin",27.833333333333333,ANGLEUNIT["degree",0.0174532925199433],ID["EPSG",8821]],PARAMETER["Longitude of false origin",-99.0,ANGLEUNIT["degree",0.0174532925199433],ID["EPSG",8822]],PARAMETER["Latitude of 1st standard parallel",28.383333333333333,ANGLEUNIT["degree",0.0174532925199433],ID["EPSG",8823]],PARAMETER["Latitude of 2nd standard parallel",30.283333333333333,ANGLEUNIT["degree",0.0174532925199433],ID["EPSG",8824]],PARAMETER["Easting at false origin",2000000.0,LENGTHUNIT["US survey foot",0.304800609601219],ID["EPSG",8826]],PARAMETER["Northing at false origin",0.0,LENGTHUNIT["US survey foot",0.304800609601219],ID["EPSG",8827]]],CS[Cartesian,2],AXIS["(X)",east],AXIS["(Y)",north],LENGTHUNIT["US survey foot",0.304800609601219]]],TARGETCRS[GEOGCRS["WGS 84",DATUM["World Geodetic System 1984",ELLIPSOID["WGS 84",6378137,298.257223563]],CS[ellipsoidal,2],AXIS["latitude",north],AXIS["longitude",east],ANGLEUNIT["degree",0.0174532925199433]]],ABRIDGEDTRANSFORMATION["NAD27 to WGS 84",METHOD["Geocentric translations",ID["EPSG",1031]],PARAMETER["X-axis translation",-8],PARAMETER["Y-axis translation",160],PARAMETER["Z-axis translation",176]]]"#;
        let Ok(ReferenceSystem::Projected(system)) = from_wkt(wkt) else {
            panic!("the projected source system");
        };
        assert_eq!(system.projection.method, Method::LambertConicConformal2Sp);
        let Some(Ok(shift)) = &system.base.datum_shift else {
            panic!("a stated transformation on the base system");
        };
        assert_eq!(shift.datum, "World Geodetic System 1984");
        assert_eq!(bits(shift.helmert.translation), bits([-8.0, 160.0, 176.0]));
        // A bound CRS with no source is refused as a whole.
        assert!(matches!(
            from_wkt(
                r#"BOUNDCRS[TARGETCRS[GEOGCRS["WGS 84",DATUM["WGS84",ELLIPSOID["WGS 84",6378137,298.257223563]]]]]"#
            ),
            Err(Refusal::Missing {
                what: "a source system",
                ..
            })
        ));
    }

    #[test]
    fn the_clauses_projected_example_is_albers_with_its_six_parameters() {
        let Ok(ReferenceSystem::Projected(system)) = from_wkt(wkt::tests::CLAUSE_EXAMPLE_2) else {
            panic!("a projected system");
        };
        assert_eq!(system.projection.method, Method::AlbersEqualArea);
        let p = system.projection.parameters;
        assert!((p.origin_latitude.expect("φF") - 40f64.to_radians()).abs() < 1e-15);
        assert!((p.origin_longitude.expect("λF") - (-96f64).to_radians()).abs() < 1e-15);
        assert!((p.parallel_1.expect("φ1") - 20f64.to_radians()).abs() < 1e-15);
        assert!((p.parallel_2.expect("φ2") - 60f64.to_radians()).abs() < 1e-15);
        assert!((system.linear_unit - 1.0).abs() < f64::EPSILON);
        // The origin itself is where the system's own parameters put it, within ADR 1587's
        // budget of half a thousandth of a second: Guidance Note 7-2 section 3.1.3's latitude
        // series stops at e⁶.
        let budget = 0.0005 / 3600.0;
        let origin = system.projection.inverse(0.0, 0.0).expect("the origin");
        assert!((origin.latitude.to_degrees() - 40.0).abs() < budget);
        assert!((origin.longitude.to_degrees() + 96.0).abs() < budget);
    }

    /// A census document's system, the commonest in the crawl (41 documents): ETRS89 / UTM
    /// zone 32N in the older form.
    const UTM_32N: &str = r#"PROJCS["ETRS89_UTM_zone_32N",GEOGCS["GCS_ETRS_1989",DATUM["D_ETRS_1989",SPHEROID["GRS_1980",6378137,298.257222101]],PRIMEM["Greenwich",0],UNIT["Degree",0.017453292519943295]],PROJECTION["Transverse_Mercator"],PARAMETER["latitude_of_origin",0],PARAMETER["central_meridian",9],PARAMETER["scale_factor",0.9996],PARAMETER["false_easting",500000],PARAMETER["false_northing",0],UNIT["Meter",1]]"#;

    #[test]
    fn the_commonest_census_system_puts_its_central_meridian_at_nine_degrees_east() {
        let Ok(ReferenceSystem::Projected(system)) = from_wkt(UTM_32N) else {
            panic!("a projected system");
        };
        assert_eq!(system.projection.method, Method::TransverseMercator);
        // On the central meridian at the false easting, northing zero, is the equator.
        let point = system.projection.inverse(500_000.0, 0.0).expect("a point");
        assert!(point.latitude.abs() < 1e-12);
        assert!((point.longitude.to_degrees() - 9.0).abs() < 1e-12);
    }

    /// A census State Plane system in US survey feet, the older form's units as ADR 1587 reads
    /// them: the false easting in feet beside `UNIT["Foot_US",…]`.
    const CALIFORNIA_V_FEET: &str = r#"PROJCS["NAD_1983_StatePlane_California_V_FIPS_0405_Feet",GEOGCS["GCS_North_American_1983",DATUM["D_North_American_1983",SPHEROID["GRS_1980",6378137.0,298.257222101]],PRIMEM["Greenwich",0.0],UNIT["Degree",0.0174532925199433]],PROJECTION["Lambert_Conformal_Conic"],PARAMETER["False_Easting",6561666.666666666],PARAMETER["False_Northing",1640416.666666667],PARAMETER["Central_Meridian",-118.0],PARAMETER["Standard_Parallel_1",34.03333333333333],PARAMETER["Standard_Parallel_2",35.46666666666667],PARAMETER["Latitude_Of_Origin",33.5],UNIT["Foot_US",0.3048006096012192]]"#;

    #[test]
    fn a_state_plane_system_in_feet_puts_its_false_origin_where_it_says() {
        let Ok(ReferenceSystem::Projected(system)) = from_wkt(CALIFORNIA_V_FEET) else {
            panic!("a projected system");
        };
        assert_eq!(system.projection.method, Method::LambertConicConformal2Sp);
        // The false origin's grid coordinates, in the system's feet, converted as a host
        // converts a point: by the linear unit.
        let feet = system.linear_unit;
        let origin = system
            .projection
            .inverse(6_561_666.666_666_666 * feet, 1_640_416.666_666_667 * feet)
            .expect("the false origin");
        assert!((origin.latitude.to_degrees() - 33.5).abs() < 1e-9);
        assert!((origin.longitude.to_degrees() + 118.0).abs() < 1e-9);
    }

    #[test]
    fn a_one_parallel_lambert_is_the_one_parallel_method() {
        // The census's Wisconsin county systems: one standard parallel, a scale factor, and the
        // origin on that parallel.
        let wkt = r#"PROJCS["NAD_1983_HARN_WISCRS_Oneida_County_Feet",GEOGCS["GCS_North_American_1983_HARN",DATUM["D_North_American_1983_HARN",SPHEROID["GRS_1980",6378137.0,298.257222101]],PRIMEM["Greenwich",0.0],UNIT["Degree",0.0174532925199433]],PROJECTION["Lambert_Conformal_Conic"],PARAMETER["False_Easting",230000.0],PARAMETER["False_Northing",188936.744],PARAMETER["Central_Meridian",-89.54444444444444],PARAMETER["Standard_Parallel_1",45.70422377027778],PARAMETER["Scale_Factor",1.0000686968],PARAMETER["Latitude_Of_Origin",45.70422377027778],UNIT["Foot_US",0.3048006096012192]]"#;
        let Ok(ReferenceSystem::Projected(system)) = from_wkt(wkt) else {
            panic!("a projected system");
        };
        assert_eq!(system.projection.method, Method::LambertConicConformal1Sp);
    }

    #[test]
    fn esri_web_mercator_is_the_pseudo_mercator_by_either_name() {
        let auxiliary = r#"PROJCS["WGS_1984_Web_Mercator_Auxiliary_Sphere",GEOGCS["GCS_WGS_1984",DATUM["D_WGS_1984",SPHEROID["WGS_1984",6378137.0,298.257223563]],PRIMEM["Greenwich",0.0],UNIT["Degree",0.0174532925199433]],PROJECTION["Mercator_Auxiliary_Sphere"],PARAMETER["False_Easting",0.0],PARAMETER["False_Northing",0.0],PARAMETER["Central_Meridian",0.0],PARAMETER["Standard_Parallel_1",0.0],PARAMETER["Auxiliary_Sphere_Type",0.0],UNIT["Meter",1.0]]"#;
        let sphere = r#"PROJCS["WGS_1984_Web_Mercator",GEOGCS["GCS_WGS_1984_Major_Auxiliary_Sphere",DATUM["D_WGS_1984_Major_Auxiliary_Sphere",SPHEROID["WGS_1984_Major_Auxiliary_Sphere",6378137.0,0.0]],PRIMEM["Greenwich",0.0],UNIT["Degree",0.0174532925199433]],PROJECTION["Mercator"],PARAMETER["False_Easting",0.0],PARAMETER["False_Northing",0.0],PARAMETER["Central_Meridian",0.0],PARAMETER["Standard_Parallel_1",0.0],UNIT["Meter",1.0]]"#;
        // Guidance Note 7-2 section 3.2.1.2's reverse example, through both spellings, to ADR
        // 1587's budget: the example's grid is printed to the centimetre.
        let budget = (0.0005f64 / 3600.0).to_radians();
        for wkt in [auxiliary, sphere] {
            let Ok(ReferenceSystem::Projected(system)) = from_wkt(wkt) else {
                panic!("a projected system");
            };
            let point = system
                .projection
                .inverse(-11_169_055.58, 2_810_000.00)
                .expect("a point");
            assert!((point.latitude - 0.426_970_023).abs() < budget);
            assert!((point.longitude + 1.751_147_016).abs() < budget);
        }
    }

    #[test]
    fn the_newer_form_reads_units_per_parameter() {
        // ISO 19162 section 9.5's EXAMPLE 2 shape: NAD27 / Texas South Central in the newer
        // form, the parameters of Guidance Note 7-2 section 3.1.1.1's example.
        let wkt = r#"PROJCRS["NAD27 / Texas South Central",
            BASEGEOGCRS["NAD27",DATUM["North American Datum 1927",
              ELLIPSOID["Clarke 1866",20925832.164,294.97869821,LENGTHUNIT["US survey foot",0.304800609601219]]],
              PRIMEM["Greenwich",0]],
            CONVERSION["Texas South Central SPCS27",
              METHOD["Lambert Conic Conformal (2SP)",ID["EPSG",9802]],
              PARAMETER["Latitude of false origin",27.833333333333333,ANGLEUNIT["degree",0.0174532925199433],ID["EPSG",8821]],
              PARAMETER["Longitude of false origin",-99.0,ANGLEUNIT["degree",0.0174532925199433],ID["EPSG",8822]],
              PARAMETER["Latitude of 1st standard parallel",28.383333333333333,ANGLEUNIT["degree",0.0174532925199433],ID["EPSG",8823]],
              PARAMETER["Latitude of 2nd standard parallel",30.283333333333333,ANGLEUNIT["degree",0.0174532925199433],ID["EPSG",8824]],
              PARAMETER["Easting at false origin",2000000.0,LENGTHUNIT["US survey foot",0.304800609601219],ID["EPSG",8826]],
              PARAMETER["Northing at false origin",0.0,LENGTHUNIT["US survey foot",0.304800609601219],ID["EPSG",8827]]],
            CS[Cartesian,2],
              AXIS["(X)",east],AXIS["(Y)",north],
              LENGTHUNIT["US survey foot",0.304800609601219]]"#;
        let Ok(ReferenceSystem::Projected(system)) = from_wkt(wkt) else {
            panic!("a projected system");
        };
        assert_eq!(system.projection.method, Method::LambertConicConformal2Sp);
        let feet = system.linear_unit;
        let point = system
            .projection
            .inverse(2_963_503.91 * feet, 254_759.80 * feet)
            .expect("a point");
        // 28°30′00.000″N 96°00′00.000″W, to half the example's last printed digit.
        let half = (0.0005f64 / 3600.0).to_radians();
        assert!((point.latitude - 28.5f64.to_radians()).abs() < half);
        assert!((point.longitude - (-96f64).to_radians()).abs() < half);
    }

    #[test]
    fn what_the_census_found_and_this_reader_does_not_carry_is_refused_by_name() {
        // ESRI's GUID for an unknown system, four census documents: not Well Known Text.
        assert!(matches!(
            from_wkt("{B286C06B-0879-11D2-AACA-00C04FA33C20}"),
            Err(Refusal::Wkt(_))
        ));
        // A projected system with no projection at all, one census document.
        let no_projection = r#"PROJCS["WGS_1984_UTM_Zone_55S",GEOGCS["GCS_WGS_1984",DATUM["D_World Geodetic System 1984",SPHEROID["WGS_1984",6378137,298.257223563]],PRIMEM["Greenwich",0],UNIT["Degree",0.017453292519943295]],PARAMETER["latitude_of_origin",0],PARAMETER["central_meridian",147],PARAMETER["scale_factor",0.9996],PARAMETER["false_easting",500000],PARAMETER["false_northing",10000000],UNIT["Meter",1]]"#;
        assert!(matches!(
            from_wkt(no_projection),
            Err(Refusal::Missing {
                what: "a projection",
                ..
            })
        ));
        // Hotine, Krovak and the vertical perspective, by name.
        let hotine = r#"PROJCS["NAD_1983_Michigan_GeoRef_Meters",GEOGCS["GCS_North_American_1983",DATUM["D_North_American_1983",SPHEROID["GRS_1980",6378137.0,298.257222101]],PRIMEM["Greenwich",0.0],UNIT["Degree",0.0174532925199433]],PROJECTION["Hotine_Oblique_Mercator_Azimuth_Natural_Origin"],PARAMETER["False_Easting",2546731.496],PARAMETER["False_Northing",-4354009.816],PARAMETER["Scale_Factor",0.9996],PARAMETER["Azimuth",337.25556],PARAMETER["Longitude_Of_Center",-86.0],PARAMETER["Latitude_Of_Center",45.30916666666666],UNIT["Meter",1.0]]"#;
        assert_eq!(
            from_wkt(hotine),
            Err(Refusal::UnsupportedMethod(
                "Hotine_Oblique_Mercator_Azimuth_Natural_Origin".to_owned()
            ))
        );
        // A vertical system is not a geographic or projected one.
        assert!(matches!(
            from_wkt(
                r#"VERT_CS["NAVD88",VERT_DATUM["North American Vertical Datum 1988",2005],UNIT["metre",1]]"#
            ),
            Err(Refusal::NotASystem(_))
        ));
    }

    #[test]
    fn a_parameter_this_reader_does_not_know_is_refused_rather_than_dropped() {
        let wkt = UTM_32N.replace(
            r#"UNIT["Meter",1]]"#,
            r#"PARAMETER["rectified_grid_angle",3],UNIT["Meter",1]]"#,
        );
        assert!(matches!(
            from_wkt(&wkt),
            Err(Refusal::UnknownParameter { .. })
        ));
    }
}

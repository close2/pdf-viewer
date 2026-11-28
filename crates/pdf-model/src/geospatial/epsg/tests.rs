use super::*;
use crate::geospatial::datum;
use crate::geospatial::projection::LatLon;
use crate::geospatial::{GeographicPosition, system::from_wkt};

/// Ten micro-degrees of a micro-degree: a position agreeing this closely is the same position to
/// well under a millimetre, far finer than any coordinate §12.10's registration states.
pub(super) const SAME_POSITION: f64 = 1e-9;

pub(super) fn projected_of(code: i64) -> ProjectedSystem {
    match system(code) {
        Ok(ReferenceSystem::Projected(projected)) => projected,
        other => panic!("EPSG {code} as a projected system, got {other:?}"),
    }
}

pub(super) fn projected_wkt(text: &str) -> ProjectedSystem {
    match from_wkt(text) {
        Ok(ReferenceSystem::Projected(projected)) => projected,
        other => panic!("a projected system, got {other:?}"),
    }
}

/// Both systems' latitude and longitude of each grid point, to [`SAME_POSITION`].
pub(super) fn same_positions(
    by_code: &ProjectedSystem,
    by_wkt: &ProjectedSystem,
    points: &[[f64; 2]],
) {
    for &[easting, northing] in points {
        let a = by_code
            .geographic(easting, northing)
            .expect("the code's position");
        let b = by_wkt
            .geographic(easting, northing)
            .expect("the string's position");
        assert!(
            (a.latitude - b.latitude).abs() < SAME_POSITION
                && (a.longitude - b.longitude).abs() < SAME_POSITION,
            "({easting}, {northing}): {a:?} by the code, {b:?} by the string"
        );
    }
}

#[test]
fn sexagesimal_dms_reads_minutes_and_seconds_as_digits() {
    // EPSG unit 9110's definition: DDD.MMSSsss, so 4.2133177 is 4°21′33.177″ — the longitude of
    // false origin of the Belgian Lambert 2008 conversion, as the Dataset states it.
    let expected = 4.0 + 21.0 / 60.0 + 33.177 / 3600.0;
    assert!((sexagesimal_dms(4.213_317_7) - expected).abs() < 1e-12);
    // Thirty minutes, which a binary fraction holds just below .30.
    assert!((sexagesimal_dms(27.30) - 27.5).abs() < 1e-12);
    assert!((sexagesimal_dms(-89.45) + 89.75).abs() < 1e-12);
}

#[test]
fn every_reference_in_the_table_resolves() {
    // Every system the table carries is built or refused by its method, never by a record the
    // generator failed to carry, a unit it cannot read, or a parameter the eight methods do not
    // take.
    let mut built = 0;
    for code in dataset::GEOGRAPHIC
        .iter()
        .map(|g| g.0)
        .chain(dataset::PROJECTED.iter().map(|p| p.0))
    {
        match system(i64::from(code)) {
            Ok(_) => built += 1,
            Err(Refusal::EpsgMethod { method_code, .. }) => {
                assert!(method(u32::try_from(method_code).expect("a code")).is_none());
            }
            // The one record the generator's source cannot carry: PROJ's conversion table holds
            // seven parameters, and Krovak Modified's polynomial is twelve more (ADR 3875).
            Err(Refusal::MissingParameter {
                method: "Krovak Modified" | "Krovak Modified (North Orientated)",
                parameter: "ordinate 1 of evaluation point",
            }) => {}
            Err(refusal) => panic!("EPSG {code}: {refusal}"),
        }
    }
    assert!(built > dataset::GEOGRAPHIC.len(), "{built}");
    for table in [
        dataset::UNITS.iter().map(|u| u.0).collect::<Vec<_>>(),
        dataset::GEOGRAPHIC.iter().map(|g| g.0).collect(),
        dataset::PROJECTED.iter().map(|p| p.0).collect(),
        dataset::CONVERSIONS.iter().map(|c| c.0).collect(),
        dataset::DATUMS.iter().map(|d| d.0).collect(),
    ] {
        assert!(
            table.windows(2).all(|pair| pair[0] < pair[1]),
            "sorted by code"
        );
    }
}

#[test]
fn epsg_4326_is_the_system_its_wkt_states() {
    let Ok(ReferenceSystem::Geographic(by_code)) = system(4326) else {
        panic!("EPSG 4326 as a geographic system");
    };
    let Ok(ReferenceSystem::Geographic(by_wkt)) = from_wkt(
        r#"GEOGCS["WGS 84",DATUM["WGS_1984",SPHEROID["WGS 84",6378137,298.257223563]],PRIMEM["Greenwich",0],UNIT["degree",0.0174532925199433]]"#,
    ) else {
        panic!("the string as a geographic system");
    };
    assert_eq!(by_code.ellipsoid, by_wkt.ellipsoid);
    assert!(datum::same_datum(&by_code, &by_wkt), "{by_code:?}");
    assert!((by_code.angular_unit - by_wkt.angular_unit).abs() < 1e-15);
    // One datum: a position on one is the same position on the other, as §12.10.2's `/DCS`
    // carries it.
    let position = GeographicPosition {
        latitude: 52.5,
        longitude: 13.4,
    };
    let carried = datum::transform(&by_code, &by_wkt, position).expect("one datum");
    assert!((carried.latitude - 52.5).abs() < SAME_POSITION);
    assert!((carried.longitude - 13.4).abs() < SAME_POSITION);
}

#[test]
fn epsg_27700_is_the_british_national_grid_its_wkt_states() {
    let by_code = projected_of(27700);
    let by_wkt = projected_wkt(
        r#"PROJCS["OSGB36 / British National Grid",GEOGCS["OSGB36",DATUM["Ordnance_Survey_of_Great_Britain_1936",SPHEROID["Airy 1830",6377563.396,299.3249646]],PRIMEM["Greenwich",0],UNIT["degree",0.0174532925199433]],PROJECTION["Transverse_Mercator"],PARAMETER["latitude_of_origin",49],PARAMETER["central_meridian",-2],PARAMETER["scale_factor",0.9996012717],PARAMETER["false_easting",400000],PARAMETER["false_northing",-100000],UNIT["metre",1]]"#,
    );
    same_positions(
        &by_code,
        &by_wkt,
        &[
            [577_274.99, 69_740.50],
            [400_000.0, 100_000.0],
            [100_000.0, 900_000.0],
            [650_000.0, 1_200_000.0],
        ],
    );
    // Guidance Note 7-2 section 3.2.3.1's worked example is this system: (577274.99 E,
    // 69740.50 N) is 50°30′N 0°30′E, to the example's printed thousandth of a second.
    let example = by_code
        .geographic(577_274.99, 69_740.50)
        .expect("the example's point");
    assert!(
        (example.latitude - 50.5).abs() < 0.001 / 3600.0,
        "{example:?}"
    );
    assert!(
        (example.longitude - 0.5).abs() < 0.001 / 3600.0,
        "{example:?}"
    );
    assert_eq!(by_code.linear_unit_name, "metre");
}

#[test]
fn epsg_2278_reads_sexagesimal_parameters_and_us_survey_feet() {
    // NAD83 / Texas South Central (ftUS): Lambert Conic Conformal (2SP), its angles stated in
    // the Dataset as sexagesimal DMS and its false origin and axes in US survey feet.
    let by_code = projected_of(2278);
    let by_wkt = projected_wkt(
        r#"PROJCS["NAD83 / Texas South Central (ftUS)",GEOGCS["NAD83",DATUM["North_American_Datum_1983",SPHEROID["GRS 1980",6378137,298.257222101]],PRIMEM["Greenwich",0],UNIT["degree",0.0174532925199433]],PROJECTION["Lambert_Conformal_Conic_2SP"],PARAMETER["standard_parallel_1",30.28333333333333],PARAMETER["standard_parallel_2",28.38333333333333],PARAMETER["latitude_of_origin",27.83333333333333],PARAMETER["central_meridian",-99],PARAMETER["false_easting",1968500],PARAMETER["false_northing",13123333.333],UNIT["US survey foot",0.3048006096012192]]"#,
    );
    same_positions(
        &by_code,
        &by_wkt,
        &[
            [1_968_500.0, 13_123_333.333],
            [2_500_000.0, 13_500_000.0],
            [1_000_000.0, 12_800_000.0],
        ],
    );
    assert_eq!(by_code.linear_unit_name, "US survey foot");
}

#[test]
fn a_code_whose_method_is_not_built_is_refused_by_its_number_and_the_methods_name() {
    // Vanua Levu 1915 / Vanua Levu Grid is Hyperbolic Cassini-Soldner, EPSG method 9833,
    // `doc/todo/79`'s nineteenth entry.
    assert_eq!(
        system(3139),
        Err(Refusal::EpsgMethod {
            code: 3139,
            method: "Hyperbolic Cassini-Soldner",
            method_code: 9833,
        })
    );
}

#[test]
fn a_code_the_table_does_not_carry_is_refused_by_its_number() {
    // 4978 is WGS 84's geocentric system, whose coordinates are not latitudes; 7405 is a compound
    // system; −1 and 2³² name nothing.
    for code in [4978, 7405, -1, 1 << 32] {
        assert_eq!(system(code), Err(Refusal::EpsgNotCarried(code)));
    }
}

/// A latitude or longitude in degrees, minutes and seconds, signed by its hemisphere.
pub(super) fn dms(degrees: f64, minutes: f64, seconds: f64) -> f64 {
    degrees.signum() * (degrees.abs() + minutes / 60.0 + seconds / 3600.0)
}

/// A worked example of Guidance Note 7-2 through the EPSG code of the system it is stated for:
/// the forward answer within `grid` of the printed easting and northing (in the system's unit),
/// and the reverse answer within a thousandth of a second, the example's printed precision.
pub(super) fn worked_example(
    code: i64,
    [latitude, longitude]: [f64; 2],
    [easting, northing]: [f64; 2],
    grid: f64,
) {
    let system = projected_of(code);
    let [e, n] = system
        .grid(GeographicPosition {
            latitude,
            longitude,
        })
        .expect("the example's point projects");
    assert!(
        (e - easting).abs() < grid && (n - northing).abs() < grid,
        "EPSG {code}: ({e}, {n}) against ({easting}, {northing})"
    );
    let back = system
        .geographic(easting, northing)
        .expect("the example's grid point");
    let second = 0.001 / 3600.0;
    assert!(
        (back.latitude - latitude).abs() < second && (back.longitude - longitude).abs() < second,
        "EPSG {code}: {back:?} against ({latitude}, {longitude})"
    );
}

#[test]
fn cassini_soldner_reproduces_the_trinidad_grid_example() {
    // Guidance Note 7-2 section 3.2.2's example, Trinidad 1903 / Trinidad Grid (EPSG 30200): its
    // ellipsoid in Clarke's feet, its grid in Clarke's links, the Dataset's own units. 10°N 62°W
    // is (66644.94, 82536.22) links; one link is 0.2 m, so a hundredth of a link is the printed
    // digit.
    worked_example(30200, [10.0, -62.0], [66_644.94, 82_536.22], 0.01);
}

#[test]
fn lambert_azimuthal_equal_area_reproduces_the_etrs_laea_example() {
    // Section 3.3.2's example, ETRS89 / ETRS-LAEA, which is EPSG 3035's conversion: 50°N 5°E is
    // (3962799.45, 2999718.85) m.
    worked_example(3035, [50.0, 5.0], [3_962_799.45, 2_999_718.85], 0.01);
}

#[test]
fn polar_stereographic_a_reproduces_the_ups_north_example() {
    // Section 3.3.1.2's variant A example, WGS 84 / UPS North (EPSG 32661): 73°N 44°E is
    // (3320416.75, 632668.43) m.
    worked_example(32661, [73.0, 44.0], [3_320_416.75, 632_668.43], 0.01);
}

#[test]
fn polar_stereographic_b_reproduces_the_australian_antarctic_example() {
    // Section 3.3.1.2's variant B example, WGS 84 / Australian Antarctic Polar Stereographic
    // (EPSG 3032): 75°S 120°E is (7255380.79, 7053389.56) m.
    worked_example(3032, [-75.0, 120.0], [7_255_380.79, 7_053_389.56], 0.01);
}

#[test]
fn polar_stereographic_c_reproduces_the_terre_adelie_example() {
    // Section 3.3.1.2's variant C example, stated for Petrels 1972 / Terre Adelie Polar
    // Stereographic; EPSG 2986, Perroud 1950's, has the same conversion on the same International
    // 1924 ellipsoid. 66°36′18.820″S 140°04′17.040″E is (303169.52, 244055.72) m.
    worked_example(
        2986,
        [-dms(66.0, 36.0, 18.820), dms(140.0, 4.0, 17.040)],
        [303_169.52, 244_055.72],
        0.01,
    );
}

#[test]
fn transverse_mercator_south_orientated_reproduces_the_lo29_example() {
    // Guidance Note 7-2 section 3.2.3.3's example, Hartebeesthoek94 / Lo29 (EPSG 2053), whose
    // axes are the westing and the southing: 25°43′55.302″S 28°16′57.479″E is a westing of
    // 71984.49 m and a southing of 2847342.74 m. The point is printed to a thousandth of a
    // second, about 3 cm on the ground, so the grid is held to ADR 1672's 1.5 cm.
    worked_example(
        2053,
        [-dms(25.0, 43.0, 55.302), dms(28.0, 16.0, 57.479)],
        [71_984.49, 2_847_342.74],
        0.015,
    );
}

#[test]
fn hotine_oblique_mercator_b_reproduces_the_borneo_example() {
    // Section 3.2.4.1's example, Timbalai 1948 / R.S.O. Borneo (m) (EPSG 29873), variant B:
    // 5°23′14.1129″N 115°48′19.8196″E is (679245.73, 596562.78) m.
    worked_example(
        29873,
        [dms(5.0, 23.0, 14.1129), dms(115.0, 48.0, 19.8196)],
        [679_245.73, 596_562.78],
        0.01,
    );
}

#[test]
fn colombia_urban_reproduces_the_bogota_example() {
    // Section 3.4.3's example, MAGNA-SIRGAS / Bogota urban grid (EPSG 6247): 4°48′N 74°15′W is
    // (80859.033, 122543.174) m, printed to the millimetre.
    worked_example(
        6247,
        [dms(4.0, 48.0, 0.0), -dms(74.0, 15.0, 0.0)],
        [80_859.033, 122_543.174],
        0.001,
    );
}

#[test]
fn lambert_west_orientated_is_the_1sp_projection_with_its_easting_mirrored() {
    // Section 3.1.1.3 prints no example: its formulas are the 1SP variant's "except for: W = FE
    // – r * sin θ", so the westing of a point is 2·FE less the 1SP easting and the northing is
    // the 1SP northing. Qornoq 1927 / Greenland zone 1 east (EPSG 2297) is checked against a 1SP
    // projection of the same parameters, and its reverse against the point.
    let west = projected_of(2297);
    let east = Projection::new(
        ProjectionMethod::LambertConicConformal1Sp,
        west.projection.ellipsoid,
        west.projection.parameters,
    )
    .expect("the 1SP parameters");
    let false_easting = west.projection.parameters.false_easting.unwrap_or(0.0);
    for [latitude, longitude] in [[82.0, -45.0], [81.0, -30.0], [83.5, -40.0]] {
        let point = LatLon {
            latitude: f64::to_radians(latitude),
            longitude: f64::to_radians(longitude),
        };
        let w = west.projection.forward(point).expect("the westing");
        let e = east.forward(point).expect("the 1SP easting");
        assert!(
            (w.easting - (2.0 * false_easting - e.easting)).abs() < 1e-6
                && (w.northing - e.northing).abs() < 1e-6,
            "{w:?} against {e:?}"
        );
        let back = west
            .projection
            .inverse(w.easting, w.northing)
            .expect("the reverse");
        assert!(
            (back.latitude - point.latitude).abs() < SAME_POSITION
                && (back.longitude - point.longitude).abs() < SAME_POSITION,
            "{back:?} against {point:?}"
        );
    }
}

#[test]
fn krovak_reproduces_the_s_jtsk_example() {
    // Section 3.1.2.1's example, S-JTSK (Ferro) / Krovak (EPSG 2065), whose base system counts
    // longitude from Ferro: 50°12′32.442″N 34°30′59.179″E of Ferro is the southing 1050538.64 m
    // and the westing 568991.00 m, carried westing first.
    worked_example(
        2065,
        [dms(50.0, 12.0, 32.442), dms(34.0, 30.0, 59.179)],
        [568_991.00, 1_050_538.64],
        0.01,
    );
}

#[test]
fn krovak_north_orientated_negates_both_coordinates() {
    // Section 3.1.2.2's example, S-JTSK (Ferro) / Krovak East North (EPSG 5221): the same point
    // is (−568991.00, −1050538.64) m.
    worked_example(
        5221,
        [dms(50.0, 12.0, 32.442), dms(34.0, 30.0, 59.179)],
        [-568_991.00, -1_050_538.64],
        0.01,
    );
}

/// Section 3.1.2.3's example parameters, S-JTSK/05 (Ferro) / Modified Krovak, as the section
/// prints them.
fn modified_krovak(method: ProjectionMethod) -> Projection {
    let parameters = Parameters {
        origin_latitude: Some(dms(49.0, 30.0, 0.0).to_radians()),
        origin_longitude: Some(dms(42.0, 30.0, 0.0).to_radians()),
        azimuth: Some(dms(30.0, 17.0, 17.303_11).to_radians()),
        parallel_1: Some(dms(78.0, 30.0, 0.0).to_radians()),
        scale: Some(0.9999),
        false_easting: Some(5_000_000.0),
        false_northing: Some(5_000_000.0),
        evaluation_point: [Some(1_089_000.0), Some(654_000.0)],
        coefficients: [
            2.946_529_277e-2,
            2.515_965_696e-2,
            1.193_845_912e-7,
            -4.668_270_147e-7,
            9.233_980_362e-12,
            1.523_735_715e-12,
            1.696_780_024e-18,
            4.408_314_235e-18,
            -8.331_083_518e-24,
            -3.689_471_323e-24,
        ]
        .map(Some),
        ..Parameters::default()
    };
    let bessel = Ellipsoid::new(6_377_397.155, 299.152_812_8).expect("Bessel 1841");
    Projection::new(method, bessel, parameters).expect("the example's parameters")
}

#[test]
fn krovak_modified_reproduces_the_s_jtsk_05_examples() {
    // Sections 3.1.2.3 and 3.1.2.4's examples, for the same point: the southing 6050538.71 m and
    // westing 5568990.91 m, and negated for the north-orientated grid. The carried table states
    // the polynomial of neither system (its conversions hold seven parameters), so the examples are
    // built from the section's parameters, and the codes are refused by the parameter they lack.
    let point = LatLon {
        latitude: dms(50.0, 12.0, 32.442).to_radians(),
        longitude: dms(34.0, 30.0, 59.179).to_radians(),
    };
    for (method, sign) in [
        (ProjectionMethod::KrovakModified, 1.0),
        (ProjectionMethod::KrovakModifiedNorthOrientated, -1.0),
    ] {
        let projection = modified_krovak(method);
        let grid = projection.forward(point).expect("the example's point");
        assert!(
            (grid.easting - sign * 5_568_990.91).abs() < 0.01
                && (grid.northing - sign * 6_050_538.71).abs() < 0.01,
            "{method:?}: {grid:?}"
        );
        let back = projection
            .inverse(sign * 5_568_990.91, sign * 6_050_538.71)
            .expect("the example's grid point");
        let second = (0.001 / 3600.0_f64).to_radians();
        assert!(
            (back.latitude - point.latitude).abs() < second
                && (back.longitude - point.longitude).abs() < second,
            "{method:?}: {back:?}"
        );
    }
    for code in [5224, 5225] {
        assert!(
            matches!(
                system(code),
                Err(Refusal::MissingParameter {
                    parameter: "ordinate 1 of evaluation point",
                    ..
                })
            ),
            "EPSG {code}: {:?}",
            system(code)
        );
    }
}

#[test]
fn lambert_2sp_michigan_reproduces_the_michigan_central_example() {
    // Section 3.1.1.5's example, NAD27 / Michigan Central (EPSG 6201), in US survey feet:
    // 43°45′N 83°10′W is (2308335.75, 160210.48) ftUS. The reverse is printed "83°10'00.000"E",
    // the point's own longitude with the hemisphere misprinted; the forward's input is west.
    worked_example(
        6201,
        [43.75, -dms(83.0, 10.0, 0.0)],
        [2_308_335.75, 160_210.48],
        0.01,
    );
}

#[test]
fn american_polyconic_reproduces_professional_paper_1395s_example() {
    // Section 3.1.5 prints no example (it names GIGS test procedure 5107); its formulas are
    // Professional Paper 1395's, whose Appendix A works the ellipsoid: Clarke 1866, origin 30°N
    // 96°W, 40°N 75°W is (1776774.5, 1319657.8) m, printed to the decimetre.
    let clarke = Ellipsoid::new(6_378_206.4, 294.978_698_2).expect("Clarke 1866");
    let parameters = Parameters {
        origin_latitude: Some(30f64.to_radians()),
        origin_longitude: Some((-96f64).to_radians()),
        ..Parameters::default()
    };
    let projection = Projection::new(ProjectionMethod::AmericanPolyconic, clarke, parameters)
        .expect("the example's parameters");
    let point = LatLon {
        latitude: 40f64.to_radians(),
        longitude: (-75f64).to_radians(),
    };
    let grid = projection.forward(point).expect("the example's point");
    assert!(
        (grid.easting - 1_776_774.5).abs() < 0.1 && (grid.northing - 1_319_657.8).abs() < 0.1,
        "{grid:?}"
    );
    let back = projection
        .inverse(1_776_774.5, 1_319_657.8)
        .expect("the example's grid point");
    // The decimetre the grid point is printed to is about four micro-degrees.
    let printed = 1e-5f64.to_radians();
    assert!(
        (back.latitude - point.latitude).abs() < printed
            && (back.longitude - point.longitude).abs() < printed,
        "{back:?}"
    );
    // The equator's case on both sides of it: the forward's `a(λ − λO)` is the limit of the
    // general formula, and the reverse's case is `A = 0`, the northing at the equator being
    // `FN − MO` (ADR 3876).
    for latitude in [-1e-9, 0.0, 1e-9] {
        let grid = projection
            .forward(LatLon {
                latitude,
                longitude: (-90f64).to_radians(),
            })
            .expect("near the equator");
        let back = projection
            .inverse(grid.easting, grid.northing)
            .expect("its grid point");
        assert!(
            (back.latitude - latitude).abs() < 1e-10
                && (back.longitude - (-90f64).to_radians()).abs() < 1e-10,
            "{latitude}: {grid:?} {back:?}"
        );
    }
    // Through a system the table holds: SIRGAS 2000 / Brazil Polyconic (EPSG 5880).
    let brazil = projected_of(5880);
    for [latitude, longitude] in [[-15.0, -47.0], [-30.0, -60.0], [2.0, -40.0]] {
        let [e, n] = brazil
            .grid(GeographicPosition {
                latitude,
                longitude,
            })
            .expect("a point of Brazil");
        let back = brazil.geographic(e, n).expect("its grid point");
        assert!(
            (back.latitude - latitude).abs() < SAME_POSITION
                && (back.longitude - longitude).abs() < SAME_POSITION,
            "{back:?}"
        );
    }
}

#[test]
fn lambert_azimuthal_equal_area_spherical_reproduces_professional_paper_1395s_example() {
    // Section 3.3.2.1 sends its reader to Professional Paper 1395 "for formulas and example";
    // Appendix A's spherical example: R = 3, φ1 = 40°N, λ0 = 100°W, 20°S 100°E is
    // (−4.2339303, 4.0257775).
    let sphere = Ellipsoid::new(3.0, 0.0).expect("the example's sphere");
    let parameters = Parameters {
        origin_latitude: Some(40f64.to_radians()),
        origin_longitude: Some((-100f64).to_radians()),
        ..Parameters::default()
    };
    let projection = Projection::new(
        ProjectionMethod::LambertAzimuthalEqualAreaSpherical,
        sphere,
        parameters,
    )
    .expect("the example's parameters");
    let point = LatLon {
        latitude: (-20f64).to_radians(),
        longitude: 100f64.to_radians(),
    };
    let grid = projection.forward(point).expect("the example's point");
    assert!(
        (grid.easting + 4.233_930_3).abs() < 1e-7 && (grid.northing - 4.025_777_5).abs() < 1e-7,
        "{grid:?}"
    );
    let back = projection
        .inverse(-4.233_930_3, 4.025_777_5)
        .expect("the example's grid point");
    let printed = 1e-6f64.to_radians();
    assert!(
        (back.latitude - point.latitude).abs() < printed
            && (back.longitude - point.longitude).abs() < printed,
        "{back:?}"
    );
    // NSIDC EASE-Grid North (EPSG 3408) is on the NSIDC authalic sphere, R = 6371228 m: the polar
    // aspect puts the equator at 2R sin 45° = R√2 from the pole (equation 24-4), and on λ0 below
    // it. WGS 84 / NSIDC EASE-Grid North (EPSG 3973) states the same on the WGS 84 ellipsoid,
    // whose authalic radius RA is 6371007.18 m.
    for (code, radius) in [(3408, 6_371_228.0), (3973, 6_371_007.181)] {
        let system = projected_of(code);
        let [e, n] = system
            .grid(GeographicPosition {
                latitude: 0.0,
                longitude: 0.0,
            })
            .expect("the equator");
        assert!(
            e.abs() < 1e-6 && (n + radius * std::f64::consts::SQRT_2).abs() < 0.01,
            "EPSG {code}: ({e}, {n})"
        );
        let back = system
            .geographic(1_000_000.0, -2_000_000.0)
            .expect("a point");
        let [e, n] = system.grid(back).expect("its grid point");
        assert!(
            (e - 1_000_000.0).abs() < 1e-6 && (n + 2_000_000.0).abs() < 1e-6,
            "EPSG {code}: ({e}, {n})"
        );
    }
}

#[test]
fn hotine_b_at_an_azimuth_of_ninety_degrees_puts_the_centre_at_its_stated_grid_point() {
    // Section 3.2.4.1 states variant B's (EC, NC) at the projection centre. CH1903 / LV03 (EPSG
    // 21781) is the section's special case of αC = 90°: its centre, 46°57′08.66″N 7°26′22.50″E,
    // is (600000, 200000) m, and a point a metre of longitude either side lies within a metre
    // or so of it — the continuity ADR 3871's reading of the special case keeps.
    let system = projected_of(21781);
    let centre = [dms(46.0, 57.0, 8.66), dms(7.0, 26.0, 22.50)];
    let grid = |longitude: f64| {
        system
            .grid(GeographicPosition {
                latitude: centre[0],
                longitude,
            })
            .expect("the point projects")
    };
    let [e, n] = grid(centre[1]);
    assert!(
        (e - 600_000.0).abs() < 0.001 && (n - 200_000.0).abs() < 0.001,
        "({e}, {n})"
    );
    // A metre of longitude at 47°N is about 1.3e-5°.
    for longitude in [centre[1] - 1.3e-5, centre[1] + 1.3e-5] {
        let [e, n] = grid(longitude);
        assert!(
            (e - 600_000.0).abs() < 1.5 && (n - 200_000.0).abs() < 0.01,
            "({e}, {n}) at {longitude}"
        );
        let back = system.geographic(e, n).expect("the reverse");
        let second = 0.0001 / 3600.0;
        assert!(
            (back.latitude - centre[0]).abs() < second
                && (back.longitude - longitude).abs() < second,
            "{back:?}"
        );
    }
}

#[test]
fn the_polar_aspect_of_lambert_azimuthal_equal_area_returns_from_its_own_forward() {
    // Section 3.3.2 prints no example of its polar aspect, so this is the reverse undoing the
    // forward, at both poles, on the GRS 1980 figure, to a ten-thousandth of a second.
    for pole in [90.0_f64, -90.0] {
        let projection = Projection::new(
            ProjectionMethod::LambertAzimuthalEqualArea,
            Ellipsoid::new(6_378_137.0, 298.257_222_101).expect("GRS 1980"),
            Parameters {
                origin_latitude: Some(pole.to_radians()),
                origin_longitude: Some(0.0),
                ..Parameters::default()
            },
        )
        .expect("a polar aspect");
        for (latitude, longitude) in [(60.0_f64, 30.0_f64), (75.0, -120.0), (45.0, 179.0)] {
            let latitude = latitude.copysign(pole);
            let grid = projection
                .forward(LatLon {
                    latitude: latitude.to_radians(),
                    longitude: longitude.to_radians(),
                })
                .expect("a point of the hemisphere");
            let back = projection
                .inverse(grid.easting, grid.northing)
                .expect("its grid point");
            let tenth = (0.0001 / 3600.0_f64).to_radians();
            assert!(
                (back.latitude - latitude.to_radians()).abs() < tenth,
                "{back:?}"
            );
            assert!(
                (back.longitude - longitude.to_radians()).abs() < tenth,
                "{back:?}"
            );
        }
    }
}

/// The table is what its generator writes from the pinned PROJ release, byte for byte.
///
/// Ignored by default because it downloads that release (about 50 MB) into
/// `$PDF_VIEWER_SCRATCH/epsg/` when it is not there; run it with `--ignored` after changing the
/// generator or the pin (ADR 3821).
#[test]
#[ignore = "downloads the pinned PROJ release"]
fn the_table_is_what_the_generator_writes_from_the_pinned_release() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let script = root.join("tools/epsg-table.py");
    for step in [["--fetch"].as_slice(), ["--check"].as_slice()] {
        let status = std::process::Command::new("python3")
            .args(["-I", "-B"])
            .arg(&script)
            .args(step)
            .current_dir(&root)
            .status()
            .expect("python3 runs");
        assert!(status.success(), "tools/epsg-table.py {step:?}: {status}");
    }
}

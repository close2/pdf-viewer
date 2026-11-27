use super::*;
use crate::geospatial::datum;
use crate::geospatial::{GeographicPosition, system::from_wkt};

/// Ten micro-degrees of a micro-degree: a position agreeing this closely is the same position to
/// well under a millimetre, far finer than any coordinate §12.10's registration states.
const SAME_POSITION: f64 = 1e-9;

fn projected_of(code: i64) -> ProjectedSystem {
    match system(code) {
        Ok(ReferenceSystem::Projected(projected)) => projected,
        other => panic!("EPSG {code} as a projected system, got {other:?}"),
    }
}

fn projected_wkt(text: &str) -> ProjectedSystem {
    match from_wkt(text) {
        Ok(ReferenceSystem::Projected(projected)) => projected,
        other => panic!("a projected system, got {other:?}"),
    }
}

/// Both systems' latitude and longitude of each grid point, to [`SAME_POSITION`].
fn same_positions(by_code: &ProjectedSystem, by_wkt: &ProjectedSystem, points: &[[f64; 2]]) {
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
    // CH1903+ / LV95 is Hotine Oblique Mercator (variant B), EPSG method 9815.
    assert_eq!(
        system(2056),
        Err(Refusal::EpsgMethod {
            code: 2056,
            method: "Hotine Oblique Mercator (variant B)",
            method_code: 9815,
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
fn dms(degrees: f64, minutes: f64, seconds: f64) -> f64 {
    degrees.signum() * (degrees.abs() + minutes / 60.0 + seconds / 3600.0)
}

/// A worked example of Guidance Note 7-2 through the EPSG code of the system it is stated for:
/// the forward answer within `grid` of the printed easting and northing (in the system's unit),
/// and the reverse answer within a thousandth of a second, the example's printed precision.
fn worked_example(
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
                .forward(crate::geospatial::LatLon {
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

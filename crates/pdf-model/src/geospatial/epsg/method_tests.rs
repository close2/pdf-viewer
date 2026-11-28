//! `doc/todo/79`'s rows 12 to 18, each on its section's worked example or, where the section
//! prints none, on the example of the work it names, and through the systems the table carries.

use super::tests::{
    SAME_POSITION, dms, projected_of, projected_wkt, same_positions, worked_example,
};
use super::*;
use crate::geospatial::GeographicPosition;
use crate::geospatial::projection::LatLon;

/// Clarke 1866, the ellipsoid of Professional Paper 1395's examples (`e² = 0.00676866`).
fn clarke_1866() -> Ellipsoid {
    Ellipsoid::new(6_378_206.4, 294.978_698_2).expect("Clarke 1866")
}

/// A projection of the given method on the given figure.
fn projection(
    method: ProjectionMethod,
    ellipsoid: Ellipsoid,
    parameters: &Parameters,
) -> Projection {
    Projection::new(method, ellipsoid, *parameters).expect("the example's parameters")
}

/// The forward answer within `grid` of the printed one, and the reverse of the printed grid point
/// within `angle` radians of the point.
fn example(
    projection: &Projection,
    [latitude, longitude]: [f64; 2],
    [e, n]: [f64; 2],
    grid: f64,
    angle: f64,
) {
    let point = LatLon {
        latitude: latitude.to_radians(),
        longitude: longitude.to_radians(),
    };
    let forward = projection.forward(point).expect("the example's point");
    assert!(
        (forward.easting - e).abs() < grid && (forward.northing - n).abs() < grid,
        "{forward:?} against ({e}, {n})"
    );
    let back = projection.inverse(e, n).expect("the example's grid point");
    assert!(
        (back.latitude - point.latitude).abs() < angle
            && (back.longitude - point.longitude).abs() < angle,
        "{back:?} against {point:?}"
    );
}

/// Each point through a system's forward and back again, to [`SAME_POSITION`], or to
/// [`SERIES`] for a method whose reverse is a truncated series rather than an iteration.
fn round_trip(code: i64, points: &[[f64; 2]]) {
    round_trip_within(code, points, SAME_POSITION);
}

/// The reverse of a method whose latitude returns by section 3.3.2's authalic series, truncated
/// at `e⁶`: its own error is of the order of `e⁸`, about a centimetre, or 1e-7°.
const SERIES: f64 = 1e-7;

/// [`round_trip`] to `within` degrees.
fn round_trip_within(code: i64, points: &[[f64; 2]], within: f64) {
    let system = projected_of(code);
    for &[latitude, longitude] in points {
        let [e, n] = system
            .grid(GeographicPosition {
                latitude,
                longitude,
            })
            .unwrap_or_else(|refusal| panic!("EPSG {code}: ({latitude}, {longitude}): {refusal}"));
        let back = system.geographic(e, n).expect("its grid point");
        assert!(
            (back.latitude - latitude).abs() < within
                && (back.longitude - longitude).abs() < within,
            "EPSG {code}: {back:?} against ({latitude}, {longitude})"
        );
    }
}

/// A system's grid point for a latitude and longitude in degrees.
fn grid_of(code: i64, latitude: f64, longitude: f64) -> [f64; 2] {
    projected_of(code)
        .grid(GeographicPosition {
            latitude,
            longitude,
        })
        .expect("the point projects")
}

/// A spread of points over the whole of a world map.
const WORLD: [[f64; 2]; 6] = [
    [0.0, 0.0],
    [34.0, -117.0],
    [-45.0, 170.0],
    [89.0, -179.0],
    [-89.5, 60.0],
    [12.5, 179.9],
];

#[test]
fn equal_earth_reproduces_the_americas_example() {
    // Section 3.4.4's example, WGS 84 / Equal Earth Americas (EPSG 8858): 34°03′27.169″N
    // 117°11′48.349″W is (−2390749.042, 4242849.758) m.
    worked_example(
        8858,
        [dms(34.0, 3.0, 27.169), -dms(117.0, 11.0, 48.349)],
        [-2_390_749.042, 4_242_849.758],
        0.001,
    );
    for code in [8857, 8858, 8859] {
        round_trip_within(code, &WORLD, SERIES);
    }
    // The pole is `θ = π/3`, the polynomial's own end: north of its northing there is no
    // latitude, south of it there is one (the northing is flat in latitude there, so a metre is
    // some hundredths of a degree).
    let [_, pole] = grid_of(8857, 90.0, 0.0);
    let system = projected_of(8857);
    let below = system
        .geographic(0.0, pole - 1.0)
        .expect("a metre south of the pole");
    assert!(below.latitude < 90.0 && below.latitude > 89.9, "{below:?}");
    assert!(system.geographic(0.0, pole + 1.0).is_err());
    // The spherical form, `β = ϕ` and `Rq = R`, is the same evaluation on a sphere.
    let sphere = Ellipsoid::new(6_371_007.0, 0.0).expect("a sphere");
    let parameters = Parameters {
        origin_longitude: Some(0.0),
        ..Parameters::default()
    };
    let on_sphere = projection(ProjectionMethod::EqualEarth, sphere, &parameters);
    let latitude = 0.5_f64;
    let theta = (latitude.sin() * 3f64.sqrt() / 2.0).asin();
    let grid = on_sphere
        .forward(LatLon {
            latitude,
            longitude: 0.0,
        })
        .expect("a point");
    let t2 = theta * theta;
    let northing = 6_371_007.0
        * theta
        * (1.340_264 - 0.081_106 * t2 + t2.powi(3) * (0.000_893 + 0.003_796 * t2));
    assert!((grid.northing - northing).abs() < 1e-6, "{grid:?}");
}

#[test]
fn lambert_cylindrical_equal_area_reproduces_professional_paper_1395s_examples() {
    // Section 3.2.6 prints no formulas and names Professional Paper 1395 "for formulas and
    // example". Its ellipsoid example: Clarke 1866, standard parallel 5°, central meridian 75°W,
    // 5°N 78°W is (−332699.8, 554248.5) m, printed to the decimetre (about 1e-6°).
    let parameters = Parameters {
        parallel_1: Some(5f64.to_radians()),
        origin_longitude: Some((-75f64).to_radians()),
        ..Parameters::default()
    };
    let ellipsoidal = projection(
        ProjectionMethod::LambertCylindricalEqualArea,
        clarke_1866(),
        &parameters,
    );
    example(
        &ellipsoidal,
        [5.0, -78.0],
        [-332_699.8, 554_248.5],
        0.1,
        1e-6f64.to_radians(),
    );
    // Its sphere example, section 3.2.6.1's formulas: R = 1, standard parallel 30°, central
    // meridian 75°W, 35°N 80°E is (2.3428242, 0.6623090).
    let parameters = Parameters {
        parallel_1: Some(30f64.to_radians()),
        origin_longitude: Some((-75f64).to_radians()),
        ..Parameters::default()
    };
    let spherical = projection(
        ProjectionMethod::LambertCylindricalEqualAreaSpherical,
        Ellipsoid::new(1.0, 0.0).expect("the unit sphere"),
        &parameters,
    );
    example(
        &spherical,
        [35.0, 80.0],
        [2.342_824_2, 0.662_309_0],
        1e-7,
        // The paper's own reverse of the printed point is 34.9999988°, "35° N. lat, if there
        // were no round-off errors".
        2e-6f64.to_radians(),
    );
    // Through the table: WGS 84 / NSIDC EASE-Grid 2.0 Global (6933, ellipsoidal), and NSIDC
    // EASE-Grid Global on its sphere and on WGS 84 (3410 and 3975, spherical, the latter on the
    // authalic sphere). The ellipsoidal form's pole is `a qP / (2 k0)`, the spherical one's
    // `R / cos ϕ1`, and a northing past it has no latitude.
    for (code, within) in [(6933, SERIES), (3410, SAME_POSITION), (3975, SAME_POSITION)] {
        round_trip_within(code, &WORLD, within);
        let [_, pole] = grid_of(code, 90.0, 0.0);
        assert!(
            projected_of(code).geographic(0.0, pole + 1.0).is_err(),
            "{code}"
        );
    }
    let [_, pole] = grid_of(3410, 90.0, 0.0);
    assert!(
        (pole - 6_371_228.0 / 30f64.to_radians().cos()).abs() < 1e-6,
        "{pole}"
    );
}

#[test]
fn equidistant_cylindrical_reproduces_the_world_example() {
    // Section 3.2.5's example, WGS 84 / World Equidistant Cylindrical (EPSG 4087): 55°N 10°E is
    // (1113194.91, 6097230.31) m.
    worked_example(4087, [55.0, 10.0], [1_113_194.91, 6_097_230.31], 0.01);
    // 32663 states the same system by method 9842, the code 1028 replaced, its standard
    // parallel as a latitude of natural origin (ADR 3880).
    for [latitude, longitude] in WORLD {
        assert_eq!(
            grid_of(4087, latitude, longitude),
            grid_of(32663, latitude, longitude)
        );
    }
    // The reverse series in `n` against the forward arc, across the meridian: a wrong
    // coefficient shows far above the truncation's `n⁸`.
    let system = projected_of(4087);
    for tenth in -899..=899 {
        let latitude = f64::from(tenth) / 10.0;
        let [e, n] = grid_of(4087, latitude, 1.0);
        let back = system.geographic(e, n).expect("its grid point");
        assert!(
            (back.latitude - latitude).abs() < 1e-11,
            "{latitude}: {back:?}"
        );
    }
    assert!(system.geographic(0.0, 10_002_000.0).is_err());
}

#[test]
fn equidistant_cylindrical_spherical_takes_the_conformal_sphere_at_its_parallel() {
    // Section 3.2.5.1 prints no example. On a sphere `E = R (λ − λO) cos ϕ1` and `N = R ϕ`:
    // World Equidistant Cylindrical (Sphere), EPSG 4088 and its replaced code 3786, on the
    // authalic sphere of radius 6371007 m.
    let r = 6_371_007.0;
    for code in [4088, 3786] {
        let [e, n] = grid_of(code, 55.0, 10.0);
        assert!(
            (e - r * 10f64.to_radians()).abs() < 1e-6 && (n - r * 55f64.to_radians()).abs() < 1e-6,
            "{code}: ({e}, {n})"
        );
        round_trip(code, &WORLD);
    }
    // On an ellipsoid `R` is the implementation notes' `RC = a √(1 − e²) / (1 − e² sin² ϕ1)`,
    // which at ϕ1 = 0 is the semi-minor axis: WGS 84 / Plate Carree (EPSG 32662, method 9823).
    let b = 6_378_137.0 * (1.0 - 1.0 / 298.257_223_563);
    let [e, n] = grid_of(32662, 55.0, 10.0);
    assert!(
        (e - b * 10f64.to_radians()).abs() < 1e-6 && (n - b * 55f64.to_radians()).abs() < 1e-6,
        "({e}, {n})"
    );
    round_trip(32662, &WORLD);
}

/// Professional Paper 1395's Bonne example's projection: Clarke 1866, standard parallel `ϕO`,
/// central meridian 75°W.
fn bonne(method: ProjectionMethod, origin: f64) -> Projection {
    let parameters = Parameters {
        origin_latitude: Some(origin.to_radians()),
        origin_longitude: Some((-75f64).to_radians()),
        ..Parameters::default()
    };
    projection(method, clarke_1866(), &parameters)
}

#[test]
fn bonne_reproduces_professional_paper_1395s_example() {
    // Section 3.1.4 prints no example; its formulas are Professional Paper 1395's (19-8 to 19-14),
    // whose ellipsoid example takes Clarke 1866, ϕ1 = 40°N, λ0 = 75°W: 30°N 85°W is
    // (−962915.1, −1056065.0) m, printed to the decimetre.
    let general = bonne(ProjectionMethod::Bonne, 40.0);
    example(
        &general,
        [30.0, -85.0],
        [-962_915.1, -1_056_065.0],
        0.1,
        1e-6f64.to_radians(),
    );
    // Section 3.1.4.1's south-orientated form: `W = FE − ρ sin T`, `S = FN − (…)`.
    let south = bonne(ProjectionMethod::BonneSouthOrientated, 40.0);
    example(
        &south,
        [30.0, -85.0],
        [962_915.1, 1_056_065.0],
        0.1,
        1e-6f64.to_radians(),
    );
    // `ρ` and the `atan2` take the sign of ϕO: the origin mirrored south mirrors the northing.
    let mirrored = bonne(ProjectionMethod::Bonne, -40.0);
    example(
        &mirrored,
        [-30.0, -85.0],
        [-962_915.1, 1_056_065.0],
        0.1,
        1e-6f64.to_radians(),
    );
    // At a pole `m = 0`, the section's "use λ = λO"; a hair off it the general formula holds.
    let pole = general
        .forward(LatLon {
            latitude: 90f64.to_radians(),
            longitude: 0.3,
        })
        .expect("the pole");
    let at_pole = general
        .inverse(pole.easting, pole.northing)
        .expect("the pole's grid point");
    assert_eq!(at_pole.longitude, (-75f64).to_radians());
    let near = LatLon {
        latitude: 90f64.to_radians() - 1e-7,
        longitude: 0.3,
    };
    let grid = general.forward(near).expect("near the pole");
    let back = general
        .inverse(grid.easting, grid.northing)
        .expect("its grid point");
    assert!(
        (back.latitude - near.latitude).abs() < 1e-9 && (back.longitude - 0.3).abs() < 1e-5,
        "{back:?}"
    );
    // The equator divides `a mO / sin ϕO` by zero.
    assert!(matches!(
        Projection::new(
            ProjectionMethod::Bonne,
            clarke_1866(),
            Parameters {
                origin_latitude: Some(0.0),
                origin_longitude: Some(0.0),
                ..Parameters::default()
            }
        ),
        Err(Refusal::Unsupported { .. })
    ));
    // Lisbon 1890 (Lisbon) / Portugal Bonne (2963) and Lisbon 1890 / Portugal Bonne New (5017).
    let portugal = [[39.67, 0.5], [37.0, 0.0], [42.0, 1.5]];
    round_trip(2963, &portugal);
    let greenwich = [[39.67, -8.13], [37.0, -8.5], [42.0, -7.0]];
    round_trip(5017, &greenwich);
}

#[test]
fn laborde_reproduces_the_tananarive_example() {
    // Section 3.2.4.2's example, Tananarive (Paris) / Laborde Grid (EPSG 29701), whose base
    // counts longitude from Paris: 16°11′23.280″S, 46.800381173 grads east of Paris, is
    // (188333.848, 1098841.091) m.
    let east_of_paris = 46.800_381_173 * 0.9;
    worked_example(
        29701,
        [-dms(16.0, 11.0, 23.28), east_of_paris],
        [188_333.848, 1_098_841.091],
        0.001,
    );
    // Tananarive / Laborde Grid (8441) states the same grid from Greenwich: the point is
    // 44°27′27.260″E.
    worked_example(
        8441,
        [-dms(16.0, 11.0, 23.28), dms(44.0, 27.0, 27.26)],
        [188_333.848, 1_098_841.091],
        0.001,
    );
    // The comparison table's points by the Laborde formula, to the millimetre (northing column
    // first). Its second row's two columns are transposed: the pair printed under "Using Laborde
    // formula" is the one the Hotine variant B formulas give there (to the millimetre), and the
    // pair under Hotine is the Laborde one, which agrees with the worked example six kilometres
    // away to the same millimetre (ADR 3878). The second row is tested on the column it reads.
    for ([latitude, longitude], [northing, easting]) in [
        ([-dms(18.0, 54.0, 0.0), 47.5], [799_665.521, 511_921.054]),
        ([-dms(16.0, 12.0, 0.0), 44.4], [1_097_651.426, 182_184.985]),
        ([-dms(25.0, 40.0, 0.0), 45.3], [50_636.222, 285_294.334]),
        ([-12.0, 49.2], [1_561_109.146, 701_354.056]),
    ] {
        worked_example(8441, [latitude, longitude], [easting, northing], 0.001);
    }
}

#[test]
fn the_zoned_transverse_mercator_is_its_zones_transverse_mercator_with_the_zone_prefixed() {
    // Section 3.2.3.2 prints no example. WGS 84 / UTM grid system (northern hemisphere), EPSG
    // 32600, is in each zone the Transverse Mercator of that zone's system, its easting prefixed
    // by `Z × 10⁶`: 50°N 15°E is zone 33's (WGS 84 / UTM zone 33N, EPSG 32633).
    let [e, n] = grid_of(32600, 50.0, 15.0);
    let [e33, n33] = grid_of(32633, 50.0, 15.0);
    assert!(
        (e - (33e6 + e33)).abs() < 1e-6 && (n - n33).abs() < 1e-9,
        "({e}, {n})"
    );
    let [e, n] = grid_of(32700, -33.9, 18.4);
    let [e34, n34] = grid_of(32734, -33.9, 18.4);
    assert!(
        (e - (34e6 + e34)).abs() < 1e-6 && (n - n34).abs() < 1e-9,
        "({e}, {n})"
    );
    round_trip(
        32600,
        &[[50.0, 15.0], [10.0, -179.9], [70.0, 179.9], [0.0, 0.0]],
    );
    round_trip(32700, &[[-50.0, 15.0], [-10.0, -179.9], [-70.0, 179.9]]);
    // Both sides of a zone's edge, 12°E between zones 32 and 33, and of the antimeridian.
    let system = projected_of(32600);
    let zone = |longitude: f64| system.projection.zone_number(longitude.to_radians());
    assert_eq!(zone(12.0 - 1e-9), 32.0);
    assert_eq!(zone(12.0 + 1e-9), 33.0);
    assert_eq!(zone(180.0 - 1e-9), 60.0);
    assert_eq!(zone(-180.0 + 1e-9), 1.0);
    // An initial longitude east of a point: the section's floor-based `mod` with `INT` read as
    // `floor` puts 3°W in zone 60 of a grid starting at Greenwich (ADR 3879).
    let parameters = Parameters {
        origin_latitude: Some(0.0),
        initial_longitude: Some(0.0),
        zone_width: Some(6f64.to_radians()),
        scale: Some(0.9996),
        false_easting: Some(500_000.0),
        false_northing: Some(0.0),
        ..Parameters::default()
    };
    let from_greenwich = projection(
        ProjectionMethod::TransverseMercatorZonedGridSystem,
        system.base.ellipsoid,
        &parameters,
    );
    assert_eq!(from_greenwich.zone_number((-3f64).to_radians()), 60.0);
    assert_eq!(from_greenwich.zone_number((3f64).to_radians()), 1.0);
    // An easting whose millions name no zone has no position.
    assert!(system.geographic(500_000.0, 0.0).is_err());
    assert!(system.geographic(61_500_000.0, 0.0).is_err());
}

/// The older form's WGS 84 base, for the strings below.
const WGS84: &str = r#"GEOGCS["WGS 84",DATUM["WGS_1984",SPHEROID["WGS 84",6378137,298.257223563]],PRIMEM["Greenwich",0],UNIT["degree",0.0174532925199433]]"#;

#[test]
fn the_methods_are_read_from_a_wkt_string_by_their_names_and_codes() {
    let string = |projection: &str, parameters: &str| {
        projected_wkt(&format!(
            r#"PROJCS["a system",{WGS84},{projection},{parameters},PARAMETER["false_easting",0],PARAMETER["false_northing",0],UNIT["metre",1]]"#
        ))
    };
    // Equal Earth Americas by the Dataset's name.
    let americas = string(
        r#"PROJECTION["Equal_Earth"]"#,
        r#"PARAMETER["central_meridian",-90]"#,
    );
    same_positions(
        &projected_of(8858),
        &americas,
        &[[-2_390_749.042, 4_242_849.758], [1e6, -3e6]],
    );
    // The zoned Transverse Mercator's two parameters of its own, by name.
    let zoned = projected_wkt(&format!(
        r#"PROJCS["UTM grid system",{WGS84},PROJECTION["Transverse_Mercator_Zoned_Grid_System"],PARAMETER["latitude_of_origin",0],PARAMETER["Initial_Longitude",-180],PARAMETER["Zone_Width",6],PARAMETER["scale_factor",0.9996],PARAMETER["false_easting",500000],PARAMETER["false_northing",0],UNIT["metre",1]]"#
    ));
    same_positions(
        &projected_of(32600),
        &zoned,
        &[[33_500_000.0, 5_500_000.0], [1_400_000.0, 100_000.0]],
    );
    // Method 9842, the code 1028 replaced, states its parallel as a latitude of natural origin:
    // read by its code, it is 1028 with that latitude as ϕ1 (ADR 3880).
    let replaced = string(
        r#"PROJECTION["Equidistant_Cylindrical",AUTHORITY["EPSG","9842"]]"#,
        r#"PARAMETER["latitude_of_origin",30],PARAMETER["central_meridian",10]"#,
    );
    let current = string(
        r#"PROJECTION["Equidistant_Cylindrical"]"#,
        r#"PARAMETER["standard_parallel_1",30],PARAMETER["central_meridian",10]"#,
    );
    same_positions(&current, &replaced, &[[0.0, 0.0], [2e6, 5e6], [-1e6, -7e6]]);
    // Bonne and Laborde by name, and the cylindrical equal-area forms.
    for (projection, parameters) in [
        (
            r#"PROJECTION["Bonne"]"#,
            r#"PARAMETER["latitude_of_origin",40],PARAMETER["central_meridian",-75]"#,
        ),
        (
            r#"PROJECTION["Bonne_South_Orientated"]"#,
            r#"PARAMETER["latitude_of_origin",40],PARAMETER["central_meridian",-75]"#,
        ),
        (
            r#"PROJECTION["Laborde_Oblique_Mercator"]"#,
            r#"PARAMETER["latitude_of_center",-18.9],PARAMETER["longitude_of_center",46.437229],PARAMETER["azimuth_of_initial_line",18.9],PARAMETER["scale_factor_on_initial_line",0.9995]"#,
        ),
        (
            r#"PROJECTION["Lambert_Cylindrical_Equal_Area"]"#,
            r#"PARAMETER["standard_parallel_1",30],PARAMETER["central_meridian",0]"#,
        ),
        (
            r#"PROJECTION["Lambert_Cylindrical_Equal_Area_Spherical"]"#,
            r#"PARAMETER["standard_parallel_1",30],PARAMETER["central_meridian",0]"#,
        ),
        (
            r#"PROJECTION["Equidistant_Cylindrical_Spherical"]"#,
            r#"PARAMETER["standard_parallel_1",0],PARAMETER["central_meridian",0]"#,
        ),
    ] {
        let system = string(projection, parameters);
        let [e, n] = system
            .grid(GeographicPosition {
                latitude: -20.0,
                longitude: 45.0,
            })
            .expect("a point");
        let back = system.geographic(e, n).expect("its grid point");
        assert!(
            (back.latitude + 20.0).abs() < SERIES && (back.longitude - 45.0).abs() < SERIES,
            "{projection}: {back:?}"
        );
    }
}

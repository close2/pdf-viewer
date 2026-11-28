//! `doc/todo/79`'s rows 19 to 27, the methods one system each states, each on its section's
//! worked example through that system's code, or, where the section prints none, on the example
//! of the work it names.

use super::tests::{
    SAME_POSITION, dms, projected_of, projected_wkt, same_positions, worked_example,
};
use super::*;
use crate::geospatial::GeographicPosition;
use crate::geospatial::projection::LatLon;

/// A system's grid point for a latitude and longitude in degrees.
fn grid_of(code: i64, latitude: f64, longitude: f64) -> [f64; 2] {
    projected_of(code)
        .grid(GeographicPosition {
            latitude,
            longitude,
        })
        .expect("the point projects")
}

/// Each point through a system's forward and back again, to `within` degrees.
fn round_trip(code: i64, points: &[[f64; 2]], within: f64) {
    let system = projected_of(code);
    for &[latitude, longitude] in points {
        let [e, n] = grid_of(code, latitude, longitude);
        let back = system.geographic(e, n).expect("its grid point");
        let east = (back.longitude - longitude + 180.0).rem_euclid(360.0) - 180.0;
        assert!(
            (back.latitude - latitude).abs() < within && east.abs() < within,
            "EPSG {code}: {back:?} against ({latitude}, {longitude})"
        );
    }
}

#[test]
fn hyperbolic_cassini_reproduces_the_vanua_levu_example() {
    // Section 3.2.2.1's example, Vanua Levu 1915 / Vanua Levu Grid (EPSG 3139), whose grid is in
    // links: 16°50′29.2435″S 179°59′39.6115″E is (16015.2890, 13369.6601) chains, a hundred links
    // each, so the printed digit is a hundredth of a link.
    worked_example(
        3139,
        [-dms(16.0, 50.0, 29.2435), dms(179.0, 59.0, 39.6115)],
        [1_601_528.90, 1_336_966.01],
        0.01,
    );
    // The reverse is the section's estimate of the forward's `X³/6ρν`, which it offers for this
    // grid alone: over the island and its sea, to a ten-thousandth of a second.
    let island = [
        [-16.25, 179.33],
        [-16.0, 178.6],
        [-16.9, -179.8],
        [-15.7, 179.9],
        [-17.1, 178.9],
    ];
    round_trip(3139, &island, 1e-4 / 3600.0);
}

#[test]
fn modified_azimuthal_equidistant_reproduces_the_yap_islands_example() {
    // Section 3.3.3.1's example, Guam 1963 / Yap Islands (EPSG 3295): 9°35′47.493″N
    // 138°11′34.908″E is (42665.90, 65509.82) m.
    worked_example(
        3295,
        [dms(9.0, 35.0, 47.493), dms(138.0, 11.0, 34.908)],
        [42_665.90, 65_509.82],
        0.01,
    );
    round_trip(
        3295,
        &[[9.0, 138.0], [10.5, 139.5], [8.0, 137.0], [9.55, 136.5]],
        SAME_POSITION * 1e3,
    );
    // The section's case `sin α = 0`, a point on the origin's meridian, north and south of the
    // origin: the same point as the general formula's on either side of the meridian.
    let meridian = dms(138.0, 10.0, 7.48);
    for latitude in [10.5, 8.5] {
        let [e, n] = grid_of(3295, latitude, meridian);
        assert!((e - 40_000.0).abs() < 1e-6, "{e}");
        assert_eq!(n > 60_000.0, latitude > 9.55, "{n}");
        for side in [-1e-9, 1e-9] {
            let [e_side, n_side] = grid_of(3295, latitude, meridian + side);
            assert!(
                (e_side - e).abs() < 1e-3 && (n_side - n).abs() < 1e-6,
                "{latitude} {side}: ({e_side}, {n_side}) against ({e}, {n})"
            );
        }
    }
}

#[test]
fn guam_reproduces_the_guam_spcs_example() {
    // Section 3.3.3.2's example, Guam 1963 / Guam SPCS (EPSG 3993): 13°20′20.53846″N
    // 144°38′07.19265″E is (37712.48, 35242.00) m, and its three reverse steps return it.
    worked_example(
        3993,
        [dms(13.0, 20.0, 20.538_46), dms(144.0, 38.0, 7.192_65)],
        [37_712.48, 35_242.00],
        0.01,
    );
    // Three steps are "satisfactory over the small area of application": the island, to a
    // ten-thousandth of a second.
    round_trip(
        3993,
        &[[13.25, 144.65], [13.65, 144.95], [13.45, 144.7]],
        1e-4 / 3600.0,
    );
}

#[test]
fn spherical_mercator_reproduces_its_example_and_the_pseudo_mercators_sphere() {
    // Section 3.2.1.1's example, on a sphere of R = 6371007 m: 24°22′54.433″N 100°20′W is
    // (−11156569.90, 2796869.94) m, and `D` of its reverse is −0.438999665.
    let sphere = Ellipsoid::new(6_371_007.0, 0.0).expect("a sphere");
    let parameters = Parameters {
        origin_latitude: Some(0.0),
        origin_longitude: Some(0.0),
        ..Parameters::default()
    };
    let projection =
        Projection::new(ProjectionMethod::MercatorSpherical, sphere, parameters).expect("it");
    let point = LatLon {
        latitude: dms(24.0, 22.0, 54.433).to_radians(),
        longitude: -dms(100.0, 20.0, 0.0).to_radians(),
    };
    let grid = projection.forward(point).expect("the point");
    assert!(
        (grid.easting + 11_156_569.90).abs() < 0.01 && (grid.northing - 2_796_869.94).abs() < 0.01,
        "{grid:?}"
    );
    assert!((-2_796_869.94 / 6_371_007.0 + 0.438_999_665_f64).abs() < 1e-9);
    let back = projection
        .inverse(-11_156_569.90, 2_796_869.94)
        .expect("the grid point");
    assert!(
        (back.latitude - point.latitude).abs() < 1e-9
            && (back.longitude - point.longitude).abs() < 1e-9,
        "{back:?}"
    );
    // Popular Visualisation CRS / Mercator (EPSG 3785) states 9841 on a sphere of WGS 84's
    // semi-major axis with `kO = 1`: section 3.2.1.2's Pseudo-Mercator is the same formulas on
    // that radius, so WGS 84 / Pseudo-Mercator (3857) puts every point where it does.
    for [latitude, longitude] in [[24.381_787, -100.333_333], [-60.0, 170.0], [0.0, 0.0]] {
        let [e, n] = grid_of(3785, latitude, longitude);
        let [e_web, n_web] = grid_of(3857, latitude, longitude);
        assert!((e - e_web).abs() < 1e-6 && (n - n_web).abs() < 1e-6);
    }
    // Poleward of 88° the section says the formula "should not be used".
    let system = projected_of(3785);
    assert!(
        system
            .grid(GeographicPosition {
                latitude: 88.5,
                longitude: 0.0,
            })
            .is_err()
    );
}

#[test]
fn lambert_belgium_reproduces_the_belge_lambert_72_example() {
    // Section 3.1.1.4's example, Belge 1972 / Belge Lambert 72, which the table carries as
    // BD72 / Belge Lambert 72 (EPSG 31300): 50°40′46.461″N 5°48′26.533″E is (251763.20,
    // 153034.13) m. The example rounds the false easting to 150000.01 m and the Dataset states
    // 150000.01256, inside the printed digit.
    worked_example(
        31300,
        [dms(50.0, 40.0, 46.461), dms(5.0, 48.0, 26.533)],
        [251_763.20, 153_034.13],
        0.01,
    );
    round_trip(
        31300,
        &[[49.6, 2.6], [51.4, 6.3], [50.5, 4.36]],
        SAME_POSITION,
    );
}

#[test]
fn lambert_near_conformal_reproduces_the_levant_zone_example() {
    // Section 3.1.1.6's example, Deir ez Zor / Levant Zone (EPSG 22700): 37°31′17.625″N
    // 34°08′11.291″E is (15707.96, 623165.96) m by the truncated formulas.
    worked_example(
        22700,
        [dms(37.0, 31.0, 17.625), dms(34.0, 8.0, 11.291)],
        [15_707.96, 623_165.96],
        0.01,
    );
    round_trip(
        22700,
        &[[34.65, 37.35], [32.5, 35.5], [37.0, 42.0]],
        SAME_POSITION,
    );
}

#[test]
fn new_zealand_map_grid_reproduces_the_surveyor_generals_test_points() {
    // OSG Technical Report 4.2's section 5 test data, NZGD49 / New Zealand Map Grid (EPSG 27200),
    // its latitudes and longitudes to 1e-8° and its grid to the millimetre.
    let system = projected_of(27200);
    for ([latitude, longitude], [easting, northing]) in [
        (
            [-34.444_066_32, 172.739_193_71],
            [2_487_100.638, 6_751_049.719],
        ),
        (
            [-40.512_409_08, 172.723_105_54],
            [2_486_533.395, 6_077_263.661],
        ),
        (
            [-46.651_294_56, 169.172_062_43],
            [2_216_746.425, 5_388_508.765],
        ),
    ] {
        let [e, n] = grid_of(27200, latitude, longitude);
        assert!(
            (e - easting).abs() < 0.001 && (n - northing).abs() < 0.001,
            "({e}, {n}) against ({easting}, {northing})"
        );
        let back = system.geographic(easting, northing).expect("its point");
        assert!(
            (back.latitude - latitude).abs() < 1e-8 && (back.longitude - longitude).abs() < 1e-8,
            "{back:?} against ({latitude}, {longitude})"
        );
    }
    round_trip(
        27200,
        &[
            [-41.0, 173.0],
            [-36.85, 174.76],
            [-45.87, 170.5],
            [-47.0, 167.5],
        ],
        SAME_POSITION,
    );
}

#[test]
fn tunisia_mining_grid_reproduces_the_sections_example_and_the_decrees_ends() {
    // Section 3.4.2's example through Carthage (Paris) / Tunisia Mining Grid (EPSG 22300), whose
    // grid is in kilometres and whose base counts grads from Paris: grid reference 302598 is
    // 38.97997 grads north and 8.22437 grads east of Paris.
    let system = projected_of(22300);
    let grads = |position: GeographicPosition| [position.latitude / 0.9, position.longitude / 0.9];
    let at = |e: f64, n: f64| grads(system.geographic(e, n).expect("a grid point"));
    let [latitude, longitude] = at(302.0, 598.0);
    assert!((latitude - 38.979_97).abs() < 5e-6, "{latitude}");
    assert!((longitude - 8.224_37).abs() < 5e-6, "{longitude}");
    // The decree's ends, which the section's rates give to the printed digit: easting 94 km at
    // 5.68989 grads and 490 km at 10.51515; northing 40 km at 33.39 and 860 km at 41.6039.
    assert!((at(94.0, 360.0)[1] - 5.689_89).abs() < 5e-6);
    assert!((at(490.0, 360.0)[1] - 10.515_15).abs() < 5e-6);
    assert!((at(270.0, 40.0)[0] - 33.39).abs() < 5e-6);
    assert!((at(270.0, 860.0)[0] - 41.6039).abs() < 5e-6);
    // The origin of the grid, Djebel Kebar, at grid reference 270582 is 38.81973 grads north.
    assert!((at(270.0, 582.0)[0] - 38.819_73).abs() < 5e-6);
    // Either side of the 360 km northing, where the rate changes, the grid is one piece, and the
    // forward takes the rate the reverse did.
    let [below, _] = at(270.0, 360.0 - 1e-9);
    let [above, _] = at(270.0, 360.0 + 1e-9);
    assert!((above - below).abs() < 1e-10 && above > below);
    round_trip(
        22300,
        &[
            [36.5964 * 0.9, 7.834_45 * 0.9],
            [38.0, 8.0],
            [31.0, 7.0],
            [36.5964 * 0.9 + 1e-7, 6.0],
            [36.5964 * 0.9 - 1e-7, 6.0],
        ],
        SAME_POSITION,
    );
    let [_, north] = grid_of(22300, 36.5964 * 0.9 + 0.9, 7.0);
    let [_, south] = grid_of(22300, 36.5964 * 0.9 - 0.9, 7.0);
    assert!((north - 360.0 - 1.0 / 0.010_015).abs() < 1e-9, "{north}");
    assert!((360.0 - south - 1.0 / 0.010_02).abs() < 1e-9, "{south}");
}

#[test]
fn transverse_mercator_3d_reproduces_the_luxembourg_example() {
    // The Dataset's own example for method 1111, LUREF / Luxembourg TM (3D) (EPSG 9895):
    // 49°34′00.7381″N 5°56′16.1460″E is (63444.834, 70387.372) m, its ellipsoidal height of
    // 328.061 m carried across unchanged, which a two-dimensional grid has no axis for.
    worked_example(
        9895,
        [dms(49.0, 34.0, 0.7381), dms(5.0, 56.0, 16.146)],
        [63_444.834, 70_387.372],
        0.001,
    );
}

#[test]
fn the_methods_are_read_from_a_wkt_string_by_their_codes() {
    // Each by the method code a WKT string's `AUTHORITY` gives it, the system's own parameters
    // by their codes, at two grid points of the system the code states.
    let wkt = |base: &str, method: &str, code: u32, parameters: &str| {
        projected_wkt(&format!(
            r#"PROJCS["a system",{base},PROJECTION["{method}",AUTHORITY["EPSG","{code}"]],{parameters},UNIT["metre",1]]"#
        ))
    };
    let international = r#"GEOGCS["NZGD49",DATUM["NZGD49",SPHEROID["International 1924",6378388,297]],PRIMEM["Greenwich",0],UNIT["degree",0.0174532925199433]]"#;
    let nzmg = wkt(
        international,
        "New_Zealand_Map_Grid",
        9811,
        r#"PARAMETER["latitude_of_origin",-41],PARAMETER["central_meridian",173],PARAMETER["false_easting",2510000],PARAMETER["false_northing",6023150]"#,
    );
    same_positions(
        &projected_of(27200),
        &nzmg,
        &[
            [2_487_100.638, 6_751_049.719],
            [2_216_746.425, 5_388_508.765],
        ],
    );
    let clarke_1866 = r#"GEOGCS["Guam 1963",DATUM["Guam 1963",SPHEROID["Clarke 1866",6378206.4,294.978698213898]],PRIMEM["Greenwich",0],UNIT["degree",0.0174532925199433]]"#;
    let guam = wkt(
        clarke_1866,
        "Guam_Projection",
        9831,
        r#"PARAMETER["latitude_of_origin",13.4724663528],PARAMETER["central_meridian",144.7487507056],PARAMETER["false_easting",50000],PARAMETER["false_northing",50000]"#,
    );
    same_positions(
        &projected_of(3993),
        &guam,
        &[[37_712.48, 35_242.00], [60_000.0, 70_000.0]],
    );
}

//! Exact-xref source-derived checks for patch continuation fields and Type 7 control order.
//! Table-derived control coordinates and an independently inverted Bernstein surface are oracles.

#![expect(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "small exact fixtures and their in-bounds pixel/control-point checks should fail loudly"
)]

use std::fmt::Write as _;

use pdf_render::{Color, PatchCorners, Rasterizer, Shading, ShadingKind, TargetSpec};
use pdf_syntax::Document;
use render_cpu::CpuRasterizer;

const GENEROUS: u64 = 1 << 24;

fn pdf(mesh: &str) -> Vec<u8> {
    let content = "/Sh0 sh";
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] \
         /Resources << /Shading << /Sh0 5 0 R >> >> /Contents 4 0 R >>\nendobj\n\
         4 0 obj\n<< /Length {} >>\nstream\n{}\nendstream\nendobj\n\
         5 0 obj\n{}\nendobj\n",
        content.len() + 1,
        content,
        mesh
    );
    let mut bytes = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for object in body.split_inclusive("endobj\n") {
        offsets.push(bytes.len());
        bytes.push_str(object);
    }
    let xref = bytes.len();
    let size = offsets.len() + 1;
    let _ = writeln!(bytes, "xref\n0 {size}");
    bytes.push_str("0000000000 65535 f \n");
    for offset in offsets {
        let _ = writeln!(bytes, "{offset:010} 00000 n ");
    }
    let _ = write!(
        bytes,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n"
    );
    bytes.into_bytes()
}

fn interpret(mesh: &str) -> pdf_model::content::Interpretation {
    let document = Document::open(pdf(mesh)).expect("complete exact-xref PDF");
    assert!(!document.was_recovered(), "fixture xref is exact");
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    let interpretation = pdf_model::interpret(&document, &page);
    assert!(
        interpretation.is_complete(),
        "valid patch fixture: {:?}",
        interpretation.unsupported
    );
    interpretation
}

fn shading(interpretation: &pdf_model::content::Interpretation) -> std::sync::Arc<Shading> {
    interpretation
        .display_list
        .commands()
        .iter()
        .find_map(|command| match command {
            pdf_render::Command::Fill {
                paint: pdf_render::Paint::Shading(shading),
                ..
            } => Some(std::sync::Arc::clone(shading)),
            _ => None,
        })
        .expect("the page paints the shading")
}

fn patch_mesh(kind: u8, data: &str, color_decode: &str, function: &str) -> String {
    format!(
        "<< /ShadingType {kind} /ColorSpace /DeviceRGB /BitsPerCoordinate 8 \
         /BitsPerComponent 8 /BitsPerFlag 8 /Decode [0 255 0 255 {color_decode}] \
         {function} /Filter /ASCIIHexDecode /Length {} >>\nstream\n{}\nendstream",
        data.len(),
        data
    )
}

fn pixel(raster: &pdf_render::Raster, x: u32, y: u32) -> [u8; 4] {
    let at = usize::try_from((y * raster.width + x) * 4).expect("bounded pixel");
    [
        raster.data[at],
        raster.data[at + 1],
        raster.data[at + 2],
        raster.data[at + 3],
    ]
}

fn render_shading(mesh: &str) -> pdf_render::Raster {
    let interpretation = interpret(mesh);
    let target =
        TargetSpec::for_page(&interpretation.display_list, 1.0, GENEROUS).expect("small target");
    CpuRasterizer::new()
        .with_medium(pdf_render::Medium::NONE)
        .rasterize(&interpretation.display_list, target)
        .expect("supported shading")
}

const PREVIOUS: &str = "00 0A0A 0A14 0A1E 0A28 1428 1E28 2828 281E 2814 280A 1E0A 140A \
     FF0000 00FF00 0000FF FFFF00";

fn continuation(flag: u8) -> String {
    // Each row lists the eight explicit point pairs after the four shared controls, followed
    // by the two explicit corner colours. All values are byte aligned; this avoids the open
    // per-patch padding interpretation while testing the three Table 84 inheritance maps.
    let (rest, corner_colors) = match flag {
        1 => ("3228 3C28 4628 461E 4614 460A 3C0A 320A", "0000FF FF0000"),
        2 => ("282E 283C 2846 1E46 1446 0A46 0A3C 0A32", "FF0000 00FF00"),
        _ => ("1E28 1428 0A28 0A1E 0A14 0A0A 140A 1E0A", "00FF00 FF0000"),
    };
    let data = format!("{PREVIOUS} {flag:02X} {rest} {corner_colors} >");
    patch_mesh(6, &data, "0 1 0 1 0 1", "")
}

#[test]
fn type6_continuation_flags_reuse_the_table84_edge_controls_and_colours() {
    let first = [
        Color {
            r: 1.0,
            g: 0.0,
            b: 0.0,
            a: 1.0,
        },
        Color {
            r: 0.0,
            g: 1.0,
            b: 0.0,
            a: 1.0,
        },
        Color {
            r: 0.0,
            g: 0.0,
            b: 1.0,
            a: 1.0,
        },
        Color {
            r: 1.0,
            g: 1.0,
            b: 0.0,
            a: 1.0,
        },
    ];
    let expected_edges = [
        [[10.0, 40.0], [20.0, 40.0], [30.0, 40.0], [40.0, 40.0]],
        [[40.0, 40.0], [40.0, 30.0], [40.0, 20.0], [40.0, 10.0]],
        [[40.0, 10.0], [30.0, 10.0], [20.0, 10.0], [10.0, 10.0]],
    ];
    let expected_shared = [
        [first[1], first[2]],
        [first[2], first[3]],
        [first[3], first[0]],
    ];

    for flag in 1_u8..=3 {
        let mesh = continuation(flag);
        let interpretation = interpret(&mesh);
        let shading = shading(&interpretation);
        let ShadingKind::Mesh { patches, .. } = shading.kind.as_ref() else {
            panic!("Type 6 is a patch mesh")
        };
        let patches = &patches.as_ref().expect("two complete patches").patches;
        assert_eq!(patches.len(), 2, "flag {flag}");
        // Table 84's inherited first edge becomes p00,p01,p02,p03 in the new patch.
        for (actual, expected) in patches[1].net[0]
            .iter()
            .zip(expected_edges[usize::from(flag - 1)])
        {
            assert!(
                (actual.x - expected[0]).abs() < 0.01 && (actual.y - expected[1]).abs() < 0.01,
                "flag {flag} inherited point differs from Table 84: {actual:?}, expected {expected:?}"
            );
        }
        let PatchCorners::Colours(actual) = &patches[1].corners else {
            panic!("fixture uses direct RGB colours")
        };
        for (actual, expected) in actual
            .iter()
            .take(2)
            .zip(expected_shared[usize::from(flag - 1)])
        {
            assert_eq!(
                *actual, expected,
                "flag {flag} inherits the Table 84 corner colour"
            );
        }
    }
}

#[test]
fn type7_table85_interior_order_changes_the_public_parameterized_pixel() {
    // Table 85 streams p11,p12,p22,p21 after the 12 boundary points. The four asymmetric
    // interior controls make swapping p11/p12 materially move the surface. At device pixel
    // (39,51), whose centre is page point (39.5,48.5), the source tensor Bernstein equation
    // gives v≈0.55448, so Table 85's t=v and t² function produce red≈78. Swapping p11/p12
    // instead gives v≈0.79568 (red≈161). This expected pixel is independently calculated
    // from the table's p_ij order and Bernstein weights, not read from another renderer.
    let data = "00 \
        0A0A 0A1E 0A32 0A46 1E46 3246 4646 4632 461E 460A 320A 1E0A \
        0000 0064 6400 6464 \
        00 FF FF 00 >";
    let mesh = patch_mesh(
        7,
        data,
        "0 1",
        "/Function << /FunctionType 2 /Domain [0 1] /C0 [0 0 0] /C1 [1 0 0] /N 2 >>",
    );
    let interpretation = interpret(&mesh);
    let shading = shading(&interpretation);
    let ShadingKind::Mesh { patches, .. } = shading.kind.as_ref() else {
        panic!("Type 7 is a patch mesh")
    };
    let net = &patches.as_ref().expect("one patch").patches[0].net;
    // The stream's four interiors are placed at [i][j] = [1][1], [1][2], [2][2], [2][1].
    for (actual, expected) in [
        (net[1][1], [0.0, 0.0]),
        (net[1][2], [0.0, 100.0]),
        (net[2][2], [100.0, 0.0]),
        (net[2][1], [100.0, 100.0]),
    ] {
        assert!(
            (actual.x - expected[0]).abs() < 0.01 && (actual.y - expected[1]).abs() < 0.01,
            "Table 85 interior control order: {actual:?} != {expected:?}"
        );
    }
    let target =
        TargetSpec::for_page(&interpretation.display_list, 1.0, GENEROUS).expect("small target");
    let raster = CpuRasterizer::new()
        .with_medium(pdf_render::Medium::NONE)
        .rasterize(&interpretation.display_list, target)
        .expect("supported tensor patch");
    let sample = pixel(&raster, 39, 51);
    assert_eq!(
        sample[3], 255,
        "the computed interior point is covered: {sample:?}"
    );
    assert!(
        (68..=90).contains(&sample[0]) && sample[1] < 4 && sample[2] < 4,
        "p11,p12,p22,p21 order gives v≈0.554 and t²≈78/255 red; got {sample:?}"
    );
}

#[test]
fn axial_and_radial_nonunit_domains_map_geometry_fraction_into_function_input() {
    // Tables 79/80 define Domain as the function-input limits, not as geometry bounds. At a
    // quarter of either gradient, [0.2,0.8] therefore gives t=0.2+0.25*0.6=0.35 and a linear
    // black-to-red function gives about 89/255. Treating the default [0,1] as fixed would give
    // about 64/255. The radial sample is about 10.5119 units from the centre in a radius-40 family,
    // so its s is about 0.2628; both expected values come from the source equations.
    let function = "/Function << /FunctionType 2 /Domain [0 1] \
                    /C0 [0 0 0] /C1 [1 0 0] /N 1 >>";
    let axial = format!(
        "<< /ShadingType 2 /ColorSpace /DeviceRGB /Coords [0 0 100 0] \
         /Domain [0.2 0.8] {function} >>"
    );
    let radial = format!(
        "<< /ShadingType 3 /ColorSpace /DeviceRGB /Coords [50 50 0 50 50 40] \
         /Domain [0.2 0.8] {function} >>"
    );
    for (kind, mesh, x, y) in [("axial", axial, 25, 50), ("radial", radial, 60, 50)] {
        let raster = render_shading(&mesh);
        let sample = pixel(&raster, x, y);
        assert_eq!(
            sample[3], 255,
            "{kind} paints the in-domain sample: {sample:?}"
        );
        assert!(
            (82..=97).contains(&sample[0]) && sample[1] < 4 && sample[2] < 4,
            "{kind} maps its quarter-position into Domain [0.2,0.8] before Function; got {sample:?}"
        );
    }
}

const TYPE7_FIRST_PATCH: &str = "00 \
    0A0A 0A14 0A1E 0A28 1428 1E28 2828 281E 2814 280A 1E0A 140A \
    1414 141E 1E1E 1E14 \
    FF0000 00FF00 0000FF FFFF00";

fn type7_continuation_mesh(flag: u8) -> String {
    let new_points = "3228 3C28 4628 4620 4618 4610 3C10 3210 3218 3220 3C20 3C18";
    let new_corner_colours = "FFFF00 FF0000";
    let data = format!("{TYPE7_FIRST_PATCH} {flag:02X} {new_points} {new_corner_colours} >");
    patch_mesh(7, &data, "0 1 0 1 0 1", "")
}

#[test]
fn type7_continuations_reuse_the_table85_edge_and_read_all_tensor_interior_points() {
    let expected_edges = [
        [[10.0, 40.0], [20.0, 40.0], [30.0, 40.0], [40.0, 40.0]],
        [[40.0, 40.0], [40.0, 30.0], [40.0, 20.0], [40.0, 10.0]],
        [[40.0, 10.0], [30.0, 10.0], [20.0, 10.0], [10.0, 10.0]],
    ];
    let expected_shared = [
        [Color::rgb(0.0, 1.0, 0.0), Color::rgb(0.0, 0.0, 1.0)],
        [Color::rgb(0.0, 0.0, 1.0), Color::rgb(1.0, 1.0, 0.0)],
        [Color::rgb(1.0, 1.0, 0.0), Color::rgb(1.0, 0.0, 0.0)],
    ];
    let expected_interior = [[50.0, 24.0], [50.0, 32.0], [60.0, 32.0], [60.0, 24.0]];

    for flag in 1_u8..=3 {
        let mesh = type7_continuation_mesh(flag);
        let bytes = pdf(&mesh);
        let document = Document::open(bytes).expect("valid exact-xref Type 7 continuation");
        assert!(!document.was_recovered(), "flag {flag} has an exact xref");
        assert!(
            !document.cross_reference_chain_was_cut(),
            "flag {flag} has no cut xref chain"
        );

        let interpretation = interpret(&mesh);
        let shading = shading(&interpretation);
        let ShadingKind::Mesh { patches, .. } = shading.kind.as_ref() else {
            panic!("Type 7 is a patch mesh")
        };
        let patches = &patches
            .as_ref()
            .expect("two complete tensor patches")
            .patches;
        assert_eq!(
            patches.len(),
            2,
            "flag {flag} reads the Table 85 continuation payload"
        );
        for (actual, expected) in patches[1].net[0]
            .iter()
            .zip(expected_edges[usize::from(flag - 1)])
        {
            assert!(
                (actual.x - expected[0]).abs() < 0.01 && (actual.y - expected[1]).abs() < 0.01,
                "flag {flag} inherited edge differs from Table 85: {actual:?} vs {expected:?}"
            );
        }
        for (actual, expected) in [
            patches[1].net[1][1],
            patches[1].net[1][2],
            patches[1].net[2][2],
            patches[1].net[2][1],
        ]
        .into_iter()
        .zip(expected_interior)
        {
            assert!(
                (actual.x - expected[0]).abs() < 0.01 && (actual.y - expected[1]).abs() < 0.01,
                "flag {flag} interior point from Table 85 is misplaced: {actual:?}"
            );
        }
        let PatchCorners::Colours(actual) = &patches[1].corners else {
            panic!("fixture specifies direct RGB corner colours")
        };
        for (actual, expected) in actual.iter().zip(
            expected_shared[usize::from(flag - 1)]
                .into_iter()
                .chain([Color::rgb(1.0, 1.0, 0.0), Color::rgb(1.0, 0.0, 0.0)]),
        ) {
            assert_eq!(
                *actual, expected,
                "flag {flag} has the inherited and explicit Table 85 corner colours"
            );
        }
    }
}

fn two_overlapping_type6_patches(first: &str, second: &str) -> String {
    let points = "0A0A 0A14 0A1E 0A28 1428 1E28 2828 281E 2814 280A 1E0A 140A";
    let data = format!("00 {points} {first} 00 {points} {second} >");
    patch_mesh(6, &data, "0 1 0 1 0 1", "")
}

#[test]
fn a_later_overlapping_patch_paints_over_the_earlier_patch() {
    let red = "FF0000 FF0000 FF0000 FF0000";
    let blue = "0000FF 0000FF 0000FF 0000FF";

    // Single-patch controls establish both literal colors at page (25.5, 25.5),
    // which is device pixel (25, 74) on this 100-point page with a top-down raster.
    let points = "0A0A 0A14 0A1E 0A28 1428 1E28 2828 281E 2814 280A 1E0A 140A";
    for (colours, expected) in [(red, [255, 0, 0, 255]), (blue, [0, 0, 255, 255])] {
        let data = format!("00 {points} {colours} >");
        let control = render_shading(&patch_mesh(6, &data, "0 1 0 1 0 1", ""));
        assert_eq!(pixel(&control, 25, 74), expected, "single patch control");
    }

    // Section 8.7.4.5.7 (physical page 261) requires the later patch to paint over the earlier.
    // Both complete patches map the same square [10,40]×[10,40]; their interiors
    // overlap exactly. Literal color controls isolate stream-order compositing.
    let red_then_blue = render_shading(&two_overlapping_type6_patches(red, blue));
    assert_eq!(
        pixel(&red_then_blue, 25, 74),
        [0, 0, 255, 255],
        "later blue patch wins"
    );
    let blue_then_red = render_shading(&two_overlapping_type6_patches(blue, red));
    assert_eq!(
        pixel(&blue_then_red, 25, 74),
        [255, 0, 0, 255],
        "reversing stream order reverses winner"
    );
}

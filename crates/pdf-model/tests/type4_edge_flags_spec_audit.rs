//! Source-derived Type 4 topology regression for Table 81 flag 1/2 inheritance.
//! The expected vertices and colors come directly from §8.7.4.5.5, not another code path.

#![expect(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "small exact fixture and expected triangle values must fail loudly"
)]

use std::fmt::Write as _;

use pdf_render::{Color, Corners, ShadingKind};
use pdf_syntax::{Document, Limits};

fn vertex(flag: u8, x: u8, y: u8, rgb: [u8; 3]) -> String {
    let mut bits = format!("{flag:02b}{x:08b}{y:08b}");
    for component in rgb {
        let _ = write!(bits, "{component:08b}");
    }
    // The prose following Table 81 requires each complete vertex record to end at a byte boundary; pad bits are
    // ignored. Fields are serialized high-order bit first as the table specifies.
    while bits.len() % 8 != 0 {
        bits.push('0');
    }
    let mut hex = String::new();
    for byte in bits.as_bytes().chunks(8) {
        let octet = std::str::from_utf8(byte).expect("ASCII bits");
        let value = u8::from_str_radix(octet, 2).expect("eight bits");
        let _ = write!(hex, "{value:02X} ");
    }
    hex
}

fn pdf(mesh: &str) -> Vec<u8> {
    let content = "/Sh0 sh\n";
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] \
         /Resources << /Shading << /Sh0 5 0 R >> >> /Contents 4 0 R >>\nendobj\n\
         4 0 obj\n<< /Length {} >>\nstream\n{}endstream\nendobj\n\
         5 0 obj\n{}\nendobj\n",
        content.len(),
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

fn triangles() -> Vec<pdf_render::Triangle> {
    let data = [
        vertex(0, 10, 10, [255, 0, 0]),   // va: red
        vertex(2, 80, 10, [0, 255, 0]),   // vb: ignored flag; green
        vertex(1, 10, 80, [0, 0, 255]),   // vc: ignored flag; blue
        vertex(1, 80, 80, [255, 255, 0]), // flag 1: (vb, vc, vd)
        vertex(2, 50, 95, [255, 0, 255]), // flag 2: (va, vc, vd) of previous triangle
    ]
    .concat()
        + ">";
    let shading = format!(
        "<< /ShadingType 4 /ColorSpace /DeviceRGB /BitsPerCoordinate 8 \
         /BitsPerComponent 8 /BitsPerFlag 2 /Decode [0 255 0 255 0 1 0 1 0 1] \
         /Filter /ASCIIHexDecode /Length {} >>\nstream\n{}\nendstream",
        data.len(),
        data
    );
    let bytes = pdf(&shading);
    let document = Document::open_with_limits(bytes, Limits::DEFAULT).expect("exact-xref PDF");
    assert!(!document.was_recovered(), "the source xref is exact");
    assert!(!document.cross_reference_chain_was_cut());
    let page = pdf_model::Pages::new(&document).get(0).expect("one page");
    let interpretation = pdf_model::interpret(&document, &page);
    assert!(
        interpretation.is_complete(),
        "valid Type 4 records: {:?}",
        interpretation.unsupported
    );
    interpretation
        .display_list
        .commands()
        .iter()
        .find_map(|command| match command {
            pdf_render::Command::Fill {
                paint: pdf_render::Paint::Shading(shading),
                ..
            } => match shading.kind.as_ref() {
                ShadingKind::Mesh { triangles, .. } => Some(triangles.to_vec()),
                _ => None,
            },
            _ => None,
        })
        .expect("the display list contains a Type 4 mesh")
}

fn color(r: f32, g: f32, b: f32) -> Color {
    Color { r, g, b, a: 1.0 }
}

#[test]
fn type4_continuation_flags_reuse_the_table81_triangle_edges() {
    let triangles = triangles();
    assert_eq!(
        triangles.len(),
        3,
        "flag 0 starts one; flags 1 and 2 add one each"
    );
    assert_eq!(
        triangles[0].points,
        [
            pdf_render::Point::new(10.0, 10.0),
            pdf_render::Point::new(80.0, 10.0),
            pdf_render::Point::new(10.0, 80.0),
        ],
        "the initial flag-0 control triangle"
    );
    assert_eq!(
        triangles[0].corners,
        Corners::Colours([
            color(1.0, 0.0, 0.0),
            color(0.0, 1.0, 0.0),
            color(0.0, 0.0, 1.0)
        ])
    );
    assert_eq!(
        triangles[1].points,
        [
            pdf_render::Point::new(80.0, 10.0),
            pdf_render::Point::new(10.0, 80.0),
            pdf_render::Point::new(80.0, 80.0),
        ],
        "Table 81 flag 1 selects (vb, vc, vd)"
    );
    assert_eq!(
        triangles[1].corners,
        Corners::Colours([
            color(0.0, 1.0, 0.0),
            color(0.0, 0.0, 1.0),
            color(1.0, 1.0, 0.0)
        ]),
        "flag 1 inherits vb/vc colors and reads vd"
    );
    assert_eq!(
        triangles[2].points,
        [
            pdf_render::Point::new(80.0, 10.0),
            pdf_render::Point::new(80.0, 80.0),
            pdf_render::Point::new(50.0, 95.0),
        ],
        "Table 81 flag 2 selects (va, vc, vd) from the preceding triangle"
    );
    assert_eq!(
        triangles[2].corners,
        Corners::Colours([
            color(0.0, 1.0, 0.0),
            color(1.0, 1.0, 0.0),
            color(1.0, 0.0, 1.0)
        ]),
        "flag 2 inherits va/vc colors and reads its new vertex"
    );
}

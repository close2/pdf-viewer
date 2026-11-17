//! Type5 scalar interpolation precedes its nonlinear Function (Table82).
#![expect(
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "small exact-xref fixture and known in-bounds pixel should fail loudly"
)]

use pdf_render::{Rasterizer, TargetSpec};
use pdf_syntax::Document;
use render_cpu::CpuRasterizer;
use std::fmt::Write as _;

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

#[test]
fn type5_interpolates_vertex_parameter_before_nonlinear_function() {
    // Each Type 5 record is x y t (no flag, and one t rather than three color samples).
    // The parameter is 0 at the left vertices and 1 at the right vertices.
    let data = "00 00 00  FF 00 FF  00 FF 00  FF FF FF >";
    let mesh = format!(
        "<< /ShadingType 5 /ColorSpace /DeviceRGB /VerticesPerRow 2 \
         /BitsPerCoordinate 8 /BitsPerComponent 8 /Decode [0 100 0 100 0 1] \
         /Function << /FunctionType 2 /Domain [0 1] /C0 [0 0 0] /C1 [1 0 0] /N 2 >> \
         /Filter /ASCIIHexDecode /Length {} >>\nstream\n{data}\nendstream",
        data.len()
    );
    let document = Document::open(pdf(&mesh)).expect("valid exact-xref PDF fixture");
    assert!(!document.was_recovered(), "the fixture xref is exact");
    assert!(!document.cross_reference_chain_was_cut());
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    let interpretation = pdf_model::interpret(&document, &page);
    assert!(
        interpretation.is_complete(),
        "all four vertices and the function are valid: {:?}",
        interpretation.unsupported
    );
    let target = TargetSpec::for_page(&interpretation.display_list, 1.0, GENEROUS)
        .expect("small page target");
    let raster = CpuRasterizer::new()
        .with_medium(pdf_render::Medium::NONE)
        .rasterize(&interpretation.display_list, target)
        .expect("supported Type 5 mesh");

    // All four centers are strictly inside triangles, away from the shared diagonal.
    // Parameter is x/100, then red = t². At x=25.5,50.5,75.5 the independent
    // rounded 8-bit values are17,65,145; color-first interpolation gives65,129,193.
    // The two y positions at x=50 exercise both triangles of the lattice cell.
    for (x, y, expected) in [
        (50_u32, 25_u32, 65_u8),
        (50, 75, 65),
        (25, 50, 17),
        (75, 50, 145),
    ] {
        let at = usize::try_from((y * raster.width + x) * 4).expect("bounded pixel index");
        let red = raster.data[at];
        let green = raster.data[at + 1];
        let blue = raster.data[at + 2];
        let alpha = raster.data[at + 3];
        assert_eq!(alpha, 255, "the lattice covers interior pixel {x},{y}");
        assert!(
            red.abs_diff(expected) <= 4,
            "interpolate t then t² at {x},{y}: red {red}, expected {expected}"
        );
        assert!(
            green < 4 && blue < 4,
            "red-only function at {x},{y}: {red},{green},{blue}"
        );
    }
}

//! Exact-xref public-pixel tests for the Type 6/7 patch branches that current tests only sample
//! partially: all four Figure 45 corners, and a non-linear /Function applied after interpolating t.
//! Source-derived corner and interpolation oracles are independent of the renderer.

#![expect(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "small exact PDF fixture and in-bounds pixels should fail loudly if malformed"
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

fn render(mesh: &str) -> pdf_render::Raster {
    let document = Document::open(pdf(mesh)).expect("complete exact-xref PDF");
    assert!(!document.was_recovered(), "the fixture xref is exact");
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    let interpretation = pdf_model::interpret(&document, &page);
    assert!(
        interpretation.is_complete(),
        "valid mesh fixture: {:?}",
        interpretation.unsupported
    );
    let target =
        TargetSpec::for_page(&interpretation.display_list, 1.0, GENEROUS).expect("small target");
    CpuRasterizer::new()
        .with_medium(pdf_render::Medium::NONE)
        .rasterize(&interpretation.display_list, target)
        .expect("supported mesh")
}

// Raster rows run from page top to bottom, matching the checked-in shadings.rs helper.
fn pixel(raster: &pdf_render::Raster, x: u32, y: u32) -> [u8; 4] {
    let at = usize::try_from((y * raster.width + x) * 4).expect("bounded pixel");
    [
        raster.data[at],
        raster.data[at + 1],
        raster.data[at + 2],
        raster.data[at + 3],
    ]
}

fn patch_mesh(kind: u8, function: &str, data: &str, decode: &str) -> String {
    format!(
        "<< /ShadingType {kind} /ColorSpace /DeviceRGB /BitsPerCoordinate 8 \
         /BitsPerComponent 8 /BitsPerFlag 8 /Decode {decode} {function} \
         /Filter /ASCIIHexDecode /Length {} >>\nstream\n{}\nendstream",
        data.len(),
        data
    )
}

const SQUARE_BOUNDARY: &str = "00 0A0A 1E0A 320A 460A 461E 4632 4646 3246 1E46 0A46 0A32 0A1E";

#[test]
fn type6_paints_each_figure45_corner_colour_at_its_named_corner() {
    // Figure 45 and its preceding prose order c1 at p00, c2 at p03, c3 at p33 and c4 at p30. Samples are close to each
    // corner of this straight-sided square; the expected hues come from the source ordering.
    let data = format!("{SQUARE_BOUNDARY} FF0000 00FF00 0000FF FFFF00 >");
    let raster = render(&patch_mesh(6, "", &data, "[0 100 0 100 0 1 0 1 0 1]"));

    let c1 = pixel(&raster, 5, 94); // page point (5.5, 5.5), near p00
    let c2 = pixel(&raster, 25, 94); // near p03
    let c3 = pixel(&raster, 25, 74); // near p33
    let c4 = pixel(&raster, 5, 74); // near p30
    assert!(
        c1[0] > 210 && c1[1] < 55 && c1[2] < 55,
        "c1=p00 red: {c1:?}"
    );
    assert!(
        c2[1] > 210 && c2[0] < 55 && c2[2] < 55,
        "c2=p03 green: {c2:?}"
    );
    assert!(
        c3[2] > 210 && c3[0] < 55 && c3[1] < 55,
        "c3=p33 blue: {c3:?}"
    );
    assert!(
        c4[0] > 210 && c4[1] > 210 && c4[2] < 55,
        "c4=p30 yellow: {c4:?}"
    );
}

fn type6_parameter_mesh() -> String {
    let data = format!("{SQUARE_BOUNDARY} 00 FF 00 FF >");
    patch_mesh(
        6,
        "/Function << /FunctionType 2 /Domain [0 1] /C0 [0 0 0] /C1 [1 0 0] /N 2 >>",
        &data,
        "[0 100 0 100 0 1]",
    )
}

fn type7_parameter_mesh() -> String {
    // Table 85 order: 12 boundary points, p11,p12,p22,p21, then c00,c03,c33,c30.
    // The interior controls complete a planar net; t values match the Type 6 fixture.
    let data = "00 \
        0A0A 1E0A 320A 460A 461E 4632 4646 3246 1E46 0A46 0A32 0A1E \
        1E1E 321E 3232 1E32 \
        00 FF 00 FF >";
    patch_mesh(
        7,
        "/Function << /FunctionType 2 /Domain [0 1] /C0 [0 0 0] /C1 [1 0 0] /N 2 >>",
        data,
        "[0 100 0 100 0 1]",
    )
}

fn assert_parameter_then_function(mesh: &str, kind: &str) {
    let raster = render(mesh);
    // The square center has bilinear t=0.5. Interpolate t, then apply t^2: 0.25*255 ~= 64.
    // Applying the function to corners and interpolating their colors would give about 128.
    let sample = pixel(&raster, 16, 83); // page point (16.5, 16.5)
    assert_eq!(sample[3], 255, "{kind} patch covers its center: {sample:?}");
    assert!(
        sample[0].abs_diff(64) <= 4 && sample[1] < 4 && sample[2] < 4,
        "{kind}: function follows parameter interpolation (about 64 red), not corner-color interpolation (128): {sample:?}"
    );
}

#[test]
fn type6_calls_function_with_interpolated_patch_parameter() {
    assert_parameter_then_function(&type6_parameter_mesh(), "Type 6");
}

#[test]
fn type7_calls_function_with_interpolated_patch_parameter() {
    assert_parameter_then_function(&type7_parameter_mesh(), "Type 7");
}

#[test]
fn type4_one_bit_samples_align_each_vertex_to_its_byte() {
    // Table 81 permits one-bit coordinates/components and a two-bit flag. Each record is 7
    // meaningful bits (flag, x, y, RGB) plus one ignored pad bit. These bytes are authored from
    // the source field order: 0x08 = flag0,(0,0),red; 0x24 = flag0,(1,0),green; 0x12 =
    // flag0,(0,1),blue. Without the per-vertex byte alignment, later vertices shift.
    let data = "08 24 12 >";
    let mesh = format!(
        "<< /ShadingType 4 /ColorSpace /DeviceRGB /BitsPerCoordinate 1 \
         /BitsPerComponent 1 /BitsPerFlag 2 /Decode [0 100 0 100 0 1 0 1 0 1] \
         /Filter /ASCIIHexDecode /Length {} >>\nstream\n{}\nendstream",
        data.len(),
        data
    );
    let raster = render(&mesh);
    let red = pixel(&raster, 5, 94);
    let green = pixel(&raster, 94, 94);
    let blue = pixel(&raster, 5, 5);
    assert!(
        red[0] > 210 && red[1] < 55 && red[2] < 55,
        "one-bit red: {red:?}"
    );
    assert!(
        green[1] > 210 && green[0] < 55 && green[2] < 55,
        "one-bit green: {green:?}"
    );
    assert!(
        blue[2] > 210 && blue[0] < 55 && blue[1] < 55,
        "one-bit blue: {blue:?}"
    );
}

//! Public raster coverage for a stream shading used through a Type 2 pattern.
//!
//! Exercises Table 75's stream alternative through a Type 2 pattern, not another dictionary
//! shading. The direct sh case is a positive control for the same valid Type 5 mesh stream.
#![expect(
    clippy::arithmetic_side_effects,
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "small exact-xref test fixture and in-bounds raster samples should fail loudly"
)]

use std::fmt::Write as _;

use pdf_render::{Raster, Rasterizer, TargetSpec};
use pdf_syntax::Document;
use render_cpu::CpuRasterizer;

const PIXEL_BUDGET: u64 = 1 << 20;

fn type5_stream_fixture(pattern_route: bool, bbox_left: u32, clip_right: u32) -> Vec<u8> {
    let content = if pattern_route {
        // The filled path covers the whole page. Parameters let tests isolate the stream BBox
        // from the caller's current clipping path.
        format!(
            "10 10 {} 80 re W n /Pattern cs /P0 scn 0 0 100 100 re f",
            clip_right.saturating_sub(10)
        )
    } else {
        format!("10 10 {} 80 re W n /Sh0 sh", clip_right.saturating_sub(10))
    };
    // Row-major vertices: red/green at y=20 and blue/yellow at y=80.
    // Coordinates decode to 20..80, matching the stream's common Table 77 BBox.
    let data = "00 00 FF 00 00  FF 00 00 FF 00  00 FF 00 00 FF  FF FF FF FF 00 >";
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] \
         /Resources << /Shading << /Sh0 5 0 R >> /Pattern << /P0 6 0 R >> >> \
         /Contents 4 0 R >>\nendobj\n\
         4 0 obj\n<< /Length {} >>\nstream\n{content}\nendstream\nendobj\n\
         5 0 obj\n<< /ShadingType 5 /ColorSpace /DeviceRGB /VerticesPerRow 2 \
         /BitsPerCoordinate 8 /BitsPerComponent 8 \
         /Decode [20 80 20 80 0 1 0 1 0 1] /BBox [{bbox_left} 20 80 80] \
         /Filter /ASCIIHexDecode /Length {} >>\nstream\n{data}\nendstream\nendobj\n\
         6 0 obj\n<< /Type /Pattern /PatternType 2 /Shading 5 0 R >>\nendobj\n",
        content.len().saturating_add(1),
        data.len()
    );

    // PDF 1.7 fixture with only ASCII stream bytes: no binary-header marker is needed.
    let mut pdf = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for object in body.split_inclusive("endobj\n") {
        offsets.push(pdf.len());
        pdf.push_str(object);
    }
    let xref_at = pdf.len();
    let size = offsets.len().saturating_add(1);
    let _ = writeln!(pdf, "xref\n0 {size}");
    pdf.push_str("0000000000 65535 f \n");
    for offset in offsets {
        let _ = writeln!(pdf, "{offset:010} 00000 n ");
    }
    let _ = write!(
        pdf,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n"
    );
    pdf.into_bytes()
}

fn raster_strict(bytes: Vec<u8>) -> Raster {
    let document = Document::open(bytes).expect("valid exact-xref PDF");
    assert!(!document.was_recovered(), "all object offsets are exact");
    assert!(
        !document.cross_reference_chain_was_cut(),
        "the xref chain is complete"
    );
    let page = pdf_model::Pages::new(&document).get(0).expect("one page");
    let interpretation = pdf_model::interpret(&document, &page);
    assert!(
        interpretation.is_complete(),
        "both shadings are fully interpreted: {:?}",
        interpretation.unsupported
    );
    let list = interpretation.display_list;
    let target = TargetSpec::for_page(&list, 1.0, PIXEL_BUDGET).expect("page fits");
    let raster = CpuRasterizer::new()
        .with_medium(pdf_render::Medium::NONE)
        .rasterize(&list, target)
        .expect("CPU rasterization succeeds");
    assert_eq!(
        (raster.width, raster.height),
        (100, 100),
        "strict sample bounds"
    );
    raster
}

fn user_pixel(raster: &Raster, x: u32, y: u32) -> [u8; 4] {
    let row = raster.height.saturating_sub(1).saturating_sub(y);
    let at = ((row.saturating_mul(raster.width)).saturating_add(x) as usize).saturating_mul(4);
    [
        raster.data[at],
        raster.data[at + 1],
        raster.data[at + 2],
        raster.data[at + 3],
    ]
}

fn assert_four_literal_lattice_corners(raster: &Raster) {
    // Table 82's four decoded corner colors: near each lattice vertex the corresponding
    // channel(s) remain dominant. These are literal color checks, independent of direct-sh.
    let red = user_pixel(raster, 22, 22);
    assert!(
        red[0] > 220 && red[1] < 30 && red[2] < 30,
        "red corner: {red:?}"
    );
    let green = user_pixel(raster, 78, 22);
    assert!(
        green[1] > 220 && green[0] < 30 && green[2] < 30,
        "green corner: {green:?}"
    );
    let blue = user_pixel(raster, 22, 78);
    assert!(
        blue[2] > 220 && blue[0] < 30 && blue[1] < 30,
        "blue corner: {blue:?}"
    );
    let yellow = user_pixel(raster, 78, 78);
    assert!(
        yellow[0] > 220 && yellow[1] > 220 && yellow[2] < 30,
        "yellow corner: {yellow:?}"
    );
    for corner in [red, green, blue, yellow] {
        assert_eq!(corner[3], 255, "the mesh paints its corners");
    }
}

fn assert_literal_lattice_interior(raster: &Raster) {
    // Pixel (45,45) samples user (45.5,45.5): weights .15 red, .425 green, .425 blue.
    // Allow integer raster rounding while checking all independently predicted channels.
    let interior = user_pixel(raster, 45, 45);
    assert!(
        (35..=41).contains(&interior[0])
            && (105..=111).contains(&interior[1])
            && (105..=111).contains(&interior[2])
            && interior[3] == 255,
        "interior literal is approximately (38,108,108): {interior:?}"
    );
}

#[test]
fn type2_pattern_accepts_a_type5_shading_stream_with_literal_color_control() {
    // Baseline geometry is 20..80; the caller clip is 10..90. The direct-sh route is a valid
    // positive control, but both routes also have independent literal color expectations.
    let pattern = raster_strict(type5_stream_fixture(true, 20, 90));
    let direct = raster_strict(type5_stream_fixture(false, 20, 90));
    assert_four_literal_lattice_corners(&pattern);
    assert_literal_lattice_interior(&pattern);
    assert_four_literal_lattice_corners(&direct);
    assert_literal_lattice_interior(&direct);
}

#[test]
fn type2_stream_shading_bbox_clips_mesh_geometry_inside_the_caller_clip() {
    let raster = raster_strict(type5_stream_fixture(true, 30, 90));

    // Red's mesh vertex at (20,20) is now outside BBox, while green at (80,20) survives.
    assert_eq!(
        user_pixel(&raster, 22, 22)[3],
        0,
        "BBox removes the red corner"
    );
    let green = user_pixel(&raster, 78, 22);
    assert!(
        green[1] > 220 && green[0] < 30 && green[2] < 30,
        "green remains: {green:?}"
    );
    assert_literal_lattice_interior(&raster);
}

#[test]
fn type2_stream_shading_respects_the_caller_clip_inside_its_bbox() {
    let raster = raster_strict(type5_stream_fixture(true, 20, 60));

    // The BBox still contains both vertices, but the current path clip ends at x=60.
    let red = user_pixel(&raster, 22, 22);
    assert!(
        red[0] > 220 && red[1] < 30 && red[2] < 30,
        "red remains: {red:?}"
    );
    assert_eq!(
        user_pixel(&raster, 78, 22)[3],
        0,
        "caller clip removes green corner"
    );
    assert_literal_lattice_interior(&raster);
}

#[test]
fn type2_stream_shading_composes_bbox_and_caller_clip_while_retaining_interior() {
    let raster = raster_strict(type5_stream_fixture(true, 30, 60));

    // The red corner is removed by BBox and the green corner by the caller clip; a point inside
    // both boundaries remains painted with its independently expected lattice interpolation.
    assert_eq!(user_pixel(&raster, 22, 22)[3], 0, "BBox removes red corner");
    assert_eq!(
        user_pixel(&raster, 78, 22)[3],
        0,
        "caller clip removes green corner"
    );
    assert_literal_lattice_interior(&raster);
}

//! Public shading-BBox assertion for Table 77 NOTE 2 / §10.7.4.
//! Distinct from `degenerate_tiling_bbox_clip_spec_audit`: that test covers a tiling-cell
//! `BBox`; this uses Table 77's shading `BBox` on a direct `sh` consumer.
#![expect(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "bounded exact-xref fixture and literal pixel assertions"
)]

use std::fmt::Write as _;

use pdf_render::{Rasterizer, TargetSpec};
use pdf_syntax::Document;
use render_cpu::CpuRasterizer;

const CONTENT: &str = "q /Sh0 sh Q";
const SHADING: &str = "<< /ShadingType 2 /ColorSpace /DeviceRGB /Coords [0 50 100 50] \
    /Extend [true true] /BBox BBOX \
    /Function << /FunctionType 2 /Domain [0 1] /C0 [1 0 0] /C1 [1 0 0] /N 1 >> >>";

fn fixture(bbox: &str) -> Vec<u8> {
    let shading = SHADING.replace("BBOX", bbox);
    let objects = [
        "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] \
          /Resources << /Shading << /Sh0 5 0 R >> >> /Contents 4 0 R >>"
            .to_owned(),
        format!(
            "<< /Length {} >>\nstream\n{CONTENT}\nendstream",
            CONTENT.len() + 1
        ),
        shading,
    ];
    let mut body = String::new();
    let mut offsets = Vec::new();
    for (index, object) in objects.iter().enumerate() {
        offsets.push(body.len());
        let _ = writeln!(body, "{} 0 obj\n{object}\nendobj", index + 1);
    }
    let xref = "%PDF-1.7\n".len() + body.len();
    let mut out = format!("%PDF-1.7\n{body}");
    let size = offsets.len() + 1;
    let _ = writeln!(out, "xref\n0 {size}");
    out.push_str("0000000000 65535 f \n");
    for offset in offsets {
        let _ = writeln!(out, "{:010} 00000 n ", "%PDF-1.7\n".len() + offset);
    }
    let _ = write!(
        out,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n"
    );
    out.into_bytes()
}

fn render(bbox: &str) -> pdf_render::Raster {
    let document = Document::open(fixture(bbox)).expect("valid exact-xref PDF");
    assert!(!document.was_recovered(), "the single xref is exact");
    let page = pdf_model::Pages::new(&document).get(0).expect("one page");
    let interpretation = pdf_model::interpret(&document, &page);
    assert!(
        interpretation.is_complete(),
        "the valid shading is fully interpreted: {:?}",
        interpretation.unsupported
    );
    let target = TargetSpec::for_page(&interpretation.display_list, 1.0, 1 << 20)
        .expect("small page target");
    CpuRasterizer::new()
        .with_medium(pdf_render::Medium::NONE)
        .rasterize(&interpretation.display_list, target)
        .expect("CPU rasterizer supports the shading")
}

fn painted_pixels(raster: &pdf_render::Raster) -> usize {
    raster
        .data
        .chunks_exact(4)
        .filter(|rgba| rgba[3] != 0)
        .count()
}

fn alpha(raster: &pdf_render::Raster, x: u32, y: u32) -> u8 {
    let i = usize::try_from((y * raster.width + x) * 4).expect("bounded pixel index");
    raster.data[i + 3]
}

#[test]
fn zero_area_shading_bbox_still_scan_converts_a_bounded_pixel_line() {
    // Positive control: a two-unit-wide ordinary clip intersects the same red shading.
    let ordinary = render("[29 20 31 80]");
    assert!(painted_pixels(&ordinary) > 0);
    let ordinary_axis =
        usize::try_from((50_u32 * ordinary.width + 30) * 4).expect("bounded pixel index");
    assert_eq!(
        ordinary.data[ordinary_axis..ordinary_axis + 4],
        [255, 0, 0, 255]
    );
    assert_eq!(alpha(&ordinary, 10, 10), 0, "the ordinary BBox clips");

    // Table 77 NOTE 2 and §10.7.4 require a zero-width/height rectangle to retain a one-pixel
    // line. Use fractional coordinates to exercise scan conversion away from an integer edge;
    // do not prescribe which adjacent pixel owns that half-open boundary.
    let boxes = ["[30.25 20 30.25 80]", "[20 30.25 80 30.25]"];
    let observations = boxes.map(|bbox| (bbox, render(bbox)));
    for (orientation, (bbox, raster)) in observations.into_iter().enumerate() {
        let painted = painted_pixels(&raster);
        assert!(
            painted > 0,
            "degenerate shading BBox {bbox} keeps a pixel line"
        );
        assert!(
            painted <= 256,
            "{bbox} remains a bounded one-pixel line, got {painted}"
        );
        assert_eq!(
            alpha(&raster, 10, 10),
            0,
            "{bbox} still clips distant paint"
        );
        // At scale 1 the vertical line is x=30.25; the horizontal line is raster y=69.75.
        // Allow boundary/AA ownership within two pixels, but require the interior length
        // and reject a wrong-location spot or any distant coloured paint.
        let width = usize::try_from(raster.width).expect("small page width");
        let cross_band = if orientation == 0 { 28..=32 } else { 67..=72 };
        for (index, rgba) in raster.data.chunks_exact(4).enumerate() {
            if rgba[3] == 0 {
                continue;
            }
            let (x, y) = (index % width, index / width);
            let (across, along) = if orientation == 0 { (x, y) } else { (y, x) };
            assert!(
                cross_band.contains(&across) && (18..=82).contains(&along),
                "{bbox}: paint at ({x},{y}) is outside the expected narrow segment"
            );
            assert!(
                rgba[0] > 0 && rgba[1] == 0 && rgba[2] == 0,
                "{bbox}: the retained line is red"
            );
        }
        for along in 25..75 {
            assert!(
                cross_band.clone().any(|across| {
                    let (x, y) = if orientation == 0 {
                        (across, along)
                    } else {
                        (along, across)
                    };
                    raster.data[(y * width + x) * 4 + 3] != 0
                }),
                "{bbox}: the line must cover interior coordinate {along}"
            );
        }
    }
}

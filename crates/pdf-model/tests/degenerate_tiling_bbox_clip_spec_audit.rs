//! Regression for ISO 32000-2 §8.7.3.1 Table 74 NOTE 1 and §10.7.4.
//! A degenerate pattern `BBox` must remain a clip; scan conversion preserves at least one pixel
//! around its boundary, but content far from that boundary must still be excluded.
#![expect(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects
)]

use std::fmt::Write as _;

use pdf_render::{Rasterizer, TargetSpec};
use pdf_syntax::Document;
use render_cpu::CpuRasterizer;

fn fixture(bbox: &str) -> Vec<u8> {
    let cell = "1 0 0 rg 0 0 10 10 re f";
    let pattern = format!(
        "<< /Type /Pattern /PatternType 1 /PaintType 1 /TilingType 2 /BBox {bbox} \
         /XStep 20 /YStep 20 /Resources << >> /Length {} >>\nstream\n{cell}\nendstream",
        cell.len()
    );
    let content = "/Pattern cs /P0 scn 0 0 100 100 re f";
    let objects = [
        b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n".to_vec(),
        b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n".to_vec(),
        b"3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Resources << /Pattern << /P0 5 0 R >> >> /Contents 4 0 R >>\nendobj\n".to_vec(),
        format!("4 0 obj\n<< /Length {} >>\nstream\n{content}\nendstream\nendobj\n", content.len()).into_bytes(),
        format!("5 0 obj\n{pattern}\nendobj\n").into_bytes(),
    ];
    let mut bytes = b"%PDF-1.7\n".to_vec();
    let mut offsets = vec![0usize];
    for object in objects {
        offsets.push(bytes.len());
        bytes.extend_from_slice(&object);
    }
    let xref = bytes.len();
    let mut trailer = String::new();
    let _ = writeln!(trailer, "xref\n0 {}", offsets.len());
    trailer.push_str("0000000000 65535 f \n");
    for offset in offsets.iter().skip(1) {
        let _ = writeln!(trailer, "{offset:010} 00000 n ");
    }
    let _ = write!(
        trailer,
        "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
        offsets.len()
    );
    bytes.extend_from_slice(trailer.as_bytes());
    bytes
}

fn render(bbox: &str) -> pdf_render::Raster {
    let document = Document::open(fixture(bbox)).expect("exact-xref PDF opens");
    assert!(
        !document.was_recovered(),
        "fixture uses computed xref offsets"
    );
    let page = pdf_model::Pages::new(&document).get(0).expect("one page");
    let interpretation = pdf_model::interpret(&document, &page);
    assert!(
        interpretation.is_complete(),
        "valid cell is supported: {:?}",
        interpretation.unsupported
    );
    let target =
        TargetSpec::for_page(&interpretation.display_list, 1.0, 1 << 20).expect("small target");
    CpuRasterizer::new()
        .with_medium(pdf_render::Medium::NONE)
        .rasterize(&interpretation.display_list, target)
        .expect("rasterizes")
}

fn pixel(raster: &pdf_render::Raster, x: u32, y: u32) -> [u8; 4] {
    let i = ((y * raster.width + x) * 4) as usize;
    [
        raster.data[i],
        raster.data[i + 1],
        raster.data[i + 2],
        raster.data[i + 3],
    ]
}

#[test]
fn a_zero_height_pattern_bbox_clips_distant_cell_marks_but_scan_converts_its_boundary() {
    // Positive control: the same red 10x10 cell and a normal BBox paint at y=2.
    let ordinary = render("[0 0 10 10]");
    assert_eq!(pixel(&ordinary, 4, 97), [255, 0, 0, 255]);

    // The zero-height BBox is a clip at PDF y=5. §10.7.4's scan-conversion note makes a
    // degenerate filled boundary paint at least one device pixel. Do not pin the exact row:
    // pixel ownership at the line follows the standard's half-open boundary convention.
    let degenerate = render("[0 5 10 5]");
    assert!(
        (92..=96).any(|row| pixel(&degenerate, 4, row)[3] != 0),
        "Table 74 NOTE 1: the degenerate box boundary still paints a pixel"
    );
    assert_eq!(
        pixel(&degenerate, 4, 97),
        [0, 0, 0, 0],
        "the cell's mark two units away is outside the zero-height BBox clip"
    );
}

//! Effective EC3 #23/#34: Table 166 /BM applies when a stored annotation AP is composited.
#![expect(
    clippy::indexing_slicing,
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    reason = "small complete exact-xref fixture and explicit pixel expectations"
)]

use std::io::Write as _;

use pdf_render::{Rasterizer, TargetSpec};
use pdf_syntax::Document;
use render_cpu::CpuRasterizer;

fn pdf(mode: Option<&str>) -> Vec<u8> {
    let annotation_mode = mode.map_or(String::new(), |value| format!("/BM /{value}"));
    let appearance = "1 0 0 rg 0 0 80 80 re f";
    let objects = [
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Resources << >> /Contents 4 0 R /Annots [5 0 R] >>".to_vec(),
        {
            let page_content = "0 0 1 rg 0 0 100 100 re f";
            format!("<< /Length {} >>\nstream\n{page_content}\nendstream", page_content.len()).into_bytes()
        },
        format!("<< /Type /Annot /Subtype /Square /Rect [10 10 90 90] /F 4 {annotation_mode} /AP << /N 6 0 R >> >>").into_bytes(),
        format!("<< /Type /XObject /Subtype /Form /BBox [0 0 80 80] /Resources << >> /Group << /S /Transparency /CS /DeviceRGB /I true /K false >> /Length {} >>\nstream\n{appearance}\nendstream", appearance.len() + 1).into_bytes(),
    ];

    let mut out = b"%PDF-2.0\n%\xE2\xE3\xCF\xD3\n".to_vec();
    let mut offsets = Vec::new();
    for (index, object) in objects.iter().enumerate() {
        offsets.push(out.len());
        out.extend_from_slice(format!("{} 0 obj\n", index + 1).as_bytes());
        out.extend_from_slice(object);
        out.extend_from_slice(b"\nendobj\n");
    }
    let xref = out.len();
    let size = objects.len() + 1;
    let _ = write!(out, "xref\n0 {size}\n0000000000 65535 f \n");
    for offset in offsets {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    let _ = write!(
        out,
        "trailer\n<< /Size {size} /Root 1 0 R /ID [<00112233445566778899AABBCCDDEEFF> <00112233445566778899AABBCCDDEEFF>] >>\nstartxref\n{xref}\n%%EOF\n"
    );
    out
}

fn pixel(mode: Option<&str>, x: u32, y: u32) -> (u8, u8, u8) {
    let document = Document::open(pdf(mode)).expect("generated PDF parses");
    assert!(!document.was_recovered(), "exact xref offsets");
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    let interpretation = pdf_model::interpret(&document, &page);
    assert!(
        interpretation.is_complete(),
        "appearance and group are supported"
    );
    let target =
        TargetSpec::for_page(&interpretation.display_list, 1.0, 1 << 30).expect("small page");
    let raster = CpuRasterizer::new()
        .with_medium(pdf_render::Medium::NONE)
        .rasterize(&interpretation.display_list, target)
        .expect("supported raster");
    let row = raster.height - 1 - y;
    let index = ((row * raster.width + x) as usize) * 4;
    assert_eq!(raster.data[index + 3], 255, "opaque page and appearance");
    (
        raster.data[index],
        raster.data[index + 1],
        raster.data[index + 2],
    )
}

#[test]
fn stored_annotation_bm_changes_group_composite_without_an_internal_bm() {
    // Independent source-over calculation: opaque red Multiply blue is black; Normal is red.
    assert_eq!(pixel(Some("Multiply"), 50, 50), (0, 0, 0));
    assert_eq!(pixel(Some("Normal"), 50, 50), (255, 0, 0));
    assert_eq!(
        pixel(None, 50, 50),
        (255, 0, 0),
        "Table 166 defaults the mode to Normal"
    );
    assert_eq!(
        pixel(Some("Multiply"), 5, 5),
        (0, 0, 255),
        "outside the annotation rectangle the page remains blue"
    );
}

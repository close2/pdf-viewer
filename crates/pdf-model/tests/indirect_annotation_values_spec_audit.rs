//! §7.3.10: valid indirect values in the public annotation rendering path.
#![expect(
    clippy::indexing_slicing,
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    reason = "small exact-xref fixture and literal raster coordinates"
)]

use std::io::Write as _;

use pdf_render::{Rasterizer, TargetSpec};
use pdf_syntax::Document;
use render_cpu::CpuRasterizer;

fn pdf(indirect: bool, flags: i64) -> Vec<u8> {
    let annotation = if indirect {
        "<< /Type /Annot /Subtype /Square /Rect 6 0 R /F 7 0 R /P 3 0 R /AP 8 0 R /AS 9 0 R >>"
            .to_owned()
    } else {
        format!(
            "<< /Type /Annot /Subtype /Square /Rect [20 20 60 60] /F {flags} /P 3 0 R \
             /AP << /N << /On 15 0 R >> >> /AS /On >>"
        )
    };
    let rect_values = [20, 20, 60, 60];
    let indirect_rect = "[10 0 R 11 0 R 12 0 R 13 0 R]".to_owned();
    let form = "1 0 0 rg 0 0 10 10 re f";
    let bodies = [
        "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Resources << >> \
         /Contents 4 0 R /Annots [5 0 R] >>"
            .to_owned(),
        {
            let page = "0 1 0 rg 0 0 100 100 re f";
            format!(
                "<< /Length {} >>\nstream\n{page}\nendstream",
                page.len().saturating_add(1)
            )
        },
        annotation,
        if indirect {
            indirect_rect
        } else {
            "<< >>".to_owned()
        },
        flags.to_string(),
        "<< /N 14 0 R >>".to_owned(),
        "/On".to_owned(),
        if indirect {
            rect_values[0].to_string()
        } else {
            "0".to_owned()
        },
        if indirect {
            rect_values[1].to_string()
        } else {
            "0".to_owned()
        },
        if indirect {
            rect_values[2].to_string()
        } else {
            "0".to_owned()
        },
        if indirect {
            rect_values[3].to_string()
        } else {
            "0".to_owned()
        },
        "<< /On 15 0 R >>".to_owned(),
        format!(
            "<< /Type /XObject /Subtype /Form /BBox [0 0 10 10] /Resources << >> \
             /Length {} >>\nstream\n{form}\nendstream",
            form.len().saturating_add(1)
        ),
    ];
    let mut out = b"%PDF-1.7\n%\xE2\xE3\xCF\xD3\n".to_vec();
    let mut offsets = Vec::new();
    for (index, body) in bodies.iter().enumerate() {
        offsets.push(out.len());
        let _ = write!(out, "{} 0 obj\n{body}\nendobj\n", index.saturating_add(1));
    }
    let xref = out.len();
    let size = bodies.len().saturating_add(1);
    let _ = write!(out, "xref\n0 {size}\n0000000000 65535 f \n");
    for offset in offsets {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    let _ = write!(
        out,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n"
    );
    out
}

fn pixel(indirect: bool, flags: i64, x: u32, y: u32) -> (u8, u8, u8, u8) {
    let document = Document::open(pdf(indirect, flags)).expect("valid PDF");
    assert!(!document.was_recovered(), "the explicit xref is exact");
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    let interpretation = pdf_model::interpret(&document, &page);
    assert!(interpretation.is_complete(), "valid appearance is readable");
    let target =
        TargetSpec::for_page(&interpretation.display_list, 1.0, 1 << 30).expect("small page");
    let raster = CpuRasterizer::new()
        .with_medium(pdf_render::Medium::NONE)
        .rasterize(&interpretation.display_list, target)
        .expect("supported raster");
    let index = (((raster.height - 1 - y) * raster.width + x) as usize) * 4;
    (
        raster.data[index],
        raster.data[index + 1],
        raster.data[index + 2],
        raster.data[index + 3],
    )
}

#[test]
fn indirect_rect_members_flags_and_appearance_state_reach_the_reader() {
    // Independent literal controls: red annotation over a green page, and the NoView flag's
    // source-defined behavior leaves the green page underneath it visible.
    assert_eq!(pixel(false, 4, 40, 40), (255, 0, 0, 255));
    assert_eq!(pixel(true, 4, 40, 40), (255, 0, 0, 255));
    assert_eq!(pixel(false, 32, 40, 40), (0, 255, 0, 255));
    assert_eq!(pixel(true, 32, 40, 40), (0, 255, 0, 255));
    assert_eq!(pixel(true, 4, 5, 40), (0, 255, 0, 255), "outside /Rect");
}

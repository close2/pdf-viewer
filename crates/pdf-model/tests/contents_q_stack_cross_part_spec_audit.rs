//! A balanced q/Q pair spans two page Contents parts, with a raster red control.
//! Source: ISO 32000-2 §7.7.3.3 Table 31 and §8.4.2 (physical 119–120 / 175;
//! printed 104–105 / 160).

#![expect(
    clippy::arithmetic_side_effects,
    clippy::expect_used,
    reason = "small exact-xref fixture and bounded raster coordinates"
)]

use std::fmt::Write as _;

use pdf_render::{Rasterizer, TargetSpec};
use pdf_syntax::Document;

fn pdf(pop_before_restore: bool) -> Vec<u8> {
    let first = "0 0 1 rg q 1 0 0 rg";
    let second = if pop_before_restore {
        "Q 10 10 20 20 re f"
    } else {
        "10 10 20 20 re f Q"
    };
    let stream = |contents: &str| {
        format!(
            "<< /Length {} >>\nstream\n{contents}\nendstream",
            contents.len()
        )
    };
    let bodies = [
        "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 50 40] /Resources << >> \
         /Contents [4 0 R 5 0 R] >>"
            .to_owned(),
        stream(first),
        stream(second),
    ];

    let mut out = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for (index, body) in bodies.iter().enumerate() {
        offsets.push(out.len());
        let _ = writeln!(out, "{} 0 obj\n{body}\nendobj", index + 1);
    }
    let xref = out.len();
    let _ = writeln!(out, "xref\n0 6\n0000000000 65535 f ");
    for offset in offsets {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    let _ = write!(
        out,
        "trailer\n<< /Size 6 /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n"
    );
    out.into_bytes()
}

fn center_pixel(pop_before_restore: bool) -> [u8; 4] {
    let document = Document::open(pdf(pop_before_restore)).expect("exact-xref PDF opens");
    assert!(!document.was_recovered(), "all xref offsets are exact");
    let page = pdf_model::Pages::new(&document).get(0).expect("one page");
    let interpretation = pdf_model::interpret(&document, &page);
    assert!(
        interpretation.is_complete(),
        "all operators are standard and supported: {:?}",
        interpretation.unsupported
    );
    let target = TargetSpec::for_page(&interpretation.display_list, 1.0, 1 << 20)
        .expect("small bounded page");
    let raster = render_cpu::CpuRasterizer::new()
        .rasterize(&interpretation.display_list, target)
        .expect("the rectangle renders");
    // The page raster is top-down, while this fixture's sample is in page coordinates.
    let (x, y) = (20_u32, 20_u32);
    let row = raster.height - 1 - y;
    let start = ((row * raster.width + x) * 4) as usize;
    [
        raster.data[start],
        raster.data[start + 1],
        raster.data[start + 2],
        raster.data[start + 3],
    ]
}

#[test]
fn q_push_in_one_contents_part_restores_state_on_pop_in_the_next_part() {
    // Explicit blue before q and red inside q avoid dependence on default fill color.
    // Part 1 ends at an operator-token boundary; Part 2 starts at another operator.
    // The Table 31 inserted whitespace separates them, and the whole page sequence is balanced.
    assert_eq!(center_pixel(true), [0, 0, 255, 255]);
}

#[test]
fn same_color_change_without_cross_part_restore_is_the_red_control() {
    // Same two streams and color changes. Painting before Q is intentionally still inside q.
    assert_eq!(center_pixel(false), [255, 0, 0, 255]);
}

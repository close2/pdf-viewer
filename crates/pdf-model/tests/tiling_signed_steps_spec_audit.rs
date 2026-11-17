//! Bounded §8.7.3.1 Table 74 control for valid signed XStep/YStep values.
//! The lattice with either step sign reversed names the same infinite set of cell origins;
//! Integer-aligned steps and `TilingType` 2 isolate sign handling in this backend. The source
//! permits up to one pixel of spacing variation; byte equality here is a regression control.
#![expect(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "small exact-xref fixture and bounded pixel coordinates"
)]

use std::fmt::Write as _;

use pdf_render::{Rasterizer, TargetSpec};
use pdf_syntax::Document;
use render_cpu::CpuRasterizer;

fn fixture(x_step: i32, y_step: i32) -> Vec<u8> {
    let cell = "1 0 0 rg 4 5 3 3 re f";
    let content = "/Pattern cs /P0 scn 0 0 100 100 re f";
    let pattern = format!(
        "<< /Type /Pattern /PatternType 1 /PaintType 1 /TilingType 2 \
         /BBox [2 3 12 13] /XStep {x_step} /YStep {y_step} /Resources << >> /Length {} >>\n\
         stream\n{cell}\nendstream",
        cell.len() + 1
    );
    let objects = [
        b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n".to_vec(),
        b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n".to_vec(),
        b"3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Resources << /Pattern << /P0 5 0 R >> >> /Contents 4 0 R >>\nendobj\n".to_vec(),
        format!(
            "4 0 obj\n<< /Length {} >>\nstream\n{content}\nendstream\nendobj\n",
            content.len()
        )
        .into_bytes(),
        format!("5 0 obj\n{pattern}\nendobj\n").into_bytes(),
    ];
    let mut bytes = b"%PDF-1.7\n".to_vec();
    let mut offsets = vec![0usize];
    for object in objects {
        offsets.push(bytes.len());
        bytes.extend_from_slice(&object);
    }
    let xref = bytes.len();
    let mut tail = String::new();
    let _ = writeln!(tail, "xref\n0 {}", offsets.len());
    tail.push_str("0000000000 65535 f \n");
    for offset in offsets.iter().skip(1) {
        let _ = writeln!(tail, "{offset:010} 00000 n ");
    }
    let _ = write!(
        tail,
        "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
        offsets.len()
    );
    bytes.extend_from_slice(tail.as_bytes());
    bytes
}

fn render(x_step: i32, y_step: i32) -> pdf_render::Raster {
    let document = Document::open(fixture(x_step, y_step)).expect("exact-xref PDF opens");
    assert!(
        !document.was_recovered(),
        "fixture has computed xref offsets"
    );
    assert!(
        !document.cross_reference_chain_was_cut(),
        "fixture xref chain is complete"
    );
    let page = pdf_model::Pages::new(&document).get(0).expect("one page");
    let interpretation = pdf_model::interpret(&document, &page);
    assert!(
        interpretation.is_complete(),
        "valid pattern is supported: {:?}",
        interpretation.unsupported
    );
    let target =
        TargetSpec::for_page(&interpretation.display_list, 1.0, 1 << 20).expect("small page");
    CpuRasterizer::new()
        .with_medium(pdf_render::Medium::NONE)
        .rasterize(&interpretation.display_list, target)
        .expect("supported raster")
}

fn pixel(raster: &pdf_render::Raster, x: u32, y: u32) -> [u8; 4] {
    let at = ((y * raster.width + x) * 4) as usize;
    [
        raster.data[at],
        raster.data[at + 1],
        raster.data[at + 2],
        raster.data[at + 3],
    ]
}

#[test]
fn nonzero_signed_tiling_steps_keep_the_same_infinite_lattice() {
    // Independent positive-step expectations: the cell's red mark lands at PDF x=4..7,
    // y=5..8 (raster y=92..95), while the gap remains clear.
    let positive = render(15, 17);
    assert_eq!(pixel(&positive, 5, 93), [255, 0, 0, 255]);
    assert_eq!(pixel(&positive, 10, 93)[3], 0);
    for (x_step, y_step) in [(-15, 17), (15, -17), (-15, -17)] {
        let signed = render(x_step, y_step);
        assert_eq!(
            signed.data, positive.data,
            "step signs {x_step}, {y_step} changed the infinite tile set"
        );
    }
}

//! Exact-xref integration check for noncommuting `cm` concatenation and q/Q restoration.
//!
//! Unlike a path with a graphics-state operator inserted between construction and painting,
//! this fixture follows Figure 9 (§8.2, physical163/printed148): q/cm are at content-stream level, each re/f pair is a whole
//! path object, and Q occurs after its painting operator. It uses a non-origin `CropBox`,
//! `Rotate` 90, and `UserUnit` 2 so the expected initial page transform is not identity.

#![expect(
    clippy::arithmetic_side_effects,
    reason = "small exact-xref fixture and bounded raster coordinates"
)]

use std::fmt::Write as _;

use pdf_render::{Command, Rasterizer, TargetSpec, Transform};
use pdf_syntax::Document;

fn pdf() -> Vec<u8> {
    let content = "q 2 0 0 3 5 7 cm 1 0 0 1 4 6 cm 10 10 20 20 re f Q 30 40 10 10 re f";
    let bodies = [
        "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] /CropBox [10 20 110 120] \
         /Rotate 90 /UserUnit 2 /Resources << >> /Contents 4 0 R >>"
            .to_owned(),
        format!(
            "<< /Length {} >>\nstream\n{content}\nendstream",
            content.len() + 1
        ),
    ];
    let mut out = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for (i, body) in bodies.iter().enumerate() {
        offsets.push(out.len());
        let _ = writeln!(out, "{} 0 obj\n{body}\nendobj", i + 1);
    }
    let xref = out.len();
    let _ = writeln!(out, "xref\n0 5\n0000000000 65535 f ");
    for offset in offsets {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    let _ = write!(
        out,
        "trailer\n<< /Size 5 /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n"
    );
    out.into_bytes()
}

#[test]
fn cm_matrices_compose_in_content_order_and_q_restores_the_page_initial_ctm() {
    let document = Document::open(pdf()).expect("valid exact-xref page");
    assert!(
        !document.was_recovered(),
        "the classic xref offsets are exact"
    );
    let page = pdf_model::Pages::new(&document).get(0).expect("one page");
    let interpretation = pdf_model::interpret(&document, &page);
    assert!(
        interpretation.is_complete(),
        "fixture has no unsupported operators: {:?}",
        interpretation.unsupported
    );
    let transforms: Vec<_> = interpretation
        .display_list
        .commands()
        .iter()
        .filter_map(|command| match command {
            Command::Fill { transform, .. } => Some(*transform),
            _ => None,
        })
        .collect();

    assert_eq!(transforms.len(), 2, "both path objects are painted");
    // Base page map: subtract CropBox origin (10,20), rotate clockwise, then scale UserUnit:
    // (x,y) -> (2y - 40, 220 - 2x), represented [0 -2 2 0 -40 220].
    let page_ctm = Transform::new(0.0, -2.0, 2.0, 0.0, -40.0, 220.0);
    assert_eq!(transforms[1], page_ctm, "Q restores the initial page CTM");

    // First matrix maps (x,y) to (2x+5, 3y+7). The following translation is applied
    // in the transformed coordinates, so (10,10) first becomes (14,16), then (33,55).
    // Applying the page map yields (70,154), independently fixing the full composition order.
    // Combined CTM is [0 -4 6 0 10 194]. Reversing the two `cm`s gives another answer.
    // §8.3.4 matrix multiplication and Table 56 cm concatenate in this order.
    let two_cms_then_page = Transform::new(0.0, -4.0, 6.0, 0.0, 10.0, 194.0);
    assert_eq!(transforms[0], two_cms_then_page);

    // Independent rendered-position controls: the first rectangle covers display-space
    // x=70..190, y=74..154; after Q, the second covers x=40..60, y=140..160.
    let target = TargetSpec::for_page(&interpretation.display_list, 1.0, 1 << 20)
        .expect("small rotated page");
    assert_eq!((target.width, target.height), (200, 200));
    let raster = render_cpu::CpuRasterizer::new()
        .rasterize(&interpretation.display_list, target)
        .expect("the composed paths render");
    let at_page_point = |x: u32, y: u32| {
        let row = raster.height - 1 - y;
        let at = ((row * raster.width + x) * 4) as usize;
        [
            raster.data[at],
            raster.data[at + 1],
            raster.data[at + 2],
            raster.data[at + 3],
        ]
    };
    assert_eq!(
        at_page_point(100, 140),
        [0, 0, 0, 255],
        "composed-transform interior"
    );
    assert_eq!(
        at_page_point(50, 150),
        [0, 0, 0, 255],
        "restored-CTM control interior"
    );
    assert_eq!(
        at_page_point(100, 50),
        [255, 255, 255, 255],
        "outside both paths"
    );
}

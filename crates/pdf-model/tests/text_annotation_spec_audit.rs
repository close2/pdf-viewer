//! Text subtype rotation and fixed-anchor consumer assertions.
//!
//! Complete one-page PDF fixtures with exact stream lengths and xref offsets. The first test
//! isolates the Text subtype's implicit `NoRotate` behavior with the same asymmetric stored form
//! used by the existing generic `NoRotate` test. The second checks that a synthesized icon is
//! attached to Rect's upper-left point and does not acquire a size from the rectangle's extent.

#![expect(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "a malformed fixture or invalid raster coordinate must fail loudly"
)]

use std::fmt::Write as _;

use pdf_render::{Rasterizer, TargetSpec};
use pdf_syntax::Document;
use render_cpu::CpuRasterizer;

fn assemble(body: &str) -> Vec<u8> {
    let mut out = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for object in body.split_inclusive("endobj\n") {
        offsets.push(out.len());
        out.push_str(object);
    }
    let xref = out.len();
    let size = offsets.len() + 1;
    writeln!(out, "xref\n0 {size}\n0000000000 65535 f ").expect("xref header");
    for offset in offsets {
        writeln!(out, "{offset:010} 00000 n ").expect("xref entry");
    }
    write!(
        out,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n"
    )
    .expect("trailer");
    out.into_bytes()
}

fn pdf(subtype: &str, flags: i64, rotate: u16, rect: &str, appearance: Option<&str>) -> Vec<u8> {
    let mut body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Rotate {rotate} \
         /Resources << >> /Contents 4 0 R /Annots [5 0 R] >>\nendobj\n\
         4 0 obj\n<< /Length 0 >>\nstream\n\nendstream\nendobj\n\
         5 0 obj\n<< /Type /Annot /Subtype /{subtype} /Rect {rect} /F {flags}"
    );
    if appearance.is_some() {
        body.push_str(" /AP << /N 6 0 R >>");
    }
    body.push_str(" >>\nendobj\n");
    if let Some(contents) = appearance {
        writeln!(
            body,
            "6 0 obj\n<< /Type /XObject /Subtype /Form /BBox [0 0 30 30] /Resources << >> /Length {} >>\nstream\n{contents}\nendstream\nendobj",
            contents.len() + 1
        )
        .expect("appearance object");
    }
    assemble(&body)
}

fn render(bytes: Vec<u8>) -> (pdf_render::Raster, bool) {
    let document = Document::open(bytes).expect("complete PDF fixture opens");
    assert!(!document.was_recovered(), "the stated xref is exact");
    let page = pdf_model::Pages::new(&document).get(0).expect("one page");
    let interpretation = pdf_model::interpret(&document, &page);
    assert!(
        interpretation.is_complete(),
        "fixture interpreted completely: {:?}",
        interpretation.unsupported
    );
    let list = interpretation.display_list;
    let target = TargetSpec::for_page(&list, 1.0, 1 << 20).expect("page-sized target");
    let raster = CpuRasterizer::new()
        .with_medium(pdf_render::Medium::NONE)
        .rasterize(&list, target)
        .expect("supported display list");
    (raster, interpretation.view_dependent)
}

fn extent(raster: &pdf_render::Raster) -> (u32, u32, u32, u32) {
    let (mut left, mut top, mut right, mut bottom) = (u32::MAX, u32::MAX, 0, 0);
    let mut found = false;
    for (index, pixel) in raster.data.chunks_exact(4).enumerate() {
        if pixel[3] > 0 {
            found = true;
            let x = u32::try_from(index % raster.width as usize).expect("x fits");
            let y = u32::try_from(index / raster.width as usize).expect("y fits");
            left = left.min(x);
            top = top.min(y);
            right = right.max(x);
            bottom = bottom.max(y);
        }
    }
    assert!(found, "the asymmetric appearance has visible pixels");
    (left, top, right, bottom)
}

fn painted(raster: &pdf_render::Raster, x: u32, y: u32) -> bool {
    let index = ((y * raster.width) + x) as usize * 4;
    raster.data[index + 3] > 0
}

#[test]
fn a_text_annotation_is_upright_when_page_rotation_changes() {
    // §12.5.6.4 requires Text annotations to behave as if NoRotate is always set. The left-half
    // form is asymmetric, so the page's /Rotate 90 visibly distinguishes an implicit NoRotate
    // from the same annotation with only /Print. /F 20 is the positive control with explicit
    // NoRotate; Square /F 4 is the rotate-with-page control.
    // On a 100-unit page rotated90, the fixed upper-left user point(40,70) maps
    // to device(70,40). The upright left half is15 wide and30 high from that anchor.
    let contents = "0 0 0 rg 0 0 15 30 re f";
    let (explicit, _) = render(pdf("Text", 20, 90, "[40 40 70 70]", Some(contents)));
    assert_eq!(
        extent(&explicit),
        (70, 40, 84, 69),
        "explicit NoRotate oracle"
    );

    let (square, _) = render(pdf("Square", 4, 90, "[40 40 70 70]", Some(contents)));
    assert!(painted(&square, 50, 50) && !painted(&square, 75, 45));
    assert!(
        square.data != explicit.data,
        "the rotation control distinguishes the paths"
    );

    // Regression assertion last: clearing Text's subtype override would rotate this mark.
    let (text_default, _) = render(pdf("Text", 4, 90, "[40 40 70 70]", Some(contents)));
    assert!(
        text_default.data == explicit.data,
        "a Text annotation's NoRotate does not depend on /F stating bit 5"
    );
}

#[test]
fn a_synthesized_text_icon_uses_rect_as_an_anchor_not_a_size() {
    // §12.5.6.4 says a Text annotation is attached to a point and held at a fixed screen size;
    // choosing one size for all synthesized icons is project policy, not a normative pixel rule.
    // These rectangles share upper-left corner
    // (50, 50) but have different extents, so the synthesized icon should render identically.
    let (point, _) = render(pdf("Text", 4, 0, "[50 50 50 50]", None));
    let (wide, _) = render(pdf("Text", 4, 0, "[50 30 90 50]", None));
    assert_ne!(point.data.iter().filter(|byte| **byte != 0).count(), 0);
    assert!(
        point.data == wide.data,
        "same point, same icon placement and size"
    );

    // A changed upper-left point is a positive control: the icon follows the file's location.
    let (moved, _) = render(pdf("Text", 4, 0, "[30 50 30 50]", None));
    assert!(
        point.data != moved.data,
        "moving the attached point moves the icon"
    );
}

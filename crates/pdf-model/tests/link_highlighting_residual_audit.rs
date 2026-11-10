//! Link-specific Table 176 `/H /O` and `/H /P` tests.
//! Existing annotation raster tests exercise these modes on Widgets only.
#![expect(
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    clippy::indexing_slicing,
    reason = "a malformed fixture or wrong pixel should fail this test"
)]

use std::fmt::Write as _;

use pdf_render::{Rasterizer, TargetSpec};
use pdf_syntax::{Document, ObjectId};
use render_cpu::CpuRasterizer;

fn assemble(body: &str) -> Vec<u8> {
    let mut out = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for object in body.split_inclusive("endobj\n") {
        offsets.push(out.len());
        out.push_str(object);
    }
    let xref_at = out.len();
    let size = offsets.len() + 1;
    writeln!(out, "xref\n0 {size}").expect("string write");
    out.push_str("0000000000 65535 f \n");
    for offset in offsets {
        writeln!(out, "{offset:010} 00000 n ").expect("string write");
    }
    writeln!(
        out,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF"
    )
    .expect("string write");
    out.into_bytes()
}

fn fixture(mode: &str, down: bool) -> Vec<u8> {
    let normal = "1 0 0 rg 0 0 80 80 re f";
    let down_stream = "0 0 1 rg 0 0 80 80 re f";
    let h_entry = if mode.is_empty() {
        String::new()
    } else {
        format!("/H {mode}")
    };
    let ap = if down {
        "/AP << /N 6 0 R /D 7 0 R >>"
    } else {
        "/AP << /N 6 0 R >>"
    };
    let down_object = if down {
        format!(
            "7 0 obj\n<< /Type /XObject /Subtype /Form /BBox [0 0 80 80] /Resources << >> \
             /Length {} >>\nstream\n{down_stream}\nendstream\nendobj\n",
            down_stream.len() + 1
        )
    } else {
        String::new()
    };
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Resources << >> \
         /Contents 4 0 R /Annots [5 0 R] >>\nendobj\n\
         4 0 obj\n<< /Length 0 >>\nstream\n\nendstream\nendobj\n\
         5 0 obj\n<< /Type /Annot /Subtype /Link /Rect [10 10 90 90] /F 4 {h_entry} \
         /Border [0 0 4] {ap} >>\nendobj\n\
         6 0 obj\n<< /Type /XObject /Subtype /Form /BBox [0 0 80 80] /Resources << >> \
         /Length {} >>\nstream\n{normal}\nendstream\nendobj\n{down_object}",
        normal.len() + 1
    );
    assemble(&body)
}

fn raster(mode: &str, down: bool, pressed: bool) -> pdf_render::Raster {
    let document = Document::open(fixture(mode, down)).expect("complete valid PDF fixture");
    assert!(!document.was_recovered(), "fixture xref is exact");
    let page = pdf_model::Pages::new(&document).get(0).expect("one page");
    let mut state = pdf_model::view::ViewState::of(&document);
    if pressed {
        state.set_pointer(Some((
            ObjectId {
                number: 5,
                generation: 0,
            },
            pdf_model::view::Pointer::Down,
        )));
    }
    let interpretation = pdf_model::content::interpret_with(&document, &page, &state);
    assert!(interpretation.is_complete(), "fixture is fully interpreted");
    let target =
        TargetSpec::for_page(&interpretation.display_list, 1.0, 1 << 20).expect("target fits");
    CpuRasterizer::new()
        .rasterize(&interpretation.display_list, target)
        .expect("CPU rasterizer supports fixture")
}

fn colour_at(raster: &pdf_render::Raster, x: u32, y: u32) -> (u8, u8, u8) {
    let row = raster.height - 1 - y;
    let index = ((row * raster.width + x) * 4) as usize;
    (
        raster.data[index],
        raster.data[index + 1],
        raster.data[index + 2],
    )
}

#[test]
fn link_outline_mode_inverts_the_border_not_contents() {
    let untouched = raster("/O", false, false);
    let pressed = raster("/O", false, true);
    assert_eq!(
        colour_at(&untouched, 50, 50),
        (255, 0, 0),
        "positive normal-appearance control"
    );
    assert_eq!(
        colour_at(&pressed, 50, 50),
        (255, 0, 0),
        "contents stay unchanged"
    );
    assert_eq!(
        colour_at(&pressed, 50, 12),
        (0, 255, 255),
        "Table 176 O inverts border"
    );
}

#[test]
fn link_push_mode_uses_the_down_appearance() {
    let normal = raster("/P", true, false);
    let pressed = raster("/P", true, true);
    assert_eq!(
        colour_at(&normal, 50, 50),
        (255, 0, 0),
        "normal appearance before press"
    );
    assert_eq!(
        colour_at(&pressed, 50, 50),
        (0, 0, 255),
        "P displays link as pushed"
    );
}

#[test]
fn an_unstated_link_mode_still_defaults_to_invert_when_a_down_appearance_exists() {
    // Table 176 gives the link entry a default of I. Unlike Table 191, it has no sentence
    // saying a non-P mode overrides /D; §12.5.5 therefore selects /D and /H I inverts it.
    let absent_mode_without_down = raster("", false, false);
    assert_eq!(colour_at(&absent_mode_without_down, 50, 50), (255, 0, 0));
    assert_eq!(
        colour_at(&raster("", false, true), 50, 50),
        (0, 255, 255),
        "absent /H with no /D uses default I"
    );
    let explicit_i_with_down_pressed = raster("/I", true, true);
    assert_eq!(
        colour_at(&explicit_i_with_down_pressed, 50, 50),
        (255, 255, 0),
        "explicit Link /H /I inverts the selected blue /D appearance"
    );
    let absent_mode_with_down_pressed = raster("", true, true);
    assert!(
        absent_mode_with_down_pressed.data == explicit_i_with_down_pressed.data,
        "absent /H defaults to I and matches explicit I over the same /D"
    );
    assert_eq!(
        colour_at(&absent_mode_with_down_pressed, 50, 50),
        (255, 255, 0),
        "the blue down appearance selected by §12.5.5 is inverted by Table 176's default I"
    );
}

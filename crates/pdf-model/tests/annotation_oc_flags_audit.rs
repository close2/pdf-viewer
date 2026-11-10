//! Standalone raster test for Table 166 `/OC` together with §12.5.3 flags.
//!
//! Each fixture is a complete one-page PDF 1.7 with
//! exact stream lengths and classic xref offsets. The appearance fills its nonempty Rect; a
//! hidden output must therefore have no alpha-bearing pixels rather than merely a missed sample.

#![expect(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "standalone test fixture; malformed syntax or pixel indexing should fail loudly"
)]

use std::fmt::Write as _;

use pdf_model::optional_content::Purpose;
use pdf_model::view::ViewState;
use pdf_render::{Rasterizer, TargetSpec};
use pdf_syntax::Document;
use render_cpu::CpuRasterizer;

const PIXEL_BUDGET: u64 = 1 << 20;
const APPEARANCE: &str = "1 0 0 rg 0 0 60 60 re f";

fn assemble(body: &str) -> Vec<u8> {
    let mut out = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for object in body.split_inclusive("endobj\n") {
        offsets.push(out.len());
        out.push_str(object);
    }
    let xref_at = out.len();
    let size = offsets.len().saturating_add(1);
    let _ = writeln!(out, "xref\n0 {size}");
    out.push_str("0000000000 65535 f \n");
    for offset in offsets {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    let _ = write!(
        out,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n"
    );
    out.into_bytes()
}

fn annotation_pdf(flags: i64, base_state: &str, oc_entry: &str, extra_object: &str) -> Vec<u8> {
    let empty_content = "";
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R \
         /OCProperties << /OCGs [5 0 R] /D << /BaseState /{base_state} >> >> >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] \
         /Resources << >> /Contents 4 0 R /Annots [6 0 R] >>\nendobj\n\
         4 0 obj\n<< /Length {} >>\nstream\n{empty_content}\nendstream\nendobj\n\
         5 0 obj\n<< /Type /OCG /Name (Test layer) >>\nendobj\n\
         6 0 obj\n<< /Type /Annot /Subtype /Square /Rect [20 20 80 80] /F {flags} \
         /OC {oc_entry} /AP << /N 7 0 R >> >>\nendobj\n\
         7 0 obj\n<< /Type /XObject /Subtype /Form /BBox [0 0 60 60] \
         /Resources << >> /Length {} >>\nstream\n{APPEARANCE}\nendstream\nendobj\n\
         {extra_object}",
        empty_content.len().saturating_add(1),
        APPEARANCE.len().saturating_add(1),
    );
    assemble(&body)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Extent {
    left: u32,
    bottom: u32,
    right: u32,
    top: u32,
}

fn render_extent(bytes: Vec<u8>, purpose: Purpose) -> Option<Extent> {
    let document = Document::open(bytes).expect("the fixture is a valid PDF");
    assert!(!document.was_recovered(), "exact xref fixture");
    let page = pdf_model::Pages::new(&document).get(0).expect("one page");
    let mut state = ViewState::of(&document);
    state.set_purpose(purpose);
    let interpretation = pdf_model::content::interpret_with(&document, &page, &state);
    assert!(
        interpretation.unsupported.is_empty(),
        "the fixture has ordinary optional content and a stored Square appearance: {:?}",
        interpretation.unsupported
    );
    let list = interpretation.display_list;
    let target = TargetSpec::for_page(&list, 1.0, PIXEL_BUDGET).expect("valid target");
    let raster = CpuRasterizer::new()
        .with_medium(pdf_render::Medium::NONE)
        .rasterize(&list, target)
        .expect("supported display list");

    let (mut left, mut bottom, mut right, mut top) = (u32::MAX, u32::MAX, 0, 0);
    let mut drew = false;
    for row in 0..raster.height {
        let y = raster.height.saturating_sub(1).saturating_sub(row);
        for x in 0..raster.width {
            let offset =
                ((row.saturating_mul(raster.width)).saturating_add(x) as usize).saturating_mul(4);
            if raster.data[offset + 3] > 0 {
                drew = true;
                left = left.min(x);
                bottom = bottom.min(y);
                right = right.max(x);
                top = top.max(y);
            }
        }
    }
    drew.then_some(Extent {
        left,
        bottom,
        right,
        top,
    })
}

#[test]
fn annotation_oc_and_flags_both_gate_visibility_for_screen_and_print() {
    // /F bit 2 is Hidden; bit 3 is Print; bit 6 is NoView. All use the same OCG and nonempty
    // stored AP, so differences below come only from purpose, OCG state, or the specified flags.
    let group = "5 0 R";
    let cases = [
        // purpose, flag bits, OCG BaseState, expected ink
        (Purpose::View, 0, "ON", true),
        (Purpose::View, 4, "ON", true), // Print alone does not turn screen display off.
        (Purpose::View, 2, "ON", false), // Hidden suppresses even when the OCG is on.
        (Purpose::View, 4, "OFF", false), // A suitable screen flag cannot override /OC.
        (Purpose::Print, 4, "ON", true),
        (Purpose::Print, 0, "ON", false), // Visible OCG does not override the clear Print bit.
        (Purpose::Print, 6, "ON", false), // Hidden wins even when Print and /OC are both on.
        (Purpose::Print, 4, "OFF", false),
        (Purpose::View, 36, "ON", false), // NoView suppresses the screen despite Print.
        (Purpose::Print, 36, "ON", true), // NoView does not suppress a permitted print.
    ];
    for (purpose, flags, base_state, expected) in cases {
        let actual = render_extent(annotation_pdf(flags, base_state, group, ""), purpose);
        assert_eq!(
            actual.is_some(),
            expected,
            "{purpose:?}, /F {flags}, OCG {base_state}: {actual:?}"
        );
        if let Some(extent) = actual {
            assert_eq!(
                extent,
                Extent {
                    left: 20,
                    bottom: 20,
                    right: 79,
                    top: 79
                }
            );
        }
    }
}

#[test]
fn an_annotation_oc_membership_dictionary_uses_its_policy_with_print_flags() {
    // AllOff is the positive case: the annotation appears only while every named group is off.
    // The same PDF and F=Print control differ only in the configuration's OCG state.
    let membership = "8 0 R";
    let extra = "8 0 obj\n<< /Type /OCMD /OCGs [5 0 R] /P /AllOff >>\nendobj\n";
    let off = render_extent(annotation_pdf(4, "OFF", membership, extra), Purpose::Print);
    let on = render_extent(annotation_pdf(4, "ON", membership, extra), Purpose::Print);

    assert_eq!(
        off,
        Some(Extent {
            left: 20,
            bottom: 20,
            right: 79,
            top: 79
        })
    );
    assert_eq!(on, None);
}

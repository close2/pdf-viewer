//! Standalone public-raster checks for ISO 32000-2 §10.5's halftone override.
//!
//! Source anchors: §10.5's second bullet says a halftone `TransferFunction` overrides the
//! corresponding graphics-state transfer; Tables 128–131 repeat the rule. Table 129 permits a
//! `TransferFunction` in a Type 6 halftone stream. §10.6.1 allows the continuous-tone raster device
//! to ignore the threshold screen; that permission does not remove §10.5's transfer mapping.
//! EC3 #310 clarifies the Type 5 component-presence producer constraint; this draft uses only
//! well-formed values and asserts reader output, not malformed-input rejection.
//!
//! These are valid exact-xref PDFs. Expected values come from constant type-2 functions with
//! independently stated C0/C1 values. The no-override arms keep the same image/color/content
//! route and show that /TR remains active where a halftone component says nothing.
#![expect(
    clippy::arithmetic_side_effects,
    clippy::expect_used,
    reason = "controlled exact-xref PDF fixtures; failures should be explicit"
)]

use std::fmt::Write as _;

use pdf_render::{Rasterizer, TargetSpec};
use pdf_syntax::Document;
use render_cpu::CpuRasterizer;

const PAGE_CONTENT: &str = "/G gs 1 1 1 rg 0 0 10 10 re f";
const GENEROUS: u64 = 1 << 20;

fn function(number: u32, value: &str) -> String {
    format!(
        "{number} 0 obj\n<< /FunctionType 2 /Domain [0 1] /C0 [{value}] \
         /C1 [{value}] /N 1 >>\nendobj\n"
    )
}

fn fixture(
    transfer_entries: &str,
    halftone_entry: &str,
    halftone_object: &str,
    other_objects: &str,
) -> Vec<u8> {
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] \
         /Resources << /ExtGState << /G << {transfer_entries} /HT {halftone_entry} >> >> >> \
         /Contents 4 0 R >>\nendobj\n\
         4 0 obj\n<< /Length {} >>\nstream\n{}\nendstream\nendobj\n\
         {halftone_object}{other_objects}",
        PAGE_CONTENT.len().saturating_add(1),
        PAGE_CONTENT,
    );
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

fn center_rgb(bytes: Vec<u8>) -> [u8; 3] {
    let document = Document::open(bytes).expect("valid PDF");
    assert!(!document.was_recovered(), "fixture has exact xref offsets");
    let page = pdf_model::Pages::new(&document).get(0).expect("one page");
    let state = pdf_model::view::ViewState::of(&document);
    let interpretation = pdf_model::content::interpret_with(&document, &page, &state);
    assert!(
        interpretation.is_complete(),
        "supported transfer fixture: {:?}",
        interpretation.unsupported
    );
    let list = interpretation.display_list;
    let target = TargetSpec::for_page(&list, 1.0, GENEROUS).expect("target fits");
    let raster = CpuRasterizer::new()
        .rasterize(&list, target)
        .expect("supported public CPU rasterization");
    let at = ((5u32.saturating_mul(raster.width)).saturating_add(5) as usize).saturating_mul(4);
    [raster.data[at], raster.data[at + 1], raster.data[at + 2]]
}

fn type_1_component(function_ref: Option<u32>) -> String {
    let transfer = function_ref.map_or_else(String::new, |n| format!(" /TransferFunction {n} 0 R"));
    format!(
        "<< /Type /Halftone /HalftoneType 1 /Frequency 60 /Angle 45 \
         /SpotFunction /Round{transfer} >>"
    )
}

#[test]
fn type_5_halftone_transfer_overrides_are_applied_per_rgb_component_in_the_public_raster() {
    let red = type_1_component(Some(7));
    let default = type_1_component(Some(8));
    let halftone = format!(
        "5 0 obj\n<< /Type /Halftone /HalftoneType 5 /Red {red} /Default {default} >>\nendobj\n"
    );
    let functions = format!(
        "{}{}{}",
        function(6, "0.125"),
        function(7, "0.5"),
        function(8, "0.25")
    );
    assert_eq!(
        center_rgb(fixture("/TR 6 0 R", "5 0 R", &halftone, &functions)),
        [128, 64, 64],
        "Type 5 /Red overrides red; /Default supplies green and blue"
    );

    // The same valid Type 5 screen without TransferFunction entries leaves /TR in force.
    let silent = format!(
        "5 0 obj\n<< /Type /Halftone /HalftoneType 5 /Red {} /Default {} >>\nendobj\n",
        type_1_component(None),
        type_1_component(None),
    );
    assert_eq!(
        center_rgb(fixture("/TR 6 0 R", "5 0 R", &silent, &functions)),
        [32, 32, 32],
        "a component with no halftone transfer inherits the graphics-state /TR"
    );
}

#[test]
fn type_6_stream_transfer_override_is_applied_even_when_its_screen_is_ignored() {
    let type_6 = |transfer: bool| {
        let entry = if transfer {
            " /TransferFunction 7 0 R"
        } else {
            ""
        };
        format!(
            "5 0 obj\n<< /Type /Halftone /HalftoneType 6 /Width 1 /Height 1 \
             /Filter /ASCIIHexDecode /Length 3{entry} >>\nstream\n00>\nendstream\nendobj\n"
        )
    };
    let functions = format!("{}{}", function(6, "0.125"), function(7, "0.5"));
    assert_eq!(
        center_rgb(fixture("/TR 6 0 R", "5 0 R", &type_6(true), &functions)),
        [128, 128, 128],
        "the stream dictionary's Table 129 TransferFunction overrides /TR"
    );
    assert_eq!(
        center_rgb(fixture("/TR 6 0 R", "5 0 R", &type_6(false), &functions)),
        [32, 32, 32],
        "the same valid threshold stream without an override leaves /TR active"
    );
}

#[test]
fn a_four_function_array_uses_the_first_three_for_rgb_not_the_gray_function() {
    // Type 1 is a valid device halftone but states no TransferFunction, so it does not mask /TR.
    let halftone = "5 0 obj\n<< /Type /Halftone /HalftoneType 1 /Frequency 60 /Angle 45 \
         /SpotFunction /Round >>\nendobj\n"
        .to_owned();
    let functions = format!(
        "{}{}{}{}",
        function(6, "0.125"),
        function(7, "0.5"),
        function(8, "0.75"),
        function(9, "0.25")
    );
    assert_eq!(
        center_rgb(fixture(
            "/TR [6 0 R 7 0 R 8 0 R 9 0 R]",
            "5 0 R",
            &halftone,
            &functions,
        )),
        [32, 128, 191],
        "RGB uses red, green and blue maps; the fourth gray map is not used"
    );
}

#[test]
fn tr2_takes_precedence_over_tr_when_both_graphics_state_entries_are_present() {
    let halftone = "5 0 obj\n<< /Type /Halftone /HalftoneType 1 /Frequency 60 /Angle 45 \
         /SpotFunction /Round >>\nendobj\n"
        .to_owned();
    let functions = format!("{}{}", function(6, "0.125"), function(7, "0.75"));
    assert_eq!(
        center_rgb(fixture(
            "/TR 6 0 R /TR2 7 0 R",
            "5 0 R",
            &halftone,
            &functions,
        )),
        [191, 191, 191],
        "Table 57 specifies that TR2 wins over TR"
    );
    assert_eq!(
        center_rgb(fixture("/TR 6 0 R", "5 0 R", &halftone, &functions)),
        [32, 32, 32],
        "without TR2, the same mark takes TR"
    );
}

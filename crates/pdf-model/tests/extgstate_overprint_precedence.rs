//! Table 57's `/OP` and `/op` precedence, ISO 32000-2 §8.4.5 and §8.6.7.
//!
//! A dictionary that names `/OP` sets the stroking and nonstroking parameters together unless
//! that same dictionary also names `/op`; in that case `/OP` sets stroking only. An absent `/op`
//! inherits `/OP`. These three outcomes are distinguished by drawing on a `DeviceCMYK` page group,
//! where §11.7.4.3 gives overprinting a visible effect.

#![expect(
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    reason = "test fixtures must fail loudly, and pixel offsets are bounded by the fixed target"
)]

use std::fmt::Write as _;

use pdf_render::{Rasterizer, TargetSpec};
use pdf_syntax::Document;

fn cmyk_page(ext_gstate: &str, content: &str) -> Vec<u8> {
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 40 40] \
         /Group << /S /Transparency /CS /DeviceCMYK >> \
         /Resources << /ExtGState << /GS << {ext_gstate} >> /Second << /ca 0.5 >> >> >> /Contents 4 0 R >>\nendobj\n\
         4 0 obj\n<< /Length {} >>\nstream\n{content}\nendstream\nendobj\n",
        content.len() + 1
    );
    let mut out = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for object in body.split_inclusive("endobj\n") {
        offsets.push(out.len());
        out.push_str(object);
    }
    let xref_at = out.len();
    let size = offsets.len() + 1;
    let _ = writeln!(out, "xref\n0 {size}");
    out.push_str("0000000000 65535 f \n");
    for offset in &offsets {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    let _ = write!(
        out,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n"
    );
    out.into_bytes()
}

fn centre(ext_gstate: &str, content: &str) -> (u8, u8, u8) {
    let document = Document::open(cmyk_page(ext_gstate, content)).expect("valid fixture");
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    let list = &pdf_model::interpret(&document, &page).display_list;
    let target = TargetSpec::for_page(list, 1.0, 1 << 20).expect("40 by 40 target");
    let raster = render_cpu::CpuRasterizer::new()
        .rasterize(list, target)
        .expect("fixture rasterises");
    let at = ((20 * raster.width) + 20) as usize * 4;
    (raster.data[at], raster.data[at + 1], raster.data[at + 2])
}

fn paint_pair(setup: &str, operator: &str) -> String {
    format!("0 0 0 1 k 0 0 40 40 re f {setup} 0.9 0 0 0 k 0 0 40 40 re {operator}")
}

/// Table 57: absent `/op` inherits `/OP`; an explicit `/op` overrides it only for fills.
#[test]
fn op_sets_both_unless_op_is_explicit_and_then_only_for_stroking() {
    let overprinting = centre("/OP true /OPM 1", &paint_pair("/GS gs", "f"));
    let explicitly_disabled_fill = centre("/OP true /op false /OPM 1", &paint_pair("/GS gs", "f"));
    let explicit_fill_value = centre("/OP false /op true /OPM 1", &paint_pair("/GS gs", "f"));
    let erasing = centre("", &paint_pair("", "f"));

    assert_ne!(
        overprinting, erasing,
        "the DeviceCMYK group exposes overprinting"
    );
    assert_eq!(
        explicitly_disabled_fill, erasing,
        "`/op false` overrides `/OP true` for fill"
    );
    assert_eq!(
        explicit_fill_value, overprinting,
        "`/op true` controls fill when present"
    );

    let stroke = |entries: &str| {
        centre(
            entries,
            "0 0 0 1 K 4 w 0 20 m 40 20 l S /GS gs 0.9 0 0 0 K 4 w 0 20 m 40 20 l S",
        )
    };
    let overprinting_stroke = stroke("/OP true /op false /OPM 1");
    let erasing_stroke = stroke("/OP false /op false /OPM 1");
    assert_eq!(
        overprinting_stroke,
        centre("", "0 0 0 1 k 0 0 40 40 re f 0.9 0 0 1 k 0 0 40 40 re f"),
        "stroke OP=true preserves the zero CMYK components, as an explicit source colour does"
    );
    assert_eq!(
        erasing_stroke,
        centre("", "0 0 0 1 k 0 0 40 40 re f 0.9 0 0 0 k 0 0 40 40 re f"),
        "stroke OP=false replaces the zero source components with zeros"
    );
    assert_eq!(
        stroke("/OP true /op false /OPM 1"),
        stroke("/OP true /OPM 1"),
        "an explicit nonstroking value does not turn stroking overprint off"
    );
}

/// A later partial `ExtGState` changes only its stated alpha key and preserves prior overprint.
#[test]
fn an_omitted_key_in_a_later_ext_gstate_preserves_the_current_value() {
    let after_partial_state = centre("/OP true /OPM 1", &paint_pair("/GS gs /Second gs", "f"));
    let explicit_fill_overprint = centre(
        "/OP true /op true /OPM 1 /ca 0.5",
        &paint_pair("/GS gs", "f"),
    );
    let explicit_fill_no_overprint = centre(
        "/OP false /op false /OPM 1 /ca 0.5",
        &paint_pair("/GS gs", "f"),
    );

    assert_eq!(
        after_partial_state, explicit_fill_overprint,
        "a later dictionary that states only ca does not reset op"
    );
    assert_ne!(
        after_partial_state, explicit_fill_no_overprint,
        "the overprint difference remains visible after the partial state"
    );
}

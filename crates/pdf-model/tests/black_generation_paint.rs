//! Stateful §11.7.5.3 solid-source conversion regressions.

#![expect(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "generated 100-unit fixture, bounded pixel indices and byte conversion; a malformed fixture must stop the instrument"
)]

use pdf_render::{Rasterizer, TargetSpec};
use pdf_syntax::Document;
use std::fmt::Write as _;
/// The unused nested form keeps the fixture resource structure fixed.
const NESTED: &str = "";
/// Interprets the generated one-page witness.
fn interpret(bytes: Vec<u8>) -> pdf_model::Interpretation {
    let document = Document::open(bytes).expect("the fixture is a valid PDF");
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    pdf_model::interpret(&document, &page)
}
/// Reads the opaque left-half sample from the CPU result.
fn pixel(interpretation: &pdf_model::Interpretation, x: u32, y: u32) -> [u8; 4] {
    let list = &interpretation.display_list;
    let target = TargetSpec::for_page(list, 1.0, 1 << 20).expect("a 100x100 target");
    let raster = render_cpu::CpuRasterizer::new()
        .rasterize(list, target)
        .expect("the fixture rasterises");
    let at = ((y * raster.width + x) * 4) as usize;
    let bytes = &raster.data[at..at + 4];
    [bytes[0], bytes[1], bytes[2], bytes[3]]
}
/// The page and nested forms have explicit blending spaces; the right half forces compositing.
fn page_group_fixture(
    page_group: &str,
    form_group: &str,
    form_resources: &str,
    form: &str,
    page: &str,
) -> Vec<u8> {
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] {page_group} \
         /Resources << /ExtGState << /GS << /ca 0.5 /CA 0.5 >> \
         /GK << /BG2 << /FunctionType 2 /Domain [0 1] /C0 [0] /C1 [1] /N 1 >> >> \
         /GR << /BG2 /Default /UCR2 /Default >> /GD << /BG2 /Default /BG << /FunctionType 2 /Domain [0 1] /C0 [0] /C1 [1] /N 1 >> >> \
         /GB << /BG << /FunctionType 2 /Domain [0 1] /C0 [1] /C1 [0] /N 1 >> \
         /UCR << /FunctionType 2 /Domain [0 1] /C0 [0] /C1 [0.5] /N 1 >> >> >> \
         /XObject << /Fm 5 0 R >> >> /Contents 4 0 R >>\nendobj\n\
         4 0 obj\n<< /Length {} >>\nstream\n{page}\nendstream\nendobj\n\
         5 0 obj\n<< /Type /XObject /Subtype /Form /BBox [0 0 100 100] {form_group} \
         /Resources << /ExtGState << /GS << /ca 0.5 /CA 0.5 >> >> \
         {form_resources} /XObject << /In 6 0 R >> >> /Length {} >>\n\
         stream\n{form}\nendstream\nendobj\n\
         6 0 obj\n<< /Type /XObject /Subtype /Form /BBox [0 0 100 100] \
         /Group << /S /Transparency /CS /DeviceCMYK >> \
         /Resources << /ExtGState << /GS << /ca 0.5 >> >> >> /Length {} >>\n\
         stream\n{NESTED}\nendstream\nendobj\n",
        page.len() + 1,
        form.len() + 1,
        NESTED.len() + 1
    );

    let cell = "0 0 100 100 re f";
    let body = format!(
        "{body}7 0 obj\n<< /Type /Pattern /PatternType 1 /PaintType 2 /TilingType 1 /BBox [0 0 100 100] /XStep 100 /YStep 100 /Resources << >> /Length {} >>\nstream\n{cell}\nendstream\nendobj\n",
        cell.len() + 1
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

/// §11.7.5.3 uses the functions at paint, independent of colour-selection order.
#[test]
fn solid_selection_order_matches_image_and_clause_arithmetic() {
    for mark in [
        "/GB gs 0.2 0.4 0.6 rg 0 0 50 100 re f",
        "0.2 0.4 0.6 rg /GB gs 0 0 50 100 re f",
        "/DeviceRGB cs 0.2 0.4 0.6 scn /GB gs 0 0 50 100 re f",
        "0.2 0.4 0.6 RG /GB gs 50 w 25 0 m 25 100 l S",
        "/DeviceRGB CS 0.2 0.4 0.6 SCN /GB gs 50 w 25 0 m 25 100 l S",
        "0.2 0.4 0.6 rg /GB gs q 50 0 0 100 0 0 cm BI /W 1 /H 1 /IM true /BPC 1 /F /AHx ID 00> EI Q",
        "/GB gs q 50 0 0 100 0 0 cm BI /W 1 /H 1 /BPC 8 /CS /RGB /F /AHx ID 336699> EI Q",
    ] {
        let drawn = paint(mark);
        // k=.4, BG=.6, UCR=.2 -> CMYK(.6,.4,.2,.6), through the device press.
        assert_eq!(pixel(&drawn, 25, 50), [52, 62, 79, 255], "{mark}");
    }
}

/// Keeps a real transparency operation away from the sampled half of the page.
fn paint(mark: &str) -> pdf_model::Interpretation {
    interpret(page_group_fixture(
        "/Group << /S /Transparency /CS /DeviceCMYK >>",
        "/Group << /S /Transparency >>",
        "",
        "",
        &format!("{mark} /GS gs 0 0 0 0 k 60 0 40 100 re f"),
    ))
}

/// q/Q restores source and functions together; explicit Default restores page defaults.
#[test]
fn changes_and_restores_do_not_keep_a_converted_colour() {
    for (prefix, current) in [
        ("0.2 0.4 0.6 rg /GB gs", "/GB gs"),
        ("/GB gs 0.2 0.4 0.6 rg /GR gs", ""),
        ("0.2 0.4 0.6 rg q /GB gs 0 0 10 10 re f Q", ""),
        (
            "/GB gs 0.2 0.4 0.6 rg q /GR gs 1 0 0 rg 0 0 10 10 re f Q",
            "/GB gs",
        ),
        ("0.2 0.4 0.6 rg /GB gs /GK gs", "/GB gs /GK gs"),
        ("/GB gs 0.2 0.4 0.6 rg /GD gs", "/GB gs /GD gs"),
    ] {
        let changed = paint(&format!("{prefix} 0 0 50 100 re f"));
        let selected = paint(&format!("{current} 0.2 0.4 0.6 rg 0 0 50 100 re f"));
        assert_eq!(
            pixel(&changed, 25, 50),
            pixel(&selected, 25, 50),
            "{prefix}"
        );
    }
}

/// A Form inherits source colour, but conversion belongs to its own group at paint.
#[test]
fn inherited_rgb_enters_the_forms_current_group() {
    for group in ["", "/Group << /S /Transparency /CS /DeviceCMYK >>"] {
        let render = |selection: &str, form: &str| {
            interpret(page_group_fixture(
                "",
                group,
                "",
                form,
                &format!("/GB gs {selection} /Fm Do"),
            ))
        };
        let inherited = render(
            "0.2 0.4 0.6 rg",
            "0 0 50 100 re f /GS gs 0 0 0 0 k 60 0 40 100 re f",
        );
        let explicit = render(
            "",
            "0.2 0.4 0.6 rg 0 0 50 100 re f /GS gs 0 0 0 0 k 60 0 40 100 re f",
        );
        assert_eq!(
            pixel(&inherited, 25, 50),
            pixel(&explicit, 25, 50),
            "{group}"
        );
    }
}

/// Table 74's uncoloured cell uses its supplied tint, not the Form's inherited RGB source.
#[test]
fn uncoloured_pattern_discards_inherited_rgb_source() {
    let render = |outer: &str| {
        interpret(page_group_fixture(
            "/Group << /S /Transparency /CS /DeviceCMYK >>",
            "",
            "/ColorSpace << /Pat [/Pattern /DeviceRGB] >> /Pattern << /P 7 0 R >>",
            "/Pat cs 0.2 0.4 0.6 /P scn 0 0 50 100 re f",
            &format!("/GB gs {outer} /Fm Do /GS gs 0 0 0 0 k 60 0 40 100 re f"),
        ))
    };
    let rgb_parent = render("1 0 0 rg");
    let gray_parent = render("0 g");
    assert_eq!(pixel(&rgb_parent, 25, 50), pixel(&gray_parent, 25, 50));
    // This control isolates inherited-source replacement, not the separate tint BG policy.
}

/// Repeated paints and a pair change after an earlier paint must use the current pair.
#[test]
fn repeated_paints_follow_a_later_pair_change() {
    let mut marks = String::from("0.2 0.4 0.6 rg 0 0 10 10 re f /GB gs ");
    for _ in 0..100 {
        marks.push_str("0 0 50 100 re f ");
    }
    assert_eq!(pixel(&paint(&marks), 25, 50), [52, 62, 79, 255]);
}

/// §8.9.6.2 supplies the current colour to the same stencil on both uses; its cache key
/// must therefore distinguish the colours produced before and after changing the pair.
#[test]
fn repeated_stencil_uses_the_current_pair() {
    let drawn = interpret(page_group_fixture(
        "/Group << /S /Transparency /CS /DeviceCMYK >>",
        "",
        "",
        "",
        "0.2 0.4 0.6 rg q 40 0 0 100 0 0 cm BI /W 1 /H 1 /IM true /BPC 1 /F /AHx ID 00> EI Q \
         /GB gs q 40 0 0 100 50 0 cm BI /W 1 /H 1 /IM true /BPC 1 /F /AHx ID 00> EI Q \
         /GS gs 0 0 0 0 k 90 0 10 100 re f",
    ));
    assert_eq!(pixel(&drawn, 20, 50), [51, 102, 154, 255]);
    assert_eq!(pixel(&drawn, 70, 50), [52, 62, 79, 255]);
}

/// A later non-RGB selection retires source metadata, including the initial colour of cs.
#[test]
fn changing_space_retires_rgb_components() {
    for selected in [
        "0.5 g",
        "/DeviceGray cs 0.5 scn",
        "/DeviceGray cs",
        "0 0.5 0 0 k",
    ] {
        let changed = paint(&format!("0.2 0.4 0.6 rg {selected} /GB gs 0 0 50 100 re f"));
        let control = paint(&format!("{selected} /GB gs 0 0 50 100 re f"));
        assert_eq!(
            pixel(&changed, 25, 50),
            pixel(&control, 25, 50),
            "{selected}"
        );
    }
}

/// The same retained source enters a different group and returns to the parent context.
#[test]
fn one_rgb_source_changes_target_and_returns() {
    let drawn = interpret(page_group_fixture(
        "",
        "/Group << /S /Transparency /CS /DeviceGray /I true >>",
        "",
        "35 0 30 100 re f",
        "0.2 0.4 0.6 rg 0 0 30 100 re f /Fm Do 70 0 30 100 re f",
    ));
    // §10.4.2.2: .3*.2 + .59*.4 + .11*.6 = .362, or byte92.
    assert_eq!(pixel(&drawn, 15, 50), [51, 102, 153, 255]);
    assert_eq!(pixel(&drawn, 50, 50), [92, 92, 92, 255]);
    assert_eq!(pixel(&drawn, 85, 50), [51, 102, 153, 255]);
}

/// The memo is pre-alpha: Table57's changed constant applies even on a colour hit.
#[test]
fn a_reused_colour_takes_the_current_alpha() {
    let drawn = interpret(page_group_fixture(
        "",
        "",
        "",
        "",
        "0.2 0.4 0.6 rg 0 0 40 100 re f /GS gs 50 0 40 100 re f",
    ));
    assert_eq!(pixel(&drawn, 20, 50), [51, 102, 153, 255]);
    let cold = interpret(page_group_fixture(
        "",
        "",
        "",
        "",
        "0.2 0.4 0.6 rg /GS gs 50 0 40 100 re f",
    ));
    let reused = pixel(&drawn, 70, 50);
    assert_eq!(reused, pixel(&cold, 70, 50));
    // §11.3.4's source-over with alpha1/2 on white gives (.6,.7,.8).
    // The raster's quantized alpha can move a component by one byte.
    for (actual, ideal) in reused.into_iter().zip([153_u8, 179, 204, 255]) {
        assert!(actual.abs_diff(ideal) <= 1);
    }
}

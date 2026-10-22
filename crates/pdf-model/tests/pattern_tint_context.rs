//! §11.7.5.3: the tint conversion uses the functions at painting, not defaults at selection.

#![expect(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "generated 100-unit fixture, bounded pixel indices and byte conversion; a malformed fixture must stop the instrument"
)]

use pdf_render::{Rasterizer, TargetSpec};
use pdf_syntax::Document;
use std::fmt::Write as _;
/// Interprets the generated one-page witness.
fn interpret(bytes: Vec<u8>) -> pdf_model::Interpretation {
    let document = Document::open(bytes).expect("the fixture is a valid PDF");
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    pdf_model::interpret(&document, &page)
}
/// Reads the opaque left-half sample from the CPU result.
fn pixel(interpretation: &pdf_model::Interpretation, x: u32, y: u32) -> [u8; 4] {
    pixel_list(&interpretation.display_list, x, y)
}

fn pixel_list(list: &pdf_render::DisplayList, x: u32, y: u32) -> [u8; 4] {
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
    fixture_with_cell(
        page_group,
        form_group,
        form_resources,
        form,
        page,
        "0 0 100 100 re f",
        "",
        "/DeviceCMYK",
    )
}

#[expect(
    clippy::too_many_arguments,
    reason = "independent page, Form, cell and nested-group fixture axes"
)]
fn fixture_with_cell(
    page_group: &str,
    form_group: &str,
    form_resources: &str,
    form: &str,
    page: &str,
    cell: &str,
    nested: &str,
    nested_space: &str,
) -> Vec<u8> {
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] {page_group} \
         /Resources << /ExtGState << /GS << /ca 0.5 /CA 0.5 >> \
         /GK << /BG2 << /FunctionType 2 /Domain [0 1] /C0 [0] /C1 [1] /N 1 >> >> \
         /GD << /BG2 /Default /BG << /FunctionType 2 /Domain [0 1] /C0 [0] /C1 [1] /N 1 >> >> \
         /GB << /BG << /FunctionType 2 /Domain [0 1] /C0 [1] /C1 [0] /N 1 >> \
         /UCR << /FunctionType 2 /Domain [0 1] /C0 [0] /C1 [0.5] /N 1 >> >> >> \
         /XObject << /Fm 5 0 R >> >> /Contents 4 0 R >>\nendobj\n\
         4 0 obj\n<< /Length {} >>\nstream\n{page}\nendstream\nendobj\n\
         5 0 obj\n<< /Type /XObject /Subtype /Form /BBox [0 0 100 100] {form_group} \
         /Resources << /ExtGState << /GS << /ca 0.5 /CA 0.5 >> /GB << /BG << /FunctionType 2 /Domain [0 1] /C0 [1] /C1 [0] /N 1 >> /UCR << /FunctionType 2 /Domain [0 1] /C0 [0] /C1 [0.5] /N 1 >> >> >> \
         {form_resources} /Font << /Ft 9 0 R >> /XObject << /In 6 0 R /Mask 8 0 R >> >> /Length {} >>\n\
         stream\n{form}\nendstream\nendobj\n\
         6 0 obj\n<< /Type /XObject /Subtype /Form /BBox [0 0 100 100] \
         /Group << /S /Transparency /CS {nested_space} /I true >> \
         /Resources << /ExtGState << /GS << /ca 0.5 /BM /Multiply >> >> >> /Length {} >>\n\
         stream\n{nested}\nendstream\nendobj\n",
        page.len() + 1,
        form.len() + 1,
        nested.len() + 1
    );

    let body = format!(
        "{body}7 0 obj\n<< /Type /Pattern /PatternType 1 /PaintType 2 /TilingType 1 /BBox [0 0 100 100] /XStep 100 /YStep 100 /Resources << /Font << /Ft 9 0 R >> /XObject << /In 6 0 R /Mask 8 0 R >> >> /Length {} >>\nstream\n{cell}\nendstream\nendobj\n",
        cell.len() + 1
    );
    let glyph = "100 0 0 0 100 100 d1 /In Do";
    let body = format!(
        "{body}8 0 obj\n<< /Type /XObject /Subtype /Image /Width 1 /Height 1 /ImageMask true /BitsPerComponent 1 /Filter /ASCIIHexDecode /Length 3 >>\nstream\n00>\nendstream\nendobj\n\
        9 0 obj\n<< /Type /Font /Subtype /Type3 /FontBBox [0 0 100 100] /FontMatrix [1 0 0 1 0 0] /CharProcs << /A 10 0 R >> /Encoding << /Type /Encoding /Differences [65 /A] >> /FirstChar 65 /LastChar 65 /Widths [100] /Resources << /XObject << /In 6 0 R >> >> >>\nendobj\n\
        10 0 obj\n<< /Length {} >>\nstream\n{glyph}\nendstream\nendobj\n",
        glyph.len() + 1
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

#[test]
fn selected_pattern_tint_uses_current_black_generation() {
    let pattern = interpret(page_group_fixture(
        "/Group << /S /Transparency /CS /DeviceCMYK >>",
        "",
        "/ColorSpace << /Pat [/Pattern /DeviceRGB] >> /Pattern << /P 7 0 R >>",
        "/Pat cs 0.2 0.4 0.6 /P scn 0 0 50 100 re f",
        "/GB gs 1 0 0 rg /Fm Do /GS gs 0 0 0 0 k 60 0 40 100 re f",
    ));
    // RGB(.2,.4,.6) gives C/M/Y=(.8,.6,.4), k=.4. UCR(k)=.2 and BG(k)=.6,
    // hence device-press CMYK(.6,.4,.2,.6), independently pinned by the solid/image witnesses.
    assert_eq!(pixel(&pattern, 25, 50), [52, 62, 79, 255]);
}

/// §11.6.7 keeps definition-entry state distinct from the mark binding.
#[test]
fn nested_targets_preserve_source_and_definition_state() {
    for nested_space in ["/DeviceRGB", "/DeviceCMYK", "/DeviceGray"] {
        for (name, page, form) in [
            (
                "entry-default_mark-default",
                "/Fm Do /GS gs 0 0 0 0 k 60 0 40 100 re f",
                "/Pat cs .2 .4 .6 /P scn 0 0 50 100 re f",
            ),
            (
                "entry-stated_mark-stated",
                "/GB gs /Fm Do /GS gs 0 0 0 0 k 60 0 40 100 re f",
                "/Pat cs .2 .4 .6 /P scn 0 0 50 100 re f",
            ),
            (
                "entry-default_mark-stated",
                "/Fm Do /GS gs 0 0 0 0 k 60 0 40 100 re f",
                "/GB gs /Pat cs .2 .4 .6 /P scn 0 0 50 100 re f",
            ),
        ] {
            let bytes = fixture_with_cell(
                "/Group << /S /Transparency /CS /DeviceCMYK >>",
                "",
                "/ColorSpace << /Pat [/Pattern /DeviceRGB] >> /Pattern << /P 7 0 R >>",
                form,
                page,
                "/In Do",
                "0 0 100 100 re f /GS gs 30 0 10 100 re f",
                nested_space,
            );
            let result = interpret(bytes);
            let solid_form = form.replace(
                "/Pat cs .2 .4 .6 /P scn 0 0 50 100 re f",
                ".2 .4 .6 rg /In Do",
            );
            let solid = interpret(fixture_with_cell(
                "/Group << /S /Transparency /CS /DeviceCMYK >>",
                "",
                "",
                &solid_form,
                page,
                "",
                "0 0 100 100 re f /GS gs 30 0 10 100 re f",
                nested_space,
            ));
            let reference = if name == "entry-default_mark-stated" && nested_space != "/DeviceCMYK"
            {
                // RGB/Gray exit conversion belongs to the nested Do in definition state.
                // The cell's stream began before GB; changing the outer mark cannot replace it.
                interpret(fixture_with_cell(
                    "/Group << /S /Transparency /CS /DeviceCMYK >>",
                    "",
                    "",
                    &solid_form.replace("/GB gs ", ""),
                    page,
                    "",
                    "0 0 100 100 re f /GS gs 30 0 10 100 re f",
                    nested_space,
                ))
            } else {
                solid
            };
            assert_eq!(
                pixel(&result, 35, 50),
                pixel(&reference, 35, 50),
                "{nested_space}: {name}"
            );
            if nested_space == "/DeviceGray" && name == "entry-default_mark-default" {
                let [r, g, b, a] = pixel(&result, 35, 50);
                assert_eq!(
                    (r, g, a),
                    (g, b, 255),
                    "a default Gray group must be achromatic"
                );
            }
        }
    }
}

#[test]
fn selected_tint_observes_later_pair_and_restores_after_q() {
    let result = interpret(page_group_fixture(
        "/Group << /S /Transparency /CS /DeviceCMYK >>",
        "",
        "/ColorSpace << /Pat [/Pattern /DeviceRGB] >> /Pattern << /P 7 0 R >>",
        "/Pat cs .2 .4 .6 /P scn 0 0 20 100 re f q /GB gs 20 0 20 100 re f Q 40 0 20 100 re f",
        "/Fm Do /GS gs 0 0 0 0 k 70 0 30 100 re f",
    ));
    assert_eq!(pixel(&result, 10, 50), [51, 102, 154, 255]);
    assert_eq!(pixel(&result, 30, 50), [52, 62, 79, 255]);
    assert_eq!(pixel(&result, 50, 50), [51, 102, 154, 255]);
}

#[test]
fn stencil_on_either_side_of_the_cell_uses_the_selected_source() {
    for (cell, mark) in [
        ("0 0 100 100 re f", "q 50 0 0 100 0 0 cm /Mask Do Q"),
        ("q 100 0 0 100 0 0 cm /Mask Do Q", "0 0 50 100 re f"),
    ] {
        let form = format!("/Pat cs .2 .4 .6 /P scn /GB gs {mark}");
        let result = interpret(fixture_with_cell(
            "/Group << /S /Transparency /CS /DeviceCMYK >>",
            "",
            "/ColorSpace << /Pat [/Pattern /DeviceRGB] >> /Pattern << /P 7 0 R >>",
            &form,
            "/Fm Do /GS gs 0 0 0 0 k 70 0 30 100 re f",
            cell,
            "",
            "/DeviceCMYK",
        ));
        assert_eq!(pixel(&result, 25, 50), [52, 62, 79, 255]);
    }
}

#[test]
fn type3_without_a_tint_binding_keeps_its_own_space_refusal() {
    let result = interpret(fixture_with_cell(
        "",
        "",
        "",
        ".2 .4 .6 rg BT /Ft 1 Tf 0 0 Td (A) Tj ET",
        "/Fm Do",
        "",
        "0 0 100 100 re f /GS gs 30 0 10 100 re f",
        "/DeviceGray",
    ));
    let [r, g, b, a] = pixel(&result, 35, 50);
    assert_eq!(a, 255);
    assert!(
        r != g && g != b,
        "unbound d1 must not gain source-backed Gray admission"
    );
}

#[test]
fn text_inside_an_uncoloured_cell_keeps_the_source_binding() {
    let result = interpret(fixture_with_cell(
        "/Group << /S /Transparency /CS /DeviceCMYK >>",
        "",
        "/ColorSpace << /Pat [/Pattern /DeviceRGB] >> /Pattern << /P 7 0 R >>",
        "/Pat cs .2 .4 .6 /P scn /GB gs 0 0 50 100 re f",
        "/Fm Do /GS gs 0 0 0 0 k 70 0 30 100 re f",
        "BT /Ft 1 Tf 0 0 Td (A) Tj ET",
        "0 0 100 100 re f",
        "/DeviceCMYK",
    ));
    assert_eq!(pixel(&result, 25, 50), [52, 62, 79, 255]);
}

#[test]
fn the_mark_alpha_is_applied_once_after_tint_conversion() {
    let render = |form: &str| {
        interpret(page_group_fixture(
            "/Group << /S /Transparency /CS /DeviceCMYK >>",
            "",
            "/ColorSpace << /Pat [/Pattern /DeviceRGB] >> /Pattern << /P 7 0 R >>",
            form,
            "/Fm Do /GS gs 0 0 0 0 k 70 0 30 100 re f",
        ))
    };
    let patterned = render("/Pat cs .2 .4 .6 /P scn /GB gs /GS gs 0 0 50 100 re f");
    let solid = render(".2 .4 .6 rg /GB gs /GS gs 0 0 50 100 re f");
    assert_eq!(pixel(&patterned, 25, 50), pixel(&solid, 25, 50));
    assert_ne!(pixel(&patterned, 25, 50), [52, 62, 79, 255]);
}

#[test]
fn colour_operators_inside_the_cell_cannot_replace_its_tint() {
    let render = |cell: &str| {
        interpret(fixture_with_cell(
            "/Group << /S /Transparency /CS /DeviceCMYK >>",
            "",
            "/ColorSpace << /Pat [/Pattern /DeviceRGB] >> /Pattern << /P 7 0 R >>",
            "/Pat cs .2 .4 .6 /P scn /GB gs 0 0 50 100 re f",
            "/Fm Do /GS gs 0 0 0 0 k 70 0 30 100 re f",
            cell,
            "",
            "/DeviceCMYK",
        ))
    };
    let plain = render("0 0 100 100 re f");
    let forbidden = render("1 0 0 rg 0 g /AbsoluteColorimetric ri 0 0 100 100 re f");
    assert_eq!(pixel(&plain, 25, 50), [52, 62, 79, 255]);
    assert_eq!(pixel(&forbidden, 25, 50), pixel(&plain, 25, 50));
}

#[test]
fn spot_tints_survive_nested_targets_and_return_to_the_parent_plane() {
    const FUNCTION: &str = "<< /FunctionType 2 /Domain [0 1] /C0 [0 0 0 0] /C1 [0 1 1 0] /N 1 >>";
    for space in [
        format!("[/Separation /Tint /DeviceCMYK {FUNCTION}]"),
        format!("[/DeviceN [/Tint] /DeviceCMYK {FUNCTION}]"),
    ] {
        for target in ["/DeviceRGB", "/DeviceCMYK", "/DeviceGray"] {
            let resources = format!(
                "/ColorSpace << /Pat [/Pattern {space}] /Source {space} >> /Pattern << /P 7 0 R >>"
            );
            let interpret_spots = |form: &str| {
                let document = Document::open(fixture_with_cell(
                    "",
                    "",
                    &resources,
                    form,
                    "/Fm Do",
                    "/In Do",
                    "0 0 50 100 re f /GS gs 30 0 10 100 re f",
                    target,
                ))
                .expect("spot fixture");
                let page = pdf_model::Pages::new(&document).get(0).expect("page");
                let mut state = pdf_model::view::ViewState::of(&document);
                state.set_separation_simulation(true);
                pdf_model::content::interpret_with(&document, &page, &state)
            };
            let pattern = interpret_spots(
                "/Pat cs .5 /P scn 0 0 50 100 re f /Source cs .25 scn 60 0 40 100 re f",
            );
            let solid =
                interpret_spots("/Source cs .5 scn /In Do /Source cs .25 scn 60 0 40 100 re f");
            let selected = pattern.separation.as_ref().expect("spot planes");
            let reference = solid.separation.as_ref().expect("reference spot planes");
            assert_eq!(selected.colourants(), reference.colourants());
            assert_eq!(selected.colourants().len(), 1);
            assert_eq!(selected.plane_count(), reference.plane_count());
            assert!(selected.without_a_plane().is_empty());
            for plane in [
                pdf_model::colour::Plane::Chromatic,
                pdf_model::colour::Plane::Black,
                pdf_model::colour::Plane::Spot(0),
            ] {
                let actual =
                    pdf_model::colourants::plane(&pattern.display_list, plane).expect("plane");
                let expected = pdf_model::colourants::plane(&solid.display_list, plane)
                    .expect("reference plane");
                for x in [25, 35, 75] {
                    assert_eq!(
                        pixel_list(actual, x, 50),
                        pixel_list(expected, x, 50),
                        "{target}; {space}; {plane:?}; x{x}"
                    );
                }
            }
        }
    }
}

#[test]
fn a_pattern_tint_cannot_leave_its_conversion_in_an_ordinary_solid() {
    let render = |form: &str| {
        interpret(page_group_fixture(
            "/Group << /S /Transparency /CS /DeviceCMYK >>",
            "",
            "/ColorSpace << /Pat [/Pattern /DeviceRGB] >> /Pattern << /P 7 0 R >>",
            form,
            "/Fm Do /GS gs 0 0 0 0 k 90 0 10 100 re f",
        ))
    };
    let mixed = render(
        ".2 .4 .6 rg .2 .4 .6 RG q /GB gs /Pat cs .2 .4 .6 /P scn 0 0 20 100 re f Q 20 0 20 100 re f 10 w 60 0 m 60 100 l S q /GB gs /Pat cs .2 .4 .6 /P scn /GS gs 70 0 10 100 re f Q",
    );
    let cold = render(
        ".2 .4 .6 rg .2 .4 .6 RG 20 0 20 100 re f 10 w 60 0 m 60 100 l S q /GB gs /GS gs 70 0 10 100 re f Q",
    );
    assert_eq!(pixel(&mixed, 10, 50), [52, 62, 79, 255]);
    for x in [30, 60, 75] {
        assert_eq!(
            pixel(&mixed, x, 50),
            pixel(&cold, x, 50),
            "consumer at x{x}"
        );
    }
    assert_eq!(pixel(&mixed, 30, 50), [51, 102, 154, 255]);
}

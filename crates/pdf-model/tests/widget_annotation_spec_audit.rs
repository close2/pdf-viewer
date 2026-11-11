//! Selected Widget appearance controls for shared icon fit, caption layering and highlight modes.

#![expect(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "complete test fixtures and checked pixel coordinates should fail loudly"
)]

use std::fmt::Write as _;

use pdf_model::view::{Pointer, ViewState};
use pdf_render::{Raster, Rasterizer, TargetSpec};
use pdf_syntax::{Document, ObjectId};
use render_cpu::CpuRasterizer;

const PIXEL_BUDGET: u64 = 1 << 20;

fn stream_form(colour: &str) -> String {
    let content = format!("{colour} rg 0 0 10 10 re f");
    format!(
        "<< /Type /XObject /Subtype /Form /BBox [0 0 10 10] /Resources << >> /Length {} >>\nstream\n{}\nendstream",
        content.len(),
        content
    )
}

fn assembled_pdf(objects: &[String]) -> Vec<u8> {
    let mut out = String::from("%PDF-1.7\n");
    let mut offsets = vec![0usize];
    for (index, body) in objects.iter().enumerate() {
        offsets.push(out.len());
        writeln!(out, "{} 0 obj\n{}\nendobj", index + 1, body).expect("String write");
    }
    let xref = out.len();
    writeln!(out, "xref\n0 {}", offsets.len()).expect("String write");
    out.push_str("0000000000 65535 f \n");
    for offset in offsets.iter().skip(1) {
        writeln!(out, "{offset:010} 00000 n ").expect("String write");
    }
    write!(
        out,
        "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
        offsets.len()
    )
    .expect("String write");
    out.into_bytes()
}

fn widget_with_three_icons(fit: Option<&str>) -> Vec<u8> {
    let fit = fit.map_or_else(String::new, |fit| format!(" /IF {fit}"));
    let objects = vec![
        "<< /Type /Catalog /Pages 2 0 R /AcroForm << /Fields [5 0 R] >> >>".to_owned(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Resources << >> /Contents 4 0 R /Annots [5 0 R] >>".to_owned(),
        "<< /Length 0 >>\nstream\n\nendstream".to_owned(),
        format!(
            "<< /Type /Annot /Subtype /Widget /Rect [10 10 90 90] /F 4 /P 3 0 R \
             /FT /Btn /Ff 65536 /T (button) /H /N \
             /MK << /I 6 0 R /RI 7 0 R /IX 8 0 R /TP 1{fit} >> >>"
        ),
        stream_form("1 0 0"),
        stream_form("0 1 0"),
        stream_form("0 0 1"),
    ];
    assembled_pdf(&objects)
}

fn widget_with_down_appearance(mode: &str) -> Vec<u8> {
    let mode = if mode.is_empty() {
        String::new()
    } else {
        format!("/H {mode}")
    };
    let normal = "1 0 0 rg 0 0 80 80 re f";
    let down = "0 0 1 rg 0 0 80 80 re f";
    let form = |content: &str| {
        format!(
            "<< /Type /XObject /Subtype /Form /BBox [0 0 80 80] /Resources << >> /Length {} >>\nstream\n{}\nendstream",
            content.len(),
            content
        )
    };
    let objects = vec![
        "<< /Type /Catalog /Pages 2 0 R /AcroForm << /Fields [5 0 R] >> >>".to_owned(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Resources << >> /Contents 4 0 R /Annots [5 0 R] >>".to_owned(),
        "<< /Length 0 >>\nstream\n\nendstream".to_owned(),
        format!(
            "<< /Type /Annot /Subtype /Widget /Rect [10 10 90 90] /F 4 /P 3 0 R \
             /FT /Btn /Ff 65536 /T (button) {mode} /AP << /N 6 0 R /D 7 0 R >> >>"
        ),
        form(normal),
        form(down),
    ];
    assembled_pdf(&objects)
}

fn raster(bytes: Vec<u8>, pointer: Option<Pointer>) -> Raster {
    let document = Document::open(bytes).expect("valid PDF syntax");
    assert!(!document.was_recovered(), "exact xref offsets");
    let page = pdf_model::Pages::new(&document).get(0).expect("one page");
    let mut state = ViewState::of(&document);
    state.set_pointer(pointer.map(|pointer| {
        (
            ObjectId {
                number: 5,
                generation: 0,
            },
            pointer,
        )
    }));
    let interpretation = pdf_model::content::interpret_with(&document, &page, &state);
    assert!(
        interpretation.is_complete(),
        "complete input should construct/render: {:?}",
        interpretation.unsupported
    );
    let target = TargetSpec::for_page(&interpretation.display_list, 1.0, PIXEL_BUDGET)
        .expect("100-point page fits target budget");
    CpuRasterizer::new()
        .with_medium(pdf_render::Medium::NONE)
        .rasterize(&interpretation.display_list, target)
        .expect("supported forms")
}

fn rgba(raster: &Raster, x: u32, y: u32) -> [u8; 4] {
    let row = raster.height.saturating_sub(1).saturating_sub(y);
    let index = ((row.saturating_mul(raster.width)).saturating_add(x) as usize).saturating_mul(4);
    [
        raster.data[index],
        raster.data[index + 1],
        raster.data[index + 2],
        raster.data[index + 3],
    ]
}

#[test]
fn widget_if_is_applied_to_normal_rollover_and_alternate_icons() {
    // A positive control: without /IF, Table250 defaults scale the normal 10×10 icon to the
    // 80×80 widget, so a corner pixel inside /Rect is painted.
    let default_fit = raster(widget_with_three_icons(None), None);
    assert_eq!(rgba(&default_fit, 15, 15), [255, 0, 0, 255]);

    // /SW /N plus centered /A keeps every state icon 10×10. A Table192 implementation that
    // applies /IF only to /I would incorrectly scale the green or blue state to fill /Rect.
    let fit = Some("<< /SW /N /S /P /A [0.5 0.5] >>");
    for (pointer, expected, state) in [
        (None, [255, 0, 0, 255], "normal /I"),
        (Some(Pointer::Over), [0, 255, 0, 255], "rollover /RI"),
        (Some(Pointer::Down), [0, 0, 255, 255], "alternate /IX"),
    ] {
        let image = raster(widget_with_three_icons(fit), pointer);
        assert_eq!(
            rgba(&image, 50, 50),
            expected,
            "center pixel selects {state}"
        );
        assert_eq!(
            rgba(&image, 20, 20),
            [0, 0, 0, 0],
            "{state} respects shared /IF"
        );
    }
}

#[test]
fn widget_highlighting_t_is_the_same_as_p_for_a_stored_down_appearance() {
    let p_normal = raster(widget_with_down_appearance("/P"), None);
    let t_normal = raster(widget_with_down_appearance("/T"), None);
    assert_eq!(rgba(&p_normal, 50, 50), [255, 0, 0, 255]);
    assert_eq!(rgba(&t_normal, 50, 50), [255, 0, 0, 255]);

    let p_down = raster(widget_with_down_appearance("/P"), Some(Pointer::Down));
    let t_down = raster(widget_with_down_appearance("/T"), Some(Pointer::Down));
    assert_eq!(rgba(&p_down, 50, 50), [0, 0, 255, 255]);
    assert_eq!(rgba(&t_down, 50, 50), [0, 0, 255, 255]);
}

fn button_with_icon_and_caption(position: u8) -> Vec<u8> {
    let icon = "1 0 0 rg 0 0 80 80 re f";
    let objects = vec![
        "<< /Type /Catalog /Pages 2 0 R /AcroForm << /Fields [5 0 R] /DR << /Font << /Helv 7 0 R >> >> >> >>".to_owned(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Resources << >> /Contents 4 0 R /Annots [5 0 R] >>".to_owned(),
        "<< /Length 0 >>\nstream\n\nendstream".to_owned(),
        format!(
            "<< /Type /Annot /Subtype /Widget /Rect [10 10 90 90] /F 4 /P 3 0 R \
             /FT /Btn /Ff 65536 /T (button) /DA (/Helv 10 Tf 0 g) \
             /MK << /I 6 0 R /CA (A) /TP {position} >> >>"
        ),
        format!(
            "<< /Type /XObject /Subtype /Form /BBox [0 0 80 80] /Resources << >> /Length {} >>\nstream\n{}\nendstream",
            icon.len(), icon
        ),
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_owned(),
    ];
    assembled_pdf(&objects)
}

#[test]
fn widget_tp_six_overlays_the_caption_on_the_icon() {
    // TP=1 is the positive control: same icon and caption, but the table says icon only.
    let icon_only_bytes = button_with_icon_and_caption(1);
    let icon_only_document = Document::open(icon_only_bytes.clone()).expect("valid PDF");
    let icon_only_page = pdf_model::Pages::new(&icon_only_document)
        .get(0)
        .expect("one page");
    let icon_only = pdf_model::interpret(&icon_only_document, &icon_only_page);
    assert!(icon_only.is_complete(), "{:?}", icon_only.unsupported);
    assert_eq!(icon_only.glyphs, 0, "TP=1 says no caption");
    assert_eq!(
        rgba(&raster(icon_only_bytes, None), 15, 15),
        [255, 0, 0, 255]
    );

    // TP=6 says the caption is overlaid directly on the icon: both must contribute.
    let overlaid_bytes = button_with_icon_and_caption(6);
    let overlaid_document = Document::open(overlaid_bytes.clone()).expect("valid PDF");
    assert!(!overlaid_document.was_recovered(), "exact xref offsets");
    let overlaid_page = pdf_model::Pages::new(&overlaid_document)
        .get(0)
        .expect("one page");
    let overlaid = pdf_model::interpret(&overlaid_document, &overlaid_page);
    assert!(overlaid.is_complete(), "{:?}", overlaid.unsupported);
    assert_eq!(overlaid.glyphs, 1, "TP=6 includes the one caption glyph");
    let image = raster(overlaid_bytes.clone(), None);
    assert!(
        image
            .data
            .chunks_exact(4)
            .any(|pixel| pixel[0] < 200 && pixel[1] == 0 && pixel[2] == 0 && pixel[3] == 255),
        "black caption must darken the red icon (allowing antialiasing)"
    );
    assert_eq!(
        rgba(&raster(overlaid_bytes, None), 15, 15),
        [255, 0, 0, 255]
    );
}

#[test]
fn omitted_widget_h_retains_the_table_191_invert_default_even_with_a_down_stream() {
    let stated = raster(widget_with_down_appearance("/I"), Some(Pointer::Down));
    assert_eq!(
        rgba(&stated, 50, 50),
        [0, 255, 255, 255],
        "explicit I overrides blue D and inverts red N"
    );
    let omitted = raster(widget_with_down_appearance(""), Some(Pointer::Down));
    assert_eq!(
        rgba(&omitted, 50, 50),
        [0, 255, 255, 255],
        "Table191 defaults H to I even with stored D"
    );
    assert!(
        omitted.data == stated.data,
        "omitted H must equal explicit default I"
    );
}

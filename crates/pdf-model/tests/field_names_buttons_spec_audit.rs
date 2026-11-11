//! Field identity and effective EC3 radio Off appearance controls.

use std::fmt::Write as _;

use pdf_render::{Rasterizer as _, TargetSpec};
use pdf_syntax::Document;
use render_cpu::CpuRasterizer;

use pdf_model::{Pages, content::interpret_with, form, view::ViewState};

fn pdf(objects: &[&str]) -> Vec<u8> {
    let mut out = String::from("%PDF-1.7\n");
    let mut offsets = Vec::with_capacity(objects.len());
    for (index, body) in objects.iter().enumerate() {
        offsets.push(out.len());
        let _ = writeln!(out, "{} 0 obj\n{body}\nendobj", index.saturating_add(1));
    }
    let xref_at = out.len();
    let size = objects.len().saturating_add(1);
    let _ = writeln!(out, "xref\n0 {size}\n0000000000 65535 f ");
    for offset in offsets {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    let _ = write!(
        out,
        "trailer\n<< /Size {size} /Root 1 0 R /ID \
         [<0123456789abcdef0123456789abcdef> <0123456789abcdef0123456789abcdef>] >>\n\
         startxref\n{xref_at}\n%%EOF\n"
    );
    out.into_bytes()
}

/// A complete, one-page PDF whose terminal field is three partial names below the root.
fn nested_name_pdf() -> Vec<u8> {
    pdf(&[
        "<< /Type /Catalog /Pages 2 0 R /AcroForm << /Fields [4 0 R] /DA (/F1 12 Tf 0 g) \
         /DR << /Font << /F1 7 0 R >> >> >> >>",
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 120] /Resources << >> \
         /Contents 10 0 R /Annots [8 0 R] >>",
        "<< /T (PersonalData) /Kids [5 0 R] >>",
        "<< /Parent 4 0 R /T (Address) /Kids [6 0 R] >>",
        "<< /Parent 5 0 R /FT /Tx /T (ZipCode) /V (02139) /DA (/F1 12 Tf 0 g) \
         /Kids [8 0 R] >>",
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>",
        "<< /Type /Annot /Subtype /Widget /Parent 6 0 R /Rect [10 10 100 34] /F 4 /AP << /N 9 0 R >> >>",
        "<< /Type /XObject /Subtype /Form /BBox [0 0 90 24] /Resources << /Font << /F1 7 0 R >> >> /Length 45 >>\nstream\nBT /F1 12 Tf 0 g 1 0 0 1 2 8 Tm (02139) Tj ET\nendstream",
        "<< /Length 0 >>\nstream\n\nendstream",
    ])
}

/// The ancestry join is the field name a host and named action use, not just a parser string.
#[test]
fn a_complete_form_reports_the_period_joined_name_and_terminal_partial_name() {
    let document = Document::open(nested_name_pdf()).expect("a complete PDF with an exact xref");
    assert!(!document.was_recovered(), "the xref is authoritative");
    let page = Pages::new(&document).get(0).expect("the one declared page");
    let fields = form::fields(&document, &page, &ViewState::of(&document));

    assert_eq!(fields.len(), 1, "one terminal field beneath two containers");
    let field = fields.first().expect("one field");
    assert_eq!(field.name.qualified, "PersonalData.Address.ZipCode");
    assert_eq!(field.partial, "ZipCode");
    assert_eq!(
        field.widgets.len(),
        1,
        "the unnamed widget is the field's child"
    );
    assert_eq!(
        field.value.as_ref().map(|value| value.text.as_str()),
        Some("02139")
    );
}

fn radio_off_state_pdf() -> Vec<u8> {
    let on = "1 0 0 rg 0 0 40 40 re f";
    let objects = [
        "<< /Type /Catalog /Pages 2 0 R /AcroForm << /Fields [5 0 R] >> >>".to_owned(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 110 60] /Resources << >> \
         /Contents 4 0 R /Annots [6 0 R 7 0 R] >>"
            .to_owned(),
        "<< /Length 0 >>\nstream\n\nendstream".to_owned(),
        "<< /FT /Btn /T (choice) /Ff 49152 /V /Yes /Kids [6 0 R 7 0 R] >>".to_owned(),
        "<< /Type /Annot /Subtype /Widget /Parent 5 0 R /F 4 /Rect [10 10 50 50] \
         /AS /Yes /AP << /N << /Yes 8 0 R /Off 9 0 R >> >> >>"
            .to_owned(),
        "<< /Type /Annot /Subtype /Widget /Parent 5 0 R /F 4 /Rect [60 10 100 50] \
         /AS /Off /AP << /N << /Yes 8 0 R >> >> >>"
            .to_owned(),
        format!(
            "<< /Type /XObject /Subtype /Form /BBox [0 0 40 40] /Resources << >> \
             /Length {} >>\nstream\n{on}\nendstream",
            on.len().saturating_add(1)
        ),
        "<< /Type /XObject /Subtype /Form /BBox [0 0 40 40] /Resources << >> \
         /Length 0 >>\nstream\n\nendstream"
            .to_owned(),
    ];
    let refs: Vec<&str> = objects.iter().map(String::as_str).collect();
    pdf(&refs)
}

/// EC3 #170 extends the optional `/Off` appearance rule to radio buttons.
#[test]
fn a_radio_off_state_without_an_off_stream_draws_nothing_and_keeps_the_other_choice() {
    let document =
        Document::open(radio_off_state_pdf()).expect("a complete PDF with an exact xref");
    assert!(!document.was_recovered(), "the xref is authoritative");
    let page = Pages::new(&document).get(0).expect("the one declared page");
    let view = ViewState::of(&document);
    let interpreted = interpret_with(&document, &page, &view);
    assert!(
        interpreted.is_complete(),
        "nothing about the radio form is unsupported: {:?}",
        interpreted.unsupported
    );

    let target = TargetSpec::for_page(&interpreted.display_list, 1.0, 1 << 20)
        .expect("the small page is inside the raster budget");
    let raster = CpuRasterizer::new()
        .with_medium(pdf_render::Medium::NONE)
        .rasterize(&interpreted.display_list, target)
        .expect("the fixture uses a solid rectangle");
    let pixel = |x: u32, y: u32| {
        let pixel = u64::from(y) * u64::from(raster.width) + u64::from(x);
        let start = usize::try_from(pixel * 4).expect("small raster index");
        <[u8; 4]>::try_from(
            raster
                .data
                .get(start..start.saturating_add(4))
                .expect("in bounds pixel"),
        )
        .expect("RGBA")
    };

    assert_eq!(
        pixel(20, 30),
        [255, 0, 0, 255],
        "the /V-selected Yes state is visible"
    );
    assert_eq!(
        pixel(80, 30),
        [0, 0, 0, 0],
        "the absent optional /Off state leaves no mark"
    );
}

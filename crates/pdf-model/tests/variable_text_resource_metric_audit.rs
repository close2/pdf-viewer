//! Saved variable-text quadding must agree with the font retained in appearance resources.

#![expect(
    clippy::expect_used,
    reason = "complete fixture and exact saved-output assertions must fail loudly"
)]

use std::fmt::Write as _;

use pdf_model::view::{Entered, ViewState};
use pdf_render::{Rasterizer, TargetSpec};
use pdf_syntax::{Document, ObjectId};
use render_cpu::CpuRasterizer;

fn fixture(default_font: &str) -> Vec<u8> {
    // The old appearance says /Helv is Courier. The form's DR says the same name is Helvetica
    // in one run and Courier in the matching-resource control.
    let old = "/Tx BMC BT /Helv 12 Tf 0 g 1 0 0 1 136.4 12 Tm (old) Tj ET EMC";
    let appearance = format!(
        "<< /Type /XObject /Subtype /Form /BBox [0 0 160 30] \
         /Resources << /Font << /Helv 7 0 R >> >> /Length {} >>\nstream\n{old}\nendstream",
        old.len().saturating_add(1)
    );
    let objects = [
        "<< /Type /Catalog /Pages 2 0 R /AcroForm << /Fields [4 0 R] \
             /DA (/Helv 12 Tf 0 g) /DR << /Font << /Helv 6 0 R >> >> >> >>"
            .to_owned(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 100] \
         /Resources << >> /Annots [4 0 R] >>"
            .to_owned(),
        "<< /Type /Annot /Subtype /Widget /Rect [20 40 180 70] /F 4 /P 3 0 R \
         /FT /Tx /T (field) /V (old) /Q 2 /AP << /N 5 0 R >> >>"
            .to_owned(),
        appearance,
        format!(
            "<< /Type /Font /Subtype /Type1 /BaseFont /{default_font} \
             /Encoding /WinAnsiEncoding >>"
        ),
        "<< /Type /Font /Subtype /Type1 /BaseFont /Courier /Encoding /WinAnsiEncoding >>"
            .to_owned(),
    ];
    let mut out = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for (index, object) in objects.iter().enumerate() {
        offsets.push(out.len());
        let _ = writeln!(out, "{} 0 obj\n{object}\nendobj", index.saturating_add(1));
    }
    let xref = out.len();
    let size = objects.len().saturating_add(1);
    let _ = writeln!(out, "xref\n0 {size}\n0000000000 65535 f ");
    for offset in offsets {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    let _ = write!(
        out,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n"
    );
    out.into_bytes()
}

fn saved(default_font: &str) -> Document {
    let document = Document::open(fixture(default_font)).expect("complete PDF fixture");
    assert!(!document.was_recovered(), "source xref offsets are exact");
    let page = pdf_model::Pages::new(&document)
        .get(0)
        .expect("source page");
    let before = pdf_model::content::interpret_with(&document, &page, &ViewState::of(&document));
    assert!(before.is_complete(), "{:?}", before.unsupported);
    assert!(before.text.contains("old"), "source AP matches its value");
    let mut view = ViewState::of(&document);
    assert_eq!(
        view.set_field(&document, "field", &Entered::Text("MMMM".to_owned())),
        1,
        "the one text widget accepted the edit"
    );
    let written = view
        .save(&document)
        .expect("the widget appearance is saved");
    assert!(written.unconstructed.is_empty(), "the edit laid out fully");
    let saved = Document::open(written.bytes).expect("the saved incremental update reopens");
    assert!(!saved.was_recovered(), "saved xref offsets are exact");
    let widget = saved
        .get(ObjectId {
            number: 4,
            generation: 0,
        })
        .as_dict()
        .cloned()
        .expect("widget remains a dictionary");
    let appearances = saved
        .get_key(&widget, "AP")
        .as_dict()
        .cloned()
        .expect("widget AP dictionary");
    let normal = saved
        .get_key(&appearances, "N")
        .as_stream()
        .cloned()
        .expect("the saved normal appearance is a stream");
    let resources = saved
        .get_key(&normal.dict, "Resources")
        .as_dict()
        .cloned()
        .expect("the appearance resources survive");
    let fonts = saved
        .get_key(&resources, "Font")
        .as_dict()
        .cloned()
        .expect("appearance Font resources survive");
    let helv = saved.get_key(&fonts, "Helv");
    assert_eq!(
        helv.as_dict().map(|dict| saved
            .get_key(dict, "BaseFont")
            .as_name()
            .map(|name| name.as_bytes().to_vec())),
        Some(Some(b"Courier".to_vec())),
        "the stream's same-named resource wins over /DR"
    );
    saved
}

fn ink_span(document: &Document) -> (u32, u32) {
    let page = pdf_model::Pages::new(document).get(0).expect("page one");
    let list = pdf_model::content::interpret_with(document, &page, &ViewState::of(document));
    assert!(
        list.is_complete(),
        "saved appearance interprets: {:?}",
        list.unsupported
    );
    assert!(list.text.contains("MMMM"), "new value is drawn");
    assert!(!list.text.contains("old"), "old value was replaced");
    let target = TargetSpec::for_page(&list.display_list, 1.0, 1 << 20).expect("page target");
    let raster = CpuRasterizer::new()
        .with_medium(pdf_render::Medium::NONE)
        .rasterize(&list.display_list, target)
        .expect("CPU rasterization");
    let mut columns = Vec::new();
    for x in 0..raster.width {
        let ink = (0..raster.height).any(|row| {
            let pixel = usize::try_from(
                row.saturating_mul(raster.width)
                    .saturating_add(x)
                    .saturating_mul(4),
            )
            .unwrap_or(usize::MAX);
            raster
                .data
                .get(pixel.saturating_add(3))
                .copied()
                .unwrap_or(0)
                > 0
        });
        if ink {
            columns.push(x);
        }
    }
    (
        columns.first().copied().expect("text has visible ink"),
        columns.last().copied().expect("text has visible ink"),
    )
}

#[test]
fn saved_quadding_uses_the_font_that_the_appearance_resource_name_resolves_to() {
    let dr_helvetica_ap_courier = saved("Helvetica");
    let dr_courier_ap_courier = saved("Courier");
    let mismatched_metrics = ink_span(&dr_helvetica_ap_courier);
    let matching_metrics = ink_span(&dr_courier_ap_courier);

    assert!(
        (177..=180).contains(&matching_metrics.1),
        "Courier's actual metrics put right-quadded MMMM at the widget's right edge: {matching_metrics:?}"
    );
    assert_eq!(
        mismatched_metrics, matching_metrics,
        "both saved appearances resolve /Helv to Courier, so layout must use Courier's metrics even if /DR's colliding /Helv is Helvetica"
    );
}

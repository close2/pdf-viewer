//! Existing appearance resources and `AcroForm` defaults through a public edit/save/reopen.

#![expect(
    clippy::expect_used,
    reason = "complete fixture construction and direct assertions on saved PDF objects"
)]

use std::fmt::Write as _;

use pdf_model::view::{Entered, ViewState};
use pdf_syntax::{Document, ObjectId};

fn resource_update_pdf(extra_fonts: &str, widget_da: bool, stored: bool) -> Vec<u8> {
    let appearance = "/Tx BMC BT /Old 12 Tf 0 g 1 0 0 1 2 12 Tm (old) Tj ET EMC";
    let widget_da = if widget_da {
        "/DA (/Helv 12 Tf 0 g)"
    } else {
        ""
    };
    let ap = if stored { "/AP << /N 6 0 R >>" } else { "" };
    let need = if stored { "" } else { "/NeedAppearances true" };
    let objects = [
        format!(
            "<< /Type /Catalog /Pages 2 0 R /AcroForm << /Fields [5 0 R] {need} /DA (/Helv 12 Tf 0 g) /DR << /Font << /Helv 7 0 R >> >> >> >>"
        ),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 100] \
         /Resources << >> /Contents 4 0 R /Annots [5 0 R] >>"
            .to_owned(),
        "<< /Length 0 >>\nstream\n\nendstream".to_owned(),
        format!(
            "<< /Type /Annot /Subtype /Widget /Rect [20 40 180 70] /P 3 0 R \
             /FT /Tx /T (field) /V (old) {widget_da} {ap} >>"
        ),
        format!(
            "<< /Type /XObject /Subtype /Form /BBox [0 0 160 30] \
             /Resources << /Font << /Old 8 0 R {extra_fonts} >> >> /Length {} >>\nstream\n{appearance}\nendstream",
            appearance.len()
        ),
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>"
            .to_owned(),
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
    let _ = writeln!(out, "xref\n0 9\n0000000000 65535 f ");
    for offset in offsets {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    let _ = write!(
        out,
        "trailer\n<< /Size 9 /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n"
    );
    out.into_bytes()
}

fn saved_font_name(extra_fonts: &str, widget_da: bool) -> Vec<u8> {
    let document =
        Document::open(resource_update_pdf(extra_fonts, widget_da, true)).expect("valid PDF");
    assert!(!document.was_recovered(), "the fixture's xref is exact");
    let mut view = ViewState::of(&document);
    assert_eq!(
        view.set_field(&document, "field", &Entered::Text("new value".to_owned())),
        1,
        "the complete form has one editable widget"
    );
    let before_page = pdf_model::Pages::new(&document)
        .get(0)
        .expect("source page");
    let before =
        pdf_model::content::interpret_with(&document, &before_page, &ViewState::of(&document));
    assert!(before.is_complete(), "{:?}", before.unsupported);
    assert!(
        before.text.contains("old"),
        "stored appearance agrees with original V"
    );
    let written = view.save(&document).expect("the edit can be written");
    assert!(
        written.unconstructed.is_empty(),
        "the appearance must be fully written"
    );
    let bytes = written.bytes;
    let saved = Document::open(bytes).expect("the saved update reopens");
    assert!(!saved.was_recovered(), "the writer emits a valid xref");
    let page = pdf_model::Pages::new(&saved).get(0).expect("saved page");
    let interpreted = pdf_model::content::interpret_with(&saved, &page, &ViewState::of(&saved));
    assert!(interpreted.is_complete(), "{:?}", interpreted.unsupported);
    assert!(
        interpreted.text.contains("new value"),
        "new value survives appearance save"
    );
    assert!(
        !interpreted.text.contains("old"),
        "the old Tx content is replaced"
    );
    let widget = saved
        .get(ObjectId {
            number: 5,
            generation: 0,
        })
        .as_dict()
        .cloned()
        .expect("the widget object is still a dictionary");
    let appearances = saved
        .get_key(&widget, "AP")
        .as_dict()
        .cloned()
        .expect("the widget retains an appearance dictionary");
    let normal = saved
        .get_key(&appearances, "N")
        .as_stream()
        .cloned()
        .expect("the normal appearance remains a stream");
    let content = saved
        .decoded_stream_data(&normal)
        .expect("saved appearance decodes");
    assert!(
        content
            .windows(b"/Helv".len())
            .any(|bytes| bytes == b"/Helv"),
        "the DA resource is actually used, not merely left unused in Resources"
    );
    let resources = saved
        .get_key(&normal.dict, "Resources")
        .as_dict()
        .cloned()
        .expect("the updated stream has resources");
    let fonts = saved
        .get_key(&resources, "Font")
        .as_dict()
        .cloned()
        .expect("the stream has a font resource category");
    let helv = saved
        .get_key(&fonts, "Helv")
        .as_dict()
        .cloned()
        .expect("the DA's font resource is present after regeneration");
    saved
        .get_key(&helv, "BaseFont")
        .as_name()
        .expect("the font states a BaseFont")
        .as_bytes()
        .to_vec()
}

#[test]
fn updating_a_widget_copies_missing_dr_font_but_keeps_same_named_stream_font() {
    // Positive control: /DR's /Helv is copied when the old AP has no /Helv entry.
    assert_eq!(
        saved_font_name("", true),
        b"Helvetica",
        "the regenerated /Tx uses a font made available by /DR"
    );

    // Discriminator: when both dictionaries name /Helv, the existing stream resource wins.
    // Replacing the entire /Font category with /DR's dictionary would incorrectly return Helvetica.
    assert_eq!(
        saved_font_name("/Helv 8 0 R", true),
        b"Courier",
        "§12.7.4.3 preserves the existing stream resource for a colliding name"
    );

    // Inheritance control: with no widget-level DA, Table 228 lets the AcroForm DA supply it.
    assert_eq!(
        saved_font_name("", false),
        b"Helvetica",
        "the public edit/save path resolves the inherited AcroForm DA"
    );
}

#[test]
fn a_new_variable_text_appearance_uses_origin_and_widget_dimensions() {
    let document =
        Document::open(resource_update_pdf("", false, false)).expect("PDF1.7 with NeedAppearances");
    assert!(!document.was_recovered());
    let mut view = ViewState::of(&document);
    assert_eq!(
        view.set_field(&document, "field", &Entered::Text("new value".to_owned())),
        1
    );
    let written = view.save(&document).expect("save new appearance");
    assert!(written.unconstructed.is_empty());
    let saved = Document::open(written.bytes).expect("saved PDF");
    assert!(!saved.was_recovered());
    let widget_object = saved.get(ObjectId {
        number: 5,
        generation: 0,
    });
    let widget = widget_object.as_dict().expect("widget");
    let ap_object = saved.get_key(widget, "AP");
    let ap = ap_object.as_dict().expect("appearance dictionary");
    assert!(
        ap.get("N")
            .and_then(pdf_syntax::Object::as_reference)
            .is_some(),
        "new stream is indirect"
    );
    let normal_object = saved.get_key(ap, "N");
    let normal = normal_object.as_stream().expect("Form stream");
    let bounds_object = saved.get_key(&normal.dict, "BBox");
    let bounds = bounds_object.as_array().expect("BBox array");
    let bounds: Vec<_> = bounds.iter().map(pdf_syntax::Object::as_number).collect();
    assert!(
        normal
            .dict
            .get("Matrix")
            .is_none_or(|value| value.as_array().is_some_and(|values| values
                .iter()
                .map(pdf_syntax::Object::as_number)
                .collect::<Vec<_>>()
                == [
                    Some(1.0),
                    Some(0.0),
                    Some(0.0),
                    Some(1.0),
                    Some(0.0),
                    Some(0.0)
                ])),
        "identity default or explicit identity"
    );
    assert!(
        normal
            .dict
            .get("FormType")
            .is_none_or(|value| value.as_integer() == Some(1)),
        "FormType default1"
    );
    assert_eq!(
        saved
            .get_key(&normal.dict, "Subtype")
            .as_name()
            .map(|name| name.as_bytes().to_vec()),
        Some(b"Form".to_vec())
    );
    let page = pdf_model::Pages::new(&saved).get(0).expect("saved page");
    let interpreted = pdf_model::content::interpret_with(&saved, &page, &ViewState::of(&saved));
    assert!(interpreted.is_complete(), "{:?}", interpreted.unsupported);
    assert!(interpreted.text.contains("new value"));
    assert_eq!(
        bounds,
        [Some(0.0), Some(0.0), Some(160.0), Some(30.0)],
        "Rect20,40,180,70 becomes local width160,height30"
    );
}

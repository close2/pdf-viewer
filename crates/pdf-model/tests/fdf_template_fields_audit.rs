//! Regression: Table 252's `/Rename false` update-to-all-fields behavior.
#![expect(
    clippy::expect_used,
    reason = "valid fixtures and matching named pages are test preconditions"
)]

use std::fmt::Write as _;

use pdf_model::{Pages, content, forms_data::FormsData, view::ViewState};
use pdf_syntax::{Document, FileBytes};

fn file(header: &str, root: &str, objects: &[String]) -> Vec<u8> {
    let mut out = String::from(header);
    let mut offsets = Vec::new();
    for (index, object) in objects.iter().enumerate() {
        offsets.push(out.len());
        let number = index.saturating_add(1);
        let _ = writeln!(out, "{number} 0 obj\n{object}\nendobj");
    }
    let xref_at = out.len();
    let size = objects.len().saturating_add(1);
    let _ = writeln!(out, "xref\n0 {size}\n0000000000 65535 f ");
    for offset in offsets {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    let _ = write!(
        out,
        "trailer\n<< /Size {size} /Root {root} /ID [<30313233343536373839616263646566> <66656463626139383736353433323130>] >>\nstartxref\n{xref_at}\n%%EOF\n"
    );
    out.into_bytes()
}

fn target() -> Document {
    let objects = vec![
        "<< /Type /Catalog /Pages 2 0 R /AcroForm << /Fields [4 0 R 5 0 R] /NeedAppearances true /DA (/Helv 10 Tf 0 g) /DR << /Font << /Helv 6 0 R >> >> >> /Names << /Templates << /Names [(stamp) 7 0 R] >> >> >>".to_owned(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 240 120] /Resources << >> /Annots [4 0 R 5 0 R] >>".to_owned(),
        "<< /Type /Annot /Subtype /Widget /Rect [10 60 220 100] /F 4 /FT /Tx /T (duplicate) /V (shared-old) /DA (/Helv 10 Tf 0 g) >>".to_owned(),
        "<< /Type /Annot /Subtype /Widget /Rect [10 10 220 50] /F 4 /FT /Tx /T (duplicate) /V (shared-old) /DA (/Helv 10 Tf 0 g) >>".to_owned(),
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>".to_owned(),
        "<< /Type /Template /MediaBox [0 0 100 100] /Resources << >> >>".to_owned(),
    ];
    let document = Document::open(FileBytes::from(file("%PDF-1.7\n", "1 0 R", &objects)))
        .expect("complete target PDF");
    assert!(!document.was_recovered(), "exact target xref");
    document
}

fn fdf(rename: Option<bool>) -> Document {
    let rename = rename.map_or_else(String::new, |value| format!("/Rename {value}"));
    // Table 252 says /Fields is an array of references to FDF field dictionaries. The field is
    // therefore a separate indirect object, not a direct dictionary in this fixture.
    let objects = vec![
        format!(
            "<< /Version /1.3 /FDF << /Pages [<< /Templates [<< /TRef << /Name (stamp) >> {rename} /Fields [2 0 R] >>] >>] >> >>"
        ),
        "<< /T (duplicate) /V (from-template) >>".to_owned(),
    ];
    let document = Document::open(FileBytes::from(file("%FDF-1.3\n", "1 0 R", &objects)))
        .expect("complete FDF");
    assert!(!document.was_recovered(), "exact FDF xref");
    document
}

/// Two distinct fields with the same fully qualified name make the Table 252 false-branch
/// sentence observable. §12.7.4.2 requires such fields to agree on FT, V, and DV; this fixture
/// gives both fields the same type and initial value before applying one imported value to both.
#[test]
fn a_template_field_updates_every_target_field_with_the_same_name() {
    let target = target();
    let fdf = fdf(Some(false));
    let data = FormsData::read(&fdf).expect("FDF page and referenced field dictionary");
    assert_eq!(data.pages.len(), 1);
    assert_eq!(data.pages[0].templates.len(), 1);
    assert_eq!(data.pages[0].templates[0].fields.len(), 1);

    let source_page = Pages::new(&target).get(0).expect("source page");
    let baseline = pdf_model::interpret(&target, &source_page);
    assert!(baseline.is_complete(), "{:?}", baseline.unsupported);
    assert_eq!(
        baseline.text.matches("shared-old").count(),
        2,
        "both originals are visible: {:?}",
        baseline.text
    );
    let mut view = ViewState::of(&target);
    let outcome = view.import(&target, &data);
    assert_eq!(outcome.pages, 1);
    assert_eq!(outcome.widgets, 2, "both same-named target fields matched");
    assert!(outcome.refused.is_empty(), "{:?}", outcome.refused);
    assert!(outcome.unmatched.is_empty(), "{:?}", outcome.unmatched);

    let page = Pages::new(&target)
        .get(0)
        .expect("the displayed source page");
    let interpreted = content::interpret_with(&target, &page, &view);
    assert!(interpreted.is_complete(), "{:?}", interpreted.unsupported);
    let text = interpreted.text;
    assert_eq!(
        text.matches("from-template").count(),
        2,
        "the FDF field's attributes reached every field named duplicate: {text:?}"
    );
    assert!(!text.contains("shared-old"), "{text:?}");
}

/// The true branch is a reported implementation departure. Valid indirect field references
/// exercise both its default and explicit spelling without certifying unique-name import.
#[test]
fn default_and_explicit_rename_true_report_the_departure_without_overwriting_fields() {
    for rename in [None, Some(true)] {
        let target = target();
        let fdf = fdf(rename);
        let data = FormsData::read(&fdf).expect("valid template field references");
        assert_eq!(data.pages.len(), 1);
        assert_eq!(data.pages[0].templates.len(), 1);
        assert!(data.pages[0].templates[0].rename);
        assert_eq!(data.pages[0].templates[0].fields.len(), 1);
        let mut view = ViewState::of(&target);
        let outcome = view.import(&target, &data);
        assert_eq!(outcome.pages, 1);
        assert_eq!(outcome.widgets, 0);
        assert_eq!(outcome.refused.len(), 1);
        assert!(outcome.refused[0].contains("/Rename"));
        let page = Pages::new(&target).get(0).expect("target page");
        let interpreted = content::interpret_with(&target, &page, &view);
        assert!(interpreted.is_complete(), "{:?}", interpreted.unsupported);
        assert_eq!(interpreted.text.matches("shared-old").count(), 2);
        assert!(!interpreted.text.contains("from-template"));
    }
}

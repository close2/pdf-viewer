//! A choice field's selection held for its commit (ISO 32000-2 §12.7.5.4; ADR 2816).

#![expect(
    clippy::expect_used,
    reason = "complete fixtures whose setup failing is the test failing"
)]

use std::fmt::Write as _;

use pdf_model::form::{Control, fields};
use pdf_model::view::{Committed, Entered, ViewState};
use pdf_syntax::Document;

fn pdf_with_field(field: &str) -> Document {
    let objects = [
        "<< /Type /Catalog /Pages 2 0 R /AcroForm << /Fields [4 0 R] /NeedAppearances true \
         /DA (/Helv 12 Tf 0 g) /DR << /Font << /Helv 6 0 R >> >> >> >>"
            .to_owned(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 300 200] /Resources << >> \
         /Contents 5 0 R /Annots [4 0 R] >>"
            .to_owned(),
        field.to_owned(),
        "<< /Length 0 >>\nstream\n\nendstream".to_owned(),
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_owned(),
    ];
    let mut out = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for (index, object) in objects.iter().enumerate() {
        offsets.push(out.len());
        let _ = writeln!(out, "{} 0 obj\n{object}\nendobj", index.saturating_add(1));
    }
    let xref = out.len();
    let _ = writeln!(out, "xref\n0 7\n0000000000 65535 f ");
    for offset in offsets {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    let _ = write!(
        out,
        "trailer\n<< /Size 7 /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n"
    );
    let document = Document::open(out.into_bytes()).expect("complete field fixture");
    assert!(!document.was_recovered(), "xref offsets are exact");
    document
}

fn shown(document: &Document, view: &ViewState) -> String {
    let page = pdf_model::Pages::new(document).get(0).expect("one page");
    let fields = fields(document, &page, view);
    let field = fields.into_iter().next().expect("one field");
    if let Some(value) = field.value {
        return value.text;
    }
    match field.control {
        Control::Choice(choice) => choice
            .selected
            .first()
            .map(|index| choice.options[*index].label.clone())
            .unwrap_or_default(),
        _ => String::new(),
    }
}

/// Table 233 bit 27: "If clear, the new value is not committed until the user exits the field."
/// A selection is held for its commit, so a commit has something to commit, and the field's `/V`
/// validation, refusing the option, puts the earlier selection back.
#[test]
fn a_selection_waits_for_its_commit_and_a_refused_commit_puts_the_old_one_back() {
    let document = pdf_with_field(
        "<< /Type /Annot /Subtype /Widget /Rect [10 10 290 50] /FT /Ch /Ff 131072 /T (pick) \
         /Opt [(5) (7) (1)] /V (5) /DA (/Helv 12 Tf 0 g) \
         /AA << /V << /S /JavaScript /JS (AFRange_Validate\\(true, 5, true, 10\\)) >> >> >>",
    );
    let mut view = ViewState::of(&document);
    view.set_field(&document, "pick", &Entered::Chosen(vec![2]));
    assert_eq!(shown(&document, &view), "1", "the selection is shown");
    assert!(
        matches!(view.commit_field(&document, "pick"), Committed::Refused(_)),
        "the selection was held for its commit, and the commit ran /V"
    );
    assert_eq!(shown(&document, &view), "5");

    view.set_field(&document, "pick", &Entered::Chosen(vec![1]));
    assert_eq!(view.commit_field(&document, "pick"), Committed::Accepted);
    assert_eq!(shown(&document, &view), "7");
}

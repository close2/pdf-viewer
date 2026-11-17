//! Table 226 mapping names apply when exporting field data. Table 240 bit 3
//! chooses HTML form encoding; a mapping name must replace the field identity in
//! that body. Field selection itself continues to use the qualified field name.
#![expect(
    clippy::expect_used,
    clippy::panic,
    clippy::arithmetic_side_effects,
    reason = "small exact-xref fixture"
)]

use std::fmt::Write as _;

use pdf_model::action::{Action, SubmitForm};
use pdf_model::submission::{Format, compose};
use pdf_model::view::ViewState;
use pdf_syntax::{Document, FileBytes, Object, ObjectId};

fn pdf(mapping: Option<&str>) -> Vec<u8> {
    let tm = mapping.map_or_else(String::new, |name| format!("/TM ({name}) "));
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R /AcroForm << /Fields [4 0 R] /NeedAppearances true /DA (/Helv 12 Tf 0 g) /DR << /Font << /Helv 6 0 R >> >> >> >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] /Resources << >> /Annots [4 0 R] >>\nendobj\n\
         4 0 obj\n<< /Type /Annot /Subtype /Widget /Rect [10 10 90 30] /F 4 /FT /Tx /T (person) {tm}/V (Ada) /A 5 0 R >>\nendobj\n\
         5 0 obj\n<< /S /SubmitForm /F << /FS /URL /F (https://example.invalid/submit) >> /Fields [(person)] /Flags 4 >>\nendobj\n\
         6 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>\nendobj\n"
    );
    let mut out = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for object in body.split_inclusive("endobj\n") {
        offsets.push(out.len());
        out.push_str(object);
    }
    let xref_at = out.len();
    let size = offsets.len() + 1;
    let _ = write!(out, "xref\n0 {size}\n0000000000 65535 f \n");
    for offset in offsets {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    let _ = write!(
        out,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n"
    );
    out.into_bytes()
}

fn submit(document: &Document) -> SubmitForm {
    let actions = pdf_model::action::read(document, &Object::Reference(ObjectId::new(5, 0)));
    let [Action::SubmitForm(submit)] = actions.as_slice() else {
        panic!("the action is parsed as SubmitForm: {actions:?}");
    };
    submit.clone()
}

fn html_body(document: &Document) -> String {
    let submission = compose(document, &ViewState::of(document), &submit(document), None)
        .expect("HTML composition succeeds");
    assert_eq!(submission.format, Format::HtmlForm);
    String::from_utf8(submission.body).expect("ASCII URL-encoded form body")
}

#[test]
fn html_export_uses_mapping_name_while_field_selection_uses_field_identity() {
    let unmapped = Document::open(FileBytes::from(pdf(None))).expect("valid control PDF");
    let mapped = Document::open(FileBytes::from(pdf(Some("exportkey")))).expect("valid mapped PDF");
    assert!(!mapped.was_recovered() && !unmapped.was_recovered());
    assert_eq!(
        html_body(&unmapped),
        "person=Ada",
        "absent mapping uses field identity"
    );
    assert_eq!(
        html_body(&mapped),
        "exportkey=Ada",
        "the action selects person but the table-defined export name is exportkey"
    );
}

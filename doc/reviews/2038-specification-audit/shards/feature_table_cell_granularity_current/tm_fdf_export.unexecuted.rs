//! Scratch candidate only; parent should confirm Table 226 `/TM` applies to FDF export names
//! before promotion. Exercises public `compose` and parses the result as FDF. No Cargo run here.
#![expect(clippy::expect_used, clippy::panic, reason = "small exact-xref fixture")]

use std::fmt::Write as _;

use pdf_model::action::{Action, SubmitForm};
use pdf_model::forms_data::FormsData;
use pdf_model::submission::{Format, compose};
use pdf_model::view::ViewState;
use pdf_syntax::{Document, FileBytes, Object, ObjectId};

fn pdf(mapping: Option<&str>) -> Vec<u8> {
    let tm = mapping.map_or_else(String::new, |name| format!("/TM ({name}) "));
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R /AcroForm << /Fields [4 0 R] >> >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] /Resources << >> /Annots [4 0 R] >>\nendobj\n\
         4 0 obj\n<< /Type /Annot /Subtype /Widget /Rect [10 10 90 30] /F 4 /FT /Tx /T (person) {tm}/V (Ada) /A 5 0 R >>\nendobj\n\
         5 0 obj\n<< /S /SubmitForm /F << /FS /URL /F (https://example.invalid/submit) >> >>\nendobj\n"
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
    let _ = write!(out, "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n");
    out.into_bytes()
}

fn submit(document: &Document) -> SubmitForm {
    let actions = pdf_model::action::read(document, &Object::Reference(ObjectId::new(5, 0)));
    let [Action::SubmitForm(submit)] = actions.as_slice() else {
        panic!("the action is parsed as SubmitForm: {actions:?}");
    };
    submit.clone()
}

fn exported_names(document: &Document) -> Vec<String> {
    let submission = compose(document, &ViewState::of(document), &submit(document), None)
        .expect("FDF composition succeeds");
    assert_eq!(submission.format, Format::Fdf);
    let fdf = Document::open(FileBytes::from(submission.body)).expect("composed FDF parses");
    assert!(!fdf.was_recovered(), "writer output uses an exact xref");
    FormsData::read(&fdf)
        .expect("FDF field tree is readable")
        .fields
        .into_iter()
        .map(|field| field.name)
        .collect()
}

#[test]
fn a_mapping_name_is_used_for_form_data_export_and_absence_keeps_the_field_name() {
    let mapped = Document::open(FileBytes::from(pdf(Some("export-key"))))
        .expect("valid mapped PDF");
    let unmapped = Document::open(FileBytes::from(pdf(None))).expect("valid control PDF");
    assert!(!mapped.was_recovered() && !unmapped.was_recovered());
    assert_eq!(exported_names(&mapped), ["export-key"]);
    assert_eq!(exported_names(&unmapped), ["person"]);
}

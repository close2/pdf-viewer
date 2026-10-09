//! ISO 32000-2 §12.7's four statements about a field's value, and which of them stands.
//!
//! A field's `/V` (Table 226), §12.7.6.3's reset to `/DV`, §12.7.8.3.2's import and what a person
//! typed all answer one question about a widget, and the one performed last is the answer. The
//! reset "shall set the value of the V entry in the field dictionary to that of the DV entry"
//! whatever the value was, and the import makes "the values of the entries in the FDF field
//! dictionary ... replace those of the corresponding entries in the field", so neither may leave
//! a person's earlier edit standing over it. ADR 1796.

#![expect(
    clippy::expect_used,
    reason = "a test's failure is its purpose, and these helpers run outside #[test] bodies \
              where `allow-panic-in-tests` does not reach"
)]

use std::fmt::Write as _;

use pdf_model::action::{Action, ResetForm};
use pdf_model::forms_data::FormsData;
use pdf_model::view::{Entered, ViewState};
use pdf_syntax::Document;

/// A one-page form with a text field, a check box, and a `/DR` naming `/Helv`.
///
/// The text field's own `/V` is `stored` and its `/DV` is `factory`, so the three statements
/// §12.7.4.3 can lay out — the file's value, §12.7.6.3's default and §12.7.8's import — are all
/// distinguishable in the readback.
fn form() -> Vec<u8> {
    let body = "1 0 obj\n<< /Type /Catalog /Pages 2 0 R /AcroForm \
         << /Fields [5 0 R 6 0 R] /DR << /Font << /Helv 7 0 R >> >> >> >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 100] \
         /Resources << >> /Contents 4 0 R /Annots [5 0 R 6 0 R] >>\nendobj\n\
         4 0 obj\n<< /Length 0 >>\nstream\n\nendstream\nendobj\n\
         5 0 obj\n<< /Type /Annot /Subtype /Widget /Rect [20 40 180 70] /F 4 /FT /Tx \
         /T (name) /V (stored) /DV (factory) /DA (/Helv 12 Tf 0 g) >>\nendobj\n\
         6 0 obj\n<< /Type /Annot /Subtype /Widget /Rect [20 10 40 30] /F 4 /FT /Btn \
         /T (agree) /V /Off /AS /Off /MK << /BG [0.9] /CA (4) >> /DA (/Helv 0 Tf 0 g) >>\nendobj\n\
         7 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica \
         /Encoding /WinAnsiEncoding >>\nendobj\n"
        .to_owned();

    let mut out = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for object in body.split_inclusive("endobj\n") {
        offsets.push(out.len());
        out.push_str(object);
    }
    let xref_at = out.len();
    let size = offsets.len().saturating_add(1);
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

/// An FDF file holding one catalog dictionary, opened by the PDF reader.
fn fdf(fdf_dictionary: &str) -> Document {
    let bytes = format!(
        "%FDF-1.2\n1 0 obj\n<< /FDF {fdf_dictionary} >>\nendobj\ntrailer\n<< /Root 1 0 R >>\n%%EOF\n"
    );
    Document::open(bytes.into_bytes()).expect("an FDF file is opened by the PDF reader")
}

/// What page one draws, as the text readback and the reports beside it.
fn drawn(document: &Document, view: &ViewState) -> (String, Vec<String>) {
    let pages = pdf_model::Pages::new(document);
    let page = pages.get(0).expect("page one");
    let interpretation = pdf_model::content::interpret_with(document, &page, view);
    (
        interpretation.text.clone(),
        interpretation
            .unsupported
            .iter()
            .map(|item| format!("{item:?}"))
            .collect(),
    )
}

/// Table 241 with no `/Fields`: "all fields in the document's interactive form are reset".
fn reset_everything() -> Action {
    Action::ResetForm(ResetForm {
        fields: Vec::new(),
        exclude: false,
    })
}

/// A reset performed after a person typed puts the field's `/DV` back.
#[test]
fn a_reset_after_typing_draws_the_default_value() {
    let document = Document::open(form()).expect("the fixture is a valid PDF");
    let mut view = ViewState::of(&document);
    assert_eq!(
        view.set_field(&document, "name", &Entered::Text("typed".to_owned())),
        1
    );
    assert!(drawn(&document, &view).0.contains("typed"));

    view.perform(&document, &reset_everything());
    let (after, _) = drawn(&document, &view);
    assert!(after.contains("factory"), "the /DV: {after:?}");
    assert!(!after.contains("typed"), "{after:?}");
    assert!(
        view.edits().next().is_none(),
        "the reset is the later statement, so nothing a person typed is left to save"
    );
}

/// The same reset, named by the field: Table 242's clear flag resets the fields listed.
#[test]
fn a_reset_naming_the_field_after_typing_draws_the_default_value() {
    let document = Document::open(form()).expect("the fixture is a valid PDF");
    let mut view = ViewState::of(&document);
    view.set_field(&document, "name", &Entered::Text("typed".to_owned()));
    view.set_field(&document, "agree", &Entered::Text("Yes".to_owned()));
    view.perform(
        &document,
        &Action::ResetForm(ResetForm {
            fields: vec![pdf_model::action::ResetTarget::Name("name".to_owned())],
            exclude: false,
        }),
    );
    let (after, _) = drawn(&document, &view);
    assert!(after.contains("factory"), "{after:?}");
    assert_eq!(
        view.edits().count(),
        1,
        "the field the action did not name keeps what the person did"
    );
}

/// An import performed after a person typed replaces what they typed.
#[test]
fn an_import_after_typing_draws_the_imported_value() {
    let document = Document::open(form()).expect("the fixture is a valid PDF");
    let mut view = ViewState::of(&document);
    view.set_field(&document, "name", &Entered::Text("typed".to_owned()));
    let data = FormsData::read(&fdf("<< /Fields [ << /T (name) /V (imported) >> ] >>"))
        .expect("an FDF catalog");
    assert_eq!(view.import(&document, &data).widgets, 1);
    let (after, _) = drawn(&document, &view);
    assert!(after.contains("imported"), "{after:?}");
    assert!(!after.contains("typed"), "{after:?}");
}

/// The control: what a person types after a reset or an import is the later statement, and stands.
#[test]
fn typing_after_a_reset_or_an_import_draws_what_was_typed() {
    let document = Document::open(form()).expect("the fixture is a valid PDF");
    let mut view = ViewState::of(&document);
    view.perform(&document, &reset_everything());
    view.set_field(&document, "name", &Entered::Text("typed".to_owned()));
    assert!(drawn(&document, &view).0.contains("typed"));

    let mut view = ViewState::of(&document);
    let data = FormsData::read(&fdf("<< /Fields [ << /T (name) /V (imported) >> ] >>"))
        .expect("an FDF catalog");
    view.import(&document, &data);
    view.set_field(&document, "name", &Entered::Text("typed".to_owned()));
    assert!(drawn(&document, &view).0.contains("typed"));
}

/// A save after a reset writes the defaults, so the file opens again showing what the viewer
/// showed: §12.7.6.3 "shall set the value of the V entry in the field dictionary to that of the DV
/// entry", and §7.5.6's update is where this program sets an entry.
#[test]
fn a_saved_reset_opens_again_showing_the_default_value() {
    let document = Document::open(form()).expect("the fixture is a valid PDF");
    let mut view = ViewState::of(&document);
    view.set_field(&document, "name", &Entered::Text("typed".to_owned()));
    view.perform(&document, &reset_everything());
    assert!(drawn(&document, &view).0.contains("factory"));

    let written = view.save(&document).expect("the fixture saves");
    let reopened =
        Document::open(pdf_syntax::FileBytes::from(written.bytes)).expect("the update reopens");
    let (after, _) = drawn(&reopened, &ViewState::of(&reopened));
    assert!(after.contains("factory"), "{after:?}");
    assert!(
        !after.contains("stored") && !after.contains("typed"),
        "{after:?}"
    );
}

/// The control: a reset that reaches a field already at its default writes nothing for it.
///
/// The fixture with the text field's `/DV` made its `/V`, padded to the same length so that every
/// cross-reference offset still holds.
#[test]
fn a_reset_that_changes_no_value_writes_nothing() {
    let at_default = String::from_utf8(form())
        .expect("the fixture is ASCII")
        .replace("/V (stored) /DV (factory)", "/V (stored) /DV (stored) ");
    let document = Document::open(at_default.into_bytes()).expect("the fixture is a valid PDF");
    let untouched = ViewState::of(&document)
        .save(&document)
        .expect("the fixture saves")
        .bytes;
    let mut view = ViewState::of(&document);
    view.perform(
        &document,
        &Action::ResetForm(ResetForm {
            fields: vec![pdf_model::action::ResetTarget::Name("name".to_owned())],
            exclude: false,
        }),
    );
    assert!(
        view.is_reset(pdf_syntax::ObjectId::new(5, 0)),
        "the reset reached the field"
    );
    assert_eq!(view.save(&document).expect("saves").bytes, untouched);
}

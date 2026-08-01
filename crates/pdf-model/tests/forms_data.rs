//! ISO 32000-2 §12.7.8's imported form data, checked by what the page then says.
//!
//! `crates/pdf-model/src/forms_data.rs`'s own tests check what an FDF file *contains*. These
//! check the only thing that makes reading one a renderer's business: that an imported value
//! reaches §12.7.4.3's layout and changes the ink. The instrument is
//! `Interpretation::text` — the readback of what was actually drawn, accumulated by the same
//! loop that places the glyphs — because it distinguishes "the new value was laid out" from
//! "something was laid out", which counting pixels cannot.
//!
//! Trap 8 is why these are synthetic: not one of the 974 corpus documents carries an
//! import-data action, and none is shipped with an FDF file. Nothing else defends these rules.

#![expect(
    clippy::expect_used,
    reason = "a test's failure is its purpose, and these helpers run outside #[test] bodies \
              where `allow-panic-in-tests` does not reach"
)]

use std::fmt::Write as _;

use pdf_model::forms_data::FormsData;
use pdf_model::view::ViewState;
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

/// An FDF file with the header §12.7.8.2.2 states, the trailer §12.7.8.2.4 does, and no
/// cross-reference table — which §12.7.8.1 makes optional and which is how they are written.
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

/// §12.7.8.3.2's one sentence, end to end: "importing a field causes the values of the entries
/// in the FDF field dictionary to replace those of the corresponding entries in the field with
/// the same fully qualified name in the target document" — and the value that replaces `/V` is
/// what §12.7.4.3 lays out.
#[test]
fn an_imported_value_is_the_one_that_is_drawn() {
    let document = Document::open(form()).expect("the fixture is a valid PDF");
    let mut view = ViewState::of(&document);
    let (before, reports) = drawn(&document, &view);
    assert!(before.contains("stored"), "the file's own /V: {before:?}");
    assert!(reports.is_empty(), "{reports:?}");

    let data = FormsData::read(&fdf("<< /Fields [ << /T (name) /V (imported) >> ] >>"))
        .expect("an FDF catalog");
    let outcome = view.import(&document, &data);
    assert_eq!(outcome.widgets, 1);
    assert!(outcome.unmatched.is_empty(), "{:?}", outcome.unmatched);

    let (after, reports) = drawn(&document, &view);
    assert!(after.contains("imported"), "{after:?}");
    assert!(!after.contains("stored"), "replaced, not added: {after:?}");
    assert!(reports.is_empty(), "{reports:?}");
}

/// An FDF field with no `/V` at all still *replaces*, so the widget is left with no value —
/// the same state §12.7.6.3's reset leaves a field with no `/DV` in, and drawn the same way.
#[test]
fn an_imported_field_with_no_value_empties_the_one_it_names() {
    let document = Document::open(form()).expect("the fixture is a valid PDF");
    let mut view = ViewState::of(&document);
    let data = FormsData::read(&fdf("<< /Fields [ << /T (name) >> ] >>")).expect("an FDF catalog");
    assert_eq!(view.import(&document, &data).widgets, 1);

    let (after, reports) = drawn(&document, &view);
    assert!(!after.contains("stored"), "{after:?}");
    assert!(!after.contains("factory"), "not the /DV either: {after:?}");
    assert!(reports.is_empty(), "{reports:?}");
}

/// The two clauses that replace a field's value are two statements about the same thing, so
/// the later one stands alone. A reset after an import gives the document's `/DV` back.
#[test]
fn a_reset_after_an_import_takes_the_documents_own_default() {
    let document = Document::open(form()).expect("the fixture is a valid PDF");
    let mut view = ViewState::of(&document);
    let data = FormsData::read(&fdf("<< /Fields [ << /T (name) /V (imported) >> ] >>"))
        .expect("an FDF catalog");
    view.import(&document, &data);
    assert!(drawn(&document, &view).0.contains("imported"));

    // Table 241 with no `/Fields`: "all fields in the document's interactive form are reset".
    let reset = pdf_model::action::Action::ResetForm(pdf_model::action::ResetForm {
        fields: Vec::new(),
        exclude: false,
    });
    view.perform(&document, &reset);
    let (after, _) = drawn(&document, &view);
    assert!(after.contains("factory"), "the /DV: {after:?}");
    assert!(!after.contains("imported"), "{after:?}");
}

/// §12.7.8.3.2 matches "the field with the same fully qualified name", so a name this form has
/// not got imports into nothing — and is reported rather than dropped, because a caller looking
/// at both files is the only one who can say whether it is the wrong FDF or a changed form.
#[test]
fn a_name_this_form_has_not_got_is_named_rather_than_dropped() {
    let document = Document::open(form()).expect("the fixture is a valid PDF");
    let mut view = ViewState::of(&document);
    let data = FormsData::read(&fdf(
        "<< /Fields [ << /T (name) /V (here) >> << /T (nowhere) /V (x) >> ] >>",
    ))
    .expect("an FDF catalog");
    let outcome = view.import(&document, &data);
    assert_eq!(outcome.widgets, 1);
    assert_eq!(outcome.unmatched, ["nowhere"]);
    assert!(drawn(&document, &view).0.contains("here"));
}

/// Table 249's `/F` "shall replace that of the F entry in the form's corresponding annotation
/// dictionary" — so an import can hide a widget, which §12.5.3's Hidden flag decides and which
/// is a display change with no value in it at all.
#[test]
fn an_imported_annotation_flag_hides_a_widget() {
    let document = Document::open(form()).expect("the fixture is a valid PDF");
    let mut view = ViewState::of(&document);
    assert!(drawn(&document, &view).0.contains("stored"));

    // Table 167 bit 2 is Hidden; bit 3, Print, is what the fixture's own `/F 4` sets.
    let data =
        FormsData::read(&fdf("<< /Fields [ << /T (name) /SetF 2 >> ] >>")).expect("an FDF catalog");
    view.import(&document, &data);
    let (after, reports) = drawn(&document, &view);
    assert!(!after.contains("stored"), "hidden: {after:?}");
    assert!(
        reports.is_empty(),
        "a hidden annotation is not a gap: {reports:?}"
    );
}

/// §12.7.5.2.3 makes `/AS` decide a check box's state, and §12.7.6.3's argument applies to an
/// import for the same reason: the `/AS` in the file describes the state the import replaced.
/// So an imported `/V` turns the box on even though the document's `/AS` says `Off`.
#[test]
fn an_imported_check_box_answers_from_its_new_value_rather_than_the_stored_as() {
    let document = Document::open(form()).expect("the fixture is a valid PDF");
    let mut view = ViewState::of(&document);
    let stated = view.annotation(pdf_syntax::ObjectId::new(6, 0));
    assert_eq!(stated.value, pdf_model::view::FieldValue::Stored);

    let data =
        FormsData::read(&fdf("<< /Fields [ << /T (agree) /V /On >> ] >>")).expect("an FDF catalog");
    assert_eq!(view.import(&document, &data).widgets, 1);
    let (after, reports) = drawn(&document, &view);
    // A check box with no appearance stream has its mark constructed, and §12.7.5.2.3's own
    // ZapfDingbats check is what `appearance.rs` draws — read back as the character it is.
    assert!(after.contains('4'), "the check mark's own code: {after:?}");
    assert!(reports.is_empty(), "{reports:?}");
}

/// §12.7.6.4's import-data action is read rather than refused, and what it carries is a file
/// name for a caller to resolve — never a path this crate opens.
#[test]
fn an_import_data_action_names_its_file_and_its_format() {
    let document = Document::open(form()).expect("the fixture is a valid PDF");
    let mut dictionary = pdf_syntax::Dictionary::new();
    dictionary.insert(
        pdf_syntax::Name::new(b"S".to_vec()),
        pdf_syntax::Object::Name(pdf_syntax::Name::new(b"ImportData".to_vec())),
    );
    dictionary.insert(
        pdf_syntax::Name::new(b"F".to_vec()),
        pdf_syntax::Object::String(b"answers.FDF".to_vec().into()),
    );
    let actions = pdf_model::action::read(&document, &pdf_syntax::Object::Dictionary(dictionary));
    let [pdf_model::action::Action::ImportData(import)] = actions.as_slice() else {
        panic!("one import-data action, got {actions:?}");
    };
    assert_eq!(import.file, "answers.FDF");
    assert_eq!(
        import.format,
        pdf_model::action::DataFormat::Fdf,
        "§12.7.8.1's extension, whatever case the exporting platform wrote it in"
    );
}

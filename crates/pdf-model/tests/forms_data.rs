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
use pdf_model::view::Entered;
use pdf_model::view::ViewState;
use pdf_syntax::Document;

/// The same builder, with the catalog's `/Names` dictionary and extra objects given.
///
/// §12.7.7's template page is object 8: outside the page tree, with no `/Parent` and no `/B`,
/// which is exactly what the clause requires of a page "not intended to be displayed".
fn form_with_a_template() -> Vec<u8> {
    let form = String::from_utf8(form()).expect("the fixture is ASCII");
    let form = form.replace(
        "/DR << /Font << /Helv 7 0 R >> >> >> >>",
        "/DR << /Font << /Helv 7 0 R >> >> >> /Names          << /Templates << /Names [(blank) 8 0 R] >> >> >>",
    );
    let template = "8 0 obj\n<< /Type /Template /MediaBox [0 0 300 150] /Resources << >>          /Contents 9 0 R >>\nendobj\n\
         9 0 obj\n<< /Length 0 >>\nstream\n\nendstream\nendobj\n";
    rebuild(&form, template)
}

/// Reassembles a fixture whose objects have changed length, keeping every object number.
fn rebuild(document: &str, extra: &str) -> Vec<u8> {
    let body: String = document
        .split_inclusive("endobj\n")
        .filter(|part| part.contains(" 0 obj"))
        .collect::<String>()
        + extra;

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

/// The same form, with a **stored appearance stream** on the text field and no
/// `/NeedAppearances`.
///
/// The one shape [`form`] cannot exercise: a widget with no `/AP` has its appearance
/// *constructed* from the value, so a replaced value is drawn by construction. A widget that
/// states one is the case where the file's own artwork could be shown instead, and where a
/// processor that only regenerated under Table 224's flag would draw the value the field had
/// before the import.
///
/// The stream is written the way §12.7.4.3 asks a writer to write one — the variable text inside
/// a `/Tx BMC` … `EMC` region — so that the splice has a region to replace.
fn form_with_appearance() -> Vec<u8> {
    let appearance = "/Tx BMC\nBT /Helv 12 Tf 0 g 2 8 Td (stored) Tj ET\nEMC\n";
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R /AcroForm \
         << /Fields [5 0 R] /DR << /Font << /Helv 7 0 R >> >> >> >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 100] \
         /Resources << >> /Contents 4 0 R /Annots [5 0 R] >>\nendobj\n\
         4 0 obj\n<< /Length 0 >>\nstream\n\nendstream\nendobj\n\
         5 0 obj\n<< /Type /Annot /Subtype /Widget /Rect [20 40 180 70] /F 4 /FT /Tx \
         /T (name) /V (stored) /DA (/Helv 12 Tf 0 g) /AP << /N 8 0 R >> >>\nendobj\n\
         6 0 obj\nnull\nendobj\n\
         7 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica \
         /Encoding /WinAnsiEncoding >>\nendobj\n\
         8 0 obj\n<< /Type /XObject /Subtype /Form /BBox [0 0 160 30] \
         /Resources << /Font << /Helv 7 0 R >> >> /Length {} >>\nstream\n\
         {appearance}endstream\nendobj\n",
        appearance.len()
    );

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

/// The same form with Table 227's `ReadOnly` bit set on its text field.
fn read_only_form() -> Vec<u8> {
    let form = String::from_utf8(form()).expect("the fixture is ASCII");
    // The fixture's own offsets stay valid because the replacement is the same length.
    let with_flag = form.replace("/T (name) /V (stored)", "/Ff 1 /T (name) /V (stored)");
    assert_ne!(with_flag, form, "the fixture states the field this edits");
    rebuilt(&with_flag)
}

/// Re-writes a fixture's cross-reference table after its objects have moved.
fn rebuilt(document: &str) -> Vec<u8> {
    let body = document
        .split_once("xref\n")
        .map_or(document, |(body, _)| body);
    let mut out = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for object in body
        .trim_start_matches("%PDF-1.7\n")
        .split_inclusive("endobj\n")
    {
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

/// §12.7.7 and §12.7.8.3.3 together: an FDF page names a template, the template names a page
/// this document already holds outside its page tree, and importing adds it.
#[test]
fn an_imported_template_adds_a_page_the_document_already_held() {
    let document = Document::open(form_with_a_template()).expect("the fixture is a valid PDF");
    assert_eq!(pdf_model::Pages::new(&document).len(), 1, "the page tree");

    let mut view = ViewState::of(&document);
    assert!(view.appended_pages().is_empty());

    let data = FormsData::read(&fdf(
        "<< /Pages [ << /Templates [ << /TRef << /Name (blank) >> >> ] >> ] >>",
    ))
    .expect("an FDF catalog");
    let outcome = view.import(&document, &data);
    assert_eq!(outcome.pages, 1);
    assert!(outcome.refused.is_empty(), "{:?}", outcome.refused);
    assert_eq!(view.appended_pages(), [pdf_syntax::ObjectId::new(8, 0)]);

    // §7.7.3.4's inheritance runs up `/Parent` and a template has none, so the page states its
    // own geometry and `Pages::detached` reads it from the dictionary alone.
    let pages = pdf_model::Pages::new(&document);
    let object = document.get(pdf_syntax::ObjectId::new(8, 0));
    let template = pages.detached(object.as_dict().expect("a page dictionary"));
    assert_eq!((template.width(), template.height()), (300.0, 150.0));
}

/// Two refusals, both named: a template in another file, and a name this document does not have.
#[test]
fn a_template_this_document_cannot_reach_is_named() {
    let document = Document::open(form_with_a_template()).expect("the fixture is a valid PDF");
    let mut view = ViewState::of(&document);
    let data = FormsData::read(&fdf("<< /Pages [ << /Templates [ \
         << /TRef << /Name (blank) /F (library.pdf) >> >> \
         << /TRef << /Name (absent) >> >> ] >> ] >>"))
    .expect("an FDF catalog");
    let outcome = view.import(&document, &data);
    assert_eq!(outcome.pages, 0);
    assert_eq!(outcome.refused.len(), 2, "{:?}", outcome.refused);
    assert!(outcome.refused[0].contains("library.pdf"));
    assert!(outcome.refused[1].contains("names no page absent"));
    assert!(view.appended_pages().is_empty());
}

/// A value this program replaced makes a *stored* appearance stale, whatever Table 224 says.
///
/// §12.7.2 obliges the file to keep its appearance "consistent with the object's current value
/// as a field", and the file kept it — the stream matches the `/V` the file states. What breaks
/// the promise is the import, and at that point drawing the stored stream would show a value the
/// field no longer has. This regenerated only under `/NeedAppearances` until the
/// hundred-and-thirty-fifth session, so an imported value went unseen on every document that
/// states an appearance and not that flag.
#[test]
fn a_replaced_value_is_drawn_even_where_the_file_stores_an_appearance() {
    let document = Document::open(form_with_appearance()).expect("the fixture is a valid PDF");
    let mut view = ViewState::of(&document);
    let (before, reports) = drawn(&document, &view);
    assert!(
        before.contains("stored"),
        "the file's own artwork: {before:?}"
    );
    assert!(reports.is_empty(), "{reports:?}");

    let data = FormsData::read(&fdf("<< /Fields [ << /T (name) /V (imported) >> ] >>"))
        .expect("an FDF catalog");
    assert_eq!(view.import(&document, &data).widgets, 1);

    let (after, reports) = drawn(&document, &view);
    assert!(after.contains("imported"), "{after:?}");
    assert!(
        !after.contains("stored"),
        "spliced, not added to: {after:?}"
    );
    assert!(reports.is_empty(), "{reports:?}");
}

/// Table 227's `ReadOnly` flag refuses a *person*, and not the document's own actions.
///
/// ISO 32000-2 §12.7.4.1, Table 227, bit 1:
///
/// > If set, an interactive PDF processor shall not allow a user to change the value of the
/// > field.
///
/// The distinction is the whole of the flag: §12.7.6.3's reset and §12.7.8's import are the
/// document changing its own value and neither is a user, so both still apply. Only
/// `ViewState::set_field` — what somebody typed — is refused.
#[test]
fn a_read_only_field_refuses_a_person_and_not_an_import() {
    let document = Document::open(read_only_form()).expect("the fixture is a valid PDF");
    let mut view = ViewState::of(&document);

    assert_eq!(
        view.set_field(&document, "name", &Entered::Text("typed".to_owned())),
        0
    );
    let (after, _) = drawn(&document, &view);
    assert!(!after.contains("typed"), "{after:?}");
    assert!(
        after.contains("stored"),
        "the file's own value stands: {after:?}"
    );

    let data = FormsData::read(&fdf("<< /Fields [ << /T (name) /V (imported) >> ] >>"))
        .expect("an FDF catalog");
    assert_eq!(view.import(&document, &data).widgets, 1);
    let (after, _) = drawn(&document, &view);
    assert!(after.contains("imported"), "{after:?}");
}

/// The same form with a signature field carrying §12.7.5.5's `/Lock`, signed or not.
///
/// `action` is Table 236's `/Action` verbatim, so a caller can hand it a name the table does not
/// define; `fields` is the `/Fields` array's contents, empty for `/All`. `signed` decides whether
/// the signature field states a `/V` — which is the clause's own condition, "after this signature
/// has been **signed**", and the only thing that separates a lock that binds from an instruction
/// to whatever will do the signing.
///
/// The signature dictionary is the smallest one `signature::read` accepts: a `/ByteRange` and
/// nothing else. It signs nothing and verifies as nothing, which is exactly right here — this is
/// a test of what a field lock *asserts*, and §12.8.1's three questions about the signature
/// itself are a different clause with its own fixtures.
fn form_locking(action: &str, fields: &[&str], signed: bool) -> Vec<u8> {
    let form = String::from_utf8(form()).expect("the fixture is ASCII");
    let with_field = form.replace("/Fields [5 0 R 6 0 R]", "/Fields [5 0 R 6 0 R 8 0 R]");
    assert_ne!(with_field, form, "the fixture states a field list");
    let mut names = String::new();
    for name in fields {
        let _ = write!(names, "({name}) ");
    }
    let value = if signed { "/V 9 0 R " } else { "" };
    let objects = format!(
        "8 0 obj\n<< /Type /Annot /Subtype /Widget /Rect [0 0 0 0] /F 4 /FT /Sig \
         /T (sig) {value}/Lock 10 0 R >>\nendobj\n\
         9 0 obj\n<< /Type /Sig /ByteRange [0 100 200 300] >>\nendobj\n\
         10 0 obj\n<< /Type /SigFieldLock /Action /{action} /Fields [{names}] >>\nendobj\n"
    );
    let body = with_field
        .split_once("xref\n")
        .map_or(with_field.as_str(), |(body, _)| body);
    rebuilt(&format!("{body}{objects}"))
}

/// §12.7.5.5's signature field lock, in all three of Table 236's actions and unsigned.
///
/// The clause states the prohibition in prose and the vocabulary in the table, and the two are
/// worded differently — the table's column says the fields "should be locked" and the sentence
/// under it, about the lock dictionary, is a `shall`:
///
/// > contains the names of form fields whose values shall no longer be changed after this
/// > signature has been signed.
///
/// **Nothing in the corpus states one**, which is trap 8's shape and why this is a hand-built
/// file: six of the 974 documents carry a signature and none of the six carries a `/Lock`. So the
/// fixture is the only witness there is, and each case below is one line of the table.
///
/// The last case is the clause's own condition rather than the table's, and it is the one a
/// reader is most likely to get wrong: an *unsigned* signature field with a `/Lock` locks
/// nothing. §12.7.5.5's NOTE 1 says what such a field is for — it "can also hold information
/// needed later when the actual signing takes place" — so the entry is an instruction to the
/// signer until there is a signature to have been signed.
#[test]
fn a_signed_signature_field_locks_the_fields_its_lock_names() {
    use pdf_model::restriction::{Operation, Restriction, asserted};

    let locked = vec![Restriction::FieldLocked];

    // Include: "All fields specified in Fields".
    let include = Document::open(form_locking("Include", &["name"], true)).expect("a valid PDF");
    assert_eq!(
        asserted(&include, Operation::FillInForm, Some("name"), None),
        locked
    );
    assert_eq!(
        asserted(&include, Operation::FillInForm, Some("agree"), None),
        Vec::new()
    );
    // The lock is about a field's *value*; §12.7.5.5 says nothing about annotating.
    assert_eq!(
        asserted(&include, Operation::Annotate, Some("name"), None),
        Vec::new()
    );

    // All: "All fields in the document".
    let all = Document::open(form_locking("All", &[], true)).expect("a valid PDF");
    for field in ["name", "agree", "sig"] {
        assert_eq!(
            asserted(&all, Operation::FillInForm, Some(field), None),
            locked,
            "{field} is a field in the document"
        );
    }

    // Exclude: "All fields except those specified in Fields".
    let exclude = Document::open(form_locking("Exclude", &["name"], true)).expect("a valid PDF");
    assert_eq!(
        asserted(&exclude, Operation::FillInForm, Some("name"), None),
        Vec::new()
    );
    assert_eq!(
        asserted(&exclude, Operation::FillInForm, Some("agree"), None),
        locked
    );

    // A name Table 236 does not define states nothing this clause defines, and a lock is not
    // guessed at: falling back to `All` would close a document on a word the standard does not
    // use.
    let unknown = Document::open(form_locking("Everything", &[], true)).expect("a valid PDF");
    assert_eq!(
        asserted(&unknown, Operation::FillInForm, Some("name"), None),
        Vec::new()
    );

    // And the condition the clause states: the same lock, on a field nobody has signed.
    let unsigned = Document::open(form_locking("All", &[], false)).expect("a valid PDF");
    assert_eq!(
        asserted(&unsigned, Operation::FillInForm, Some("name"), None),
        Vec::new(),
        "a /Lock binds after this signature has been signed, and this one has not"
    );
}

/// A form whose author certified the document with §12.8.2.2's `/P`.
///
/// The catalog gains §12.8.6's `/Perms /DocMDP`, pointing at a signature whose `/Reference`
/// names the `DocMDP` transform and states the level. Objects are appended, so the
/// cross-reference table is rebuilt.
fn certified_form(level: i64) -> Vec<u8> {
    let form = String::from_utf8(form()).expect("the fixture is ASCII");
    let with_perms = form.replace(
        "<< /Type /Catalog /Pages 2 0 R /AcroForm",
        "<< /Type /Catalog /Perms << /DocMDP 8 0 R >> /Pages 2 0 R /AcroForm",
    );
    assert_ne!(with_perms, form, "the fixture states a catalog");
    let signature = format!(
        "8 0 obj\n<< /Type /Sig /Reference [9 0 R] >>\nendobj\n\
         9 0 obj\n<< /Type /SigRef /TransformMethod /DocMDP \
         /TransformParams << /Type /TransformParams /P {level} /V /1.2 >> >>\nendobj\n"
    );
    let body = with_perms
        .split_once("xref\n")
        .map_or(with_perms.as_str(), |(body, _)| body);
    rebuilt(&format!("{body}{signature}"))
}

/// A form whose author attached §12.8.2.3's usage rights signature, granting `form` rights.
///
/// The `/UR3` sits in the catalog's `/Perms` as a *direct* dictionary here, which is the harder
/// of the two shapes `withdrawn_usage_rights` handles: a direct dictionary has no object number
/// of its own, so the catalog is what has to be rewritten.
fn form_with_usage_rights(form_rights: &str) -> Vec<u8> {
    let form = String::from_utf8(form()).expect("the fixture is ASCII");
    let with_perms = form.replace(
        "<< /Type /Catalog /Pages 2 0 R /AcroForm",
        "<< /Type /Catalog /Perms << /UR3 8 0 R >> /Pages 2 0 R /AcroForm",
    );
    assert_ne!(with_perms, form, "the fixture states a catalog");
    let signature = format!(
        "8 0 obj\n<< /Type /Sig /Reference [9 0 R] >>\nendobj\n\
         9 0 obj\n<< /Type /SigRef /TransformMethod /UR3 \
         /TransformParams << /Type /TransformParams /V /2.2 /P true \
         /Document [/FullSave] /Form [{form_rights}] >> >>\nendobj\n"
    );
    let body = with_perms
        .split_once("xref\n")
        .map_or(with_perms.as_str(), |(body, _)| body);
    rebuilt(&format!("{body}{signature}"))
}

/// §12.8.2.3: a save in excess of what a usage rights signature grants withdraws it.
///
/// > A PDF processor that modifies a PDF, with a UR signature in excess of the rights that are
/// > granted by that signature, should remove that signature prior to writing the newly modified
/// > PDF.
///
/// The two fixtures differ in one name — `/Form [/FillIn]` against `/Form [/Import]` — so the
/// same edit is inside the grant in one and outside it in the other, which is what makes this a
/// test of the clause rather than of a flag. Both state `/P true`, because Table 258's default
/// of `false` says "any possible restriction may be ignored" and a fixture at the default would
/// pass whatever the code did.
///
/// **What is removed is the `/Perms` entry, not the object.** `CLAUDE.md` permits only §7.5.6's
/// incremental update, so the signature's bytes stay where the producer put them; §12.8.6 makes
/// a usage rights signature the one "referred to from the UR3 entry in the permissions
/// dictionary", and after this save there is no such entry. ADR 0159.
#[test]
fn a_save_beyond_the_granted_usage_rights_withdraws_the_signature() {
    let granted = Document::open(form_with_usage_rights("/FillIn")).expect("a valid PDF");
    let mut view = ViewState::of(&granted);
    assert_eq!(
        view.set_field(&granted, "name", &Entered::Text("typed".to_owned())),
        1
    );
    let saved = view
        .save(&granted)
        .expect("the fixture can be updated")
        .bytes;
    let reopened = Document::open(saved).expect("what was written is a PDF");
    assert!(
        pdf_model::signature::permissions(&reopened)
            .usage_rights
            .is_some(),
        "filling in a field is what /Form [/FillIn] grants, so the signature stands"
    );

    let withheld = Document::open(form_with_usage_rights("/Import")).expect("a valid PDF");
    let mut view = ViewState::of(&withheld);
    assert_eq!(
        view.set_field(&withheld, "name", &Entered::Text("typed".to_owned())),
        1
    );
    let saved = view
        .save(&withheld)
        .expect("the fixture can be updated")
        .bytes;
    let reopened = Document::open(saved).expect("what was written is a PDF");
    assert!(
        pdf_model::signature::permissions(&reopened)
            .usage_rights
            .is_none(),
        "/Form [/Import] does not grant filling in, so the signature is withdrawn"
    );
    // The rest of the permissions dictionary survives: this removes one entry, not the feature.
    let catalog = reopened.catalog().expect("the fixture has a catalog");
    assert!(
        reopened.get_key(&catalog, "Perms").as_dict().is_some(),
        "the permissions dictionary is still there, without its /UR3"
    );
    // And the value the person typed is in the file that came back.
    let (after, _) = drawn(&reopened, &ViewState::of(&reopened));
    assert!(after.contains("typed"), "{after:?}");
}

/// §12.8.2.2's `/P` says which operation an author forbade, and it is stated rather than obeyed.
///
/// ISO 32000-2 §12.8.2.2.1, in a parenthesis that is easy to read past:
///
/// > (These changes to the document shall also be prevented if the signature dictionary is
/// > referred from the DocMDP entry in the permissions dictionary.)
///
/// Table 257 makes `/P` 1 "no changes to the document shall be permitted", `/P` 2 "filling in
/// forms, instantiating page templates, and signing", and `/P` 3 the same "as well as annotation
/// creation, deletion, and modification". So one fixture at three levels gives **different
/// answers for two different operations**, which is what makes this a test of the clause rather
/// than of a flag: level 2 is where filling in and annotating part company.
///
/// **What changed in the three-hundred-and-seventy-third session** is who acts on it. Until then
/// `ViewState::set_field` refused by returning zero widgets, which is a number three other things
/// also produce; now `restriction::asserted` says *which clause* and *which level*, and the host
/// holding the reader's policy decides. `CLAUDE.md` makes that policy the reader's, with four
/// levels, and two of them have to describe the operation to a person before it happens — which a
/// count of widgets cannot. ADR 0212.
#[test]
fn a_certified_document_states_which_operation_its_author_forbade() {
    use pdf_model::restriction::{Operation, Restriction, asserted};
    use pdf_model::signature::Modification;

    let final_document = Document::open(certified_form(1)).expect("the fixture is a valid PDF");
    for operation in [Operation::FillInForm, Operation::Annotate] {
        assert_eq!(
            asserted(&final_document, operation, None, None),
            vec![Restriction::Certified {
                level: Modification::None
            }],
            "/P 1 permits no change at all"
        );
    }

    // Level 2 is the level that separates them: "filling in forms … and signing" and not
    // annotation, which level 3 adds.
    let fillable = Document::open(certified_form(2)).expect("the fixture is a valid PDF");
    assert_eq!(
        asserted(&fillable, Operation::FillInForm, None, None),
        Vec::new()
    );
    assert_eq!(
        asserted(&fillable, Operation::Annotate, None, None),
        vec![Restriction::Certified {
            level: Modification::FormFilling
        }]
    );

    let commented = Document::open(certified_form(3)).expect("the fixture is a valid PDF");
    assert_eq!(
        asserted(&commented, Operation::Annotate, None, None),
        Vec::new()
    );

    // Table 257 defines 1, 2 and 3 and nothing else; a value outside them may not lock a
    // document a person is entitled to fill in.
    let odd = Document::open(certified_form(9)).expect("the fixture is a valid PDF");
    assert_eq!(
        asserted(&odd, Operation::FillInForm, None, None),
        Vec::new()
    );
    assert_eq!(asserted(&odd, Operation::Annotate, None, None), Vec::new());

    // And `pdf-model` itself no longer refuses: the value goes in, because whether to obey the
    // document is not a question this crate is entitled to answer.
    let mut view = ViewState::of(&final_document);
    assert_eq!(
        view.set_field(&final_document, "name", &Entered::Text("typed".to_owned())),
        1
    );
    let (after, _) = drawn(&final_document, &view);
    assert!(after.contains("typed"), "{after:?}");
}

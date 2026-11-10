//! Effective form-action selection, output encoding, coordinates and import dispatch.
#![expect(
    clippy::expect_used,
    clippy::panic,
    reason = "test fixture construction"
)]

use std::fmt::Write as _;

use pdf_model::action::{Action, SubmitForm};
use pdf_model::forms_data::FormsData;
use pdf_model::submission::{Format, compose};
use pdf_model::view::ViewState;
use pdf_syntax::{Document, FileBytes, Object, ObjectId};

fn assembled(body: &str) -> Vec<u8> {
    assembled_as("2.0", body)
}

fn assembled_as(version: &str, body: &str) -> Vec<u8> {
    let mut out = format!("%PDF-{version}\n");
    let mut offsets = Vec::new();
    for object in body.split_inclusive("endobj\n") {
        offsets.push(out.len());
        out.push_str(object);
    }
    let xref_at = out.len();
    let size = offsets.len().saturating_add(1);
    let _ = writeln!(out, "xref\n0 {size}");
    out.push_str("0000000000 65535 f \n");
    for offset in offsets {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    let _ = write!(
        out,
        "trailer\n<< /Size {size} /Root 1 0 R /ID \
         [<30313233343536373839616263646566> <66656463626139383736353433323130>] >>\n\
         startxref\n{xref_at}\n%%EOF\n"
    );
    out.into_bytes()
}

fn read_submit(document: &Document, id: u32) -> SubmitForm {
    let actions = pdf_model::action::read(document, &Object::Reference(ObjectId::new(id, 0)));
    let [Action::SubmitForm(submit)] = actions.as_slice() else {
        panic!("the widget-referenced Table239 action is parsed whole: {actions:?}");
    };
    submit.clone()
}

fn fdf_names(submission: &pdf_model::submission::Submission) -> Vec<(String, Option<String>)> {
    let fdf =
        Document::open(FileBytes::from(submission.body.clone())).expect("composed FDF parses");
    assert!(
        !fdf.was_recovered(),
        "the writer's output has an exact xref"
    );
    FormsData::read(&fdf)
        .expect("the FDF dictionary and field tree are readable")
        .fields
        .into_iter()
        .map(|field| {
            let value = field.value.and_then(|value| match value {
                Object::String(bytes) => Some(pdf_syntax::text_string(&bytes)),
                Object::Name(name) => {
                    Some(format!("/{}", String::from_utf8_lossy(name.as_bytes())))
                }
                _ => None,
            });
            (field.name, value)
        })
        .collect()
}

/// Table239 permits a Fields reference to a nonterminal field; Table240 requires its descendants.
#[test]
fn submit_fields_reference_selects_the_referenced_fields_descendants() {
    fn form(fields: &str) -> Vec<u8> {
        let body = format!(
            "1 0 obj\n<< /Type /Catalog /Pages 2 0 R /AcroForm << /Fields [4 0 R 6 0 R] /NeedAppearances true /DA (/Helv 12 Tf 0 g) /DR << /Font << /Helv 8 0 R >> >> >> >>\nendobj\n\
             2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
             3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] /Resources << >> \
             /Annots [5 0 R 6 0 R] >>\nendobj\n\
             4 0 obj\n<< /FT /Tx /T (group) /Kids [5 0 R] >>\nendobj\n\
             5 0 obj\n<< /Type /Annot /Subtype /Widget /Rect [10 10 90 30] /F 4 /Parent 4 0 R \
             /T (inner) /V (nested) /A 7 0 R >>\nendobj\n\
             6 0 obj\n<< /Type /Annot /Subtype /Widget /Rect [10 40 90 60] /F 4 /FT /Tx \
             /T (other) /V (sibling) >>\nendobj\n\
             7 0 obj\n<< /S /SubmitForm /F << /FS /URL /F (https://example.invalid/submit) >> \
             {fields} >>\nendobj\n\
             8 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>\nendobj\n"
        );
        assembled_as("1.7", &body)
    }
    let document = Document::open(FileBytes::from(form("/Fields [4 0 R]"))).expect("valid PDF");
    assert!(!document.was_recovered(), "exact source xref");
    let action = read_submit(&document, 7);
    let view = ViewState::of(&document);
    let chosen = compose(&document, &view, &action, None).expect("FDF composition succeeds");
    assert_eq!(chosen.format, Format::Fdf);
    assert_eq!(
        fdf_names(&chosen),
        [
            ("group".to_owned(), None),
            ("group.inner".to_owned(), Some("nested".to_owned()))
        ],
        "an indirect reference selects the terminal descendants under the fully qualified name"
    );

    // Positive control: omitting Fields selects all exportable fields in the form.
    let all_document = Document::open(FileBytes::from(form(""))).expect("valid control PDF");
    let all_action = read_submit(&all_document, 7);
    let all = compose(
        &all_document,
        &ViewState::of(&all_document),
        &all_action,
        None,
    )
    .expect("default composition succeeds");
    assert_eq!(
        fdf_names(&all),
        [
            ("group".to_owned(), None),
            ("group.inner".to_owned(), Some("nested".to_owned())),
            ("other".to_owned(), Some("sibling".to_owned()))
        ],
        "control establishes both fields were available before selection"
    );
}

/// Table239 `CharSet` is a PDF2 entry; current HTML composition uses UTF-8 and explicitly reports
/// non-UTF-8 requests. This test pins that bounded behavior instead of pretending `BigFive` bytes
/// were emitted. UTF-8 is the positive control.
#[test]
fn submit_charset_non_utf8_is_reported_and_utf8_control_is_not() {
    fn form(charset: &str) -> Vec<u8> {
        let body = format!(
            "1 0 obj\n<< /Type /Catalog /Pages 2 0 R /AcroForm << /Fields [4 0 R] /DA (/Helv 12 Tf 0 g) /DR << /Font << /Helv 6 0 R >> >> >> >>\nendobj\n\
             2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
             3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] /Resources << >> \
             /Annots [4 0 R] >>\nendobj\n\
             4 0 obj\n<< /Type /Annot /Subtype /Widget /Rect [10 10 90 30] /F 4 /FT /Tx \
             /T (name) /V <FEFF004D00FC006C006C00650072> /A 5 0 R /AP << /N 7 0 R >> >>\nendobj\n\
             5 0 obj\n<< /S /SubmitForm /F << /FS /URL /F (https://example.invalid/submit) >> \
             /Flags 4 /Fields [(name)] /CharSet ({charset}) >>\nendobj\n\
             6 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>\nendobj\n\
             7 0 obj\n<< /Type /XObject /Subtype /Form /FormType 1 /BBox [0 0 80 20] /Resources << /Font << /Helv 6 0 R >> >> \
             /Length 47 >>\nstream\nq BT /Helv 12 Tf 0 g 1 4 Td (M\\374ller) Tj ET Q\nendstream\nendobj\n"
        );
        assembled(&body)
    }
    fn submission(bytes: Vec<u8>) -> pdf_model::submission::Submission {
        let document = Document::open(FileBytes::from(bytes)).expect("valid PDF");
        assert!(!document.was_recovered(), "exact source xref");
        let action = read_submit(&document, 5);
        compose(&document, &ViewState::of(&document), &action, None).expect("HTML composition")
    }

    let requested = submission(form("BigFive"));
    assert_eq!(requested.format, Format::HtmlForm);
    assert_eq!(
        String::from_utf8(requested.body.clone()).expect("UTF-8 body"),
        "name=M%C3%BCller"
    );
    assert!(
        requested
            .owed
            .iter()
            .any(|item| item.contains("CharSet BigFive") && item.contains("UTF-8")),
        "unsupported output encoding is named, not silently claimed: {:?}",
        requested.owed
    );

    let utf8 = submission(form("utf-8"));
    assert!(
        utf8.owed.iter().all(|item| !item.contains("CharSet")),
        "UTF-8 is supported: {:?}",
        utf8.owed
    );
    assert_eq!(
        String::from_utf8(utf8.body).expect("UTF-8 body"),
        "name=M%C3%BCller"
    );
}

/// Table243 `ImportData` travels through the view as a host request instead of opening a file in
/// the model layer. The host boundary is the actual consumer of the required file specification.
#[test]
fn import_action_becomes_a_file_request_with_the_named_format() {
    let body = "1 0 obj\n<< /Type /Catalog /Pages 2 0 R /AcroForm << /Fields [4 0 R] /NeedAppearances true /DA (/Helv 12 Tf 0 g) /DR << /Font << /Helv 6 0 R >> >> >> >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] /Resources << >> \
         /Annots [4 0 R] >>\nendobj\n\
         4 0 obj\n<< /Type /Annot /Subtype /Widget /Rect [10 10 90 30] /F 4 /FT /Btn /Ff 65536 \
         /T (import) /A 5 0 R /AP << /N 7 0 R >> >>\nendobj\n\
         5 0 obj\n<< /S /ImportData /F (answers.FDF) >>\nendobj\n\
         6 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>\nendobj\n\
         7 0 obj\n<< /Type /XObject /Subtype /Form /FormType 1 /BBox [0 0 80 20] /Resources << >> /Length 24 >>\nstream\nq 0.2 g 0 0 80 20 re f Q\nendstream\nendobj\n";
    let document =
        Document::open(FileBytes::from(assembled_as("1.7", body))).expect("complete valid PDF");
    assert!(!document.was_recovered(), "exact xref");
    let actions = pdf_model::action::read(&document, &Object::Reference(ObjectId::new(5, 0)));
    let [Action::ImportData(import)] = actions.as_slice() else {
        panic!("the action's required file specification is read: {actions:?}");
    };
    let request = ViewState::of(&document).perform(&document, &Action::ImportData(import.clone()));
    assert_eq!(
        request,
        Some(pdf_model::view::Request::Import(import.clone())),
        "the model asks its caller for the file instead of opening a filesystem path"
    );
    assert_eq!(import.file, "answers.FDF");
    assert_eq!(import.format, pdf_model::action::DataFormat::Fdf);
}

/// Table240 bit5's exact `/TM` single-space exception versus fallback to the fully qualified
/// field name. The click point is 15 right and 15 down from the widget's upper-left corner.
#[test]
fn submit_coordinates_suppresses_name_for_space_tm() {
    fn form(mapping: Option<&str>) -> Vec<u8> {
        let mapping = mapping.map_or_else(String::new, |value| format!("/TM ({value}) "));
        let body = format!(
            "1 0 obj\n<< /Type /Catalog /Pages 2 0 R /AcroForm << /Fields [4 0 R 6 0 R] /NeedAppearances true /DA (/Helv 12 Tf 0 g) /DR << /Font << /Helv 8 0 R >> >> >> >>\nendobj\n\
             2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
             3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] /Resources << >> \
             /Annots [4 0 R 6 0 R] >>\nendobj\n\
             4 0 obj\n<< /Type /Annot /Subtype /Widget /Rect [20 30 100 60] /F 4 /FT /Btn /Ff 65536 \
             /T (submit) {mapping}/A 5 0 R /AP << /N 7 0 R >> >>\nendobj\n\
             5 0 obj\n<< /S /SubmitForm /F << /FS /URL /F (https://example.invalid/submit) >> \
             /Fields [(name)] /Flags 20 >>\nendobj\n\
             6 0 obj\n<< /Type /Annot /Subtype /Widget /Rect [20 80 100 100] /F 4 /FT /Tx /T (name) /V (Ada) >>\nendobj\n\
             7 0 obj\n<< /Type /XObject /Subtype /Form /FormType 1 /BBox [0 0 80 30] /Resources << >> /Length 24 >>\nstream\nq 0.2 g 0 0 80 30 re f Q\nendstream\nendobj\n\
             8 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>\nendobj\n"
        );
        assembled_as("1.7", &body)
    }
    fn compose_for(mapping: Option<&str>) -> pdf_model::submission::Submission {
        let document = Document::open(FileBytes::from(form(mapping))).expect("complete valid PDF");
        assert!(!document.was_recovered(), "exact source xref");
        let action = read_submit(&document, 5);
        let view = ViewState::of(&document);
        compose(
            &document,
            &view,
            &action,
            Some(&pdf_model::submission::Click {
                point: (35.0, 45.0),
                widget: Some(ObjectId::new(4, 0)),
            }),
        )
        .expect("HTML form submission composes")
    }

    let suppressed = compose_for(Some(" "));
    assert_eq!(suppressed.format, Format::HtmlForm);
    assert_eq!(
        String::from_utf8(suppressed.body).expect("form body"),
        "name=Ada&x=15&y=15"
    );

    // Positive control: absent /TM uses the field's /T followed by .x/.y.
    let fallback = compose_for(None);
    assert_eq!(
        String::from_utf8(fallback.body).expect("form body"),
        "name=Ada&submit.x=15&submit.y=15"
    );
}

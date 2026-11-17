//! Table239/241 distinguish an omitted Fields entry from a present empty array.
#![expect(
    clippy::expect_used,
    clippy::panic,
    reason = "standalone spec-audit fixture construction"
)]

use std::fmt::Write as _;

use pdf_model::action::Action;
use pdf_model::forms_data::FormsData;
use pdf_model::submission::compose;
use pdf_model::view::ViewState;
use pdf_syntax::{Document, FileBytes, Object, ObjectId};

fn pdf(action_kind: &str, fields: Option<&str>, flags: u32) -> Document {
    let fields = fields.map_or_else(String::new, |array| format!("/Fields {array}"));
    let action = match action_kind {
        "SubmitForm" => format!(
            "<< /S /SubmitForm /F << /FS /URL /F (https://example.invalid/submit) >> \
             {fields} /Flags {flags} >>"
        ),
        "ResetForm" => format!("<< /S /ResetForm {fields} /Flags {flags} >>"),
        _ => panic!("known action kind"),
    };
    let objects = [
        "<< /Type /Catalog /Pages 2 0 R /AcroForm << /Fields [5 0 R 6 0 R 7 0 R 8 0 R] \
         /NeedAppearances true /DA (/Helv 12 Tf 0 g) /DR << /Font << /Helv 10 0 R >> >> >> >>"
            .to_owned(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] /Resources << >> \
         /Contents 4 0 R /Annots [5 0 R 6 0 R 7 0 R 8 0 R] >>"
            .to_owned(),
        "<< /Length 0 >>\nstream\n\nendstream".to_owned(),
        "<< /Type /Annot /Subtype /Widget /Rect [10 110 180 140] /P 3 0 R /F 4 \
         /FT /Tx /T (first) /V (one) /DV (factory-one) /DA (/Helv 12 Tf 0 g) >>"
            .to_owned(),
        "<< /Type /Annot /Subtype /Widget /Rect [10 70 180 100] /P 3 0 R /F 4 \
         /FT /Tx /T (second) /V (two) /DV (factory-two) /DA (/Helv 12 Tf 0 g) >>"
            .to_owned(),
        "<< /Type /Annot /Subtype /Widget /Rect [10 30 180 60] /P 3 0 R /F 4 \
         /FT /Tx /T (blank) /DA (/Helv 12 Tf 0 g) >>"
            .to_owned(),
        "<< /Type /Annot /Subtype /Widget /Rect [10 10 180 25] /P 3 0 R /F 4 \
         /FT /Tx /T (secret) /Ff 4 /DA (/Helv 12 Tf 0 g) >>"
            .to_owned(),
        action,
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>"
            .to_owned(),
    ];
    let mut out = String::from("%PDF-1.7\n");
    let mut offsets = vec![0_usize];
    for (index, object) in objects.iter().enumerate() {
        offsets.push(out.len());
        writeln!(out, "{} 0 obj\n{object}\nendobj", index.saturating_add(1))
            .expect("in-memory object write succeeds");
    }
    let xref = out.len();
    writeln!(out, "xref\n0 {}\n0000000000 65535 f ", offsets.len())
        .expect("in-memory xref header write succeeds");
    for offset in offsets.iter().skip(1) {
        writeln!(out, "{offset:010} 00000 n ").expect("in-memory xref entry write succeeds");
    }
    write!(out, "trailer\n<< /Size {} /Root 1 0 R /ID [<01020304> <05060708>] >>\nstartxref\n{xref}\n%%EOF\n", offsets.len())
        .expect("in-memory trailer write succeeds");
    let document = Document::open(FileBytes::from(out.into_bytes()))
        .expect("the fixture PDF structure is readable");
    assert!(!document.was_recovered(), "exact xref offsets");
    document
}

fn action(document: &Document) -> Action {
    let actions = pdf_model::action::read(document, &Object::Reference(ObjectId::new(9, 0)));
    let [action] = actions.as_slice() else {
        panic!("the referenced action is read: {actions:?}");
    };
    action.clone()
}

fn composed(fields: Option<&str>, flags: u32) -> pdf_model::submission::Submission {
    let document = pdf("SubmitForm", fields, flags);
    let Action::SubmitForm(submit) = action(&document) else {
        panic!("SubmitForm parsed");
    };
    let view = ViewState::of(&document);
    compose(&document, &view, &submit, None).expect("submission composition succeeds")
}

fn fdf_fields(submission: &pdf_model::submission::Submission) -> Vec<(String, bool)> {
    let fdf = Document::open(FileBytes::from(submission.body.clone())).expect("FDF parses");
    assert!(!fdf.was_recovered(), "composed FDF has exact xref");
    let mut fields = FormsData::read(&fdf)
        .expect("FDF catalog and fields parse")
        .fields
        .into_iter()
        .map(|field| (field.name, field.value.is_some()))
        .collect::<Vec<_>>();
    fields.sort_by(|left, right| left.0.cmp(&right.0));
    fields
}

#[test]
fn submit_fields_presence_survives_selection_and_other_flag_rules() {
    // Omitted Fields: all exportable values survive even when Include/Exclude is set; that bit is
    // ignored. IncludeNoValueFields independently decides whether the ordinary blank field appears.
    let omitted_default = composed(None, 0);
    let expected = [("first".to_owned(), true), ("second".to_owned(), true)];
    assert_eq!(fdf_fields(&omitted_default), expected);
    let omitted_exclude_bit = composed(None, 1);
    assert_eq!(fdf_fields(&omitted_exclude_bit), expected);
    let omitted_include_blank = composed(None, 2);
    assert_eq!(
        fdf_fields(&omitted_include_blank),
        [
            ("blank".to_owned(), false),
            ("first".to_owned(), true),
            ("second".to_owned(), true)
        ]
    );
    let omitted_both_bits = composed(None, 3);
    assert_eq!(
        fdf_fields(&omitted_both_bits),
        fdf_fields(&omitted_include_blank)
    );
    assert!(
        fdf_fields(&omitted_include_blank)
            .iter()
            .all(|(name, _)| name != "secret"),
        "NoExport excludes even a valueless field selected under IncludeNoValueFields"
    );

    // With Include/Exclude set, an empty list excludes nothing and leaves ordinary values
    // selected. This is a control for the empty include-list case below.
    let explicit_empty_exclude = composed(Some("[]"), 1);
    assert_eq!(fdf_fields(&explicit_empty_exclude), expected);

    // A nonempty include list selects only its named field (literal membership control).
    let named_include = composed(Some("[(first)]"), 0);
    assert_eq!(fdf_fields(&named_include), [("first".to_owned(), true)]);

    // A present empty array with the Include/Exclude bit clear selects no fields. The no-value
    // flag cannot turn an empty selected set into all fields. Compute both cases before comparing.
    let explicit_empty = composed(Some("[]"), 0);
    let explicit_empty_with_blank_flag = composed(Some("[]"), 2);
    let empty_outputs = (
        fdf_fields(&explicit_empty),
        fdf_fields(&explicit_empty_with_blank_flag),
    );
    assert_eq!(empty_outputs, (Vec::new(), Vec::new()));
}

fn reset_values(fields: Option<&str>, flags: u32) -> (Option<String>, Option<String>, bool, bool) {
    let document = pdf("ResetForm", fields, flags);
    let parsed = action(&document);
    let mut view = ViewState::of(&document);
    view.perform(&document, &parsed);
    let first = view.field_value(&document, "first").map(|value| value.text);
    let second = view
        .field_value(&document, "second")
        .map(|value| value.text);
    (
        first,
        second,
        view.is_reset(ObjectId::new(5, 0)),
        view.is_reset(ObjectId::new(6, 0)),
    )
}

#[test]
fn reset_fields_presence_distinguishes_no_selection_from_reset_all() {
    // Omission means all fields reset; an Include/Exclude bit cannot change that special case.
    let reset_all = (
        Some("factory-one".to_owned()),
        Some("factory-two".to_owned()),
        true,
        true,
    );
    assert_eq!(reset_values(None, 0), reset_all);
    assert_eq!(reset_values(None, 1), reset_all);

    // A nonempty include list resets only the selected field; the other value is a control.
    assert_eq!(
        reset_values(Some("[(first)]"), 0),
        (
            Some("factory-one".to_owned()),
            Some("two".to_owned()),
            true,
            false
        )
    );

    // With Include/Exclude set, an empty list excludes no fields, so all reset.
    assert_eq!(reset_values(Some("[]"), 1), reset_all);

    // Explicit [] with the clear flag is an empty include set, so neither populated field changes.
    assert_eq!(
        reset_values(Some("[]"), 0),
        (Some("one".to_owned()), Some("two".to_owned()), false, false)
    );
}

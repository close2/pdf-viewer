//! ISO 32000-2 §12.7, audited: what a field's value is exported as, which button a value turns on
//! and what a save writes for it, which values a field type takes, and how a name and a comb are
//! read (`doc/reviews/2148-specification-audit-12-7.md`).
//!
//! Each test holds a control beside the finding's own assertion, so a failure is the defect and
//! not the fixture.
#![expect(
    clippy::expect_used,
    clippy::panic,
    reason = "test fixture construction"
)]

use std::fmt::Write as _;

use pdf_model::Pages;
use pdf_model::action::{Action, SubmitForm};
use pdf_model::form::{self, Control};
use pdf_model::submission::{Format, compose};
use pdf_model::view::{Entered, ViewState, widgets_by_field_name};
use pdf_syntax::{Document, FileBytes, Object, ObjectId};

/// A PDF of `objects`, numbered from 1 in order, with an exact cross-reference table.
fn pdf(objects: &[&str]) -> Vec<u8> {
    let mut out = String::from("%PDF-1.7\n");
    let mut offsets = Vec::with_capacity(objects.len());
    for (index, body) in objects.iter().enumerate() {
        offsets.push(out.len());
        let _ = writeln!(out, "{} 0 obj\n{body}\nendobj", index.saturating_add(1));
    }
    let xref_at = out.len();
    let size = objects.len().saturating_add(1);
    let _ = writeln!(out, "xref\n0 {size}\n0000000000 65535 f ");
    for offset in offsets {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    let _ = write!(
        out,
        "trailer\n<< /Size {size} /Root 1 0 R /ID \
         [<0123456789abcdef0123456789abcdef> <0123456789abcdef0123456789abcdef>] >>\n\
         startxref\n{xref_at}\n%%EOF\n"
    );
    out.into_bytes()
}

fn opened(bytes: Vec<u8>) -> Document {
    let document = Document::open(FileBytes::from(bytes)).expect("a complete PDF");
    assert!(!document.was_recovered(), "the fixture's xref is exact");
    document
}

fn submit_action(document: &Document, id: u32) -> SubmitForm {
    let actions = pdf_model::action::read(document, &Object::Reference(ObjectId::new(id, 0)));
    let [Action::SubmitForm(submit)] = actions.as_slice() else {
        panic!("one Table 239 submit-form action: {actions:?}");
    };
    submit.clone()
}

/// The HTML Form body a submit of the whole form composes (Table 240 bit 3 set).
fn html_body(bytes: Vec<u8>, action: u32) -> String {
    let document = opened(bytes);
    let submit = submit_action(&document, action);
    let view = ViewState::of(&document);
    let submission = compose(&document, &view, &submit, None).expect("the submission composes");
    assert_eq!(
        submission.format,
        Format::HtmlForm,
        "Table 240 bit 3 is set"
    );
    String::from_utf8(submission.body).expect("an urlencoded body is ASCII")
}

const HELVETICA: &str = "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>";
const SUBMIT_HTML: &str =
    "<< /S /SubmitForm /F << /FS /URL /F (https://example.invalid/s) >> /Flags 4 >>";
const EMPTY_FORM: &str =
    "<< /Type /XObject /Subtype /Form /BBox [0 0 20 20] /Length 0 >>\nstream\n\nendstream";

/// §12.7.5.4 makes a choice field's `/V` the *displayed* string of a two-element `/Opt` entry
/// ("the name string is the second of the two array elements"), and the first element "a text
/// string representing the export value that shall be used when exporting interactive form field
/// data from the document". A submission exports form field data, so it carries the export value.
#[test]
fn a_choice_is_submitted_by_its_export_value_not_its_label_spec_audit() {
    let objects = [
        "<< /Type /Catalog /Pages 2 0 R /AcroForm << /Fields [4 0 R 5 0 R] \
         /DA (/F1 12 Tf 0 g) /DR << /Font << /F1 6 0 R >> >> >> >>",
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] /Resources << >> \
         /Annots [4 0 R 5 0 R] >>",
        "<< /Type /Annot /Subtype /Widget /Rect [10 10 90 30] /F 4 /FT /Ch /Ff 131072 \
         /T (colour) /Opt [[(r) (Red)] [(g) (Green)]] /V (Green) >>",
        "<< /Type /Annot /Subtype /Widget /Rect [10 40 90 60] /F 4 /FT /Ch /Ff 131072 \
         /T (plain) /Opt [(Blue) (Grey)] /V (Grey) >>",
        HELVETICA,
        SUBMIT_HTML,
    ];
    let body = html_body(pdf(&objects), 7);
    // Control: an `/Opt` entry that is one string has no export value but itself.
    assert!(
        body.contains("plain=Grey"),
        "a one-string option exports itself: {body}"
    );
    assert!(
        body.contains("colour=g"),
        "the selected item's export value, (g), is what a submission carries: {body}"
    );
}

/// Table 230's `/Opt` for a check box: "an array of text strings representing the export value
/// of each annotation in the field" — and the `/AP` names "may use numerical position (starting
/// with 0)". The `/V` here is `/0`, which selects an appearance; the export value is `(Ja)`.
#[test]
fn a_check_box_with_opt_is_submitted_by_its_export_value_spec_audit() {
    let objects = [
        "<< /Type /Catalog /Pages 2 0 R /AcroForm << /Fields [4 0 R 6 0 R] >> >>",
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] /Resources << >> \
         /Annots [5 0 R 6 0 R] >>",
        "<< /FT /Btn /T (agree) /Opt [(Ja)] /V /0 /Kids [5 0 R] >>",
        "<< /Type /Annot /Subtype /Widget /Parent 4 0 R /Rect [10 10 30 30] /F 4 /AS /0 \
         /AP << /N << /0 7 0 R /Off 7 0 R >> >> >>",
        "<< /Type /Annot /Subtype /Widget /Rect [40 10 60 30] /F 4 /FT /Btn /T (plain) /V /Yes \
         /AS /Yes /AP << /N << /Yes 7 0 R /Off 7 0 R >> >> >>",
        EMPTY_FORM,
        SUBMIT_HTML,
    ];
    let body = html_body(pdf(&objects), 8);
    // Control: a check box without `/Opt` exports its state name.
    assert!(
        body.contains("plain=Yes"),
        "a check box without /Opt exports its on-state name: {body}"
    );
    assert!(
        body.contains("agree=Ja"),
        "Table 230's export value of the widget that is on: {body}"
    );
}

/// A radio set of two buttons sharing the on-state name `Yes`, with `RadiosInUnison` clear
/// (`/Ff` is Radio and `NoToggleToOff`, bits 16 and 15).
fn shared_name_radios() -> Vec<u8> {
    pdf(&[
        "<< /Type /Catalog /Pages 2 0 R /AcroForm << /Fields [4 0 R] >> >>",
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] /Resources << >> \
         /Annots [5 0 R 6 0 R] >>",
        "<< /FT /Btn /T (r) /Ff 49152 /V /Off /Kids [5 0 R 6 0 R] >>",
        "<< /Type /Annot /Subtype /Widget /Parent 4 0 R /Rect [10 10 30 30] /F 4 /AS /Off \
         /AP << /N << /Yes 7 0 R /Off 7 0 R >> >> >>",
        "<< /Type /Annot /Subtype /Widget /Parent 4 0 R /Rect [40 10 60 30] /F 4 /AS /Off \
         /AP << /N << /Yes 7 0 R /Off 7 0 R >> >> >>",
        EMPTY_FORM,
    ])
}

fn buttons_on(document: &Document, view: &ViewState) -> Vec<bool> {
    let page = Pages::new(document).get(0).expect("page one");
    let fields = form::fields(document, &page, view);
    let [field] = fields.as_slice() else {
        panic!("one radio field: {fields:?}");
    };
    assert!(
        matches!(field.control, Control::RadioButton { .. }),
        "Table 229 bit 16: {:?}",
        field.control
    );
    field.widgets.iter().map(|widget| widget.on).collect()
}

/// §12.7.5.2.3: "If it is not set, at most one radio button in a field shall be set at a time."
/// The view obeys it (the first of the two goes on); the save writes `/AS /Yes` on both, and
/// §12.7.5.2.3 makes `/AS` decide, so the saved file shows two buttons on.
#[test]
fn a_saved_radio_set_keeps_at_most_one_button_on_spec_audit() {
    let document = opened(shared_name_radios());
    let mut view = ViewState::of(&document);
    let touched = view.set_field(&document, "r", &Entered::Text("Yes".to_owned()));
    assert_eq!(touched, 2, "the field owns two widgets");
    // Control: the view itself keeps one button on.
    assert_eq!(buttons_on(&document, &view), [true, false]);

    let written = view.save(&document).expect("the update is written");
    let reopened = opened(written.bytes);
    let on = buttons_on(&reopened, &ViewState::of(&reopened));
    assert!(
        on.iter().filter(|on| **on).count() <= 1,
        "the saved file states at most one button of the set on, as the view showed: {on:?}"
    );
}

/// §12.7.5.2.4: "selecting any of the buttons automatically deselects all the others". A host
/// selects a button by naming its widget ([`Entered::Selected`], ADR 3311, `doc/questions/A974`):
/// for the *second* of two buttons sharing an on-state name, the second goes on alone, which the
/// name by itself cannot say.
#[test]
fn selecting_the_second_of_two_same_named_buttons_turns_it_on_spec_audit() {
    let document = opened(shared_name_radios());
    let page = Pages::new(&document).get(0).expect("page one");
    let mut view = ViewState::of(&document);
    let fields = form::fields(&document, &page, &view);
    let second = fields
        .first()
        .and_then(|field| field.widgets.get(1))
        .expect("the second button");
    let state = second
        .on_state
        .clone()
        .expect("the second button has an on state");
    assert_eq!(state, "Yes");
    view.set_field(&document, "r", &Entered::Selected(second.annotation));
    let on = buttons_on(&document, &view);
    // Control: exactly one button is on, which the RadiosInUnison rule asks.
    assert_eq!(on.iter().filter(|on| **on).count(), 1, "{on:?}");
    assert_eq!(
        on,
        [false, true],
        "the button selected is the one that goes on"
    );
}

/// A list box and a non-editable combo box: Table 233 bit 19 Edit, "if clear, it shall include
/// only a drop-down list", and §12.7.5.4's value is "one or more" of "several text items". Typed
/// characters that name no option are not a value either field can take from a person.
#[test]
fn typed_text_is_not_a_value_of_a_choice_without_edit_spec_audit() {
    let document = opened(pdf(&[
        "<< /Type /Catalog /Pages 2 0 R /AcroForm << /Fields [4 0 R] \
         /DA (/F1 12 Tf 0 g) /DR << /Font << /F1 5 0 R >> >> >> >>",
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] /Resources << >> /Annots [4 0 R] >>",
        "<< /Type /Annot /Subtype /Widget /Rect [10 10 90 60] /F 4 /FT /Ch /T (pick) \
         /Opt [(a) (b)] >>",
        HELVETICA,
    ]));
    let mut view = ViewState::of(&document);
    // Control: an option chosen by index is taken.
    assert_eq!(
        view.set_field(&document, "pick", &Entered::Chosen(vec![1])),
        1
    );
    assert_eq!(
        view.set_field(&document, "pick", &Entered::Text("zzz".to_owned())),
        0,
        "a list box takes no typed value that is not one of its options"
    );
}

/// §12.7.5.2.2: a push-button "retains no permanent value, it shall not use the V and DV entries
/// in the field dictionary". A value set on one is written to the file as `/V`.
#[test]
fn a_push_button_takes_no_value_spec_audit() {
    let document = opened(pdf(&[
        "<< /Type /Catalog /Pages 2 0 R /AcroForm << /Fields [4 0 R 5 0 R] \
         /DA (/F1 12 Tf 0 g) /DR << /Font << /F1 6 0 R >> >> >> >>",
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] /Resources << >> \
         /Annots [4 0 R 5 0 R] >>",
        "<< /Type /Annot /Subtype /Widget /Rect [10 10 90 30] /F 4 /FT /Btn /Ff 65536 /T (go) >>",
        "<< /Type /Annot /Subtype /Widget /Rect [10 40 90 60] /F 4 /FT /Tx /T (name) >>",
        HELVETICA,
    ]));
    let mut view = ViewState::of(&document);
    // Control: a text field beside it takes a value.
    assert_eq!(
        view.set_field(&document, "name", &Entered::Text("x".to_owned())),
        1
    );
    let applied = view.set_field(&document, "go", &Entered::Text("x".to_owned()));
    let written = view.save(&document).expect("the update is written");
    let reopened = opened(written.bytes);
    let button = reopened.get(ObjectId::new(4, 0));
    let stated = button
        .as_dict()
        .map_or(Object::Null, |dict| reopened.get_key(dict, "V"));
    assert!(
        applied == 0 && stated.is_null(),
        "a push-button is given no /V: applied to {applied} widget(s), saved /V {stated:?}"
    );
}

/// Table 231 bit 25 permits Comb only where `/MaxLen` is present and the Multiline, Password and
/// `FileSelect` flags are all clear. `TextControl::comb`'s own comment says a field claiming a comb
/// it is not entitled to gets no cell count; `FileSelect` is not read.
#[test]
fn a_file_select_field_is_not_a_comb_spec_audit() {
    let document = opened(pdf(&[
        "<< /Type /Catalog /Pages 2 0 R /AcroForm << /Fields [4 0 R 5 0 R] \
         /DA (/F1 12 Tf 0 g) /DR << /Font << /F1 6 0 R >> >> >> >>",
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] /Resources << >> \
         /Annots [4 0 R 5 0 R] >>",
        "<< /Type /Annot /Subtype /Widget /Rect [10 10 90 30] /F 4 /FT /Tx /T (comb) \
         /Ff 16777216 /MaxLen 5 >>",
        "<< /Type /Annot /Subtype /Widget /Rect [10 40 90 60] /F 4 /FT /Tx /T (file) \
         /Ff 17825792 /MaxLen 5 >>",
        HELVETICA,
    ]));
    let page = Pages::new(&document).get(0).expect("page one");
    let fields = form::fields(&document, &page, &ViewState::of(&document));
    let comb = |name: &str| {
        fields
            .iter()
            .find(|field| field.name.qualified == name)
            .and_then(|field| match &field.control {
                Control::Text(text) => Some(text.comb),
                _ => None,
            })
            .expect("a text field")
    };
    // Control: Comb alone, with MaxLen, is a comb of five cells.
    assert_eq!(comb("comb"), Some(5));
    assert_eq!(
        comb("file"),
        None,
        "Comb with FileSelect set is not a comb field"
    );
}

/// §12.7.4.2: "the fully qualified name shall be formed by appending the child field's partial
/// name to the parent's fully qualified name, separated by a PERIOD". A root field whose `/T` is
/// the empty string is a field (its own comment says so), so its child `a` is `.a` — and not the
/// top-level field `a` beside it.
#[test]
fn a_child_of_a_field_named_by_an_empty_string_keeps_its_separator_spec_audit() {
    let document = opened(pdf(&[
        "<< /Type /Catalog /Pages 2 0 R /AcroForm << /Fields [4 0 R 6 0 R] >> >>",
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] /Resources << >> \
         /Annots [5 0 R 6 0 R] >>",
        "<< /T () /Kids [5 0 R] >>",
        "<< /Type /Annot /Subtype /Widget /Parent 4 0 R /Rect [10 10 90 30] /F 4 /FT /Tx \
         /T (a) /V (inner) >>",
        "<< /Type /Annot /Subtype /Widget /Rect [10 40 90 60] /F 4 /FT /Tx /T (a) /V (outer) >>",
    ]));
    let table = widgets_by_field_name(&document);
    let names: Vec<&str> = table.keys().map(String::as_str).collect();
    // Control: both widgets are in the table under some name.
    assert_eq!(table.values().map(Vec::len).sum::<usize>(), 2);
    assert_eq!(
        names,
        [".a", "a"],
        "two fields, the child's name with its separator"
    );
}

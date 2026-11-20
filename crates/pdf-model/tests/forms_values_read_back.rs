//! The fixes for audit 2148's B findings, each read back the way the next reader reads it: a
//! submission parsed by this tree's own FDF and XFDF readers, and a save reopened (ADRs 3310 to
//! 3312). "A value has a reader on the way out as well as on the way in."
#![expect(
    clippy::expect_used,
    clippy::panic,
    reason = "test fixture construction"
)]

use std::fmt::Write as _;

use pdf_model::Pages;
use pdf_model::action::{Action, SubmitForm};
use pdf_model::form::{self, Control};
use pdf_model::forms_data::FormsData;
use pdf_model::submission::compose;
use pdf_model::view::{Entered, ViewState};
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
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n"
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

const EMPTY_FORM: &str =
    "<< /Type /XObject /Subtype /Form /BBox [0 0 20 20] /Length 0 >>\nstream\n\nendstream";

/// A list box whose two options share a label, a check box with Table 230's `/Opt`, and a submit
/// action whose `/Flags` is the caller's (0 is FDF, Table 240 bit 6 XFDF).
fn exporting_form(flags: u32) -> Vec<u8> {
    let submit = format!(
        "<< /S /SubmitForm /F << /FS /URL /F (https://example.invalid/s) >> /Flags {flags} >>"
    );
    pdf(&[
        "<< /Type /Catalog /Pages 2 0 R /AcroForm << /Fields [4 0 R 5 0 R] >> >>",
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] /Resources << >> \
         /Annots [4 0 R 6 0 R] >>",
        "<< /Type /Annot /Subtype /Widget /Rect [10 40 90 90] /F 4 /FT /Ch /T (size) \
         /Opt [[(s) (Small)] [(m) (Medium)] [(m2) (Medium)]] /V (Medium) /I [2] >>",
        "<< /FT /Btn /T (agree) /Opt [(Ja)] /V /0 /Kids [6 0 R] >>",
        "<< /Type /Annot /Subtype /Widget /Parent 5 0 R /Rect [10 10 30 30] /F 4 /AS /0 \
         /AP << /N << /0 7 0 R /Off 7 0 R >> >> >>",
        EMPTY_FORM,
        &submit,
    ])
}

/// What a submission of [`exporting_form`] states for each field, read back by this tree's own
/// reader of its format.
fn submitted(flags: u32) -> Vec<(String, Option<String>)> {
    let document = opened(exporting_form(flags));
    let view = ViewState::of(&document);
    let submission = compose(&document, &view, &submit_action(&document, 8), None)
        .expect("the submission composes");
    let data = if flags == 0 {
        let fdf = Document::open(FileBytes::from(submission.body))
            .expect("the composed FDF parses as §12.7.8.1's file");
        FormsData::read(&fdf).expect("and identifies itself with /FDF")
    } else {
        pdf_model::xfdf::read(&submission.body).expect("the composed XFDF reads back")
    };
    data.fields
        .into_iter()
        .map(|field| {
            let value = field.value.map(|value| match value {
                Object::String(bytes) => pdf_syntax::text_string(&bytes),
                Object::Name(name) => String::from_utf8_lossy(name.as_bytes()).into_owned(),
                other => format!("{other:?}"),
            });
            (field.name, value)
        })
        .collect()
}

/// 2148-B01 and B02 in the two formats the audit's HTML test does not reach: §12.7.5.4's export
/// value "shall be used when exporting interactive form field data", `/I` picks between two
/// options sharing a label, and Table 230's `/Opt` is the check box's export value.
#[test]
fn fdf_and_xfdf_carry_the_export_values() {
    for flags in [0, 32] {
        let fields = submitted(flags);
        assert!(
            fields.contains(&("size".to_owned(), Some("m2".to_owned()))),
            "/Flags {flags}: Table 234's /I names the second Medium: {fields:?}"
        );
        assert!(
            fields.contains(&("agree".to_owned(), Some("Ja".to_owned()))),
            "/Flags {flags}: Table 230's export value: {fields:?}"
        );
    }
}

/// A radio set of two buttons sharing the on-state name `Yes`; `/Ff` is the caller's.
fn shared_name_radios(flags: u32) -> Vec<u8> {
    let field = format!("<< /FT /Btn /T (r) /Ff {flags} /V /Off /Kids [5 0 R 6 0 R] >>");
    pdf(&[
        "<< /Type /Catalog /Pages 2 0 R /AcroForm << /Fields [4 0 R] >> >>",
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] /Resources << >> \
         /Annots [5 0 R 6 0 R] >>",
        &field,
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
    assert!(matches!(field.control, Control::RadioButton { .. }));
    field.widgets.iter().map(|widget| widget.on).collect()
}

/// What the view shows and what the saved file shows, after `entered`.
fn shown_and_saved(flags: u32, entered: &Entered) -> (Vec<bool>, Vec<bool>) {
    let document = opened(shared_name_radios(flags));
    let mut view = ViewState::of(&document);
    assert_eq!(view.set_field(&document, "r", entered), 2);
    let shown = buttons_on(&document, &view);
    let written = view.save(&document).expect("the update is written");
    let reopened = opened(written.bytes);
    (shown, buttons_on(&reopened, &ViewState::of(&reopened)))
}

/// 2148-B03 and B04, read back: §12.7.5.2.3's "at most one radio button in a field shall be set
/// at a time" with `RadiosInUnison` clear, and Table 229 bit 26's buttons that "turn on and off
/// in unison" with it set — the view and the reopened save agree on which.
#[test]
fn the_button_selected_is_the_one_shown_and_saved() {
    let radio = 49_152; // Table 229 bits 15 and 16: NoToggleToOff, Radio.
    let unison = radio | (1 << 25);
    let second = Entered::Selected(ObjectId::new(6, 0));

    let (shown, saved) = shown_and_saved(radio, &Entered::Text("Yes".to_owned()));
    assert_eq!(
        (shown, saved),
        (vec![true, false], vec![true, false]),
        "a name alone"
    );

    let (shown, saved) = shown_and_saved(radio, &second);
    assert_eq!(
        (shown, saved),
        (vec![false, true], vec![false, true]),
        "the second selected"
    );

    let (shown, saved) = shown_and_saved(unison, &second);
    assert_eq!(
        (shown, saved),
        (vec![true, true], vec![true, true]),
        "in unison"
    );

    // A widget that is not one of the field's buttons is no edit.
    let document = opened(shared_name_radios(radio));
    let mut view = ViewState::of(&document);
    assert_eq!(
        view.set_field(&document, "r", &Entered::Selected(ObjectId::new(7, 0))),
        0
    );
}

/// 2148-B05's control the audit test does not hold: typed text naming one of a list box's own
/// options is that option, and an editable combo box takes any text.
#[test]
fn a_choice_takes_its_own_labels_and_an_editable_combo_takes_any_text() {
    let document = opened(pdf(&[
        "<< /Type /Catalog /Pages 2 0 R /AcroForm << /Fields [4 0 R 5 0 R] >> >>",
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] /Resources << >> \
         /Annots [4 0 R 5 0 R] >>",
        "<< /Type /Annot /Subtype /Widget /Rect [10 10 90 60] /F 4 /FT /Ch /T (pick) \
         /Opt [(a) (b)] >>",
        "<< /Type /Annot /Subtype /Widget /Rect [10 70 90 90] /F 4 /FT /Ch /Ff 393216 \
         /T (open) /Opt [(a) (b)] >>",
    ]));
    let mut view = ViewState::of(&document);
    assert_eq!(
        view.set_field(&document, "pick", &Entered::Text("b".to_owned())),
        1
    );
    assert_eq!(
        view.set_field(&document, "open", &Entered::Text("zzz".to_owned())),
        1
    );
    let written = view.save(&document).expect("the update is written");
    let reopened = opened(written.bytes);
    let value = |number: u32| {
        let object = reopened.get(ObjectId::new(number, 0));
        object.as_dict().and_then(|dict| {
            reopened
                .get_key(dict, "V")
                .as_string()
                .map(pdf_syntax::text_string)
        })
    };
    assert_eq!(value(4).as_deref(), Some("b"));
    assert_eq!(value(5).as_deref(), Some("zzz"));
}

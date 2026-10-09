//! Table 199's `/K` at a choice field's selection, driven through `pdf_model::view::ViewState` with
//! the in-process engine (ADR 1786).
//!
//! ISO 32000-2 §12.6.3, Table 199: the keystroke action "shall be performed when the user modifies
//! a character in a text field or combo box or modifies the selection in a scrollable list box".
//! Every expected change is the text Table 234's `/Opt` shows for the option the selection names,
//! and every `changeEx` that option's export value, as ADR 1626 reads Adobe's reference; `keyDown`
//! is what the host said of the arrows (ADR 1762).

#![cfg(feature = "engine")]
#![expect(
    clippy::expect_used,
    reason = "test code: an explanatory panic is the intended failure"
)]

use std::fmt::Write as _;
use std::sync::Arc;

use pdf_model::view::{Entered, Keys, ViewState};
use pdf_script::{Budget, Engine};
use pdf_syntax::Document;

/// The keystroke script: it writes what it was handed into `Out`, refuses `Green`, turns `Red`
/// into `Blue`, and turns `Grey` into a text no option shows.
const KEYSTROKE: &str = r"this.getField\('Out'\).value = [event.change, event.changeEx, event.keyDown, event.willCommit].join\('|'\); if \(event.change == 'Green'\) event.rc = false; if \(event.change == 'Red'\) event.change = 'Blue'; if \(event.change == 'Grey'\) event.change = 'Purple';";

/// One page holding the list box `Colour` (object 4) and the text field `Out` (object 5); `flags`
/// is the list box's Table 233 `/Ff`.
fn form(flags: u32) -> Document {
    let bodies = [
        "<< /Type /Catalog /Pages 2 0 R /AcroForm << /Fields [4 0 R 5 0 R] >> >>".to_owned(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 /MediaBox [0 0 612 792] >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R /Annots [4 0 R 5 0 R] >>".to_owned(),
        format!(
            "<< /Type /Annot /Subtype /Widget /FT /Ch /Ff {flags} /T (Colour) \
             /Rect [10 50 210 120] /F 4 /P 3 0 R /DA (/Helv 10 Tf 0 g) \
             /Opt [[(r) (Red)] (Green) [(b) (Blue)] (Grey)] /AA << /K << /S /JavaScript \
             /JS ({KEYSTROKE}) >> >> >>"
        ),
        "<< /Type /Annot /Subtype /Widget /FT /Tx /T (Out) /Rect [10 200 300 230] /F 4 \
         /P 3 0 R /DA (/Helv 10 Tf 0 g) >>"
            .to_owned(),
    ];
    let mut out = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for (index, body) in bodies.iter().enumerate() {
        offsets.push(out.len());
        let _ = write!(out, "{} 0 obj\n{body}\nendobj\n", index.saturating_add(1));
    }
    let xref_at = out.len();
    let size = bodies.len().saturating_add(1);
    let _ = write!(out, "xref\n0 {size}\n0000000000 65535 f \n");
    for offset in &offsets {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    let _ = write!(
        out,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n"
    );
    Document::open(out.into_bytes()).expect("the fixture opens")
}

/// A view state of `document` with the engine supplied and the document opened.
fn opened(document: &Document) -> ViewState {
    let mut view = ViewState::of(document);
    view.run_scripts_with(Some(Arc::new(Engine::new(Budget::FIELD_EVENT))));
    view.run_open_scripts(document, 0);
    view
}

/// What a field shows, as text.
fn value(view: &ViewState, document: &Document, name: &str) -> String {
    view.field_value(document, name)
        .map(|shown| shown.text)
        .unwrap_or_default()
}

/// Which of Table 234's `/Opt` entries the list box `Colour` selects, as a host's control reads it.
fn selected(view: &ViewState, document: &Document) -> Vec<usize> {
    let page = pdf_model::Pages::new(document).get(0).expect("page one");
    pdf_model::form::fields(document, &page, view)
        .iter()
        .find(|field| field.partial == "Colour")
        .and_then(|field| match &field.control {
            pdf_model::form::Control::Choice(choice) => Some(choice.selected.clone()),
            _ => None,
        })
        .expect("the list box is on page one")
}

#[test]
fn a_selection_runs_the_keystroke_with_the_option_s_text_and_export_value() {
    let document = form(0);
    let mut view = opened(&document);
    assert_eq!(
        view.set_field(&document, "Colour", &Entered::Chosen(vec![2])),
        1,
        "{:?}",
        view.script_reports()
    );
    assert_eq!(value(&view, &document, "Out"), "Blue|b|false|false");
    assert_eq!(selected(&view, &document), [2]);
    // An option that is one string exports itself.
    view.set_field(&document, "Colour", &Entered::Chosen(vec![3]));
    assert_eq!(value(&view, &document, "Out"), "Grey|Grey|false|false");
}

#[test]
fn key_down_is_true_where_the_host_said_an_arrow_made_the_selection() {
    let document = form(0);
    let mut view = opened(&document);
    view.set_keys(Keys {
        shift: false,
        modifier: false,
        arrows: true,
    });
    view.set_field(&document, "Colour", &Entered::Chosen(vec![2]));
    assert_eq!(value(&view, &document, "Out"), "Blue|b|true|false");
    // The control: the same selection with the arrows let go.
    view.set_keys(Keys::default());
    view.set_field(&document, "Colour", &Entered::Chosen(vec![2]));
    assert_eq!(value(&view, &document, "Out"), "Blue|b|false|false");
}

#[test]
fn a_rejected_selection_is_not_taken() {
    let document = form(0);
    let mut view = opened(&document);
    view.set_field(&document, "Colour", &Entered::Chosen(vec![2]));
    assert_eq!(
        view.set_field(&document, "Colour", &Entered::Chosen(vec![1])),
        0,
        "Table 199: the action may \"reject\" the change"
    );
    assert_eq!(selected(&view, &document), [2]);
}

#[test]
fn a_change_rewritten_to_an_option_s_text_selects_that_option() {
    let document = form(0);
    let mut view = opened(&document);
    assert_eq!(
        view.set_field(&document, "Colour", &Entered::Chosen(vec![0])),
        1
    );
    assert_eq!(value(&view, &document, "Out"), "Red|r|false|false");
    assert_eq!(selected(&view, &document), [2], "Table 199: \"modify it\"");
}

#[test]
fn a_change_rewritten_to_no_option_is_reported_and_the_selection_stands() {
    let document = form(0);
    let mut view = opened(&document);
    assert_eq!(
        view.set_field(&document, "Colour", &Entered::Chosen(vec![3])),
        1
    );
    assert_eq!(selected(&view, &document), [3]);
    assert!(
        view.script_reports()
            .iter()
            .any(|report| report.contains("\"Purple\", which no option of /Opt shows")),
        "{:?}",
        view.script_reports()
    );
}

#[test]
fn a_multiple_selection_hands_the_first_option_and_an_empty_one_hands_nothing() {
    // Table 233 bit 22, MultiSelect.
    let document = form(1 << 21);
    let mut view = opened(&document);
    assert_eq!(
        view.set_field(&document, "Colour", &Entered::Chosen(vec![3, 2])),
        1
    );
    assert_eq!(value(&view, &document, "Out"), "Blue|b|false|false");
    view.set_field(&document, "Colour", &Entered::Chosen(Vec::new()));
    assert_eq!(value(&view, &document, "Out"), "||false|false");
}

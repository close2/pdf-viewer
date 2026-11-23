//! The `script_corpus` walks' synthetic commit of a choice field without Table 233's `Edit` flag:
//! entered by its selection, so its `/K` and `/V` run (ADR 3470).
//!
//! The field `Country` is `FOP-2216-4.pdf`'s, one of the ten ADR 3453 found untaken: a combo box
//! with bit 19 clear, Table 234's `/Opt` of two pairs, no `/V`, and a `/V` action that logs. ISO
//! 32000-2 §12.7.5.4: "The default value of V is null , indicating that no item is currently
//! selected", so the empty selection is the value it holds; typed text is refused it unless the text
//! is a label (ADR 3311), and the empty string is none.

#![cfg(feature = "engine")]
#![expect(
    clippy::expect_used,
    reason = "test code: an explanatory panic is the intended failure"
)]

use std::fmt::Write as _;
use std::sync::Arc;

use pdf_model::view::{Entered, ViewState};
use pdf_script::{Budget, Engine};
use pdf_syntax::Document;

#[path = "../../pdf-model/tests/support/synthetic_commit.rs"]
mod synthetic_commit;

/// The validate script each combo box carries: it writes its field's name into `Out`.
fn validate(name: &str) -> String {
    format!("<< /V << /S /JavaScript /JS (this.getField\\('Out'\\).value = '{name}';) >> >>")
}

/// One page: `Country` (FOP-2216's field, no value), `Region` (its value `spn` an export value, as
/// pdf.js's `issue17998.pdf` writes one), `Picked` (its value `Holland`, a label), and the text
/// field `Out`.
fn form() -> Document {
    let combo = |name: &str, value: &str| {
        format!(
            "<< /Type /Annot /Subtype /Widget /FT /Ch /Ff 131072 /T ({name}) {value} \
             /Rect [10 50 210 70] /F 4 /P 3 0 R /DA (/Helv 10 Tf 0 g) \
             /Opt [[(spn) (Spain)] [(hol) (Holland)]] /AA {} >>",
            validate(name)
        )
    };
    let bodies = [
        "<< /Type /Catalog /Pages 2 0 R /AcroForm << /Fields [4 0 R 5 0 R 6 0 R 7 0 R] >> >>"
            .to_owned(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 /MediaBox [0 0 612 792] >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R /Annots [4 0 R 5 0 R 6 0 R 7 0 R] >>".to_owned(),
        combo("Country", ""),
        combo("Region", "/V (spn)"),
        combo("Picked", "/V (Holland)"),
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

/// A view state of `document` as the walks make one: the engine supplied, sequences held to the
/// walks' budget, and the document opened.
fn opened(document: &Document) -> ViewState {
    let mut view = ViewState::of(document);
    view.hold_sequences_to(synthetic_commit::WALK_SEQUENCE_BUDGET);
    view.run_scripts_with(Some(Arc::new(Engine::new(Budget::FIELD_EVENT))));
    view.run_open_scripts(document, 0);
    view
}

/// What `name` shows, as text.
fn shown(view: &ViewState, document: &Document, name: &str) -> String {
    view.field_value(document, name)
        .map_or_default(|shown| shown.text)
}

/// The walk's synthetic commit of `name`: what it enters, and `Out` after the commit.
fn committed(name: &str) -> (Option<Entered>, String) {
    let document = form();
    let mut view = opened(&document);
    let selection = synthetic_commit::selection_fields(&document, &view);
    let text = shown(&view, &document, name);
    let entered = synthetic_commit::re_entered(&document, &view, name, text, &selection);
    if let Some(entered) = &entered {
        assert_eq!(
            view.set_field(&document, name, entered),
            1,
            "{name} takes {entered:?}"
        );
        view.commit_field(&document, name);
    }
    (entered, shown(&view, &document, "Out"))
}

#[test]
fn a_selection_only_field_with_no_value_is_committed_by_its_empty_selection() {
    let document = form();
    let mut view = opened(&document);
    assert_eq!(
        view.set_field(&document, "Country", &Entered::Text(String::new())),
        0,
        "the walk's old commit, the text it shows, is refused (ADR 3311)"
    );
    assert_eq!(
        committed("Country"),
        (Some(Entered::Chosen(Vec::new())), "Country".to_owned()),
        "entered as the empty selection, its /V runs"
    );
}

#[test]
fn a_selection_only_field_whose_value_names_a_label_is_committed_by_that_option() {
    assert_eq!(
        committed("Picked"),
        (Some(Entered::Chosen(vec![1])), "Picked".to_owned())
    );
}

#[test]
fn a_selection_only_field_whose_value_names_no_option_is_not_entered() {
    // §12.7.5.4: "(For items represented in the Opt array by a two-element array, the name string
    // is the second of the two array elements.)" `spn` is the first, so it selects nothing (ADR
    // 0248), and no choice in the list makes the value it holds.
    assert_eq!(committed("Region"), (None, String::new()));
}

//! §12.7.4.1 inheritance and §12.7.6.3 reset, followed through public form reads, page drawing,
//! and the saved file. The sibling's local /DV is an independent control for nearest-value lookup.

#![expect(
    clippy::expect_used,
    reason = "a failing assertion is the purpose of this integration test"
)]

use std::fmt::Write as _;

use pdf_model::{Pages, content::interpret_with, view::ViewState};
use pdf_syntax::{Document, FileBytes};

fn pdf() -> Vec<u8> {
    let objects = [
        "<< /Type /Catalog /Pages 2 0 R /AcroForm << /Fields [4 0 R] /NeedAppearances true \
         /DA (/Helv 12 Tf 0 g) /DR << /Font << /Helv 7 0 R >> >> >> >>",
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 240 100] /Resources << >> \
         /Annots [5 0 R 6 0 R] >>",
        "<< /T (group) /FT /Tx /V (stored) /DV (shared-default) /Kids [8 0 R 9 0 R] >>",
        "<< /Type /Annot /Subtype /Widget /Parent 8 0 R /F 4 /Rect [10 55 220 85] >>",
        "<< /Type /Annot /Subtype /Widget /Parent 9 0 R /F 4 /Rect [10 10 220 40] >>",
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>",
        "<< /Parent 4 0 R /T (first) /Kids [5 0 R] >>",
        "<< /Parent 4 0 R /T (second) /DV (local-default) /Kids [6 0 R] >>",
    ];
    let mut out = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
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

fn page_text(document: &Document, view: &ViewState) -> String {
    let page = Pages::new(document).get(0).expect("one declared page");
    let interpretation = interpret_with(document, &page, view);
    assert!(
        interpretation.is_complete(),
        "the form appearance is supported: {:?}",
        interpretation.unsupported
    );
    interpretation.text
}

fn reset(document: &Document, view: &mut ViewState) {
    view.perform(
        document,
        &pdf_model::action::Action::ResetForm(pdf_model::action::ResetForm {
            fields: Vec::new(),
            exclude: false,
        }),
    );
}

fn assert_reset_values(document: &Document, view: &ViewState) {
    assert_eq!(
        view.field_value(document, "group.first")
            .map(|value| value.text),
        Some("shared-default".to_owned()),
        "the /DV absent on this leaf is inherited"
    );
    assert_eq!(
        view.field_value(document, "group.second")
            .map(|value| value.text),
        Some("local-default".to_owned()),
        "a leaf's own /DV takes precedence over the ancestor's"
    );
    let drawn = page_text(document, view);
    assert!(drawn.contains("shared-default"), "first widget: {drawn:?}");
    assert!(drawn.contains("local-default"), "second widget: {drawn:?}");
    assert!(
        !drawn.contains("stored"),
        "stale inherited value: {drawn:?}"
    );
}

#[test]
fn reset_uses_inherited_and_nearest_defaults_in_both_terminal_fields() {
    let document = Document::open(pdf()).expect("valid PDF with an explicit xref");
    assert!(
        !document.was_recovered(),
        "the fixture xref is authoritative"
    );
    let mut view = ViewState::of(&document);

    assert_eq!(
        view.field_value(&document, "group.first")
            .map(|value| value.text),
        Some("stored".to_owned()),
        "the first terminal field inherits /V from its nonterminal ancestor"
    );
    assert_eq!(
        view.field_value(&document, "group.second")
            .map(|value| value.text),
        Some("stored".to_owned()),
        "the sibling begins with the same inherited /V"
    );
    let before = page_text(&document, &view);
    assert!(
        before.contains("stored"),
        "both fields share stored /V: {before:?}"
    );

    reset(&document, &mut view);
    assert_reset_values(&document, &view);
}

#[test]
fn reset_saves_each_terminal_fields_effective_default() {
    let document = Document::open(pdf()).expect("valid PDF with an explicit xref");
    assert!(
        !document.was_recovered(),
        "the fixture xref is authoritative"
    );
    let mut view = ViewState::of(&document);
    let before = page_text(&document, &view);
    assert!(
        before.contains("stored"),
        "control before reset: {before:?}"
    );
    reset(&document, &mut view);

    let written = view.save(&document).expect("the reset is saved");
    let reopened = Document::open(FileBytes::from(written.bytes)).expect("updated PDF reopens");
    assert!(
        !reopened.was_recovered(),
        "the incremental update parses cleanly"
    );
    let reopened_view = ViewState::of(&reopened);
    assert_reset_values(&reopened, &reopened_view);
}

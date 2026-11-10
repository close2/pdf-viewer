//! Selected effective text/choice rules through complete PDF reader and edit fixtures.

#![expect(
    clippy::expect_used,
    reason = "complete fixture setup and precise API contract assertions"
)]

use std::fmt::Write as _;

use pdf_model::form::{Control, fields};
use pdf_model::view::ViewState;
use pdf_syntax::Document;

// PDF1.7 allows missing appearances when NeedAppearances=true. This avoids a blank
// stored appearance contradicting a nonempty V; native controls read the same model.
fn pdf_with_field(field: &str) -> Vec<u8> {
    let objects = [
        "<< /Type /Catalog /Pages 2 0 R /AcroForm << /Fields [4 0 R] /NeedAppearances true /DA (/Helv 12 Tf 0 g) /DR << /Font << /Helv 6 0 R >> >> >> >>".to_owned(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 300 200] /Resources << >> \
         /Contents 5 0 R /Annots [4 0 R] >>"
            .to_owned(),
        field.to_owned(),
        "<< /Length 0 >>\nstream\n\nendstream".to_owned(),
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_owned(),
    ];
    let mut out = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for (index, object) in objects.iter().enumerate() {
        offsets.push(out.len());
        let _ = writeln!(out, "{} 0 obj\n{object}\nendobj", index.saturating_add(1));
    }
    let xref = out.len();
    let _ = writeln!(out, "xref\n0 7\n0000000000 65535 f ");
    for offset in offsets {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    let _ = write!(
        out,
        "trailer\n<< /Size 7 /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n"
    );
    out.into_bytes()
}

fn read_field(field: &str) -> pdf_model::form::FormField {
    let document = Document::open(pdf_with_field(field)).expect("complete field fixture");
    assert!(!document.was_recovered(), "xref offsets are exact");
    let page = pdf_model::Pages::new(&document)
        .get(0)
        .expect("one complete page");
    let view = ViewState::of(&document);
    let fields = fields(&document, &page, &view);
    assert_eq!(fields.len(), 1, "fixture has one field: {fields:?}");
    fields.into_iter().next().expect("one field")
}

#[test]
fn a_sort_flag_does_not_reorder_choice_options_for_a_reader() {
    let prefix = "<< /Type /Annot /Subtype /Widget /Rect [10 10 290 50] \
                  /FT /Ch /T (sort-order) /Opt [(Zebra) (Alpha)]";
    let unsorted = read_field(&format!("{prefix} >>"));
    let sorted_flag = read_field(&format!("{prefix} /Ff 524288 >>")); // bit 20
    let labels = |field: &pdf_model::form::FormField| {
        let Control::Choice(choice) = &field.control else {
            panic!("expected choice field, got {:?}", field.control)
        };
        choice
            .options
            .iter()
            .map(|option| option.label.clone())
            .collect::<Vec<_>>()
    };
    assert_eq!(labels(&unsorted), ["Zebra", "Alpha"]);
    assert_eq!(
        labels(&sorted_flag),
        ["Zebra", "Alpha"],
        "Table 233 bit 20 is a writer instruction; readers retain /Opt order"
    );
}

#[test]
fn choice_value_wins_when_the_stated_index_disagrees() {
    // Same export value, distinct display names: the post-EC3 /I rule's tie case. Deliberately
    // contradictory /I points at Alpha while /V names Beta; the clause says /V wins a mismatch.
    let field = read_field(
        "<< /Type /Annot /Subtype /Widget /Rect [10 10 290 50] /FT /Ch /T (choice) \
         /Opt [[(same) (Alpha)] [(same) (Beta)]] /V (Beta) /I [0] >>",
    );
    let Control::Choice(choice) = field.control else {
        panic!("expected choice field")
    };
    assert_eq!(
        choice.selected,
        vec![1],
        "the value-selected option wins over /I"
    );
    assert_eq!(
        choice
            .options
            .iter()
            .map(|option| option.export.as_deref())
            .collect::<Vec<_>>(),
        [Some("same"), Some("same")]
    );
}

#[test]
fn max_len_zero_survives_reading_as_a_real_limit() {
    // EC3 Issue #191 (physical p. 556 / printed p. 541) makes Table 232's integer explicitly
    // greater-than-or-equal-to zero. Do not collapse a valid zero limit into absence.
    let field = read_field(
        "<< /Type /Annot /Subtype /Widget /Rect [10 10 290 50] /FT /Tx /T (empty-only) \
         /MaxLen 0 >>",
    );
    let Control::Text(text) = field.control else {
        panic!("expected text field")
    };
    assert_eq!(text.max_len, Some(0));
}

#[test]
fn a_null_text_value_is_not_mistaken_for_a_string_to_display() {
    let stated = read_field(
        "<< /Type /Annot /Subtype /Widget /Rect [10 10 290 50] /FT /Tx /T (status) \
         /V (pending) >>",
    );
    let null = read_field(
        "<< /Type /Annot /Subtype /Widget /Rect [10 10 290 50] /FT /Tx /T (status) \
         /V null >>",
    );
    assert_eq!(
        stated.value.as_ref().map(|value| value.text.as_str()),
        Some("pending")
    );
    assert!(
        null.value
            .as_ref()
            .is_none_or(|value| value.text.is_empty()),
        "a null value must not become visible text; an empty ShownValue is a valid API representation"
    );
}

//! Reader-level regression for duplicate display strings in a multi-select choice field.
//!
//! EC3 Issue #468 changes Table 234's `/I` wording: it is not limited to fields whose
//! `MultiSelect` flag is set, and deletes the former special trigger for array-valued `/V`.
//! Table 234 still says `/V` wins if `/I` identifies different items. Here both `/V` values
//! are the same label, so `/V` identifies the same two options that `/I [0 1]` does; the index
//! array distinguishes two different entries with duplicate display names.

#![expect(
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "a fixture which cannot exercise the reader must fail loudly"
)]

use std::fmt::Write as _;

use pdf_model::form::Control;
use pdf_syntax::Document;
use viewer_core::{Answer, Command, DocumentId, Query, Viewer};

const DOC: DocumentId = DocumentId(431);

fn pdf(value: &str, indices: &str) -> Vec<u8> {
    let bodies = [
        "<< /Type /Catalog /Pages 2 0 R /AcroForm << /Fields [4 0 R] /NeedAppearances true /DA (/F1 12 Tf 0 g) /DR << /Font << /F1 5 0 R >> >> >> >>"
            .to_owned(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 300 200] /Resources << >> /Annots [4 0 R] >>"
            .to_owned(),
        format!(
            "<< /Type /Annot /Subtype /Widget /Rect [10 10 290 50] /F 4 /FT /Ch /T (choice) \
             /DA (/F1 12 Tf 0 g) /Ff 2097152 /Opt [[(x)(Twin)] [(y)(Twin)] (Other)] /V {value} /I {indices} >>"
        ),
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_owned(),
    ];
    let mut out = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for (i, body) in bodies.iter().enumerate() {
        offsets.push(out.len());
        let _ = writeln!(out, "{} 0 obj\n{body}\nendobj", i.saturating_add(1));
    }
    let xref = out.len();
    let _ = writeln!(out, "xref\n0 6\n0000000000 65535 f ");
    for offset in offsets {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    let _ = write!(
        out,
        "trailer\n<< /Size 6 /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n"
    );
    out.into_bytes()
}

fn selected(bytes: Vec<u8>) -> Vec<usize> {
    let parsed = Document::open(bytes.clone()).expect("fixture parses");
    assert!(
        !parsed.was_recovered(),
        "fixture has exact classic-xref offsets"
    );
    let mut viewer = Viewer::new(300, 200, 1.0);
    viewer
        .handle(Command::Open {
            id: DOC,
            bytes: bytes.into(),
            password: None,
            fragment: None,
        })
        .for_each(drop);
    let Answer::Fields(fields) = viewer.query(Query::Fields) else {
        panic!("open page exposes the choice through the public field query")
    };
    let [field] = fields.as_slice() else {
        panic!("fixture has exactly one widget field: {fields:?}")
    };
    let Control::Choice(choice) = &field.control else {
        panic!("field is a choice: {:?}", field.control)
    };
    assert_eq!(choice.options[0].export.as_deref(), Some("x"));
    assert_eq!(choice.options[1].export.as_deref(), Some("y"));
    assert_eq!(choice.options[0].label, "Twin");
    assert_eq!(choice.options[1].label, "Twin");
    choice.selected.clone()
}

#[test]
fn duplicate_labels_keep_both_array_selected_indices() {
    let selected_with_duplicate_labels = selected(pdf("[(Twin) (Twin)]", "[0 1]"));
    assert_eq!(
        selected_with_duplicate_labels,
        [0, 1],
        "V and I identify both duplicate-label choices; collapsing to one loses a selection"
    );
}

#[test]
fn distinct_label_control_confirms_option_order_and_multi_selection() {
    // Independent positive control for two values and two indices that need no duplicate-label
    // disambiguation. This catches an accidental fixture or parser issue unrelated to the case.
    let selected_distinct = selected(pdf("[(Twin) (Other)]", "[0 2]"));
    assert_eq!(selected_distinct, [0, 2]);
}

#[test]
fn one_duplicate_label_is_disambiguated_by_its_single_index() {
    assert_eq!(selected(pdf("(Twin)", "[1]")), [1]);
}

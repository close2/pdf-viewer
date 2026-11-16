//! Table 175 defaults with `StateModel` present and State absent.
//! Explicit Completed and the two distinct default values distinguish the paths.

#![expect(
    clippy::arithmetic_side_effects,
    reason = "small exact-xref test fixtures; malformed setup must fail loudly"
)]

use std::fmt::Write as _;

use pdf_model::Pages;
use pdf_model::annotation_state::{State, StateModel, states};
use pdf_syntax::{Document, ObjectId};

fn fixture() -> Vec<u8> {
    let objects = [
        "<< /Type /Catalog /Pages 2 0 R >>",
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] /Resources << >> \
          /Annots [4 0 R 5 0 R 6 0 R 7 0 R] >>",
        "<< /Type /Annot /Subtype /Text /Rect [10 10 30 30] /T (original) >>",
        "<< /Type /Annot /Subtype /Text /Rect [10 10 30 30] /IRT 4 0 R /T (Ada) \
          /StateModel (Marked) >>",
        "<< /Type /Annot /Subtype /Text /Rect [10 10 30 30] /IRT 4 0 R /T (Ada) \
          /StateModel (Review) >>",
        "<< /Type /Annot /Subtype /Text /Rect [10 10 30 30] /IRT 4 0 R /T (Grace) \
          /State (Completed) /StateModel (Review) >>",
    ];
    let mut bytes = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for (index, body) in objects.iter().enumerate() {
        offsets.push(bytes.len());
        let _ = writeln!(bytes, "{} 0 obj\n{body}\nendobj", index + 1);
    }
    let xref_at = bytes.len();
    let size = objects.len() + 1;
    let _ = writeln!(bytes, "xref\n0 {size}\n0000000000 65535 f ");
    for offset in offsets {
        let _ = writeln!(bytes, "{offset:010} 00000 n ");
    }
    let _ = write!(
        bytes,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n"
    );
    bytes.into_bytes()
}

#[test]
fn a_state_model_without_state_uses_its_table_default() {
    let document = Document::open(fixture()).expect("exact-xref PDF opens");
    assert!(!document.was_recovered(), "fixture xref offsets are exact");
    let page = Pages::new(&document).get(0).expect("one page");
    let current = states(&document, &page, ObjectId::new(4, 0));

    assert_eq!(current.len(), 3);
    assert_eq!(current[0].user.as_deref(), Some("Ada"));
    assert_eq!(current[0].model, StateModel::Marked);
    assert_eq!(current[0].state, State::Unmarked);
    assert_eq!(current[0].set_by, ObjectId::new(5, 0));
    assert_eq!(current[1].user.as_deref(), Some("Ada"));
    assert_eq!(current[1].model, StateModel::Review);
    assert_eq!(current[1].state, State::None);
    assert_eq!(current[1].set_by, ObjectId::new(6, 0));
    assert_eq!(current[2].user.as_deref(), Some("Grace"));
    assert_eq!(current[2].model, StateModel::Review);
    assert_eq!(current[2].state, State::Completed, "explicit state control");
}

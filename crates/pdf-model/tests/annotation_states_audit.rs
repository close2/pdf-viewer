//! Complete-PDF test for a valid state chain with a comment-only reply in between.
#![expect(
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "the controlled fixture is assembled with an exact xref and should fail loudly if malformed"
)]

use pdf_model::Pages;
use pdf_model::annotation_state::{State, StateModel, states};
use pdf_syntax::{Document, ObjectId};

fn assemble(objects: &[&[u8]]) -> Vec<u8> {
    let mut output = b"%PDF-1.7\n".to_vec();
    let mut offsets = Vec::new();
    for (number, object) in (1_usize..).zip(objects) {
        offsets.push(output.len());
        output.extend_from_slice(format!("{number} 0 obj\n").as_bytes());
        output.extend_from_slice(object);
        output.extend_from_slice(b"\nendobj\n");
    }
    let xref = output.len();
    let size = offsets.len() + 1;
    output.extend_from_slice(format!("xref\n0 {size}\n0000000000 65535 f \n").as_bytes());
    for offset in offsets {
        output.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    output.extend_from_slice(
        format!("trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n").as_bytes(),
    );
    output
}

#[test]
fn a_state_chain_crosses_a_text_reply_without_a_state() {
    let pdf = assemble(&[
        b"<< /Type /Catalog /Pages 2 0 R >>",
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] /Resources << >> \
          /Annots [4 0 R 5 0 R 6 0 R 7 0 R] >>",
        b"<< /Type /Annot /Subtype /Text /Rect [10 10 30 30] /T (original author) >>",
        b"<< /Type /Annot /Subtype /Text /Rect [10 10 30 30] /IRT 4 0 R \
          /T (Ada) /State (Accepted) /StateModel (Review) >>",
        b"<< /Type /Annot /Subtype /Text /Rect [10 10 30 30] /IRT 5 0 R \
          /T (Ada) /Contents (A comment, not a state change) >>",
        b"<< /Type /Annot /Subtype /Text /Rect [10 10 30 30] /IRT 6 0 R \
          /T (Ada) /State (Completed) /StateModel (Review) >>",
    ]);
    let document = Document::open(pdf).expect("the exact-xref PDF fixture parses");
    assert!(!document.was_recovered(), "exact cross references");
    let page = Pages::new(&document).get(0).expect("one page");

    let current = states(&document, &page, ObjectId::new(4, 0));
    assert_eq!(current.len(), 1);
    assert_eq!(current[0].user.as_deref(), Some("Ada"));
    assert_eq!(current[0].model, StateModel::Review);
    assert_eq!(current[0].state, State::Completed);
    assert_eq!(current[0].set_by, ObjectId::new(7, 0));

    // The middle Text reply carries no State/StateModel, so it cannot overwrite the state, but it
    // remains the previous reply for Ada and can be the target of the later state change.
    // A state is attached to its target, not to the annotation that carries it.
    assert!(states(&document, &page, ObjectId::new(7, 0)).is_empty());
}

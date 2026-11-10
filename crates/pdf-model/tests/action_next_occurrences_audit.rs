//! Table 196: each non-cyclic Next-array occurrence contributes to the ordered sequence.
#![expect(
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    clippy::panic,
    reason = "a broken test fixture must fail loudly"
)]

use std::fmt::Write as _;

use pdf_model::action::{Action, Named, read};
use pdf_syntax::{Document, Object, ObjectId};

fn document(next: &str, extra: &[&str]) -> Document {
    let mut objects = vec![
        "<< /Type /Catalog /Pages 2 0 R /OpenAction 4 0 R >>".to_owned(),
        "<< /Type /Pages /Count 1 /Kids [3 0 R] >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] /Resources << >> >>".to_owned(),
        format!("<< /S /Named /N /FirstPage /Next {next} >>"),
    ];
    objects.extend(extra.iter().map(|object| (*object).to_owned()));

    let mut bytes = String::from("%PDF-1.7\n");
    let mut offsets = Vec::with_capacity(objects.len());
    for (index, body) in objects.iter().enumerate() {
        offsets.push(bytes.len());
        writeln!(bytes, "{} 0 obj\n{body}\nendobj", index + 1)
            .expect("writing to String cannot fail");
    }
    let xref = bytes.len();
    writeln!(bytes, "xref\n0 {}\n0000000000 65535 f ", objects.len() + 1)
        .expect("writing to String cannot fail");
    for offset in offsets {
        writeln!(bytes, "{offset:010} 00000 n ").expect("writing to String cannot fail");
    }
    writeln!(
        bytes,
        "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF",
        objects.len() + 1
    )
    .expect("writing to String cannot fail");
    let document = Document::open(bytes.into_bytes()).expect("complete valid PDF fixture");
    assert!(!document.was_recovered(), "fixture has exact xref offsets");
    document
}

fn id(number: u32) -> ObjectId {
    ObjectId {
        number,
        generation: 0,
    }
}

fn kinds(document: &Document) -> Vec<Named> {
    read(document, &Object::Reference(id(4)))
        .into_iter()
        .map(|action| match action {
            Action::Named(named) => named,
            other => panic!("the fixture only names page actions: {other:?}"),
        })
        .collect()
}

#[test]
fn next_array_reads_each_occurrence_even_when_two_entries_share_an_object() {
    // Positive control: distinct children establish the parent-first, array-order sequence.
    let control = document(
        "[5 0 R 6 0 R]",
        &[
            "<< /S /Named /N /NextPage >>",
            "<< /S /Named /N /PrevPage >>",
        ],
    );
    assert_eq!(
        kinds(&control),
        vec![Named::FirstPage, Named::NextPage, Named::PrevPage],
        "root then each distinct Next member in array order"
    );

    // Same action object occurs twice as two array members. That is repeated sequence data, not
    // a path from an action back to itself; NOTE 1's self-reference recommendation does not apply.
    let repeated = document("[5 0 R 5 0 R]", &["<< /S /Named /N /LastPage >>"]);
    assert_eq!(
        kinds(&repeated),
        vec![Named::FirstPage, Named::LastPage, Named::LastPage],
        "each array occurrence is performed, including a repeated indirect reference"
    );
}

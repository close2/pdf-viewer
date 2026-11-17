//! Exercises Table 156's typed date comparison and missing per-field /A default through the
//! public host row-order consumer. Expected chronology is computed explicitly in UTC below.

#![expect(
    clippy::panic,
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    reason = "an invalid exact-xref fixture or missing public collection answer must fail loudly"
)]

use std::fmt::Write as _;

use viewer_core::{Answer, Command, DocumentId, Query, Viewer};
use viewer_host::{RowAction, collection_rows};

fn pdf(ascending: &str) -> Vec<u8> {
    let objects = vec![
        "<< /Type /Catalog /Pages 2 0 R /Names << /EmbeddedFiles << /Names \
         [(a.txt) 4 0 R (b.txt) 5 0 R (c.txt) 6 0 R] >> >> /Collection 13 0 R >>"
            .to_owned(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Resources << >> >>".to_owned(),
        "<< /Type /Filespec /F (a.txt) /UF (a.txt) /CI 10 0 R /EF << /F 7 0 R >> >>".to_owned(),
        "<< /Type /Filespec /F (b.txt) /UF (b.txt) /CI 11 0 R /EF << /F 8 0 R >> >>".to_owned(),
        "<< /Type /Filespec /F (c.txt) /UF (c.txt) /CI 12 0 R /EF << /F 9 0 R >> >>".to_owned(),
        "<< /Type /EmbeddedFile /Length 3 >>\nstream\nabc\nendstream".to_owned(),
        "<< /Type /EmbeddedFile /Length 3 >>\nstream\ndef\nendstream".to_owned(),
        "<< /Type /EmbeddedFile /Length 3 >>\nstream\nghi\nendstream".to_owned(),
        "<< /Type /CollectionItem /rank 2 /received (D:20240101010000+02'00') >>".to_owned(),
        "<< /Type /CollectionItem /rank 2 /received (D:20231231233000Z) >>".to_owned(),
        "<< /Type /CollectionItem /rank 1 /received (D:20240101000000Z) >>".to_owned(),
        "<< /Type /Collection /D (a.txt) /View /D /Schema 14 0 R \
         /Sort << /S [/rank /received] /A {ascending} >> >>"
            .replace("{ascending}", ascending),
        "<< /Type /CollectionSchema \
         /rank << /Subtype /N /N (Rank) >> \
         /received << /Subtype /D /N (Received) >> >>"
            .to_owned(),
    ];
    let mut bytes = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for (index, body) in objects.iter().enumerate() {
        offsets.push(bytes.len());
        let _ = writeln!(bytes, "{} 0 obj\n{body}\nendobj", index + 1);
    }
    let xref = bytes.len();
    let size = objects.len() + 1;
    let _ = write!(bytes, "xref\n0 {size}\n0000000000 65535 f \n");
    for offset in offsets {
        let _ = writeln!(bytes, "{offset:010} 00000 n ");
    }
    let _ = writeln!(
        bytes,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF"
    );
    bytes.into_bytes()
}

fn public_collection_names(ascending: &str) -> Vec<String> {
    let bytes = pdf(ascending);
    let parsed = pdf_syntax::Document::open(bytes.clone()).expect("exact-xref PDF 1.7 opens");
    assert!(!parsed.was_recovered(), "no-recovery guard");
    assert!(
        !parsed.cross_reference_chain_was_cut(),
        "complete xref chain"
    );
    assert_eq!(
        pdf_model::Pages::new(&parsed).len(),
        1,
        "independent page-tree control"
    );

    let mut viewer = Viewer::new(800, 1000, 1.0);
    viewer
        .handle(Command::Open {
            id: DocumentId(1),
            bytes: bytes.into(),
            password: None,
            fragment: None,
        })
        .for_each(drop);
    let Answer::Collection {
        collection,
        initial,
        order,
    } = viewer.query(Query::Collection)
    else {
        panic!("the catalog's collection is reached by the public query");
    };
    let Answer::Attachments(attachments) = viewer.query(Query::Attachments) else {
        panic!("all three EmbeddedFiles entries are available to the row consumer");
    };
    assert_eq!(attachments.len(), 3, "all items are present before sorting");

    let rows = collection_rows(&collection, &initial, &order, &attachments);
    rows.iter()
        .filter_map(|row| match &row.action {
            RowAction::Extract { name } => Some(name.clone()),
            _ => None,
        })
        .collect()
}

#[test]
fn public_collection_rows_use_date_chronology_and_default_missing_direction_to_ascending() {
    // Date a = 2024-01-01 01:00 +02:00 = 2023-12-31 23:00Z; date b is 23:30Z.
    // Both Rank values are 2, so explicit [false true] makes the independent date-order
    // control place a before b despite their raw strings starting 2024 and 2023 respectively.
    // Rank 1 in c independently confirms the first key is descending. The shorter [false]
    // array is valid and must use the Table 156 default (true) for its omitted second entry.
    let explicit = public_collection_names("[false true]");
    assert_eq!(
        explicit,
        ["a.txt", "b.txt", "c.txt"],
        "explicit date-direction control"
    );

    let short = public_collection_names("[false]");
    assert_eq!(
        short,
        ["a.txt", "b.txt", "c.txt"],
        "omitted /A entry defaults to ascending"
    );
}

#[test]
fn public_collection_date_sort_has_explicit_and_scalar_direction_controls() {
    // The two rank-2 members are ordered chronologically, not by their PDF date strings.
    assert_eq!(
        public_collection_names("[false true]"),
        ["a.txt", "b.txt", "c.txt"]
    );
    assert_eq!(
        public_collection_names("[false false]"),
        ["b.txt", "a.txt", "c.txt"]
    );
    // A scalar false applies to all fields; unlike a one-entry array, it has no missing entry.
    assert_eq!(
        public_collection_names("false"),
        ["b.txt", "a.txt", "c.txt"]
    );
}

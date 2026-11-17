//! Regression for EC3 #101 / ISO 32000-2 §7.5.5 Table 15 and §7.5.8.
//!
//! This builds an exact-xref PDF 2.0 file: the newest xref stream's /Prev points to an older
//! xref stream. The only copy of the Catalog and Pages objects is in that older section, so
//! resolving the current page tree proves the chain was followed. `was_recovered == false`
//! prevents body scanning from serving as a false positive.

#![expect(
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    reason = "small hand-built exact-offset test fixture"
)]

use std::fmt::Write as _;

use pdf_syntax::{Document, Object, ObjectId};

fn add_object(bytes: &mut Vec<u8>, number: u32, body: &str) -> usize {
    let offset = bytes.len();
    bytes.extend_from_slice(format!("{number} 0 obj\n{body}\nendobj\n").as_bytes());
    offset
}

fn entry(kind: u8, offset: usize, generation: u16) -> [u8; 7] {
    let mut row = [0; 7];
    row[0] = kind;
    row[1..5].copy_from_slice(
        &u32::try_from(offset)
            .expect("fixture under 4 GiB")
            .to_be_bytes(),
    );
    row[5..7].copy_from_slice(&generation.to_be_bytes());
    row
}

fn append_xref_stream(
    bytes: &mut Vec<u8>,
    object_number: u32,
    size: u32,
    index: &[(u32, [u8; 7])],
    previous: Option<usize>,
) -> usize {
    let xref_offset = bytes.len();
    let mut data = Vec::with_capacity(index.len() * 7);
    let mut index_text = String::new();
    for (number, row) in index {
        data.extend_from_slice(row);
        let _ = write!(index_text, "{number} 1 ");
    }
    let prev = previous.map_or_else(String::new, |offset| format!("/Prev {offset} "));
    let changing_id = if previous.is_some() {
        "FFEEDDCCBBAA9988776655443322110001"
    } else {
        "00112233445566778899AABBCCDDEEFF"
    };
    let dict = format!(
        "{object_number} 0 obj\n<< /Type /XRef /Size {size} /Index [{index_text}] \
         /W [1 4 2] /Root 1 0 R {prev}/ID [<00112233445566778899AABBCCDDEEFF> \
         <{changing_id}>] /Length {} >>\nstream\n",
        data.len()
    );
    bytes.extend_from_slice(dict.as_bytes());
    bytes.extend_from_slice(&data);
    bytes.extend_from_slice(b"\nendstream\nendobj\n");
    bytes.extend_from_slice(format!("startxref\n{xref_offset}\n%%EOF\n").as_bytes());
    xref_offset
}

fn skeleton_with_first_xref_stream() -> (Vec<u8>, usize) {
    let mut bytes = b"%PDF-2.0\n%\xE2\xE3\xCF\xD3\n".to_vec();
    let catalog = add_object(&mut bytes, 1, "<< /Type /Catalog /Pages 2 0 R >>");
    let pages = add_object(&mut bytes, 2, "<< /Type /Pages /Count 1 /Kids [3 0 R] >>");
    let page = add_object(
        &mut bytes,
        3,
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] /Resources << >> >>",
    );
    let xref_object_number = 4;
    let xref_offset = bytes.len();
    let rows = [
        (0, entry(0, 0, u16::MAX)),
        (1, entry(1, catalog, 0)),
        (2, entry(1, pages, 0)),
        (3, entry(1, page, 0)),
        (4, entry(1, xref_offset, 0)),
    ];
    append_xref_stream(&mut bytes, xref_object_number, 5, &rows, None);
    (bytes, xref_offset)
}

#[test]
fn valid_incremental_xref_stream_prev_can_name_a_prior_xref_stream() {
    let (mut bytes, previous_xref) = skeleton_with_first_xref_stream();
    let updated_page = add_object(
        &mut bytes,
        3,
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] /Resources << >> /Marker (updated) >>",
    );
    let latest_xref = bytes.len();
    let latest_rows = [
        (3, entry(1, updated_page, 0)),
        (5, entry(1, latest_xref, 0)),
    ];
    append_xref_stream(&mut bytes, 5, 6, &latest_rows, Some(previous_xref));

    let document = Document::open(bytes).expect("valid two-revision PDF opens");
    assert!(
        !document.was_recovered(),
        "both xref streams, including the older stream named by /Prev, are valid"
    );
    assert!(
        !document.cross_reference_chain_was_cut(),
        "the previous stream was read without fallback"
    );
    assert!(
        document.catalog().is_ok(),
        "catalog is in the older xref section"
    );
    let pages = document.get(ObjectId {
        number: 2,
        generation: 0,
    });
    assert_eq!(
        pages
            .as_dict()
            .and_then(|dict| dict.get("Count"))
            .and_then(Object::as_integer),
        Some(1),
        "the page-tree object is reachable only through the prior xref stream"
    );
    let page = document.get(ObjectId {
        number: 3,
        generation: 0,
    });
    assert_eq!(
        page.as_dict()
            .and_then(|dict| dict.get("Marker"))
            .and_then(Object::as_string),
        Some(&b"updated"[..]),
        "the newest section's replacement wins while older objects are retained"
    );
}

#[test]
fn the_single_stream_revision_is_a_positive_control_for_the_incremental_fixture() {
    let (bytes, _) = skeleton_with_first_xref_stream();
    let document = Document::open(bytes).expect("valid one-revision PDF opens");
    assert!(!document.was_recovered(), "the base xref stream is exact");
    assert!(
        !document.cross_reference_chain_was_cut(),
        "no lazy repair of an unread predecessor"
    );
    assert!(document.catalog().is_ok());
    let page = document.get(ObjectId {
        number: 3,
        generation: 0,
    });
    assert!(page.as_dict().is_some());
    assert!(page.as_dict().and_then(|dict| dict.get("Marker")).is_none());
}

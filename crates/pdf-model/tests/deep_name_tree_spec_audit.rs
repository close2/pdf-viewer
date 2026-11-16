//! Valid multi-level §7.9.6 producer tree through the public named-destination reader.
#![expect(
    clippy::arithmetic_side_effects,
    reason = "bounded exact-xref fixture and explicit public-reader expectations"
)]

use std::io::Write as _;

use pdf_model::{Pages, destination::Destination};
use pdf_syntax::{Document, Object};

fn pdf() -> Vec<u8> {
    let objects: [&[u8]; 13] = [
        b"<< /Type /Catalog /Pages 2 0 R /Names << /Dests 5 0 R >> >>",
        b"<< /Type /Pages /Kids [3 0 R 4 0 R] /Count 2 >>",
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Resources << >> >>",
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Resources << >> >>",
        // Root -> intermediate -> leaf, with exact byte ranges and indirect Kids at each level.
        b"<< /Kids [6 0 R 10 0 R] >>",
        b"<< /Limits [(A) (a)] /Kids [7 0 R] >>",
        b"<< /Limits [(A) (a)] /Names [(A) 8 0 R (a) 9 0 R] >>",
        b"[3 0 R /Fit]",
        b"[4 0 R /Fit]",
        b"<< /Limits [(b) (z)] /Kids [11 0 R] >>",
        b"<< /Limits [(b) (z)] /Names [(b) 12 0 R (z) 13 0 R] >>",
        b"[4 0 R /Fit]",
        b"[3 0 R /Fit]",
    ];
    let mut out = b"%PDF-1.7\n".to_vec();
    let mut offsets = Vec::new();
    for (index, object) in objects.iter().enumerate() {
        offsets.push(out.len());
        out.extend_from_slice(format!("{} 0 obj\n", index + 1).as_bytes());
        out.extend_from_slice(object);
        out.extend_from_slice(b"\nendobj\n");
    }
    let xref = out.len();
    let size = objects.len() + 1;
    let _ = write!(out, "xref\n0 {size}\n0000000000 65535 f \n");
    for offset in offsets {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    let _ = write!(
        out,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n"
    );
    out
}

#[test]
fn named_destination_lookup_descends_valid_indirect_nodes_and_uses_exact_byte_keys() {
    let bytes = pdf();
    let document = Document::open(bytes).expect("generated PDF parses");
    assert!(!document.was_recovered(), "all xref offsets are exact");
    let pages = Pages::new(&document);

    for (key, expected_page) in [(b"A".as_slice(), 0), (b"a", 1), (b"b", 1), (b"z", 0)] {
        let destination = Destination::read(&document, &Object::String(key.to_vec().into()))
            .expect("the key is present in a valid leaf");
        assert_eq!(
            destination.page_index(&document, &pages),
            Some(expected_page)
        );
    }
    assert!(Destination::read(&document, &Object::String(b"B".to_vec().into())).is_none());
}

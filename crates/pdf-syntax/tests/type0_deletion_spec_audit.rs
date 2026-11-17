//! Producer-valid type-0 deletion in a hybrid-reference update.
//! The xref-stream W widths are all nonzero, Index ranges are sorted/disjoint,
//! object 0 and deleted object 4 form a linked free list, and the companion classic
//! table carries object 5's required xref-stream self-entry.

#![expect(
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    reason = "small exact-offset fixture"
)]

use std::fmt::Write as _;

use pdf_syntax::{Document, Object, ObjectId};

fn u64_field(value: u64, width: usize, into: &mut Vec<u8>) {
    let bytes = value.to_be_bytes();
    into.extend_from_slice(bytes.get(8 - width..).expect("field width <= 8"));
}

fn fixture(with_deletion: bool) -> Vec<u8> {
    let mut bytes = b"%PDF-1.7\n%\xE2\xE3\xCF\xD3\n".to_vec();
    let mut offsets = Vec::new();
    for body in [
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n",
        "2 0 obj\n<< /Type /Pages /Count 1 /Kids [3 0 R] >>\nendobj\n",
        "3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] /Resources << >> >>\nendobj\n",
        "4 0 obj\n(the original)\nendobj\n",
    ] {
        offsets.push(bytes.len());
        bytes.extend_from_slice(body.as_bytes());
    }

    let previous = bytes.len();
    let _ = writeln!(StringWriter(&mut bytes), "xref\n0 5");
    let _ = writeln!(StringWriter(&mut bytes), "0000000000 65535 f ");
    for offset in &offsets {
        let _ = writeln!(StringWriter(&mut bytes), "{offset:010} 00000 n ");
    }
    let _ = write!(
        StringWriter(&mut bytes),
        "trailer\n<< /Size 5 /Root 1 0 R >>\nstartxref\n{previous}\n%%EOF\n"
    );
    if !with_deletion {
        return bytes;
    }

    let stream_at = bytes.len();
    let mut records = Vec::with_capacity(14);
    // Table18 type 0: object 0 links to newly freed object 4; its generation is 65535.
    records.push(0);
    u64_field(4, 4, &mut records);
    u64_field(65535, 2, &mut records);
    // Object 4 links back to the free-list sentinel and has next-use generation 1.
    records.push(0);
    u64_field(0, 4, &mut records);
    u64_field(1, 2, &mut records);

    let header = format!(
        "5 0 obj\n<< /Type /XRef /Size 6 /Root 1 0 R \
         /Index [0 1 4 1] /W [1 4 2] /Length {} >>\nstream\n",
        records.len()
    );
    bytes.extend_from_slice(header.as_bytes());
    bytes.extend_from_slice(&records);
    bytes.extend_from_slice(b"\nendstream\nendobj\n");

    // In a hybrid update, the companion classic table carries the stream object's own entry.
    let table_at = bytes.len();
    let _ = write!(
        StringWriter(&mut bytes),
        "xref\n5 1\n{stream_at:010} 00000 n \n\
         trailer\n<< /Size 6 /Root 1 0 R /XRefStm {stream_at} /Prev {previous} >>\n\
         startxref\n{table_at}\n%%EOF\n"
    );
    bytes
}

struct StringWriter<'a>(&'a mut Vec<u8>);

impl std::fmt::Write for StringWriter<'_> {
    fn write_str(&mut self, value: &str) -> std::fmt::Result {
        self.0.extend_from_slice(value.as_bytes());
        Ok(())
    }
}

#[test]
fn type_zero_with_valid_width_in_hybrid_update_deletes_the_old_entry() {
    let before = Document::open(fixture(false)).expect("valid classic baseline");
    assert!(!before.was_recovered());
    assert_eq!(
        before.get(ObjectId::new(4, 0)).as_string(),
        Some(&b"the original"[..]),
        "old in-use object is the positive control"
    );

    let after = Document::open(fixture(true)).expect("valid hybrid xref update");
    assert!(
        !after.was_recovered(),
        "the table and xref stream are read directly"
    );
    assert!(
        !after.cross_reference_chain_was_cut(),
        "the prior classic table is followed"
    );
    assert!(after.catalog().is_ok(), "catalog remains reachable");
    assert!(
        after.get(ObjectId::new(5, 0)).as_stream().is_some(),
        "classic table carries the hybrid stream self-entry"
    );
    assert_eq!(
        after.get(ObjectId::new(4, 0)),
        Object::Null,
        "the hybrid stream type-0 entry explicitly deletes the older object"
    );
    assert_eq!(
        after.get(ObjectId::new(4, 1)),
        Object::Null,
        "generation 1 is the next reusable generation, not a live object"
    );
}

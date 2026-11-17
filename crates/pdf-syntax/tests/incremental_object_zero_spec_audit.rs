//! Regression test for EC3 Issue #147 / §7.5.4.
//!
//! This is intentionally a valid incremental update: it deletes object 5 from a valid base.
//! The target update puts object 0 and the deleted object in separate subsections so object 0
//! can head the linked free list. The control uses the permitted detached-free-entry mechanism.

#![expect(
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    reason = "fixed exact-offset fixture construction fails loudly"
)]

use std::fmt::Write as _;

use pdf_syntax::object::{Object, ObjectId};
use pdf_syntax::{Document, xref};

fn classic() -> Vec<u8> {
    let body = "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 20 10] /Resources << >> >>\nendobj\n\
         4 0 obj\n<< /Producer (control) >>\nendobj\n\
         5 0 obj\n123\nendobj\n"
        .to_owned();

    let mut bytes = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for object in body.split_inclusive("endobj\n") {
        offsets.push(bytes.len());
        bytes.push_str(object);
    }
    let xref_at = bytes.len();
    let size = offsets.len().saturating_add(1);
    let _ = writeln!(bytes, "xref\n0 {size}");
    // ISO 32000-2 §7.5.4: object 0 is free, is generation 65 535 and begins the free list.
    bytes.push_str("0000000000 65535 f \n");
    for offset in offsets {
        let _ = writeln!(bytes, "{offset:010} 00000 n ");
    }
    let _ = write!(
        bytes,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n"
    );
    bytes.into_bytes()
}

fn last_startxref(bytes: &[u8]) -> usize {
    let text = String::from_utf8_lossy(bytes);
    let marker = text.rfind("startxref\n").expect("fixture has startxref");
    text[marker + "startxref\n".len()..]
        .lines()
        .next()
        .expect("startxref has an offset")
        .parse()
        .expect("offset is decimal")
}

fn append_free_object_five(rehead_zero: bool) -> (Vec<u8>, usize) {
    let mut bytes = classic();
    let base = Document::open(bytes.clone()).expect("base is a valid PDF");
    assert!(
        !base.was_recovered(),
        "base xref and object-zero entry are valid"
    );
    assert!(base.xref().location(5).is_some(), "object 5 begins in use");
    assert_eq!(
        base.get(ObjectId {
            number: 5,
            generation: 0
        }),
        Object::Integer(123)
    );
    let previous = last_startxref(&bytes);
    let xref_at = bytes.len();
    if rehead_zero {
        // Add object 5 to the linked list: object 0 points to 5; object 5 points back to 0.
        // The two noncontiguous subsections exercise the corrected rule that subsection first
        // object numbers are non-negative, including zero.
        bytes.extend_from_slice(b"xref\n0 1\n0000000005 65535 f \n5 1\n0000000000 00001 f \n");
    } else {
        // Control: the second permitted mechanism leaves object 5 outside the linked list.
        bytes.extend_from_slice(b"xref\n5 1\n0000000000 65535 f \n");
    }
    let _ = write!(
        StringSink(&mut bytes),
        "trailer\n<< /Size 6 /Root 1 0 R /Prev {previous} >>\nstartxref\n{xref_at}\n%%EOF\n"
    );
    (bytes, xref_at)
}

struct StringSink<'a>(&'a mut Vec<u8>);

impl std::fmt::Write for StringSink<'_> {
    fn write_str(&mut self, value: &str) -> std::fmt::Result {
        self.0.extend_from_slice(value.as_bytes());
        Ok(())
    }
}

#[test]
fn incremental_object_zero_subsection_is_read_without_recovery() {
    for (rehead_zero, expected_free_line) in [
        (false, "0000000000 65535 f \n"),
        (true, "0000000005 65535 f \n"),
    ] {
        let (bytes, new_xref) = append_free_object_five(rehead_zero);
        let newest = String::from_utf8_lossy(&bytes[new_xref..]);
        if rehead_zero {
            assert!(newest.starts_with("xref\n0 1\n"));
            assert!(newest.contains("0000000005 65535 f \n"));
            assert!(newest.contains("5 1\n0000000000 00001 f \n"));
        } else {
            assert!(newest.starts_with("xref\n5 1\n"));
            assert!(newest.contains("5 1\n0000000000 65535 f \n"));
        }
        assert!(
            bytes
                .windows(expected_free_line.len())
                .any(|w| w == expected_free_line.as_bytes()),
            "object 0 is the free-list head with generation 65 535"
        );

        let document = Document::open(bytes).expect("both update forms are valid");
        assert!(
            !document.was_recovered(),
            "valid xref data must not need repair"
        );
        assert!(document.catalog().is_ok(), "the /Root remains available");
        assert!(
            !document.cross_reference_chain_was_cut(),
            "the /Prev chain is complete"
        );
        assert!(
            document
                .get(ObjectId {
                    number: 5,
                    generation: 0
                })
                .is_null()
        );
        assert!(document.xref().location(5).is_none());

        let sections = xref::sections(document.bytes(), document.limits());
        assert_eq!(sections.len(), 2, "base then free-object section");
        assert!(sections[0].classic);
        assert_eq!(sections[0].offset, new_xref);
    }
}

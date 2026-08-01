//! ISO 32000-2 §7.5.6's incremental update, written and then read back.
//!
//! # Why the reader is the test
//!
//! An update is bytes appended to a file, and the only statement worth making about it is that a
//! *reader* finds what it says. This tree has one — the same one 974 corpus documents go
//! through — so every test here writes an update, opens the result from scratch, and asks the
//! reader what the document now says.
//!
//! That is stronger than comparing the bytes with an expected string, and it is stronger in the
//! way that matters: it fails if the offsets are wrong, if the `/Prev` chain is broken, if the
//! subsection headers do not match the entries, or if the object was written in a syntax the
//! lexer does not accept. None of those is visible in a diff.
//!
//! # What it also asserts, every time
//!
//! **The original bytes are still there, unchanged.** §7.5.6's whole point is that changes are
//! appended "leaving its original contents intact", and a file this program has saved must still
//! contain whatever was signed, archived or notarised in it.

#![expect(
    clippy::panic,
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "test code: a fixture that cannot exercise the rule must fail loudly rather than \
              pass by doing nothing"
)]

use std::collections::BTreeMap;
use std::fmt::Write as _;

use pdf_syntax::object::{Dictionary, Name, Object, ObjectId};
use pdf_syntax::write::{UpdateError, incremental_update};
use pdf_syntax::{Document, Limits};

/// A small document with a classic §7.5.4 cross-reference table.
fn classic() -> Vec<u8> {
    let body = "1 0 obj\n<< /Type /Catalog /Pages 2 0 R /Marked (no) >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 100] /Contents 4 0 R >>\nendobj\n\
         4 0 obj\n<< /Length 0 >>\nstream\n\nendstream\nendobj\n"
        .to_owned();

    let mut out = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for object in body.split_inclusive("endobj\n") {
        offsets.push(out.len());
        out.push_str(object);
    }
    let xref_at = out.len();
    let size = offsets.len().saturating_add(1);
    let _ = writeln!(out, "xref\n0 {size}");
    out.push_str("0000000000 65535 f \n");
    for offset in &offsets {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    let _ = write!(
        out,
        "trailer\n<< /Size {size} /Root 1 0 R /ID [<0102> <0304>] >>\nstartxref\n{xref_at}\n%%EOF\n"
    );
    out.into_bytes()
}

/// The same document with a §7.5.8 cross-reference *stream* instead of a table.
///
/// Uncompressed, with Table 18's three fields at the widths `/W` states — which is what this
/// tree's own writer produces, so a file it writes is a file it can chain onto.
fn with_cross_reference_stream() -> Vec<u8> {
    let body = "1 0 obj\n<< /Type /Catalog /Pages 2 0 R /Marked (no) >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 100] /Contents 4 0 R >>\nendobj\n\
         4 0 obj\n<< /Length 0 >>\nstream\n\nendstream\nendobj\n"
        .to_owned();

    let mut out = Vec::from(&b"%PDF-1.7\n"[..]);
    let mut offsets = Vec::new();
    for object in body.split_inclusive("endobj\n") {
        offsets.push(out.len());
        out.extend_from_slice(object.as_bytes());
    }
    let xref_at = out.len();

    // Object 0 is free, 1..=4 are the objects above, and 5 is the stream itself.
    let mut data = vec![0, 0, 0, 0, 0, 255, 255];
    for offset in &offsets {
        data.push(1);
        data.extend_from_slice(&u32::try_from(*offset).unwrap().to_be_bytes());
        data.extend_from_slice(&0_u16.to_be_bytes());
    }
    data.push(1);
    data.extend_from_slice(&u32::try_from(xref_at).unwrap().to_be_bytes());
    data.extend_from_slice(&0_u16.to_be_bytes());

    let mut header = String::new();
    let _ = write!(
        header,
        "5 0 obj\n<< /Type /XRef /Size 6 /W [1 4 2] /Root 1 0 R /ID [<0102> <0304>] /Length {} >>\nstream\n",
        data.len()
    );
    out.extend_from_slice(header.as_bytes());
    out.extend_from_slice(&data);
    out.extend_from_slice(b"\nendstream\nendobj\n");
    let mut tail = String::new();
    let _ = write!(tail, "startxref\n{xref_at}\n%%EOF\n");
    out.extend_from_slice(tail.as_bytes());
    out
}

/// One replacement: the catalog, with `/Marked` changed.
fn marked(document: &Document, value: &str) -> BTreeMap<ObjectId, Object> {
    let mut catalog = document.catalog().expect("the fixture states a /Root");
    catalog.insert(
        Name::new(&b"Marked"[..]),
        Object::String(value.as_bytes().to_vec().into()),
    );
    let mut replacements = BTreeMap::new();
    replacements.insert(
        ObjectId {
            number: 1,
            generation: 0,
        },
        Object::Dictionary(catalog),
    );
    replacements
}

/// What the catalog's `/Marked` now says.
fn read_back(bytes: &[u8]) -> String {
    let document = Document::open(bytes.to_vec()).expect("the update is readable");
    let catalog = document.catalog().expect("a /Root");
    let value = document.get_key(&catalog, "Marked");
    let Object::String(bytes) = value else {
        panic!("/Marked is a string: {value:?}");
    };
    String::from_utf8(bytes.to_vec()).expect("the fixture writes ASCII")
}

#[test]
fn an_update_replaces_an_object_and_keeps_the_file_under_it() {
    let original = classic();
    let document = Document::open(original.clone()).unwrap();
    assert_eq!(read_back(&original), "no");

    let updated = incremental_update(&document, &marked(&document, "yes")).unwrap();
    assert!(
        updated.starts_with(&original),
        "§7.5.6 appends, 'leaving its original contents intact'"
    );
    assert_eq!(read_back(&updated), "yes");
}

#[test]
fn an_update_chains_onto_an_update() {
    // §7.5.6's `/Prev` is a chain rather than a pair, and a second save has to find the first.
    let document = Document::open(classic()).unwrap();
    let once = incremental_update(&document, &marked(&document, "yes")).unwrap();

    let reopened = Document::open(once.clone()).unwrap();
    let twice = incremental_update(&reopened, &marked(&reopened, "again")).unwrap();
    assert!(twice.starts_with(&once));
    assert_eq!(read_back(&twice), "again");

    // And the objects the *first* update did not touch are still reachable through both links.
    let document = Document::open(twice).unwrap();
    let catalog = document.catalog().unwrap();
    let pages = document.get_key(&catalog, "Pages");
    assert!(matches!(pages, Object::Dictionary(_)), "{pages:?}");
}

#[test]
fn the_cross_reference_section_is_the_kind_the_file_already_uses() {
    // Nothing in the standard requires them to match, and a file whose sections are all one kind
    // is a file whose next reader has one thing to do.
    let table = Document::open(classic()).unwrap();
    let updated = incremental_update(&table, &marked(&table, "yes")).unwrap();
    let appended = &updated[classic().len()..];
    assert!(
        appended.windows(5).any(|window| window == b"xref\n"),
        "a table after a table"
    );

    let stream = Document::open(with_cross_reference_stream()).unwrap();
    let updated = incremental_update(&stream, &marked(&stream, "yes")).unwrap();
    let appended = &updated[with_cross_reference_stream().len()..];
    assert!(
        appended.windows(5).any(|window| window == b"/XRef"),
        "a stream after a stream"
    );
    assert_eq!(read_back(&updated), "yes");
}

#[test]
fn the_file_identifier_keeps_its_first_element_and_changes_its_second() {
    // §14.4: "The first byte string shall be a permanent identifier based on the contents of the
    // file at the time it was originally created … The second byte string shall be a changing
    // identifier based on the file's contents at the time it was last updated."
    let document = Document::open(classic()).unwrap();
    let updated = incremental_update(&document, &marked(&document, "yes")).unwrap();
    let reopened = Document::open(updated.clone()).unwrap();

    let id = reopened.trailer().get("ID").cloned().expect("an /ID");
    let Object::Array(parts) = id else {
        panic!("/ID is an array: {id:?}");
    };
    assert_eq!(parts.len(), 2);
    assert_eq!(
        parts[0],
        Object::String(vec![1, 2].into()),
        "the permanent half is the one the file was created with"
    );
    assert_ne!(parts[1], Object::String(vec![3, 4].into()));

    // And saving the same edit twice produces the same bytes: rule 3 says this crate has no
    // clock, and a changing identifier derived from the file's contents needs none.
    let again = incremental_update(&document, &marked(&document, "yes")).unwrap();
    assert_eq!(updated, again);
}

#[test]
fn a_document_that_cannot_be_chained_onto_is_refused_by_name() {
    // Two refusals, each the same shape as every other refusal in this tree: named rather than
    // written wrong.
    let broken = classic()
        .windows(9)
        .rposition(|window| window == b"startxref")
        .map(|at| classic()[..at].to_vec())
        .unwrap();
    let document = Document::open(broken).unwrap();
    assert!(document.was_recovered(), "the fixture is broken enough");
    assert_eq!(
        incremental_update(&document, &BTreeMap::new()),
        Err(UpdateError::Recovered)
    );

    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../doc/pdf.js/test/pdfs/issue7665.pdf");
    let Ok(bytes) = std::fs::read(path) else {
        eprintln!("skipped: doc/pdf.js is not checked out");
        return;
    };
    let encrypted = Document::open_with_limits(bytes, Limits::DEFAULT).unwrap();
    assert!(encrypted.is_encrypted());
    assert_eq!(
        incremental_update(&encrypted, &BTreeMap::new()),
        Err(UpdateError::Encrypted)
    );
}

#[test]
fn every_object_type_survives_being_written_and_read() {
    // The serialiser, through the reader that has to accept what it writes. A name with a
    // delimiter in it and a string with a byte outside ASCII are the two shapes where clause 7's
    // escaping rules decide whether the file parses at all.
    let mut dict = Dictionary::new();
    dict.insert(Name::new(&b"Null"[..]), Object::Null);
    dict.insert(Name::new(&b"True"[..]), Object::Boolean(true));
    dict.insert(Name::new(&b"Int"[..]), Object::Integer(-42));
    dict.insert(Name::new(&b"Real"[..]), Object::Real(1.5));
    dict.insert(Name::new(&b"Whole"[..]), Object::Real(3.0));
    dict.insert(
        Name::new(&b"Text"[..]),
        Object::String(vec![0xFE, 0xFF, 0x00, 0x41].into()),
    );
    dict.insert(
        Name::new(&b"Odd Name#"[..]),
        Object::Name(Name::new(&b"a/b"[..])),
    );
    dict.insert(
        Name::new(&b"Array"[..]),
        Object::Array(vec![
            Object::Integer(1),
            Object::Reference(ObjectId {
                number: 2,
                generation: 0,
            }),
        ]),
    );

    let document = Document::open(classic()).unwrap();
    let mut replacements = BTreeMap::new();
    replacements.insert(
        ObjectId {
            number: 1,
            generation: 0,
        },
        Object::Dictionary({
            let mut catalog = document.catalog().unwrap();
            catalog.insert(Name::new(&b"Probe"[..]), Object::Dictionary(dict.clone()));
            catalog
        }),
    );
    let updated = incremental_update(&document, &replacements).unwrap();

    let reopened = Document::open(updated).unwrap();
    let catalog = reopened.catalog().unwrap();
    let probe = reopened.get_key(&catalog, "Probe");
    let Object::Dictionary(probe) = probe else {
        panic!("the probe survived: {probe:?}");
    };
    for (key, value) in dict.iter() {
        let name = String::from_utf8_lossy(key.as_bytes()).into_owned();
        let read = probe.get(&name);
        if *value == Object::Real(3.0) {
            // §7.3.3 makes an integer and a real two spellings of one type — "PDF provides two
            // types of numeric objects: integer and real" — and a real with no fractional part
            // is written without one, so it reads back as the integer it is. Asserted rather
            // than glossed over, because a writer that produced `3.0` here would be producing a
            // spelling no producer uses.
            assert_eq!(read, Some(&Object::Integer(3)), "{name}");
            continue;
        }
        if *value == Object::Null {
            // §7.3.9: "A dictionary entry whose value is null … shall be treated the same as if
            // the entry does not exist." So the one object type that does *not* survive being
            // written and read is the one the clause says must not.
            assert_eq!(read, None, "{name}");
            continue;
        }
        assert_eq!(read, Some(value), "{name}");
    }
}

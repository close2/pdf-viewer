//! Review 2136's audit of ISO 32000-2 §7.5, file structure, and §7.3.10, indirect objects
//! (`doc/reviews/2136-specification-audit-7-5.md`): one failing test per behaviour finding a test
//! can reach.
//!
//! Each test states a sentence of the clause, builds the smallest file that exercises it from
//! bytes written here, and carries a control in the same body that the fixture reads as intended
//! where the rule is not at stake, so that a failure is the finding and not a fixture that cannot
//! be read.

#![expect(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "test code: a fixture that cannot exercise the rule must fail loudly rather than \
              pass by doing nothing"
)]

use std::collections::BTreeMap;
use std::fmt::Write as _;

use pdf_syntax::object::{Object, ObjectId};
use pdf_syntax::serialize::flate_encode;
use pdf_syntax::write::{incremental_update, incremental_update_freeing};
use pdf_syntax::{
    Assembly, Document, Form, Limits, Location, ObjectStreams, Options, Streams, Version, serialize,
};

/// One cross-reference stream record of `/W [1 4 2]`.
fn record(kind: u8, second: u32, third: u16, out: &mut Vec<u8>) {
    out.push(kind);
    out.extend_from_slice(&second.to_be_bytes());
    out.extend_from_slice(&third.to_be_bytes());
}

/// A file whose objects 10 and 11 are compressed in object stream 5, the stream's data being
/// `pairs` followed by `payload`, and whose cross-reference stream is object 6.
///
/// The catalog (1) names both compressed objects, so a reader reaches them by reference.
fn packed(pairs: &str, payload: &str) -> Vec<u8> {
    let mut out = b"%PDF-1.7\n%\xE2\xE3\xCF\xD3\n".to_vec();
    let mut offsets = BTreeMap::new();
    let top = [
        (
            1_u32,
            "<< /Type /Catalog /Pages 2 0 R /A 10 0 R /B 11 0 R >>".to_owned(),
        ),
        (2, "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned()),
        (
            3,
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 20 20] >>".to_owned(),
        ),
    ];
    for (number, body) in &top {
        offsets.insert(*number, out.len());
        out.extend_from_slice(format!("{number} 0 obj\n{body}\nendobj\n").as_bytes());
    }
    let data = format!("{pairs}{payload}");
    offsets.insert(5, out.len());
    out.extend_from_slice(
        format!(
            "5 0 obj\n<< /Type /ObjStm /N 2 /First {} /Length {} >>\nstream\n{data}\nendstream\nendobj\n",
            pairs.len(),
            data.len()
        )
        .as_bytes(),
    );
    let at = out.len();
    let mut rows = Vec::new();
    record(0, 0, 65_535, &mut rows);
    for number in 1..=3 {
        record(1, u32::try_from(offsets[&number]).unwrap(), 0, &mut rows);
    }
    record(0, 0, 65_535, &mut rows); // 4 is unused
    record(1, u32::try_from(offsets[&5]).unwrap(), 0, &mut rows);
    record(1, u32::try_from(at).unwrap(), 0, &mut rows);
    for _ in 7..10 {
        record(0, 0, 65_535, &mut rows);
    }
    record(2, 5, 0, &mut rows);
    record(2, 5, 1, &mut rows);
    out.extend_from_slice(
        format!(
            "6 0 obj\n<< /Type /XRef /Size 12 /W [1 4 2] /Root 1 0 R /Length {} >>\nstream\n",
            rows.len()
        )
        .as_bytes(),
    );
    out.extend_from_slice(&rows);
    out.extend_from_slice(format!("\nendstream\nendobj\nstartxref\n{at}\n%%EOF\n").as_bytes());
    out
}

/// §7.5.7: "Following the N pairs of integers, the N objects are stored consecutively", and NOTE 7
/// (2020): "Previous editions of this specification incorrectly stated that white-space was
/// required between objects. This document corrects the text to clarify that processing of each
/// object in an object stream starts at the specified byte offset in the decompressed stream and
/// ends prior to the byte offset of the next object or when the end of stream is encountered."
///
/// Two integers stored with nothing between them are therefore two objects, `5` and `7`.
#[test]
fn a_compressed_object_ends_where_the_next_one_begins_spec_audit() {
    // Control: the same two objects with white-space between them.
    let spaced = Document::open(packed("10 0 11 2 ", "5 7")).expect("the control opens");
    assert_eq!(
        spaced.get(ObjectId::new(10, 0)),
        Object::Integer(5),
        "control, object 10"
    );
    assert_eq!(
        spaced.get(ObjectId::new(11, 0)),
        Object::Integer(7),
        "control, object 11"
    );

    let adjacent = Document::open(packed("10 0 11 1 ", "57")).expect("the document opens");
    assert_eq!(
        adjacent.get(ObjectId::new(10, 0)),
        Object::Integer(5),
        "object 10 is the one byte before object 11's offset"
    );
    assert_eq!(adjacent.get(ObjectId::new(11, 0)), Object::Integer(7));
}

/// A classic one-section file of four objects, `prefix` bytes of junk in front of its header.
fn prefixed(prefix: usize) -> Vec<u8> {
    let mut body = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for object in [
        "<< /Type /Catalog /Pages 2 0 R /Note 4 0 R >>",
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 20 20] >>",
        "(original)",
    ] {
        offsets.push(body.len());
        let _ = writeln!(body, "{} 0 obj\n{object}\nendobj", offsets.len());
    }
    let xref = body.len();
    body.push_str("xref\n0 5\n0000000000 65535 f \n");
    for offset in offsets {
        let _ = writeln!(body, "{offset:010} 00000 n ");
    }
    let _ = write!(
        body,
        "trailer\n<< /Size 5 /Root 1 0 R /ID [<00112233445566778899aabbccddeeff> <00112233445566778899aabbccddeeff>] >>\nstartxref\n{xref}\n%%EOF\n"
    );
    let mut bytes = vec![b'x'; prefix];
    bytes.extend_from_slice(body.as_bytes());
    bytes
}

/// §7.5.2: "The PDF file begins with the 5 characters '%PDF-' and byte offsets shall be calculated
/// from the PERCENT SIGN (25h)", and NOTE 1: "This provision allows for arbitrary bytes preceding
/// the %PDF- without impacting the viability of the PDF file and its byte offsets." §7.5.6's update
/// appended to such a file states its offsets the same way, so the file reads back through its
/// own cross-reference sections — the replaced object replaced, the freed one gone — and is not
/// rebuilt by scanning.
#[test]
fn an_update_to_a_file_with_bytes_before_its_header_counts_from_the_header_spec_audit() {
    for prefix in [0_usize, 100] {
        let original = Document::open(prefixed(prefix)).expect("the original opens");
        assert!(
            !original.was_recovered(),
            "control: the original at prefix {prefix} reads its table"
        );

        let mut replaced = BTreeMap::new();
        replaced.insert(
            ObjectId::new(4, 0),
            Object::String(b"changed".to_vec().into()),
        );
        let updated = Document::open(incremental_update(&original, &replaced).expect("written"))
            .expect("the update opens");
        assert_eq!(
            updated.get(ObjectId::new(4, 0)),
            Object::String(b"changed".to_vec().into()),
            "prefix {prefix}"
        );

        let freed = Document::open(
            incremental_update_freeing(&original, &BTreeMap::new(), &[ObjectId::new(4, 0)])
                .expect("written"),
        )
        .expect("the update opens");
        assert_eq!(
            freed.get(ObjectId::new(4, 0)),
            Object::Null,
            "prefix {prefix}: freed"
        );
        assert!(
            !updated.was_recovered() && !freed.was_recovered(),
            "both updates at prefix {prefix} are read through their own sections"
        );
    }
}

/// Peak resident memory of this process, from Linux's `/proc/self/status`; `None` elsewhere.
///
/// nextest runs each test in a process of its own, so the mark is this test's.
fn high_water_kib() -> Option<u64> {
    let status = std::fs::read_to_string("/proc/self/status").ok()?;
    status
        .lines()
        .find_map(|line| line.strip_prefix("VmHWM:"))
        .and_then(|rest| rest.trim().trim_end_matches("kB").trim().parse().ok())
}

/// A conforming file of a few hundred bytes whose cross-reference stream (`/W [1 2 2]`) states
/// `free` type 0 entries between its first five objects and its last.
///
/// The last object is defined at number `5 + free`, so `/Size` is "1 greater than the highest
/// object number defined in the PDF file" (Table 15) and every number below it has its entry
/// (§7.5.4); object 0 is free at generation 65 535, and the stream (object 4) has an entry of its
/// own (§7.5.8.3).
fn mostly_free(free: u32) -> Vec<u8> {
    let last = 5_u32.saturating_add(free);
    let mut out = b"%PDF-1.7\n".to_vec();
    let mut offsets = Vec::new();
    for (number, body) in [
        (
            1,
            "<< /Type /Catalog /Pages 2 0 R /Last 0 0 R >>"
                .replace("0 0 R", &format!("{last} 0 R")),
        ),
        (2, "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned()),
        (
            3,
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 20 20] >>".to_owned(),
        ),
        (last, "(last)".to_owned()),
    ] {
        offsets.push(u16::try_from(out.len()).unwrap());
        out.extend_from_slice(format!("{number} 0 obj\n{body}\nendobj\n").as_bytes());
    }
    let at = u16::try_from(out.len()).unwrap();
    let in_use = |offset: u16| -> [u8; 5] {
        let [high, low] = offset.to_be_bytes();
        [1, high, low, 0, 0]
    };
    let mut rows = vec![0_u8, 0, 0, 0xFF, 0xFF];
    for offset in &offsets[..3] {
        rows.extend_from_slice(&in_use(*offset));
    }
    rows.extend_from_slice(&in_use(at));
    rows.resize(
        rows.len()
            .saturating_add(usize::try_from(free).unwrap().saturating_mul(5)),
        0,
    );
    rows.extend_from_slice(&in_use(offsets[3]));
    let once = flate_encode(&rows, 9).expect("deflate");
    let twice = flate_encode(&once, 9).expect("deflate");
    out.extend_from_slice(
        format!(
            "4 0 obj\n<< /Type /XRef /Size {} /W [1 2 2] /Root 1 0 R /Filter [/FlateDecode /FlateDecode] /Length {} >>\nstream\n",
            last.saturating_add(1),
            twice.len()
        )
        .as_bytes(),
    );
    out.extend_from_slice(&twice);
    out.extend_from_slice(format!("\nendstream\nendobj\nstartxref\n{at}\n%%EOF\n").as_bytes());
    out
}

/// CLAUDE.md principle 3 ("Explicit memory and time budgets guard against decompression bombs,
/// xref cycles") against §7.5.4's "one entry for each object number from 0 to the maximum object
/// number defined in the PDF file" and §7.5.8.2's `/Index` default `[0 Size]`: a cross-reference
/// stream may state millions of free entries in a few hundred bytes, and a free entry names nothing
/// a reader has to keep. Two million of them inflate from a file under half a kilobyte; the table
/// read from it is held within 32 MiB of the process's high-water mark, or the file is refused by
/// name.
#[test]
fn free_entries_do_not_cost_the_reader_memory_apiece_spec_audit() {
    let file = mostly_free(2 << 20);
    let length = file.len();
    let Some(before) = high_water_kib() else {
        // No high-water mark to read on this platform; the Linux runs carry the finding.
        return;
    };
    let document = Document::open(file);
    let grown = high_water_kib()
        .expect("read a moment ago")
        .saturating_sub(before);
    if let Ok(document) = &document {
        let catalog = document.catalog().expect("control: the catalog is reached");
        assert_eq!(
            document.get_key(&catalog, "Last"),
            Object::String(b"last".to_vec().into()),
            "control: the object past the free run is read"
        );
    }
    assert!(
        grown < 32 << 10,
        "a {length}-byte file grew the process by {grown} KiB"
    );
}

/// §7.5.8.3 Table 18, type 2, field 3: "The index of this object within the object stream." The
/// whole-file writer is public about how many objects one stream may hold
/// ([`ObjectStreams::Generate`]), and its cross-reference stream writes the index in a two-byte
/// field; past 65 535 members the entry must still state the member's own index (or the writer
/// must refuse by name, as it does an offset its fields cannot hold).
#[test]
fn a_compressed_entry_states_its_own_index_however_many_members_spec_audit() {
    let source = Document::open(prefixed(0)).expect("the source opens");
    let mut assembly = Assembly::new(vec![&source]);
    for number in 1..=4 {
        assembly.copy(0, ObjectId::new(number, 0)).expect("copied");
    }
    assembly.set_root(ObjectId::new(1, 0));
    let mut last = ObjectId::new(4, 0);
    for value in 0..70_000 {
        last = assembly.add(Object::Integer(value)).expect("added");
    }
    let mut bytes = Vec::new();
    let written = serialize(
        &assembly,
        Version { major: 1, minor: 7 },
        Options {
            form: Form::Stream,
            object_streams: ObjectStreams::Generate {
                max_objects: 100_000,
                max_bytes: 1 << 30,
            },
            streams: Streams::Carry,
        },
        &mut bytes,
    );
    let Ok(written) = written else {
        // A refusal by name is the other answer the clause allows.
        return;
    };
    assert_eq!(
        written.object_streams, 1,
        "control: one object stream holds every member"
    );
    let table = pdf_syntax::xref::read(&bytes.into(), Limits::default()).expect("the table reads");
    let Some(Location::InStream { index: early, .. }) = table.location(10) else {
        panic!("control: object 10 is compressed");
    };
    assert_eq!(early, 9, "control: the tenth member is at index 9");
    assert_eq!(
        table.location(last.number),
        Some(Location::InStream {
            stream: 70_005,
            index: last.number.saturating_sub(1),
        }),
        "the last member's entry"
    );
}

//! ISO 32000-2 §7.5's structure written whole, and then read back.
//!
//! # Why the reader is the test, again
//!
//! `incremental_update.rs` says it for §7.5.6 and it is truer here: a whole file is a header, a
//! body, a cross-reference section and a trailer that have to agree with each other about where
//! every object is, and none of that is visible in a diff. So every test writes a file, opens
//! the result from scratch with this tree's own reader — the one 974 corpus documents go
//! through — and asks it what the document now says.
//!
//! # The hostile half
//!
//! Three of these are not about a well-formed source at all. A serializer is the first code in
//! this project that *produces* bytes another parser will read (RFC 0002 section 11.3), so the
//! constructions it must not propagate are pinned here: an object referring to one that is not
//! in the output, a stream whose `/Length` disagrees with its own bytes, and a cycle in the
//! object graph.

#![expect(
    clippy::expect_used,
    reason = "test code: a fixture that cannot exercise the rule must fail loudly rather than \
              pass by doing nothing"
)]

use std::fmt::Write as _;

use pdf_syntax::object::{Dictionary, Name, Object, ObjectId};
use pdf_syntax::serialize::{Assembly, AssemblyError, Form, SerializeError, serialize};
use pdf_syntax::{Document, Limits, Version};

/// Assembles a file out of object bodies, with a §7.5.4 classic cross-reference table.
///
/// The bodies are numbered from 1 in the order given, which is what every fixture here wants.
fn file_of(bodies: &[&str], trailer_extra: &str) -> Vec<u8> {
    let mut out = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for (index, body) in bodies.iter().enumerate() {
        offsets.push(out.len());
        let _ = write!(out, "{} 0 obj\n{body}\nendobj\n", index.saturating_add(1));
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
        "trailer\n<< /Size {size} /Root 1 0 R {trailer_extra} >>\nstartxref\n{xref_at}\n%%EOF\n"
    );
    out.into_bytes()
}

/// One page, one content stream, one font-free resource dictionary.
fn one_page() -> Vec<u8> {
    file_of(
        &[
            "<< /Type /Catalog /Pages 2 0 R >>",
            "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 100] /Contents 4 0 R \
             /Resources << /ProcSet [/PDF] >> >>",
            "<< /Length 26 >>\nstream\n0 0 1 rg 10 10 50 50 re f\nendstream",
        ],
        "/ID [<0102> <0304>]",
    )
}

/// Opens bytes, failing loudly.
fn open(bytes: Vec<u8>) -> Document {
    Document::open_with_limits(bytes, Limits::DEFAULT).expect("the fixture opens")
}

/// Copies every object of a document into an assembly, keeping its catalog as the root.
fn copy_whole(assembly: &mut Assembly<'_>, document: &Document, highest: u32) {
    for number in 1..=highest {
        assembly
            .copy(0, ObjectId::new(number, 0))
            .expect("source 0 exists");
    }
    let root = document
        .trailer()
        .get("Root")
        .and_then(Object::as_reference)
        .expect("the fixture states /Root");
    let mapped = assembly.copied(0, root).expect("the catalog was copied");
    assembly.set_root(mapped);
}

/// Writes an assembly, failing loudly.
fn write_out(assembly: &Assembly<'_>, form: Form) -> (Vec<u8>, pdf_syntax::Written) {
    let mut bytes = Vec::new();
    let written = serialize(assembly, Version { major: 1, minor: 7 }, form, &mut bytes)
        .expect("the assembly writes");
    (bytes, written)
}

/// The whole document, copied object for object, is the same document.
///
/// §7.5.1's order — header, body, cross-reference section, trailer — has to hold together for
/// this to pass at all: a wrong offset, a miscounted subsection or a `/Root` that does not
/// resolve all show up as a document with no pages.
#[test]
fn a_document_copied_object_for_object_reads_back_as_itself() {
    for form in [Form::Table, Form::Stream] {
        let source = open(one_page());
        let mut assembly = Assembly::new(vec![&source]);
        copy_whole(&mut assembly, &source, 4);
        let (bytes, written) = write_out(&assembly, form);

        assert_eq!(written.objects, 4, "{form:?}");
        assert_eq!(written.dangling, 0, "{form:?}");
        assert_eq!(
            u64::try_from(bytes.len()).unwrap(),
            written.bytes,
            "{form:?}"
        );
        assert!(bytes.starts_with(b"%PDF-1."), "{form:?}");
        assert!(bytes.ends_with(b"%%EOF\n"), "{form:?}");

        let read = open(bytes);
        let catalog = read.catalog().expect("the output has a catalog");
        let pages = read.get_key(&catalog, "Pages");
        let kids = read.get_key(pages.as_dict().expect("a page tree"), "Kids");
        let kids = kids.as_array().expect("/Kids is an array");
        assert_eq!(kids.len(), 1, "{form:?}");
        let page = read.resolve(&kids[0]);
        let page = page.as_dict().expect("a page");
        let contents = read.get_key(page, "Contents");
        let stream = contents.as_stream().expect("the content stream survived");
        assert_eq!(
            &*stream.data, b"0 0 1 rg 10 10 50 50 re f\n",
            "the producer's content stream crossed byte for byte ({form:?})"
        );
    }
}

/// §7.5.8.1 introduces the cross-reference stream at PDF 1.5, so an output that uses one says
/// at least 1.5 in its header whatever version the caller asked for.
#[test]
fn a_cross_reference_stream_raises_the_headers_version_to_the_clauses() {
    let source = open(one_page());
    let mut assembly = Assembly::new(vec![&source]);
    copy_whole(&mut assembly, &source, 4);
    let mut bytes = Vec::new();
    serialize(
        &assembly,
        Version { major: 1, minor: 3 },
        Form::Stream,
        &mut bytes,
    )
    .expect("the assembly writes");
    assert!(bytes.starts_with(b"%PDF-1.5\n"), "header was not raised");
    let read = open(bytes);
    assert_eq!(read.header_version(), Some(Version { major: 1, minor: 5 }));
}

/// The form follows the sources: a document whose own last section is a table gets a table.
#[test]
fn the_form_of_a_document_is_the_kind_its_own_last_section_uses() {
    let classic = open(one_page());
    assert_eq!(Form::of(&classic), Form::Table);
    assert_eq!(Form::of_all([&classic]), Form::Table);
    // Nothing to follow is nothing to raise: an assembly with no source at all writes the form
    // every reader has always understood.
    assert_eq!(Form::of_all(std::iter::empty()), Form::Table);
}

/// ISO 32000-2 §7.3.10: a reference to an object that is not there is null, not an error.
///
/// > An indirect reference to an undefined object shall not be considered an error by a PDF
/// > processor; it shall be treated as a reference to the null object.
///
/// A transform makes these deliberately — a page's closure stops at the piece's edge — so the
/// requirement is that the reference becomes null *and is counted*, never that it is quietly
/// carried into a file naming an object number nothing defines.
#[test]
fn a_reference_the_assembly_does_not_hold_is_written_as_null_and_counted() {
    let source = open(file_of(
        &[
            "<< /Type /Catalog /Pages 2 0 R >>",
            "<< /Type /Pages /Kids [3 0 R] /Count 1 /Absent 9 0 R >>",
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] >>",
        ],
        "",
    ));
    let mut assembly = Assembly::new(vec![&source]);
    copy_whole(&mut assembly, &source, 3);
    let (bytes, written) = write_out(&assembly, Form::Table);
    assert_eq!(
        written.dangling, 1,
        "the one reference out of the assembly was not counted"
    );
    assert!(
        bytes.windows(13).any(|window| window == b"/Absent null "),
        "the clause's own answer is written out, rather than the entry being dropped"
    );

    let read = open(bytes);
    let catalog = read.catalog().unwrap();
    let pages = read.get_key(&catalog, "Pages");
    let pages = pages.as_dict().unwrap();
    // §7.3.7 then makes the two indistinguishable to a reader — "a dictionary entry whose value
    // is null shall be treated the same as if the entry does not exist" — which is why the
    // bytes are asserted above and the reading here.
    assert!(
        read.get_key(pages, "Absent").is_null(),
        "the dangling entry must not resolve to anything"
    );
    assert!(
        read.get_key(pages, "Kids")
            .as_array()
            .is_some_and(|kids| kids.len() == 1),
        "the references that were in the assembly still resolve"
    );
}

/// ISO 32000-2 §7.3.8.2 makes `/Length` a statement about the bytes actually written.
///
/// > Every stream dictionary shall have a Length entry that indicates how many bytes of the PDF
/// > file are used for the stream's data. (If the stream has a filter, Length shall be the
/// > number of bytes of encoded data.)
///
/// The source here says 4 and holds 26. What is written must say 26, and the count of
/// corrections must be one — a serializer that copied the lie would be putting this program's
/// name on a file its own reader has to recover.
#[test]
fn a_stream_whose_length_lies_is_written_with_the_length_it_has() {
    let source = open(file_of(
        &[
            "<< /Type /Catalog /Pages 2 0 R >>",
            "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] /Contents 4 0 R >>",
            "<< /Length 4 >>\nstream\n0 0 1 rg 10 10 50 50 re f\nendstream",
        ],
        "",
    ));
    let stated = source
        .get(ObjectId::new(4, 0))
        .as_stream()
        .expect("a stream")
        .data
        .len();
    assert_eq!(stated, 25, "the reader recovered the real extent");

    let mut assembly = Assembly::new(vec![&source]);
    copy_whole(&mut assembly, &source, 4);
    let (bytes, written) = write_out(&assembly, Form::Table);
    assert_eq!(written.relengthed, 1, "the lie was not counted");

    let read = open(bytes);
    let stream = read.get(ObjectId::new(4, 0));
    let stream = stream.as_stream().expect("the stream survived");
    assert_eq!(
        stream.dict.get("Length").and_then(Object::as_integer),
        Some(25),
        "the written /Length must describe the bytes written"
    );
    assert_eq!(&*stream.data, b"0 0 1 rg 10 10 50 50 re f");
}

/// A cycle in the object graph is written once and read back whole.
///
/// `copy` is idempotent, so a walk that arrives twice numbers once and terminates; and the
/// renumbering walks one object's value tree, which is finite whatever the graph does. Both
/// halves are asserted: the output holds three objects, and both directions of the cycle
/// resolve.
#[test]
fn a_cycle_in_the_object_graph_is_written_once_and_reads_back_whole() {
    let source = open(file_of(
        &[
            "<< /Type /Catalog /Pages 2 0 R >>",
            "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] /Self 3 0 R >>",
        ],
        "",
    ));
    let mut assembly = Assembly::new(vec![&source]);
    // The walk a transform performs, arriving at every object by every path there is.
    for _ in 0..3 {
        for number in [1_u32, 2, 3, 2, 3, 1] {
            assembly.copy(0, ObjectId::new(number, 0)).unwrap();
        }
    }
    assert_eq!(
        assembly.len(),
        3,
        "an idempotent copy numbers each object once"
    );
    let root = assembly.copied(0, ObjectId::new(1, 0)).unwrap();
    assembly.set_root(root);

    let (bytes, written) = write_out(&assembly, Form::Table);
    assert_eq!(written.objects, 3);
    assert_eq!(written.dangling, 0);

    let read = open(bytes);
    let page = read.get(ObjectId::new(3, 0));
    let page = page.as_dict().expect("the page");
    assert_eq!(
        page.get("Self").and_then(Object::as_reference),
        Some(ObjectId::new(3, 0)),
        "the self-reference survived renumbering"
    );
    assert_eq!(
        page.get("Parent").and_then(Object::as_reference),
        Some(ObjectId::new(2, 0)),
        "and so did the other direction"
    );
}

/// §7.5.5's Table 15 makes `/Root` required, so an assembly that names none is refused.
#[test]
fn an_assembly_with_no_root_is_refused() {
    let source = open(one_page());
    let mut assembly = Assembly::new(vec![&source]);
    assembly.copy(0, ObjectId::new(1, 0)).unwrap();
    let mut bytes = Vec::new();
    let error = serialize(
        &assembly,
        Version { major: 1, minor: 7 },
        Form::Table,
        &mut bytes,
    )
    .expect_err("a file with no catalog is not a file");
    assert!(matches!(error, SerializeError::NoRoot), "{error:?}");
}

/// A number promised to whatever referred to it, and never filled, is a caller's mistake and is
/// refused rather than written as null.
#[test]
fn a_slot_reserved_and_never_placed_is_refused() {
    let source = open(one_page());
    let mut assembly = Assembly::new(vec![&source]);
    let root = assembly.reserve().unwrap();
    let orphan = assembly.reserve().unwrap();
    let mut catalog = Dictionary::new();
    catalog.insert(
        Name::new(&b"Type"[..]),
        Object::Name(Name::new(&b"Catalog"[..])),
    );
    assembly.place(root, Object::Dictionary(catalog)).unwrap();
    assembly.set_root(root);

    let mut bytes = Vec::new();
    let error = serialize(
        &assembly,
        Version { major: 1, minor: 7 },
        Form::Table,
        &mut bytes,
    )
    .expect_err("an unplaced slot is refused");
    assert!(
        matches!(error, SerializeError::Unplaced { id } if id == orphan),
        "{error:?}"
    );
}

/// A slot may be filled once, and a copied object's number is not a slot at all.
#[test]
fn a_slot_is_filled_once_and_a_copied_objects_number_is_not_a_slot() {
    let source = open(one_page());
    let mut assembly = Assembly::new(vec![&source]);
    let copied = assembly.copy(0, ObjectId::new(1, 0)).unwrap();
    let slot = assembly.reserve().unwrap();
    assembly.place(slot, Object::Integer(1)).unwrap();
    assert!(matches!(
        assembly.place(slot, Object::Integer(2)),
        Err(AssemblyError::AlreadyPlaced { .. })
    ));
    assert!(matches!(
        assembly.place(copied, Object::Integer(3)),
        Err(AssemblyError::NotReserved { .. })
    ));
    assert!(matches!(
        assembly.place(ObjectId::new(99, 0), Object::Integer(4)),
        Err(AssemblyError::NotReserved { .. })
    ));
    assert!(matches!(
        assembly.copy(1, ObjectId::new(1, 0)),
        Err(AssemblyError::NoSuchSource {
            at: 1,
            count: 1,
            ..
        })
    ));
}

/// RFC 0002 section 9's first layer: same sources, same plan, same bytes, with no flag needed.
///
/// This is what makes every other layer a test instead of a demo, and it is the reason §14.4's
/// identifier is a digest of the output rather than a clock or a random string.
#[test]
fn the_same_plan_over_the_same_sources_writes_the_same_bytes() {
    let once = {
        let source = open(one_page());
        let mut assembly = Assembly::new(vec![&source]);
        copy_whole(&mut assembly, &source, 4);
        write_out(&assembly, Form::Table).0
    };
    let twice = {
        let source = open(one_page());
        let mut assembly = Assembly::new(vec![&source]);
        copy_whole(&mut assembly, &source, 4);
        write_out(&assembly, Form::Table).0
    };
    assert_eq!(once, twice, "the serializer is a function of its inputs");
    assert!(
        once.windows(4).any(|window| window == b"/ID "),
        "§14.4's identifier is required in the trailer"
    );
}

/// Two documents, one output: renumbering is total, and each source's references follow it.
#[test]
fn two_sources_are_renumbered_into_one_file() {
    let first = open(one_page());
    let second = open(file_of(
        &[
            "<< /Type /Catalog /Pages 2 0 R >>",
            "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 42 42] /Marker (second) >>",
        ],
        "",
    ));
    let mut assembly = Assembly::new(vec![&first, &second]);
    let page_one = assembly.copy(0, ObjectId::new(3, 0)).unwrap();
    assembly.copy(0, ObjectId::new(4, 0)).unwrap();
    let page_two = assembly.copy(1, ObjectId::new(3, 0)).unwrap();
    assert_ne!(page_one, page_two, "two sources' object 3 are two objects");

    let tree = assembly.reserve().unwrap();
    let catalog = assembly.reserve().unwrap();
    // The copied pages keep the `/Parent` their source stated, which no longer names anything
    // in this output; the transform above this layer is what replaces it, and this test is
    // about the renumbering rather than about a well-formed page tree.
    let mut pages = Dictionary::new();
    pages.insert(
        Name::new(&b"Type"[..]),
        Object::Name(Name::new(&b"Pages"[..])),
    );
    pages.insert(
        Name::new(&b"Kids"[..]),
        Object::Array(vec![
            Object::Reference(page_one),
            Object::Reference(page_two),
        ]),
    );
    pages.insert(Name::new(&b"Count"[..]), Object::Integer(2));
    assembly.place(tree, Object::Dictionary(pages)).unwrap();
    let mut root = Dictionary::new();
    root.insert(
        Name::new(&b"Type"[..]),
        Object::Name(Name::new(&b"Catalog"[..])),
    );
    root.insert(Name::new(&b"Pages"[..]), Object::Reference(tree));
    assembly.place(catalog, Object::Dictionary(root)).unwrap();
    assembly.set_root(catalog);

    let (bytes, _) = write_out(&assembly, Form::Table);
    let read = open(bytes);
    let catalog = read.catalog().unwrap();
    let tree = read.get_key(&catalog, "Pages");
    let kids = read.get_key(tree.as_dict().unwrap(), "Kids");
    let kids = kids.as_array().unwrap().to_vec();
    assert_eq!(kids.len(), 2);
    let second_page = read.resolve(&kids[1]);
    assert_eq!(
        second_page
            .as_dict()
            .and_then(|dict| dict.get("Marker"))
            .and_then(Object::as_string),
        Some(&b"second"[..]),
        "the second source's page came across"
    );
}

//! ISO 32000-2 §§7.5.2, 7.5.4 and 7.5.6: serialized file structure and incremental trailers.

#![expect(
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    reason = "test code: malformed fixtures must fail loudly rather than pass by doing nothing"
)]

use std::fmt::Write as _;

use pdf_syntax::object::{Name, Object, ObjectId};
use pdf_syntax::serialize::{Assembly, Form, Options, serialize};
use pdf_syntax::write::incremental_update;
use pdf_syntax::xref;
use pdf_syntax::{Document, Limits, Version};

/// Builds a one-page PDF with a classic table and exact offsets.
///
/// The fifth object is an unreferenced stream with binary bytes, so the header-comment
/// requirement has its stated condition. `/Resources` is present on the page and the `/ID`
/// strings meet Table 15's minimum length.
fn source_pdf(extension: &str) -> Vec<u8> {
    let objects: [&[u8]; 5] = [
        b"<< /Type /Catalog /Pages 2 0 R /Lang (en-US) >>",
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 100] /Resources << >> /Contents 4 0 R >>",
        b"<< /Length 3 >>\nstream\nq Q\nendstream",
        b"<< /Length 4 >>\nstream\n\x80\x81\xFE\xFF\nendstream",
    ];
    let mut out = b"%PDF-1.7\n%\x81\x82\x83\x84\n".to_vec();
    let mut offsets = Vec::new();
    for (index, body) in objects.iter().enumerate() {
        offsets.push(out.len());
        let header = format!("{} 0 obj\n", index.saturating_add(1));
        out.extend_from_slice(header.as_bytes());
        out.extend_from_slice(body);
        out.extend_from_slice(b"\nendobj\n");
    }

    let xref_at = out.len();
    let size = offsets.len().saturating_add(1);
    let mut section = String::new();
    let _ = writeln!(section, "xref\n0 {size}");
    section.push_str("0000000000 65535 f \n");
    for offset in &offsets {
        let _ = writeln!(section, "{offset:010} 00000 n ");
    }
    let _ = write!(
        section,
        "trailer\n<< /Size {size} /Root 1 0 R /ID [<00112233445566778899AABBCCDDEEFF> <00112233445566778899AABBCCDDEEFF>] {extension}>>\nstartxref\n{xref_at}\n%%EOF\n"
    );
    out.extend_from_slice(section.as_bytes());
    out
}

fn open(bytes: Vec<u8>) -> Document {
    Document::open_with_limits(bytes, Limits::DEFAULT).expect("the complete fixture opens")
}

fn assert_one_page(document: &Document) {
    assert!(
        !document.was_recovered(),
        "the valid fixture uses its own xref table"
    );
    let catalog = document.catalog().expect("/Root reaches the catalog");
    let pages = document.get_key(&catalog, "Pages");
    let pages = pages.as_dict().expect("/Pages is a dictionary");
    let kids = document.get_key(pages, "Kids");
    let kids = kids.as_array().expect("/Kids is an array");
    assert_eq!(kids.len(), 1);
    let page = document.resolve(&kids[0]);
    let page = page.as_dict().expect("the child is a page dictionary");
    assert!(document.get_key(page, "MediaBox").as_array().is_some());
    assert!(document.get_key(page, "Resources").as_dict().is_some());
}

fn root_id(document: &Document) -> ObjectId {
    document
        .trailer()
        .get("Root")
        .and_then(Object::as_reference)
        .expect("the trailer names an indirect catalog")
}

fn with_language(
    document: &Document,
    language: &[u8],
) -> std::collections::BTreeMap<ObjectId, Object> {
    let mut catalog = document.catalog().expect("the fixture has a catalog");
    catalog.insert(
        Name::new(&b"Lang"[..]),
        Object::String(language.to_vec().into()),
    );
    std::collections::BTreeMap::from([(root_id(document), Object::Dictionary(catalog))])
}

fn serialize_table(source: &Document) -> Vec<u8> {
    let mut assembly = Assembly::new(vec![source]);
    for number in 1..=5 {
        assembly
            .copy(0, ObjectId::new(number, 0))
            .expect("source object exists");
    }
    assembly.set_root(
        assembly
            .copied(0, root_id(source))
            .expect("the catalog was copied"),
    );
    let mut bytes = Vec::new();
    serialize(
        &assembly,
        Version { major: 1, minor: 7 },
        Options::new(Form::Table),
        &mut bytes,
    )
    .expect("the complete assembly serializes");
    bytes
}

fn startxref(bytes: &[u8]) -> usize {
    let at = bytes
        .windows(b"startxref".len())
        .rposition(|window| window == b"startxref")
        .expect("the output ends with startxref");
    let tail = bytes.get(at + b"startxref".len()..).expect("tail follows");
    let digits = tail
        .iter()
        .position(u8::is_ascii_digit)
        .expect("startxref has an offset");
    let tail = tail.get(digits..).expect("offset begins in tail");
    let end = tail
        .iter()
        .position(|byte| !byte.is_ascii_digit())
        .expect("offset ends before whitespace");
    std::str::from_utf8(tail.get(..end).expect("offset bytes exist"))
        .expect("decimal offset is ASCII")
        .parse::<usize>()
        .expect("offset fits usize")
}

fn line_with_eol(bytes: &[u8], at: usize) -> (&[u8], usize) {
    let rest = bytes.get(at..).expect("line begins in output");
    let end = rest
        .iter()
        .position(|byte| matches!(*byte, b'\r' | b'\n'))
        .expect("line ends with CR or LF");
    let width = if rest.get(end) == Some(&b'\r') && rest.get(end + 1) == Some(&b'\n') {
        2
    } else {
        1
    };
    (
        rest.get(..end + width).expect("line includes its EOL"),
        at + end + width,
    )
}

fn decimal(bytes: &[u8]) -> usize {
    std::str::from_utf8(bytes)
        .expect("decimal is ASCII")
        .parse::<usize>()
        .expect("decimal fits usize")
}

/// §7.5.2 / EC3 #272: binary stream data requires an immediately following comment-only line
/// whose comment begins with at least four binary characters. Any four bytes >=128 are valid.
#[test]
fn a_whole_file_writer_places_a_binary_comment_after_the_header() {
    let source_bytes = source_pdf("");
    assert!(source_bytes.iter().any(|byte| *byte >= 128));
    let source = open(source_bytes);
    assert_one_page(&source);

    let output = serialize_table(&source);
    assert!(
        output
            .windows(4)
            .any(|window| window == b"\x80\x81\xFE\xFF")
    );
    let written = open(output.clone());
    assert_one_page(&written);

    let (header, after_header) = line_with_eol(&output, 0);
    assert!(header.starts_with(b"%PDF-"));
    let (comment, _) = line_with_eol(&output, after_header);
    assert!(
        comment.starts_with(b"%"),
        "the immediately following line is a comment"
    );
    let comment = comment
        .strip_suffix(b"\r\n")
        .or_else(|| comment.strip_suffix(b"\r"))
        .or_else(|| comment.strip_suffix(b"\n"))
        .expect("the comment line has its one-line terminator");
    let bytes_after_percent = comment.get(1..).expect("comment marker is present");
    let leading_binary = bytes_after_percent
        .iter()
        .take_while(|byte| **byte >= 128)
        .count();
    assert!(
        leading_binary >= 4,
        "EC3 #272 requires the comment to start with at least four bytes >=128: {comment:?}"
    );
}

/// §7.5.4 producer obligations checked from the bytes, not just by reopening with this reader.
#[test]
fn a_whole_file_writer_emits_a_complete_classic_table() {
    let source = open(source_pdf(""));
    assert_one_page(&source);

    let output = serialize_table(&source);
    let reopened = open(output.clone());
    assert_one_page(&reopened);

    let xref_at = startxref(&output);
    assert_eq!(output.get(xref_at..xref_at + 4), Some(&b"xref"[..]));
    let (_, subsection_at) = line_with_eol(&output, xref_at + b"xref".len());
    let (subsection, mut cursor) = line_with_eol(&output, subsection_at);
    let values: Vec<usize> = subsection
        .split(u8::is_ascii_whitespace)
        .filter(|part| !part.is_empty())
        .map(decimal)
        .collect();
    assert_eq!(
        values.first(),
        Some(&0),
        "initial subsection begins at object 0"
    );
    assert_eq!(
        values.len(),
        2,
        "subsection has first object and count only"
    );
    let count = *values.get(1).expect("subsection declares a count");

    let trailer_at = output
        .windows(b"trailer\n".len())
        .rposition(|window| window == b"trailer\n")
        .expect("classic table has a trailer");
    let trailer = output.get(trailer_at..).expect("trailer follows xref");
    let size_at = trailer
        .windows(b"/Size ".len())
        .position(|window| window == b"/Size ")
        .expect("trailer states /Size");
    let size_start = size_at + b"/Size ".len();
    let size_tail = trailer.get(size_start..).expect("/Size value follows");
    let size_end = size_tail
        .iter()
        .position(|byte| !byte.is_ascii_digit())
        .expect("/Size is followed by whitespace");
    let size = decimal(size_tail.get(..size_end).expect("/Size digits exist"));
    assert_eq!(
        count, size,
        "one initial subsection covers 0 through Size-1"
    );

    for expected_number in 0..count {
        let (entry, next) = line_with_eol(&output, cursor);
        cursor = next;
        assert_eq!(
            entry.len(),
            20,
            "every classic entry is exactly 20 bytes, including EOL"
        );
        assert!(
            entry
                .get(0..10)
                .is_some_and(|part| part.iter().all(u8::is_ascii_digit))
        );
        assert_eq!(entry.get(10), Some(&b' '));
        assert!(
            entry
                .get(11..16)
                .is_some_and(|part| part.iter().all(u8::is_ascii_digit))
        );
        assert_eq!(entry.get(16), Some(&b' '));
        let eol = entry.get(18..).expect("entry includes its two-byte ending");
        assert!(eol == b" \n" || eol == b" \r" || eol == b"\r\n");

        let generation = decimal(entry.get(11..16).expect("generation field exists"));
        match expected_number {
            0 => {
                assert_eq!(entry.get(0..10), Some(&b"0000000000"[..]));
                assert_eq!(generation, 65_535);
                assert_eq!(entry.get(17), Some(&b'f'));
            }
            number => {
                assert_eq!(generation, 0);
                assert_eq!(entry.get(17), Some(&b'n'));
                let offset = decimal(entry.get(0..10).expect("object offset exists"));
                let header = format!("{number} 0 obj");
                assert_eq!(
                    output.get(offset..offset + header.len()),
                    Some(header.as_bytes()),
                    "entry offset names that object's own header"
                );
            }
        }
    }
    assert!(output.get(cursor..trailer_at).is_some_and(<[u8]>::is_empty));
    assert!(reopened.xref().location(0).is_none(), "object zero is free");
}

/// §7.5.6 requires every old trailer entry except /Prev to be restated. Annex E.2 gives
/// `/pdfa_NewFeature` as a second-class name. Inspect the newest section's own trailer because
/// `Document::trailer()` merges old and new dictionaries and would hide an omission.
#[test]
fn an_incremental_update_repeats_a_second_class_trailer_entry() {
    // Positive control: valid complete document, ordinary update, and raw newest trailer.
    let control = open(source_pdf(""));
    assert_one_page(&control);
    let control_update = incremental_update(&control, &with_language(&control, b"fr-FR"))
        .expect("ordinary update writes");
    assert!(control_update.starts_with(control.bytes().whole().expect("source bytes held")));
    let control_reopened = open(control_update);
    assert_one_page(&control_reopened);
    assert_eq!(
        control_reopened
            .get_key(
                &control_reopened.catalog().expect("control catalog"),
                "Lang"
            )
            .as_string(),
        Some(&b"fr-FR"[..]),
        "positive control really applied the requested catalog update"
    );
    let control_sections = xref::sections(control_reopened.bytes(), control_reopened.limits());
    assert_eq!(control_sections.len(), 2);
    let control_latest = control_sections
        .first()
        .expect("the update section is present");
    assert!(control_latest.classic);
    assert!(control_latest.trailer.get("Root").is_some());
    assert!(control_latest.trailer.get("ID").is_some());
    assert_eq!(
        control_latest
            .trailer
            .get("Prev")
            .and_then(Object::as_integer),
        Some(i64::try_from(control_sections[1].offset).expect("small fixture offset")),
        "/Prev points to the original xref section"
    );

    let source_bytes = source_pdf("/pdfa_NewFeature << /Feature1 true >> ");
    let original = open(source_bytes.clone());
    assert_one_page(&original);
    let extension = original
        .trailer()
        .get("pdfa_NewFeature")
        .and_then(Object::as_dict)
        .expect("Annex E example entry is present");
    assert_eq!(
        original.get_key(extension, "Feature1"),
        Object::Boolean(true)
    );

    let updated = incremental_update(&original, &with_language(&original, b"fr-FR"))
        .expect("the extension-bearing document updates");
    assert!(
        updated.starts_with(&source_bytes),
        "update preserves its source prefix"
    );
    let reopened = open(updated);
    assert_one_page(&reopened);
    let sections = xref::sections(reopened.bytes(), reopened.limits());
    assert_eq!(sections.len(), 2);
    let newest = sections.first().expect("new section precedes the original");
    assert!(newest.classic);
    assert!(newest.trailer.get("Root").is_some());
    assert!(newest.trailer.get("ID").is_some());
    assert_eq!(
        newest.trailer.get("Prev").and_then(Object::as_integer),
        Some(i64::try_from(sections[1].offset).expect("small fixture offset")),
        "/Prev points to the original xref section"
    );
    assert_eq!(
        newest.trailer.get("pdfa_NewFeature"),
        original.trailer().get("pdfa_NewFeature"),
        "§7.5.6 requires every prior trailer entry except /Prev to be restated"
    );
}

/// §7.5.2's enumerated headers are accepted without recovery or a version substitution.
#[test]
fn every_required_header_version_opens_the_same_complete_document() {
    for (major, minor) in (0..=7).map(|minor| (1u8, minor)).chain([(2, 0)]) {
        let mut bytes = source_pdf("");
        bytes[5] = b'0' + major;
        bytes[7] = b'0' + minor;
        let document = open(bytes);
        assert_one_page(&document);
        assert_eq!(document.version(), Some(Version { major, minor }));
        assert_eq!(
            document.get_key(&document.catalog().expect("catalog"), "Lang"),
            Object::String(b"en-US".to_vec().into())
        );
    }
}

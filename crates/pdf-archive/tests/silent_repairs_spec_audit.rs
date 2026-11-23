//! The silent-repair inventory's behaviour findings (review 2194, ADR 3477).
//!
//! Each test builds the smallest file on which a reader crate repairs, recovers, substitutes or
//! tolerates instead of reading what the bytes say, beside a twin that differs only by the
//! damage, and asks that [`pdf_archive::check`] fail at least one requirement on the damaged file
//! that it does not fail on the twin. The twin is not a conforming PDF/A file (it has no output
//! intent and no metadata); what the comparison isolates is whether the *repair* reaches a
//! verdict. Every test here fails until the fix rounds of ADR 3477 carry the repair to the
//! validator, and each is listed in `doc/reviews/2194-silent-repairs/expected-failures.json`.

#![expect(
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    reason = "test code: fixtures are small, built here, and a fixture that cannot open should fail loudly"
)]

use std::fmt::Write as _;

use pdf_archive::{Level, Outcome, Target, check};
use pdf_syntax::Document;

/// The target every test holds its pair to: Level U, so that section 6.2.11.7.2 binds as well.
const TARGET: Target = Target::Two(Level::U);

/// The file identifier every fixture states, so that section 6.1.3 is met on both twins.
const ID: &str = "<00112233445566778899aabbccddeeff>";

/// A whole file from object bodies, numbered from 1, with an exact cross-reference table.
///
/// A body that already holds an `obj` keyword is written as it stands, so a test can damage the
/// header or the `endobj`; the table still names the offset the body begins at. `offset_error`
/// is added to every offset the table states.
fn file_with(objects: &[String], offset_error: usize) -> Vec<u8> {
    let mut bytes: Vec<u8> = b"%PDF-1.7\n%\xe2\xe3\xcf\xd3\n".to_vec();
    let mut offsets = Vec::with_capacity(objects.len());
    for (index, body) in objects.iter().enumerate() {
        offsets.push(bytes.len());
        if body.contains(" obj\n") {
            bytes.extend_from_slice(body.as_bytes());
            bytes.push(b'\n');
        } else {
            bytes.extend_from_slice(format!("{} 0 obj\n{body}\nendobj\n", index + 1).as_bytes());
        }
    }
    let xref = bytes.len();
    let size = objects.len() + 1;
    let mut table = format!("xref\n0 {size}\n0000000000 65535 f \n");
    for offset in offsets {
        let _ = writeln!(table, "{:010} 00000 n ", offset + offset_error);
    }
    let _ = write!(
        table,
        "trailer\n<< /Size {size} /Root 1 0 R /ID [{ID} {ID}] >>\nstartxref\n{xref}\n%%EOF\n"
    );
    bytes.extend_from_slice(table.as_bytes());
    bytes
}

/// [`file_with`] with every offset exact.
fn file(objects: &[String]) -> Vec<u8> {
    file_with(objects, 0)
}

/// A content stream whose `/Length` is exact.
fn content(operators: &str) -> String {
    format!(
        "<< /Length {} >>\nstream\n{operators}\nendstream",
        operators.len()
    )
}

/// A one-page document: catalog, page tree, page (with `page_extra` inside its dictionary and
/// `resources` as its resources), and the page's content stream as object 4.
fn page(operators: &str, resources: &str, page_extra: &str) -> Vec<String> {
    vec![
        "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] /Contents 4 0 R \
             /Resources {resources} {page_extra} >>"
        ),
        content(operators),
    ]
}

/// The requirements `check` fails on a file.
fn failed(bytes: Vec<u8>) -> Vec<&'static str> {
    let document = Document::open(bytes).expect("the fixture opens");
    check(&document, TARGET)
        .judgements
        .into_iter()
        .filter(|judgement| matches!(judgement.outcome, Outcome::Failed { .. }))
        .map(|judgement| judgement.id)
        .collect()
}

/// Asserts that the damaged file fails a requirement its twin does not.
fn repair_is_reported(twin: Vec<u8>, damaged: Vec<u8>, repair: &str) {
    let control = Document::open(twin.clone()).expect("the twin opens");
    assert!(
        !control.was_recovered(),
        "the twin's cross-reference table is the file's own"
    );
    let before = failed(twin);
    let after = failed(damaged);
    let new: Vec<_> = after.iter().filter(|id| !before.contains(id)).collect();
    assert!(
        !new.is_empty(),
        "the reader repaired {repair}, and the validator failed nothing the twin does not \
         fail (the twin fails {before:?}, the damaged file {after:?})"
    );
}

/// The twin every page-level test damages.
fn plain_page() -> Vec<String> {
    page("0 0 m 1 1 l S", "<< >>", "")
}

// ---------------------------------------------------------------------------------------------
// 2194-B01: an integer past 64 bits outside a content stream.

/// ISO 19005-2 section 6.1.13 bounds every integer in the file; ADR 3393 carried the lexer's
/// `integer_out_of_range` to the validator for content streams only. A dictionary's value is
/// read by the same lexer as the nearest real, and the integer row reads the real.
#[test]
fn an_integer_past_sixty_four_bits_in_a_dictionary_is_an_integer_silent_repair() {
    let damaged = page(
        "0 0 m 1 1 l S",
        "<< >>",
        "/StructParents 1234567890123456789012345678901234",
    );
    repair_is_reported(
        file(&plain_page()),
        file(&damaged),
        "a 34-digit integer in a page dictionary as a real",
    );
}

// ---------------------------------------------------------------------------------------------
// 2194-B02: an object the reader cannot load is read as null, and its content leaves the walk.

/// ISO 32000-2 §7.3.10's null is the value of a reference to an object that does not exist; an
/// object that exists and does not parse is not that. `Document::get` caches `Object::Null` for
/// both, so the page's content stream vanishes from every content row.
#[test]
fn an_unparsable_content_stream_object_is_not_an_empty_page_silent_repair() {
    let mut damaged = plain_page();
    damaged[3] = "<< /Length 3 >> )) not an object".to_owned();
    repair_is_reported(
        file(&plain_page()),
        file(&damaged),
        "an object that does not parse as null",
    );
}

/// The cross-reference entry for object 4 points at an object whose header names 9 0: the
/// reader reads object 4 as null (`Document::misfiled_objects` records it), and no row asks.
#[test]
fn a_cross_reference_entry_at_another_objects_header_is_not_null_silent_repair() {
    let mut damaged = plain_page();
    damaged[3] = format!("9 0 obj\n{}\nendobj", content("0 0 m 1 1 l S"));
    repair_is_reported(
        file(&plain_page()),
        file(&damaged),
        "a misfiled object as null",
    );
}

// ---------------------------------------------------------------------------------------------
// 2194-B03: an object with no endobj.

/// ISO 32000-2 §7.3.10 closes every indirect object with `endobj`, and ISO 19005-2 section 6.1.9
/// sets the keyword's line endings. The reader reads the object without it, and the row's
/// `end_of_object` returns when it finds no keyword to judge.
#[test]
fn an_object_without_endobj_fails_the_object_syntax_silent_repair() {
    let mut damaged = plain_page();
    damaged[1] = "2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\n".to_owned();
    repair_is_reported(
        file(&plain_page()),
        file(&damaged),
        "an indirect object with no endobj keyword",
    );
}

// ---------------------------------------------------------------------------------------------
// 2194-B04: the cross-reference information is repaired.

/// Every offset of the table is three bytes past its object's header (ISO 32000-2 §7.5.4: the
/// offset is the object's). The reader finds the objects anyway, without a rebuild and without
/// a record.
#[test]
fn cross_reference_offsets_that_miss_their_objects_are_reported_silent_repair() {
    repair_is_reported(
        file(&plain_page()),
        file_with(&plain_page(), 3),
        "cross-reference offsets that name no object header",
    );
}

/// An update section whose trailer's `/Prev` is its own offset (ISO 32000-2 §7.5.6): the reader
/// rebuilds the table by scanning (`Document::was_recovered`), which no row asks.
#[test]
fn a_prev_chain_that_loops_is_reported_silent_repair() {
    let twin = file(&plain_page());
    let mut damaged = twin.clone();
    let at = damaged.len();
    damaged.extend_from_slice(
        format!(
            "xref\n0 1\n0000000000 65535 f \ntrailer\n<< /Size 5 /Root 1 0 R /Prev {at} \
             /ID [{ID} {ID}] >>\nstartxref\n{at}\n%%EOF\n"
        )
        .as_bytes(),
    );
    repair_is_reported(twin, damaged, "a /Prev cycle by rebuilding the table");
}

// ---------------------------------------------------------------------------------------------
// 2194-B05: an embedded font program the reader repaired or could only partly read.

/// `LiberationSans`, embedded whole as `/FontFile2`, shown through a non-symbolic `TrueType` font.
fn truetype_document(program: &[u8]) -> Vec<u8> {
    let operators = "BT /F1 12 Tf (ab) Tj ET";
    let mut objects: Vec<Vec<u8>> = vec![
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] /Contents 4 0 R \
          /Resources << /Font << /F1 5 0 R >> >> >>"
            .to_vec(),
        content(operators).into_bytes(),
        b"<< /Type /Font /Subtype /TrueType /BaseFont /AAAAAA+LiberationSans /FirstChar 97 \
          /LastChar 98 /Widths [556 556] /Encoding /WinAnsiEncoding /FontDescriptor 6 0 R >>"
            .to_vec(),
        b"<< /Type /FontDescriptor /FontName /AAAAAA+LiberationSans /Flags 32 \
          /FontBBox [-203 -303 1050 910] /ItalicAngle 0 /Ascent 905 /Descent -212 \
          /CapHeight 716 /StemV 80 /FontFile2 7 0 R >>"
            .to_vec(),
    ];
    let mut stream = format!(
        "<< /Length {} /Length1 {} >>\nstream\n",
        program.len(),
        program.len()
    )
    .into_bytes();
    stream.extend_from_slice(program);
    stream.extend_from_slice(b"\nendstream");
    objects.push(stream);
    let mut bytes: Vec<u8> = b"%PDF-1.7\n%\xe2\xe3\xcf\xd3\n".to_vec();
    let mut offsets = Vec::with_capacity(objects.len());
    for (index, body) in objects.iter().enumerate() {
        offsets.push(bytes.len());
        bytes.extend_from_slice(format!("{} 0 obj\n", index + 1).as_bytes());
        bytes.extend_from_slice(body);
        bytes.extend_from_slice(b"\nendobj\n");
    }
    let xref = bytes.len();
    let size = objects.len() + 1;
    let mut table = format!("xref\n0 {size}\n0000000000 65535 f \n");
    for offset in offsets {
        let _ = writeln!(table, "{offset:010} 00000 n ");
    }
    let _ = write!(
        table,
        "trailer\n<< /Size {size} /Root 1 0 R /ID [{ID} {ID}] >>\nstartxref\n{xref}\n%%EOF\n"
    );
    bytes.extend_from_slice(table.as_bytes());
    bytes
}

/// The program every font test embeds.
const LIBERATION_SANS: &[u8] =
    include_bytes!("../../../data/standard-fonts/LiberationSans-Regular.ttf");

/// `head.indexToLocFormat` flipped, so the `loca` the program carries is read at the wrong
/// width. `pdf_font::sfnt::repaired_font_program` rewrites the format before the glyphs are
/// read, and nothing records that the program was rewritten (ISO 19005-2 section 6.2.11.4.1:
/// the embedded program is the one a reader renders).
#[test]
fn a_truetype_program_whose_loca_format_is_wrong_is_reported_silent_repair() {
    let mut damaged = LIBERATION_SANS.to_vec();
    let tables = usize::from(u16::from_be_bytes([damaged[4], damaged[5]]));
    let head = (0..tables)
        .map(|index| 12 + 16 * index)
        .find(|record| damaged.get(*record..record + 4) == Some(b"head".as_slice()))
        .expect("LiberationSans has a head table");
    let offset = usize::try_from(u32::from_be_bytes([
        damaged[head + 8],
        damaged[head + 9],
        damaged[head + 10],
        damaged[head + 11],
    ]))
    .expect("an offset fits a usize");
    // indexToLocFormat is the int16 at byte 50 of `head`; its low byte selects the width.
    damaged[offset + 51] ^= 1;
    repair_is_reported(
        truetype_document(LIBERATION_SANS),
        truetype_document(&damaged),
        "a TrueType program's loca format",
    );
}

/// The program cut to its first half, its `/Length1` saying so: the glyphs of `a` and `b` are
/// past the cut, and the reader draws what it can without a refusal the font rows read.
#[test]
fn a_truetype_program_cut_in_half_is_reported_silent_repair() {
    let half = LIBERATION_SANS
        .get(..LIBERATION_SANS.len() / 2)
        .expect("a prefix");
    repair_is_reported(
        truetype_document(LIBERATION_SANS),
        truetype_document(half),
        "a truncated TrueType program",
    );
}

// ---------------------------------------------------------------------------------------------
// 2194-B06: the base standard's syntax, tolerated (section 5.1 only).

/// ISO 32000-2 §7.3.7: "Multiple entries in the same dictionary shall not have the same key."
/// The parser keeps the first and drops the rest, and section 5.1's row is unchecked.
#[test]
fn a_dictionary_with_a_repeated_key_is_reported_silent_repair() {
    let damaged = page("0 0 m 1 1 l S", "<< >>", "/MediaBox [0 0 20 20]");
    repair_is_reported(
        file(&plain_page()),
        file(&damaged),
        "a repeated dictionary key",
    );
}

/// ISO 32000-2 §7.3.4.2: a literal string is balanced parentheses. One with no closing `)`
/// runs to the end of the content stream and is read as a string.
#[test]
fn an_unterminated_literal_string_is_reported_silent_repair() {
    repair_is_reported(
        file(&plain_page()),
        file(&page("0 0 m 1 1 l S (abc", "<< >>", "")),
        "an unterminated literal string",
    );
}

/// ISO 32000-2 §7.3.7: every key is a name. A dictionary entry whose key is an integer is
/// skipped and the dictionary read on.
#[test]
fn a_dictionary_key_that_is_not_a_name_is_reported_silent_repair() {
    let mut damaged = plain_page();
    damaged[2] = "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] 5 /Contents 4 0 R \
                  /Resources << >> >>"
        .to_owned();
    repair_is_reported(
        file(&plain_page()),
        file(&damaged),
        "an integer where a key belongs",
    );
}

// ---------------------------------------------------------------------------------------------
// 2194-B07: page-tree entries of the wrong type or value, substituted.

/// ISO 32000-2 §7.7.3.3, Table 31: `/MediaBox` is a rectangle. A name there is replaced by the
/// reader's own box (`Page::substituted_media_box`), which no row reads.
#[test]
fn a_media_box_that_is_not_a_rectangle_is_reported_silent_repair() {
    let mut damaged = plain_page();
    damaged[2] = "<< /Type /Page /Parent 2 0 R /MediaBox /Letter /Contents 4 0 R \
                  /Resources << >> >>"
        .to_owned();
    repair_is_reported(
        file(&plain_page()),
        file(&damaged),
        "a page's /MediaBox that is a name",
    );
}

/// Table 31: `/Rotate` "shall be a multiple of 90". 45 is read as no rotation.
#[test]
fn a_rotation_that_is_not_a_multiple_of_ninety_is_reported_silent_repair() {
    repair_is_reported(
        file(&plain_page()),
        file(&page("0 0 m 1 1 l S", "<< >>", "/Rotate 45")),
        "a /Rotate of 45",
    );
}

// ---------------------------------------------------------------------------------------------
// 2194-B08: content-stream operators given operands they do not take.

/// ISO 32000-2 §8.5.2.1, Table 58: `m` takes two numbers. A string in their place makes the
/// operator do nothing; the survey walks tokens and does not ask.
#[test]
fn an_operand_of_the_wrong_type_is_reported_silent_repair() {
    repair_is_reported(
        file(&plain_page()),
        file(&page("(a) 0 m 1 1 l S", "<< >>", "")),
        "a string operand to m",
    );
}

/// §8.5.2.1: `m` takes two operands, and one is given. The interpreter reports
/// `Unsupported::OperandShortfall` to the viewer; nothing reaches the validator.
#[test]
fn an_operator_short_of_its_operands_is_reported_silent_repair() {
    repair_is_reported(
        file(&plain_page()),
        file(&page("0 m 1 1 l S", "<< >>", "")),
        "m with one operand",
    );
}

/// ISO 32000-2 §8.4.2: "Occurrences of the q and Q operators shall be balanced within a given
/// content stream". A `Q` with nothing saved is ignored.
#[test]
fn a_restore_with_nothing_saved_is_reported_silent_repair() {
    repair_is_reported(
        file(&plain_page()),
        file(&page("Q 0 0 m 1 1 l S", "<< >>", "")),
        "Q with an empty stack",
    );
}

/// ISO 32000-2 §9.4.1: text-showing operators appear only inside a text object (Figure 9). A
/// `Tj` after the path is reported to the viewer as
/// `StreamSyntaxFault::TextOperatorOutsideATextObject`, and to the validator not at all.
#[test]
fn a_text_operator_outside_a_text_object_is_reported_silent_repair() {
    repair_is_reported(
        file(&plain_page()),
        file(&page("0 0 m 1 1 l S (a) Tj", "<< >>", "")),
        "Tj outside BT … ET",
    );
}

/// ISO 32000-2 §8.6.8, Table 73: `sc` takes as many operands as the current space has
/// components. Three under `DeviceGray` are read as `DeviceRGB` (`content::colour`), a colour
/// space the stream never selected.
#[test]
fn colour_operands_that_do_not_fit_the_space_are_reported_silent_repair() {
    repair_is_reported(
        file(&page("/DeviceGray cs 0.5 sc 0 0 m 1 1 l S", "<< >>", "")),
        file(&page("/DeviceGray cs 1 0 0 sc 0 0 m 1 1 l S", "<< >>", "")),
        "three sc operands under DeviceGray as DeviceRGB",
    );
}

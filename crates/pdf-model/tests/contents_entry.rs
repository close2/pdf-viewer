//! Table 31's `/Contents`, and the four ways a page can end up with no drawing.
//!
//! ISO 32000-2 §7.7.3.3 Table 31 makes `/Contents` optional:
//!
//! > ( Optional ) A content stream (see 7.8.2, "Content streams") that shall describe the
//! > contents of this page. If this entry is absent, the page shall be empty.
//!
//! So a page without one is a *page*, not a defect, and saying anything about it would be
//! noise. Three other shapes reach the same blank page and are not the same statement, and
//! this file is about telling them apart:
//!
//! | the file says | the standard says | this reader says |
//! |---|---|---|
//! | nothing | "the page shall be empty" (Table 31) | nothing |
//! | `/Contents null` | "shall be equivalent to omitting the entry entirely" (§7.3.9) | nothing |
//! | `/Contents 99 0 R`, and 99 is not there | "shall be treated as a reference to the null object" (§7.3.10) | [`ContentIssue::Unreachable`] |
//! | `/Contents 4 0 R`, and 4 is a dictionary | Table 31 requires a stream | [`ContentIssue::NotAStream`] |
//!
//! # Why the third one is a report and not a silence
//!
//! §7.3.10 makes an undefined reference conforming, so drawing nothing is *allowed*. Drawing
//! nothing in silence is a different question, and it is `CLAUDE.md`'s: a page whose producer
//! named a content stream and got a blank page is not a page whose producer stated none.
//! ADR 0258, and it is ADR 0255's argument one clause over.
//!
//! # Where the witness came from
//!
//! Not from the pdf.js corpus, which contains no such page — the gate's count is unmoved by
//! this report. It came from `doc/corpora/pdf-differences`, added in the same session:
//! `UnknownFilter-PageContentStream.pdf`, whose content stream object's dictionary ends with
//! one `>` where §7.3.7 requires two, so the object does not parse and the reference lands on
//! nothing. This tree drew a blank page and said nothing; `poppler` prints
//! *Syntax Error: Illegal character '>'*.

#![expect(
    clippy::expect_used,
    reason = "test code: a malformed fixture should fail loudly"
)]

use std::fmt::Write as _;
use std::path::Path;

use pdf_model::page::ContentIssue;
use pdf_syntax::{Damage, Document};

/// A one-page PDF whose page dictionary states `contents` for `/Contents`.
///
/// Object 4 is a stream and object 5 is a plain dictionary, so a caller can point the entry
/// at either — or at object 99, which is not written at all.
fn page_stating(contents: &str) -> Vec<u8> {
    page_stating_with(contents, "")
}

/// The same, with one further object — written out in full, `endobj` and all — after object 5.
///
/// The zero-length and truncated parts below cannot be expressed by pointing at objects 4 or 5,
/// because what they are about is the *stream* rather than which object the entry names.
fn page_stating_with(contents: &str, extra: &str) -> Vec<u8> {
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 50] \
         /Resources << >> {contents}>>\nendobj\n\
         4 0 obj\n<< /Length 11 >>\nstream\n0 0 10 10 re\nendstream\nendobj\n\
         5 0 obj\n<< /Type /NotAStream >>\nendobj\n{extra}"
    );

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
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n"
    );
    out.into_bytes()
}

/// What the page's `/Contents` produced.
fn issues(contents: &str) -> Vec<ContentIssue> {
    let document = Document::open(page_stating(contents)).expect("the fixture opens");
    let page = pdf_model::Pages::new(&document)
        .get(0)
        .expect("the fixture has a page");
    page.content_with_report(&document).1
}

/// Table 31's own sentence: an absent entry is an empty page and nothing is owed about it.
#[test]
fn a_page_that_states_no_contents_is_silent() {
    assert_eq!(issues(""), Vec::new());
}

/// §7.3.9, verbatim: "Specifying the null object as the value of a dictionary entry (7.3.7,
/// "Dictionary objects") shall be equivalent to omitting the entry entirely."
#[test]
fn a_contents_entry_that_is_the_null_object_is_silent() {
    assert_eq!(issues("/Contents null "), Vec::new());
}

/// The finding. §7.3.10 permits the blank page; it does not permit the silence.
#[test]
fn a_contents_entry_naming_an_object_that_is_not_there_says_so() {
    let reported = issues("/Contents 99 0 R ");
    assert_eq!(
        reported,
        vec![ContentIssue::Unreachable {
            index: 0,
            object: pdf_syntax::ObjectId::new(99, 0),
        }]
    );
}

/// The same inside Table 31's array form, where the entry that fails is one of several — so
/// the page still draws the parts it can and the index says which one it lost.
#[test]
fn one_unreachable_part_of_an_array_is_named_by_its_index() {
    let document =
        Document::open(page_stating("/Contents [4 0 R 99 0 R] ")).expect("the fixture opens");
    let page = pdf_model::Pages::new(&document)
        .get(0)
        .expect("the fixture has a page");
    let (bytes, reported) = page.content_with_report(&document);
    assert!(
        String::from_utf8_lossy(&bytes).contains("re"),
        "the reachable part still contributes its bytes"
    );
    assert_eq!(
        reported,
        vec![ContentIssue::Unreachable {
            index: 1,
            object: pdf_syntax::ObjectId::new(99, 0),
        }]
    );
}

/// A `/Contents` that resolves to something which is not a stream was already reported, and
/// stays reported: the new variant must not have swallowed it.
#[test]
fn a_contents_entry_that_is_not_a_stream_still_says_so() {
    assert_eq!(
        issues("/Contents 5 0 R "),
        vec![ContentIssue::NotAStream { index: 0 }]
    );
}

/// **A bare integer where the reference should be: `/Contents 8`.**
///
/// The row above states the same verdict, but it gets there by naming an object that *is*
/// there and is the wrong kind. This one never becomes a reference at all — §7.3.10 writes an
/// indirect reference as "the object number, the generation number and the keyword `R`", and
/// a lone `8` supplies one of the three, so §7.3.3 has the token and it is the integer 8.
///
/// It is worth its own row because the two shapes fail in different places. The four-object
/// fixture reaches [`ContentIssue::NotAStream`] after a resolution that succeeded; this one
/// reaches it after no resolution at all, through the arm that has to tell an integer apart
/// from the null object — and null is the shape §7.3.9 says must stay silent. A reader that
/// collapsed the two would either lose this report or invent one for every empty page.
///
/// `hayro`'s issue 1189 is this literal dictionary — `/Pages 4 0 R /Type /Catalog /Contents 8`
/// — and there it unwrapped a `None`. Nothing here can: the parser rewinds and hands back
/// `Object::Integer(8)` when the `R` does not follow, and the reader matches on what it got.
#[test]
fn a_contents_entry_that_is_a_bare_integer_says_so_too() {
    assert_eq!(
        issues("/Contents 8 "),
        vec![ContentIssue::NotAStream { index: 0 }],
        "an integer is not a reference, and it is not the null object either"
    );
    // The same integer inside the array form, where the index has to name which part.
    assert_eq!(
        issues("/Contents [4 0 R 8] "),
        vec![ContentIssue::NotAStream { index: 1 }],
        "a good part beside a bad one keeps its own index"
    );
}

/// The witness, from `doc/corpora/pdf-differences` — a real file rather than a fixture.
///
/// Skipped where the submodule is not checked out, which is the pattern `tests/corpus.rs`
/// uses: a checkout without submodules is not a broken build.
#[test]
fn the_pdf_association_witness_reports_rather_than_drawing_a_blank_page() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(
        "../../doc/corpora/pdf-differences/UnknownFilter/UnknownFilter-PageContentStream.pdf",
    );
    let Ok(bytes) = std::fs::read(&path) else {
        println!("skipped: {} is not checked out", path.display());
        return;
    };
    let document = Document::open(bytes).expect("the witness opens");
    let page = pdf_model::Pages::new(&document)
        .get(0)
        .expect("the witness has a page");
    let interpretation = pdf_model::interpret(&document, &page);
    assert!(
        !interpretation.is_complete(),
        "a page that draws nothing because it could not reach its content stream must say so"
    );
    assert_eq!(
        page.content_with_report(&document).1,
        vec![ContentIssue::Unreachable {
            index: 0,
            // Object 10, whose dictionary ends `>` where §7.3.7 requires `>>`.
            object: pdf_syntax::ObjectId::new(10, 0),
        }]
    );
}

/// A fifth shape, and the one a 5944-document web sample brought: a part that states a filter
/// and no bytes.
///
/// §7.3.8.1 makes "zero or more bytes" a conforming stream, so `<< /Filter /FlateDecode
/// /Length 0 >>` is a *valid* part that carries nothing — and Table 5 has `/Filter` name what
/// "shall be applied in processing the stream data found between the keywords stream and
/// endstream ", which with no data found is nothing at all. `flate` refuses an empty input,
/// because RFC 1950 gives a zlib stream a six-byte floor, so this part used to be reported as
/// drawing this page missed. It cannot be: an empty part draws nothing by construction.
#[test]
fn a_part_the_file_states_is_empty_is_not_a_missing_content_stream() {
    let extra = "6 0 obj\n<< /Filter /FlateDecode /Length 0 >>\nstream\n\nendstream\nendobj\n";
    let document = Document::open(page_stating_with("/Contents [4 0 R 6 0 R] ", extra))
        .expect("the fixture opens");
    let page = pdf_model::Pages::new(&document)
        .get(0)
        .expect("the fixture has a page");
    let (content, issues) = page.content_with_report(&document);
    assert_eq!(issues, Vec::new(), "an empty part is missing no drawing");
    assert!(
        content.starts_with(b"0 0 10 10 re"),
        "the part that does hold drawing is still there: {content:?}"
    );
}

/// A part that decodes only as far as its damage draws its prefix **and says so**.
///
/// ADR 0343, and the third statement this file distinguishes. [`ContentIssue::Undecodable`] is a
/// part nothing survived of and [`ContentIssue::TooLarge`] is a bound of ours; this one is a part
/// that gave what it had. §7.4.1 asks a reader to "invoke the corresponding decoding filter or
/// filters to convert the information back to its original form", and a damaged stream is a
/// decode that did the first half and could not finish the second — so the bytes go on the page,
/// because they are the producer's own, and the shortfall goes in the report, because a page cut
/// short otherwise looks like a page meant to be sparse.
///
/// `RunLengthDecode` is the filter here for a reason about the fixture rather than about the
/// rule: §7.4.5 states its end-of-data as a length byte of 128, so a stream ending without one
/// is truncated in the clause's own words, and unlike a deflate stream it can be written into
/// this file's text fixture a byte at a time. `flate` and `lzw` answer the same question the
/// same way, and `stream_length_bound.rs` pins those two at the filter.
#[test]
fn a_part_that_decodes_part_way_draws_its_prefix_and_reports_the_shortfall() {
    // One literal run of thirteen bytes — the length byte is 12, because "the following length +
    // 1 (1 to 128) bytes shall be copied literally" — and then nothing, where a 128 should be.
    let extra = "6 0 obj\n<< /Filter /RunLengthDecode /Length 14 >>\nstream\n\u{c}0 20 10 10 re\nendstream\nendobj\n";
    let document = Document::open(page_stating_with("/Contents [4 0 R 6 0 R] ", extra))
        .expect("the fixture opens");
    let page = pdf_model::Pages::new(&document)
        .get(0)
        .expect("the fixture has a page");
    let (content, issues) = page.content_with_report(&document);

    assert_eq!(
        issues,
        vec![ContentIssue::Damaged {
            index: 1,
            damage: Damage::Truncated,
            kept: 13,
            filters: vec!["RunLengthDecode".to_owned()],
        }],
        "the shortfall is named, with what survived it"
    );
    assert!(
        String::from_utf8_lossy(&content).contains("0 20 10 10 re"),
        "and the run that did decode is on the page: {content:?}"
    );
}

/// The other side of it, and the reason the condition is not "the bytes are empty".
///
/// A stream cut off at the `stream` keyword also arrives holding nothing, because `pdf-syntax`
/// recovers a wrong `/Length` by searching for `endstream`. §7.3.8.2 tells the two apart — the
/// `/Length` entry "indicates how many bytes of the PDF file are used for the stream's data",
/// and here it states 4096 that are not there — so this one stays reported. Two `SafeDocs`
/// documents of each kind are named in ADR 0266.
#[test]
fn a_part_truncated_to_nothing_is_still_reported() {
    let extra = "6 0 obj\n<< /Filter /FlateDecode /Length 4096 >>\nstream\n\nendstream\nendobj\n";
    let document = Document::open(page_stating_with("/Contents [4 0 R 6 0 R] ", extra))
        .expect("the fixture opens");
    let page = pdf_model::Pages::new(&document)
        .get(0)
        .expect("the fixture has a page");
    assert_eq!(
        page.content_with_report(&document).1,
        vec![ContentIssue::Undecodable {
            index: 1,
            filters: vec!["FlateDecode".to_owned()],
        }]
    );
}

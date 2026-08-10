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
use pdf_syntax::Document;

/// A one-page PDF whose page dictionary states `contents` for `/Contents`.
///
/// Object 4 is a stream and object 5 is a plain dictionary, so a caller can point the entry
/// at either — or at object 99, which is not written at all.
fn page_stating(contents: &str) -> Vec<u8> {
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 50] \
         /Resources << >> {contents}>>\nendobj\n\
         4 0 obj\n<< /Length 11 >>\nstream\n0 0 10 10 re\nendstream\nendobj\n\
         5 0 obj\n<< /Type /NotAStream >>\nendobj\n"
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

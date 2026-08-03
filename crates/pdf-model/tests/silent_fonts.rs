//! A font whose program draws nothing must say so.
//!
//! `Interpretation::is_complete` is the claim that the interpreter drew everything the page
//! asked for, and trap 1 is the standing warning about how little that can mean. This is the
//! narrowest version of it: a page of text whose font program answers **every** code with no
//! outline draws a blank page and, until the hundred-and-ninety-third session, reported
//! `unsupported: []` while doing it.
//!
//! `issue13316_reduced.pdf` is the witness — 200×50 points, one `Tj` of nine codes through an
//! embedded `TrueType` program, and `0 commands` — and the condition is the one ADR 0152 wrote
//! for a substituted face, applied where the code had been applying something else: **no code
//! reached an outline**. What keeps it from firing on ordinary pages is that a code reading
//! back as whitespace is not counted at all: a space is *meant* to be blank, and counting one
//! took the corpus's incomplete documents from 79 to 109.
//!
//! The tests are against real documents, which is trap 4's rule: a hand-built font program with
//! no outlines would be built by the same reading of the format the code under test uses.

#![expect(
    clippy::panic,
    reason = "test code: a document that stops opening should fail loudly, naming itself"
)]

use std::path::{Path, PathBuf};

use pdf_syntax::Document;

/// Page one's interpretation, or `None` when the corpus submodule is not checked out.
fn page_one(name: &str) -> Option<pdf_model::Interpretation> {
    let path: PathBuf = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../doc/pdf.js/test/pdfs")
        .join(name);
    let bytes = std::fs::read(path).ok()?;
    let document = Document::open(bytes).unwrap_or_else(|e| panic!("{name} does not open: {e}"));
    let page = pdf_model::Pages::new(&document)
        .get(0)
        .unwrap_or_else(|| panic!("{name} has no page one"));
    Some(pdf_model::interpret(&document, &page))
}

/// Every report the interpretation carries, as one string.
fn reports(interpretation: &pdf_model::Interpretation) -> String {
    format!("{:?}", interpretation.unsupported)
}

/// A page whose only text draws nothing says so, and names the font.
#[test]
fn a_font_that_draws_none_of_its_codes_is_reported() {
    let Some(interpretation) = page_one("issue13316_reduced.pdf") else {
        return;
    };
    assert!(
        interpretation.display_list.commands().is_empty(),
        "the page still draws nothing: that is the defect this reports, not one it fixes"
    );
    let said = reports(&interpretation);
    assert!(
        said.contains("/F1") && said.contains("no outline for any"),
        "a blank page of text must not be silent: {said}"
    );
}

/// And a page whose text draws normally says nothing about its fonts.
///
/// The discriminating half. `tracemonkey.pdf` is fourteen pages of dense embedded text with
/// spaces in every line — the exact shape that a report counting *blank* glyphs as missing
/// marks would fire on, and it fired on thirty such documents before the whitespace codes were
/// excluded.
#[test]
fn a_page_of_ordinary_text_reports_nothing_about_its_fonts() {
    let Some(interpretation) = page_one("tracemonkey.pdf") else {
        return;
    };
    let said = reports(&interpretation);
    assert!(
        !said.contains("no outline for any"),
        "a page that draws its text must not report: {said}"
    );
    assert!(interpretation.is_complete(), "and it is complete: {said}");
}

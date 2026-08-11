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

/// A code whose glyph the font *contains* and draws as empty is not a mark missed.
///
/// `pr12564.pdf` was the largest single contributor to the corpus's silent missing-glyph count
/// — 26 of 62 — and not one of them is a missing mark: every one is code 35 through `/TT3`,
/// whose glyph 1 the program contains and describes with no contours, which is how an sfnt
/// stores a space. What made it look like a loss is the `/ToUnicode`, which reads that code
/// back as `#`, so the whitespace exemption in front of the count could not see it — and
/// `pdftotext` renders the page as `1101#Strayer#Drive`, which is the same statement from
/// outside. §9.6.5.4's routes ended at a glyph; what that glyph draws is the program's to say.
#[test]
fn a_code_whose_font_contains_an_empty_glyph_is_counted_apart_from_a_missing_mark() {
    let Some(interpretation) = page_one("pr12564.pdf") else {
        return;
    };
    assert_eq!(
        (
            interpretation.codes_without_a_glyph,
            interpretation.codes_reaching_a_blank_glyph
        ),
        (0, 26),
        "all 26 reach a glyph the program describes as empty"
    );
    assert!(
        interpretation.is_complete(),
        "and the page is drawn: {}",
        reports(&interpretation)
    );
}

/// And a code that reaches no glyph, or `.notdef`, still counts as one.
///
/// `issue14821.pdf` is the corpus's other half of the same branch and both ends of it appear on
/// one page. Five of its codes are `Identity-H` CIDs whose `loca` entries are empty by the
/// glyph table's own statement — the program contains those glyphs and draws them as nothing.
/// Three are ASCII codes in a nonsymbolic `TrueType` subset whose `(3, 1)` `cmap` maps all
/// three to glyph 0 and whose `post` is version 3.0 with no names at all, so every route
/// §9.6.5.4 states ends at `.notdef`: those three are text the reader loses.
#[test]
fn a_code_that_reaches_notdef_is_still_a_mark_missed() {
    let Some(interpretation) = page_one("issue14821.pdf") else {
        return;
    };
    assert_eq!(
        (
            interpretation.codes_without_a_glyph,
            interpretation.codes_reaching_a_blank_glyph
        ),
        (3, 5),
        "three codes reach .notdef and five reach an empty glyph"
    );
}

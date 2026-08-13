//! §7.8.2's rule that an operator's operands are the ones that *immediately precede* it.
//!
//! ISO 32000-2 §7.8.2 states it in one sentence:
//!
//! > In PDF, all of the operands needed by an operator shall immediately precede that
//! > operator. Operators do not return results, and operands shall not be left over when an
//! > operator finishes execution.
//!
//! A conforming stream never leaves one over, so the rule is invisible on every valid file:
//! the operands an operator is given are simultaneously the first and the last of what
//! precedes it, and reading from either end produces the same page. That is trap 8's fourth
//! shape — a rule the corpus exercises constantly and cannot show you — so each test below is
//! a **pair** of content streams differing only in a leftover operand, and the claim is that
//! the two draw the same thing.
//!
//! # Where the witness came from
//!
//! `openpreserve/format-corpus`'s `pdf-handbuilt-test-corpus`, whose files each state one
//! deliberate defect. `T02-05-01_008_Font-set-operator-missing.pdf` writes
//!
//! ```text
//! BT
//! /F0 36.
//! (Hello PDF-world!) Tj
//! ET
//! ```
//!
//! — the `Tf` keyword deleted and its operands left standing. Reading `Tj`'s operand from the
//! front of what preceded it found `/F0`, which is not a string, so the show was skipped: this
//! tree drew a blank page and **reported nothing**, which is the silent fallback `CLAUDE.md`
//! principle 1 forbids. Read from the back it is the string, `show_text` runs with no font set,
//! and the page says so.

#![expect(
    clippy::expect_used,
    reason = "test code: a malformed fixture should fail loudly"
)]

use std::fmt::Write as _;

use pdf_model::{Interpretation, Unsupported};
use pdf_syntax::Document;

/// A one-page PDF, 200 × 50, whose page states `content` and offers `/F0` as Helvetica.
fn page_drawing(content: &str) -> Vec<u8> {
    let length = content.len();
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 50] \
         /Resources << /Font << /F0 5 0 R >> >> /Contents 4 0 R >>\nendobj\n\
         4 0 obj\n<< /Length {length} >>\nstream\n{content}\nendstream\nendobj\n\
         5 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>\nendobj\n"
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

/// Interprets one content stream on that page.
fn interpretation(content: &str) -> Interpretation {
    let document = Document::open(page_drawing(content)).expect("the fixture opens");
    let page = pdf_model::Pages::new(&document)
        .get(0)
        .expect("the fixture has a page");
    pdf_model::interpret(&document, &page)
}

/// The display list's own `Debug` rendering, which is what "drew the same thing" means here:
/// every command, every coordinate and every colour, rather than a count of them.
fn drawn(content: &str) -> String {
    format!("{:?}", interpretation(content).display_list)
}

/// The two streams of a pair draw the same page, and it is not a blank one.
fn same_page(conforming: &str, with_leftovers: &str) {
    let expected = drawn(conforming);
    assert!(
        expected.contains("Glyph") || expected.contains("Fill"),
        "the conforming half has to draw something for the comparison to mean anything: \
         {expected}"
    );
    assert_eq!(drawn(with_leftovers), expected);
}

/// `Tj`, which is the shape the witness has: a name and a number left in front of the string.
#[test]
fn a_show_operator_reads_the_string_that_precedes_it() {
    same_page(
        "BT /F0 24 Tf 10 10 Td (Hi) Tj ET",
        "BT /F0 24 Tf 10 10 Td /F0 24 (Hi) Tj ET",
    );
}

/// `re` and `f`: the geometry half of the same rule, where a leftover shifts every coordinate
/// by one and the rectangle lands somewhere else entirely rather than being dropped.
#[test]
fn a_rectangle_reads_the_four_numbers_that_precede_it() {
    same_page("10 10 100 20 re f", "999 10 10 100 20 re f");
}

/// `cm`: six numbers, and a leftover in front of them turns the matrix into a different one
/// — the failure that draws a plausible page in the wrong place.
#[test]
fn a_matrix_reads_the_six_numbers_that_precede_it() {
    same_page(
        "2 0 0 2 5 5 cm 0 0 20 10 re f",
        "7 2 0 0 2 5 5 cm 0 0 20 10 re f",
    );
}

/// `Tf`: the two operands the font is named by, with a stray number in front of them.
#[test]
fn a_font_reads_the_name_and_size_that_precede_it() {
    same_page(
        "BT /F0 24 Tf 10 10 Td (Hi) Tj ET",
        "BT 99 /F0 24 Tf 10 10 Td (Hi) Tj ET",
    );
}

/// The witness itself, and the half that is about *silence* rather than about placement.
///
/// With the `Tf` keyword gone there is no font, so §9.3.1's "[t]here is no initial value for
/// either font or size; they shall be specified explicitly by using Tf before any text is
/// shown" makes the show undrawable. Drawing nothing is the only thing this reader can do;
/// **saying nothing is not**, and reading `Tj`'s operand from the front is what made the
/// operator disappear before `show_text` could report it.
#[test]
fn a_show_with_no_font_set_is_reported_rather_than_skipped() {
    let interpretation = interpretation("BT /F0 36. (Hello PDF-world!) Tj ET");
    assert_eq!(
        interpretation.unsupported,
        vec![Unsupported::Text { operations: 1 }],
        "a text-showing operator with no font is a lost mark and has to say so"
    );
}

/// The rule cuts both ways: an operator given *fewer* operands than it needs reads what is
/// there, which is what it did before, and is still refused rather than half-applied.
#[test]
fn an_operator_with_too_few_operands_is_unchanged() {
    // `re` with three numbers is not a rectangle; the path stays empty and `f` paints nothing.
    let interpretation = interpretation("10 10 100 re f");
    assert!(
        !format!("{:?}", interpretation.display_list).contains("Fill"),
        "three operands do not make a rectangle"
    );
}

/// `TJ` and `d` take an array the content lexer flattens into one operand per element, so
/// neither has a fixed count and both are given everything that precedes them. This pins that
/// they still are: a `TJ` whose array is long draws every run in it.
#[test]
fn an_array_operator_still_reads_all_of_its_operands() {
    let interpretation = interpretation("BT /F0 24 Tf 10 10 Td [(a) -20 (b) -20 (c)] TJ ET");
    assert_eq!(interpretation.text.trim(), "abc");
}

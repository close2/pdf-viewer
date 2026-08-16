//! §7.3.3's rule that a number is made of digits, and what a content stream owes a run that
//! states none.
//!
//! ISO 32000-2 §7.3.3 writes both numeric forms the same way round:
//!
//! > An integer shall be written as one or more decimal digits optionally preceded by a sign.
//!
//! > A real value shall be written as one or more decimal digits with an optional sign and a
//! > leading, trailing, or embedded PERIOD (2Eh) (decimal point).
//!
//! So `.` is not a real, `-` is not an integer, and neither is any object at all: each is a
//! run of regular characters (§7.2.3) that spells nothing the standard defines. §7.8.2 then
//! decides what one is *in a content stream* — "[a]n operator is a PDF keyword … distinguished
//! from a name object by the absence of an initial SOLIDUS", and "when a PDF reader encounters
//! an operator in a content stream that it does not recognise, an error shall occur".
//!
//! # Where the witness came from
//!
//! `openpreserve/format-corpus`'s `pdf-handbuilt-test-corpus`, whose files each state one
//! deliberate defect. `T02-05-01_006_font-size-operator-missing.pdf` writes
//!
//! ```text
//! BT
//! /F0 . Tf
//! (Hello PDF-world!) Tj
//! ET
//! ```
//!
//! and this tree read the `.` as a number, set a text font size of nought, drew invisible text
//! and said nothing — the plausible fallback trap 5 forbids. §9.3.1 is the other half of why
//! the silence is the defect: "[t]here is no initial value for either font or size; they shall
//! be specified explicitly by using Tf before any text is shown", so a `Tf` that states no size
//! leaves the show undrawable and the page has to say so.
//!
//! # Why every test here is a pair
//!
//! Trap 8's fourth shape: a rule the corpus exercises constantly and cannot show you. Every
//! conforming stream states digits, so the condition never fires on one, and the two halves of
//! each pair differ in exactly one character. The conforming half is asserted to draw, because
//! a comparison against a blank page proves nothing.

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

/// The conforming half of a pair: it draws, and it reports nothing.
fn draws_in_silence(content: &str) -> String {
    let interpretation = interpretation(content);
    assert_eq!(
        interpretation.unsupported,
        Vec::new(),
        "the conforming half states digits everywhere and has nothing to report"
    );
    let drawn = format!("{:?}", interpretation.display_list);
    assert!(
        drawn.contains("Glyph") || drawn.contains("Fill"),
        "the conforming half has to draw something for the comparison to mean anything: {drawn}"
    );
    drawn
}

/// The run this tree used to read as zero, named the way the interpreter names an operator it
/// does not recognise.
fn unrecognised(run: &str) -> Unsupported {
    Unsupported::Operator {
        operator: run.to_owned(),
    }
}

/// The witness itself: `/F0 . Tf`.
///
/// The `.` is no operand, so the `Tf` that follows it states no size — and §9.3.1 gives size
/// no initial value, so the show cannot be drawn. Two reports, and both are needed: the first
/// names the malformed token, the second names the mark that was lost because of it.
#[test]
fn a_font_size_that_states_no_digit_is_reported_rather_than_read_as_zero() {
    draws_in_silence("BT /F0 24 Tf 10 10 Td (Hi) Tj ET");

    let interpretation = interpretation("BT /F0 . Tf 10 10 Td (Hi) Tj ET");
    assert_eq!(
        interpretation.unsupported,
        vec![Unsupported::Text { operations: 1 }, unrecognised(".")],
        "a lone `.` is not a number, and the show it costs has to be reported"
    );
    assert!(
        !format!("{:?}", interpretation.display_list).contains("Glyph"),
        "a size that was never stated may not become a size of zero"
    );
}

/// The same rule on a coordinate, where reading zero draws a mark in the wrong place rather
/// than no mark at all.
///
/// `10 10 . 20 re` used to be `10 10 0 20 re`: a rectangle of no width, which §8.5.3.3.1 calls
/// device-dependent and which this tree paints as nothing — silently. Now the `.` ends the
/// operand run, `re` is left with too few operands to be a rectangle, and the stream says so.
#[test]
fn a_coordinate_that_states_no_digit_is_reported_rather_than_read_as_zero() {
    draws_in_silence("10 10 100 20 re f");

    let interpretation = interpretation("10 10 . 20 re f");
    assert_eq!(interpretation.unsupported, vec![unrecognised(".")]);
    assert!(
        !format!("{:?}", interpretation.display_list).contains("Fill"),
        "three operands do not make a rectangle"
    );
}

/// A lone sign is the same finding one character over, and it is the form the older reading
/// documented as "no digits at all becomes zero".
#[test]
fn a_lone_sign_is_no_number_either() {
    draws_in_silence("0 0 0 rg 10 10 100 20 re f");

    let interpretation = interpretation("0 0 - rg 10 10 100 20 re f");
    assert_eq!(
        interpretation.unsupported,
        vec![unrecognised("-")],
        "`-` states no digit, so it is no number and the `rg` that wanted it is malformed"
    );
}

/// §7.3.3's own EXAMPLE 2, drawn: every form the clause prints stays a number.
///
/// This is the half that keeps the condition honest. `4.` and `-.002` are as legal as `0`, and
/// a rule that asked for a digit *before* the point, or for a point at all, would refuse them.
#[test]
fn every_form_the_clause_prints_still_draws() {
    let drawn = draws_in_silence("+123.6 4. 34.5 -3.62 re f");
    assert!(
        drawn.contains("Fill"),
        "the clause's own real forms make a rectangle: {drawn}"
    );
}

/// **The other end of the same boundary: a digit run that swallows an operator.**
///
/// Every test above is a run stating no digit. This one states a digit and then keeps going
/// into letters — `5f`, with no delimiter between them. §7.2.3 ends a token at a delimiter or
/// a white-space character and at nothing else, and `f` is neither: Table 2 lists the
/// delimiters and `f` is not among them, so `5f` is **one** token. It spells no number
/// (§7.3.3 wants digits and an optional sign and point, not a letter) and no operator, so
/// nothing is painted.
///
/// The distinction is visible rather than theoretical, which is why it is worth a test:
/// `hayro`'s issue 994 is a hand-built stream that ends `... re 1 0 0 rg 5f`, and the two
/// readings differ by a red square. A lexer that split the run at the last digit hands the
/// interpreter a `5` and an `f`, the fill operator runs, and the square appears. One that
/// respects §7.2.3's boundary paints nothing.
///
/// **This tree paints nothing, and says nothing** — the run is salvaged to the number 5 and
/// the letters are dropped, rather than surfacing as `Unsupported::Operator("5f")` the way
/// `.` does above. The ink is what the clause asks for; the silence is not, and it is not
/// fixed here because the same leniency is what reads `12pt` as 12 in the streams that need
/// it (ADR 0303 scoped its correction to digit-less runs deliberately). `doc/todo/53` carries
/// the residue. What this test pins is the half that decides the page.
#[test]
fn a_digit_run_that_swallows_an_operator_paints_nothing() {
    let drawn = draws_in_silence("1 0 0 rg 10 10 100 20 re f");
    assert!(drawn.contains("Fill"), "the delimited form fills: {drawn}");

    let interpretation = interpretation("1 0 0 rg 10 10 100 20 re 5f");
    assert!(
        !format!("{:?}", interpretation.display_list).contains("Fill"),
        "`5f` is one token under §7.2.3, so there is no `f` operator and no fill"
    );
}

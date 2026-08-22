//! ISO 32000-2 §14.8.3.3's content rectangle, derived from the marks a sequence made.
//!
//! §14.8.3.3 gives every block- and inline-level structure element two enclosing rectangles and
//! states where the first comes from:
//!
//! > The content rectangle shall be derived from the shape of the enclosed content and defines the
//! > bounds used for the layout of any included child elements.
//!
//! §14.8.5.4.5 states the derivation for the two cases that are marks rather than layout — a table
//! cell's rectangle is "determined from the bounding box of all graphics objects in the cell's
//! content", and an inline element holding "an illustration or table" the same. So the union of
//! what a §14.7.5.2 marked-content sequence painted is the standard's own construction, and
//! `Interpretation::marked` carries it per sequence.
//!
//! These are content streams rather than unit tests over the accumulator because the whole
//! question is whether the *interpreter* attributes each command to the sequence enclosing it:
//! the clip in force, a form `XObject`'s own matrix, and a sequence that encloses nothing are all
//! decided in the run loop and nowhere else. **Nothing here needs a font**, deliberately — the
//! population this answers for is the one that marks no text.

#![expect(
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    reason = "test code: a fixture that cannot exercise what the test is about is a failure, \
              and these offsets are within a fixture written in this file"
)]

use std::fmt::Write as _;

use pdf_syntax::Document;

/// A one-page fixture on a 200 × 100 media box, with one form `XObject` named `/Fm`.
///
/// The form draws the unit square, so a caller placing it with a `cm` states the rectangle it
/// expects directly and the test asserts the matrix rather than a shape.
fn fixture(content: &str) -> Vec<u8> {
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 100] \
         /Resources << /XObject << /Fm 5 0 R >> >> /Contents 4 0 R >>\nendobj\n\
         4 0 obj\n<< /Length {} >>\nstream\n{content}\nendstream\nendobj\n\
         5 0 obj\n<< /Type /XObject /Subtype /Form /BBox [0 0 1 1] /Length 26 >>\nstream\n\
         0 0 1 1 re f\nendstream\nendobj\n",
        content.len() + 1,
    );

    let mut out = String::from("%PDF-1.7\n");
    let mut offsets: std::collections::BTreeMap<usize, usize> = std::collections::BTreeMap::new();
    let mut cursor = out.len();
    for object in body.split_inclusive("endobj\n") {
        let number: usize = object
            .split_whitespace()
            .next()
            .and_then(|word| word.parse().ok())
            .expect("every object states its number");
        offsets.insert(number, cursor);
        cursor += object.len();
    }
    out.push_str(&body);
    let xref_at = out.len();
    let size = offsets.keys().copied().max().unwrap_or(0) + 1;
    let _ = write!(out, "xref\n0 {size}\n0000000000 65535 f \n");
    for number in 1..size {
        match offsets.get(&number) {
            Some(offset) => {
                let _ = writeln!(out, "{offset:010} 00000 n ");
            }
            None => {
                let _ = writeln!(out, "0000000000 65535 f ");
            }
        }
    }
    let _ = write!(
        out,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n"
    );
    out.into_bytes()
}

/// The rectangle the sequence with the given `/MCID` drew, in the display list's own space.
fn drawn(content: &str, mcid: i64) -> Option<[f32; 4]> {
    let document = Document::open(fixture(content)).expect("the fixture is a valid file");
    let pages = pdf_model::Pages::new(&document);
    let page = pages.get(0).expect("the fixture has one page");
    let interpreted = pdf_model::interpret(&document, &page);
    let span = interpreted
        .marked
        .iter()
        .find(|span| span.mcid == mcid)
        .expect("the sequence closed and was recorded");
    span.drawn
}

/// Compares two rectangles at the tolerance a `f32` transform leaves.
#[track_caller]
fn close(got: Option<[f32; 4]>, want: [f32; 4]) {
    let got = got.expect("the sequence marked the page, so it has a content rectangle");
    for (index, expected) in want.iter().enumerate() {
        assert!(
            (got[index] - expected).abs() < 1e-3,
            "edge {index}: {got:?} against {want:?}"
        );
    }
}

/// A sequence enclosing one fill is bounded by that fill.
///
/// The simplest form of §14.8.5.4.5's "bounding box of all graphics objects in the … content", and
/// the case a `Figure` holding one image is: the page draws no text at all, so nothing but this
/// could say where the element is.
#[test]
fn a_sequence_takes_the_bounds_of_the_one_shape_it_encloses() {
    close(
        drawn("/Figure << /MCID 0 >> BDC 20 30 40 25 re f EMC", 0),
        [20.0, 30.0, 60.0, 55.0],
    );
}

/// Two shapes union, and a shape outside the sequence is not in it.
///
/// The second half is what makes this an attribution rather than a page bound: an interpreter that
/// unioned everything it drew would pass the first assertion of the test above and fail this.
#[test]
fn a_sequence_unions_its_own_shapes_and_no_others() {
    close(
        drawn(
            "10 10 5 5 re f \
             /Figure << /MCID 0 >> BDC 20 30 40 25 re f 100 10 20 20 re f EMC \
             150 80 10 10 re f",
            0,
        ),
        [20.0, 10.0, 120.0, 55.0],
    );
}

/// A clip in force narrows the rectangle, because §14.8.5.4.3's is of *visible* content.
///
/// Table 379 states the bounding box as one enclosing "its visible content", and §8.5.4 makes the
/// clipping path "the boundary of the area to be painted" — so a fill reaching past its clip marks
/// nothing out there and a rectangle that counted it would point a magnifier at blank paper.
#[test]
fn a_clip_narrows_what_the_sequence_is_taken_to_have_marked() {
    close(
        drawn(
            "q 30 35 20 10 re W n /Figure << /MCID 0 >> BDC 20 30 40 25 re f EMC Q",
            0,
        ),
        [30.0, 35.0, 50.0, 45.0],
    );
}

/// A `Do` inside the sequence contributes the form's marks under the matrix in force.
///
/// §14.7.5.2's first way of incorporating a form `XObject`: a `Do` that paints one may itself be
/// part of a marked-content sequence, and the clause says the whole `XObject` is then part of the
/// element's content, as if it were inserted at the point of the `Do`. The form here draws the
/// unit square, so the rectangle is the `cm` and nothing else — which is also what makes this a
/// test of the *transform* rather than of the shape.
#[test]
fn a_form_xobject_painted_inside_the_sequence_is_part_of_its_content() {
    close(
        drawn(
            "/Figure << /MCID 0 >> BDC q 60 0 0 40 25 15 cm /Fm Do Q EMC",
            0,
        ),
        [25.0, 15.0, 85.0, 55.0],
    );
}

/// A sequence that encloses no operator has no rectangle, and says so.
///
/// The honest answer, and the one trap 5 is about: a producer that opened and closed a sequence
/// around nothing has drawn nothing anywhere, and a rectangle standing in for that would turn a
/// silence into a place. The span is still recorded — "this element's content marked nothing" and
/// "this element has no content" are different statements.
#[test]
fn a_sequence_that_marked_nothing_has_no_content_rectangle() {
    assert_eq!(
        drawn("/Figure << /MCID 0 >> BDC EMC", 0),
        None,
        "an empty sequence must not be given a place it has not got"
    );
}

/// Neither has one whose every mark a clip excluded.
///
/// Different cause, same answer, and it is the one an interpreter that ignored the clip would get
/// wrong in the loudest way: the fill is stated at a definite place and painted at none.
#[test]
fn a_sequence_clipped_away_entirely_has_no_content_rectangle() {
    assert_eq!(
        drawn(
            "q 150 80 10 10 re W n /Figure << /MCID 0 >> BDC 20 30 40 25 re f EMC Q",
            0,
        ),
        None,
        "a mark the clip excludes is painted nowhere, so it places nothing"
    );
}

/// A sequence's own marks are its own, and a second sequence's are the second's.
///
/// §14.7.5.4's identifiers index the parent tree per content stream, so two elements of one page
/// are told apart by nothing else; a union that leaked between them would place both on top of
/// each other and a screen reader would point at the wrong one half the time.
#[test]
fn two_sequences_on_one_page_keep_their_own_rectangles() {
    let content = "/Figure << /MCID 0 >> BDC 10 10 20 20 re f EMC \
                   /Figure << /MCID 1 >> BDC 120 60 30 20 re f EMC";
    close(drawn(content, 0), [10.0, 10.0, 30.0, 30.0]);
    close(drawn(content, 1), [120.0, 60.0, 150.0, 80.0]);
}

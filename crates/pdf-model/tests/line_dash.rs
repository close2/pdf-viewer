//! The line dash pattern, ISO 32000-2 §8.4.3.6.
//!
//! # Why this file exists
//!
//! No dashed line in any document was ever dashed. Both backends implement dashing —
//! `render-cpu` hands a `StrokeDash` to `tiny-skia` and `render-gpu` gives Vello a dash
//! iterator — and `pdf-render`'s `Stroke` has carried a dash array and phase from the start.
//! What was missing was the middle: the `d` operator's arguments never reached the graphics
//! state, because the content lexer hands an operator its array *flattened* and the code
//! read only the case where nothing was between the brackets.
//!
//! Nothing reported it, and no metric could: a solid line where a dashed one belongs is a
//! plausible page. It was found by a Type 3 fixture whose glyphs were drawn dashed by three
//! reference renderers and solid by us, and it was worth one corpus page outright —
//! `close-path-bug.pdf`, which draws the specification's own example figure and had been on
//! the oracle's unexplained-contradiction list.
//!
//! These tests assert against the display list rather than pixels, because a dash pattern is
//! a set of numbers and a rasterised line answers only whether some pixel is dark.

#![expect(
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    reason = "test code: a malformed fixture should fail loudly, and the one length computed \
              here is a fixed string's"
)]

use std::fmt::Write as _;

use pdf_render::Command;
use pdf_syntax::Document;

/// Builds a one-page fixture whose content stream is `content`, with one `/ExtGState`.
///
/// The graphics state dictionary is always present so that the `/D` entry can be exercised
/// by naming it; a stream that never says `gs` is unaffected by its being there.
fn fixture(content: &str, ext_gstate: &str) -> Vec<u8> {
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] \
         /Resources << /ExtGState << /GS << {ext_gstate} >> >> >> /Contents 4 0 R >>\n\
         endobj\n\
         4 0 obj\n<< /Length {} >>\nstream\n{content}\nendstream\nendobj\n",
        content.len() + 1
    );

    let mut out = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for object in body.split_inclusive("endobj\n") {
        offsets.push(out.len());
        out.push_str(object);
    }
    let xref_at = out.len();
    let size = offsets.len() + 1;
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

/// The dash array and phase of every stroke the content stream produces.
fn dashes(content: &str, ext_gstate: &str) -> Vec<(Vec<f32>, f32)> {
    let document =
        Document::open(fixture(content, ext_gstate)).expect("the fixture is a valid PDF");
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    pdf_model::interpret(&document, &page)
        .display_list
        .commands()
        .iter()
        .filter_map(|command| match command {
            Command::Stroke { stroke, .. } => Some((stroke.dash_array.clone(), stroke.dash_phase)),
            _ => None,
        })
        .collect()
}

/// One stroked line, after whatever `setup` puts in front of it.
fn one_line(setup: &str) -> (Vec<f32>, f32) {
    let content = format!("{setup} 10 10 m 90 10 l S");
    let mut strokes = dashes(&content, "");
    assert_eq!(strokes.len(), 1, "the fixture strokes exactly one line");
    strokes.remove(0)
}

/// A dash array reaches the graphics state at all.
///
/// This is the defect: `[4 6] 0 d` is the specification's own example in Table 55 ("3 units
/// on, 3 units off" is the neighbouring row) and `close-path-bug.pdf` writes exactly it.
#[test]
fn a_dash_array_reaches_the_stroke() {
    assert_eq!(one_line("[4 6] 0 d"), (vec![4.0, 6.0], 0.0));
}

/// An empty array is a solid line, and clears whatever came before it.
///
/// §8.4.3.6: "If the dash array is empty, the dash phase shall be zero and the path shall be
/// stroked with a solid, unbroken line."
#[test]
fn an_empty_array_is_a_solid_line() {
    assert_eq!(one_line("[4 6] 2 d [] 0 d"), (Vec::new(), 0.0));
}

/// An odd-length array states the same alternation as itself repeated.
///
/// Table 55: "[3] 0" is "3 units on, 3 units off, …", which is what `[3 3]` says with an
/// even length. The doubling happens here rather than in each backend so that both receive
/// one meaning; a rasteriser's dash primitive takes pairs.
#[test]
fn an_odd_length_array_is_stated_as_pairs() {
    assert_eq!(one_line("[3] 0 d"), (vec![3.0, 3.0], 0.0));
    assert_eq!(
        one_line("[2 1 3] 0 d"),
        (vec![2.0, 1.0, 3.0, 2.0, 1.0, 3.0], 0.0)
    );
}

/// A negative phase is brought into range by the clause's own arithmetic.
///
/// §8.4.3.6: "If the dash phase is negative, it shall be incremented by twice the sum of all
/// lengths in the dash array until it is positive." Table 55's last row is `[2 1 3] -2`, for
/// which twice the sum is 12, so the phase is 10.
#[test]
fn a_negative_phase_is_incremented_by_twice_the_arrays_sum() {
    let (array, phase) = one_line("[2 1 3] -2 d");
    assert_eq!(array, vec![2.0, 1.0, 3.0, 2.0, 1.0, 3.0]);
    assert!((phase - 10.0).abs() < 1e-6, "phase came out {phase}");
}

/// A degenerate array describes no pattern, and is drawn solid.
///
/// §8.4.3.6 requires the elements to be "nonnegative and not all zero". A file breaking that
/// has stated nothing to draw, so the line is solid rather than left carrying whatever the
/// previous `d` set — which would make one malformed operator change the appearance of every
/// stroke after it.
#[test]
fn a_degenerate_array_is_drawn_solid() {
    assert_eq!(one_line("[4 6] 0 d [0 0] 0 d"), (Vec::new(), 0.0));
    assert_eq!(one_line("[4 6] 0 d [-2 3] 0 d"), (Vec::new(), 0.0));
}

/// An `/ExtGState`'s `/D` sets the same pattern the `d` operator does.
///
/// Table 57 defines `/D` as "the line dash pattern, expressed as an array of the form
/// [ dashArray dashPhase ]" — the same state, written as a real array rather than as
/// flattened operands, which is why one function decides what both mean.
#[test]
fn an_ext_gstate_sets_the_dash_pattern_too() {
    let strokes = dashes("/GS gs 10 10 m 90 10 l S", "/D [[5 2] 1]");
    assert_eq!(strokes, vec![(vec![5.0, 2.0], 1.0)]);
}

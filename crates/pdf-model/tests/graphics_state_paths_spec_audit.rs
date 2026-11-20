//! Audit 2155 of ISO 32000-2 §8.4.3.6 and Table 57 against the interpreter: a dash array has
//! a type as well as a place.
//!
//! The `d` operator's array and Table 57's `/D` entry are read into one pattern by
//! `content.rs`'s `apply_dash`, which states what it does with an array that breaks the clause's
//! range — a negative or an all-zero one "describes no pattern at all" and is drawn solid. An
//! element of the wrong *type* breaks the same sentence, and the test below asks that it is not
//! read as a different, valid pattern.

#![expect(
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    reason = "test fixtures are deliberately small; malformed fixtures should fail loudly"
)]

use std::fmt::Write as _;

use pdf_render::Command;
use pdf_syntax::Document;

/// A one-page fixture whose content stream is `content`, with one `/ExtGState` named `/GS`.
fn fixture(content: &str, ext_gstate: &str) -> Vec<u8> {
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] \
         /Resources << /ExtGState << /GS << {ext_gstate} >> >> >> /Contents 4 0 R >>\nendobj\n\
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
    for offset in offsets {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    let _ = write!(
        out,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n"
    );
    out.into_bytes()
}

/// The dash array of the one stroke `content` paints.
fn dash_of(content: &str, ext_gstate: &str) -> Vec<f32> {
    let document = Document::open(fixture(content, ext_gstate)).expect("fixture opens");
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    let strokes: Vec<Vec<f32>> = pdf_model::interpret(&document, &page)
        .display_list
        .commands()
        .iter()
        .filter_map(|command| match command {
            Command::Stroke { stroke, .. } => Some(stroke.dash_array.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(strokes.len(), 1, "the fixture strokes one path");
    strokes.into_iter().next().expect("one stroke")
}

/// §8.4.3.6: "The dash array's elements shall be numbers that specify the lengths of alternating
/// dashes and gaps"; Table 57 `/D`: "dashArray shall be itself an array".
///
/// `[3 (x) 1]` is not a dash array: its second element is a string. Dropping the string reads it
/// as `[3 1]` — a valid pattern the file never stated, in which the `1` that was the third length
/// (a dash) has become the second (a gap). Whether such an array is drawn solid, as
/// `apply_dash` draws an array that breaks the clause's range, or leaves the previous pattern in
/// force, it is not `[3 1]`.
#[test]
fn a_dash_array_with_a_non_number_is_not_read_as_another_pattern_spec_audit() {
    // Control: the pattern the file does state reaches the stroke through both routes.
    assert_eq!(dash_of("[3 1] 0 d 10 50 m 90 50 l S", ""), [3.0, 1.0]);
    assert_eq!(
        dash_of("/GS gs 10 50 m 90 50 l S", "/D [[3 1] 0]"),
        [3.0, 1.0]
    );

    assert_ne!(
        dash_of("[3 (x) 1] 0 d 10 50 m 90 50 l S", ""),
        [3.0, 1.0],
        "the d operator reads [3 (x) 1] as the pattern [3 1]"
    );
    assert_ne!(
        dash_of("/GS gs 10 50 m 90 50 l S", "/D [[3 (x) 1] 0]"),
        [3.0, 1.0],
        "Table 57's /D reads [3 (x) 1] as the pattern [3 1]"
    );
}

//! ISO 32000-2 §8.4.3 / §8.4.5: an omitted `ExtGState` key leaves the current parameter alone.
//!
//! Table 51's initial values apply when a graphics state starts. A later parameter dictionary
//! changes only the keys it contains; absent `/LC`, `/LJ`, or `/ML` entries must not reset a
//! value previously set by an operator.

#![expect(
    clippy::arithmetic_side_effects,
    clippy::float_cmp,
    reason = "test code: malformed fixtures should fail loudly and the exact parsed miter
              limit is carried unchanged from its literal"
)]

use std::fmt::Write as _;

use pdf_render::{Command, LineCap, LineJoin};
use pdf_syntax::Document;

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

#[test]
fn extgstate_only_changes_the_line_keys_it_contains() {
    let document = Document::open(fixture("1 J 2 j 3 M /GS gs 10 10 m 90 10 l S", "/LW 4"))
        .expect("the fixture is a valid PDF");
    assert!(!document.was_recovered(), "fixture needs no recovery");
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    let interpretation = pdf_model::interpret(&document, &page);
    assert!(
        interpretation.is_complete(),
        "{:?}",
        interpretation.unsupported
    );
    let list = interpretation.display_list;
    let stroke = list
        .commands()
        .iter()
        .find_map(|command| match command {
            Command::Stroke { stroke, .. } => Some(stroke),
            _ => None,
        })
        .expect("one stroke");

    assert_eq!(stroke.cap, LineCap::Round, "missing /LC preserves `1 J`");
    assert_eq!(stroke.join, LineJoin::Bevel, "missing /LJ preserves `2 j`");
    assert_eq!(stroke.miter_limit, 3.0, "missing /ML preserves `3 M`");
    assert_eq!(stroke.width, 4.0, "the stated /LW entry is applied");
}

#[test]
fn the_initial_line_parameters_match_table_51() {
    let document =
        Document::open(fixture("10 10 m 90 10 l S", "")).expect("the fixture is a valid PDF");
    assert!(!document.was_recovered(), "fixture needs no recovery");
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    let interpretation = pdf_model::interpret(&document, &page);
    assert!(
        interpretation.is_complete(),
        "{:?}",
        interpretation.unsupported
    );
    let list = interpretation.display_list;
    let stroke = list
        .commands()
        .iter()
        .find_map(|command| match command {
            Command::Stroke { stroke, .. } => Some(stroke),
            _ => None,
        })
        .expect("one stroke");

    assert_eq!(stroke.width, 1.0);
    assert_eq!(stroke.cap, LineCap::Butt);
    assert_eq!(stroke.join, LineJoin::Miter);
    assert_eq!(stroke.miter_limit, 10.0);
    assert!(stroke.dash_array.is_empty());
    assert_eq!(stroke.dash_phase, 0.0);
}

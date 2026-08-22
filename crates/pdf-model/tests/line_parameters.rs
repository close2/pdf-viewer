//! The stroke's shape parameters, ISO 32000-2 §8.4.3.3 to §8.4.3.5, by both routes.
//!
//! # Why this file exists
//!
//! §8.4.1's NOTE 1 says a graphics state parameter may be set "with specific PDF operators"
//! or "by including a particular entry in a graphics state parameter dictionary", and several
//! "either way". The line cap, line join and miter limit are three of those, and only the
//! operator half existed: `J`, `j` and `M` reached the stroke, and Table 57's `/LC`, `/LJ`
//! and `/ML` reached nothing at all.
//!
//! Three corpus documents set all three that way — `issue16287.pdf`, `issue7878.pdf` and
//! `extgstate.pdf` — and nothing reported it, because the *operator* was implemented and the
//! feature therefore looked finished. That is the shape the `d` operator's own defect had
//! (`line_dash.rs`), one level up, and it is why a clause family is the unit of review: this
//! was found by reading §8.4.3 through, not by any page looking wrong.

#![expect(
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    clippy::float_cmp,
    reason = "test code: a malformed fixture should fail loudly, the one length computed \
              here is a fixed string's, and the miter limits and line widths compared are \
              values a clause states, carried from a decimal literal in the fixture to the \
              same literal here with no arithmetic between them"
)]

use std::fmt::Write as _;

use pdf_render::{Command, LineCap, LineJoin};
use pdf_syntax::Document;

/// Builds a one-page fixture whose content stream is `content`, with one `/ExtGState`.
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

/// The cap, join and miter limit of the one stroke the fixture paints.
fn parameters(setup: &str, ext_gstate: &str) -> (LineCap, LineJoin, f32) {
    let content = format!("{setup} 10 10 m 90 10 l S");
    let document =
        Document::open(fixture(&content, ext_gstate)).expect("the fixture is a valid PDF");
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    let list = pdf_model::interpret(&document, &page).display_list;
    let mut found: Vec<(LineCap, LineJoin, f32)> = list
        .commands()
        .iter()
        .filter_map(|command| match command {
            Command::Stroke { stroke, .. } => Some((stroke.cap, stroke.join, stroke.miter_limit)),
            _ => None,
        })
        .collect();
    assert_eq!(found.len(), 1, "the fixture strokes exactly one line");
    found.remove(0)
}

/// The line width of the one stroke the fixture paints, in the path's own space.
///
/// Separate from [`parameters`] because it is a different clause's question: the cap, join and
/// limit are §8.4.3.3 to §8.4.3.5, and this is §8.4.1's clipping rule reaching §8.4.3.2's
/// parameter.
fn width(setup: &str, ext_gstate: &str) -> f32 {
    let content = format!("{setup} 10 10 m 90 10 l S");
    let document =
        Document::open(fixture(&content, ext_gstate)).expect("the fixture is a valid PDF");
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    let list = pdf_model::interpret(&document, &page).display_list;
    let mut found: Vec<f32> = list
        .commands()
        .iter()
        .filter_map(|command| match command {
            Command::Stroke { stroke, .. } => Some(stroke.width),
            _ => None,
        })
        .collect();
    assert_eq!(found.len(), 1, "the fixture strokes exactly one line");
    found.remove(0)
}

/// Table 51's initial values: butt caps, mitered joins, a limit of 10.
#[test]
fn the_initial_values_are_table_51s() {
    assert_eq!(
        parameters("", ""),
        (LineCap::Butt, LineJoin::Miter, 10.0),
        "Table 51 gives 0 for the cap and the join and 10.0 for the miter limit"
    );
}

/// The operators set all three: Table 53's and Table 54's codes, and `M`'s number.
#[test]
fn the_operators_set_all_three() {
    assert_eq!(
        parameters("1 J 2 j 3 M", ""),
        (LineCap::Round, LineJoin::Bevel, 3.0)
    );
    assert_eq!(
        parameters("2 J 1 j", ""),
        (LineCap::Square, LineJoin::Round, 10.0)
    );
}

/// Table 57's `/LC`, `/LJ` and `/ML` set the same three, which is the defect this file exists
/// for: before §8.4.3 was read as a family, every one of these was dropped.
#[test]
fn an_ext_gstate_sets_all_three_too() {
    assert_eq!(
        parameters("/GS gs", "/LC 1 /LJ 2 /ML 3"),
        (LineCap::Round, LineJoin::Bevel, 3.0)
    );
}

/// A `gs` and an operator set one parameter, so the later of the two wins.
///
/// §8.4.5: "The results of gs shall be cumulative; parameter values established in previous
/// invocations persist until explicitly overridden."
#[test]
fn the_later_of_the_two_routes_wins() {
    assert_eq!(
        parameters("2 J /GS gs", "/LC 1"),
        (LineCap::Round, LineJoin::Miter, 10.0),
        "the gs comes second"
    );
    assert_eq!(
        parameters("/GS gs 2 J", "/LC 1"),
        (LineCap::Square, LineJoin::Miter, 10.0),
        "the operator comes second"
    );
}

/// A code outside Table 53's or Table 54's three is the initial value, by either route.
///
/// §8.4.1 requires parameters to "be of the correct type or have values that fall within a
/// certain range", and states no other recovery.
#[test]
fn a_code_outside_the_clauses_three_is_the_initial_value() {
    assert_eq!(
        parameters("7 J -1 j", ""),
        (LineCap::Butt, LineJoin::Miter, 10.0)
    );
    assert_eq!(
        parameters("/GS gs", "/LC 7 /LJ -1"),
        (LineCap::Butt, LineJoin::Miter, 10.0)
    );
}

/// A miter limit below 1 is clipped to it, which §8.4.1 asks for.
///
/// The ratio §8.4.3.5 bounds is `1 / sin(φ/2)`, which is never below one, so a smaller limit
/// describes no angle at all — it would bevel even a straight join.
#[test]
fn a_miter_limit_below_one_is_clipped_into_range() {
    assert_eq!(parameters("0.5 M", "").2, 1.0);
    assert_eq!(parameters("/GS gs", "/ML 0.5").2, 1.0);
}

/// A negative line width is clipped to zero, by either route, which §8.4.1 requires.
///
/// The clause names this parameter in the sentence that requires it — "[p]arameters that are
/// numeric values, such as the current colour, line width, and miter limit, shall be clipped
/// into valid range, if necessary" — and §8.4.3.2 gives the range: "[i]t shall be a nonnegative
/// number expressed in user space units".
///
/// The width kept here is the *clipped* one rather than the device-pixel minimum §8.4.3.2 then
/// substitutes for zero, because the same bullet forbids storing a device adjustment back into
/// the graphics state. `Stroke::device_width` is where that minimum is applied, and
/// `render-cpu`'s `stroke_width.rs` is where it is asserted.
///
/// `issue19633.pdf` is the corpus's only witness, and `oracle.rs`'s
/// `CONTRADICTED_NEGATIVE_LINE_WIDTH` has the ladder that says two references stroke the
/// magnitude instead.
#[test]
fn a_negative_line_width_is_clipped_into_range() {
    assert_eq!(width("-0.1 w", ""), 0.0);
    assert_eq!(width("/GS gs", "/LW -0.1"), 0.0);
    assert_eq!(width("-1000 w", ""), 0.0);
    assert_eq!(
        width("0.4 w", ""),
        0.4,
        "a width inside the range is untouched"
    );
}

/// `q` and `Q` save and restore all three, §8.4.2.
#[test]
fn the_stack_saves_and_restores_them() {
    assert_eq!(
        parameters("1 J 2 j 3 M q 0 J 0 j 10 M Q", ""),
        (LineCap::Round, LineJoin::Bevel, 3.0)
    );
}

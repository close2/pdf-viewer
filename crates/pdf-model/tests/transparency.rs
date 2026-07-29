//! The constant shape and opacity of ISO 32000-2 §11.6.4.
//!
//! # Why this file exists
//!
//! `ca` and `CA` have been implemented since the first graphics state and nothing isolated
//! them: they reached the page through the corpus and the oracle, which is coverage of a
//! kind and cannot say *which* of the two a wrong page swapped. Reading §11.6.4 as a family
//! in the fourteenth session — for §11.6.4.3, which decides the precedence between an image's
//! `/SMask` and its `/Mask` — is what found the gap, and it is the ledger's usual yield: not
//! a defect, but a rule everybody believes with nothing pinning it.
//!
//! The clause the tests below quote is short and the whole of it is here. What is *not*
//! implemented is named in the ledger rather than tested: `/AIS`, which decides whether the
//! two constants are shape or opacity values, and the sentence that gives a transparency
//! group's result the non-stroking constant — both of which need §11.4.6's groups before they
//! can show on a page.

#![expect(
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    reason = "test code: a malformed fixture should fail loudly, and this page is 100 units \
              square where no arithmetic can overflow"
)]

use std::fmt::Write as _;

use pdf_render::{Command, Paint};
use pdf_syntax::Document;

/// A one-page fixture with one `/ExtGState`, drawing whatever `content` says.
fn fixture(gs: &str, content: &str) -> Vec<u8> {
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] \
         /Resources << /ExtGState << /GS << {gs} >> >> >> /Contents 4 0 R >>\nendobj\n\
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

/// The alpha of the first fill and the first stroke a content stream produces.
fn alphas(gs: &str, content: &str) -> (Option<f32>, Option<f32>) {
    let document = Document::open(fixture(gs, content)).expect("the fixture is a valid PDF");
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    let interpretation = pdf_model::interpret(&document, &page);
    let mut fill = None;
    let mut stroke = None;
    for command in interpretation.display_list.commands() {
        let (slot, paint) = match command {
            Command::Fill { paint, .. } => (&mut fill, paint),
            Command::Stroke { paint, .. } => (&mut stroke, paint),
            _ => continue,
        };
        if let Paint::Solid(colour) = paint
            && slot.is_none()
        {
            *slot = Some(colour.a);
        }
    }
    (fill, stroke)
}

/// §11.6.4.4: `ca` and `CA` are two constants, and each reaches its own kind of painting.
///
/// > The current alpha constant parameter in the graphics state (see 8.4, "Graphics state")
/// > shall be two scalar values -one for strokes and one for all other painting operations
/// > -to be used for the constant shape ( f k ) or constant opacity ( qk ) component in the
/// > colour compositing formulas.
///
/// The two values differ, and neither is a half, so a fixture that swapped them or applied
/// one to both fails. The clause's own sentence about which entry sets which — "The stroking
/// and nonstroking alpha constants shall be set, respectively, by the CA and ca entries" —
/// is the part that is easy to get backwards and impossible to see on a page, because a
/// page drawn with the two exchanged looks like a page drawn with different constants.
#[test]
fn the_two_alpha_constants_reach_stroking_and_non_stroking_paint_separately() {
    let (fill, stroke) = alphas(
        "/ca 0.25 /CA 0.75",
        "/GS gs 0 0 1 rg 1 0 0 RG 10 10 50 50 re f 10 10 50 50 re S",
    );

    assert_eq!(fill, Some(0.25), "ca is the non-stroking constant");
    assert_eq!(stroke, Some(0.75), "CA is the stroking constant");
}

/// §11.6.4.2: an elementary object's intrinsic opacity is 1.0, so opacity comes from `ca`.
///
/// > All elementary objects shall have an intrinsic opacity q j of 1.0 everywhere. Any
/// > desired opacity less than 1.0 shall be applied by means of an opacity mask or constant
///
/// Which is a statement about where alpha may *not* come from: a fill drawn without a `gs`
/// is opaque whatever else the state holds. Worth pinning because the natural mistake is the
/// reverse of the one above — carrying an alpha from the colour operands, which `rg` and `g`
/// have none of, or leaving a previous `gs` in force after a `Q`.
#[test]
fn an_object_drawn_without_a_constant_is_opaque() {
    let (fill, _) = alphas("/ca 0.25", "0 0 1 rg 10 10 50 50 re f");
    assert_eq!(fill, Some(1.0), "an unused /ExtGState changes nothing");

    let (restored, _) = alphas(
        "/ca 0.25",
        "q /GS gs 0 0 1 rg 10 10 50 50 re f Q 0 0 1 rg 10 10 20 20 re f",
    );
    assert_eq!(
        restored,
        Some(0.25),
        "the first fill is still the masked one"
    );
}

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
    fixture_with(gs, "", content)
}

/// The same, with `extra` added to the page's resource dictionary.
fn fixture_with(gs: &str, extra: &str, content: &str) -> Vec<u8> {
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] \
         /Resources << /ExtGState << /GS << {gs} >> >> {extra} >> /Contents 4 0 R >>\nendobj\n\
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

/// §11.6.4.4's constant reaches a shading too, which is the one paint it can be dropped from.
///
/// A shading replaces the current colour rather than tinting it, so the natural implementation
/// returns the shading and loses the alpha with the colour it did not use. That is what this
/// tree did until the fifteenth session, and `alphatrans.pdf` — a page that states
/// `Gradient: .5` on itself — was contradicted by all three references for it, its gradient
/// painted opaque over the three objects it should have shown through.
///
/// `sh` is the shorter of the two ways to get a shading onto the page and the same paint
/// reaches a shading *pattern*; §11.6.7 puts them under one sentence, since a shading pattern
/// "composites with its backdrop as if the shading dictionary were applied with the sh
/// operator".
#[test]
fn a_shading_carries_the_non_stroking_constant() {
    let shading = "/Shading << /Sh << /ShadingType 2 /ColorSpace /DeviceRGB \
                   /Coords [0 0 100 0] /Function << /FunctionType 2 /Domain [0 1] \
                   /C0 [1 0 0] /C1 [0 0 1] /N 1 >> >> >>";
    let document = Document::open(fixture_with("/ca 0.25", shading, "/GS gs /Sh sh"))
        .expect("the fixture is a valid PDF");
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    let interpretation = pdf_model::interpret(&document, &page);

    let mut alphas = Vec::new();
    for command in interpretation.display_list.commands() {
        if let Command::Fill {
            paint: Paint::Shading(shading),
            ..
        } = command
            && let pdf_render::ShadingKind::Axial { ramp, .. } = shading.kind.as_ref()
        {
            alphas.extend(ramp.stops.iter().map(|stop| stop.colour.a));
        }
    }

    assert!(!alphas.is_empty(), "the `sh` should have painted a shading");
    assert!(
        alphas.iter().all(|alpha| (alpha - 0.25).abs() < 1e-6),
        "every colour of the ramp carries the constant, not just some: {alphas:?}"
    );
}

/// §11.6.3: an array of blend mode names is read for the first one this reader *knows*.
///
/// > If encountered, a PDF processor shall use the first blend mode in the array that it
/// > recognizes (or Normal if it recognizes none of them).
///
/// The natural implementation takes the first name and maps it, which maps an unrecognised
/// leading name to Normal and never looks at the rest — indistinguishable from a correct
/// reader on every array whose first entry is a real mode, which is every array anyone
/// writes. Found by reading the clause rather than by a page.
#[test]
fn a_blend_mode_array_takes_the_first_name_this_reader_knows() {
    let blends = |gs: &str| {
        let document = Document::open(fixture(gs, "/GS gs 0 0 1 rg 10 10 50 50 re f"))
            .expect("the fixture is a valid PDF");
        let page = pdf_model::Pages::new(&document).get(0).expect("page one");
        pdf_model::interpret(&document, &page)
            .display_list
            .commands()
            .iter()
            .find_map(|command| match command {
                Command::Fill { blend, .. } => Some(*blend),
                _ => None,
            })
    };

    assert_eq!(
        blends("/BM [/Fictional /Multiply]"),
        Some(pdf_render::BlendMode::Multiply),
        "the first recognised name wins, not the first name"
    );
    assert_eq!(
        blends("/BM [/Fictional /AlsoFictional]"),
        Some(pdf_render::BlendMode::Normal),
        "an array of names none of which is a mode is Normal"
    );
}

/// §11.6.2: a path filled *and* stroked is one object, on the pages where that shows.
///
/// > Portions of an object shall not be composited with one another, even if they are
/// > described in a way that would seem to cause overlaps (such as a self-intersecting path,
/// > combined fill and stroke of a path, or a shading pattern containing an overlap or
/// > fold-over).
///
/// `B` is one object; a display list holding a `Fill` and a `Stroke` composites the band they
/// share twice. Since the seventy-second session the pair becomes one knockout group
/// (§11.4.6), which is the clause's own construction for "not composited with one another" —
/// and the *condition* is what is pinned here, because building a group for every `B` in
/// every document would cost a buffer per path for a difference almost none of them can show:
/// an opaque `B` paints the same either way, and so does one whose fill or stroke paints
/// nothing at all. `tests/transparency_groups.rs` has the pixel this changes.
#[test]
fn a_filled_and_stroked_path_is_one_object_only_where_it_can_show() {
    let grouped = |gs: &str, content: &str| {
        let document = Document::open(fixture(gs, content)).expect("the fixture is a valid PDF");
        let page = pdf_model::Pages::new(&document).get(0).expect("page one");
        pdf_model::interpret(&document, &page)
            .display_list
            .commands()
            .iter()
            .any(|command| matches!(command, Command::Group { knockout: true, .. }))
    };

    assert!(
        grouped(
            "/ca 0.5 /CA 0.5",
            "/GS gs 0 0 1 rg 1 0 0 RG 10 10 50 50 re B"
        ),
        "a compositing B would paint its stroke over its own fill"
    );
    assert!(
        !grouped("/ca 1 /CA 1", "/GS gs 0 0 1 rg 1 0 0 RG 10 10 50 50 re B"),
        "an opaque B under the Normal blend mode draws the same either way"
    );
    assert!(
        !grouped("/ca 0 /CA 0.5", "/GS gs 0 0 1 rg 1 0 0 RG 10 10 50 50 re B"),
        "a fill that paints nothing leaves one part, which cannot overlap itself"
    );
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

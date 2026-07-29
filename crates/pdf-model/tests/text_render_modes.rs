//! Text rendering modes, ISO 32000-2 §9.3.6 Table 104, one rule at a time.
//!
//! # Why this file exists
//!
//! Table 104's eight modes are three operations — fill, stroke, add to the clipping path —
//! and for four sessions this tree implemented one of them. Mode 1 and mode 2 were drawn as
//! a plain fill in the *non-stroking* colour, so a page that outlines its display type came
//! out solid; modes 4 to 7 built no clip at all, so a rectangle painted afterwards to show
//! through the letters covered its whole area — `text_clip_cff_cid.pdf` drew a solid blue
//! bar where four renderers show the word "ABC123". Both were reported rather than silent,
//! which is the only reason they were schedulable.
//!
//! These assert against the *display list* rather than against pixels, because every rule
//! here is about which commands exist, in what order, with which paint and which clip — and
//! a rasterised page answers those only through what it happens to cover. The oracle covers
//! the other direction.
//!
//! # The one machine dependency, and why it is a panic rather than a skip
//!
//! A glyph outline has to come from somewhere, and the standard 14 fonts are not embedded
//! in any file — `pdf-font`'s `substitute` finds one installed on this machine. A machine
//! with no fonts at all would produce no outlines and every assertion below would pass
//! vacuously. So the helper checks that the substitute produced something and panics
//! naming the reason if it did not: a missing corpus is a skip, but a fixture that cannot
//! exercise what the test is about is a failure. The twelfth session shipped two tests that
//! quietly checked nothing for exactly this reason.

#![expect(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "test code: a malformed fixture should fail loudly, and these pages are 100 \
              units square where no index can overflow"
)]
#![expect(
    clippy::float_cmp,
    reason = "the one exact comparison here is a line width that must arrive unscaled: 3.0 \
              is exactly representable and any arithmetic on the way would show as a miss, \
              which is the whole point of the assertion"
)]

use std::fmt::Write as _;

use pdf_render::{Command, Paint};
use pdf_syntax::Document;

/// A one-page fixture carrying both a substituted simple font and a Type 3 font.
///
/// Both are always present so that one builder serves every test; a content stream that
/// never names `/FT3` is unaffected by its being there. The Type 3 glyph is §9.6.4's own
/// EXAMPLE square, which is the one glyph description whose intended appearance the
/// standard itself states.
fn fixture(content: &str) -> Vec<u8> {
    const SQUARE: &str = "1000 0 0 0 750 750 d1\n0 0 750 750 re f";
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] \
         /Resources << /Font << /F1 5 0 R /FT3 6 0 R >> >> /Contents 4 0 R >>\nendobj\n\
         4 0 obj\n<< /Length {} >>\nstream\n{content}\nendstream\nendobj\n\
         5 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>\nendobj\n\
         6 0 obj\n<< /Type /Font /Subtype /Type3 /FontBBox [0 0 750 750] \
         /FontMatrix [0.001 0 0 0.001 0 0] /CharProcs 8 0 R /Encoding 7 0 R \
         /FirstChar 97 /LastChar 97 /Widths [1000] >>\nendobj\n\
         7 0 obj\n<< /Type /Encoding /Differences [97 /square] >>\nendobj\n\
         8 0 obj\n<< /square 9 0 R >>\nendobj\n\
         9 0 obj\n<< /Length {} >>\nstream\n{SQUARE}\nendstream\nendobj\n",
        content.len() + 1,
        SQUARE.len() + 1,
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

/// Interprets a content stream against the fixture above.
fn interpret(content: &str) -> pdf_model::Interpretation {
    let document = Document::open(fixture(content)).expect("the fixture is a valid PDF");
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    pdf_model::interpret(&document, &page)
}

/// Interprets a content stream, insisting that the substituted font produced glyphs.
///
/// See the module comment: without an installed font every assertion about a glyph's
/// commands would hold vacuously. The caller's own stream cannot be the witness, because
/// several of these tests expect it to paint *nothing* — so a separate probe in mode 0,
/// whose answer is never nothing, is what decides whether a substitute was found.
fn page(content: &str) -> pdf_model::Interpretation {
    let probe = interpret("BT /F1 24 Tf 10 10 Td (A) Tj ET");
    assert!(
        !probe.display_list.commands().is_empty(),
        "no font on this machine substitutes for Helvetica, so these tests would all pass \
         without exercising anything; install a font or run this where one exists"
    );
    interpret(content)
}

/// The command kinds a display list holds, in painting order.
fn kinds(interpretation: &pdf_model::Interpretation) -> Vec<&'static str> {
    interpretation
        .display_list
        .commands()
        .iter()
        .map(|command| match command {
            Command::Fill { .. } => "fill",
            Command::Stroke { .. } => "stroke",
            Command::Image { .. } => "image",
            _ => "other",
        })
        .collect()
}

/// The clip in effect for the last command, which every clipping test asks about.
fn last_clip(interpretation: &pdf_model::Interpretation) -> Option<pdf_render::ClipId> {
    interpretation
        .display_list
        .commands()
        .last()
        .expect("the fixture paints something")
        .clip()
}

/// Mode 0 fills with the non-stroking colour and does not stroke.
#[test]
fn mode_0_fills_only() {
    let drawn = page("BT /F1 24 Tf 10 10 Td 0 Tr (A) Tj ET");
    assert_eq!(kinds(&drawn), ["fill"]);
}

/// Mode 1 strokes with the *stroking* colour and does not fill.
///
/// The failing case this pins is the one that shipped: mode 1 was approximated as a fill in
/// the non-stroking colour, so a page that outlines its display type came out solid, in the
/// wrong colour, and — because the file usually sets only the stroking colour — often black
/// where the producer asked for an outline.
#[test]
fn mode_1_strokes_only_and_in_the_stroking_colour() {
    let drawn = page("BT /F1 24 Tf 1 0 0 RG 0 0 1 rg 10 10 Td 1 Tr (A) Tj ET");
    assert_eq!(kinds(&drawn), ["stroke"]);

    let Some(Command::Stroke {
        paint: Paint::Solid(colour),
        ..
    }) = drawn.display_list.commands().first()
    else {
        panic!("mode 1 must produce one solid stroke");
    };
    assert_eq!(
        (colour.r, colour.g, colour.b),
        (1.0, 0.0, 0.0),
        "§9.3.6: \"if it calls for stroking, the current stroking colour shall be used\""
    );
}

/// Mode 2 fills *then* strokes, glyph by glyph rather than string by string.
///
/// §9.3.6: "If any of the glyphs overlap, the result shall be equivalent to filling and
/// stroking them one at a time, producing the appearance of stacked opaque glyphs, rather
/// than first filling and then stroking them all at once." Two glyphs therefore give
/// fill, stroke, fill, stroke — not fill, fill, stroke, stroke, which is what emitting the
/// two passes per *string* would produce and what a reader would not see in a rasterised
/// page unless the glyphs happened to overlap.
#[test]
fn mode_2_fills_then_strokes_each_glyph_in_turn() {
    let drawn = page("BT /F1 24 Tf 10 10 Td 2 Tr (AB) Tj ET");
    assert_eq!(kinds(&drawn), ["fill", "stroke", "fill", "stroke"]);
}

/// Mode 3 paints nothing at all, and still reads the text.
///
/// The second half matters more than the first: mode 3 is what a scanner's OCR layer uses
/// under the page image, and the text it carries is the only text the document has.
#[test]
fn mode_3_paints_nothing_and_still_extracts_its_text() {
    let drawn = page("BT /F1 24 Tf 10 10 Td 3 Tr (AB) Tj ET");
    assert!(kinds(&drawn).is_empty(), "mode 3 paints nothing");
    assert!(
        drawn.text.contains("AB"),
        "mode 3 text must still be read: {:?}",
        drawn.text
    );
}

/// A stroke's line width is in user space, not in the glyph's or the text object's.
///
/// §9.3.6: "The graphics state parameters affecting those operations, such as line width,
/// shall be interpreted in user space rather than in text space." A `Command::Stroke`'s
/// width is in its path's own space, so the glyph outline has to arrive in user space. The
/// number this pins is not arbitrary: leaving the outline in em units would have left a
/// width of 3 against a path one unit tall, outlining a 100-point glyph roughly a hundred
/// times too thickly.
#[test]
fn a_glyph_is_stroked_with_a_width_stated_in_user_space() {
    let drawn = page("BT /F1 100 Tf 3 w 10 10 Td 1 Tr (A) Tj ET");
    let Some(Command::Stroke {
        path,
        transform,
        stroke,
        ..
    }) = drawn.display_list.commands().first()
    else {
        panic!("mode 1 must produce one stroke");
    };

    assert_eq!(stroke.width, 3.0, "the width reaches the command unscaled");
    assert_eq!(
        *transform,
        pdf_render::Transform::IDENTITY,
        "this page has no rotation and its crop box is at the origin, so user space *is* \
         page space here — the glyph's own placement must be in the path"
    );

    // A 100-point capital is tens of units tall in user space and one unit tall in em
    // space; nothing between those two readings is possible, so a loose bound suffices to
    // tell them apart.
    let tallest = path
        .commands()
        .iter()
        .filter_map(|command| match *command {
            pdf_render::PathCommand::MoveTo(p)
            | pdf_render::PathCommand::LineTo(p)
            | pdf_render::PathCommand::CurveTo(_, _, p) => Some(p.y),
            pdf_render::PathCommand::Close => None,
        })
        .fold(f32::MIN, f32::max);
    assert!(
        tallest > 30.0,
        "the outline must be in user space, but its highest point is at y = {tallest}"
    );
}

/// Mode 7 adds the glyphs to the clipping path and paints nothing.
///
/// The rectangle after `ET` is the whole point: it is what the producer expects to see only
/// through the letters, and it is what we drew over the whole page for four sessions.
#[test]
fn mode_7_clips_what_follows_and_paints_nothing() {
    let drawn = page("BT /F1 48 Tf 10 10 Td 7 Tr (A) Tj ET 0 0 100 100 re f");
    assert_eq!(kinds(&drawn), ["fill"], "mode 7 itself paints nothing");

    let clip = last_clip(&drawn).expect("the rectangle after ET is clipped to the glyphs");
    let clip = drawn.display_list.clip(clip).expect("the clip exists");
    assert!(
        !clip.path.is_empty(),
        "the glyph outlines are the clip path"
    );
    assert_eq!(
        clip.fill_rule,
        pdf_render::FillRule::NonZero,
        "§9.3.6 applies \"the non-zero winding number rule\""
    );
    assert_eq!(
        clip.transform,
        pdf_render::Transform::IDENTITY,
        "each glyph carried its own transform, so they are baked into the path"
    );
    assert!(
        clip.parent.is_none(),
        "nothing clipped this page before the text object"
    );
}

/// Mode 4 fills *and* clips; the two are independent bits of the mode.
#[test]
fn mode_4_fills_and_clips() {
    let drawn = page("BT /F1 48 Tf 10 10 Td 4 Tr (A) Tj ET 0 0 100 100 re f");
    assert_eq!(kinds(&drawn), ["fill", "fill"]);
    assert!(
        drawn.display_list.commands()[0].clip().is_none(),
        "the glyph itself is painted before its own clip exists"
    );
    assert!(
        last_clip(&drawn).is_some(),
        "and the rectangle after ET is clipped by it"
    );
}

/// The clip appears at `ET`, not at the glyph.
///
/// §9.3.6: "As is the case for path objects, this clipping shall occur after all filling and
/// stroking operations for the text object have occurred." A rectangle painted before the
/// text object is unaffected, and mode 6's own fill and stroke are too — which the previous
/// test checks for the fill and this one leaves to it.
#[test]
fn the_clip_begins_at_et() {
    let drawn = page("0 0 10 10 re f BT /F1 48 Tf 10 10 Td 7 Tr (A) Tj ET 0 0 100 100 re f");
    let clips: Vec<bool> = drawn
        .display_list
        .commands()
        .iter()
        .map(|command| command.clip().is_some())
        .collect();
    assert_eq!(clips, [false, true]);
}

/// `Q` ends it, and nothing else does.
///
/// §9.3.6: "It remains in effect until a previous clipping path is restored by an invocation
/// of the Q operator." So the clip has to be set on the live graphics state — a text clip
/// applied to a copy would end at `ET` and leave the rectangle inside the `q`…`Q` unclipped.
#[test]
fn a_text_clip_survives_et_and_ends_at_q() {
    let drawn = page("q BT /F1 48 Tf 10 10 Td 7 Tr (A) Tj ET 0 0 100 100 re f Q 0 0 100 100 re f");
    let clips: Vec<bool> = drawn
        .display_list
        .commands()
        .iter()
        .map(|command| command.clip().is_some())
        .collect();
    assert_eq!(clips, [true, false]);
}

/// A clipping mode that shows no outlines clips nothing — it does not clip everything.
///
/// §9.3.6: "If no glyphs are shown or if the only glyphs shown have no outlines (for
/// example, if they are ASCII SPACE characters (20h)), no clipping shall occur." The
/// obvious implementation sets the clip to whatever accumulated, which for a blank line of
/// OCR text is an empty path — and an empty clip hides the rest of the page. Nothing but
/// pixels could see that, and by then it is a blank page rather than a subtle error.
#[test]
fn a_clipping_mode_showing_only_spaces_clips_nothing() {
    let drawn = page("BT /F1 48 Tf 10 10 Td 7 Tr (   ) Tj ET 0 0 100 100 re f");
    assert!(last_clip(&drawn).is_none());

    // The same rule for a text object that shows nothing at all.
    let empty = page("BT /F1 48 Tf 10 10 Td 7 Tr ET 0 0 100 100 re f");
    assert!(last_clip(&empty).is_none());
}

/// A Type 3 glyph is drawn in every mode but 3 and 7, and never joins the clipping path.
///
/// §9.3.6 states both as exceptions in so many words, and the second is not a shortcut we
/// are taking: a Type 3 glyph description is an arbitrary content stream with no outline to
/// contribute, so "nothing shall be added to the clipping path" is the only rule that could
/// have been written.
#[test]
fn a_type3_glyph_draws_in_mode_4_and_adds_nothing_to_the_clip() {
    let clipping = interpret("BT /FT3 48 Tf 10 10 Td 4 Tr (a) Tj ET 0 0 100 100 re f");
    assert_eq!(
        kinds(&clipping),
        ["fill", "fill"],
        "the square, then the page"
    );
    assert!(
        last_clip(&clipping).is_none(),
        "§9.3.6: \"If text rendering mode is set to a value of 4, 5, 6 or 7, nothing shall \
         be added to the clipping path\""
    );

    let invisible = interpret("BT /FT3 48 Tf 10 10 Td 7 Tr (a) Tj ET 0 0 100 100 re f");
    assert_eq!(
        kinds(&invisible),
        ["fill"],
        "§9.3.6: in mode 3 or 7 \"the text shall not be rendered\""
    );
    assert!(last_clip(&invisible).is_none());
}

/// A rendering mode Table 104 does not define is reported, and does not silently blank the
/// text object.
///
/// Three of the mode's operations are selected by matching against the eight defined
/// values, so an undefined one matches none of them and would draw nothing whatsoever —
/// a whole text object missing with `unsupported: []`, which is the failure mode this
/// project's third principle exists to prevent.
#[test]
fn an_undefined_rendering_mode_is_reported_rather_than_obeyed() {
    let drawn = page("BT /F1 24 Tf 10 10 Td 9 Tr (A) Tj ET");
    assert_eq!(kinds(&drawn), ["fill"], "the mode stays as it was");
    assert!(
        format!("{:?}", drawn.unsupported).contains("Tr with mode 9"),
        "an undefined mode must be named: {:?}",
        drawn.unsupported
    );
}

/// Nothing about a rendering mode is reported as unsupported any more.
///
/// Every one of Table 104's eight modes is implemented, so the report that stood in for
/// four of them is gone. This pins that: a report left behind after the feature lands is
/// how a corpus count comes to say a document is incomplete when it is not.
#[test]
fn no_defined_rendering_mode_reports_anything() {
    for mode in 0..=7 {
        let drawn = page(&format!(
            "BT /F1 24 Tf 1 0 0 RG 10 10 Td {mode} Tr (A) Tj ET"
        ));
        assert!(
            !format!("{:?}", drawn.unsupported).contains("render mode"),
            "mode {mode} reported {:?}",
            drawn.unsupported
        );
    }
}

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
//! # The machine dependency that used to be here
//!
//! A glyph outline has to come from somewhere, and the standard 14 fonts are not embedded in any
//! file. Until the hundred-and-forty-eighth session `pdf-font`'s `substitute` found one installed
//! on *this machine*, so a machine with no fonts at all would produce no outlines and every
//! assertion below would pass vacuously — which is why the helper panics rather than skipping: a
//! missing corpus is a skip, but a fixture that cannot exercise what the test is about is a
//! failure. The twelfth session shipped two tests that quietly checked nothing for exactly this
//! reason.
//!
//! §9.6.2.2's fourteen are compiled in now (`pdf_font::standard`, ADR 0133), so the dependency is
//! gone and the panic cannot fire for the reason it was written for. It stays, because what it
//! guards against — every assertion below holding vacuously — has not gone anywhere.

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
    // A form whose own `/Resources` defines no `/Font`, for `a_forms_font_resource_is_not_the_pages`.
    const FORM: &str = "BT /F1 24 Tf 50 50 Td (B) Tj ET";
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] \
         /Resources << /Font << /F1 5 0 R /FT3 6 0 R >> /XObject << /Fm 12 0 R >> \
         /ExtGState << /Half 10 0 R /HalfNoTk 11 0 R >> >> /Contents 4 0 R >>\nendobj\n\
         4 0 obj\n<< /Length {} >>\nstream\n{content}\nendstream\nendobj\n\
         5 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>\nendobj\n\
         6 0 obj\n<< /Type /Font /Subtype /Type3 /FontBBox [0 0 750 750] \
         /FontMatrix [0.001 0 0 0.001 0 0] /CharProcs 8 0 R /Encoding 7 0 R \
         /FirstChar 97 /LastChar 97 /Widths [1000] >>\nendobj\n\
         7 0 obj\n<< /Type /Encoding /Differences [97 /square] >>\nendobj\n\
         8 0 obj\n<< /square 9 0 R >>\nendobj\n\
         9 0 obj\n<< /Length {} >>\nstream\n{SQUARE}\nendstream\nendobj\n\
         10 0 obj\n<< /Type /ExtGState /ca 0.5 /CA 0.5 >>\nendobj\n\
         11 0 obj\n<< /Type /ExtGState /ca 0.5 /CA 0.5 /TK false >>\nendobj\n\
         12 0 obj\n<< /Type /XObject /Subtype /Form /BBox [0 0 100 100] \
         /Resources << >> /Length {} >>\nstream\n{FORM}\nendstream\nendobj\n",
        content.len() + 1,
        SQUARE.len() + 1,
        FORM.len() + 1,
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
/// Without a face for `/Helvetica` every assertion about a glyph's commands would hold
/// vacuously, and several of these tests expect the caller's own stream to paint *nothing* — so
/// a separate probe in mode 0, whose answer is never nothing, is what decides whether a
/// substitute was found.
///
/// **The failure message used to end "install a font or run this where one exists".** Since the
/// hundred-and-forty-eighth session §9.6.2.2's fourteen are compiled into the binary, so
/// `/Helvetica` resolves on a machine with no fonts at all and this probe cannot fail for that
/// reason. It stays because it is still the thing that would make every assertion below vacuous,
/// and a probe that can no longer fail for the reason it was written for is a probe that will
/// catch the next reason.
fn page(content: &str) -> pdf_model::Interpretation {
    let probe = interpret("BT /F1 24 Tf 10 10 Td (A) Tj ET");
    assert!(
        !probe.display_list.commands().is_empty(),
        "nothing substituted for /Helvetica, which §9.6.2.2 makes a font every processor has \
         and `pdf_font::standard` compiles in — so these tests would all pass without \
         exercising anything"
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

/// §11.7.4.4: a translucent mode-2 glyph is one object, not a fill and a stroke.
///
/// > the painting of glyphs with text rendering mode 2 or 6 … For transparency compositing
/// > purposes, the combined fill and stroke shall be treated as a single graphics object, as
/// > if they were enclosed in a transparency group.
///
/// The clause's second bullet is the one that applies here — the first needs overprinting,
/// which §8.6.7 says this device never enables — and it asks for "a non-isolated knockout
/// group", composited "using an alpha value of 1.0 and the Normal blend mode". NOTE 2 says
/// what goes wrong without it: "a non-opaque stroke composite with the result of the fill in
/// the region of overlap, which would produce a double border effect".
///
/// Opaque paint is the control, and it is the case `mode_2_fills_then_strokes_each_glyph_in_turn`
/// already pins: with alpha 1 under Normal the stroke covers the fill either way, so no group
/// is built and the two commands stay flat.
#[test]
fn a_translucent_mode_2_glyph_is_one_knockout_group() {
    let drawn = page("BT /F1 24 Tf /Half gs 10 10 Td 2 Tr (A) Tj ET");
    let commands = drawn.display_list.commands();
    let [
        Command::Group {
            commands: parts,
            alpha,
            clip,
            mask,
            blend,
            isolated,
            knockout,
        },
    ] = commands
    else {
        panic!("expected one group, got {:?}", kinds(&drawn));
    };
    assert!(*knockout, "the clause asks for a knockout group");
    assert!(
        *isolated,
        "§11.4.6 composites each element with the group's *initial* backdrop, and this \
         group's elements are the two portions of one glyph"
    );
    assert_eq!(*alpha, 1.0, "composited \"using an alpha value of 1.0\"");
    assert_eq!(*blend, pdf_render::BlendMode::Normal);
    assert!(clip.is_none() && mask.is_none());
    assert!(
        matches!(
            parts.as_slice(),
            [Command::Fill { .. }, Command::Stroke { .. }]
        ),
        "the fill and the stroke, in that order"
    );
}

/// And it does not depend on `Tk`, which §11.7.4.4 says in a note of its own.
///
/// > NOTE 1 In the case of showing text with the combined filling and stroking text rendering
/// > modes, this behaviour is independent of the text knockout parameter in the graphics state
///
/// This is the discriminating case for the way the two clauses were confused: §9.3.8's group
/// exists only where `Tk` is true and two glyphs overlap, and a reader that built §11.7.4.4's
/// group out of that machinery would draw nothing special here.
#[test]
fn the_implicit_group_does_not_depend_on_the_text_knockout_parameter() {
    let drawn = page("BT /F1 24 Tf /HalfNoTk gs 10 10 Td 2 Tr (A) Tj ET");
    assert!(
        matches!(
            drawn.display_list.commands(),
            [Command::Group { knockout: true, .. }]
        ),
        "got {:?}",
        kinds(&drawn)
    );
}

/// Each glyph gets its own group, because each glyph is its own object.
///
/// Two glyphs that do not overlap leave §9.3.8 with nothing to do, so what remains is one
/// implicit group per glyph rather than one around the pair — which is the difference between
/// reading §11.7.4.4 as being about a glyph and reading it as being about a show string.
#[test]
fn every_combined_glyph_is_its_own_object() {
    let drawn = page("BT /F1 24 Tf /Half gs 10 10 Td 2 Tr (AB) Tj ET");
    let commands = drawn.display_list.commands();
    assert_eq!(commands.len(), 2, "got {:?}", kinds(&drawn));
    for command in commands {
        assert!(matches!(command, Command::Group { knockout: true, .. }));
    }
}

/// Where §9.3.8's own group encloses the object, it subsumes every glyph's.
///
/// Overlapping glyphs under `Tk` make the whole text object a knockout group, and a knockout
/// group inside a knockout group is not something either backend can state. It does not have
/// to be: in a knockout group every element composites with the initial backdrop, so at each
/// point the topmost element wins, and nesting cannot change which element that is. The flat
/// group therefore computes both clauses at once — which this pins by asserting that the four
/// commands are its direct elements.
#[test]
fn the_text_objects_own_knockout_group_holds_the_glyphs_parts_flat() {
    // The second glyph is drawn on top of the first: same `Td`, no advance between them.
    let drawn = page("BT /F1 24 Tf /Half gs 10 10 Td 2 Tr (A) Tj 10 10 Td (B) Tj ET");
    let [
        Command::Group {
            commands: parts,
            knockout: true,
            ..
        },
    ] = drawn.display_list.commands()
    else {
        panic!("expected one group, got {:?}", kinds(&drawn));
    };
    assert_eq!(parts.len(), 4, "four flat elements, not two nested groups");
    assert!(
        parts
            .iter()
            .all(|part| !matches!(part, Command::Group { .. })),
        "no group inside the group"
    );
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

/// §14.9.4's `/ActualText` replaces the text a marked-content sequence reads back.
///
/// > The ActualText value shall be used as a replacement, not a description, for the content,
/// > providing text that is equivalent to what a person would see when viewing the content.
///
/// The fixture is `issue13226.pdf`'s shape: a space glyph whose `/ActualText` is a soft hyphen
/// (U+00AD), which is what a producer writes for §14.8.2.3's "visible hyphen that is introduced
/// through the incidental division of a word". Two things are asserted because two things are
/// true at once — the *marks* are unchanged, so the space is still drawn, and the *text* is the
/// replacement rather than the space.
///
/// The property list is inline, which is the form real documents use and the form the content
/// lexer could not assemble until the fifty-fifth session: a content stream yields tokens, so a
/// dictionary written inside one has to be put back together.
#[test]
fn actual_text_replaces_what_a_sequence_reads_back() {
    let page = page(
        "BT /F1 24 Tf 10 10 Td (Mit) Tj \
         /Span << /ActualText <FEFF00AD> >> BDC ( ) Tj EMC \
         (arbeiter) Tj ET",
    );
    assert_eq!(page.text.trim_end(), "Mit\u{ad}arbeiter");

    // Without the entry the same stream reads back the space it drew, which is the other half
    // of the claim: this is a statement about extraction and not about painting.
    let plain = page.text.len();
    let bare = self::page("BT /F1 24 Tf 10 10 Td (Mit) Tj ( ) Tj (arbeiter) Tj ET");
    assert_eq!(bare.text.trim_end(), "Mit arbeiter");
    assert!(plain > 0);
}

/// An inline property list's booleans are values, not operators.
///
/// `true`, `false` and `null` lex as keywords in a content stream, so a property list written
/// inline used to send them to the operator dispatch one token at a time — which is why two
/// corpus documents reported `true` and `false` as unknown operators. §7.3.2 makes them objects
/// wherever an object belongs, and inside `<< … >>` an object belongs.
#[test]
fn an_inline_property_lists_booleans_are_not_operators() {
    let page = page(
        "BT /F1 24 Tf 10 10 Td \
         /Span << /Flag true /Other false /Nothing null /ActualText (X) >> BDC (abc) Tj EMC ET",
    );
    assert_eq!(page.text.trim_end(), "X");
    assert!(
        page.unsupported.is_empty(),
        "nothing in that stream is unsupported: {:?}",
        page.unsupported
    );
}

/// A form `XObject`'s `/F1` is not the page's `/F1`.
///
/// §8.10.1 gives a form `XObject` a `/Resources` entry of its own, so a resource name is scoped
/// to the dictionary that defines it. The interpreter's font cache was keyed by the *name*, so a
/// form naming `/F1` was handed whatever `/F1` the page had loaded — with nothing reported,
/// which is trap 1's archetype and is what two corpus documents were doing in silence
/// (`issue17492.pdf`, `issue19182.pdf`). The cache is keyed by the font dictionary's object
/// identity now.
///
/// The fixture's form defines no `/Font` at all, so the only correct answer is a report. A
/// reader with the old cache draws the page's Helvetica and says nothing, and the second
/// assertion is what tells the two apart: the form's own text must not reach the page.
#[test]
fn a_forms_font_resource_is_not_the_pages() {
    let drawn = page("BT /F1 24 Tf 10 10 Td (A) Tj ET\n/Fm Do");
    let reports = format!("{:?}", drawn.unsupported);
    assert!(
        reports.contains("no /Font resource named /F1"),
        "the form defines no /F1 and must say so: {reports}"
    );
    assert_eq!(
        kinds(&drawn),
        ["fill"],
        "only the page's own glyph may be drawn"
    );
}

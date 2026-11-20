//! Audit 2161 of ISO 32000-2 §9.2 to §9.4 against the interpreter: what a text object owes at its
//! end, what an operand is besides its place, and what reaches the page from a text object that is
//! not the page's.
//!
//! Each test pairs a control that passes on main with the case the audit found. The fixture holds
//! `/Helvetica` (compiled in, ADR 0133), §9.6.4's EXAMPLE square as a Type 3 glyph, and a second
//! Type 3 font whose glyph description shows text of its own, which §9.4.1's NOTE allows: "text
//! can be shown using a Type 3 font whose glyph descriptions include any graphics objects,
//! including another text object".

#![expect(
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    reason = "test fixtures are deliberately small; malformed fixtures should fail loudly"
)]

use std::fmt::Write as _;

use pdf_render::Command;
use pdf_syntax::Document;

/// A one-page fixture: `/F1` Helvetica, `/FT3` a Type 3 square, `/FT3T` a Type 3 font whose one
/// glyph shows an `A` of its own in an invisible mode, both Type 3 glyphs 1000 units wide under a
/// `/FontMatrix` of 0.001, and `/Half` an `/ExtGState` with both alpha constants at 0.5.
fn fixture(content: &str) -> Vec<u8> {
    const SQUARE: &str = "1000 0 0 0 750 750 d1\n0 0 750 750 re f";
    const WITH_TEXT: &str = "1000 0 d0\n3 Tr BT /F1 500 Tf 100 100 Td (A) Tj ET";
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] \
         /Resources << /Font << /F1 5 0 R /FT3 6 0 R /FT3T 10 0 R >> \
         /ExtGState << /Half 13 0 R >> >> /Contents 4 0 R >>\nendobj\n\
         4 0 obj\n<< /Length {} >>\nstream\n{content}\nendstream\nendobj\n\
         5 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>\nendobj\n\
         6 0 obj\n<< /Type /Font /Subtype /Type3 /FontBBox [0 0 750 750] \
         /FontMatrix [0.001 0 0 0.001 0 0] /CharProcs 8 0 R /Encoding 7 0 R \
         /FirstChar 97 /LastChar 97 /Widths [1000] >>\nendobj\n\
         7 0 obj\n<< /Type /Encoding /Differences [97 /a] >>\nendobj\n\
         8 0 obj\n<< /a 9 0 R >>\nendobj\n\
         9 0 obj\n<< /Length {} >>\nstream\n{SQUARE}\nendstream\nendobj\n\
         10 0 obj\n<< /Type /Font /Subtype /Type3 /FontBBox [0 0 750 750] \
         /FontMatrix [0.001 0 0 0.001 0 0] /CharProcs 11 0 R /Encoding 7 0 R \
         /FirstChar 97 /LastChar 97 /Widths [1000] \
         /Resources << /Font << /F1 5 0 R >> >> >>\nendobj\n\
         11 0 obj\n<< /a 12 0 R >>\nendobj\n\
         12 0 obj\n<< /Length {} >>\nstream\n{WITH_TEXT}\nendstream\nendobj\n\
         13 0 obj\n<< /Type /ExtGState /ca 0.5 /CA 0.5 >>\nendobj\n",
        content.len() + 1,
        SQUARE.len() + 1,
        WITH_TEXT.len() + 1,
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

/// Interprets `content` against the fixture.
fn interpret(content: &str) -> pdf_model::Interpretation {
    let document = Document::open(fixture(content)).expect("the fixture is a valid PDF");
    assert!(!document.was_recovered(), "the fixture has a complete xref");
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    pdf_model::interpret(&document, &page)
}

/// Where each glyph fill was placed: the translation of its glyph-to-device transform.
fn glyph_origins(interpretation: &pdf_model::Interpretation) -> Vec<(f32, f32)> {
    interpretation
        .display_list
        .commands()
        .iter()
        .filter_map(|command| match command {
            // The glyphs are the fills drawn under a scale of the font size; the page's own
            // rectangles are drawn under the identity.
            Command::Fill { transform, .. } if (transform.a - 1.0).abs() > 0.5 => {
                Some((transform.e, transform.f))
            }
            _ => None,
        })
        .collect()
}

/// The clip of the last command, which the rectangle every clip test ends with is.
fn last_clip(interpretation: &pdf_model::Interpretation) -> Option<pdf_render::ClipId> {
    interpretation
        .display_list
        .commands()
        .last()
        .expect("the stream paints something")
        .clip()
}

/// Whether the display list holds a knockout group, which §11.7.4.4 makes of a translucent glyph
/// filled and stroked by rendering mode 2.
fn has_knockout_group(interpretation: &pdf_model::Interpretation) -> bool {
    interpretation
        .display_list
        .commands()
        .iter()
        .any(|command| matches!(command, Command::Group { knockout: true, .. }))
}

/// 2161-B01: a second `BT` discards what the text object it interrupts owes at its end.
///
/// ISO 32000-2 Table 105 says "a second BT shall not appear before an ET", and states no reading
/// of one; §9.3.6 states what the glyphs shown in a clipping mode owe whatever follows: "Glyphs
/// shall accumulate until the text object is ended by an ET operator". The only `ET` in the
/// streams below follows a `BT` that cleared the accumulator, so the glyph shown in mode 7 clips
/// nothing on either reading of the second `BT` — as an implicit `ET`, or as nothing — and the
/// page says nothing. And §11.7.4.4's group for a glyph filled and stroked under a translucent
/// alpha is the glyph's own, owed whatever text object it is in: "a non-isolated knockout group
/// shall be established" — which the interrupted object's pairs lose with it.
#[test]
fn a_second_bt_does_not_discard_what_the_open_text_object_owes_spec_audit() {
    // Controls: the same glyphs in a text object that ends.
    let ended = interpret("BT /F1 48 Tf 10 10 Td 7 Tr (A) Tj ET 0 Tr 0 0 100 100 re f");
    assert!(
        last_clip(&ended).is_some(),
        "control: a glyph shown in mode 7 clips what follows its ET"
    );
    let ended_pair = interpret("/Half gs BT /F1 48 Tf 2 Tr 10 10 Td (A) Tj ET");
    assert!(
        has_knockout_group(&ended_pair),
        "control: a translucent mode 2 glyph is one knockout group (§11.7.4.4)"
    );

    let interrupted = interpret("BT /F1 48 Tf 10 10 Td 7 Tr (A) Tj BT 0 Tr ET 0 0 100 100 re f");
    assert!(
        last_clip(&interrupted).is_some() || !interrupted.unsupported.is_empty(),
        "the glyph shown in mode 7 neither clips the rectangle nor is its loss reported: {:?}",
        interrupted.unsupported
    );
    let interrupted_pair = interpret("/Half gs BT /F1 48 Tf 2 Tr 10 10 Td (A) Tj BT ET ET");
    assert!(
        has_knockout_group(&interrupted_pair),
        "a second BT turned the translucent mode 2 glyph into a flat fill and stroke"
    );
}

/// 2161-B02: `TJ` reads operands standing outside its array as elements of it.
///
/// ISO 32000-2 Table 107 gives `TJ` one operand, an array, and "[e]ach element of array shall be
/// either a string or a number"; a number before the `[` is no element. The content lexer
/// flattens arrays, and `set_dash` splits on the brackets it leaves (as nulls), but `TJ` reads
/// every operand: a leftover `5000` before the array moves the text position 50 units left.
#[test]
fn tj_reads_only_the_elements_of_its_array_spec_audit() {
    let control = interpret("BT /F1 10 Tf 10 10 Td [(A) (A)] TJ ET");
    let control_origins = glyph_origins(&control);
    assert_eq!(control_origins.len(), 2, "control: two glyphs");
    assert!(
        (control_origins[0].0 - 10.0).abs() < 0.01,
        "control: the first glyph at the text position: {control_origins:?}"
    );

    let stray = interpret("BT /F1 10 Tf 10 10 Td 5000 [(A) (A)] TJ ET");
    let stray_origins = glyph_origins(&stray);
    assert_eq!(
        stray_origins.len(),
        2,
        "two glyphs either way: {stray_origins:?}"
    );
    assert!(
        (stray_origins[0].0 - control_origins[0].0).abs() < 0.01,
        "a number outside the array was applied as an adjustment: {stray_origins:?} against \
         {control_origins:?}"
    );
}

/// 2161-B03: a text state operand of the wrong type is dropped without a word.
///
/// Table 103: `Tf`'s "size shall be a number", `Tr`'s "render ... shall be an integer", and
/// "There is no initial value for either font or size". ADR 0303 made a `Tf` that states no size
/// a reported loss rather than a size of nought drawn in silence, and the `9.3.1` row says so;
/// a size operand that is a *string* states no size either, and draws nothing and says nothing.
/// `Tr` names an integer outside 0 to 7, and keeps the old mode in silence for `1.5` or a name.
#[test]
fn a_text_state_operand_of_the_wrong_type_is_reported_spec_audit() {
    // Controls: a size with no operand at all is reported (ADR 0303), and so is a mode out of
    // range.
    let no_size = interpret("BT /F1 . Tf 50 50 Td (A) Tj ET");
    assert!(
        !no_size.unsupported.is_empty(),
        "control: a Tf stating no size is a reported loss"
    );
    let out_of_range = interpret("BT /F1 10 Tf -1 Tr 50 50 Td (A) Tj ET");
    assert!(
        !out_of_range.unsupported.is_empty(),
        "control: a Tr outside 0 to 7 is reported"
    );

    let string_size = interpret("BT /F1 (x) Tf 50 50 Td (A) Tj ET");
    assert!(
        !string_size.unsupported.is_empty(),
        "a Tf whose size is a string drew {} commands and reported nothing",
        string_size.display_list.commands().len()
    );
    let fractional_mode = interpret("BT /F1 10 Tf 1.5 Tr 50 50 Td (A) Tj ET");
    assert!(
        !fractional_mode.unsupported.is_empty(),
        "a Tr of 1.5 is no integer, and was ignored without a report"
    );
}

/// 2161-B04: a show after `ET` is placed by the text matrix `ET` discarded, and not reported.
///
/// Table 105: `ET` is "End a text object, discarding the text matrix", and §9.4.3: "The
/// text-showing operators shall only appear within text objects." No reading of a `Tj` outside
/// one is stated, but the one position it cannot have is the discarded matrix's, and an
/// operator the clause forbids where it stands is said, as an unclosed `BT` is.
#[test]
fn a_show_after_et_is_not_placed_by_the_discarded_text_matrix_spec_audit() {
    let inside = interpret("BT /F1 10 Tf 50 50 Td (A) Tj ET");
    assert_eq!(
        glyph_origins(&inside),
        [(50.0, 50.0)],
        "control: inside the text object the glyph is at the Td"
    );
    assert!(inside.unsupported.is_empty(), "control: nothing to report");

    let outside = interpret("BT /F1 10 Tf 50 50 Td ET (A) Tj");
    let origins = glyph_origins(&outside);
    assert!(
        !origins
            .iter()
            .any(|(x, y)| (x - 50.0).abs() < 0.01 && (y - 50.0).abs() < 0.01),
        "the glyph shown after ET is placed by the matrix ET discarded: {origins:?}"
    );
    assert!(
        !outside.unsupported.is_empty(),
        "a text-showing operator outside a text object is not reported"
    );
}

/// 2161-B05: the text object inside a Type 3 glyph description adds a word break to the page's
/// text.
///
/// §9.4.1's NOTE allows a glyph description to hold "another text object", and the page's own
/// positions are unchanged by it (§9.4.4 advances the outer `Tm` by the glyph's width alone):
/// two glyphs 10 units wide from x 10, then an `A` at x 30, exactly adjacent. With the square
/// glyph the page reads `aaA`; with the glyph whose description shows an invisible `A`, the inner
/// text object moves the readback's cursor and the page reads a space the page never placed.
#[test]
fn a_type3_glyphs_own_text_object_adds_no_word_break_to_the_page_spec_audit() {
    let square = interpret("BT /FT3 10 Tf 10 10 Td (aa) Tj /F1 10 Tf (A) Tj ET");
    assert_eq!(
        square.text, "aaA",
        "control: adjacent glyphs read as one word"
    );

    let with_text = interpret("BT /FT3T 10 Tf 10 10 Td (aa) Tj /F1 10 Tf (A) Tj ET");
    assert_eq!(
        glyph_origins(&with_text),
        [(30.0, 10.0)],
        "the page's own A is where the square's would be"
    );
    assert_eq!(
        with_text.text, "aaA",
        "the glyph description's own text object put a break into the page's text"
    );
}

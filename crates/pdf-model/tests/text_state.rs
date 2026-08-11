//! The text state parameters of ISO 32000-2 §9.3, one clause at a time.
//!
//! # Why this file exists
//!
//! §9.3.6, the rendering mode, has `tests/text_render_modes.rs` to itself. This covers the
//! rest of the family, which was read against the code in the thirteenth session and had
//! never been tested at all — the arithmetic reached the page through `tests/type3.rs` and
//! the corpus, neither of which isolates one parameter.
//!
//! Reading it produced one defect, and it is the shape this project keeps finding: a rule
//! stated about *how a code is encoded* implemented as a rule about the code's value.
//! §9.3.3 applies word spacing to "the single-byte character code 32" and says in the next
//! sentence that it "shall not apply to occurrences of the byte value 32 in multiple-byte
//! codes" — so an `Identity-H` string containing `00 20` selects code 32 and takes no word
//! spacing. We applied it, which pushes the rest of the line right by `Tw` per space-valued
//! code. Nothing reported, and no page of Latin text could show it.
//!
//! These assert against the display list, because every parameter here is a number that
//! decides where a glyph goes.
//!
//! Like `tests/text_render_modes.rs`, the outlines come from a font installed on this
//! machine, so a machine with none would pass every assertion vacuously; see that file's
//! module comment for why the helper panics rather than skipping.

#![expect(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "test code: a malformed fixture should fail loudly, and these pages are 100 \
              units square where no index can overflow"
)]

use std::fmt::Write as _;

use pdf_render::Command;
use pdf_syntax::Document;

/// A `/ToUnicode` `CMap` for the composite font below, mapping two codes to themselves.
///
/// A composite font with no embedded program reaches a substitute only through what its
/// codes *mean*, which is what `/ToUnicode` records — so without this the font would draw
/// nothing and the word-spacing tests would measure an empty display list.
const TO_UNICODE: &str = "/CIDInit /ProcSet findresource begin\n\
     12 dict begin\nbegincmap\n1 begincodespacerange\n<0000> <FFFF>\nendcodespacerange\n\
     2 beginbfchar\n<0020> <0020>\n<0041> <0041>\nendbfchar\nendcmap\n\
     CMapName currentdict /CMap defineresource pop\nend\nend";

/// A one-page fixture with a simple font `/F1` and an `Identity-H` composite font `/F0`.
///
/// Both are substituted rather than embedded, which is what makes the pair comparable: the
/// same installed face draws both, so a difference between them is a difference in the
/// rules being tested rather than in the glyphs.
fn fixture(content: &str) -> Vec<u8> {
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] \
         /Resources << /Font << /F1 5 0 R /F0 6 0 R >> \
         /ExtGState << /Half << /ca 0.5 /CA 0.5 >> /Knock << /TK true >> \
         /NoKnock << /TK false >> /Mult << /ca 0.5 /BM /Multiply >> >> >> /Contents 4 0 R >>\nendobj\n\
         4 0 obj\n<< /Length {} >>\nstream\n{content}\nendstream\nendobj\n\
         5 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>\nendobj\n\
         6 0 obj\n<< /Type /Font /Subtype /Type0 /BaseFont /TestCID /Encoding /Identity-H \
         /DescendantFonts [7 0 R] /ToUnicode 9 0 R >>\nendobj\n\
         7 0 obj\n<< /Type /Font /Subtype /CIDFontType2 /BaseFont /TestCID \
         /CIDSystemInfo << /Registry (Adobe) /Ordering (Identity) /Supplement 0 >> \
         /DW 1000 /FontDescriptor 8 0 R >>\nendobj\n\
         8 0 obj\n<< /Type /FontDescriptor /FontName /TestCID /Flags 4 \
         /FontBBox [0 0 1000 1000] /ItalicAngle 0 /Ascent 800 /Descent -200 \
         /CapHeight 700 /StemV 80 >>\nendobj\n\
         9 0 obj\n<< /Length {} >>\nstream\n{TO_UNICODE}\nendstream\nendobj\n",
        content.len() + 1,
        TO_UNICODE.len() + 1,
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

/// Where each glyph a content stream draws was placed, in page units.
///
/// Panics if nothing was drawn, because an empty list would satisfy every assertion below
/// about *where* something is. See the module comment.
fn placements(content: &str) -> Vec<(f32, f32)> {
    let placed: Vec<(f32, f32)> = interpret(content)
        .display_list
        .commands()
        .iter()
        .filter_map(|command| match command {
            Command::Fill { transform, .. } | Command::Stroke { transform, .. } => {
                Some((transform.e, transform.f))
            }
            _ => None,
        })
        .collect();
    assert!(
        !placed.is_empty(),
        "no glyph was drawn for {content:?}: either the fixture is wrong or no font on this \
         machine substitutes for it, and in the second case every test here is vacuous"
    );
    placed
}

/// §9.3.3: word spacing applies to a single-byte code 32.
#[test]
fn word_spacing_moves_the_glyph_after_a_simple_fonts_space() {
    // Helvetica's space is 278/1000 em, so at size 10 the `A` sits at 2.78 without word
    // spacing. Asking for 50 units of it must put the `A` beyond 50.
    let without = placements("BT /F1 10 Tf 0 Tw 0 0 Td ( A) Tj ET");
    let with = placements("BT /F1 10 Tf 50 Tw 0 0 Td ( A) Tj ET");

    let moved = with[0].0 - without[0].0;
    assert!(
        (moved - 50.0).abs() < 0.01,
        "one space, one word spacing: the glyph moved by {moved}"
    );
}

/// §9.3.3: and *not* to the byte value 32 inside a two-byte code.
///
/// This is the defect the family review found. The clause is explicit — word spacing "shall
/// not apply to occurrences of the byte value 32 in multiple-byte codes" — and we applied it
/// to any code equal to 32 whatever its encoded length, so an `Identity-H` line containing
/// `<0020>` was pushed right by `Tw` for every one of them. A page of Latin text cannot show
/// this, because a composite font's space is usually some other CID entirely.
#[test]
fn word_spacing_does_not_reach_a_two_byte_code_32() {
    let without = placements("BT /F0 10 Tf 0 Tw 0 0 Td <00200041> Tj ET");
    let with = placements("BT /F0 10 Tf 50 Tw 0 0 Td <00200041> Tj ET");

    assert_eq!(
        with, without,
        "a two-byte code 32 takes no word spacing, so `Tw` must change nothing"
    );
}

/// §9.3.2 and §9.3.4: character spacing is added per glyph, and horizontal scaling scales it.
///
/// §9.3.4 is the half worth pinning: "If the writing mode is horizontal, it shall also
/// affect the spacing parameters Tc and Tw, as well as any positioning adjustments performed
/// by the TJ operator." Applying `Tz` to the glyph and not to the spacing is the plausible
/// wrong reading, and it drifts a line further out of place the longer it is.
#[test]
fn horizontal_scaling_reaches_the_character_spacing_too() {
    let plain = placements("BT /F1 10 Tf 5 Tc 0 0 Td (AAA) Tj ET");
    let halved = placements("BT /F1 10 Tf 5 Tc 50 Tz 0 0 Td (AAA) Tj ET");

    // Three glyphs, so two advances, each of them (width * size + Tc) * Th.
    let plain_step = plain[1].0 - plain[0].0;
    let halved_step = halved[1].0 - halved[0].0;
    assert!(
        (halved_step - plain_step / 2.0).abs() < 0.01,
        "halving Th must halve the whole advance including Tc: {plain_step} then {halved_step}"
    );
    assert!(
        plain_step > 10.0,
        "the character spacing has to be in there at all: {plain_step}"
    );
}

/// §9.3.4 and §9.4.3: a `TJ` adjustment is scaled horizontally as well.
#[test]
fn horizontal_scaling_reaches_a_tj_adjustment() {
    let plain = placements("BT /F1 10 Tf 0 0 Td [(A) -1000 (A)] TJ ET");
    let halved = placements("BT /F1 10 Tf 50 Tz 0 0 Td [(A) -1000 (A)] TJ ET");

    let plain_step = plain[1].0 - plain[0].0;
    let halved_step = halved[1].0 - halved[0].0;
    assert!(
        (halved_step - plain_step / 2.0).abs() < 0.01,
        "the TJ adjustment scales with Th: {plain_step} then {halved_step}"
    );
}

/// §9.3.7: text rise moves the baseline, in *unscaled* text space units.
///
/// "Unscaled" is the rule that is easy to lose: a rise of 5 lifts the baseline by 5 user
/// units whatever the font size, so multiplying it by the size — which the surrounding
/// glyph-space-to-text-space matrix does to everything else — would make a superscript on a
/// 24-point heading five times higher than on a 5-point footnote.
#[test]
fn text_rise_is_not_scaled_by_the_font_size() {
    let small = placements("BT /F1 5 Tf 5 Ts 0 0 Td (A) Tj ET");
    let large = placements("BT /F1 25 Tf 5 Ts 0 0 Td (A) Tj ET");

    assert!(
        (small[0].1 - 5.0).abs() < 0.01 && (large[0].1 - 5.0).abs() < 0.01,
        "the rise is the same 5 units at either size: {} and {}",
        small[0].1,
        large[0].1
    );
}

/// §9.3.5: leading is used by `T*`, `'` and `"`, and `TD` sets it.
///
/// Table 103 names exactly four operators, and Table 106 gives `T*` as `0 -Tl TD` — the
/// negation being the part a reader gets backwards, since leading is "expressed as a
/// positive number" and going to the next line decreases y.
#[test]
fn leading_moves_the_next_line_downwards() {
    let lines = placements("BT /F1 10 Tf 12 TL 0 50 Td (A) Tj T* (A) Tj (A) ' ET");
    assert_eq!(lines.len(), 3, "three lines, three glyphs");
    assert!(
        (lines[0].1 - 50.0).abs() < 0.01
            && (lines[1].1 - 38.0).abs() < 0.01
            && (lines[2].1 - 26.0).abs() < 0.01,
        "each line sits one leading below the last: {lines:?}"
    );
}

/// ISO 32000-2 §9.3.1: a text state operator outside a text object, and what it survives.
///
/// > The text state operators may appear outside text objects, and the values they set are
/// > retained across text objects in a single content stream.
///
/// The clause's next sentence is the other half — "[l]ike other graphics state parameters,
/// these parameters shall be initialised to their default values at the beginning of each
/// page" — and both fall out of the same construction, which is that the text state lives in
/// the graphics state rather than in the text object. So `Q` restores it, which is the part a
/// text object of its own could not show: the `TL` set before the first `BT` is still in force
/// inside the *second* one, and the `TL` set inside a `q` is not.
///
/// **This is the sentence §9.3's row rested its `implemented` on from the
/// four-hundred-and-thirty-seventh session**, and nothing had asserted it: the family's tests
/// each set a parameter inside the text object that uses it.
#[test]
fn a_text_state_operator_outside_a_text_object_is_retained_and_saved() {
    let across =
        placements("12 TL BT /F1 10 Tf 0 50 Td (A) Tj ET BT /F1 10 Tf 0 30 Td (A) Tj T* (A) Tj ET");
    assert_eq!(across.len(), 3, "three glyphs: {across:?}");
    assert!(
        (across[2].1 - 18.0).abs() < 0.01,
        "the leading set before the first BT still moves the line inside the second: {across:?}"
    );

    let saved = placements("12 TL q 40 TL Q BT /F1 10 Tf 0 50 Td (A) Tj T* (A) Tj ET");
    assert_eq!(saved.len(), 2, "two glyphs: {saved:?}");
    assert!(
        (saved[1].1 - 38.0).abs() < 0.01,
        "and `Q` restores it, because the text state is part of the graphics state: {saved:?}"
    );
}

/// §9.3.1: "Zero sized text shall not mark or clip any pixels (depending on text render
/// mode)."
///
/// The clipping half is the one worth a test, and it is worth it because the safe-looking
/// implementation is wrong in the dangerous direction: a zero-sized glyph collapses to a
/// point, and clipping to a degenerate path hides everything painted afterwards.
#[test]
fn zero_sized_text_neither_marks_nor_clips() {
    let marked = interpret("BT /F1 0 Tf 0 0 Td (A) Tj ET");
    assert!(
        marked.display_list.commands().is_empty(),
        "zero-sized text marks nothing: {:?}",
        marked.display_list.commands().len()
    );

    let clipped = interpret("BT /F1 0 Tf 7 Tr 0 0 Td (A) Tj ET 0 0 100 100 re f");
    let rectangle = clipped
        .display_list
        .commands()
        .last()
        .expect("the rectangle is drawn");
    assert!(
        rectangle.clip().is_none(),
        "zero-sized text clips nothing either"
    );
}

/// §9.3.1 NOTE: "Negative text font size is permitted."
///
/// It draws the glyph upside down and advances leftwards, both of which fall out of the
/// arithmetic. What does not fall out is the extracted text: the word-break threshold is a
/// fraction of the font size, and a negative threshold is below every gap there is, so every
/// glyph looked like the start of a new word until the thirteenth session took its
/// magnitude.
#[test]
fn a_negative_font_size_draws_and_reads_normally() {
    let document = Document::open(fixture("BT /F1 -10 Tf 0 50 Td (AB) Tj ET")).expect("valid PDF");
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    let interpretation = pdf_model::interpret(&document, &page);

    assert_eq!(
        interpretation.text.trim(),
        "AB",
        "no word break belongs between the two: {:?}",
        interpretation.text
    );
    let Some(Command::Fill { transform, .. }) = interpretation.display_list.commands().first()
    else {
        panic!("a negative size still draws");
    };
    assert!(
        transform.d < 0.0,
        "and the glyph is flipped rather than skipped: {transform:?}"
    );
}

/// Whether a content stream reported §9.3.8's text knockout.
fn reports_knockout(content: &str) -> bool {
    interpret(content)
        .unsupported
        .iter()
        .any(|item| matches!(item, pdf_model::Unsupported::TextKnockout { .. }))
}

/// Whether a content stream's text object became §9.3.8's knockout group.
fn draws_knockout(content: &str) -> bool {
    interpret(content)
        .display_list
        .commands()
        .iter()
        .any(|command| matches!(command, Command::Group { knockout: true, .. }))
}

/// §9.3.8 makes a text object a knockout group where it could change the page.
///
/// `Tk`'s initial value is true, and the clause defines that as treating the text object "as
/// if it were a non-isolated knockout transparency group", so that "later glyphs shall
/// overwrite ('knock out') earlier ones in the area of overlap". Since the seventy-second
/// session the display list can say exactly that (§11.4.6), so the object becomes one
/// `Command::Group` — but only where the two models differ, which needs *both* of the
/// clause's conditions: the paint composites, and two glyphs overlap.
///
/// The test drives all four combinations from one fixture, because the value of the
/// condition is entirely in its precision — wrapping every page of text under a constant
/// alpha would build a group for a difference that cannot be on almost any of them, and cost
/// every one of those pages a buffer. Two glyphs are overlapped by moving the text matrix
/// back by less than an advance.
#[test]
fn text_knockout_becomes_a_group_only_where_the_two_models_differ() {
    let overlapping = "BT /F1 24 Tf 10 50 Td (A) Tj -8 0 Td (A) Tj ET";
    let apart = "BT /F1 24 Tf 10 50 Td (A) Tj 40 0 Td (A) Tj ET";

    assert!(
        draws_knockout(&format!("/Half gs {overlapping}")),
        "two overlapping glyphs at half alpha are where knockout shows"
    );
    assert!(
        !draws_knockout(&format!("/Half gs {apart}")),
        "glyphs that do not overlap composite the same either way"
    );
    assert!(
        !draws_knockout(overlapping),
        "opaque glyphs under the Normal blend mode overwrite what they cover either way"
    );
    assert!(
        !draws_knockout(&format!("/Half gs /NoKnock gs {overlapping}")),
        "/TK false asks for exactly the model this renderer has"
    );
    assert!(
        !reports_knockout(&format!("/Half gs {overlapping}")),
        "and what is drawn is not also reported"
    );
}

/// A text object whose glyphs *blend* keeps §9.3.8's report.
///
/// The clause makes the implicit group **non-isolated**, and this renderer composites a
/// group's elements onto transparency. §11.4.4's NOTE 3 is what makes those the same
/// computation — the backdrop is composited in and removed again — and it is only the same
/// where every element blends Normal. A glyph with a blend mode is exactly the case where the
/// group's own backdrop is load-bearing, so the object is drawn as it was before and says so.
#[test]
fn text_knockout_still_reports_where_a_glyph_blends() {
    let overlapping = "BT /F1 24 Tf 10 50 Td (A) Tj -8 0 Td (A) Tj ET";
    assert!(
        reports_knockout(&format!("/Mult gs {overlapping}")),
        "a non-isolated group whose elements blend needs the backdrop this one drops"
    );
    assert!(
        !draws_knockout(&format!("/Mult gs {overlapping}")),
        "and it is not drawn as one"
    );
}

/// §9.3.8: a `/TK` set inside a text object is ignored.
///
/// > Any TK value in a graphics state parameter dictionary installed using the gs operator
/// > shall be ignored between the BT and ET operators delimiting a text object.
///
/// So the `/NoKnock gs` below changes nothing, and the object still reports — where the same
/// operator one line earlier, outside the `BT`, silences it. This is the only text state
/// parameter with a rule of that kind, and the natural implementation reads the key wherever
/// it appears.
#[test]
fn a_text_knockout_set_inside_a_text_object_is_ignored() {
    assert!(
        draws_knockout("/Half gs BT /NoKnock gs /F1 24 Tf 10 50 Td (A) Tj -8 0 Td (A) Tj ET"),
        "a /TK between BT and ET does not take effect"
    );
    assert!(
        !draws_knockout("/Half gs /NoKnock gs BT /F1 24 Tf 10 50 Td (A) Tj -8 0 Td (A) Tj ET"),
        "the same dictionary outside the text object does"
    );
}

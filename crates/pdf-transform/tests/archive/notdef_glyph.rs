//! A code a TrueType font sends to its program's `.notdef` glyph is given a glyph of its own
//! whose outline and advance are `.notdef`'s, and the content stream stays the producer's bytes
//! (`doc/adr/2543`, `doc/questions/A694` item 3).
//!
//! ISO 19005-2 section 6.2.11.8 and ISO 19005-4 section 6.2.10.9 forbid a text-showing operator
//! to reference `.notdef`, which in a TrueType program is glyph 0 by position. The tests hold the
//! conversion to what makes the answer a restatement rather than an invention: the codes that
//! drew glyph 0 draw a glyph whose outline is glyph 0's, every other code its own glyph, and the
//! output conforms.

use pdf_font::{Code, LoadedFont};
use pdf_syntax::object::ObjectId;
use pdf_syntax::{Document, Limits};

use super::{
    Authorisations, Conforming, Decision, LIBERATION_SANS, Level, Rewrite, Target, Verdict,
    conversion, convert, decision, holds, stream,
};

const NOTDEF_SHOWN: &str = "fonts/no-notdef-glyph-shown";

/// A page showing `A`, codes 1 and 2, and `B` in one TrueType font whose `Differences` names, for
/// codes 1 and 2, characters the program has no glyph for — so both reach glyph 0 — and whose
/// `/Widths` gives the two codes `widths`.
fn a_page_showing_notdef_twice(widths: (u16, u16)) -> Vec<u8> {
    let mut each = vec!["667".to_owned(); 66];
    each[0] = widths.0.to_string();
    each[1] = widths.1.to_string();
    let widths = each.join(" ");
    Conforming {
        resources: "/Font << /F1 6 0 R >> /ColorSpace << /CS0 [/CalGray << /WhitePoint \
                    [0.9505 1.0 1.089] >>] >>"
            .to_owned(),
        contents: Some((
            String::new(),
            b"/CS0 cs 0 sc BT /F1 12 Tf 10 100 Td (A\\001\\002B) Tj ET".to_vec(),
        )),
        objects: vec![
            format!(
                "<< /Type /Font /Subtype /TrueType /BaseFont /LiberationSans /FirstChar 1 \
                 /LastChar 66 /Widths [{widths}] /FontDescriptor 7 0 R /Encoding << /Type \
                 /Encoding /BaseEncoding /WinAnsiEncoding /Differences [1 /uni4E00 /uni4E01] >> >>"
            ),
            "<< /Type /FontDescriptor /FontName /LiberationSans /Flags 32 /FontBBox [-543 -303 \
             1300 980] /ItalicAngle 0 /Ascent 905 /Descent -212 /CapHeight 716 /StemV 80 \
             /FontFile2 8 0 R >>"
                .to_owned(),
        ],
        binary_objects: vec![stream(
            &format!("/Length {}", LIBERATION_SANS.len()),
            LIBERATION_SANS,
        )],
        ..Conforming::part_two()
    }
    .build()
}

/// The glyph each of codes 1, 2, 65 and 66 reaches in the font object 6 of `bytes`.
fn glyphs_of(bytes: &[u8]) -> [Option<u16>; 4] {
    let held = Document::open_with_limits(bytes.to_vec(), Limits::DEFAULT).expect("it opens");
    let dict = held
        .get(ObjectId::new(6, 0))
        .as_dict()
        .cloned()
        .expect("the font object");
    let font = LoadedFont::load(&held, &dict, "").expect("the font loads");
    [1, 2, 65, 66].map(|byte| font.glyph_index(Code::single_byte(byte)))
}

#[test]
fn two_codes_on_notdef_at_one_width_reach_one_glyph_of_their_own_and_the_rest_stay() {
    given_glyphs(&a_page_showing_notdef_twice((667, 667)), true);
}

#[test]
fn two_codes_on_notdef_at_two_widths_reach_two_glyphs_each_at_its_width() {
    given_glyphs(&a_page_showing_notdef_twice((250, 1000)), false);
}

/// Converts `source` at 2b and part 4 and holds each output to the restatement's three
/// properties, the two codes on one glyph where `shared` and on two otherwise.
fn given_glyphs(source: &[u8], shared: bool) {
    let before = glyphs_of(source);
    assert_eq!(
        before[..2],
        [Some(0), Some(0)],
        "both codes reach .notdef: {before:?}"
    );
    let failed: Vec<&str> = holds(source, Target::Two(Level::B))
        .failures()
        .map(|failed| failed.id)
        .collect();
    assert!(failed.contains(&NOTDEF_SHOWN), "{failed:?}");
    for target in [
        Target::Two(Level::B),
        Target::Four(pdf_archive::Flavour::Plain),
    ] {
        let (report, output) = convert(source, target, Authorisations::default());
        assert!(
            matches!(
                decision(&report, NOTDEF_SHOWN),
                Decision::Stated {
                    rewrite: Rewrite::NotdefGlyphGiven,
                    ..
                }
            ),
            "{:?}",
            decision(&report, NOTDEF_SHOWN)
        );
        let output = output.unwrap_or_else(|| {
            panic!(
                "{:?}\n{:?}\n{:?}",
                conversion(&report).achieved,
                report.refused,
                report.warnings
            )
        });
        let held = holds(&output, target);
        assert_eq!(held.verdict(), Verdict::Conforms, "{}", held.render());
        let after = glyphs_of(&output);
        let given = after[0].expect("code 1 reaches a glyph");
        assert_ne!(given, 0, "no code reaches .notdef");
        if shared {
            assert_eq!(after[1], Some(given), "the two codes share the one copy");
        } else {
            assert!(
                after[1].is_some_and(|other| other != given && other != 0),
                "two widths, two glyphs: {after:?}"
            );
        }
        assert_eq!(
            after[2..],
            before[2..],
            "A and B reach the glyphs they reached"
        );
        let text = String::from_utf8_lossy(&output).into_owned();
        assert!(
            text.contains("BT /F1 12 Tf 10 100 Td (A\\001\\002B) Tj ET"),
            "the content stream is the producer's bytes"
        );
        let packet = String::from_utf8(super::document_packet(&output)).expect("UTF-8");
        assert!(packet.contains("ISO 19005-2 section 6.2.11.8"), "{packet}");
    }
}

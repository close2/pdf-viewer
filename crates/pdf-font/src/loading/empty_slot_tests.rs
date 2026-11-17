//! An embedded Type 1 program's code whose slot its own `/Encoding` array never assigned.
//!
//! The Adobe Type 1 Font Format, a normative reference of ISO 32000-2, has a special encoding's
//! assignments "performed as shown in the example in section 2.3", and that example fills the
//! array before assigning anything: `0 1 255 {1 index exch /.notdef put } for`. Its `BuildChar`
//! takes the name of the character to build from the array for every code. So a code the array
//! never assigns names `.notdef`, which §9.6.5.2 requires every program to contain, and the code
//! draws that glyph (ADR 2956). The table still reports the code as reaching no glyph of its own
//! (`type1::Program`'s `assigned` says why), and a code meaning a space keeps the whitespace
//! departure `substitutes_notdef` documents.
//!
//! The fixture's `.notdef` is a box, so drawing it and drawing nothing are told apart by the
//! outline itself; `type1::tests::program`'s array fills every slot with `.notdef` and assigns
//! code 65 alone.

use crate::Code;
use crate::fixture::document_of;
use crate::loading::LoadedFont;
use crate::type1::tests::{ENDCHAR, HSBW, number, plain_glyph, program};

/// Type 1 charstring operators (Adobe Type 1 Font Format, section 6.4).
const RLINETO: u8 = 5;
const CLOSEPATH: u8 = 9;

/// A `.notdef` that paints a 400 × 700 box after `0 500 hsbw`.
fn boxed_notdef() -> Vec<u8> {
    let mut body = number(0);
    body.extend(number(500));
    body.push(HSBW);
    for (dx, dy) in [(0, 700), (400, 0), (0, -700)] {
        body.extend(number(dx));
        body.extend(number(dy));
        body.push(RLINETO);
    }
    body.extend([CLOSEPATH, ENDCHAR]);
    body
}

/// The fixture program under the font dictionary entries given, with `extra` objects after it.
fn font(entries: &str, extra: &[&[u8]]) -> LoadedFont {
    let data = program(&boxed_notdef(), &plain_glyph(600), &[]);
    let widths = ["500"; 256].join(" ");
    let font_object = format!(
        "1 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Fixture \
         /FirstChar 0 /LastChar 255 /Widths [{widths}] {entries} \
         /FontDescriptor << /Type /FontDescriptor /FontName /Fixture /Flags 32 \
         /FontFile 2 0 R >> >>\nendobj\n"
    );
    let mut stream = format!("2 0 obj\n<< /Length {} >>\nstream\n", data.len()).into_bytes();
    stream.extend_from_slice(&data);
    stream.extend_from_slice(b"\nendstream\nendobj\n");
    let mut objects: Vec<&[u8]> = vec![font_object.as_bytes(), &stream];
    objects.extend_from_slice(extra);
    let (document, dict) = document_of(&objects);
    LoadedFont::load(&document, &dict, "F1").expect("the fixture font loads")
}

/// A code the program's array never assigned draws the program's `.notdef`.
#[test]
fn a_code_the_array_never_assigned_draws_notdef() {
    let font = font("", &[]);
    assert!(!font.is_substituted(), "the embedded program is drawn");
    let unassigned = Code::single_byte(66);

    assert_eq!(font.glyph_index(unassigned), None, "no glyph of its own");
    assert!(font.draws_notdef_in_place(unassigned));
    let drawn = font
        .outline(unassigned)
        .expect("the code draws .notdef's box");
    let named = self::font("/Encoding << /Differences [66 /.notdef] >>", &[])
        .outline(unassigned)
        .expect("a code naming .notdef outright draws it");
    assert_eq!(
        drawn.commands(),
        named.commands(),
        "the fill names the glyph a /Differences entry names"
    );
    assert!(
        font.outline(Code::single_byte(65)).is_none(),
        "the control: the assigned code reaches `A`, which paints nothing"
    );
}

/// A code the array never assigned and the `/ToUnicode` reads as a space draws nothing.
#[test]
fn a_code_never_assigned_that_means_a_space_draws_nothing() {
    const TO_UNICODE: &[u8] = b"/CIDInit /ProcSet findresource begin\n\
        12 dict begin\n\
        begincmap\n\
        /CMapName /Fixture def\n\
        1 begincodespacerange\n<00> <FF>\nendcodespacerange\n\
        2 beginbfchar\n<20> <0020>\n<42> <0042>\nendbfchar\n\
        endcmap\n\
        CMapName currentdict /CMap defineresource pop\n\
        end end\n";
    let mut stream = format!("3 0 obj\n<< /Length {} >>\nstream\n", TO_UNICODE.len()).into_bytes();
    stream.extend_from_slice(TO_UNICODE);
    stream.extend_from_slice(b"\nendstream\nendobj\n");
    let font = font("/ToUnicode 3 0 R", &[&stream]);

    assert!(!font.draws_notdef_in_place(Code::single_byte(0x20)));
    assert!(font.outline(Code::single_byte(0x20)).is_none());
    assert!(
        font.draws_notdef_in_place(Code::single_byte(0x42)),
        "the control: a code meaning a letter still draws .notdef"
    );
}

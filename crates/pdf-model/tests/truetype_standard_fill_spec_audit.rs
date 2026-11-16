//! Exact-xref public font-consumer regression for §9.6.5.4's final `StandardEncoding` fill.
//!
//! Source-derived distinction: Annex D.4 leaves `MacExpert` code 168 (octal 250) unassigned,
//! while Annex D.2 assigns `StandardEncoding` code 168 to `currency`. In the embedded font's
//! cmap, direct U+00A8/code-168 lookup instead reaches `dieresis`. The `WinAnsi` control reaches
//! `dieresis` through the named base. The main `MacExpert` target has no Differences at all, so it
//! is a clean producer-side form. A separate, explicitly guidance-qualified control applies a
//! Differences assignment at code 40.
//!
//! The TTF is checked in at data/standard-fonts/LiberationSans-Regular.ttf, SHA256
//! f8ace1f892b2bd9dc1792ba7f097fa7588f84fed48321480e04de5390828221f, UPM 2048. Independent
//! fontTools/TTF expectations: `currency` GID 101, advance 1139; `dieresis` GID 105, advance
//! 682; `parenright` GID 12, advance 682. PDF `/Widths` 556 and 333 are nearest-integer
//! thousandths-of-em values. The main target has an Encoding dictionary with a permitted
//! `MacExpert` base and no Differences. The separate Differences control uses a form Table 112
//! says SHOULD not be used with TrueType; it tests overlay behavior but is not needed to establish
//! the clean target's required fill.

#![expect(
    clippy::expect_used,
    reason = "fixture assertions intentionally fail at the first invalid source premise"
)]

use pdf_font::{Code, LoadedFont};
use pdf_model::Pages;
use pdf_syntax::Document;

const LIBERATION: &[u8] = include_bytes!("../../../data/standard-fonts/LiberationSans-Regular.ttf");

#[derive(Clone, Copy)]
enum Base {
    MacExpert,
    WinAnsi,
}

fn push_object(output: &mut Vec<u8>, offsets: &mut [usize], number: usize, body: &[u8]) {
    offsets[number] = output.len();
    output.extend_from_slice(format!("{number} 0 obj\n").as_bytes());
    output.extend_from_slice(body);
    output.extend_from_slice(b"\nendobj\n");
}

fn stream_body(dictionary: &[u8], data: &[u8]) -> Vec<u8> {
    let mut body = Vec::new();
    body.extend_from_slice(b"<< ");
    body.extend_from_slice(dictionary);
    body.extend_from_slice(format!(" /Length {} >>\nstream\n", data.len()).as_bytes());
    body.extend_from_slice(data);
    body.extend_from_slice(b"\nendstream");
    body
}

fn pdf(base: Base, code: u8, width: u16, with_difference: bool) -> Vec<u8> {
    let base_name: &[u8] = match base {
        Base::MacExpert => b"MacExpertEncoding",
        Base::WinAnsi => b"WinAnsiEncoding",
    };
    let mut output = b"%PDF-1.7\n%\xE2\xE3\xCF\xD3\n".to_vec();
    let mut offsets = vec![0usize; 8];
    push_object(
        &mut output,
        &mut offsets,
        1,
        b"<< /Type /Catalog /Pages 2 0 R >>",
    );
    push_object(
        &mut output,
        &mut offsets,
        2,
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
    );
    push_object(
        &mut output,
        &mut offsets,
        3,
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Resources << /Font << /F1 4 0 R >> >> /Contents 7 0 R >>",
    );
    let mut font = format!(
        "<< /Type /Font /Subtype /TrueType /BaseFont /LiberationSans /FirstChar {code} /LastChar {code} /Widths [{width}] /Encoding << /BaseEncoding /"
    )
    .into_bytes();
    font.extend_from_slice(base_name);
    if with_difference {
        // This changes the MacExpert base's parenrightsuperior at code 40 to parenright.
        font.extend_from_slice(b" /Differences [40 /parenright]");
    }
    font.extend_from_slice(b" >> /FontDescriptor 5 0 R >>");
    push_object(&mut output, &mut offsets, 4, &font);
    push_object(
        &mut output,
        &mut offsets,
        5,
        b"<< /Type /FontDescriptor /FontName /LiberationSans /Flags 32 /FontBBox [-204 -304 1051 911] /ItalicAngle 0 /Ascent 729 /Descent -211 /CapHeight 688 /StemV 80 /FontFile2 6 0 R >>",
    );
    let file_dict = format!("/Length1 {}", LIBERATION.len());
    let font_file = stream_body(file_dict.as_bytes(), LIBERATION);
    push_object(&mut output, &mut offsets, 6, &font_file);
    let content = stream_body(
        b"",
        format!("BT /F1 20 Tf 1 0 0 1 10 40 Tm <{code:02X}> Tj ET\n").as_bytes(),
    );
    push_object(&mut output, &mut offsets, 7, &content);

    let xref = output.len();
    output.extend_from_slice(format!("xref\n0 {}\n", offsets.len()).as_bytes());
    output.extend_from_slice(b"0000000000 65535 f \n");
    for offset in offsets.iter().skip(1) {
        output.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    output.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R /ID [<0123456789abcdef0123456789abcdef> <0123456789abcdef0123456789abcdef>] >>\nstartxref\n{xref}\n%%EOF\n",
            offsets.len()
        )
        .as_bytes(),
    );
    output
}

fn load(base: Base, code: u8, width: u16, with_difference: bool) -> LoadedFont {
    let document =
        Document::open(pdf(base, code, width, with_difference)).expect("valid exact-xref PDF");
    assert!(!document.was_recovered(), "fixture has exact xref offsets");
    let page = Pages::new(&document).get(0).expect("one page");
    let resource_fonts = document.get_key(&page.resources, "Font");
    let fonts = resource_fonts.as_dict().expect("Font resources");
    let dict = document
        .resolve(&document.get_key(fonts, "F1"))
        .as_dict()
        .expect("F1 dictionary")
        .clone();
    LoadedFont::load(&document, &dict, "F1").expect("embedded TrueType font loads")
}

fn glyph_and_advance(base: Base, code: u8, width: u16, with_difference: bool) -> (u16, f32, f32) {
    let font = load(base, code, width, with_difference);
    let selector = Code::single_byte(code);
    let glyph = font.glyph_index(selector).expect("selected glyph");
    let outline = font.outline(selector).expect("selected outline");
    assert!(
        !outline.commands().is_empty(),
        "selected glyph has a real outline"
    );
    let stated_advance = font.advance(selector);
    let program_advance = font.program_advance(selector).expect("font hmtx advance");
    (glyph, stated_advance, program_advance)
}

#[test]
fn explicit_macexpert_base_without_differences_fills_remaining_slot_from_standard() {
    // Named-base control: WinAnsi byte 168 is `dieresis`, GID 105; declared width 333.
    let control = glyph_and_advance(Base::WinAnsi, 168, 333, false);
    assert_eq!(control.0, 105);
    assert!((control.1 - 0.333).abs() < 0.000_001);
    assert!((control.2 - (682.0 / 2048.0)).abs() < 0.000_001);

    // Annex D.4 leaves MacExpert code 168 blank. Table D.2 gives StandardEncoding code 168
    // `currency`, GID 101 in this embedded TTF, and width 556. Direct cmap lookup of byte 168
    // would incorrectly give the control's `dieresis` GID 105. This Encoding dictionary has an
    // explicit permitted base and no Differences, so it avoids Table 112's TrueType guidance.
    let target = glyph_and_advance(Base::MacExpert, 168, 556, false);
    assert_eq!(target.0, 101);
    assert!((target.1 - 0.556).abs() < 0.000_001);
    assert!((target.2 - (1139.0 / 2048.0)).abs() < 0.000_001);
}

#[test]
fn optional_differences_overlay_remains_separate_from_the_clean_fill_case() {
    // This separate input exercises the overlay after an explicit base. Table 112 says a
    // Differences array SHOULD not be used with TrueType, so this is supplementary reader
    // coverage and not the premise for the clean code-168 fill assertion above.
    let difference = glyph_and_advance(Base::MacExpert, 40, 333, true);
    assert_eq!(difference.0, 12);
    assert!((difference.1 - 0.333).abs() < 0.000_001);
    assert!((difference.2 - (682.0 / 2048.0)).abs() < 0.000_001);
}

//! Exact-xref public font-consumer test for §7.3.10 references inside /Differences.
//!
//! It compares an inline array [39 /quoteright /A] with indirect references to the integer
//! 39 and both glyph-name objects. The selected font is the checked-in Liberation Sans TTF;
//! independent fontTools inspection recorded in the sibling simple-font audit gives
//! quoteright GID 527 (hmtx 455/2048 UPM), A GID 36 (hmtx 1366/2048 UPM). The direct control
//! and indirect target both use matching PDF widths [222.16796875 666.9921875]. These names differ from the
//! `WinAnsi` glyphs at codes 39/40 (quotesingle and parenleft), so ignored references cannot
//! pass accidentally. Table 112 says Differences should not be used with TrueType; this is a
//! recommendation, not a prohibition, and the case is supplementary consumer coverage of the
//! general object-reference rule rather than evidence that writers should prefer this form.

#![expect(
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    clippy::indexing_slicing,
    reason = "exact fixture parsing and literal assertions should fail loudly"
)]

use pdf_font::{Code, LoadedFont};
use pdf_model::Pages;
use pdf_syntax::Document;

const FONT_DATA: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../data/standard-fonts/LiberationSans-Regular.ttf"
));

fn stream(dictionary: &str, data: &[u8]) -> Vec<u8> {
    let mut body = format!("<< {dictionary} /Length {} >>\nstream\n", data.len()).into_bytes();
    body.extend_from_slice(data);
    body.extend_from_slice(b"\nendstream");
    body
}

fn pdf(indirect: bool) -> Vec<u8> {
    let differences = if indirect {
        "[8 0 R 9 0 R 10 0 R]"
    } else {
        "[39 /quoteright /A]"
    };
    let font = format!(
        "<< /Type /Font /Subtype /TrueType /BaseFont /LiberationSans \
         /FirstChar 39 /LastChar 40 /Widths [222.16796875 666.9921875] \
         /Encoding << /BaseEncoding /WinAnsiEncoding /Differences {differences} >> \
         /FontDescriptor 5 0 R >>"
    );
    let descriptor = b"<< /Type /FontDescriptor /FontName /LiberationSans /Flags 32 \
        /FontBBox [-204 -304 1051 911] /ItalicAngle 0 /Ascent 729 /Descent -211 \
        /CapHeight 688 /StemV 80 /FontFile2 6 0 R >>"
        .to_vec();
    let content = stream("", b"BT /F1 20 Tf 1 0 0 1 10 40 Tm <2728> Tj ET\n");
    let objects = vec![
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] \
          /Resources << /Font << /F1 4 0 R >> >> /Contents 7 0 R >>"
            .to_vec(),
        font.into_bytes(),
        descriptor,
        stream(&format!("/Length1 {}", FONT_DATA.len()), FONT_DATA),
        content,
        b"39".to_vec(),
        b"/quoteright".to_vec(),
        b"/A".to_vec(),
    ];
    assemble(&objects)
}

fn assemble(objects: &[Vec<u8>]) -> Vec<u8> {
    let mut out = b"%PDF-1.7\n%\xE2\xE3\xCF\xD3\n".to_vec();
    let mut offsets = vec![0usize; objects.len() + 1];
    for (index, object) in objects.iter().enumerate() {
        let number = index + 1;
        offsets[number] = out.len();
        out.extend_from_slice(format!("{number} 0 obj\n").as_bytes());
        out.extend_from_slice(object);
        out.extend_from_slice(b"\nendobj\n");
    }
    let xref = out.len();
    out.extend_from_slice(format!("xref\n0 {}\n", offsets.len()).as_bytes());
    out.extend_from_slice(b"0000000000 65535 f \n");
    for offset in offsets.iter().skip(1) {
        out.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    out.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R /ID [<0123456789abcdef0123456789abcdef> <0123456789abcdef0123456789abcdef>] >>\nstartxref\n{xref}\n%%EOF\n",
            offsets.len()
        )
        .as_bytes(),
    );
    out
}

fn assert_consumer_result(indirect: bool) {
    let document = Document::open(pdf(indirect)).expect("valid exact-xref PDF");
    assert!(
        !document.was_recovered(),
        "fixture has correct xref offsets"
    );
    let page = Pages::new(&document).get(0).expect("one page");
    let interpretation = pdf_model::interpret(&document, &page);
    assert!(
        interpretation.is_complete(),
        "embedded simple font is fully interpreted: {:?}",
        interpretation.unsupported
    );
    assert_eq!(interpretation.glyphs, 2, "both text codes reach glyphs");

    let font_resources = document.get_key(&page.resources, "Font");
    let fonts = font_resources.as_dict().expect("page Font resources");
    let dict = document
        .resolve(&document.get_key(fonts, "F1"))
        .as_dict()
        .expect("F1 is a font dictionary")
        .clone();
    let loaded = LoadedFont::load(&document, &dict, "F1").expect("embedded TTF loads");
    let quote = Code::single_byte(39);
    let letter = Code::single_byte(40);
    assert_eq!(
        loaded.glyph_index(quote),
        Some(527),
        "quoteright outline GID"
    );
    assert_eq!(loaded.glyph_index(letter), Some(36), "A outline GID");
    assert!((loaded.advance(quote) - 0.222_167_97).abs() < 0.000_001);
    assert!((loaded.advance(letter) - 0.666_992_2).abs() < 0.000_001);
}

#[test]
fn direct_differences_are_the_positive_control() {
    assert_consumer_result(false);
}

#[test]
fn indirect_integer_and_name_objects_preserve_differences_glyph_selection() {
    assert_consumer_result(true);
}

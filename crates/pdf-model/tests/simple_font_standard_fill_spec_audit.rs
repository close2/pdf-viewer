//! Exact-xref public-consumer regression proposal for TrueType §9.6.5.4 `StandardEncoding` fill.
//!
//! The
//! checked-in full Liberation Sans TTF is the deterministic program. `StandardEncoding` code 39
//! is `quoteright` (U+2019, GID 527), whereas `WinAnsi` code 39 is `quotesingle` (U+0027, GID 10).
//! Both glyphs exist in the actual embedded sfnt and have distinct outlines and advances.
//! Code 40 is explicitly `/parenright` in the target to prove the Differences array is active
//! without overriding code 39. The same code 40 is the named-base control.
//!
//! Metrics independently read from the included TTF with `fontTools` (2048 UPM):
//! SHA256 f8ace1f892b2bd9dc1792ba7f097fa7588f84fed48321480e04de5390828221f;
//! PostScript name `LiberationSans`; `head` box [-416,-621,2151,1864] font units, scaled
//! outward to [-204,-304,1051,911]; OS/2 typo ascender/descent/cap-height 1491/-431/1409,
//! scaled and rounded outward to [729,-211,688]. Glyph advances at PDF 1000-unit scale:
//! quoteright 222.167→222, quotesingle 190.918→191, parenright 333.008→333.
//! `StemV` is a descriptor characteristic without an SFNT source field; 80 is the fixture's
//! stated approximate descriptor value, not claimed as a measured TTF table value.

#![expect(
    clippy::indexing_slicing,
    clippy::expect_used,
    reason = "the exact-xref integration fixture asserts the parser and selected glyph directly"
)]

use pdf_model::Pages;
use pdf_syntax::Document;

const LIBERATION: &[u8] = include_bytes!("../../../data/standard-fonts/LiberationSans-Regular.ttf");

#[derive(Clone, Copy)]
enum Route {
    StandardFill,
    WinAnsiControl,
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

fn pdf(route: Route) -> Vec<u8> {
    let mut output = b"%PDF-1.7\n%\xE2\xE3\xCF\xD3\n".to_vec();
    let mut offsets = vec![0usize; 9];
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
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Resources << /Font << /F1 4 0 R >> >> /Contents 8 0 R >>",
    );

    let (encoding, widths): (&[u8], &[u8]) = match route {
        // A Differences code other than 39 keeps the 39 slot genuinely inherited from
        // StandardEncoding while demonstrating that the Encoding dictionary is in effect.
        Route::StandardFill => (
            b"/Encoding << /Differences [40 /parenright] >>",
            b"[222 333]",
        ),
        Route::WinAnsiControl => (
            b"/Encoding << /BaseEncoding /WinAnsiEncoding /Differences [40 /parenright] >>",
            b"[191 333]",
        ),
    };
    let mut font_dictionary = b"<< /Type /Font /Subtype /TrueType /BaseFont /LiberationSans /FirstChar 39 /LastChar 40 /Widths ".to_vec();
    font_dictionary.extend_from_slice(widths);
    font_dictionary.push(b' ');
    font_dictionary.extend_from_slice(encoding);
    font_dictionary.extend_from_slice(b" /FontDescriptor 5 0 R >>");
    push_object(&mut output, &mut offsets, 4, &font_dictionary);

    // Descriptor values derive from this included program. The rounded-out FontBBox encloses
    // its exact scaled `head` bounds; vertical values round the `OS/2` typo metrics outward.
    push_object(
        &mut output,
        &mut offsets,
        5,
        b"<< /Type /FontDescriptor /FontName /LiberationSans /Flags 32 /FontBBox [-204 -304 1051 911] /ItalicAngle 0 /Ascent 729 /Descent -211 /CapHeight 688 /StemV 80 /FontFile2 6 0 R >>",
    );
    let file_dict = format!("/Length1 {}", LIBERATION.len());
    let font_file = stream_body(file_dict.as_bytes(), LIBERATION);
    push_object(&mut output, &mut offsets, 6, &font_file);
    push_object(&mut output, &mut offsets, 7, b"null");

    // Codes 39 and 40 have independently checked descriptor widths above.
    let content = stream_body(b"", b"BT /F1 20 Tf 1 0 0 1 10 40 Tm <2728> Tj ET\n");
    push_object(&mut output, &mut offsets, 8, &content);

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

fn selected_glyphs(route: Route) -> Vec<u16> {
    let document = Document::open(pdf(route)).expect("valid exact-xref PDF");
    assert!(!document.was_recovered(), "fixture uses exact xref offsets");
    let page = Pages::new(&document).get(0).expect("one page");
    let font_resources = document.get_key(&page.resources, "Font");
    let fonts = font_resources.as_dict().expect("Font resources");
    let font_dictionary = document
        .resolve(&document.get_key(fonts, "F1"))
        .as_dict()
        .expect("F1 dictionary")
        .clone();
    let loaded = pdf_font::LoadedFont::load(&document, &font_dictionary, "F1")
        .expect("embedded TrueType program loads");
    [39u8, 40]
        .into_iter()
        .map(|byte| {
            let code = pdf_font::cmap::Code::single_byte(byte);
            let outline = loaded.outline(code).expect("selected glyph has an outline");
            assert!(!outline.commands().is_empty(), "selected glyph is nonempty");
            loaded.glyph_index(code).expect("code selects a glyph")
        })
        .collect()
}

#[test]
fn simple_truetype_differences_fill_undefined_slot_from_standard_encoding() {
    // The positive control distinguishes WinAnsi lookup from the StandardEncoding fill:
    // WinAnsi code 39 is quotesingle (GID 10); code 40 is explicitly parenright (GID 12).
    assert_eq!(selected_glyphs(Route::WinAnsiControl), [10, 12]);
    // Table 112 allows an Encoding dictionary and optional BaseEncoding/Differences. §9.6.5.4
    // fills an undefined Differences slot from StandardEncoding, making code 39 quoteright GID
    // 527. A byte-to-Unicode or WinAnsi fallback would instead select GID 10. Code 40 confirms
    // the same Differences array explicitly resolves parenright to GID 12.
    assert_eq!(selected_glyphs(Route::StandardFill), [527, 12]);
}

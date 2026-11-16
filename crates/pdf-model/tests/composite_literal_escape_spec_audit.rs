//! EC3 #550: literal-string syntax precedes composite-font code segmentation.

#![expect(
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    reason = "small exact-xref PDF fixture and literal independent character expectation"
)]

use std::io::Write as _;

use pdf_syntax::Document;

const PROGRAM: &[u8] = include_bytes!("../../../data/standard-fonts/LiberationSans-Regular.ttf");

fn stream(dictionary: &str, data: &[u8]) -> Vec<u8> {
    let mut body = format!("<< {dictionary} /Length {} >>\nstream\n", data.len()).into_bytes();
    body.extend_from_slice(data);
    body.extend_from_slice(b"\nendstream");
    body
}

fn pdf(page_content: &str) -> Vec<u8> {
    let encoding = "/CIDInit /ProcSet findresource begin 12 dict begin begincmap\n\
        /CIDSystemInfo << /Registry (Adobe) /Ordering (Identity) /Supplement 0 >> def\n\
        /CMapName /EscapedCode def /CMapType 1 def\n\
        1 begincodespacerange <0041> <0041> endcodespacerange\n\
        1 begincidchar <0041> 36 endcidchar\n\
        endcmap CMapName currentdict /CMap defineresource pop end end";
    let to_unicode = "/CIDInit /ProcSet findresource begin 12 dict begin begincmap\n\
        /CIDSystemInfo << /Registry (Adobe) /Ordering (UCS) /Supplement 0 >> def\n\
        /CMapName /EscapedUnicode def /CMapType 2 def\n\
        1 begincodespacerange <0041> <0041> endcodespacerange\n\
        1 beginbfchar <0041> <0041> endbfchar\n\
        endcmap CMapName currentdict /CMap defineresource pop end end";

    let objects = vec![
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Resources << /Font << /F1 4 0 R >> >> /Contents 9 0 R >>".to_vec(),
        b"<< /Type /Font /Subtype /Type0 /BaseFont /LiberationSans /Encoding 5 0 R /DescendantFonts [6 0 R] /ToUnicode 8 0 R >>".to_vec(),
        stream("/Type /CMap /CMapName /EscapedCode /CIDSystemInfo << /Registry (Adobe) /Ordering (Identity) /Supplement 0 >>", encoding.as_bytes()),
        b"<< /Type /Font /Subtype /CIDFontType2 /BaseFont /LiberationSans /CIDSystemInfo << /Registry (Adobe) /Ordering (Identity) /Supplement 0 >> /FontDescriptor 7 0 R /CIDToGIDMap /Identity /DW 1000 /W [36 [666.9921875]] >>".to_vec(),
        b"<< /Type /FontDescriptor /FontName /LiberationSans /Flags 32 /FontBBox [-203.125 -303.22265625 1050.29296875 910.15625] /ItalicAngle 0 /Ascent 728.02734375 /Descent -210.44921875 /CapHeight 687.98828125 /StemV 80 /FontFile2 10 0 R >>".to_vec(),
        stream("/Type /CMap /CMapName /EscapedUnicode /CIDSystemInfo << /Registry (Adobe) /Ordering (UCS) /Supplement 0 >>", to_unicode.as_bytes()),
        stream("", page_content.as_bytes()),
        stream(&format!("/Length1 {}", PROGRAM.len()), PROGRAM),
    ];

    let mut out = b"%PDF-1.7\n%\xE2\xE3\xCF\xD3\n".to_vec();
    let mut offsets = Vec::new();
    for (index, object) in objects.iter().enumerate() {
        offsets.push(out.len());
        out.extend_from_slice(format!("{} 0 obj\n", index + 1).as_bytes());
        out.extend_from_slice(object);
        out.extend_from_slice(b"\nendobj\n");
    }
    let xref = out.len();
    let size = objects.len() + 1;
    let _ = write!(out, "xref\n0 {size}\n0000000000 65535 f \n");
    for offset in offsets {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    let _ = write!(
        out,
        "trailer\n<< /Size {size} /Root 1 0 R /ID [<00112233445566778899AABBCCDDEEFF> <00112233445566778899AABBCCDDEEFF>] >>\nstartxref\n{xref}\n%%EOF\n"
    );
    out
}

fn observe(page_content: &str) -> (String, usize, usize, f32) {
    let document = Document::open(pdf(page_content)).expect("fixture parses");
    assert!(!document.was_recovered(), "all xref offsets are exact");
    let page = pdf_model::Pages::new(&document)
        .get(0)
        .expect("page one exists");
    let result = pdf_model::interpret(&document, &page);
    assert!(result.is_complete(), "{:?}", result.unsupported);
    let first = result
        .text_layer
        .first()
        .expect("one mapped glyph is placed");
    let advance = first.quad[2] - first.quad[0];
    (result.text, result.glyphs, result.text_layer.len(), advance)
}

#[test]
fn literal_string_escapes_are_resolved_before_two_byte_cmap_segmentation() {
    // In PDF syntax this literal operand is `(\000A)`: octal escape 000 contributes byte 00,
    // followed by ordinary byte 41. The resulting two-byte code is <0041>, mapped independently
    // by the CMap and ToUnicode CMap to CID 36 and the literal expected character A.
    let literal = observe("BT /F1 12 Tf 20 60 Td (\\000A) Tj ET");
    assert_eq!(
        literal.0, "A",
        "the independent ToUnicode mapping names code 0041"
    );
    assert_eq!(literal.1, 1, "the code reaches one painted glyph");
    assert_eq!(literal.2, 1, "the text layer has one source-code placement");
    assert!(
        (literal.3 - 8.003_906).abs() < 0.0001,
        "CID 36 has /W 666.9921875 at 12 pt: advance {:?}",
        literal.3
    );

    // Hexadecimal string control states the same two bytes without literal-string escapes.
    let hex_control = observe("BT /F1 12 Tf 20 60 Td <0041> Tj ET");
    assert_eq!(hex_control.0, "A");
    assert_eq!(hex_control.1, 1);
    assert_eq!(hex_control.2, 1);
    assert!((hex_control.3 - 8.003_906).abs() < 0.0001);
}

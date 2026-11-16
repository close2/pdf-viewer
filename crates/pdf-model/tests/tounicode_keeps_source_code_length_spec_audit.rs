//! §9.10.3 source codes include their byte length: <01> and <0001> are distinct codes.
#![expect(
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    reason = "small complete exact-xref PDF fixture with literal independent text expectations"
)]

use std::io::Write as _;

use pdf_syntax::Document;

const PROGRAM: &[u8] = include_bytes!("../../../data/standard-fonts/LiberationSans-Regular.ttf");

fn stream(dictionary: &str, bytes: &str) -> Vec<u8> {
    format!(
        "<< {dictionary} /Length {} >>\nstream\n{bytes}\nendstream",
        bytes.len() + 1
    )
    .into_bytes()
}

fn binary_stream(dictionary: &str, bytes: &[u8]) -> Vec<u8> {
    let mut body = format!("<< {dictionary} /Length {} >>\nstream\n", bytes.len()).into_bytes();
    body.extend_from_slice(bytes);
    body.extend_from_slice(b"\nendstream");
    body
}

fn pdf(encoding: &str, to_unicode: &str, page_content: &str) -> Vec<u8> {
    let objects = vec![
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Resources << /Font << /F1 4 0 R >> >> /Contents 9 0 R >>".to_vec(),
        b"<< /Type /Font /Subtype /Type0 /BaseFont /LiberationSans /Encoding 5 0 R /DescendantFonts [6 0 R] /ToUnicode 8 0 R >>".to_vec(),
        stream("/Type /CMap /CMapName /Probe-Codes /CIDSystemInfo << /Registry (Adobe) /Ordering (Identity) /Supplement 0 >>", encoding),
        b"<< /Type /Font /Subtype /CIDFontType2 /BaseFont /LiberationSans /CIDSystemInfo << /Registry (Adobe) /Ordering (Identity) /Supplement 0 >> /FontDescriptor 7 0 R /CIDToGIDMap /Identity /DW 666.9921875 /W [36 [666.9921875]] >>".to_vec(),
        b"<< /Type /FontDescriptor /FontName /LiberationSans /Flags 32 /FontBBox [-203.125 -303.22265625 1050.29296875 910.15625] /ItalicAngle 0 /Ascent 728.02734375 /Descent -210.44921875 /CapHeight 687.98828125 /StemV 80 /FontFile2 10 0 R >>".to_vec(),
        stream("/Type /CMap /CMapName /Probe-UCS /CIDSystemInfo << /Registry (Adobe) /Ordering (UCS) /Supplement 0 >>", to_unicode),
        stream("", page_content),
        binary_stream(&format!("/Length1 {}", PROGRAM.len()), PROGRAM),
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
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n"
    );
    out
}

fn encoding(code_space: &str, mappings: &str) -> String {
    format!(
        "/CIDInit /ProcSet findresource begin 12 dict begin begincmap\n\
         /CIDSystemInfo << /Registry (Adobe) /Ordering (Identity) /Supplement 0 >> def\n\
         /CMapName /Probe-Codes def /CMapType 1 def\n\
         {code_space}\n{mappings}\n\
         endcmap CMapName currentdict /CMap defineresource pop end end"
    )
}

fn to_unicode(code_space: &str, mappings: &str) -> String {
    format!(
        "/CIDInit /ProcSet findresource begin 12 dict begin begincmap\n\
         /CIDSystemInfo << /Registry (Adobe) /Ordering (UCS) /Supplement 0 >> def\n\
         /CMapName /Probe-UCS def /CMapType 2 def\n\
         {code_space}\n{mappings}\n\
         endcmap CMapName currentdict /CMap defineresource pop end end"
    )
}

fn text(pdf: Vec<u8>) -> String {
    let document = Document::open(pdf).expect("fixture parses");
    assert!(!document.was_recovered(), "all xref offsets are exact");
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    {
        let result = pdf_model::interpret(&document, &page);
        assert!(result.is_complete(), "{:?}", result.unsupported);
        result.text
    }
}

#[test]
fn one_byte_tounicode_control_extracts_its_code() {
    let codes = encoding(
        "1 begincodespacerange <01> <01> endcodespacerange",
        "1 begincidchar <01> 36 endcidchar",
    );
    let unicode = to_unicode(
        "1 begincodespacerange <01> <01> endcodespacerange",
        "1 beginbfchar <01> <0041> endbfchar",
    );
    assert_eq!(
        text(pdf(&codes, &unicode, "BT /F1 12 Tf 20 60 Td <01> Tj ET")),
        "A"
    );
}

#[test]
fn two_byte_tounicode_control_extracts_its_code() {
    let codes = encoding(
        "1 begincodespacerange <0001> <0001> endcodespacerange",
        "1 begincidchar <0001> 36 endcidchar",
    );
    let unicode = to_unicode(
        "1 begincodespacerange <0001> <0001> endcodespacerange",
        "1 beginbfchar <0001> <03A9> endbfchar",
    );
    assert_eq!(
        text(pdf(&codes, &unicode, "BT /F1 12 Tf 20 60 Td <0001> Tj ET")),
        "Ω"
    );
}

#[test]
fn to_unicode_distinguishes_equal_values_at_different_source_code_lengths() {
    let codes = encoding(
        "2 begincodespacerange <01> <01> <0001> <0002> endcodespacerange",
        "3 begincidchar <01> 36 <0001> 36 <0002> 36 endcidchar",
    );
    let unicode = to_unicode(
        "2 begincodespacerange <01> <01> <0001> <0002> endcodespacerange",
        "3 beginbfchar <0001> <03A9> <01> <0041> <0002> <03B2> endbfchar",
    );
    // The independent expected values are literal UTF-16BE destinations: <01>→A,
    // <0001>→Ω, and <0002>→β. The one-byte and two-byte codes intentionally share value 1.
    let bytes = pdf(&codes, &unicode, "BT /F1 12 Tf 20 60 Td <0100010002> Tj ET");
    let document = Document::open(bytes.clone()).expect("fixture parses");
    let dictionary = document.get(pdf_syntax::ObjectId {
        number: 4,
        generation: 0,
    });
    let font = pdf_font::LoadedFont::load(&document, dictionary.as_dict().expect("Type0"), "F1")
        .expect("embedded font loads");
    let decoded = font.decode(&[1, 0, 1, 0, 2]);
    assert_eq!(
        decoded
            .iter()
            .map(|code| (code.value(), code.length()))
            .collect::<Vec<_>>(),
        [(1, 1), (1, 2), (2, 2)],
        "encoding segmentation preserves both lengths before Unicode lookup"
    );
    assert!(
        decoded
            .iter()
            .all(|code| font.glyph_index(*code) == Some(36)),
        "all source codes reach the stated embedded glyph"
    );
    assert_eq!(text(bytes), "AΩβ");
}

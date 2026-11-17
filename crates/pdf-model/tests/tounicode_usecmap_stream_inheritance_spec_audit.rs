//! §9.10.3 `/UseCMap` dictionary reference: local mappings override, missing ones inherit.
#![expect(
    clippy::arithmetic_side_effects,
    reason = "small complete exact-xref PDF fixture with literal UTF-16BE destinations"
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

fn pdf() -> Vec<u8> {
    let encoding = "\
        /CIDInit /ProcSet findresource begin 12 dict begin begincmap\n\
        /CIDSystemInfo << /Registry (Adobe) /Ordering (Identity) /Supplement 0 >> def\n\
        /CMapName /Probe-Codes def /CMapType 1 def\n\
        1 begincodespacerange <0000> <FFFF> endcodespacerange\n\
        2 begincidchar <0001> 36 <0002> 36 endcidchar\n\
        endcmap CMapName currentdict /CMap defineresource pop end end";
    let base = "\
        /CIDInit /ProcSet findresource begin 12 dict begin begincmap\n\
        /CIDSystemInfo << /Registry (Adobe) /Ordering (UCS) /Supplement 0 >> def\n\
        /CMapName /Probe-UCS-Base def /CMapType 2 def\n\
        1 begincodespacerange <0000> <FFFF> endcodespacerange\n\
        2 beginbfchar <0001> <0041> <0002> <0042> endbfchar\n\
        endcmap CMapName currentdict /CMap defineresource pop end end";
    let child = "\
        /CIDInit /ProcSet findresource begin 12 dict begin begincmap\n\
        /CIDSystemInfo << /Registry (Adobe) /Ordering (UCS) /Supplement 0 >> def\n\
        /CMapName /Probe-UCS-Child def /CMapType 2 def\n\
        1 begincodespacerange <0000> <FFFF> endcodespacerange\n\
        1 beginbfchar <0002> <03A9> endbfchar\n\
        endcmap CMapName currentdict /CMap defineresource pop end end";
    let page_content = "BT /F1 12 Tf 20 60 Td <00010002> Tj ET";
    let objects = vec![
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Resources << /Font << /F1 4 0 R >> >> /Contents 10 0 R >>".to_vec(),
        b"<< /Type /Font /Subtype /Type0 /BaseFont /LiberationSans /Encoding 5 0 R /DescendantFonts [6 0 R] /ToUnicode 8 0 R >>".to_vec(),
        stream("/Type /CMap /CMapName /Probe-Codes /CIDSystemInfo << /Registry (Adobe) /Ordering (Identity) /Supplement 0 >>", encoding),
        b"<< /Type /Font /Subtype /CIDFontType2 /BaseFont /LiberationSans /CIDSystemInfo << /Registry (Adobe) /Ordering (Identity) /Supplement 0 >> /FontDescriptor 7 0 R /CIDToGIDMap /Identity /DW 666.9921875 /W [36 [666.9921875]] >>".to_vec(),
        b"<< /Type /FontDescriptor /FontName /LiberationSans /Flags 32 /FontBBox [-203.125 -303.22265625 1050.29296875 910.15625] /ItalicAngle 0 /Ascent 728.02734375 /Descent -210.44921875 /CapHeight 687.98828125 /StemV 80 /FontFile2 11 0 R >>".to_vec(),
        stream("/Type /CMap /CMapName /Probe-UCS-Child /CIDSystemInfo << /Registry (Adobe) /Ordering (UCS) /Supplement 0 >> /UseCMap 9 0 R", child),
        stream("/Type /CMap /CMapName /Probe-UCS-Base /CIDSystemInfo << /Registry (Adobe) /Ordering (UCS) /Supplement 0 >>", base),
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

#[test]
fn to_unicode_dictionary_usecmap_inherits_and_local_mapping_wins() {
    let document = Document::open(pdf()).expect("fixture parses");
    assert!(!document.was_recovered(), "all xref offsets are exact");
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    let interpretation = pdf_model::interpret(&document, &page);
    assert!(
        interpretation.is_complete(),
        "{:?}",
        interpretation.unsupported
    );
    // The base's <0001>→A survives; the child's <0002>→Ω overrides base <0002>→B.
    assert_eq!(interpretation.text, "AΩ");
}

//! Table 118 Encoding-CMap `/UseCMap` stream-object integration control.
//! The matching in-file `usecmap` name and dictionary stream refer to the same base `CMap`.

#![expect(
    clippy::indexing_slicing,
    reason = "small fixed exact-xref PDF fixture with fixture-derived offsets"
)]

use std::io::Write as _;

use pdf_render::Command;
use pdf_syntax::Document;

const TTF: &[u8] = include_bytes!("../../../data/standard-fonts/LiberationSans-Regular.ttf");

fn object(number: usize, body: &[u8], out: &mut Vec<u8>, offsets: &mut [usize]) {
    offsets[number] = out.len();
    out.extend_from_slice(format!("{number} 0 obj\n").as_bytes());
    out.extend_from_slice(body);
    out.extend_from_slice(b"\nendobj\n");
}

fn stream(dictionary: &str, bytes: &[u8]) -> Vec<u8> {
    let mut body = format!("<< {dictionary} /Length {} >>\nstream\n", bytes.len()).into_bytes();
    body.extend_from_slice(bytes);
    body.extend_from_slice(b"\nendstream");
    body
}

fn fixture() -> Vec<u8> {
    // The base states the codespace and A/B -> CID 36. The child inherits both, then
    // overrides only B -> CID 37. LiberationSans GID/CID 36 is A and 37 is B.
    let base = b"/CIDInit /ProcSet findresource begin 12 dict begin begincmap\n\
        /CIDSystemInfo << /Registry (Adobe) /Ordering (Identity) /Supplement 0 >> def\n\
        /CMapName /Audit-Base def /CMapType 1 def\n\
        1 begincodespacerange <41> <42> endcodespacerange\n\
        2 begincidchar <41> 36 <42> 36 endcidchar\n\
        endcmap CMapName currentdict /CMap defineresource pop end end";
    let child = b"/CIDInit /ProcSet findresource begin 12 dict begin begincmap\n\
        /CIDSystemInfo << /Registry (Adobe) /Ordering (Identity) /Supplement 0 >> def\n\
        /CMapName /Audit-Child def /CMapType 1 def\n\
        /Audit-Base usecmap\n\
        1 begincidchar <42> 37 endcidchar\n\
        endcmap CMapName currentdict /CMap defineresource pop end end";
    let content = stream("", b"BT /F1 12 Tf 10 50 Td <4142> Tj ET");
    let child_stream = stream(
        "/Type /CMap /CMapName /Audit-Child \
         /CIDSystemInfo << /Registry (Adobe) /Ordering (Identity) /Supplement 0 >> \
         /UseCMap 7 0 R",
        child,
    );
    let base_stream = stream(
        "/Type /CMap /CMapName /Audit-Base \
         /CIDSystemInfo << /Registry (Adobe) /Ordering (Identity) /Supplement 0 >>",
        base,
    );
    let cidfont = b"<< /Type /Font /Subtype /CIDFontType2 /BaseFont /LiberationSans \
        /CIDSystemInfo << /Registry (Adobe) /Ordering (Identity) /Supplement 0 >> \
        /FontDescriptor 9 0 R /CIDToGIDMap /Identity /DW 666.9921875 \
        /W [36 [666.9921875 666.9921875]] >>";
    let descriptor = b"<< /Type /FontDescriptor /FontName /LiberationSans /Flags 32 \
        /FontBBox [-203.125 -303.22265625 1050.29296875 910.15625] \
        /ItalicAngle 0 /Ascent 905 /Descent -212 /CapHeight 729 /StemV 80 \
        /FontFile2 10 0 R >>";
    let type0 = b"<< /Type /Font /Subtype /Type0 /BaseFont /LiberationSans \
        /Encoding 6 0 R /DescendantFonts [8 0 R] >>";
    let page = b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 100] \
        /Resources << /Font << /F1 5 0 R >> >> /Contents 4 0 R >>";
    let program = stream(&format!("/Length1 {}", TTF.len()), TTF);

    let mut out = b"%PDF-1.7\n%\xE2\xE3\xCF\xD3\n".to_vec();
    let mut offsets = vec![0_usize; 11];
    object(
        1,
        b"<< /Type /Catalog /Pages 2 0 R >>",
        &mut out,
        &mut offsets,
    );
    object(
        2,
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
        &mut out,
        &mut offsets,
    );
    object(3, page, &mut out, &mut offsets);
    object(4, &content, &mut out, &mut offsets);
    object(5, type0, &mut out, &mut offsets);
    object(6, &child_stream, &mut out, &mut offsets);
    object(7, &base_stream, &mut out, &mut offsets);
    object(8, cidfont, &mut out, &mut offsets);
    object(9, descriptor, &mut out, &mut offsets);
    object(10, &program, &mut out, &mut offsets);

    let xref = out.len();
    let _ = writeln!(out, "xref\n0 {}", offsets.len());
    out.extend_from_slice(b"0000000000 65535 f \n");
    for offset in offsets.iter().skip(1) {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    let _ = write!(
        out,
        "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
        offsets.len()
    );
    out
}

#[test]
fn encoding_cmap_stream_usecmap_inherits_and_overrides_through_type0_page() {
    let document = Document::open(fixture()).expect("valid exact-xref PDF opens");
    assert!(!document.was_recovered(), "fixture xref offsets are exact");
    let page = pdf_model::Pages::new(&document).get(0).expect("one page");
    let result = pdf_model::interpret(&document, &page);
    assert!(
        result.is_complete(),
        "embedded font page: {:?}",
        result.unsupported
    );
    assert_eq!(result.glyphs, 2, "both mapped characters are interpreted");
    assert_eq!(
        result
            .display_list
            .commands()
            .iter()
            .filter(|command| matches!(command, Command::Fill { .. }))
            .count(),
        2,
        "both independent LiberationSans outlines reach the public page display list"
    );

    let fonts = document.get_key(&page.resources, "Font");
    let root = document.resolve(&document.get_key(fonts.as_dict().expect("font resources"), "F1"));
    let font = pdf_font::LoadedFont::load(&document, root.as_dict().expect("Type 0 root"), "F1")
        .expect("valid stream-valued Encoding UseCMap loads");
    let codes = font.decode(b"AB");
    assert_eq!(codes.len(), 2);
    assert_eq!(
        font.glyph_index(codes[0]),
        Some(36),
        "base's A mapping is inherited"
    );
    assert_eq!(
        font.glyph_index(codes[1]),
        Some(37),
        "child overrides base's B mapping"
    );
    assert!(
        font.outline(codes[0]).is_some(),
        "GID 36 has a real outline"
    );
    assert!(
        font.outline(codes[1]).is_some(),
        "GID 37 has a real outline"
    );
}

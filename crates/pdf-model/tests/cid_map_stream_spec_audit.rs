//! Table 115 and §9.7.6.3: fixed CID map byte order and missing-entry fallback.
//! Known A/B GIDs in the checked-in font provide a numeric oracle independent of the loader.

#![expect(
    clippy::indexing_slicing,
    reason = "small fixed PDF fixture with exact xref"
)]

use std::io::Write as _;

use pdf_syntax::Document;

const TTF: &[u8] = include_bytes!("../../../data/standard-fonts/LiberationSans-Regular.ttf");

fn object(number: usize, body: &[u8], out: &mut Vec<u8>, offsets: &mut [usize]) {
    offsets[number] = out.len();
    out.extend_from_slice(format!("{number} 0 obj\n").as_bytes());
    out.extend_from_slice(body);
    out.extend_from_slice(b"\nendobj\n");
}

fn stream(dictionary: &[u8], data: &[u8]) -> Vec<u8> {
    let mut body = format!(
        "<< {} /Length {} >>\nstream\n",
        String::from_utf8_lossy(dictionary),
        data.len()
    )
    .into_bytes();
    body.extend_from_slice(data);
    body.extend_from_slice(b"\nendstream");
    body
}

fn fixture() -> Vec<u8> {
    // Table 115: bytes 0 and 1 encode CID 0 -> GID 36 (0x0024); bytes 10 and 11
    // encode CID 5 -> GID 37 (0x0025). The map ends after CID 5, so CID 6 has no GID.
    let mut map = vec![0u8; 12];
    map[0..2].copy_from_slice(&36u16.to_be_bytes());
    map[10..12].copy_from_slice(&37u16.to_be_bytes());
    let content = stream(b"", b"BT /F1 12 Tf 10 50 Td <00050006> Tj ET");
    let descriptor = b"<< /Type /FontDescriptor /FontName /LiberationSans /Flags 32 /FontBBox [-203.125 -303.22265625 1050.29296875 910.15625] /ItalicAngle 0 /Ascent 905 /Descent -212 /CapHeight 729 /StemV 80 /FontFile2 8 0 R >>";
    let cidfont = b"<< /Type /Font /Subtype /CIDFontType2 /BaseFont /LiberationSans /CIDSystemInfo << /Registry (Adobe) /Ordering (Identity) /Supplement 0 >> /FontDescriptor 7 0 R /DW 666.9921875 /W [5 [666.9921875 666.9921875]] /CIDToGIDMap 9 0 R >>";
    let type0 = b"<< /Type /Font /Subtype /Type0 /BaseFont /LiberationSans /Encoding /Identity-H /DescendantFonts [6 0 R] >>";
    let page = b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 100] /Resources << /Font << /F1 5 0 R >> >> /Contents 4 0 R >>";

    let mut out = b"%PDF-1.7\n".to_vec();
    let mut offsets = vec![0usize; 10];
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
    object(6, cidfont, &mut out, &mut offsets);
    object(7, descriptor, &mut out, &mut offsets);
    object(
        8,
        &stream(format!("/Length1 {}", TTF.len()).as_bytes(), TTF),
        &mut out,
        &mut offsets,
    );
    object(9, &stream(b"", &map), &mut out, &mut offsets);

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
fn cid_map_reads_big_endian_and_missing_entry_uses_cid_zero() {
    let document = Document::open(fixture()).expect("PDF fixture parses");
    assert!(!document.was_recovered(), "all xref offsets are exact");
    let page = pdf_model::Pages::new(&document).get(0).expect("one page");
    let result = pdf_model::interpret(&document, &page);
    assert!(
        result.is_complete(),
        "valid fixture: {:?}",
        result.unsupported
    );
    assert_eq!(result.glyphs, 2, "both selected codes are interpreted");
    assert_eq!(
        result
            .display_list
            .commands()
            .iter()
            .filter(|command| matches!(command, pdf_render::Command::Fill { .. }))
            .count(),
        2,
        "both real outlines reach the page display list"
    );

    let fonts = document.get_key(&page.resources, "Font");
    let font_dict =
        document.resolve(&document.get_key(fonts.as_dict().expect("Font resources"), "F1"));
    let loaded = pdf_font::LoadedFont::load(
        &document,
        font_dict.as_dict().expect("Type 0 dictionary"),
        "F1",
    )
    .expect("embedded Type 2 font loads");
    let codes = loaded.decode(&[0x00, 0x05, 0x00, 0x06]);
    assert_eq!(codes.len(), 2);
    assert_eq!(
        loaded.glyph_index(codes[0]),
        Some(37),
        "CID 5 reads bytes 10-11 as big-endian 0x0025"
    );
    assert_eq!(
        loaded.glyph_index(codes[1]),
        Some(36),
        "unmapped CID 6 substitutes the glyph selected by CID 0"
    );
    assert!(
        loaded.outline(codes[0]).is_some(),
        "mapped positive control has a real outline"
    );
    assert!(
        loaded.outline(codes[1]).is_some(),
        "CID-0 fallback control has a real outline"
    );
}

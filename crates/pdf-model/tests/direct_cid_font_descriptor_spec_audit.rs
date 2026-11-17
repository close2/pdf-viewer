//! Valid public-consumer witness for the Table 115 `/FontDescriptor` cell after EC3 #106.
//! The indirect case is a same-fixture control; the direct descriptor retains an indirect
//! `FontFile2` stream and otherwise has identical font bytes, mappings, and metric values.

#![expect(
    clippy::indexing_slicing,
    reason = "small fixed PDF fixture with exact classic-xref offsets"
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

fn fixture(direct_descriptor: bool) -> Vec<u8> {
    // CID 5 -> GID 37; zero bytes leave CID 0 as .notdef (not used by this fixture).
    let mut map = vec![0_u8; 12];
    map[10..12].copy_from_slice(&37_u16.to_be_bytes());
    let content = stream(b"", b"BT /F1 12 Tf 10 50 Td <0005> Tj ET");
    let descriptor = b"<< /Type /FontDescriptor /FontName /LiberationSans /Flags 32 \
          /FontBBox [-203.125 -303.22265625 1050.29296875 910.15625] \
          /ItalicAngle 0 /Ascent 905 /Descent -212 /CapHeight 729 /StemV 80 \
          /FontFile2 8 0 R >>";
    let cidfont = if direct_descriptor {
        let mut body = b"<< /Type /Font /Subtype /CIDFontType2 /BaseFont /LiberationSans \
               /CIDSystemInfo << /Registry (Adobe) /Ordering (Identity) /Supplement 0 >> \
               /FontDescriptor "
            .to_vec();
        body.extend_from_slice(descriptor);
        body.extend_from_slice(b" /DW 666.9921875 /W [5 [666.9921875]] /CIDToGIDMap 9 0 R >>");
        body
    } else {
        b"<< /Type /Font /Subtype /CIDFontType2 /BaseFont /LiberationSans \
           /CIDSystemInfo << /Registry (Adobe) /Ordering (Identity) /Supplement 0 >> \
           /FontDescriptor 7 0 R /DW 666.9921875 /W [5 [666.9921875]] \
           /CIDToGIDMap 9 0 R >>"
            .to_vec()
    };
    let type0 = b"<< /Type /Font /Subtype /Type0 /BaseFont /LiberationSans /Encoding /Identity-H \
           /DescendantFonts [6 0 R] >>";
    let page = b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 100] \
           /Resources << /Font << /F1 5 0 R >> >> /Contents 4 0 R >>";

    let mut out = b"%PDF-1.7\n%\xE2\xE3\xCF\xD3\n".to_vec();
    let mut offsets = vec![0_usize; 10];
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
    object(6, &cidfont, &mut out, &mut offsets);
    // Keep the same object set in both cases; the direct case leaves this copy unused.
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
fn direct_and_indirect_cidfont_descriptors_load_the_same_embedded_program() {
    for direct in [false, true] {
        let document = Document::open(fixture(direct)).expect("valid PDF opens");
        assert!(!document.was_recovered(), "fixture has exact xref offsets");
        let page = pdf_model::Pages::new(&document).get(0).expect("one page");
        let interpreted = pdf_model::interpret(&document, &page);
        assert!(
            interpreted.is_complete(),
            "valid font page: {:?}",
            interpreted.unsupported
        );
        assert_eq!(interpreted.glyphs, 1, "the mapped glyph reaches the page");
        assert_eq!(
            interpreted
                .display_list
                .commands()
                .iter()
                .filter(|command| matches!(command, pdf_render::Command::Fill { .. }))
                .count(),
            1,
            "the embedded glyph outline reaches the page display list"
        );

        let fonts = document.get_key(&page.resources, "Font");
        let font_dict =
            document.resolve(&document.get_key(fonts.as_dict().expect("Font resources"), "F1"));
        let loaded =
            pdf_font::LoadedFont::load(&document, font_dict.as_dict().expect("Type 0 font"), "F1")
                .expect("CIDFontType2 with direct or indirect descriptor loads");
        let code = loaded
            .decode(&[0x00, 0x05])
            .into_iter()
            .next()
            .expect("one CID");
        assert_eq!(
            loaded.glyph_index(code),
            Some(37),
            "Table 115 map selects GID 37"
        );
        assert!(
            loaded.outline(code).is_some(),
            "the embedded font supplies a real outline"
        );
        assert!(
            (loaded.advance(code) - 0.666_992_2).abs() < 0.000_001,
            "W advance should match LiberationSans GID 37's 1366/2048-em hmtx width"
        );
    }
}

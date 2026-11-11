//! Complete-PDF tests for the four OpenType glyph-selection bullets in §9.9.1.
//!
//! The fixtures embed repository
//! font programs and assert selected glyph IDs, not a count of codes decoded or glyphs drawn.
//! Synthetic font sources and a reproducible offline generator live in `fixtures/opentype-selection`.
//! CFF GIDs are `two`=1, `two.sups`=2, A=3 and B=4; glyf GIDs are A=1 and B=2.
//! The CFF font's version-3 `post` has no names. Its suffixed name must reach the CFF charset;
//! suffix-stripping would incorrectly select `two`. A Macintosh cmap also offers A/B at
//! the PDF bytes, exposing incorrect use of the TrueType symbolic route. The glyf PDF encoding
//! swaps A and B. Bare `Type1C` provides a control using exactly the same CFF program.

#![expect(
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    clippy::expect_used,
    reason = "this standalone fixture asserts parser and font paths directly; fixture failures must panic"
)]

fn be_u16(bytes: &[u8], at: usize) -> u16 {
    u16::from_be_bytes([bytes[at], bytes[at + 1]])
}

fn be_u32(bytes: &[u8], at: usize) -> u32 {
    u32::from_be_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]])
}

fn put_u16(bytes: &mut [u8], at: usize, value: u16) {
    bytes[at..at + 2].copy_from_slice(&value.to_be_bytes());
}

fn put_u32(bytes: &mut [u8], at: usize, value: u32) {
    bytes[at..at + 4].copy_from_slice(&value.to_be_bytes());
}

fn padded_checksum(bytes: &[u8]) -> u32 {
    bytes.chunks(4).fold(0u32, |sum, chunk| {
        let mut word = [0u8; 4];
        word[..chunk.len()].copy_from_slice(chunk);
        sum.wrapping_add(u32::from_be_bytes(word))
    })
}

fn without_cmap(base: &[u8]) -> Vec<u8> {
    let count = usize::from(be_u16(base, 4));
    let mut tables: Vec<([u8; 4], Vec<u8>)> = Vec::new();
    let mut head_adjustment_at = None;
    for index in 0..count {
        let record = 12 + index * 16;
        let tag: [u8; 4] = base[record..record + 4].try_into().expect("four-byte tag");
        if &tag == b"cmap" {
            continue;
        }
        let offset = usize::try_from(be_u32(base, record + 8)).expect("table offset fits usize");
        let length = usize::try_from(be_u32(base, record + 12)).expect("table length fits usize");
        let mut bytes = base[offset..offset + length].to_vec();
        if &tag == b"head" {
            bytes[8..12].fill(0);
        }
        tables.push((tag, bytes));
    }
    tables.sort_by_key(|(tag, _)| *tag);

    let table_count = u16::try_from(tables.len()).expect("table count fits");
    let mut out = vec![0u8; 12 + tables.len().saturating_mul(16)];
    out[..4].copy_from_slice(&base[..4]);
    put_u16(&mut out, 4, table_count);
    let leading = u16::try_from(table_count.leading_zeros()).expect("leading-zero count fits");
    let power = 1u16 << 15u16.saturating_sub(leading);
    put_u16(&mut out, 6, power.saturating_mul(16));
    put_u16(
        &mut out,
        8,
        u16::try_from(power.trailing_zeros()).expect("u16 bit index fits u16"),
    );
    put_u16(
        &mut out,
        10,
        table_count
            .saturating_mul(16)
            .saturating_sub(power.saturating_mul(16)),
    );

    for (index, (tag, bytes)) in tables.iter().enumerate() {
        while !out.len().is_multiple_of(4) {
            out.push(0);
        }
        let offset = u32::try_from(out.len()).expect("sfnt remains small");
        let record = 12 + index * 16;
        out[record..record + 4].copy_from_slice(tag);
        let checksum = if tag == b"head" {
            let mut head = bytes.clone();
            head[8..12].fill(0);
            padded_checksum(&head)
        } else {
            padded_checksum(bytes)
        };
        put_u32(&mut out, record + 4, checksum);
        put_u32(&mut out, record + 8, offset);
        put_u32(
            &mut out,
            record + 12,
            u32::try_from(bytes.len()).expect("table length fits"),
        );
        if tag == b"head" {
            head_adjustment_at = Some(offset.saturating_add(8));
        }
        out.extend_from_slice(bytes);
    }
    let adjustment_at = usize::try_from(head_adjustment_at.expect("head table exists"))
        .expect("head adjustment offset fits");
    let adjustment = 0xB1B0_AFBAu32.wrapping_sub(padded_checksum(&out));
    put_u32(&mut out, adjustment_at, adjustment);
    assert_eq!(padded_checksum(&out), 0xB1B0_AFBA, "valid sfnt checksum");
    out
}

use pdf_model::Pages;
use pdf_syntax::Document;

const CFF: &[u8] = include_bytes!("fixtures/opentype-selection/cff.otf");
const GLYF: &[u8] = include_bytes!("fixtures/opentype-selection/glyf.ttf");

#[derive(Clone, Copy)]
enum FontCase {
    SimpleCff,
    BareCff,
    SimpleGlyf,
    CidCff,
    CidGlyf,
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

#[expect(
    clippy::too_many_lines,
    reason = "complete PDF fixture keeps object references and exact xref together"
)]
fn fixture(case: FontCase, program: &[u8], codes: &[u8]) -> Vec<u8> {
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

    let (font_dictionary, descriptor_name, file_name, file_subtype) = match case {
        FontCase::SimpleCff | FontCase::BareCff => (
            b"<< /Type /Font /Subtype /Type1 /BaseFont /AuditCFF /FirstChar 65 /LastChar 66 /Widths [500 500] /Encoding << /BaseEncoding /WinAnsiEncoding /Differences [65 /two.sups 66 /two] >> /FontDescriptor 5 0 R >>".as_slice(),
            b"AuditCFF".as_slice(),
            b"FontFile3".as_slice(),
            b"/Subtype /OpenType".as_slice(),
        ),
        FontCase::SimpleGlyf => (
            b"<< /Type /Font /Subtype /TrueType /BaseFont /AuditGlyf /FirstChar 65 /LastChar 66 /Widths [500 500] /Encoding << /BaseEncoding /WinAnsiEncoding /Differences [65 /B 66 /A] >> /FontDescriptor 5 0 R >>".as_slice(),
            b"AuditGlyf".as_slice(),
            b"FontFile3".as_slice(),
            b"/Subtype /OpenType".as_slice(),
        ),
        FontCase::CidCff => (
            b"<< /Type /Font /Subtype /Type0 /BaseFont /AuditCFF /Encoding /Identity-H /DescendantFonts [7 0 R] >>".as_slice(),
            b"AuditCFF".as_slice(),
            b"FontFile3".as_slice(),
            b"/Subtype /OpenType".as_slice(),
        ),
        FontCase::CidGlyf => (
            b"<< /Type /Font /Subtype /Type0 /BaseFont /AuditGlyf /Encoding /Identity-H /DescendantFonts [7 0 R] >>".as_slice(),
            b"AuditGlyf".as_slice(),
            b"FontFile3".as_slice(),
            b"/Subtype /OpenType".as_slice(),
        ),
    };
    push_object(&mut output, &mut offsets, 4, font_dictionary);
    let flags = if matches!(case, FontCase::SimpleGlyf | FontCase::CidGlyf) {
        32
    } else {
        4
    };
    let descriptor = format!(
        "<< /Type /FontDescriptor /FontName /{} /Flags {flags} /FontBBox [-1000 -1000 2000 2000] /ItalicAngle 0 /Ascent 1000 /Descent -300 /CapHeight 700 /StemV 80 /{} 6 0 R >>",
        String::from_utf8_lossy(descriptor_name),
        String::from_utf8_lossy(file_name)
    );
    push_object(&mut output, &mut offsets, 5, descriptor.as_bytes());
    let file_subtype = if matches!(case, FontCase::BareCff) {
        b"/Subtype /Type1C".as_slice()
    } else {
        file_subtype
    };
    let file_entries = if matches!(case, FontCase::SimpleGlyf | FontCase::CidGlyf) {
        format!(
            "{} /Length1 {}",
            String::from_utf8_lossy(file_subtype),
            program.len()
        )
        .into_bytes()
    } else {
        file_subtype.to_vec()
    };
    let embedded = stream_body(&file_entries, program);
    push_object(&mut output, &mut offsets, 6, &embedded);

    if matches!(case, FontCase::CidCff | FontCase::CidGlyf) {
        let descendant = match case {
            FontCase::CidCff => b"<< /Type /Font /Subtype /CIDFontType0 /BaseFont /AuditCFF /CIDSystemInfo << /Registry (Adobe) /Ordering (Identity) /Supplement 0 >> /FontDescriptor 5 0 R /DW 1000 >>".as_slice(),
            FontCase::CidGlyf => b"<< /Type /Font /Subtype /CIDFontType2 /BaseFont /AuditGlyf /CIDSystemInfo << /Registry (Adobe) /Ordering (Identity) /Supplement 0 >> /FontDescriptor 5 0 R /DW 1000 /CIDToGIDMap /Identity >>".as_slice(),
            _ => unreachable!(),
        };
        push_object(&mut output, &mut offsets, 7, descendant);
    } else {
        push_object(&mut output, &mut offsets, 7, b"null");
    }

    let mut content = b"BT /F1 20 Tf 1 0 0 1 10 40 Tm <".to_vec();
    for byte in codes {
        content.extend_from_slice(format!("{byte:02X}").as_bytes());
    }
    content.extend_from_slice(b"> Tj ET\n");
    let content = stream_body(b"", &content);
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

fn selected_glyphs(case: FontCase, program: &[u8], codes: &[u8]) -> Vec<u16> {
    let bytes = fixture(case, program, codes);
    let document = Document::open(bytes).expect("the complete PDF fixture parses");
    assert!(!document.was_recovered(), "exact cross references");
    let page = Pages::new(&document).get(0).expect("one page");
    let resource_fonts = document.get_key(&page.resources, "Font");
    let fonts = resource_fonts.as_dict().expect("the page's font resources");
    let font_dictionary = document
        .resolve(&document.get_key(fonts, "F1"))
        .as_dict()
        .expect("the page's F1 font dictionary")
        .clone();
    let loaded = pdf_font::LoadedFont::load(&document, &font_dictionary, "F1")
        .expect("the font program loads from this PDF");
    loaded
        .decode(codes)
        .into_iter()
        .map(|code| {
            let outline = loaded.outline(code).expect("embedded glyph has an outline");
            assert!(!outline.commands().is_empty(), "selected glyph is nonempty");
            loaded.glyph_index(code).expect("code selects a glyph")
        })
        .collect()
}

#[test]
fn opentype_cff_under_simple_type1_uses_encoding_glyph_names_and_cff_charset() {
    assert_eq!(selected_glyphs(FontCase::SimpleCff, CFF, &[65, 66]), [2, 1]);
}

#[test]
fn opentype_glyf_under_simple_truetype_uses_the_glyph_name_cmap_route() {
    assert_eq!(
        selected_glyphs(FontCase::SimpleGlyf, GLYF, &[65, 66]),
        [2, 1]
    );
}

#[test]
fn opentype_cff_under_cidfonttype0_uses_identity_cid_to_gid_for_name_keyed_cff() {
    assert_eq!(
        selected_glyphs(FontCase::CidCff, CFF, &[0, 2, 0, 1]),
        [2, 1]
    );
}

#[test]
fn opentype_glyf_under_cidfonttype2_uses_identity_cid_to_gid() {
    assert_eq!(
        selected_glyphs(FontCase::CidGlyf, &without_cmap(GLYF), &[0, 1, 0, 2]),
        [1, 2]
    );
}

#[test]
fn the_same_cff_as_type1c_honours_the_symbolic_fonts_explicit_encoding() {
    let table = (0..usize::from(be_u16(CFF, 4)))
        .find_map(|index| {
            let record = 12 + index * 16;
            if &CFF[record..record + 4] != b"CFF " {
                return None;
            }
            let offset = usize::try_from(be_u32(CFF, record + 8)).expect("small font offset");
            let length = usize::try_from(be_u32(CFF, record + 12)).expect("small table length");
            Some(&CFF[offset..offset + length])
        })
        .expect("synthetic font contains CFF");
    assert_eq!(selected_glyphs(FontCase::BareCff, table, &[65, 66]), [2, 1]);
}

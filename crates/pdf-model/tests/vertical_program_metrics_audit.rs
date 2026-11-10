//! Consumer regression for §9.9.1's rule that a PDF processor never uses sfnt vhea/vmtx
//! for PDF vertical metrics.
//!
//! The fixture starts from the repository's deterministic Liberation Sans outline program,
//! removes `cmap` (`CIDFontType2` supplies the glyph mapping), and appends structurally valid
//! vertical metrics. It rewrites the sfnt directory/checksums so the embedded program loads.

#![expect(
    clippy::arithmetic_side_effects,
    clippy::expect_used,
    clippy::float_cmp,
    clippy::indexing_slicing,
    reason = "test code reads fixed fields from a checked-in font and complete synthetic fixtures"
)]

use std::io::Write as _;

use pdf_syntax::{Document, ObjectId};

const BASE: &[u8] = include_bytes!("../../../data/standard-fonts/LiberationSans-Regular.ttf");
const NUM_GLYPHS: usize = 681; // read from maxp in the included deterministic program
const UNITS_PER_EM: u16 = 2048; // read from head in that program

fn be_u16(bytes: &[u8], at: usize) -> u16 {
    u16::from_be_bytes([bytes[at], bytes[at + 1]])
}

fn be_u32(bytes: &[u8], at: usize) -> u32 {
    u32::from_be_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]])
}

fn put_u16(bytes: &mut [u8], at: usize, value: u16) {
    bytes[at..at + 2].copy_from_slice(&value.to_be_bytes());
}

fn put_i16(bytes: &mut [u8], at: usize, value: i16) {
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

#[expect(
    clippy::too_many_lines,
    reason = "one deterministic sfnt fixture rebuild keeps directory offsets and checksums together"
)]
fn with_vertical_metrics(advance: u16) -> Vec<u8> {
    let count = usize::from(be_u16(BASE, 4));
    let mut tables: Vec<([u8; 4], Vec<u8>)> = Vec::new();
    let mut head_adjustment_at = None;
    let mut glyph_count = None;
    for index in 0..count {
        let record = 12 + index * 16;
        let tag: [u8; 4] = BASE[record..record + 4].try_into().expect("four-byte tag");
        if &tag == b"cmap" {
            continue;
        }
        let offset = usize::try_from(be_u32(BASE, record + 8)).expect("table offset fits usize");
        let length = usize::try_from(be_u32(BASE, record + 12)).expect("table length fits usize");
        let mut bytes = BASE[offset..offset + length].to_vec();
        if &tag == b"head" {
            bytes[8..12].fill(0);
        }
        if &tag == b"maxp" {
            glyph_count = Some(usize::from(be_u16(&bytes, 4)));
        }
        tables.push((tag, bytes));
    }
    let glyph_count = glyph_count.expect("the base font has maxp");
    assert_eq!(glyph_count, NUM_GLYPHS, "the fixture's maxp is stable");
    let original_head = &tables
        .iter()
        .find(|(tag, _)| tag == b"head")
        .expect("head table")
        .1;
    assert_eq!(
        be_u16(original_head, 18),
        UNITS_PER_EM,
        "the fixture's unitsPerEm is stable"
    );

    // OpenType vhea: yMaxExtent = max(tsb + yMax - yMin), not simply max(yMax).
    // https://learn.microsoft.com/en-us/typography/opentype/spec/vhea
    // With zero top side bearings, these extrema follow the existing glyph boxes. This keeps
    // the added vhea/vmtx structurally coherent with the outlines rather than merely parseable.
    let head = &tables
        .iter()
        .find(|(tag, _)| tag == b"head")
        .expect("head table")
        .1;
    let loca_format = i16::from_be_bytes([head[50], head[51]]);
    let loca = &tables
        .iter()
        .find(|(tag, _)| tag == b"loca")
        .expect("loca table")
        .1;
    let glyf = &tables
        .iter()
        .find(|(tag, _)| tag == b"glyf")
        .expect("glyf table")
        .1;
    let location = |glyph: usize| -> usize {
        if loca_format == 0 {
            usize::from(u16::from_be_bytes([loca[glyph * 2], loca[glyph * 2 + 1]]))
                .saturating_mul(2)
        } else {
            usize::try_from(u32::from_be_bytes([
                loca[glyph * 4],
                loca[glyph * 4 + 1],
                loca[glyph * 4 + 2],
                loca[glyph * 4 + 3],
            ]))
            .expect("loca offset fits")
        }
    };
    let glyph_heights: Vec<i16> = (0..glyph_count)
        .map(|glyph| {
            let start = location(glyph);
            let end = location(glyph + 1);
            if end.saturating_sub(start) < 10 {
                0
            } else {
                i16::from_be_bytes([glyf[start + 8], glyf[start + 9]])
                    - i16::from_be_bytes([glyf[start + 4], glyf[start + 5]])
            }
        })
        .collect();
    let max_height = glyph_heights.iter().copied().max().expect("glyphs exist");
    let min_bottom = glyph_heights
        .iter()
        .map(|y| i32::from(advance) - i32::from(*y))
        .min()
        .expect("glyphs exist");
    let min_bottom = i16::try_from(min_bottom).expect("vhea signed metric fits");

    let mut vhea = vec![0u8; 36];
    put_u32(&mut vhea, 0, 0x0001_0000);
    put_i16(
        &mut vhea,
        4,
        i16::try_from(UNITS_PER_EM).expect("unitsPerEm fits i16"),
    );
    put_i16(&mut vhea, 6, -512);
    put_u16(&mut vhea, 10, advance);
    put_u16(&mut vhea, 12, 0); // every top side bearing below is zero
    put_i16(&mut vhea, 14, min_bottom);
    put_i16(&mut vhea, 16, max_height);
    put_i16(&mut vhea, 20, 1); // caretSlopeRun: horizontal caret for vertical text
    put_i16(&mut vhea, 32, 0); // metricDataFormat
    put_u16(
        &mut vhea,
        34,
        u16::try_from(glyph_count).expect("glyph count fits"),
    );
    let mut vmtx = Vec::with_capacity(glyph_count.saturating_mul(4));
    for _ in 0..glyph_count {
        vmtx.extend_from_slice(&advance.to_be_bytes());
        vmtx.extend_from_slice(&0i16.to_be_bytes());
    }
    tables.push((*b"vhea", vhea));
    tables.push((*b"vmtx", vmtx));
    tables.sort_by_key(|(tag, _)| *tag);

    let table_count = u16::try_from(tables.len()).expect("table count fits");
    let mut out = vec![0u8; 12 + tables.len().saturating_mul(16)];
    out[..4].copy_from_slice(&BASE[..4]);
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

fn pdf(program: &[u8], dw2: &str, vertical_metrics: &str) -> Vec<u8> {
    let content = "BT /F1 24 Tf 40 150 Td <002400250026> Tj ET";
    let bodies: Vec<Vec<u8>> = vec![
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] /Resources << /Font << /F1 5 0 R >> >> /Contents 4 0 R >>".to_vec(),
        format!("<< /Length {} >>\nstream\n{content}\nendstream", content.len()).into_bytes(),
        b"<< /Type /Font /Subtype /Type0 /BaseFont /LiberationSans /Encoding /Identity-V /DescendantFonts [6 0 R] >>".to_vec(),
        format!("<< /Type /Font /Subtype /CIDFontType2 /BaseFont /LiberationSans /CIDSystemInfo << /Registry (Adobe) /Ordering (Identity) /Supplement 0 >> /FontDescriptor 7 0 R /CIDToGIDMap /Identity /DW2 {dw2} /W2 {vertical_metrics} >>").into_bytes(),
        b"<< /Type /FontDescriptor /FontName /LiberationSans /Flags 4 /FontBBox [-200 -300 1100 1000] /ItalicAngle 0 /Ascent 900 /Descent -200 /CapHeight 700 /StemV 80 /FontFile2 8 0 R >>".to_vec(),
        {
            let mut stream = format!("<< /Length {} /Length1 {} >>\nstream\n", program.len(), program.len()).into_bytes();
            stream.extend_from_slice(program);
            stream.extend_from_slice(b"\nendstream");
            stream
        },
    ];
    let mut out = b"%PDF-1.7\n".to_vec();
    let mut offsets = Vec::new();
    for (index, body) in bodies.iter().enumerate() {
        offsets.push(out.len());
        out.extend_from_slice(format!("{} 0 obj\n", index + 1).as_bytes());
        out.extend_from_slice(body);
        out.extend_from_slice(b"\nendobj\n");
    }
    let xref = out.len();
    out.extend_from_slice(
        format!("xref\n0 {}\n0000000000 65535 f \n", bodies.len() + 1).as_bytes(),
    );
    for offset in offsets {
        out.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    let _ = write!(
        out,
        "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
        bodies.len() + 1
    );
    out
}

fn place(program: &[u8], dw2: &str, w2: &str) -> (Vec<[f32; 8]>, Vec<f32>) {
    let document = Document::open(pdf(program, dw2, w2)).expect("complete PDF opens");
    assert!(
        !document.was_recovered(),
        "the fixture has a full xref table"
    );
    let page = pdf_model::Pages::new(&document).get(0).expect("one page");
    let interpretation = pdf_model::interpret(&document, &page);
    assert!(
        interpretation.unsupported.is_empty(),
        "valid fixture: {:?}",
        interpretation.unsupported
    );
    assert_eq!(
        interpretation.text_layer.len(),
        3,
        "all embedded glyphs are placed"
    );

    let font_ref = document.get(ObjectId {
        number: 5,
        generation: 0,
    });
    let font_dict = font_ref.as_dict().expect("object 5 is Type0");
    let font =
        pdf_font::LoadedFont::load(&document, font_dict, "F1").expect("embedded Type0 loads");
    let codes = font.decode(&[0x00, 0x24, 0x00, 0x25, 0x00, 0x26]);
    assert_eq!(codes.len(), 3);
    assert!(
        codes.iter().all(|code| {
            font.outline(*code)
                .is_some_and(|path| !path.commands().is_empty())
        }),
        "all codes have actual embedded outlines"
    );
    let program_advances = codes
        .iter()
        .map(|code| {
            font.program_vertical_advance(*code)
                .expect("loaded vmtx is visible to diagnostic API")
        })
        .collect();
    let quads = interpretation
        .text_layer
        .iter()
        .map(|placed| placed.quad)
        .collect();
    (quads, program_advances)
}

#[test]
fn embedded_vmtx_cannot_override_vertical_cidfont_text_placement() {
    // CIDs 36–38 have real outlines in the embedded repository font. CID 36 has explicit /W2;
    // CIDs 37–38 use /DW2. The PDF says -1.4 em for CID 36 and -1.0 em by default, while the
    // valid embedded programs say 5 em and 7 em for every glyph.
    let w2 = "[36 [-1400 100 700]]";
    let first_program = with_vertical_metrics(10_240);
    let second_program = with_vertical_metrics(14_336);
    let (first_layout, first_vmtx) = place(&first_program, "[880 -1000]", w2);
    let (second_layout, second_vmtx) = place(&second_program, "[880 -1000]", w2);
    assert_eq!(
        first_vmtx,
        vec![-5.0, -5.0, -5.0],
        "the first embedded program really states 5 em"
    );
    assert_eq!(
        second_vmtx,
        vec![-7.0, -7.0, -7.0],
        "the second embedded program really states 7 em"
    );
    assert_eq!(
        first_layout, second_layout,
        "changing only vmtx must not move any glyph"
    );

    // Sensitivity controls: hold the program fixed and change each PDF dictionary source. /W2
    // on the first glyph changes the second glyph's origin. The third glyph uses /DW2, so
    // changing only its preceding default displacement changes its placement too.
    let (explicit_layout, _) = place(&first_program, "[880 -1000]", "[36 [-2200 100 700]]");
    assert_ne!(
        first_layout[1], explicit_layout[1],
        "changing PDF /W2 changes the next glyph's actual text placement"
    );
    let (default_layout, _) = place(&first_program, "[880 -2400]", w2);
    assert_eq!(
        first_layout[1], default_layout[1],
        "the second glyph starts at the same origin before its changed default advance"
    );
    assert_ne!(
        first_layout[2], default_layout[2],
        "changing /DW2's default vertical advance affects the third glyph"
    );
}

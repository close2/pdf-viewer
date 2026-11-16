//! Deterministic composite-font controls for §§9.3.3, 9.4.3 and 9.4.4.

#![expect(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "small exact-xref fixture; all accesses have fixture-derived bounds"
)]

use pdf_render::Command;
use pdf_syntax::Document;

const PROGRAM: &[u8] = include_bytes!("../../../data/standard-fonts/LiberationSans-Regular.ttf");

fn assemble(objects: &[Vec<u8>]) -> Vec<u8> {
    let mut out = b"%PDF-1.7\n".to_vec();
    let mut offsets = Vec::new();
    for (index, object) in objects.iter().enumerate() {
        offsets.push(out.len());
        out.extend_from_slice(format!("{} 0 obj\n", index + 1).as_bytes());
        out.extend_from_slice(object);
        out.extend_from_slice(b"\nendobj\n");
    }
    let xref = out.len();
    out.extend_from_slice(
        format!("xref\n0 {}\n0000000000 65535 f \n", objects.len() + 1).as_bytes(),
    );
    for offset in offsets {
        out.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    out.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
            objects.len() + 1
        )
        .as_bytes(),
    );
    out
}

fn stream(data: &[u8], extra: &str) -> Vec<u8> {
    let mut out = format!("<< /Length {} {extra} >>\nstream\n", data.len()).into_bytes();
    out.extend_from_slice(data);
    out.extend_from_slice(b"\nendstream");
    out
}

/// Vertical Type 0 font with an embedded face, exact xref and explicit metrics. The same
/// character is available as one-byte code 20 and two-byte code 0020, both mapped to CID 3;
/// A is CID 36. This isolates encoded code length as the Tw distinction.
fn vertical_fixture(content: &str) -> Vec<u8> {
    let encoding = b"/CIDInit /ProcSet findresource begin\n12 dict begin\nbegincmap\n/CIDSystemInfo << /Registry (Adobe) /Ordering (Identity) /Supplement 0 >> def\n/CMapName /AuditV def\n/CMapType 1 def\n/WMode 1 def\n2 begincodespacerange\n<20> <7E>\n<0020> <007E>\nendcodespacerange\n4 begincidchar\n<20> 3\n<41> 36\n<0020> 3\n<0041> 36\nendcidchar\nendcmap\nCMapName currentdict /CMap defineresource pop\nend\nend";
    let unicode = b"/CIDInit /ProcSet findresource begin\n12 dict begin\nbegincmap\n/CIDSystemInfo << /Registry (Adobe) /Ordering (UCS) /Supplement 0 >> def\n/CMapName /AuditU def\n/CMapType 2 def\n2 begincodespacerange\n<20> <7E>\n<0020> <007E>\nendcodespacerange\n4 beginbfchar\n<20> <0020>\n<41> <0041>\n<0020> <0020>\n<0041> <0041>\nendbfchar\nendcmap\nCMapName currentdict /CMap defineresource pop\nend\nend";
    let content = format!("{content}\n");
    let mut program = format!(
        "<< /Length {} /Length1 {} >>\nstream\n",
        PROGRAM.len(),
        PROGRAM.len()
    )
    .into_bytes();
    program.extend_from_slice(PROGRAM);
    program.extend_from_slice(b"\nendstream");
    assemble(&[
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Resources << /Font << /F1 5 0 R >> >> /Contents 4 0 R >>".to_vec(),
        stream(content.as_bytes(), ""),
        b"<< /Type /Font /Subtype /Type0 /BaseFont /LiberationSans /Encoding 8 0 R /ToUnicode 9 0 R /DescendantFonts [6 0 R] >>".to_vec(),
        b"<< /Type /Font /Subtype /CIDFontType2 /BaseFont /LiberationSans /CIDSystemInfo << /Registry (Adobe) /Ordering (Identity) /Supplement 0 >> /FontDescriptor 7 0 R /CIDToGIDMap /Identity /DW 1000 /W [3 [277.83203125] 36 [666.9921875]] /DW2 [880 -1000] /W2 [3 [-1000 138.916015625 880] 36 [-1000 333.49609375 880]] >>".to_vec(),
        b"<< /Type /FontDescriptor /FontName /LiberationSans /Flags 32 /FontBBox [-203.125 -303.22265625 1050.29296875 910.15625] /ItalicAngle 0 /Ascent 905 /Descent -212 /CapHeight 729 /StemV 80 /FontFile2 10 0 R >>".to_vec(),
        stream(encoding, "/Type /CMap /CMapName /AuditV /CIDSystemInfo << /Registry (Adobe) /Ordering (Identity) /Supplement 0 >> /WMode 1"),
        stream(unicode, "/Type /CMap /CMapName /AuditU /CIDSystemInfo << /Registry (Adobe) /Ordering (UCS) /Supplement 0 >>"),
        program,
    ])
}

fn a_centres(content: &str) -> Vec<(f32, f32)> {
    let document = Document::open(vertical_fixture(content)).expect("fixture opens");
    assert!(
        !document.was_recovered(),
        "fixture has exact offsets and xref"
    );
    let page = pdf_model::Pages::new(&document).get(0).expect("one page");
    let result = pdf_model::interpret(&document, &page);
    assert!(
        result.is_complete(),
        "fixture supported: {:?}",
        result.unsupported
    );
    assert_eq!(
        result
            .display_list
            .commands()
            .iter()
            .filter(|command| matches!(command, Command::Fill { .. }))
            .count(),
        2,
        "both A glyphs have paintable outlines; spacing codes contribute no ink"
    );
    let centres: Vec<_> = result
        .text_layer
        .iter()
        .filter(|placed| result.text.get(placed.span.clone()) == Some("A"))
        .map(|placed| {
            let q = placed.quad;
            (
                (q[0] + q[2] + q[4] + q[6]) / 4.0,
                (q[1] + q[3] + q[5] + q[7]) / 4.0,
            )
        })
        .collect();
    assert_eq!(
        centres.len(),
        2,
        "both mapped A glyphs are observable: {centres:?}"
    );
    centres
}

#[test]
fn tw_uses_code_length_for_composite_font_space_and_multibyte_control() {
    let one_byte_plain = a_centres("BT /F1 20 Tf 20 80 Td <412041> Tj ET");
    let one_byte_spaced = a_centres("BT /F1 20 Tf 4 Tw 20 80 Td <412041> Tj ET");
    let two_byte_plain = a_centres("BT /F1 20 Tf 20 80 Td <004100200041> Tj ET");
    let two_byte_spaced = a_centres("BT /F1 20 Tf 4 Tw 20 80 Td <004100200041> Tj ET");
    assert!(
        (one_byte_spaced[1].1 - one_byte_plain[1].1 - 4.0).abs() < 0.02,
        "single-byte composite code 32 gets Tw=4"
    );
    assert!(
        (two_byte_spaced[1].1 - two_byte_plain[1].1).abs() < 0.02,
        "the same byte value inside a two-byte code gets no Tw"
    );
}

#[test]
fn vertical_tj_is_subtracted_from_vertical_coordinate_and_not_scaled_by_tz() {
    let plain = a_centres("BT /F1 20 Tf 20 80 Td [(A) (A)] TJ ET");
    let adjusted = a_centres("BT /F1 20 Tf 20 80 Td [(A) 250 (A)] TJ ET");
    let half_scale = a_centres("BT /F1 20 Tf 50 Tz 20 80 Td [(A) 250 (A)] TJ ET");
    let plain_step = plain[1].1 - plain[0].1;
    let adjusted_step = adjusted[1].1 - adjusted[0].1;
    let half_scale_step = half_scale[1].1 - half_scale[0].1;
    assert!(
        (plain_step + 20.0).abs() < 0.05,
        "W2 -1000 at 20pt gives vertical advance -20, got {plain_step}"
    );
    assert!(
        (adjusted_step - (plain_step - 5.0)).abs() < 0.05,
        "positive TJ=250 subtracts 5 text-space units on vertical axis: {plain_step} -> {adjusted_step}"
    );
    assert!(
        (half_scale_step - adjusted_step).abs() < 0.05,
        "Tz scales glyph horizontal coordinate but does not scale vertical TJ adjustment"
    );
}

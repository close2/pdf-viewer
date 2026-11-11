//! Simple-font width consumers, distinguishing effective rules from fallback policy.
#![expect(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "the fixtures and expected glyph positions are controlled and should fail loudly"
)]

use pdf_render::Command;
use pdf_syntax::Document;

const FONT: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../data/standard-fonts/LiberationSans-Regular.ttf"
));

fn stream(dictionary: &str, data: &[u8]) -> Vec<u8> {
    let mut object = format!("<< {dictionary} /Length {} >>\nstream\n", data.len()).into_bytes();
    object.extend_from_slice(data);
    object.extend_from_slice(b"\nendstream");
    object
}

fn pdf(font: &str, content: &str, extras: &[Vec<u8>]) -> Vec<u8> {
    let mut objects = vec![
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 160 120] \
           /Resources << /Font << /F1 5 0 R >> >> /Contents 4 0 R >>"
            .to_vec(),
        stream("", content.as_bytes()),
        font.as_bytes().to_vec(),
    ];
    objects.extend_from_slice(extras);
    assemble(&objects)
}

fn assemble(objects: &[Vec<u8>]) -> Vec<u8> {
    let mut output = b"%PDF-1.7\n".to_vec();
    let mut offsets = Vec::new();
    for (number, body) in (1_usize..).zip(objects) {
        offsets.push(output.len());
        output.extend_from_slice(format!("{number} 0 obj\n").as_bytes());
        output.extend_from_slice(body);
        output.extend_from_slice(b"\nendobj\n");
    }
    let xref = output.len();
    let size = offsets.len() + 1;
    output.extend_from_slice(format!("xref\n0 {size}\n0000000000 65535 f \n").as_bytes());
    for offset in offsets {
        output.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    output.extend_from_slice(
        format!("trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n").as_bytes(),
    );
    output
}

fn glyph_origins(bytes: Vec<u8>) -> Vec<f32> {
    let document = Document::open(bytes).expect("complete PDF fixture parses");
    assert!(!document.was_recovered(), "fixture has exact xref offsets");
    let page = pdf_model::Pages::new(&document).get(0).expect("one page");
    let interpretation = pdf_model::interpret(&document, &page);
    assert!(
        interpretation.unsupported.is_empty(),
        "fixture uses supported simple-font behavior: {:?}",
        interpretation.unsupported
    );
    interpretation
        .display_list
        .commands()
        .iter()
        .filter_map(|command| match command {
            Command::Fill { transform, .. } => Some(transform.e),
            _ => None,
        })
        .collect()
}

#[test]
fn embedded_truetype_widths_are_indexed_from_firstchar_and_place_each_glyph() {
    // Liberation Sans Regular's A/B/C/D hmtx advances are 1366/2048,
    // 1366/2048, 1479/2048 and 1479/2048 ems. Table 109's integer widths
    // [667, 667, 722, 722] are consistent with those program metrics.
    let font = "<< /Type /Font /Subtype /TrueType /BaseFont /LiberationSans \
        /FirstChar 65 /LastChar 68 /Widths [667 667 722 722] \
        /Encoding /WinAnsiEncoding /FontDescriptor 6 0 R >>";
    let descriptor = b"<< /Type /FontDescriptor /FontName /LiberationSans \
        /Flags 32 /FontBBox [-543 -303 1301 981] /ItalicAngle 0 \
        /Ascent 905 /Descent -212 /CapHeight 687 /StemV 80 /FontFile2 7 0 R >>"
        .to_vec();
    let font_file = stream(&format!("/Length1 {}", FONT.len()), FONT);
    let origins = glyph_origins(pdf(
        font,
        "BT /F1 24 Tf 10 30 Td (ABCD) Tj ET",
        &[descriptor, font_file],
    ));

    assert_eq!(origins.len(), 4, "each code reaches its embedded outline");
    assert!((origins[0] - 10.0).abs() < 0.001);
    assert!(
        (origins[1] - 26.008).abs() < 0.001,
        "A's 667-unit width: {origins:?}"
    );
    assert!(
        (origins[2] - 42.016).abs() < 0.001,
        "B's 667-unit width: {origins:?}"
    );
    assert!(
        (origins[3] - 59.344).abs() < 0.001,
        "C's 722-unit width: {origins:?}"
    );
}

#[test]
fn pdf_17_standard_fourteen_may_omit_the_width_fields_and_uses_published_metrics() {
    // Under PDF 1.0–1.7, Table 109 makes these entries optional for a standard Type 1 font.
    // This fixture omits FirstChar, LastChar, Widths and FontDescriptor and exercises the
    // current reader's published-metric fallback through Tf/Tj and glyph placement.
    let font = "<< /Type /Font /Subtype /Type1 /BaseFont /Courier >>";
    let origins = glyph_origins(pdf(font, "BT /F1 20 Tf 10 30 Td (AB) Tj ET", &[]));

    assert_eq!(origins.len(), 2, "both standard Courier outlines are drawn");
    assert!((origins[0] - 10.0).abs() < 0.001);
    assert!(
        (origins[1] - 22.0).abs() < 0.001,
        "published Courier advance is 600/1000 em: {origins:?}"
    );
}

#[test]
fn pdf_17_standard_fourteen_can_state_widths_without_a_fontdescriptor() {
    // EC3 #47/#48 strikes Table 109's old all-four-or-none sentence. Each entry's
    // PDF 1.0–1.7 standard-14 optionality remains in its own Table 109 row, so this
    // effective Table 109 does not retain that all-or-none restriction. This is a
    // PDF 1.7 input consumed under the effective PDF 2.0 rules, not a claim of
    // conformance to the separate ISO 32000-1 producer requirements.
    let font = "<< /Type /Font /Subtype /Type1 /BaseFont /Courier \
        /FirstChar 65 /LastChar 66 /Widths [600 600] >>";
    let origins = glyph_origins(pdf(font, "BT /F1 20 Tf 10 30 Td (AB) Tj ET", &[]));

    assert_eq!(origins.len(), 2, "both standard Courier outlines are drawn");
    assert!((origins[0] - 10.0).abs() < 0.001);
    assert!(
        (origins[1] - 22.0).abs() < 0.001,
        "stated width applies without a FontDescriptor: {origins:?}"
    );
}

#[test]
fn mismatched_missingwidth_characterizes_the_readers_layout_policy() {
    // The first code is inside the one-entry Widths range; B and C are outside it.
    // EC3 #453 makes results implementation-dependent when MissingWidth disagrees
    // with actual glyph widths. Courier is 600 units; this test pins the reader's
    // chosen 400-unit fallback, not a uniquely prescribed ISO result.
    let font = "<< /Type /Font /Subtype /Type1 /BaseFont /Courier \
        /FirstChar 65 /LastChar 65 /Widths [600] /FontDescriptor 6 0 R >>";
    let descriptor = b"<< /Type /FontDescriptor /FontName /Courier /Flags 33 \
        /FontBBox [-23 -250 715 805] /ItalicAngle 0 /Ascent 629 /Descent -157 \
        /CapHeight 562 /StemV 80 /MissingWidth 400 >>"
        .to_vec();
    let origins = glyph_origins(pdf(
        font,
        "BT /F1 20 Tf 10 30 Td (ABC) Tj ET",
        &[descriptor],
    ));

    assert_eq!(origins.len(), 3, "all three Courier outlines are drawn");
    assert!((origins[0] - 10.0).abs() < 0.001);
    assert!(
        (origins[1] - 22.0).abs() < 0.001,
        "A uses Widths[0]: {origins:?}"
    );
    assert!(
        (origins[2] - 30.0).abs() < 0.001,
        "B outside LastChar uses MissingWidth 400: {origins:?}"
    );
}

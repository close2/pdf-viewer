//! Public font-loader regression for §7.3.10 with §9.7.4.3.
//! The direct control and indirect fixture encode the same /W, /DW2, and /W2.
#![expect(
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    reason = "controlled exact-xref fixture; parse/load failures are test failures"
)]

use std::io::Write as _;

use pdf_font::LoadedFont;
use pdf_syntax::{Dictionary, Document, ObjectId};

const PROGRAM: &[u8] = include_bytes!("../../../data/standard-fonts/LiberationSans-Regular.ttf");

fn stream(data: &[u8]) -> Vec<u8> {
    let mut object = format!(
        "<< /Length {} /Length1 {} >>\nstream\n",
        data.len(),
        data.len()
    )
    .into_bytes();
    object.extend_from_slice(data);
    object.extend_from_slice(b"\nendstream");
    object
}

fn pdf(indirect_metrics: bool) -> Vec<u8> {
    let metrics = if indirect_metrics {
        "/DW 556.15234375 /W 8 0 R /DW2 11 0 R /W2 14 0 R"
    } else {
        "/DW 556.15234375 /W [5 [354.98046875]] /DW2 [700 -900] \
         /W2 [5 [-900 177.490234375 700]]"
    };
    let cid_font = format!(
        "<< /Type /Font /Subtype /CIDFontType2 /BaseFont /LiberationSans \
         /CIDSystemInfo << /Registry (Adobe) /Ordering (Identity) /Supplement 0 >> \
         /FontDescriptor 6 0 R /CIDToGIDMap /Identity {metrics} >>"
    );
    let mut objects = vec![
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] \
           /Resources << /Font << /F1 4 0 R >> >> >>"
            .to_vec(),
        b"<< /Type /Font /Subtype /Type0 /BaseFont /LiberationSans \
           /Encoding /Identity-V /DescendantFonts [5 0 R] >>"
            .to_vec(),
        cid_font.into_bytes(),
        b"<< /Type /FontDescriptor /FontName /LiberationSans /Flags 32 \
           /FontBBox [-203 -303 1050 910] /ItalicAngle 0 /Ascent 905 /Descent -212 \
           /CapHeight 729 /StemV 80 /FontFile2 7 0 R >>"
            .to_vec(),
        stream(PROGRAM),
    ];
    if indirect_metrics {
        objects.extend([
            b"[5 9 0 R]".to_vec(),
            b"[10 0 R]".to_vec(),
            b"354.98046875".to_vec(),
            b"[12 0 R 13 0 R]".to_vec(),
            b"700".to_vec(),
            b"-900".to_vec(),
            b"[5 15 0 R]".to_vec(),
            b"[16 0 R 17 0 R 18 0 R]".to_vec(),
            b"-900".to_vec(),
            b"177.490234375".to_vec(),
            b"700".to_vec(),
        ]);
    }
    let mut output = b"%PDF-1.7\n".to_vec();
    let mut offsets = Vec::new();
    for (number, body) in (1_usize..).zip(&objects) {
        offsets.push(output.len());
        output.extend_from_slice(format!("{number} 0 obj\n").as_bytes());
        output.extend_from_slice(body);
        output.extend_from_slice(b"\nendobj\n");
    }
    let xref = output.len();
    let size = offsets.len() + 1;
    output.extend_from_slice(format!("xref\n0 {size}\n0000000000 65535 f \n").as_bytes());
    for offset in offsets {
        let _ = writeln!(output, "{offset:010} 00000 n ");
    }
    let _ = write!(
        output,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n"
    );
    output
}

fn type0(document: &Document) -> Dictionary {
    document
        .get(ObjectId {
            number: 4,
            generation: 0,
        })
        .as_dict()
        .cloned()
        .expect("object 4 is the Type 0 font")
}

#[test]
fn indirect_w_w2_and_dw2_values_match_the_direct_control() {
    for indirect in [false, true] {
        let document = Document::open(pdf(indirect)).expect("complete PDF fixture parses");
        assert!(!document.was_recovered(), "fixture has exact xref offsets");
        let font_dict = type0(&document);
        let font = LoadedFont::load(&document, &font_dict, "F1").expect("font loads");
        let code = font
            .decode(&[0, 5])
            .into_iter()
            .next()
            .expect("one two-byte CID");
        assert!(font.is_vertical());
        assert!(
            (font.advance(code) - 0.354_980_47).abs() < 1e-7,
            "CID 5's program width is 727/2048 em, stated exactly in /W"
        );
        assert_eq!(
            font.vertical_metrics(code),
            ([0.0, -0.9], [0.177_490_23, 0.7]),
            "W2's literal displacement and position vectors"
        );
        let defaulted_code = font
            .decode(&[0, 6])
            .into_iter()
            .next()
            .expect("one two-byte CID outside W/W2");
        assert!(
            (font.advance(defaulted_code) - 0.556_152_34).abs() < 1e-7,
            "CID 6's program width is 1139/2048 em, stated exactly in /DW"
        );
        assert_eq!(
            font.vertical_metrics(defaulted_code),
            ([0.0, -0.9], [0.278_076_17, 0.7]),
            "DW2's literal defaults and half-width position vector"
        );
    }
}

//! Public font-metrics regression for §7.3.10 with §9.6.2.1/Table 109.
//! No font program is embedded: `metrics_only` must answer from the PDF's own values.
#![expect(
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    reason = "controlled exact-xref fixture; structural mistakes should fail loudly"
)]

use std::io::Write as _;

use pdf_font::{Code, LoadedFont};
use pdf_syntax::{Dictionary, Document, ObjectId};

fn pdf(indirect_widths: bool, direct_descriptor: bool, indirect_missing_width: bool) -> Vec<u8> {
    let widths = if indirect_widths {
        "/Widths 8 0 R"
    } else {
        "/Widths [666.9921875 666.9921875]"
    };
    let missing_width = if indirect_missing_width {
        "11 0 R"
    } else {
        "722.16796875"
    };
    let descriptor = format!(
        "<< /Type /FontDescriptor /FontName /LiberationSans /Flags 32 \
         /FontBBox [-543 -303 1301 981] /ItalicAngle 0 /Ascent 905 /Descent -212 \
         /CapHeight 687 /StemV 80 /MissingWidth {missing_width} >>"
    );
    let descriptor_entry = if direct_descriptor {
        format!("/FontDescriptor {descriptor}")
    } else {
        "/FontDescriptor 6 0 R".to_owned()
    };
    let font = format!(
        "<< /Type /Font /Subtype /TrueType /BaseFont /LiberationSans \
         /FirstChar 65 /LastChar 66 {widths} /Encoding /WinAnsiEncoding \
         {descriptor_entry} >>"
    );
    let objects = vec![
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] \
           /Resources << /Font << /F1 5 0 R >> >> >>"
            .to_vec(),
        b"null".to_vec(),
        font.into_bytes(),
        if direct_descriptor {
            b"null".to_vec()
        } else {
            descriptor.into_bytes()
        },
        b"null".to_vec(),
        b"[9 0 R 10 0 R]".to_vec(),
        b"666.9921875".to_vec(),
        b"666.9921875".to_vec(),
        b"722.16796875".to_vec(),
    ];
    // Object 4 and the unused metric objects in direct controls are harmless.
    let mut output = b"%PDF-2.0\n".to_vec();
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

fn font(document: &Document) -> Dictionary {
    document
        .get(ObjectId {
            number: 5,
            generation: 0,
        })
        .as_dict()
        .cloned()
        .expect("object 5 is the TrueType font dictionary")
}

#[test]
fn widths_descriptor_and_member_references_keep_the_literal_metrics() {
    for (indirect_widths, direct_descriptor, indirect_missing_width) in [
        (false, false, false),
        (true, false, false),
        (false, true, true),
    ] {
        let document = Document::open(pdf(
            indirect_widths,
            direct_descriptor,
            indirect_missing_width,
        ))
        .expect("complete fixture parses");
        assert!(!document.was_recovered(), "fixture has exact xref offsets");
        let loaded = LoadedFont::metrics_only(&document, &font(&document), "F1")
            .expect("the stated simple-font metrics load");
        // A/B are explicitly stated in /Widths; C/D are outside LastChar and
        // use the descriptor's MissingWidth. These values track the original
        // Liberation Sans program's 1366/2048 and 1479/2048 advances.
        for (code, expected) in [
            (b'A', 0.666_992_2),
            (b'B', 0.666_992_2),
            (b'C', 0.722_167_97),
            (b'D', 0.722_167_97),
        ] {
            assert!(
                (loaded.advance(Code::single_byte(code)) - expected).abs() < 1e-7,
                "character code {code}"
            );
        }
    }
}

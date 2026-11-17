//! ISO 32000-2 Table 268 dictionary-to-query decimal-format propagation.
//! No other renderer is used as an oracle: 12.5 and the expected strings follow the literal
//! `NumberFormat` inputs. Default zero trimming is the adopted permitted formatting choice.

#![expect(
    clippy::arithmetic_side_effects,
    reason = "bounded xref and device coordinates"
)]
#![expect(
    clippy::panic,
    clippy::expect_used,
    reason = "missing fixture/query values must fail"
)]

use std::fmt::Write as _;

use viewer_core::{Answer, Command, DocumentId, Event, Query, Viewer};

fn pdf(decimal_format: &str) -> Vec<u8> {
    let objects = [
        "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 400 400] /Resources << >> \
         /VP [4 0 R] >>"
            .to_owned(),
        "<< /Type /Viewport /BBox [0 0 400 400] /Measure 5 0 R >>".to_owned(),
        format!(
            "<< /Type /Measure /Subtype /RL /R (1 = 1) \
             /X [<< /U (u) /C 1 >>] \
             /D [<< /U (u) /C 1 {decimal_format} >>] \
             /A [<< /U (u2) /C 1 >>] >>"
        ),
    ];
    let mut bytes = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for (index, body) in objects.iter().enumerate() {
        offsets.push(bytes.len());
        let _ = writeln!(bytes, "{} 0 obj\n{body}\nendobj", index + 1);
    }
    let xref = bytes.len();
    let _ = write!(
        bytes,
        "xref\n0 {}\n0000000000 65535 f \n",
        objects.len() + 1
    );
    for offset in offsets {
        let _ = writeln!(bytes, "{offset:010} 00000 n ");
    }
    let _ = write!(
        bytes,
        "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
        objects.len() + 1
    );
    bytes.into_bytes()
}

fn length(format: &str) -> String {
    let bytes = pdf(format);
    let parsed = pdf_syntax::Document::open(bytes.clone()).expect("valid exact-xref fixture");
    assert!(!parsed.was_recovered());
    assert!(!parsed.cross_reference_chain_was_cut());
    let mut viewer = Viewer::new(400, 400, 1.0);
    let mut events = viewer.handle(Command::Open {
        id: DocumentId(1),
        bytes: bytes.into(),
        password: None,
        fragment: None,
    });
    assert!(events.any(|event| matches!(event, Event::Opened { .. })));
    let Answer::Geometry(geometry) = viewer.query(Query::PageGeometry(0)) else {
        panic!("opened page has displayed geometry");
    };
    let device = |x: f32| {
        [
            geometry.origin.0 + x * geometry.scale,
            geometry.origin.1 + 200.0 * geometry.scale,
        ]
    };
    let points = [device(100.0), device(112.5)];
    let Answer::Measured { traced, .. } = viewer.query(Query::Measure(&points)) else {
        panic!("a valid 12.5-unit path inside the declared viewport is measurable");
    };
    traced
        .length
        .expect("Table 267 D array supplies the distance format")
}

#[test]
fn query_preserves_table_268_decimal_separator_precision_and_defaults() {
    // RD is a document-provided decimal separator. D=100 requests two decimal places, and
    // FD=true retains the trailing zero. 12.5 is exactly representable and independently known.
    let explicit = length("/F /D /D 100 /RD (,) /FD true");

    // With RD, FD, and D omitted, Table 268 defaults to decimal, '.', truncated low-order zeros,
    // and decimal precision 100. Trimming is permitted, and this implementation chooses it.
    let defaults = length("");
    assert_eq!(
        (explicit.as_str(), defaults.as_str()),
        ("12,50 u", "12.5 u")
    );
}

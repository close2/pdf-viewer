//! Public numeric controls for §12.9.1 viewport selection and Table267 conversion.

#![expect(
    clippy::arithmetic_side_effects,
    reason = "small generated xref offsets"
)]
#![expect(
    clippy::panic,
    clippy::expect_used,
    reason = "missing query response is a test failure"
)]

use std::fmt::Write as _;
use viewer_core::{Answer, Command, DocumentId, Query, Viewer};

fn two_scales() -> Vec<u8> {
    let objects = [
        "<< /Type /Catalog /Pages 2 0 R >>",
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 400 400] /Resources << >> /VP [4 0 R 5 0 R] >>",
        "<< /Type /Viewport /BBox [0 0 200 400] /Name (slow) /Measure 6 0 R >>",
        "<< /Type /Viewport /BBox [200 0 400 400] /Name (fast) /Measure 7 0 R >>",
        "<< /Type /Measure /Subtype /RL /R (one hundredth) /X [<< /U (u) /C .01 >>] /D [<< /U (u) /C 1 >>] /A [<< /U (u2) /C 1 >>] >>",
        "<< /Type /Measure /Subtype /RL /R (two hundredths) /X [<< /U (u) /C .02 >>] /D [<< /U (u) /C 1 >>] /A [<< /U (u2) /C 1 >>] >>",
    ];
    let mut out = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for (index, body) in objects.iter().enumerate() {
        offsets.push(out.len());
        let _ = write!(out, "{} 0 obj\n{body}\nendobj\n", index + 1);
    }
    let xref_at = out.len();
    let _ = write!(out, "xref\n0 {}\n0000000000 65535 f \n", objects.len() + 1);
    for offset in &offsets {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    let _ = write!(
        out,
        "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n",
        objects.len() + 1
    );
    out.into_bytes()
}

fn opened() -> Viewer {
    let mut viewer = Viewer::new(400, 400, 1.0);
    let bytes = two_scales();
    let parsed = pdf_syntax::Document::open(bytes.clone()).expect("valid generated fixture");
    assert!(!parsed.was_recovered(), "exact xref fixture");
    let opened = viewer
        .handle(Command::Open {
            id: DocumentId(41),
            bytes: bytes.into(),
            password: None,
            fragment: None,
        })
        .any(|event| matches!(event, viewer_core::Event::Opened { .. }));
    assert!(opened, "the exact-xref fixture opens");
    viewer
}

fn length(viewer: &Viewer, points: &[[f32; 2]]) -> String {
    let Answer::Measured { traced, .. } = viewer.query(Query::Measure(points)) else {
        panic!("the path lies in a viewport with a distance format");
    };
    traced.length.expect("both controls state /D")
}

#[test]
fn a_measured_path_uses_the_distinct_scale_of_its_first_points_viewport() {
    let viewer = opened();
    // 100 page units × .01 = 1 u; the first point is in the left viewport.
    assert_eq!(length(&viewer, &[[50.0, 200.0], [150.0, 200.0]]), "1 u");
    // A crossing path uses the left scale even though its last point is on the right.
    assert_eq!(length(&viewer, &[[150.0, 200.0], [250.0, 200.0]]), "1 u");
    // The same crossing leg reversed: the first point is now in the .02 viewport.
    assert_eq!(length(&viewer, &[[250.0, 200.0], [150.0, 200.0]]), "2 u");
    // Direct in-viewport control confirms the second literal scale independently.
    assert_eq!(length(&viewer, &[[250.0, 250.0], [350.0, 250.0]]), "2 u");
}

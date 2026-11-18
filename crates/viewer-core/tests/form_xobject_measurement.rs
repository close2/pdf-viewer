//! A form `XObject`'s own `/Measure` reaches the measurement query, where its `Do` drew it.
//!
//! ISO 32000-2 Table 95 gives a form a `/Measure` that "specifies the scale and units which shall
//! apply to the form", and Table 269's `/LPTS` maps its unit square "to the rectangular bounds of
//! the `Viewport`, image `XObject`, or forms `XObject` that contains the measure dictionary". A
//! form's bounds are its `/BBox` in form space (Table 93), carried onto the page by its `/Matrix`
//! and the CTM (§8.10.1). This fixture turns them a quarter, which no `/VP` rectangle could state, so the
//! expected position follows only if the form's own map is the one inverted (ADR 3030).

#![expect(
    clippy::arithmetic_side_effects,
    reason = "tiny exact-xref fixture arithmetic"
)]

use std::fmt::Write as _;

use viewer_core::{Answer, Command, DocumentId, Event, Query, Viewer};

/// The four corners of the unit square registered to whole degrees: latitude `10 + 2v`,
/// longitude `20 + 2u`, which every affine reading of the registration agrees on.
const GEO: &str = r#"<< /Type /Measure /Subtype /GEO /GCS << /Type /GEOGCS /WKT (GEOGCS["WGS 84",DATUM["WGS 84",SPHEROID["WGS 84",6378137,298.257223563]],PRIMEM["Greenwich",0],UNIT["Degree",0.0174532925199433]]) >> /GPTS [10 20 10 22 12 22 12 20] /LPTS [0 0 1 0 1 1 0 1] >>"#;

/// One PDF 2.0 page drawing one form. `/BBox [0 0 200 100]` under `/Matrix [0 0.5 -1 0 150 50]`
/// lands on page coordinates [50,50,150,150], with form x running up the page and form y
/// running to the left.
fn form_measure_pdf() -> Vec<u8> {
    let content = "/Fm Do";
    let form_content = "0 g 0 0 200 100 re f";
    let objects = [
        "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 400 400] /Resources << /XObject << /Fm 6 0 R >> >> /Contents 4 0 R >>".to_owned(),
        format!("<< /Length {} >>\nstream\n{content}\nendstream", content.len()),
        GEO.to_owned(),
        format!(
            "<< /Type /XObject /Subtype /Form /BBox [0 0 200 100] /Matrix [0 0.5 -1 0 150 50] \
             /Measure 5 0 R /Length {} >>\nstream\n{form_content}\nendstream",
            form_content.len()
        ),
    ];
    let mut out = String::from("%PDF-2.0\n");
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

#[test]
fn a_turned_form_s_own_measure_locates_a_point_in_its_bounds() {
    let bytes = form_measure_pdf();
    let parsed = pdf_syntax::Document::open(bytes.clone()).expect("valid exact-xref fixture");
    assert!(!parsed.was_recovered(), "fixture must use its exact xref");
    let mut viewer = Viewer::new(400, 400, 1.0);
    assert!(
        viewer
            .handle(Command::Open {
                id: DocumentId(72),
                bytes: bytes.into(),
                password: None,
                fragment: None,
            })
            .any(|event| matches!(event, Event::Opened { .. })),
        "the form fixture opens"
    );
    let Answer::Geometry(geometry) = viewer.query(Query::PageGeometry(0)) else {
        panic!("the page geometry supplies its displayed scale and origin");
    };
    let device = |x: f32, y: f32| {
        [
            geometry.origin.0 + x * geometry.scale,
            geometry.origin.1 + (geometry.page.height - y) * geometry.scale,
        ]
    };
    // Page (75, 100): form y = 150 − 75 = 75 and form x = (100 − 50) ÷ 0.5 = 100, so the unit
    // square's (u, v) = (100 ÷ 200, 75 ÷ 100) = (0.5, 0.75): latitude 11.5, longitude 21.
    let points = [device(125.0, 100.0), device(75.0, 100.0)];
    let answer = viewer.query(Query::Measure(&points));
    let Answer::Measured {
        located:
            Some(viewer_core::Located::At {
                latitude,
                longitude,
                ..
            }),
        ..
    } = answer
    else {
        panic!("expected the second point at 11.5 N, 21 E; got {answer:?}");
    };
    assert!((latitude - 11.5).abs() < 1e-6, "latitude {latitude}");
    assert!((longitude - 21.0).abs() < 1e-6, "longitude {longitude}");
}

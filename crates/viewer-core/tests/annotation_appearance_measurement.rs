//! An annotation's appearance stream's own `/Measure` reaches the measurement query, where §12.5.5
//! put its bounds.
//!
//! ISO 32000-2 §12.5.5: "Each appearance stream is a form `XObject`", so Table 95's `/Measure`, which
//! "specifies the scale and units which shall apply to the form", is an appearance's too, and Table
//! 269's `/LPTS` maps its unit square "to the rectangular bounds of the `Viewport`, image
//! `XObject`, or forms `XObject` that contains the measure dictionary". An appearance is not drawn
//! by a `Do`: its `/BBox` is carried by its `/Matrix` and §12.5.5's matrix `A` onto the
//! annotation's `/Rect`. This fixture turns the box a quarter by `/Matrix` and doubles it by `A`,
//! so the expected position follows only if `AA` is the map inverted (ADR 3061).

#![expect(
    clippy::arithmetic_side_effects,
    reason = "tiny exact-xref fixture arithmetic"
)]

use std::fmt::Write as _;

use viewer_core::{Answer, Command, DocumentId, Event, Query, Viewer};

/// The four corners of the unit square registered to whole degrees: latitude `10 + 2v`,
/// longitude `20 + 2u`, which every affine reading of the registration agrees on.
const GEO: &str = r#"<< /Type /Measure /Subtype /GEO /GCS << /Type /GEOGCS /WKT (GEOGCS["WGS 84",DATUM["WGS 84",SPHEROID["WGS 84",6378137,298.257223563]],PRIMEM["Greenwich",0],UNIT["Degree",0.0174532925199433]]) >> /GPTS [10 20 10 22 12 22 12 20] /LPTS [0 0 1 0 1 1 0 1] >>"#;

/// One PDF 2.0 page with no content and one `/Square` annotation whose normal appearance carries
/// [`GEO`]. `/BBox [0 0 200 100]` under `/Matrix [0 0.5 -1 0 150 50]` is the box
/// [50,50,150,150] (§12.5.5 step 1), and `/Rect [100 100 300 300]` makes `A` a scale of two
/// (step 2), so form (x, y) lands on page (300 − 2y, 100 + x).
fn appearance_measure_pdf() -> Vec<u8> {
    let form_content = "0 g 0 0 200 100 re f";
    let objects = [
        "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 400 400] /Annots [4 0 R] >>".to_owned(),
        "<< /Type /Annot /Subtype /Square /Rect [100 100 300 300] /AP << /N 6 0 R >> >>".to_owned(),
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
fn an_appearance_s_own_measure_locates_a_point_where_its_rect_put_it() {
    let bytes = appearance_measure_pdf();
    let parsed = pdf_syntax::Document::open(bytes.clone()).expect("valid exact-xref fixture");
    assert!(!parsed.was_recovered(), "fixture must use its exact xref");
    let mut viewer = Viewer::new(400, 400, 1.0);
    assert!(
        viewer
            .handle(Command::Open {
                id: DocumentId(73),
                bytes: bytes.into(),
                password: None,
                fragment: None,
            })
            .any(|event| matches!(event, Event::Opened { .. })),
        "the annotation fixture opens"
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
    // Page (150, 200): form x = 200 − 100 = 100 and form y = (300 − 150) ÷ 2 = 75, so the unit
    // square's (u, v) = (100 ÷ 200, 75 ÷ 100) = (0.5, 0.75): latitude 11.5, longitude 21. Without
    // `A` the same page point is outside the box altogether, and without `/Matrix` it is (0.75, 2).
    let points = [device(250.0, 200.0), device(150.0, 200.0)];
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

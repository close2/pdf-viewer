//! Does the live page measurement query consume a valid image `XObject`'s own GEO registration?
//!
//! Table 87 says an image `/Measure` supplies scale and units applicable to the image. §12.10.2
//! defines image-object bounds and its `LPTS` unit-square mapping. The page-VP control uses the same
//! registration on the same rectangle, making the expected location independent of the code path.
//! The image-only document has no page /VP, so a viewport-only implementation cannot satisfy it.

#![expect(
    clippy::arithmetic_side_effects,
    reason = "tiny exact-xref fixture arithmetic"
)]
#![expect(
    clippy::panic,
    clippy::expect_used,
    reason = "fixture or missing public answer must fail loudly"
)]

use std::fmt::Write as _;

use viewer_core::{Answer, Command, DocumentId, Event, Query, Viewer};

const GEO: &str = r#"<< /Type /Measure /Subtype /GEO /GCS << /Type /GEOGCS /WKT (GEOGCS["WGS 84",DATUM["WGS 84",SPHEROID["WGS 84",6378137,298.257223563]],PRIMEM["Greenwich",0],UNIT["Degree",0.0174532925199433]]) >> /GPTS [10 20 10 22 12 22 12 20] /LPTS [0 0 1 0 1 1 0 1] >>"#;

/// One PDF 2.0 page. A one-sample Gray image paints the unit square at page coordinates
/// [50,50,150,150], with /Measure on the image. The control instead attaches that same measure to
/// a page viewport covering the identical rectangle.
fn image_measure_pdf(page_viewport: bool) -> Vec<u8> {
    let content = "q 100 0 0 100 50 50 cm /Im Do Q";
    let content_obj = format!(
        "<< /Length {} >>\nstream\n{}\nendstream",
        content.len(),
        content
    );
    let page = if page_viewport {
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 400 400] /Resources << /XObject << /Im 6 0 R >> >> /Contents 4 0 R /VP [7 0 R] >>"
    } else {
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 400 400] /Resources << /XObject << /Im 6 0 R >> >> /Contents 4 0 R >>"
    };
    let image_measure = if page_viewport { "" } else { " /Measure 5 0 R" };
    let viewport = "<< /Type /Viewport /BBox [50 50 150 150] /Measure 5 0 R >>".to_owned();
    let image_obj = format!(
        "<< /Type /XObject /Subtype /Image /Width 1 /Height 1 /ColorSpace /DeviceGray \
         /BitsPerComponent 8 /Filter /ASCIIHexDecode /Length 3{image_measure} >>\nstream\n00>\nendstream"
    );
    let mut objects = vec![
        "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        page.to_owned(),
        content_obj,
        GEO.to_owned(),
        image_obj,
    ];
    if page_viewport {
        objects.push(viewport);
    }
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
        "trailer\n<< /Size {} /Root 1 0 R /ID [<00112233445566778899AABBCCDDEEFF> <00112233445566778899AABBCCDDEEFF>] >>\nstartxref\n{xref_at}\n%%EOF\n",
        objects.len() + 1
    );
    out.into_bytes()
}

fn open(bytes: Vec<u8>) -> Viewer {
    let parsed =
        pdf_syntax::Document::open(bytes.clone()).expect("valid PDF 2.0 exact-xref fixture");
    assert!(!parsed.was_recovered(), "fixture must use its exact xref");
    assert!(!parsed.cross_reference_chain_was_cut());
    let mut viewer = Viewer::new(400, 400, 1.0);
    assert!(
        viewer
            .handle(Command::Open {
                id: DocumentId(71),
                bytes: bytes.into(),
                password: None,
                fragment: None,
            })
            .any(|event| matches!(event, Event::Opened { .. })),
        "the geospatial image fixture opens"
    );
    viewer
}

fn position(viewer: &Viewer, source: &str) -> (f64, f64) {
    let Answer::Geometry(geometry) = viewer.query(Query::PageGeometry(0)) else {
        panic!("the page geometry supplies its actual displayed scale and origin");
    };
    let device = |x: f32, y: f32| {
        [
            geometry.origin.0 + x * geometry.scale,
            geometry.origin.1 + (geometry.page.height - y) * geometry.scale,
        ]
    };
    let points = [device(75.0, 100.0), device(125.0, 100.0)];
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
        panic!("{source}: expected the second point at 11 N, 21.5 E; got {answer:?}");
    };
    (latitude, longitude)
}

#[test]
fn the_viewport_control_locates_the_literal_registered_point() {
    // Same PDF coordinates: image bounds / viewport BBox are [50,50,150,150]. The public
    // PageGeometry answer supplies the actual origin and scale instead of assuming the viewer
    // centers a 400×400 page at device origin. The second point is local (0.75,0.5), whose
    // four-corner registration is latitude 11, longitude 21.5.
    let viewport_control = open(image_measure_pdf(true));
    let (latitude, longitude) = position(&viewport_control, "page viewport control");
    assert!(
        (latitude - 11.0).abs() < 1e-7,
        "viewport control latitude {latitude}"
    );
    assert!(
        (longitude - 21.5).abs() < 1e-7,
        "viewport control longitude {longitude}"
    );
}

#[test]
fn a_valid_image_measure_reaches_the_interactive_measurement_query() {
    let image_case = open(image_measure_pdf(false));
    let (latitude, longitude) = position(&image_case, "image Measure");
    assert!(
        (latitude - 11.0).abs() < 1e-7,
        "image measure latitude {latitude}"
    );
    assert!(
        (longitude - 21.5).abs() < 1e-7,
        "image measure longitude {longitude}"
    );
}

//! Actual-consumer regressions for ISO 32000-2 §12.3.2.2, Table 149.
//! Fixtures are PDF 1.7 with exact stream lengths, xref offsets and
//! startxref. This keeps them outside PDF 2.0's trailer-ID requirement.

#![expect(
    clippy::expect_used,
    clippy::panic,
    reason = "valid fixture and viewer queries must succeed"
)]

use std::fmt::Write as _;
use viewer_core::{Answer, Command, DocumentId, Event, Query, Viewer};

const DOCUMENT: DocumentId = DocumentId(92);

/// An open-action fixture with one painted 200×100 rectangle, clipped to a non-origin crop box.
/// The content is stated in PDF user space: [100,600,300,700]. Inside `CropBox` [50,80,550,780],
/// it lies 50 units from the left crop edge and 80 units below its top edge. The independent fit calculations are recorded in
/// each test, rather than asking production transform helpers for expected values.
fn content_fit_pdf(destination: &str) -> Vec<u8> {
    let content = "0 0 1 rg 100 600 200 100 re f\n";
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R /OpenAction {destination} >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 600 800] \
         /CropBox [50 80 550 780] /Resources << >> /Contents 4 0 R >>\nendobj\n\
         4 0 obj\n<< /Length {} >>\nstream\n{content}endstream\nendobj\n",
        content.len()
    );
    assemble(&body)
}

/// Builds xref entries from every contiguous object in `body`; each body ends in `endobj\n`.
fn assemble(body: &str) -> Vec<u8> {
    let mut out = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for object in body.split_inclusive("endobj\n") {
        offsets.push(out.len());
        out.push_str(object);
    }
    let xref_at = out.len();
    let size = offsets.len().saturating_add(1);
    let _ = writeln!(out, "xref\n0 {size}");
    out.push_str("0000000000 65535 f \n");
    for offset in &offsets {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    let _ = write!(
        out,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n"
    );
    out.into_bytes()
}

fn open_geometry(
    bytes: Vec<u8>,
    width: u32,
    height: u32,
    page: usize,
) -> viewer_core::PageGeometry {
    let document = pdf_syntax::Document::open(bytes.clone()).expect("complete PDF");
    assert!(!document.was_recovered(), "fixture has exact xref offsets");
    let mut viewer = Viewer::new(width, height, 1.0);
    let _ = viewer
        .handle(Command::Open {
            id: DOCUMENT,
            bytes: bytes.into(),
            password: None,
            fragment: None,
        })
        .count();
    let Answer::Geometry(geometry) = viewer.query(Query::PageGeometry(page)) else {
        panic!("the requested page has a layout");
    };
    geometry
}

#[test]
fn fitbh_fits_the_content_width_and_places_top_in_default_user_space() {
    // Table 149: FitBH fits the bounding box's full width. It is 200 units wide, so in a
    // 200×200 viewport the scale is 1.0. top=700 is measured in default user space; relative
    // to CropBox's upper edge y=780 its displayed distance from the top is 80, so origin y=-80.
    // Positive control: FitH uses the full 500-unit crop width, not the 200-unit content width;
    // its scale is 200/500=0.4 and its transformed top is 80×0.4=32 device pixels.
    let page_horizontal = open_geometry(content_fit_pdf("[3 0 R /FitH 700]"), 200, 200, 0);
    assert!(
        (page_horizontal.scale - 0.4).abs() < 1e-3,
        "FitH fits crop width 500: {}",
        page_horizontal.scale
    );
    assert!(
        (page_horizontal.origin.1 + 32.0).abs() < 2.0,
        "FitH uses the same default-space top through crop transform: {:?}",
        page_horizontal.origin
    );

    let bounding_horizontal = open_geometry(content_fit_pdf("[3 0 R /FitBH 700]"), 200, 200, 0);
    assert!(
        (bounding_horizontal.scale - 1.0).abs() < 1e-3,
        "FitBH fits 200 user units: {}",
        bounding_horizontal.scale
    );
    assert!(
        (bounding_horizontal.origin.1 + 80.0).abs() < 2.0,
        "user-space top=700 in crop [50 80 550 780]: {:?}",
        bounding_horizontal.origin
    );

    assert!(
        (bounding_horizontal.scale - page_horizontal.scale).abs() > 0.5,
        "the two fit boxes are observably different"
    );
}

#[test]
fn fitbv_fits_the_content_height_and_transforms_left_from_user_space() {
    // The painted box is 100 units high: 400/100=4.0. Its left coordinate 100 is 50 units
    // right of the crop's x=50 edge, so the page-space scroll is 50×4=200 pixels. Both values
    // are distinct from FitV's page-sized fit (400/700) and from treating x as display-space 100.
    // Positive control: FitV fits the full 700-unit crop height: 400/700. The same left
    // coordinate maps to x=50, so its origin is -(50×400/700), about -28.571 pixels.
    let page_vertical = open_geometry(content_fit_pdf("[3 0 R /FitV 100]"), 200, 400, 0);
    assert!(
        (page_vertical.scale - (400.0 / 700.0)).abs() < 1e-3,
        "FitV fits crop height 700: {}",
        page_vertical.scale
    );
    assert!(
        (page_vertical.origin.0 + (50.0 * 400.0 / 700.0)).abs() < 2.0,
        "FitV origin after crop transform: {:?}",
        page_vertical.origin
    );

    let bounding_vertical = open_geometry(content_fit_pdf("[3 0 R /FitBV 100]"), 200, 400, 0);
    assert!(
        (bounding_vertical.scale - 4.0).abs() < 1e-3,
        "FitBV fits content height 100: {}",
        bounding_vertical.scale
    );
    assert!(
        (bounding_vertical.origin.0 + 200.0).abs() < 2.0,
        "left=100 maps to x=50 in crop space: {:?}",
        bounding_vertical.origin
    );

    assert!(
        (bounding_vertical.scale - page_vertical.scale).abs() > 3.0,
        "the two fit boxes are observably different"
    );
}

/// A two-page link fixture. Page one opens at scale 2, left=120, top=500. Its /Dest is varied;
/// the target page has the same geometry so a null parameter should retain that component of
/// the current window view. Annotation object 6 is a valid local link carrier.
fn local_link_pdf(destination: &str) -> Vec<u8> {
    let content = "";
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R /OpenAction [3 0 R /XYZ 120 500 2] >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R 4 0 R] /Count 2 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 600 800] /Resources << >> \
         /Contents 5 0 R /Annots [6 0 R] >>\nendobj\n\
         4 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 600 800] /Resources << >> >>\nendobj\n\
         5 0 obj\n<< /Length {} >>\nstream\n{content}\nendstream\nendobj\n\
         6 0 obj\n<< /Type /Annot /Subtype /Link /Rect [0 0 50 50] /Dest {destination} >>\nendobj\n",
        content.len()
    );
    assemble(&body)
}

fn navigate(destination: &str, target_page: usize) -> viewer_core::PageGeometry {
    let bytes = local_link_pdf(destination);
    let document = pdf_syntax::Document::open(bytes.clone()).expect("complete link PDF");
    assert!(!document.was_recovered(), "link fixture has exact xref");
    let mut viewer = Viewer::new(300, 400, 1.0);
    let _ = viewer
        .handle(Command::Open {
            id: DOCUMENT,
            bytes: bytes.into(),
            password: None,
            fragment: None,
        })
        .count();
    let Answer::Geometry(before) = viewer.query(Query::PageGeometry(0)) else {
        panic!("the open action displays the first page");
    };
    let events: Vec<Event> = viewer
        .handle(Command::Activate(pdf_syntax::ObjectId::new(6, 0)))
        .collect();
    assert!(
        matches!(viewer.query(Query::CurrentPage), Answer::Page { index, .. } if index == target_page),
        "the link reaches the stated local page: {events:?}"
    );
    let Answer::Geometry(after) = viewer.query(Query::PageGeometry(target_page)) else {
        panic!("the destination page is displayed");
    };
    assert!(
        (before.scale - 2.0).abs() < 1e-3,
        "OpenAction established scale 2: {}",
        before.scale
    );
    assert!(
        (before.origin.0 + 240.0).abs() < 2.0 && (before.origin.1 + 600.0).abs() < 2.0,
        "OpenAction established the nonzero scroll: {:?}",
        before.origin
    );
    after
}

#[test]
fn xyz_null_coordinates_and_zero_zoom_retain_the_current_window_values() {
    // Table 149 says each null left/top/zoom retains that current parameter, and zoom zero is
    // equivalent to null. The source opens with scale 2 and 240×600 pixels of scroll; navigating
    // to an equal-sized page whose destination leaves all three fields unspecified preserves it.
    // Positive control: stated left/top are applied even while zero zoom retains scale 2.
    let stated_coordinates = navigate("[4 0 R /XYZ 40 700 0]", 1);
    assert!((stated_coordinates.scale - 2.0).abs() < 1e-3);
    assert!((stated_coordinates.origin.0 + 80.0).abs() < 2.0);
    assert!((stated_coordinates.origin.1 + 200.0).abs() < 2.0);

    // Second positive control: a positive zoom is honored, and coordinates still move the view.
    let explicit_zoom = navigate("[4 0 R /XYZ 40 700 1]", 1);
    assert!((explicit_zoom.scale - 1.0).abs() < 1e-3);
    assert!((explicit_zoom.origin.0 + 40.0).abs() < 2.0);
    assert!((explicit_zoom.origin.1 + 100.0).abs() < 2.0);

    let null = navigate("[4 0 R /XYZ null null null]", 1);
    let zero_zoom = navigate("[4 0 R /XYZ null null 0]", 1);
    for (label, geometry) in [("null zoom", null), ("zoom zero", zero_zoom)] {
        assert!(
            (geometry.scale - 2.0).abs() < 1e-3,
            "{label} keeps scale 2: {}",
            geometry.scale
        );
        assert!(
            (geometry.origin.0 + 240.0).abs() < 2.0,
            "{label} keeps horizontal location: {:?}",
            geometry.origin
        );
        assert!(
            (geometry.origin.1 + 600.0).abs() < 2.0,
            "{label} keeps vertical location: {:?}",
            geometry.origin
        );
    }
}

#[test]
fn xyz_null_coordinates_retain_user_position_when_zoom_changes_on_the_same_page() {
    let control = navigate("[3 0 R /XYZ 120 500 1]", 0);
    assert!((control.scale - 1.0).abs() < 1e-3);
    assert!((control.origin.0 + 120.0).abs() < 2.0);
    assert!((control.origin.1 + 300.0).abs() < 2.0);

    // The current left/top are 120/500 in default user space, not the old pixel scroll.
    let retained = navigate("[3 0 R /XYZ null null 1]", 0);
    assert!((retained.scale - 1.0).abs() < 1e-3);
    assert!(
        (retained.origin.0 + 120.0).abs() < 2.0,
        "left retained through zoom: {:?}",
        retained.origin
    );
    assert!(
        (retained.origin.1 + 300.0).abs() < 2.0,
        "top retained through zoom: {:?}",
        retained.origin
    );
}

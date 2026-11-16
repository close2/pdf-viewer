//! Table 149 coordinates through inherited /Rotate and a non-origin `CropBox`.
#![expect(clippy::expect_used, clippy::panic)]

use std::fmt::Write as _;

use viewer_core::{Answer, Command, DocumentId, Query, Viewer};

const DOCUMENT: DocumentId = DocumentId(194);

fn pdf(rotate: i32, named: bool) -> Vec<u8> {
    let open_action = if named {
        "<< /S /GoTo /D (chapter) >>"
    } else {
        "<< /S /GoTo /D [3 0 R /XYZ 150 680 0.5] >>"
    };
    let catalog_tree = if named {
        "/Names << /Dests 4 0 R >>"
    } else {
        ""
    };
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R /OpenAction {open_action} {catalog_tree} >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 /Rotate {rotate} >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [20 30 620 830] \
         /CropBox [50 80 550 780] /Resources << >> >>\nendobj\n\
         4 0 obj\n<< /Names [(chapter) 5 0 R] >>\nendobj\n\
         5 0 obj\n[3 0 R /XYZ 150 680 0.5]\nendobj\n"
    );
    assemble(&body)
}

fn assemble(body: &str) -> Vec<u8> {
    let mut out = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for object in body.split_inclusive("endobj\n") {
        offsets.push(out.len());
        out.push_str(object);
    }
    let xref = out.len();
    let size = offsets.len().saturating_add(1);
    let _ = writeln!(out, "xref\n0 {size}");
    out.push_str("0000000000 65535 f \n");
    for offset in offsets {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    let _ = write!(
        out,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n"
    );
    out.into_bytes()
}

fn geometry(rotate: i32, named: bool) -> viewer_core::PageGeometry {
    let bytes = pdf(rotate, named);
    let document = pdf_syntax::Document::open(bytes.clone()).expect("fixture opens");
    assert!(!document.was_recovered(), "all xref offsets are exact");

    let mut viewer = Viewer::new(50, 50, 1.0);
    let _ = viewer
        .handle(Command::Open {
            id: DOCUMENT,
            bytes: bytes.into(),
            password: None,
            fragment: None,
        })
        .count();
    viewer.handle(Command::Presented).for_each(drop);
    assert!(
        matches!(
            viewer.query(Query::CurrentPage),
            Answer::Page { index: 0, .. }
        ),
        "the destination presents the sole page"
    );
    let Answer::Geometry(result) = viewer.query(Query::PageGeometry(0)) else {
        panic!("the target page has settled geometry");
    };
    result
}

// CropBox [50,80,550,780] gives a 500x700 page and point offset (100,600).
// Clockwise 90 maps to (600,400) in rotated y-up coordinates, then (600,100)
// in display y-down coordinates. Clockwise 270 gives (100,100), then (100,400).
// At .5 scale those scroll offsets are (300,50) and (50,200). The 50x50 viewport
// is smaller than the scaled 350x250 page, and both offsets are within scroll bounds.
// Centering adds zero; no production transform computes these literal expectations.
fn assert_geometry(rotate: i32, named: bool, origin: (f32, f32)) {
    let actual = geometry(rotate, named);
    assert!((actual.scale - 0.5).abs() < 1e-3);
    assert_eq!((actual.page.width, actual.page.height), (700.0, 500.0));
    assert!(
        (actual.origin.0 - origin.0).abs() < 1.0,
        "x origin: {:?}",
        actual.origin
    );
    assert!(
        (actual.origin.1 - origin.1).abs() < 1.0,
        "y origin: {:?}",
        actual.origin
    );
}
#[test]
fn named_xyz_respects_inherited_clockwise_90_rotation_and_crop_offset() {
    assert_geometry(90, true, (-300.0, -50.0));
}
#[test]
fn named_xyz_respects_inherited_clockwise_270_rotation_and_crop_offset() {
    assert_geometry(270, true, (-50.0, -200.0));
}
#[test]
fn direct_xyz_has_the_same_independent_90_degree_geometry() {
    assert_geometry(90, false, (-300.0, -50.0));
}

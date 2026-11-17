//! Public viewer assertions for destination lookup and canonical spelling.
//! Distinguish a PDF 1.1 name-destination dictionary from a PDF 1.2 byte-string name tree
//! when both define the same decoded bytes but intentionally point to different page views.
//! The expected behavior follows §12.3.2.4's two type-specific correspondences: a name object
//! is defined by catalog /Dests; a string is defined by /Names /Dests. Annex J.3.3/J.3.4
//! canonicalize the respective objects, they do not collapse the two dictionaries' namespaces.

#![expect(
    clippy::expect_used,
    clippy::panic,
    clippy::arithmetic_side_effects,
    reason = "bounded exact-xref fixture and explicit query failure"
)]

use pdf_syntax::ObjectId;
use std::fmt::Write as _;
use viewer_core::{Answer, Command, DocumentId, Query, Viewer};

fn fixture(first_destination: &str, second_destination: &str) -> Vec<u8> {
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R /Dests 6 0 R /Names << /Dests 7 0 R >> >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R 4 0 R 5 0 R] /Count 3 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 600 800] /Resources << >> /Annots [8 0 R 9 0 R] >>\nendobj\n\
         4 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 600 800] /Resources << >> >>\nendobj\n\
         5 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 600 800] /Resources << >> >>\nendobj\n\
         6 0 obj\n<< /chapter [4 0 R /Fit] >>\nendobj\n\
         7 0 obj\n<< /Names [(chapter) [5 0 R /FitV 40]] >>\nendobj\n\
         8 0 obj\n<< /Type /Annot /Subtype /Link /Rect [0 0 50 50] /Dest {first_destination} >>\nendobj\n\
         9 0 obj\n<< /Type /Annot /Subtype /Link /Rect [60 0 110 50] /Dest {second_destination} >>\nendobj\n"
    );
    let mut out = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for object in body.split_inclusive("endobj\n") {
        offsets.push(out.len());
        out.push_str(object);
    }
    let xref_at = out.len();
    let size = offsets.len() + 1;
    let _ = writeln!(out, "xref\n0 {size}");
    out.push_str("0000000000 65535 f \n");
    for offset in offsets {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    let _ = write!(
        out,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n"
    );
    out.into_bytes()
}

fn activated(
    destination_one: &str,
    destination_two: &str,
    annotation: u32,
) -> viewer_core::PageGeometry {
    let bytes = fixture(destination_one, destination_two);
    let parsed = pdf_syntax::Document::open(bytes.clone()).expect("complete fixture");
    assert!(
        !parsed.was_recovered(),
        "the fixture has exact xref offsets"
    );
    let mut viewer = Viewer::new(200, 400, 1.0);
    let _ = viewer
        .handle(Command::Open {
            id: DocumentId(94),
            bytes: bytes.into(),
            password: None,
            fragment: None,
        })
        .count();
    let _ = viewer
        .handle(Command::Activate(ObjectId::new(annotation, 0)))
        .count();
    let expected_page = if annotation == 8 { 1 } else { 2 };
    assert!(
        matches!(viewer.query(Query::CurrentPage), Answer::Page { index, .. } if index == expected_page),
        "destination representation selects its own table's page"
    );
    let Answer::Geometry(geometry) = viewer.query(Query::PageGeometry(expected_page)) else {
        panic!("destination target has displayed geometry");
    };
    geometry
}

#[test]
fn a_name_destination_uses_the_catalog_dictionary_even_when_the_tree_has_the_same_key() {
    let control = activated("[4 0 R /Fit]", "(chapter)", 8);
    assert!((control.scale - (200.0 / 600.0)).abs() < 1e-3);
    for spelling in ["/chapter", "/ch#61pter"] {
        let named = activated(spelling, "(chapter)", 8);
        assert!((named.scale - (200.0 / 600.0)).abs() < 1e-3);
        assert_eq!(named.origin, control.origin);
    }
}

#[test]
fn a_string_destination_uses_the_name_tree_even_when_the_dictionary_has_the_same_key() {
    let control = activated("/chapter", "[5 0 R /FitV 40]", 9);
    assert!((control.scale - 0.5).abs() < 1e-3);
    assert!((control.origin.0 + 20.0).abs() < 2.0);
    for spelling in ["(chapter)", "<63686170746572>"] {
        let named = activated("/chapter", spelling, 9);
        assert!((named.scale - 0.5).abs() < 1e-3);
        assert!((named.origin.0 + 20.0).abs() < 2.0);
        assert_eq!(named.origin, control.origin);
    }
}

//! Table 99 optional-content presentation across simultaneously visible pages.

#![expect(
    clippy::arithmetic_side_effects,
    clippy::expect_used,
    clippy::panic,
    reason = "test fixture uses small fixed object counts and explanatory assertions"
)]

// Table 99 integration tests. It checks Table 99 Order/VisiblePages
// through the actual public Query::Layers when the current OneColumn arrangement shows two pages.
// Query::AccessibilityTree independently establishes that the public arrangement has two pages.
// The object writer below emits exact xref offsets; callers guard against recovery.

fn layers_pdf(list_mode: &str) -> Vec<u8> {
    use std::fmt::Write as _;
    let p1 = "/OC /a BDC 1 0 0 rg 10 10 20 20 re f EMC";
    let p2 = "/OC /b BDC 0 0 1 rg 40 10 20 20 re f EMC";
    let objects = [
        format!("<< /Type /Catalog /Pages 2 0 R /PageLayout /OneColumn /OCProperties << /OCGs [7 0 R 8 0 R] /D << /Order [[(Plans) 7 0 R 8 0 R]] /ListMode /{list_mode} >> >> >>"),
        "<< /Type /Pages /Kids [3 0 R 5 0 R] /Count 2 >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Resources << /Properties << /a 7 0 R >> >> /Contents 4 0 R >>".to_owned(),
        format!("<< /Length {} >>\nstream\n{p1}\nendstream", p1.len()),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Resources << /Properties << /b 8 0 R >> >> /Contents 6 0 R >>".to_owned(),
        format!("<< /Length {} >>\nstream\n{p2}\nendstream", p2.len()),
        "<< /Type /OCG /Name (First) >>".to_owned(),
        "<< /Type /OCG /Name (Second) >>".to_owned(),
    ];
    let mut bytes = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for (i, body) in objects.iter().enumerate() {
        offsets.push(bytes.len());
        let _ = write!(bytes, "{} 0 obj\n{body}\nendobj\n", i + 1);
    }
    let xref = bytes.len();
    let _ = write!(
        bytes,
        "xref\n0 {}\n0000000000 65535 f \n",
        objects.len().saturating_add(1)
    );
    for offset in offsets {
        let _ = writeln!(bytes, "{offset:010} 00000 n ");
    }
    let _ = write!(
        bytes,
        "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
        objects.len().saturating_add(1)
    );
    bytes.into_bytes()
}

use pdf_syntax::Document;
use viewer_core::{Answer, Command, DocumentId, Event, Layer, Query, Viewer, Zoom};

fn names(layers: &[Layer]) -> Vec<String> {
    let mut result = Vec::new();
    for layer in layers {
        match layer {
            Layer::Group { name, .. } => result.push(name.clone().unwrap_or_default()),
            Layer::Collection { children, .. } => result.extend(names(children)),
        }
    }
    result
}
fn queried(viewer: &Viewer) -> Vec<Layer> {
    match viewer.query(Query::Layers) {
        Answer::Layers(layers) => layers,
        other => panic!("expected Layers, got {other:?}"),
    }
}
fn open(list_mode: &str) -> Viewer {
    let bytes = layers_pdf(list_mode);
    let document = Document::open(bytes.clone()).expect("exact-xref fixture parses");
    assert!(!document.was_recovered(), "exact xref and stream lengths");
    let mut viewer = Viewer::new(120, 220, 1.0);
    let events: Vec<_> = viewer
        .handle(Command::Open {
            id: DocumentId(91),
            bytes: bytes.into(),
            password: None,
            fragment: None,
        })
        .collect();
    assert!(
        events
            .iter()
            .any(|e| matches!(e, Event::Opened { pages: 2, .. })),
        "{events:?}"
    );
    viewer
        .handle(Command::Zoom {
            zoom: Zoom::Scale(1.0),
            at: None,
        })
        .for_each(drop);
    let Answer::Accessibility(pages) = viewer.query(Query::AccessibilityTree) else {
        panic!("expected one structure answer per visible page");
    };
    assert_eq!(
        pages.len(),
        2,
        "OneColumn visibly places both fixture pages"
    );
    viewer
}
fn assert_both_groups(layers: &[Layer]) {
    assert_eq!(
        names(layers),
        vec![String::from("First"), String::from("Second")]
    );
    assert!(matches!(layers, [Layer::Collection { label: Some(label), .. }] if label == "Plans"));
}
#[test]
fn visible_pages_includes_every_page_in_the_current_arrangement() {
    // Two 100-unit pages and the inter-page gap fit at 1x in the 220-pixel viewport.
    assert_both_groups(&queried(&open("VisiblePages")));
}
#[test]
fn all_pages_keeps_both_groups_and_their_collection_label() {
    assert_both_groups(&queried(&open("AllPages")));
}

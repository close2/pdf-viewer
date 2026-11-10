//! Behaviorally verify a §12.3.2.4 name-tree destination is resolved and
//! its Table 149 view is actually applied.

use std::fmt::Write as _;
use viewer_core::{Answer, Command, DocumentId, Query, Viewer};

const DOCUMENT: DocumentId = DocumentId(91);

/// PDF 1.7 fixture with exact offsets and a two-page tree. The name tree maps a byte-string name
/// to the second page's `/FitV 40` view. The page is 600×800 and the viewport is 200×400, so
/// `/FitV` must choose scale 0.5 and put x=40 at the viewport's left edge (origin x=-20).
fn named_open_action_fixture(open_action: &str) -> Vec<u8> {
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R /OpenAction {open_action} \
         /Names << /Dests 5 0 R >> >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R 4 0 R] /Count 2 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 600 800] /Resources << >> >>\nendobj\n\
         4 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 600 800] /Resources << >> >>\nendobj\n\
         5 0 obj\n<< /Names [(chapter) [4 0 R /FitV 40]] >>\nendobj\n"
    );
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

#[expect(
    clippy::panic,
    reason = "a valid destination must expose page geometry"
)]
fn geometry(bytes: Vec<u8>) -> viewer_core::PageGeometry {
    let mut viewer = Viewer::new(200, 400, 1.0);
    let _ = viewer
        .handle(Command::Open {
            id: DOCUMENT,
            bytes: bytes.into(),
            password: None,
            fragment: None,
        })
        .count();
    assert!(
        matches!(
            viewer.query(Query::CurrentPage),
            Answer::Page { index: 1, .. }
        ),
        "the destination must select page two, not merely expose its geometry"
    );
    let Answer::Geometry(result) = viewer.query(Query::PageGeometry(1)) else {
        panic!("the resolved destination page is laid out");
    };
    result
}

#[test]
fn a_name_tree_destination_changes_the_page_view() {
    let named = geometry(named_open_action_fixture("<< /S /GoTo /D (chapter) >>"));
    assert!(
        (named.scale - 0.5).abs() < 1e-3,
        "FitV fits page height: {}",
        named.scale
    );
    assert!(
        (named.origin.0 + 20.0).abs() < 2.0,
        "left=40 at scale 0.5: {:?}",
        named.origin
    );

    // Positive control: the same target and view written directly must produce the same result.
    let direct = geometry(named_open_action_fixture("[4 0 R /FitV 40]"));
    assert!((direct.scale - 0.5).abs() < 1e-3);
    assert!((direct.origin.0 + 20.0).abs() < 2.0);
    assert_eq!(named.width, direct.width);
    assert_eq!(named.height, direct.height);
}

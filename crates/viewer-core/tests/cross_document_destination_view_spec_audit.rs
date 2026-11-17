//! ISO 32000-2 destinations preserve the target location and zoom across GoToR/GoToE.

#![expect(
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    clippy::panic,
    reason = "test fixtures and public viewer queries must fail loudly rather than pass by doing nothing"
)]

use std::fmt::Write as _;

use viewer_core::{Answer, Command, DocumentId, Event, Purpose, Query, Viewer};

const SOURCE_ID: DocumentId = DocumentId(81);

fn assembled(objects: &[String]) -> Vec<u8> {
    let mut out = String::from("%PDF-1.7\n");
    let mut offsets = Vec::with_capacity(objects.len());
    for (index, body) in objects.iter().enumerate() {
        offsets.push(out.len());
        let _ = writeln!(out, "{} 0 obj", index + 1);
        out.push_str(body);
        out.push_str("\nendobj\n");
    }
    let xref_at = out.len();
    let size = objects.len() + 1;
    let _ = write!(out, "xref\n0 {size}\n0000000000 65535 f \n");
    for offset in offsets {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    let _ = write!(
        out,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n"
    );
    out.into_bytes()
}

fn target(open_action: bool) -> Vec<u8> {
    let catalog_open = if open_action {
        "/OpenAction [5 0 R /XYZ 40 700 2]"
    } else {
        ""
    };
    assembled(&[
        format!("<< /Type /Catalog /Pages 2 0 R {catalog_open} >>"),
        "<< /Type /Pages /Count 3 /Kids [3 0 R 4 0 R 5 0 R] >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 600 800] /Resources << >> >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 600 800] /Resources << >> >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 600 800] /Resources << >> >>".to_owned(),
    ])
}

fn remote_source() -> Vec<u8> {
    assembled(&[
        "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
        "<< /Type /Pages /Count 1 /Kids [3 0 R] >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Resources << >> /Annots [4 0 R] >>"
            .to_owned(),
        "<< /Type /Annot /Subtype /Link /Rect [0 0 50 50] /A << /Type /Action /S /GoToR \
         /F (target.pdf) /D [2 /XYZ 40 700 2] >> >>"
            .to_owned(),
    ])
}

fn embedded_root(child: &[u8]) -> Vec<u8> {
    let mut stream = format!(
        "<< /Type /EmbeddedFile /Subtype /application#2Fpdf /Length {} >>\nstream\n",
        child.len()
    )
    .into_bytes();
    stream.extend_from_slice(child);
    stream.extend_from_slice(b"\nendstream");
    let stream = String::from_utf8(stream).expect("the embedded child fixture is ASCII");
    assembled(&[
        "<< /Type /Catalog /Pages 2 0 R /Names << /EmbeddedFiles << /Names [(child.pdf) 5 0 R] >> >> >>".to_owned(),
        "<< /Type /Pages /Count 1 /Kids [3 0 R] >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Resources << >> >>".to_owned(),
        stream,
        "<< /Type /Filespec /F (child.pdf) /UF (child.pdf) /EF << /F 4 0 R /UF 4 0 R >> >>".to_owned(),
    ])
}

fn embedded_source() -> Vec<u8> {
    assembled(&[
        "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
        "<< /Type /Pages /Count 1 /Kids [3 0 R] >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Resources << >> /Annots [4 0 R] >>"
            .to_owned(),
        "<< /Type /Annot /Subtype /Link /Rect [0 0 50 50] /A << /Type /Action /S /GoToE \
         /F (root.pdf) /T << /R /C /N (child.pdf) >> /D [2 /XYZ 40 700 2] >> >>"
            .to_owned(),
    ])
}

fn local_goto_source() -> Vec<u8> {
    assembled(&[
        "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
        "<< /Type /Pages /Count 2 /Kids [3 0 R 4 0 R] >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 600 800] /Resources << >> /Annots [5 0 R] >>"
            .to_owned(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 600 800] /Resources << >> >>".to_owned(),
        "<< /Type /Annot /Subtype /Link /Rect [0 0 50 50] /A << /Type /Action /S /GoTo \
         /D [4 0 R /XYZ 40 700 2] >> >>"
            .to_owned(),
    ])
}

fn assert_exact_xref(bytes: &[u8]) {
    let parsed = pdf_syntax::Document::open(bytes.to_vec()).expect("valid fixture PDF");
    assert!(!parsed.was_recovered(), "fixture must use its exact xref");
}

fn geometry(viewer: &Viewer) -> viewer_core::PageGeometry {
    let Answer::Page { index: 2, .. } = viewer.query(Query::CurrentPage) else {
        panic!("the action's integer destination names target page index 2");
    };
    let Answer::Geometry(geometry) = viewer.query(Query::PageGeometry(2)) else {
        panic!("target page 2 has geometry");
    };
    geometry
}

fn assert_explicit_xyz(geometry: viewer_core::PageGeometry, context: &str) {
    assert!(
        (geometry.scale - 2.0).abs() < 1e-3,
        "{context}: XYZ zoom=2: {geometry:?}"
    );
    // With a 600×800 page and 400×400 viewport, (40,700) at 2× means 80px left
    // and 200px down from the page's top-left, hence origin (-80,-200).
    assert!(
        (geometry.origin.0 + 80.0).abs() < 2.0 && (geometry.origin.1 + 200.0).abs() < 2.0,
        "{context}: XYZ coordinates are applied in the target window: {geometry:?}"
    );
}

fn open_target_directly(bytes: Vec<u8>) -> viewer_core::PageGeometry {
    assert_exact_xref(&bytes);
    let mut viewer = Viewer::new(400, 400, 1.0);
    viewer
        .handle(Command::Open {
            id: SOURCE_ID,
            bytes: bytes.into(),
            password: None,
            fragment: None,
        })
        .for_each(drop);
    geometry(&viewer)
}

#[test]
fn a_local_goto_action_applies_its_destination_view() {
    let bytes = local_goto_source();
    assert_exact_xref(&bytes);
    let mut viewer = Viewer::new(400, 400, 1.0);
    let opened: Vec<Event> = viewer
        .handle(Command::Open {
            id: SOURCE_ID,
            bytes: bytes.into(),
            password: None,
            fragment: None,
        })
        .collect();
    assert!(
        opened
            .iter()
            .any(|event| matches!(event, Event::Opened { .. }))
    );
    assert!(
        !opened
            .iter()
            .any(|event| matches!(event, Event::Reported { .. }))
    );
    let followed: Vec<Event> = viewer
        .handle(Command::Activate(pdf_syntax::ObjectId::new(5, 0)))
        .collect();
    assert!(
        followed
            .iter()
            .any(|event| matches!(event, Event::PageChanged { index: 1, .. }))
    );
    let Answer::Geometry(geometry) = viewer.query(Query::PageGeometry(1)) else {
        panic!("GoTo action's page is displayed");
    };
    assert_explicit_xyz(geometry, "local GoTo control");
}

#[test]
fn a_remote_goto_applies_the_explicit_destination_view_in_the_target() {
    // Positive geometry control: the same target and XYZ have independently expected geometry
    // when the destination comes from its catalog OpenAction.
    assert_explicit_xyz(
        open_target_directly(target(true)),
        "direct OpenAction control",
    );

    let source = remote_source();
    let target_bytes = target(false);
    assert_exact_xref(&source);
    assert_exact_xref(&target_bytes);
    let mut viewer = Viewer::new(400, 400, 1.0);
    let opened: Vec<Event> = viewer
        .handle(Command::Open {
            id: SOURCE_ID,
            bytes: source.into(),
            password: None,
            fragment: None,
        })
        .collect();
    assert!(
        opened
            .iter()
            .any(|event| matches!(event, Event::Opened { .. }))
    );
    assert!(
        !opened
            .iter()
            .any(|event| matches!(event, Event::Reported { .. }))
    );
    let asked: Vec<Event> = viewer
        .handle(Command::Activate(pdf_syntax::ObjectId::new(4, 0)))
        .collect();
    assert!(asked.iter().any(|event| matches!(
        event,
        Event::NeedsFile { purpose: Purpose::RemoteDocument, name, .. } if name == "target.pdf"
    )));
    let supplied: Vec<Event> = viewer
        .handle(Command::Supply {
            purpose: Purpose::RemoteDocument,
            bytes: Some(target_bytes),
        })
        .collect();
    assert!(
        supplied
            .iter()
            .any(|event| matches!(event, Event::Opened { pages: 3, .. }))
    );
    assert!(
        supplied
            .iter()
            .any(|event| matches!(event, Event::PageChanged { index: 2, .. }))
    );
    assert_explicit_xyz(geometry(&viewer), "cross-document destination after Supply");
}

#[test]
fn an_embedded_goto_applies_the_explicit_destination_view_in_the_target() {
    assert_explicit_xyz(
        open_target_directly(target(true)),
        "direct OpenAction control",
    );

    let source = embedded_source();
    let child = target(false);
    let root = embedded_root(&child);
    assert_exact_xref(&source);
    assert_exact_xref(&child);
    assert_exact_xref(&root);
    let mut viewer = Viewer::new(400, 400, 1.0);
    let opened: Vec<Event> = viewer
        .handle(Command::Open {
            id: SOURCE_ID,
            bytes: source.into(),
            password: None,
            fragment: None,
        })
        .collect();
    assert!(
        opened
            .iter()
            .any(|event| matches!(event, Event::Opened { .. }))
    );
    assert!(
        !opened
            .iter()
            .any(|event| matches!(event, Event::Reported { .. }))
    );
    let asked: Vec<Event> = viewer
        .handle(Command::Activate(pdf_syntax::ObjectId::new(4, 0)))
        .collect();
    assert!(asked.iter().any(|event| matches!(
        event,
        Event::NeedsFile { purpose: Purpose::TargetRoot, name, .. } if name == "root.pdf"
    )));
    let supplied: Vec<Event> = viewer
        .handle(Command::Supply {
            purpose: Purpose::TargetRoot,
            bytes: Some(root),
        })
        .collect();
    assert!(
        supplied
            .iter()
            .any(|event| matches!(event, Event::Opened { pages: 3, .. }))
    );
    assert!(
        supplied
            .iter()
            .any(|event| matches!(event, Event::PageChanged { index: 2, .. }))
    );
    assert_explicit_xyz(geometry(&viewer), "cross-document destination after Supply");
}

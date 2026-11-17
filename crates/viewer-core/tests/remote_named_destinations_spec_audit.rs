//! Public consumer control for Table 203's three /D representations.

#![expect(
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    reason = "fixture and public event assertions must fail loudly"
)]

use std::fmt::Write as _;
use viewer_core::{Command, DocumentId, Event, Purpose, Viewer};

const VIEW: DocumentId = DocumentId(43);

fn assembled(objects: &[String]) -> Vec<u8> {
    let mut out = String::from("%PDF-1.7\n");
    let mut offsets = Vec::with_capacity(objects.len());
    for (index, body) in objects.iter().enumerate() {
        offsets.push(out.len());
        let _ = writeln!(out, "{} 0 obj", index + 1);
        out.push_str(body);
        out.push_str("\nendobj\n");
    }
    let xref = out.len();
    let size = objects.len() + 1;
    let _ = write!(out, "xref\n0 {size}\n0000000000 65535 f \n");
    for offset in offsets {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    let _ = write!(
        out,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n"
    );
    out.into_bytes()
}

fn source(destination: &str) -> Vec<u8> {
    assembled(&[
        "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
        "<< /Type /Pages /Count 1 /Kids [3 0 R] >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Resources << >> /Annots [4 0 R] >>"
            .to_owned(),
        format!(
            "<< /Type /Annot /Subtype /Link /Rect [0 0 50 50] /A << /Type /Action /S /GoToR /F (target.pdf) /D {destination} >> >>"
        ),
    ])
}

fn target(destination_table: &str) -> Vec<u8> {
    let catalog = match destination_table {
        "name-tree" => "<< /Type /Catalog /Pages 2 0 R /Names << /Dests 6 0 R >> >>",
        "legacy-dests" => "<< /Type /Catalog /Pages 2 0 R /Dests 6 0 R >>",
        _ => "<< /Type /Catalog /Pages 2 0 R >>",
    };
    let mut objects = vec![
        catalog.to_owned(),
        "<< /Type /Pages /Count 3 /Kids [3 0 R 4 0 R 5 0 R] >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Resources << >> >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Resources << >> >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Resources << >> >>".to_owned(),
    ];
    let page_destination = "[5 0 R /Fit]";
    objects.push(if destination_table == "name-tree" {
        format!("<< /Names [(chapter) {page_destination}] >>")
    } else {
        format!("<< /chapter {page_destination} >>")
    });
    assembled(&objects)
}

fn assert_exact(bytes: &[u8]) {
    let document = pdf_syntax::Document::open(bytes.to_vec()).expect("valid exact-xref PDF");
    assert!(!document.cross_reference_chain_was_cut());
    assert!(
        !document.was_recovered(),
        "fixture must not depend on xref recovery"
    );
}

fn assert_remote_target(destination: &str, target_kind: &str) {
    let source = source(destination);
    let target = target(target_kind);
    assert_exact(&source);
    assert_exact(&target);

    let mut viewer = Viewer::new(400, 400, 1.0);
    let opened: Vec<Event> = viewer
        .handle(Command::Open {
            id: VIEW,
            bytes: source.into(),
            password: None,
            fragment: None,
        })
        .collect();
    assert!(
        opened
            .iter()
            .any(|event| matches!(event, Event::Opened { pages: 1, .. }))
    );
    assert!(
        !opened
            .iter()
            .any(|event| matches!(event, Event::Reported { .. }))
    );

    let request: Vec<Event> = viewer
        .handle(Command::Activate(pdf_syntax::ObjectId::new(4, 0)))
        .collect();
    assert!(request.iter().any(|event| matches!(
        event,
        Event::NeedsFile { purpose: Purpose::RemoteDocument, name, .. } if name == "target.pdf"
    )));
    let supplied: Vec<Event> = viewer
        .handle(Command::Supply {
            purpose: Purpose::RemoteDocument,
            bytes: Some(target),
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
            .any(|event| matches!(event, Event::PageChanged { index: 2, .. })),
        "Table 203 /D {destination} resolves page index 2 in the target, not in the one-page source"
    );
}

#[test]
fn remote_goto_explicit_name_and_byte_string_destinations_resolve_in_the_target() {
    // Independent explicit-array control before the two named forms.
    assert_remote_target("[2 /Fit]", "none");
    // Table 203 Name maps to the legacy Dests dictionary; byte string maps to Names/Dests.
    assert_remote_target("/chapter", "legacy-dests");
    assert_remote_target("(chapter)", "name-tree");
}

//! Public-viewer probe for §12.6.4.8 Table 210's `/IsMap` (physical p.534 / printed p.519).

#![expect(clippy::expect_used, clippy::panic, clippy::arithmetic_side_effects)]

use std::fmt::Write as _;

use viewer_core::{Answer, Command, DocumentId, Event, PointerAction, Query, Viewer};

fn pdf(is_map: Option<bool>, open_action: bool) -> Vec<u8> {
    let map = is_map.map_or(String::new(), |value| format!(" /IsMap {value}"));
    let objects = [
        format!(
            "<< /Type /Catalog /Pages 2 0 R {} >>",
            if open_action { "/OpenAction 5 0 R" } else { "" }
        ),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Resources << >> /Annots [4 0 R] >>"
            .to_owned(),
        "<< /Type /Annot /Subtype /Link /Rect [10 10 30 30] /A 5 0 R >>".to_owned(),
        format!("<< /S /URI /URI (https://example.invalid/map){map} >>"),
    ];
    let mut out = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for (index, body) in objects.iter().enumerate() {
        offsets.push(out.len());
        let _ = writeln!(out, "{} 0 obj\n{body}\nendobj", index + 1);
    }
    let xref = out.len();
    let _ = writeln!(out, "xref\n0 {}\n0000000000 65535 f ", objects.len() + 1);
    for offset in offsets {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    let _ = write!(
        out,
        "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
        objects.len() + 1
    );
    out.into_bytes()
}

fn click(is_map: Option<bool>, point: (f32, f32)) -> String {
    let bytes = pdf(is_map, false);
    let parsed = pdf_syntax::Document::open(bytes.clone()).expect("valid exact-xref fixture");
    assert!(!parsed.was_recovered(), "no recovery may mask the input");
    let mut viewer = Viewer::new(200, 200, 1.0);
    viewer
        .handle(Command::Open {
            id: DocumentId(1),
            bytes: bytes.into(),
            password: None,
            fragment: None,
        })
        .for_each(drop);
    viewer.handle(Command::Presented).for_each(drop);
    let Answer::Geometry(geometry) = viewer.query(Query::PageGeometry(0)) else {
        panic!("one page has geometry");
    };
    // Default user-space cursor, transformed through the public page geometry.
    let at = (
        geometry.origin.0 + point.0 * geometry.scale,
        geometry.origin.1 + (100.0 - point.1) * geometry.scale,
    );
    assert!(matches!(
        viewer.query(Query::LinkAt(at)),
        Answer::Link(true)
    ));
    viewer
        .handle(Command::Pointer {
            at,
            action: PointerAction::Pressed,
        })
        .for_each(drop);
    viewer
        .handle(Command::Pointer {
            at,
            action: PointerAction::Released,
        })
        .find_map(|event| match event {
            Event::OpenUri { uri, .. } => Some(uri),
            _ => None,
        })
        .expect("the link hands its resolved URI to the host")
}

#[test]
fn is_map_uses_the_clicked_point_and_false_or_absent_leaves_uri_alone() {
    assert_eq!(
        click(Some(true), (20.0, 24.0)),
        "https://example.invalid/map?10,6",
        "offset is from Rect's upper-left in user space"
    );
    assert_eq!(
        click(None, (20.0, 24.0)),
        "https://example.invalid/map",
        "Table 210 defaults IsMap to false"
    );
    assert_eq!(
        click(Some(false), (20.0, 24.0)),
        "https://example.invalid/map",
        "explicit false is the same observable control"
    );
}

#[test]
fn is_map_rounds_user_space_offsets_after_the_click_transform() {
    assert_eq!(
        click(Some(true), (20.4, 23.6)),
        "https://example.invalid/map?10,6"
    );
    assert_eq!(
        click(Some(true), (20.6, 23.4)),
        "https://example.invalid/map?11,7"
    );
}

#[test]
fn is_map_is_ignored_for_a_catalog_open_action() {
    let bytes = pdf(Some(true), true);
    let parsed = pdf_syntax::Document::open(bytes.clone()).expect("valid fixture");
    assert!(!parsed.was_recovered());
    let mut viewer = Viewer::new(200, 200, 1.0);
    let mut uris = Vec::new();
    for command in [
        Command::Open {
            id: DocumentId(1),
            bytes: bytes.into(),
            password: None,
            fragment: None,
        },
        Command::Presented,
    ] {
        for event in viewer.handle(command) {
            if let Event::OpenUri { uri, .. } = event {
                uris.push(uri);
            }
        }
    }
    assert_eq!(uris, ["https://example.invalid/map"]);
}

//! A Table 210 `/IsMap` action stays bare when reached from an outline item, but appends
//! independently derived click offsets when the same URI action is reached by a link click.
//! This exercises the public Viewer handoff only; it never opens a URI or invokes a handler.

#![expect(
    clippy::expect_used,
    reason = "invalid PDF layout or missing public event must fail the audit regression"
)]

use std::fmt::Write as _;

use viewer_core::{Answer, Command, DocumentId, Event, PointerAction, Query, Viewer};

const DOC: DocumentId = DocumentId(1);
const URI: &str = "https://example.invalid/map";
const OUTLINE_ITEM: pdf_syntax::ObjectId = pdf_syntax::ObjectId::new(7, 0);

fn fixture() -> Vec<u8> {
    let objects = [
        b"<< /Type /Catalog /Pages 2 0 R /Outlines 6 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Resources << >> /Annots [4 0 R] >>"
            .to_vec(),
        b"<< /Type /Annot /Subtype /Link /P 3 0 R /Rect [10 10 30 30] /A 5 0 R >>".to_vec(),
        format!("<< /S /URI /URI ({URI}) /IsMap true >>").into_bytes(),
        b"<< /Type /Outlines /First 7 0 R /Last 7 0 R /Count 1 >>".to_vec(),
        b"<< /Title (Map action) /Parent 6 0 R /A 5 0 R >>".to_vec(),
    ];
    let mut pdf = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for (index, body) in objects.iter().enumerate() {
        offsets.push(pdf.len());
        let _ = writeln!(pdf, "{} 0 obj", index.saturating_add(1));
        pdf.push_str(std::str::from_utf8(body).expect("fixture dictionaries are ASCII"));
        pdf.push_str("\nendobj\n");
    }
    let xref = pdf.len();
    let size = objects.len().saturating_add(1);
    let _ = write!(pdf, "xref\n0 {size}\n0000000000 65535 f \n");
    for offset in offsets {
        let _ = writeln!(pdf, "{offset:010} 00000 n ");
    }
    let _ = write!(
        pdf,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n"
    );
    pdf.into_bytes()
}

fn opened() -> Viewer {
    let bytes = fixture();
    let parsed =
        pdf_syntax::Document::open(bytes.clone()).expect("exact-xref PDF 1.7 fixture opens");
    assert!(!parsed.was_recovered(), "no-recovery guard");
    assert!(!parsed.cross_reference_chain_was_cut());
    let mut viewer = Viewer::new(200, 200, 1.0);
    let events: Vec<Event> = viewer
        .handle(Command::Open {
            id: DOC,
            bytes: bytes.into(),
            password: None,
            fragment: None,
        })
        .collect();
    assert!(
        events
            .iter()
            .any(|event| matches!(event, Event::Opened { document, pages: 1 } if *document == DOC))
    );
    viewer.handle(Command::Presented).for_each(drop);
    assert!(matches!(
        viewer.query(Query::CurrentPage),
        Answer::Page { index: 0, .. }
    ));
    viewer
}

fn uri(events: impl IntoIterator<Item = Event>) -> String {
    events
        .into_iter()
        .find_map(|event| match event {
            Event::OpenUri { uri, .. } => Some(uri),
            _ => None,
        })
        .expect("the supported action is handed to the caller")
}

#[test]
fn is_map_applies_only_to_the_annotation_click_not_the_outline_action() {
    let mut outline_viewer = opened();
    assert_eq!(
        uri(outline_viewer.handle(Command::Activate(OUTLINE_ITEM))),
        URI,
        "Table 210 explicitly ignores IsMap for an outline-item action"
    );

    let mut click_viewer = opened();
    let Answer::Geometry(geometry) = click_viewer.query(Query::PageGeometry(0)) else {
        panic!("opened one-page document has geometry");
    };
    // User-space point (20,24) inside Rect [10,10,30,30]. Table 210's independent formula:
    // x = 20 - 10 = 10; y = 30 - 24 = 6, measured down from Rect's upper-left.
    let at = (
        geometry.origin.0 + 20.0 * geometry.scale,
        geometry.origin.1 + (100.0 - 24.0) * geometry.scale,
    );
    assert!(matches!(
        click_viewer.query(Query::LinkAt(at)),
        Answer::Link(true)
    ));
    click_viewer
        .handle(Command::Pointer {
            at,
            action: PointerAction::Pressed,
        })
        .for_each(drop);
    assert_eq!(
        uri(click_viewer.handle(Command::Pointer {
            at,
            action: PointerAction::Released
        })),
        format!("{URI}?10,6"),
        "the same action appends coordinates when triggered by a user click"
    );
}

//! Public link-triggered transition action, independently of page Trans or presentation mode.
//! This tests the host event contract, not visual transition timing or action-sequence painting.

#![expect(
    clippy::arithmetic_side_effects,
    reason = "small exact-xref test fixtures; malformed setup must fail loudly"
)]

use std::fmt::Write as _;

use pdf_syntax::Document;
use viewer_core::{Answer, Command, DocumentId, Event, Query, Viewer};

fn document_bytes() -> Vec<u8> {
    let objects = [
        "<< /Type /Catalog /Pages 2 0 R >>",
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Resources << >> \
          /Annots [4 0 R] >>",
        "<< /Type /Annot /Subtype /Link /P 3 0 R /Rect [5 5 25 25] /Border [0 0 0] \
          /A << /S /Trans /Trans << /S /Wipe /D 2 /Di 180 >> >> >>",
    ];
    let mut bytes = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for (index, body) in objects.iter().enumerate() {
        offsets.push(bytes.len());
        let _ = writeln!(bytes, "{} 0 obj\n{body}\nendobj", index + 1);
    }
    let xref_at = bytes.len();
    let size = objects.len() + 1;
    let _ = writeln!(bytes, "xref\n0 {size}\n0000000000 65535 f ");
    for offset in offsets {
        let _ = writeln!(bytes, "{offset:010} 00000 n ");
    }
    let _ = write!(
        bytes,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n"
    );
    bytes.into_bytes()
}

#[test]
fn a_transition_action_is_delivered_after_link_activation_without_page_navigation() {
    let bytes = document_bytes();
    let syntax = Document::open(bytes.clone()).expect("exact-xref PDF opens");
    assert!(!syntax.was_recovered(), "fixture xref offsets are exact");

    let mut viewer = Viewer::new(300, 200, 1.0);
    let opened: Vec<Event> = viewer
        .handle(Command::Open {
            id: DocumentId(1),
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
            .any(|event| matches!(event, Event::Transition { .. }))
    );
    let events: Vec<Event> = viewer
        .handle(Command::Activate(pdf_syntax::ObjectId::new(4, 0)))
        .collect();

    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(event, Event::Transition { .. }))
            .count(),
        1
    );
    let transition = events.iter().find_map(|event| match event {
        Event::Transition { transition, .. } => Some(transition),
        _ => None,
    });
    let transition =
        transition.expect("the action asks the interactive processor to display a transition");
    assert!((transition.duration - 2.0).abs() < f32::EPSILON);
    assert_eq!(transition.style, pdf_model::navigation::Style::Wipe);
    assert_eq!(
        transition.direction,
        pdf_model::navigation::Direction::Degrees(180.0)
    );
    let Answer::Page { index, .. } = viewer.query(Query::CurrentPage) else {
        panic!("open viewer reports the current page");
    };
    assert_eq!(
        index, 0,
        "a transition action displays the current state; it is not a go-to"
    );
}

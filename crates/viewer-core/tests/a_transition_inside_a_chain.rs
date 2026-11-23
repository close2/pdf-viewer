//! ISO 32000-2 §12.6.4.15's transition action inside a §12.6.2 `/Next` chain (ADR 3497).
//!
//! The clause has the processor "render the state of the page viewing area as it exists after
//! completion of the previous action and display it using a transition". Where no later action
//! moves the page, that state is the chain's end, and the transition is named before the jump to
//! it; where a later page action moves on, the transition is still named and the difference
//! between the page it belonged to and the page the chain ends on is said out loud.
#![expect(
    clippy::expect_used,
    clippy::panic,
    clippy::arithmetic_side_effects,
    reason = "small fixed fixtures whose failure must be loud, and exact xref offsets"
)]

use std::fmt::Write as _;

use pdf_syntax::{Document, ObjectId};
use viewer_core::{Answer, Command, DocumentId, Event, Query, Viewer};

/// Three pages; the link on the first (object 6) performs `action`.
fn three_pages_linking(action: &str) -> Vec<u8> {
    let link = format!("<< /Type /Annot /Subtype /Link /Rect [0 0 50 50] /P 3 0 R /A {action} >>");
    let objects = [
        "<< /Type /Catalog /Pages 2 0 R >>",
        "<< /Type /Pages /Count 3 /Kids [3 0 R 4 0 R 5 0 R] >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Resources << >> /Annots [6 0 R] >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Resources << >> >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Resources << >> >>",
        &link,
    ];
    let mut out = String::from("%PDF-2.0\n");
    let mut offsets = Vec::with_capacity(objects.len());
    for (index, body) in objects.iter().enumerate() {
        offsets.push(out.len());
        let _ = write!(out, "{} 0 obj\n{body}\nendobj\n", index + 1);
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
    let bytes = out.into_bytes();
    let parsed = Document::open(bytes.clone()).expect("a complete fixture opens");
    assert!(!parsed.was_recovered(), "the fixture's offsets are exact");
    bytes
}

/// The page shown after the link performs `action`, the events it raised, and the sentences.
fn activated(action: &str) -> (usize, Vec<Event>, Vec<String>) {
    let mut viewer = Viewer::new(400, 400, 1.0);
    viewer
        .handle(Command::Open {
            id: DocumentId(1),
            bytes: three_pages_linking(action).into(),
            password: None,
            fragment: None,
        })
        .for_each(drop);
    let events: Vec<Event> = viewer
        .handle(Command::Activate(ObjectId::new(6, 0)))
        .collect();
    let Answer::Page { index, .. } = viewer.query(Query::CurrentPage) else {
        panic!("an opened viewer has a current page");
    };
    let notes = events
        .iter()
        .filter_map(|event| match event {
            Event::Reported { notes, .. } => Some(notes.clone()),
            _ => None,
        })
        .flatten()
        .collect();
    (index, events, notes)
}

fn transitions(events: &[Event]) -> usize {
    events
        .iter()
        .filter(|event| matches!(event, Event::Transition { .. }))
        .count()
}

const WIPE: &str = "<< /S /Trans /Trans << /S /Wipe /D 1 >> >>";

/// A transition after the chain's last page action shows the page the chain ends on, and is
/// named with nothing else said.
#[test]
fn a_transition_after_the_last_page_action_shows_the_chains_end() {
    let (page, events, notes) = activated(&format!(
        "<< /S /Named /N /NextPage /Next [<< /S /Named /N /NextPage >> {WIPE}] >>"
    ));
    assert_eq!(page, 2, "two NextPage actions, performed in order");
    assert_eq!(transitions(&events), 1);
    assert!(
        !notes.iter().any(|note| note.contains("transition")),
        "nothing to say: {notes:?}"
    );
}

/// A transition a later page action moves away from belonged to the page reached before it;
/// the chain still ends where its last action goes, and the difference is said.
#[test]
fn a_transition_a_later_page_action_moves_away_from_is_said() {
    let (page, events, notes) = activated(&format!(
        "<< /S /Named /N /NextPage /Next [{WIPE} << /S /Named /N /NextPage >>] >>"
    ));
    assert_eq!(page, 2, "the chain ends where its last action goes");
    assert_eq!(transitions(&events), 1, "the transition is still named");
    assert!(
        notes
            .iter()
            .any(|note| note.contains("transition on page 2")),
        "the page the transition belonged to is said: {notes:?}"
    );
}

//! RFC 0008 section 6.5's open sequence, asked for by a host once its first frame is up.
//!
//! `ViewState::run_open_scripts` runs Table 32's `/JavaScript` name tree, the `/OpenAction` script,
//! page one's `/O` and its annotations' `/PO` (ADR 1602); `Command::Presented` is how a window says
//! the moment has come, once a document. With no runner supplied — which is every window today —
//! the sequence runs nothing and says so, once, which is the sentence these tests read.

use std::fmt::Write as _;

use viewer_core::{Command, DocumentId, Event, Viewer};

const DOCUMENT: DocumentId = DocumentId(1);

/// One page, and a catalog whose `/Names` holds one document-level script.
fn scripted() -> Vec<u8> {
    let bodies = [
        "<< /Type /Catalog /Pages 2 0 R /Names << /JavaScript 4 0 R >> >>",
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] >>",
        "<< /Names [(helpers) 5 0 R] >>",
        "<< /S /JavaScript /JS (function twice\\(x\\) { return 2 * x; }) >>",
    ];
    let mut out = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for (index, body) in bodies.iter().enumerate() {
        offsets.push(out.len());
        let _ = write!(out, "{} 0 obj\n{body}\nendobj\n", index.saturating_add(1));
    }
    let xref_at = out.len();
    let size = bodies.len().saturating_add(1);
    let _ = write!(out, "xref\n0 {size}\n0000000000 65535 f \n");
    for offset in &offsets {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    let _ = write!(
        out,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n"
    );
    out.into_bytes()
}

fn sentences(events: &[Event]) -> Vec<String> {
    events
        .iter()
        .filter_map(|event| match event {
            Event::Reported {
                page: None, notes, ..
            } => Some(notes.clone()),
            _ => None,
        })
        .flatten()
        .collect()
}

/// The open says nothing about the scripts, the first present runs the sequence and says that the
/// one document-level script went unrun, and a second present of the same document runs nothing.
#[test]
fn the_open_sequence_runs_at_the_first_present_and_once() {
    let mut viewer = Viewer::new(400, 400, 1.0);
    let opened: Vec<Event> = viewer
        .handle(Command::Open {
            id: DOCUMENT,
            bytes: scripted().into(),
            password: None,
            fragment: None,
        })
        .collect();
    assert!(
        !sentences(&opened)
            .iter()
            .any(|sentence| sentence.contains("document-level script")),
        "the open runs no part of the sequence: {opened:?}"
    );
    let first: Vec<Event> = viewer.handle(Command::Presented).collect();
    let said = sentences(&first);
    let Some(sentence) = said
        .iter()
        .find(|sentence| sentence.contains("document-level script"))
    else {
        panic!("the first present runs the sequence and says what it did not run: {first:?}");
    };
    assert!(
        sentence.contains("carries 1 document-level script"),
        "{sentence}"
    );
    let again: Vec<Event> = viewer.handle(Command::Presented).collect();
    assert!(sentences(&again).is_empty(), "{again:?}");
}

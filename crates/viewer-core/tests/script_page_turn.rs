//! `this.pageNum = n`, carried out by the viewer as a person's page turn: a runner's
//! `ScriptEdit::GoTo` reaches `Event::PageChanged` after the command whose scripts made it, and a
//! page the document does not have turns nothing (ADR 1643).
//!
//! The runner here is the test's own, answering the open action with one edit, so what is judged is
//! the viewer's half — the request taken and the turn made — and never what a script engine does
//! with Adobe's member.

use std::fmt::Write as _;
use std::sync::Arc;

use pdf_model::view::{ScriptEdit, ScriptEvent, ScriptResult, ScriptRunner, ScriptSite};
use viewer_core::{Command, DocumentId, Event, ScriptRunners, Scripting, Viewer};

const DOCUMENT: DocumentId = DocumentId(1);

/// Three pages and an open action whose script the runner below answers.
fn three_pages() -> Vec<u8> {
    let bodies = [
        "<< /Type /Catalog /Pages 2 0 R /OpenAction << /S /JavaScript /JS (turn\\(\\);) >> >>"
            .to_owned(),
        "<< /Type /Pages /Kids [3 0 R 4 0 R 5 0 R] /Count 3 >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 400 400] >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 400 400] >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 400 400] >>".to_owned(),
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

/// A runner that answers the open action by turning to one page.
#[derive(Debug)]
struct Turning {
    page: u32,
}

impl ScriptRunner for Turning {
    fn run(&self, event: &ScriptEvent<'_>) -> ScriptResult {
        let edits = if event.site == ScriptSite::OpenAction {
            vec![ScriptEdit::GoTo { page: self.page }]
        } else {
            Vec::new()
        };
        ScriptResult {
            rc: true,
            value: None,
            change: None,
            edits,
            report: Vec::new(),
        }
    }
}

#[derive(Debug)]
struct Turners(u32);

impl ScriptRunners for Turners {
    fn runner(&self) -> Arc<dyn ScriptRunner> {
        Arc::new(Turning { page: self.0 })
    }
}

/// The pages the events after the first frame turned to, in order.
fn turned_to(page: u32) -> (Vec<usize>, Vec<Event>) {
    let mut viewer = Viewer::new(800, 1000, 1.0);
    viewer
        .handle(Command::Scripts(Scripting::Run(Arc::new(Turners(page)))))
        .for_each(drop);
    viewer
        .handle(Command::Open {
            id: DOCUMENT,
            bytes: three_pages().into(),
            password: None,
            fragment: None,
        })
        .for_each(drop);
    let events: Vec<Event> = viewer.handle(Command::Presented).collect();
    let pages = events
        .iter()
        .filter_map(|event| match event {
            Event::PageChanged {
                document, index, ..
            } if *document == DOCUMENT => Some(*index),
            _ => None,
        })
        .collect();
    (pages, events)
}

/// The open action's `this.pageNum = 2` shows the third page after the command that ran it.
#[test]
fn a_scripts_page_turn_is_made_after_the_command_that_ran_it() {
    let (pages, events) = turned_to(2);
    assert_eq!(pages, vec![2], "{events:?}");
}

/// A page the document does not have turns nothing, and the view state's sentence says so.
#[test]
fn a_page_the_document_does_not_have_turns_nothing_and_is_said() {
    let (pages, events) = turned_to(7);
    assert!(pages.is_empty(), "{events:?}");
    let said = events.iter().any(|event| {
        matches!(event, Event::Reported { notes, .. }
            if notes.iter().any(|note| note.contains("no page is turned")))
    });
    assert!(said, "{events:?}");
}

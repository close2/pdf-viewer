//! `Command::Declined`: what a host's draw left to a coarser measure reaches the reader once per
//! condition while the page's reading stands, and is kept with the page's notes (ADR 2290).

#![expect(
    clippy::arithmetic_side_effects,
    reason = "test code: the offsets are within a fixture written in this file"
)]

use std::fmt::Write as _;

use viewer_core::{Answer, Command, Declined, DocumentId, Event, Query, Viewer};

/// A one-page document with nothing on its page.
fn document() -> Vec<u8> {
    let body = "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
                2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
                3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 40 20] >>\nendobj\n";
    let mut out = String::from("%PDF-2.0\n");
    let mut offsets = Vec::new();
    for object in body.split_inclusive("endobj\n") {
        offsets.push(out.len());
        out.push_str(object);
    }
    let xref_at = out.len();
    let size = offsets.len() + 1;
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

/// What a decline of `condition` at the pixel column `left` would say — the shape
/// `render_cpu::UnionDecline`'s sentence has, whose pixels move with the magnification.
fn declined(condition: &str, left: u32) -> Declined {
    Declined::new(
        condition,
        format!(
            "a mark of 3 pieces, 4×5 pixels from ({left}, 7), was drawn at the library's \
             sixteenths rather than at its exact area because {condition}"
        ),
    )
}

/// The notes each `Event::Reported` of `events` carried about page one.
fn reported(events: impl Iterator<Item = Event>) -> Vec<Vec<String>> {
    events
        .filter_map(|event| match event {
            Event::Reported {
                page: Some(0),
                notes,
                ..
            } => Some(notes),
            _ => None,
        })
        .collect()
}

/// A page is told a condition once, however many draws at however many magnifications decline
/// it — a zoom gesture draws a frame at every step it stops at — and a second condition is told in
/// its turn; both stay with the page's notes for `Query::Reports` to say again, and a document that
/// is not open takes nothing.
#[test]
fn a_page_is_told_each_condition_once_and_keeps_it() {
    let mut viewer = Viewer::new(40, 20, 1.0);
    let id = DocumentId(1);
    let opened: Vec<Event> = viewer
        .handle(Command::Open {
            id,
            bytes: document().into(),
            password: None,
            fragment: None,
        })
        .collect();
    assert!(
        opened
            .iter()
            .any(|event| matches!(event, Event::NeedsRender(_))),
        "opening asks for page one, which is what interprets it"
    );
    let along = "two of its edges cross along a common line";
    let first = reported(viewer.handle(Command::Declined {
        document: id,
        page: 0,
        declined: vec![declined(along, 6)],
    }));
    assert_eq!(first.len(), 1, "the first decline is said: {first:?}");
    assert_eq!(first[0], vec![declined(along, 6).sentence]);

    let again = reported(viewer.handle(Command::Declined {
        document: id,
        page: 0,
        declined: vec![declined(along, 96)],
    }));
    assert!(
        again.is_empty(),
        "the same condition at another magnification is not said again: {again:?}"
    );

    let budget = "it costs more than the union's budget";
    let second = reported(viewer.handle(Command::Declined {
        document: id,
        page: 0,
        declined: vec![declined(along, 7), declined(budget, 8)],
    }));
    assert_eq!(
        second,
        vec![vec![declined(budget, 8).sentence]],
        "only the condition not yet said is said"
    );

    let Answer::Reports(pages) = viewer.query(Query::Reports) else {
        panic!("an interpreted page answers with its reports");
    };
    let notes: Vec<&String> = pages.iter().flat_map(|page| page.notes.iter()).collect();
    assert_eq!(
        notes,
        vec![&declined(along, 6).sentence, &declined(budget, 8).sentence],
        "what was said is kept with the page's notes"
    );

    let elsewhere = reported(viewer.handle(Command::Declined {
        document: DocumentId(9),
        page: 0,
        declined: vec![declined(along, 6)],
    }));
    assert!(elsewhere.is_empty(), "a document not open takes nothing");
}

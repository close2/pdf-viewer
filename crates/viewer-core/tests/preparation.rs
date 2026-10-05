//! What page one does not need is read after it, on a thread the host chooses (ADR 1543).
//!
//! `CLAUDE.md` principle 2: "[a]nything not needed to show page one is deferred until first use".
//! §12.3.3's outline and §7.7.3's page tree, placed, are what the title bar's section needs, and
//! [`Command::Open`] reads neither. These hold the three ways they are read afterwards — handed
//! over by a host that ran [`viewer_core::Preparation`], at first use by a host that did not, and
//! not at all for an answer about a file no longer open.

use std::path::Path;

use viewer_core::{Command, DocumentId, Event, PageTarget, Viewer};

const DOCUMENT: DocumentId = DocumentId(1);

/// pdf.js's `basicapi.pdf`, whose outline names "Paragraph 1.1" at its third page.
fn basicapi() -> Option<Vec<u8>> {
    std::fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../doc/pdf.js/test/pdfs/basicapi.pdf"),
    )
    .ok()
}

fn open(bytes: Vec<u8>) -> (Viewer, Vec<Event>) {
    let mut viewer = Viewer::new(800, 1000, 1.0);
    let events = viewer
        .handle(Command::Open {
            id: DOCUMENT,
            bytes: bytes.into(),
            password: None,
            fragment: None,
        })
        .collect();
    (viewer, events)
}

fn sections(events: &[Event]) -> Vec<(usize, Option<String>)> {
    events
        .iter()
        .filter_map(|event| match event {
            Event::PageChanged { index, section, .. } => Some((*index, section.clone())),
            _ => None,
        })
        .collect()
}

#[test]
fn the_open_reads_no_outline_and_the_preparation_supplies_it() {
    let Some(bytes) = basicapi() else {
        eprintln!("skipped: doc/pdf.js is not checked out");
        return;
    };
    let (mut viewer, opened) = open(bytes);
    assert_eq!(
        sections(&opened),
        vec![(0, None)],
        "the opening announcement names no section it would have to read the outline for"
    );
    let preparation = viewer
        .preparation()
        .expect("the outline is still to read after the open");
    // Run where a host would run it: on a thread of its own.
    let prepared = std::thread::spawn(move || preparation.run())
        .join()
        .expect("the preparation thread");
    let said = sections(&viewer.prepared(prepared).collect::<Vec<_>>());
    assert!(
        said.iter()
            .all(|(index, section)| *index == 0 && section.is_some()),
        "page one is said again only to add a section it has: {said:?}"
    );
    assert!(
        viewer.preparation().is_none(),
        "nothing is left to read once the answer is taken"
    );
    let turned: Vec<Event> = viewer.handle(Command::GoTo(PageTarget::Index(2))).collect();
    assert_eq!(
        sections(&turned),
        vec![(2, Some("Paragraph 1.1".to_owned()))]
    );
}

#[test]
fn a_host_that_never_prepares_reads_it_at_first_use() {
    let Some(bytes) = basicapi() else {
        eprintln!("skipped: doc/pdf.js is not checked out");
        return;
    };
    let (mut viewer, _) = open(bytes);
    let turned: Vec<Event> = viewer.handle(Command::GoTo(PageTarget::Index(2))).collect();
    assert_eq!(
        sections(&turned),
        vec![(2, Some("Paragraph 1.1".to_owned()))],
        "a turn is a first use, and reads what the open did not"
    );
    assert!(viewer.preparation().is_none());
}

#[test]
fn an_answer_about_a_file_no_longer_open_is_dropped() {
    let Some(bytes) = basicapi() else {
        eprintln!("skipped: doc/pdf.js is not checked out");
        return;
    };
    let (mut viewer, _) = open(bytes.clone());
    let preparation = viewer.preparation().expect("still to read");
    viewer.handle(Command::Close(DOCUMENT)).for_each(drop);
    // The same id, another opening of the file: the answer names the file it was read from
    // (trap 104), and that file is gone.
    viewer
        .handle(Command::Open {
            id: DOCUMENT,
            bytes: bytes.into(),
            password: None,
            fragment: None,
        })
        .for_each(drop);
    let said: Vec<Event> = viewer.prepared(preparation.run()).collect();
    assert!(said.is_empty(), "{said:?}");
    assert!(
        viewer.preparation().is_some(),
        "the document open now still has its outline to read"
    );
}

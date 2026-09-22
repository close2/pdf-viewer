//! ISO 32000-2 §12.6.4.3's remote go-to across the one thing this crate cannot do itself.
//!
//! Table 203's `/F` is "[t]he file in which the destination shall be located", so it names a file
//! on somebody's disk; `doc/ui-boundary.md`'s rule 2 keeps paths out of this crate and
//! `CLAUDE.md` principle 3 keeps a filesystem away from the process that parses a PDF. The jump
//! therefore suspends at `/F`, the name crosses as `Event::NeedsFile`, and the destination is
//! read in the document the host sends back — which is §12.6.4.4's shape one clause over, and
//! §12.7.6.4's before it.
//!
//! Two documents, hand-built, differing in what is supplied and in nothing else (trap 8): the
//! source holding the link, and the file its `/F` names. Every assertion excludes a *named* wrong
//! answer — a page index that is the source's, a destination read in the wrong document, a jump
//! made from an action whose own file was never opened — because a test that only wanted an
//! answer would pass on the document staying where it was.

use std::fmt::Write as _;
use viewer_core::{Command, DocumentId, Event, Purpose, Viewer};

/// The document under test.
const DOCUMENT: DocumentId = DocumentId(1);

/// Assembles a document from object bodies given as bytes, with object 1 the catalog.
fn assembled(objects: &[Vec<u8>]) -> Vec<u8> {
    let mut out: Vec<u8> = b"%PDF-2.0\n".to_vec();
    let mut offsets = Vec::new();
    for (index, body) in objects.iter().enumerate() {
        offsets.push(out.len());
        out.extend_from_slice(format!("{} 0 obj\n", index.saturating_add(1)).as_bytes());
        out.extend_from_slice(body);
        out.extend_from_slice(b"\nendobj\n");
    }
    let xref_at = out.len();
    let size = objects.len().saturating_add(1);
    out.extend_from_slice(format!("xref\n0 {size}\n0000000000 65535 f \n").as_bytes());
    for offset in &offsets {
        out.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    out.extend_from_slice(
        format!("trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n")
            .as_bytes(),
    );
    out
}

/// A document of `pages` pages and nothing else.
fn plain(pages: usize) -> Vec<u8> {
    let kids = (0..pages).fold(String::new(), |mut kids, index| {
        let _ = write!(kids, "{} 0 R ", index.saturating_add(3));
        kids
    });
    let mut objects = vec![
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        format!("<< /Type /Pages /Count {pages} /Kids [{kids}] >>").into_bytes(),
    ];
    objects.extend(
        (0..pages).map(|_| b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] >>".to_vec()),
    );
    assembled(&objects)
}

/// The source: two pages, one link on the first, whose action names another file and a page in it.
///
/// `[2 /Fit]` is §12.3.2.2's remote form — "the page parameter specifies an integer page number
/// within the remote document instead of a page object in the current document" — and the number
/// is the third page, which this document does not have. That is deliberate: a reader that read
/// the destination here rather than there would find no page 2 at all.
fn source(action: &str) -> Vec<u8> {
    assembled(&[
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Count 2 /Kids [3 0 R 4 0 R] >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Annots [5 0 R] >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] >>".to_vec(),
        format!("<< /Type /Annot /Subtype /Link /Rect [0 0 50 50] /A {action} >>").into_bytes(),
    ])
}

/// The action under test in every case but one.
const REMOTE: &str = "<< /Type /Action /S /GoToR /D [2 /Fit] /F (chapter2.pdf) /NewWindow false >>";

/// Opens the source and clicks its link, returning the viewer and what the click said.
fn asked(action: &str) -> (Viewer, Vec<Event>) {
    let mut viewer = Viewer::new(400, 400, 1.0);
    viewer
        .handle(Command::Open {
            id: DOCUMENT,
            bytes: source(action).into(),
            password: None,
            fragment: None,
        })
        .for_each(drop);
    let events = viewer
        .handle(Command::Activate(pdf_syntax::ObjectId::new(5, 0)))
        .collect();
    (viewer, events)
}

/// Every sentence an event list carries, in order.
fn notes(events: &[Event]) -> Vec<String> {
    events
        .iter()
        .flat_map(|event| match event {
            Event::Reported { notes, .. } => notes.clone(),
            _ => Vec::new(),
        })
        .collect()
}

/// Table 203's `/F` asks the host for a file, by the name the *document* wrote.
#[test]
fn the_file_an_action_names_is_asked_of_the_host() {
    let (_, events) = asked(REMOTE);
    let asked_for: Vec<(Purpose, String)> = events
        .iter()
        .filter_map(|event| match event {
            Event::NeedsFile { purpose, name, .. } => Some((*purpose, name.clone())),
            _ => None,
        })
        .collect();
    assert_eq!(
        asked_for,
        [(Purpose::RemoteDocument, "chapter2.pdf".to_owned())],
        "the name is the document's own words and the purpose says which clause wants it"
    );
    assert!(
        notes(&events).is_empty(),
        "nothing is refused yet: the question has been asked, not answered"
    );
}

/// The supplied file is opened, and the destination is read in **it**.
#[test]
fn a_supplied_file_is_opened_at_the_page_the_action_names() {
    let (mut viewer, _) = asked(REMOTE);
    let events: Vec<Event> = viewer
        .handle(Command::Supply {
            purpose: Purpose::RemoteDocument,
            bytes: Some(plain(4)),
        })
        .collect();
    let opened: Vec<usize> = events
        .iter()
        .filter_map(|event| match event {
            Event::Opened { pages, .. } => Some(*pages),
            _ => None,
        })
        .collect();
    assert_eq!(
        opened,
        [4],
        "the document on the screen is the file the action named, not the one holding it"
    );
    let turned: Vec<usize> = events
        .iter()
        .filter_map(|event| match event {
            Event::PageChanged { index, .. } => Some(*index),
            _ => None,
        })
        .collect();
    assert_eq!(
        turned.last(),
        Some(&2),
        "§12.3.2.2: the first element is a page number in the remote document and \"[t]he first \
         page shall be numbered 0\", so [2 /Fit] is its third page"
    );
}

/// A host that will not supply the file is answered by name rather than by silence.
#[test]
fn a_file_the_host_will_not_supply_is_declined_by_name() {
    let (mut viewer, _) = asked(REMOTE);
    let events: Vec<Event> = viewer
        .handle(Command::Supply {
            purpose: Purpose::RemoteDocument,
            bytes: None,
        })
        .collect();
    assert_eq!(
        notes(&events),
        ["this link declines — GoToR: chapter2.pdf was not supplied".to_owned()],
        "trap 5: a click that silently did nothing would look like a click on nothing"
    );
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, Event::Opened { .. })),
        "and nothing was opened"
    );
}

/// Bytes that are not a PDF name themselves rather than opening as an empty document.
#[test]
fn a_supplied_file_that_is_not_a_pdf_names_itself() {
    let (mut viewer, _) = asked(REMOTE);
    let events: Vec<Event> = viewer
        .handle(Command::Supply {
            purpose: Purpose::RemoteDocument,
            bytes: Some(b"not a PDF at all".to_vec()),
        })
        .collect();
    let said = notes(&events);
    assert_eq!(said.len(), 1, "one sentence, not none and not two");
    assert!(
        said[0].starts_with("this link declines — GoToR: cannot read chapter2.pdf: "),
        "the file is named and so is what went wrong: {}",
        said[0]
    );
}

/// Table 203's `/SD` takes precedence over `/D`, and it is resolved in the remote document.
///
/// The `/D` names page 1 and the `/SD` names a structure element whose only content item is on
/// page 3, so a reader that ignored `/SD` — or that looked for the element in the document
/// holding the action — lands somewhere this assertion names.
#[test]
fn a_structure_destination_is_resolved_in_the_remote_document() {
    let tagged = assembled(&[
        b"<< /Type /Catalog /Pages 2 0 R /StructTreeRoot 6 0 R >>".to_vec(),
        b"<< /Type /Pages /Count 3 /Kids [3 0 R 4 0 R 5 0 R] >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] >>".to_vec(),
        b"<< /Type /StructTreeRoot /K [7 0 R] /IDTree 9 0 R >>".to_vec(),
        b"<< /Type /StructElem /S /Sect /ID (Chap2) /K [8 0 R] >>".to_vec(),
        b"<< /Type /MCR /Pg 5 0 R /MCID 0 >>".to_vec(),
        b"<< /Names [(Chap2) 7 0 R] >>".to_vec(),
    ]);
    let (mut viewer, _) =
        asked("<< /Type /Action /S /GoToR /D [0 /Fit] /SD [(Chap2) /Fit] /F (chapter2.pdf) >>");
    let events: Vec<Event> = viewer
        .handle(Command::Supply {
            purpose: Purpose::RemoteDocument,
            bytes: Some(tagged),
        })
        .collect();
    let turned: Vec<usize> = events
        .iter()
        .filter_map(|event| match event {
            Event::PageChanged { index, .. } => Some(*index),
            _ => None,
        })
        .collect();
    assert_eq!(
        turned.last(),
        Some(&2),
        "Table 203: \"[i]f present, the structure destination should take precedence over \
         destination in the D entry\", and the element's content item is on the third page"
    );
}

/// §7.11.2.2's forbidden relative URL asks for no file at all.
///
/// Asking would mean having resolved it, which is the hazard that sentence is about: a relative
/// reference carrying an authority resolves against a different host from the document's.
#[test]
fn a_relative_url_the_clause_forbids_asks_for_nothing() {
    let (_, events) = asked(
        "<< /Type /Action /S /GoToR /D [0 /Fit] /F << /Type /Filespec /FS /URL \
         /F (//elsewhere/b.pdf) >> >>",
    );
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, Event::NeedsFile { .. })),
        "nothing is asked for on its account"
    );
    assert_eq!(
        notes(&events).len(),
        1,
        "and it is said rather than dropped (trap 5)"
    );
}

/// Table 203's `/NewWindow true`, with a host that has a second place to put a document.
///
/// The entry says the destination is opened "in a new window" and defers the absent case to the
/// processor's preference; what this program's preference *is* depends on the window, which is
/// why the name a second document would take arrives from out here. Both documents stay open,
/// the one the link named is focused, and the source is still the source — a reader that
/// replaced it would fail the page assertion below, which names the source's own page count.
#[test]
fn a_new_window_opens_the_destination_beside_the_document_it_was_reached_from() {
    const BESIDE: DocumentId = DocumentId(9);
    let (mut viewer, _) =
        asked("<< /Type /Action /S /GoToR /D [2 /Fit] /F (chapter2.pdf) /NewWindow true >>");
    viewer.handle(Command::Beside(Some(BESIDE))).for_each(drop);
    let events: Vec<Event> = viewer
        .handle(Command::Supply {
            purpose: Purpose::RemoteDocument,
            bytes: Some(plain(4)),
        })
        .collect();
    let opened: Vec<(DocumentId, usize)> = events
        .iter()
        .filter_map(|event| match event {
            Event::Opened { document, pages } => Some((*document, *pages)),
            _ => None,
        })
        .collect();
    assert_eq!(
        opened,
        [(BESIDE, 4)],
        "the remote document opened under the name the host offered, not under the source's"
    );
    assert!(
        notes(&events)
            .iter()
            .any(|note| note.contains("opens beside the document it was reached from")),
        "and the difference is said out loud (trap 5): {:?}",
        notes(&events)
    );
    // The source is still open, and still the source: going back to it finds the two pages it
    // has rather than the four the remote document has.
    let back: Vec<Event> = viewer.handle(Command::Focus(DOCUMENT)).collect();
    let of: Vec<usize> = back
        .iter()
        .filter_map(|event| match event {
            Event::PageChanged { of, .. } => Some(*of),
            _ => None,
        })
        .collect();
    assert_eq!(
        of.last(),
        Some(&2),
        "the document holding the link was not replaced by the one it named"
    );
}

/// The same action with no name in reserve replaces, and says so.
///
/// The `true` case is not a `shall` — Table 203 states none — so a window with one view is still
/// conforming; what it may not do is pass the request over in silence.
#[test]
fn a_new_window_with_nowhere_to_put_it_replaces_and_says_so() {
    let (mut viewer, _) =
        asked("<< /Type /Action /S /GoToR /D [2 /Fit] /F (chapter2.pdf) /NewWindow true >>");
    let events: Vec<Event> = viewer
        .handle(Command::Supply {
            purpose: Purpose::RemoteDocument,
            bytes: Some(plain(4)),
        })
        .collect();
    let opened: Vec<DocumentId> = events
        .iter()
        .filter_map(|event| match event {
            Event::Opened { document, .. } => Some(*document),
            _ => None,
        })
        .collect();
    assert_eq!(
        opened,
        [DOCUMENT],
        "it replaced the document it was reached from"
    );
    assert!(
        notes(&events)
            .iter()
            .any(|note| note.contains("this view has one, so the destination replaces")),
        "{:?}",
        notes(&events)
    );
}

/// A name is used **once**: the second `/NewWindow true` replaces unless a second name is offered.
///
/// The reserve is what stops a second remote go-to opening under the first one's identity, which
/// `Command::Open`'s own rule would make a replacement of a tab somebody is reading.
#[test]
fn a_reserved_name_is_spent_by_the_document_that_opens_under_it() {
    const BESIDE: DocumentId = DocumentId(9);
    let (mut viewer, _) =
        asked("<< /Type /Action /S /GoToR /D [2 /Fit] /F (chapter2.pdf) /NewWindow true >>");
    viewer.handle(Command::Beside(Some(BESIDE))).for_each(drop);
    viewer
        .handle(Command::Supply {
            purpose: Purpose::RemoteDocument,
            bytes: Some(plain(4)),
        })
        .for_each(drop);
    // Back to the source, and the same link again with nothing offered this time.
    viewer.handle(Command::Focus(DOCUMENT)).for_each(drop);
    viewer
        .handle(Command::Activate(pdf_syntax::ObjectId::new(5, 0)))
        .for_each(drop);
    let events: Vec<Event> = viewer
        .handle(Command::Supply {
            purpose: Purpose::RemoteDocument,
            bytes: Some(plain(4)),
        })
        .collect();
    let opened: Vec<DocumentId> = events
        .iter()
        .filter_map(|event| match event {
            Event::Opened { document, .. } => Some(*document),
            _ => None,
        })
        .collect();
    assert_eq!(
        opened,
        [DOCUMENT],
        "the spent name was not reused, so the destination replaced the source"
    );
}

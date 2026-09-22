//! ISO 32000-2 §12.6.4.4's embedded go-to across the one thing this crate cannot do itself.
//!
//! Table 204's `/F` is "[t]he root document of the target relative to the root document of the
//! source", so it names a file on somebody's disk; `doc/ui-boundary.md`'s rule 2 keeps paths out
//! of this crate and `CLAUDE.md` principle 3 keeps a filesystem away from the process that parses
//! a PDF. The walk therefore suspends at `/F`, the name crosses as `Event::NeedsFile`, and the
//! remaining `/T` steps run against the root the host sent back — which is the shape §12.7.6.4's
//! import-data already has, and ADR 1101's for a file matched rather than asked for.
//!
//! Three documents, hand-built, differing in what is supplied and in nothing else (trap 8): the
//! source holding the link, the root document its `/F` names, and the child embedded in that root
//! which the `/T` step reaches. Every assertion excludes a *named* wrong answer — a page count
//! that is the root's rather than the child's, an event that is the wrong kind — because a test
//! that only wanted an answer would pass on the document staying where it was.

use std::fmt::Write as _;
use viewer_core::{Command, DocumentId, Event, Purpose, Viewer};

/// The document under test.
const DOCUMENT: DocumentId = DocumentId(1);

/// Assembles a document from object bodies given as bytes, with object 1 the catalog.
fn assembled(objects: &[Vec<u8>]) -> Vec<u8> {
    let mut out: Vec<u8> = b"%PDF-1.7\n".to_vec();
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

/// A one-page document embedding `inner` under `key` in §7.11.4's `EmbeddedFiles` name tree.
fn embedding(key: &str, inner: &[u8]) -> Vec<u8> {
    let mut stream = format!(
        "<< /Type /EmbeddedFile /Subtype /application#2Fpdf /Length {} >>\nstream\n",
        inner.len()
    )
    .into_bytes();
    stream.extend_from_slice(inner);
    stream.extend_from_slice(b"\nendstream");
    assembled(&[
        format!(
            "<< /Type /Catalog /Pages 2 0 R /Names << /EmbeddedFiles << /Names [({key}) 5 0 R] \
             >> >> >>"
        )
        .into_bytes(),
        b"<< /Type /Pages /Count 1 /Kids [3 0 R] >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] >>".to_vec(),
        stream,
        format!("<< /Type /Filespec /F ({key}) /UF ({key}) /EF << /F 4 0 R >> >>").into_bytes(),
    ])
}

/// The source: one page, one link, whose action names a root document and one `/T` step.
fn source() -> Vec<u8> {
    assembled(&[
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Count 1 /Kids [3 0 R] >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Annots [4 0 R] >>".to_vec(),
        b"<< /Type /Annot /Subtype /Link /Rect [0 0 50 50] /A << /Type /Action /S /GoToE \
          /D [2 /Fit] /F (target.pdf) /T << /R /C /N (child.pdf) >> >> >>"
            .to_vec(),
    ])
}

/// Opens the source and clicks its link, returning the viewer and what the click said.
fn asked() -> (Viewer, Vec<Event>) {
    let mut viewer = Viewer::new(400, 400, 1.0);
    viewer
        .handle(Command::Open {
            id: DOCUMENT,
            bytes: source().into(),
            password: None,
            fragment: None,
        })
        .for_each(drop);
    let events = viewer
        .handle(Command::Activate(pdf_syntax::ObjectId::new(4, 0)))
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

/// Table 204's `/F` asks the host for a file, by the name the *document* wrote.
#[test]
fn a_root_document_the_action_names_is_asked_of_the_host() {
    let (_, events) = asked();
    let asked: Vec<(Purpose, String)> = events
        .iter()
        .filter_map(|event| match event {
            Event::NeedsFile { purpose, name, .. } => Some((*purpose, name.clone())),
            _ => None,
        })
        .collect();
    assert_eq!(
        asked,
        [(Purpose::TargetRoot, "target.pdf".to_owned())],
        "one file, for §12.6.4.4 rather than for §12.7.6.4's import"
    );
    assert!(
        notes(&events).is_empty(),
        "nothing is refused yet: the question has been asked, not answered"
    );
}

/// Supplied, the walk resumes: the `/T` step's child replaces the document, at the `/D`'s page.
///
/// The child has three pages and the root that carries it has one, so a resumption that stopped
/// at the root — or never started — is a different number here rather than the same one.
#[test]
fn a_supplied_root_resumes_the_walk_and_shows_the_destination() {
    let (mut viewer, _) = asked();
    let root = embedding("child.pdf", &plain(3));
    let events: Vec<Event> = viewer
        .handle(Command::Supply {
            purpose: Purpose::TargetRoot,
            bytes: Some(root),
        })
        .collect();

    let opened: Vec<usize> = events
        .iter()
        .filter_map(|event| match event {
            Event::Opened { pages, .. } => Some(*pages),
            _ => None,
        })
        .collect();
    assert_eq!(opened, [3], "the child of the supplied root, not the root");

    // §12.3.2.2's explicit form, read in the target: "[t]he first page shall be numbered 0", so
    // `[2 /Fit]` is the third page and the host is told the third page by its own numbering.
    let pages: Vec<usize> = events
        .iter()
        .filter_map(|event| match event {
            Event::PageChanged { index, .. } => Some(*index),
            _ => None,
        })
        .collect();
    assert_eq!(
        pages.last(),
        Some(&2),
        "the destination's page in the target"
    );
}

/// Withheld, the click declines **by name**, and the document on the screen does not move.
#[test]
fn a_root_the_host_will_not_supply_is_declined_by_name() {
    let (mut viewer, _) = asked();
    let events: Vec<Event> = viewer
        .handle(Command::Supply {
            purpose: Purpose::TargetRoot,
            bytes: None,
        })
        .collect();
    assert_eq!(
        notes(&events),
        ["this link declines — GoToE: target.pdf was not supplied".to_owned()],
        "the file the document named, so a person can tell which one was missing"
    );
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, Event::Opened { .. })),
        "nothing replaced the document"
    );
}

/// Bytes that are not a PDF are a fact about the host's answer, and are said as one.
#[test]
fn a_supplied_file_that_is_not_a_pdf_names_itself() {
    let (mut viewer, _) = asked();
    let events: Vec<Event> = viewer
        .handle(Command::Supply {
            purpose: Purpose::TargetRoot,
            bytes: Some(b"not a PDF at all".to_vec()),
        })
        .collect();
    let said = notes(&events);
    assert_eq!(said.len(), 1, "one sentence, got {said:?}");
    assert!(
        said[0].starts_with("this link declines — GoToE: cannot read target.pdf: "),
        "the name and the reader's own reason, got {said:?}"
    );
}

/// Table 204's `/NewWindow true`, with a host that has a second place to put a document.
///
/// The entry states this case with a `should` where Table 203 states it with nothing, so an
/// embedded go-to is the *stronger* of the two — and the same reserve answers both, because
/// whether this program has a second view is a fact about the window and not about the clause.
/// No file is asked for at all here: the target is inside the document already open.
#[test]
fn an_embedded_document_opens_beside_the_one_holding_it() {
    const BESIDE: DocumentId = DocumentId(9);
    let inner = plain(3);
    let mut stream = format!(
        "<< /Type /EmbeddedFile /Subtype /application#2Fpdf /Length {} >>\nstream\n",
        inner.len()
    )
    .into_bytes();
    stream.extend_from_slice(&inner);
    stream.extend_from_slice(b"\nendstream");
    let holding = assembled(&[
        b"<< /Type /Catalog /Pages 2 0 R /Names << /EmbeddedFiles << /Names [(child.pdf) 6 0 R] \
          >> >> >>"
            .to_vec(),
        b"<< /Type /Pages /Count 1 /Kids [3 0 R] >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Annots [4 0 R] >>".to_vec(),
        b"<< /Type /Annot /Subtype /Link /Rect [0 0 50 50] /A << /Type /Action /S /GoToE \
          /D [2 /Fit] /NewWindow true /T << /R /C /N (child.pdf) >> >> >>"
            .to_vec(),
        stream,
        b"<< /Type /Filespec /F (child.pdf) /UF (child.pdf) /EF << /F 5 0 R >> >>".to_vec(),
    ]);
    let mut viewer = Viewer::new(400, 400, 1.0);
    viewer
        .handle(Command::Open {
            id: DOCUMENT,
            bytes: holding.into(),
            password: None,
            fragment: None,
        })
        .for_each(drop);
    viewer.handle(Command::Beside(Some(BESIDE))).for_each(drop);
    let events: Vec<Event> = viewer
        .handle(Command::Activate(pdf_syntax::ObjectId::new(4, 0)))
        .collect();
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, Event::NeedsFile { .. })),
        "the target is inside the document already open, so no host is asked for anything"
    );
    let opened: Vec<(DocumentId, usize)> = events
        .iter()
        .filter_map(|event| match event {
            Event::Opened { document, pages } => Some((*document, *pages)),
            _ => None,
        })
        .collect();
    assert_eq!(
        opened,
        [(BESIDE, 3)],
        "the embedded document opened under the name the host offered: {:?}",
        notes(&events)
    );
    assert!(
        notes(&events)
            .iter()
            .any(|note| note.contains("opens beside the document it was reached from")),
        "{:?}",
        notes(&events)
    );
}

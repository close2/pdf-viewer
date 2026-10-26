//! §13.7.2.3.2's assets of a rich media annotation, listed, saved and never played.
//!
//! > The Assets entry takes a name tree of embedded file specification dictionaries .
//!
//! The owner's A631 puts them on the attachments' side of `CLAUDE.md`'s clause 13 entry: each is an
//! embedded file a person may save under the existing level for embedded files, and nothing hands
//! one to a player (ADR 2399). Each test names the wrong answer it excludes: an asset listed on a
//! page the reader is not on (the whole-tree walk ADR 0295 forbids at open), an asset the panel
//! lists that `Command::Extract` cannot find, a click that hands an asset to a player instead of
//! out as a file, and a name departing from the clause's rules handed over without a word.
//!
//! No corpus document on this disk carries a rich media annotation, so the fixture is written
//! here from the clause's own EXAMPLE (trap 8).

#![expect(
    clippy::panic,
    reason = "test code: a fixture that cannot exercise the rule must fail loudly rather than \
              pass by doing nothing"
)]

use std::fmt::Write as _;

use viewer_core::{Answer, Command, DocumentId, Event, PageTarget, PointerAction, Query, Viewer};

/// The document under test.
const DOCUMENT: DocumentId = DocumentId(1);

/// Two 100-unit pages, the second holding a rich media annotation over its lower-left quarter
/// whose assets are `scene.swf` (the EXAMPLE's form, `/F` and `/UF` the key) and `a|b.flv`, whose
/// name breaks one of the clause's restrictions.
fn document() -> Vec<u8> {
    let objects = [
        "<< /Type /Catalog /Pages 2 0 R >>",
        "<< /Type /Pages /Kids [3 0 R 4 0 R] /Count 2 >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Annots [5 0 R] >>",
        "<< /Type /Annot /Subtype /RichMedia /Rect [0 0 50 50] /RichMediaContent << /Assets << \
         /Names [(a|b.flv) 8 0 R (scene.swf) 6 0 R] >> >> >>",
        "<< /Type /Filespec /F (scene.swf) /UF (scene.swf) /EF << /F 7 0 R >> >>",
        "<< /Type /EmbeddedFile /Subtype /application#2Fx-shockwave-flash /Length 3 >>\nstream\n\
         FWS\nendstream",
        "<< /Type /Filespec /F (a|b.flv) /UF (a|b.flv) /EF << /F 9 0 R >> >>",
        "<< /Type /EmbeddedFile /Length 3 >>\nstream\nFLV\nendstream",
    ];
    let mut out = String::from("%PDF-2.0\n");
    let mut offsets = Vec::new();
    for (index, body) in objects.iter().enumerate() {
        offsets.push(out.len());
        let _ = write!(out, "{} 0 obj\n{body}\nendobj\n", index.saturating_add(1));
    }
    let xref_at = out.len();
    let size = objects.len().saturating_add(1);
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

/// A viewer with the document open on its first page.
fn opened() -> Viewer {
    let mut viewer = Viewer::new(400, 400, 1.0);
    viewer
        .handle(Command::Open {
            id: DOCUMENT,
            bytes: document().into(),
            password: None,
            fragment: None,
        })
        .for_each(drop);
    viewer
}

/// The names `Query::Attachments` answers with.
fn listed(viewer: &Viewer) -> Vec<String> {
    match viewer.query(Query::Attachments) {
        Answer::Attachments(files) => files.into_iter().map(|file| file.name).collect(),
        other => panic!("the query answers attachments: {other:?}"),
    }
}

/// Every file the events hand out, by name and bytes.
fn extracted(events: &[Event]) -> Vec<(String, Vec<u8>)> {
    events
        .iter()
        .filter_map(|event| match event {
            Event::Extracted { name, bytes, .. } => Some((name.clone(), bytes.clone())),
            _ => None,
        })
        .collect()
}

/// The assets join the list while the reader is on their page, in the tree's order, and come out
/// by the name they are listed under; on another page they are not listed.
#[test]
fn a_pages_rich_media_assets_are_listed_and_extracted_by_name() {
    let mut viewer = opened();
    assert!(listed(&viewer).is_empty(), "page one carries no asset");
    let _ = viewer.handle(Command::GoTo(PageTarget::Index(1))).count();
    assert_eq!(listed(&viewer), ["a|b.flv", "scene.swf"]);
    let events: Vec<Event> = viewer
        .handle(Command::Extract {
            name: "scene.swf".to_owned(),
        })
        .collect();
    assert_eq!(
        extracted(&events),
        [("scene.swf".to_owned(), b"FWS".to_vec())],
        "{events:?}"
    );
    let _ = viewer.handle(Command::GoTo(PageTarget::Index(0))).count();
    assert!(listed(&viewer).is_empty(), "the list follows the reader");
}

/// A click on the annotation hands both assets out as embedded files — never `Event::Media`,
/// because nothing plays them — and says the name that breaks §13.7.2.3.2's rules.
#[test]
fn a_click_on_a_rich_media_annotation_hands_its_assets_out_as_files() {
    let mut viewer = opened();
    let _ = viewer.handle(Command::GoTo(PageTarget::Index(1))).count();
    let Answer::Geometry(geometry) = viewer.query(Query::PageGeometry(1)) else {
        panic!("the page on the screen has a geometry");
    };
    // The middle of the annotation's `/Rect [0 0 50 50]`, flipped about the page's height into the
    // viewport (trap 12a).
    let at = (
        geometry.origin.0 + 25.0 * geometry.scale,
        geometry.origin.1 + (100.0 - 25.0) * geometry.scale,
    );
    let mut events: Vec<Event> = Vec::new();
    for action in [PointerAction::Pressed, PointerAction::Released] {
        events.extend(viewer.handle(Command::Pointer { at, action }));
    }
    assert_eq!(
        extracted(&events),
        [
            ("a|b.flv".to_owned(), b"FLV".to_vec()),
            ("scene.swf".to_owned(), b"FWS".to_vec())
        ],
        "{events:?}"
    );
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, Event::Media { .. })),
        "an asset is saved, never handed to a player: {events:?}"
    );
    let said: Vec<&String> = events
        .iter()
        .flat_map(|event| match event {
            Event::Reported { notes, .. } => notes.iter().collect(),
            _ => Vec::new(),
        })
        .collect();
    assert!(
        said.iter()
            .any(|note| note.contains("\"a|b.flv\" breaks §13.7.2.3.2's naming rules")),
        "{said:?}"
    );
}

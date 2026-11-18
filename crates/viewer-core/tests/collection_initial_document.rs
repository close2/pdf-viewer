//! ISO 32000-2 §12.3.5.1: a collection's initial document is *presented*, not marked (ADR 3040).
//!
//! Table 153's `/D` is "[a] string that identifies an entry in the `EmbeddedFiles` name tree,
//! determining the document that shall be initially presented in the user interface", and the
//! clause's prose says that document "may be the container PDF or one of the embedded documents
//! as specified by the D key". Each of the four outcomes is driven through `viewer_core` here, and
//! each asserts which document's pages are on the screen: the container has one page and the
//! embedded PDF three, so the last `Event::Opened` says which is shown.

use std::fmt::Write as _;

use viewer_core::{Answer, Command, DocumentId, Event, Query, Viewer};

const DOCUMENT: DocumentId = DocumentId(1);

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

/// A document of `pages` empty pages.
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

fn stream(body: &[u8]) -> Vec<u8> {
    let mut out =
        format!("<< /Type /EmbeddedFile /Length {} >>\nstream\n", body.len()).into_bytes();
    out.extend_from_slice(body);
    out.extend_from_slice(b"\nendstream");
    out
}

/// A one-page collection holding `a.pdf` (three pages) and `notes.txt`, with `initial` as the
/// collection dictionary's `/D` entry; `files` false leaves the `/EmbeddedFiles` tree empty.
fn collection(initial: &str, files: bool) -> Vec<u8> {
    let names = if files {
        "(a.pdf) 5 0 R (notes.txt) 7 0 R"
    } else {
        ""
    };
    assembled(&[
        format!(
            "<< /Type /Catalog /Pages 2 0 R /Names << /EmbeddedFiles << /Names [{names}] >> >> \
             /Collection << /Type /Collection {initial} >> >>"
        )
        .into_bytes(),
        b"<< /Type /Pages /Count 1 /Kids [3 0 R] >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] >>".to_vec(),
        stream(&plain(3)),
        b"<< /Type /Filespec /F (a.pdf) /UF (a.pdf) /EF << /F 4 0 R >> >>".to_vec(),
        stream(b"not a PDF"),
        b"<< /Type /Filespec /F (notes.txt) /UF (notes.txt) /EF << /F 6 0 R >> >>".to_vec(),
    ])
}

/// The viewer after the open, the page count of the document on the screen (`None` where nothing
/// opened), and what was said.
fn opened(bytes: Vec<u8>) -> (Viewer, Option<usize>, Vec<String>) {
    let mut viewer = Viewer::new(400, 400, 1.0);
    let events: Vec<Event> = viewer
        .handle(Command::Open {
            id: DOCUMENT,
            bytes: bytes.into(),
            password: None,
            fragment: None,
        })
        .collect();
    let shown = events.iter().rev().find_map(|event| match event {
        Event::Opened { pages, .. } => Some(*pages),
        _ => None,
    });
    let notes = events
        .iter()
        .flat_map(|event| match event {
            Event::Reported { notes, .. } => notes.clone(),
            _ => Vec::new(),
        })
        .collect();
    (viewer, shown, notes)
}

/// "If the D entry is missing or is not a valid byte string, the initial document shall be the
/// one that contains the collection dictionary."
#[test]
fn no_d_presents_the_container() {
    let (_, shown, _) = opened(collection("", true));
    assert_eq!(shown, Some(1), "the container's own page");
}

/// A `/D` naming an entry the tree holds presents that entry: its pages are on the screen, and
/// the panel goes on answering with the container's collection and files around it.
#[test]
fn a_d_naming_an_embedded_pdf_presents_it() {
    let (viewer, shown, notes) = opened(collection("/D (a.pdf)", true));
    assert_eq!(shown, Some(3), "the embedded document's pages: {notes:?}");
    let Answer::Collection { initial, .. } = viewer.query(Query::Collection) else {
        panic!("the panel still shows the collection");
    };
    assert_eq!(
        initial,
        pdf_model::collection::Initial::Embedded("a.pdf".to_owned())
    );
    let Answer::Attachments(files) = viewer.query(Query::Attachments) else {
        panic!("the panel still lists the collection's files");
    };
    let names: Vec<&str> = files.iter().map(|file| file.name.as_str()).collect();
    assert_eq!(names, ["a.pdf", "notes.txt"]);
}

/// "If the D entry is a valid byte string that does not match any file in the `EmbeddedFiles` name
/// tree, the interactive PDF processor shall select the first item from the list of files to
/// display in its user interface" — `a.pdf`, first in the tree's order, and a PDF.
#[test]
fn a_d_naming_nothing_presents_the_first_item() {
    let (_, shown, notes) = opened(collection("/D (missing.pdf)", true));
    assert_eq!(shown, Some(3), "the first item's pages: {notes:?}");
}

/// "If no files exist in the name tree, the interactive PDF processor shall display an empty
/// preview window": nothing is presented, and the container stays.
#[test]
fn an_empty_tree_presents_nothing() {
    let (viewer, shown, _) = opened(collection("/D (missing.pdf)", false));
    assert_eq!(shown, Some(1));
    let Answer::Collection { initial, .. } = viewer.query(Query::Collection) else {
        panic!("a document with a /Collection answers with one");
    };
    assert_eq!(initial, pdf_model::collection::Initial::Empty);
}

/// A named document this program cannot open leaves the container's pages, and says why.
#[test]
fn a_d_naming_a_file_that_is_not_a_pdf_keeps_the_container_and_says_so() {
    let (_, shown, notes) = opened(collection("/D (notes.txt)", true));
    assert_eq!(shown, Some(1), "the container stays");
    assert!(
        notes
            .iter()
            .any(|note| note.contains("notes.txt") && note.contains("cannot be presented")),
        "the window says why: {notes:?}"
    );
}

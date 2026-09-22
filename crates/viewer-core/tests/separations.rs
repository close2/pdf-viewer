//! ISO 32000-2 §10.8.3's separation simulation, as a preference that reaches the interpreter.
//!
//! The clause states its own condition, and it is not one any file can meet:
//!
//! > If it is important for the colours of the display for a PDF, on a device that normally would
//! > not be used to produce separations, to more closely match those produced when using
//! > separations, then a simulation of the separation process can be performed for the output to
//! > the non-separation device.
//!
//! §10.8.1 says whose choice that is — "[w]hether separations are produced is up to the
//! processing software" — so the answer comes from a host and arrives through
//! `pdf_model::view::ViewState`, which `CLAUDE.md` rule 1 makes the only channel by which
//! anything outside a file may decide a mark.
//!
//! What this file pins is the channel rather than the algorithm: the answer is off until somebody
//! says otherwise, it reaches a document already open and one opened afterwards, and a change of
//! answer supersedes the ink already produced. Each assertion excludes a named wrong answer — a
//! preference that applied to one document and not the next, a redraw for an answer that did not
//! move — because a value that arrived and changed nothing would pass a weaker test.

use viewer_core::{Command, DocumentId, Event, Viewer};

/// A one-page document with a mark on it.
fn page() -> Vec<u8> {
    let content = b"0 0 1 rg 10 10 80 80 re f\n";
    let mut out: Vec<u8> = b"%PDF-2.0\n".to_vec();
    let bodies: Vec<Vec<u8>> = vec![
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Count 1 /Kids [3 0 R] >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Contents 4 0 R >>".to_vec(),
        {
            let mut stream = format!("<< /Length {} >>\nstream\n", content.len()).into_bytes();
            stream.extend_from_slice(content);
            stream.extend_from_slice(b"\nendstream");
            stream
        },
    ];
    let mut offsets = Vec::new();
    for (index, body) in bodies.iter().enumerate() {
        offsets.push(out.len());
        out.extend_from_slice(format!("{} 0 obj\n", index.saturating_add(1)).as_bytes());
        out.extend_from_slice(body);
        out.extend_from_slice(b"\nendobj\n");
    }
    let xref_at = out.len();
    let size = bodies.len().saturating_add(1);
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

/// Opens one document and drains what opening it produced.
fn opened(id: DocumentId, viewer: &mut Viewer) {
    viewer
        .handle(Command::Open {
            id,
            bytes: page().into(),
            password: None,
            fragment: None,
        })
        .for_each(drop);
}

/// Whether the events include a request to draw a page again.
fn redrawn(events: &[Event]) -> bool {
    events
        .iter()
        .any(|event| matches!(event, Event::NeedsRender(_) | Event::Damage(_)))
}

/// The answer is off until a host says otherwise, and a change of answer supersedes the ink.
#[test]
fn the_preference_is_off_until_a_host_says_otherwise_and_a_change_redraws() {
    let mut viewer = Viewer::new(400, 400, 1.0);
    opened(DocumentId(1), &mut viewer);

    // §10.8.2 is what a screen does when nothing has asked for the simulation, so sending the
    // default answer is telling the viewer what it already believed: nothing is superseded.
    let unchanged: Vec<Event> = viewer.handle(Command::Separations(false)).collect();
    assert!(
        !redrawn(&unchanged),
        "an answer that did not move is not a reason to draw the page again"
    );

    let changed: Vec<Event> = viewer.handle(Command::Separations(true)).collect();
    assert!(
        redrawn(&changed),
        "§10.8.3 decides what colour every mark on the page is, so the ink already produced is \
         superseded"
    );
}

/// It applies to every open document and to every one opened afterwards.
///
/// `Command::Restrict`'s rule, for its reason: this is a fact about the *reader* rather than about
/// any one file, so a document opened after the answer was given inherits it rather than starting
/// at the default nobody chose.
#[test]
fn the_preference_reaches_a_document_opened_after_it_was_stated() {
    let mut viewer = Viewer::new(400, 400, 1.0);
    opened(DocumentId(1), &mut viewer);
    viewer.handle(Command::Separations(true)).for_each(drop);
    opened(DocumentId(2), &mut viewer);

    // The second document is the one the answer was never sent about. Re-stating the same answer
    // must move nothing anywhere, which is what says it was inherited rather than defaulted: a
    // document that had started at `false` would be superseded here.
    let again: Vec<Event> = viewer.handle(Command::Separations(true)).collect();
    assert!(
        !redrawn(&again),
        "the document opened afterwards already held the answer the reader gave"
    );

    // And the other way round, as the control (trap 13): turning it off does move both.
    let off: Vec<Event> = viewer.handle(Command::Separations(false)).collect();
    assert!(
        redrawn(&off),
        "turning it off is a change, and a test that could not see one would measure nothing"
    );
}

//! ISO 32000-2 Annex K.1: the core reports XFA identity to its host.

#![expect(
    clippy::expect_used,
    clippy::panic,
    reason = "a missing required fixture or report event must fail the test, not pass silently"
)]

use std::path::{Path, PathBuf};

use pdf_model::Pages;
use pdf_syntax::Document;
use viewer_core::{Command, DocumentId, Event, Viewer};

const DOCUMENT_ID: DocumentId = DocumentId(1);

/// The required checked-in hybrid fixture; it is not silently optional in this focused test.
fn hybrid_xfa_bytes() -> Vec<u8> {
    let path: PathBuf = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../doc/pdf.js/test/pdfs/annotation-text-widget.pdf");
    let bytes = std::fs::read(&path).unwrap_or_else(|error| {
        panic!(
            "required XFA fixture {} is available: {error}",
            path.display()
        )
    });
    let parsed = Document::open(bytes.clone()).expect("the checked-in hybrid PDF opens");
    assert!(
        !parsed.was_recovered(),
        "the fixture uses its own cross-reference data"
    );
    assert!(
        Pages::new(&parsed).get(0).is_some(),
        "the hybrid has a page"
    );
    bytes
}

/// Annex K.1 requires an interactive processor that supports XFA to tell the user when they are
/// interacting with an XFA form. The automatic first-page report reaches the host as
/// `Event::Reported`; make sure that path clearly identifies a real hybrid form.
#[test]
fn a_hybrid_xfa_report_reaches_the_host_with_its_identity() {
    let mut viewer = Viewer::new(800, 1000, 1.0);
    let opening: Vec<Event> = viewer
        .handle(Command::Open {
            id: DOCUMENT_ID,
            bytes: hybrid_xfa_bytes().into(),
            password: None,
            fragment: None,
        })
        .collect();
    assert!(
        opening
            .iter()
            .any(|event| matches!(event, Event::Opened { pages: 1, .. })),
        "the complete hybrid document opened: {opening:?}"
    );

    // A core host asks for the document report after the first frame. This exercises the same
    // Report event path while keeping the test independent of a display toolkit.
    let report: Vec<String> = viewer
        .handle(Command::Report)
        .filter_map(|event| match event {
            Event::Reported {
                page: None, notes, ..
            } => Some(notes),
            _ => None,
        })
        .flatten()
        .collect();
    assert!(
        report.iter().any(|note| {
            note.contains("carries an XFA form (Annex K)")
                && note.contains("the form shown and filled here is its AcroForm")
        }),
        "the host receives a clear indication that the file is an XFA form: {report:?}"
    );
}

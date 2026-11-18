//! ISO 32000-2 §12.8.3.4.5: a `PAdES` signature is verified "when the user opens a signed document
//! or requests verification of these signatures present in the PDF" (ADR 2969).
//!
//! The fixture is the exact-xref `ETSI.CAdES.detached` document of `current_crl_report_spec_audit`.
//! Its report names the signature's cryptographic check, which is what verification produced; the
//! open answers before any digest, and the first present carries the verification.
use viewer_core::{Command, DocumentId, Event, Viewer};

const DOCUMENT: DocumentId = DocumentId(7);
const SIGNED: &[u8] = include_bytes!("fixtures/current_crl_report/current.pdf");
const VERIFIED: &str = "signature verifies under";

fn said(events: &[Event]) -> Vec<String> {
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

#[test]
fn a_pades_signature_is_verified_at_the_open_once_page_one_is_shown() {
    let document = pdf_syntax::Document::open(SIGNED.to_vec()).expect("the fixture opens");
    assert!(!document.was_recovered(), "the fixture's xref is exact");
    let mut viewer = Viewer::new(200, 200, 1.0);
    let opened: Vec<Event> = viewer
        .handle(Command::Open {
            id: DOCUMENT,
            bytes: SIGNED.to_vec().into(),
            password: None,
            fragment: None,
        })
        .collect();
    assert!(
        opened
            .iter()
            .any(|event| matches!(event, Event::Opened { .. })),
        "{opened:?}"
    );
    // Page one waits for no digest: the open answers before the verification.
    assert!(
        !said(&opened).iter().any(|note| note.contains(VERIFIED)),
        "{opened:?}"
    );
    let presented: Vec<Event> = viewer.handle(Command::Presented).collect();
    let verified = said(&presented);
    assert!(
        verified.iter().any(|note| note.contains(VERIFIED)),
        "§12.8.3.4.5 verifies a PAdES signature when the document is opened, without a request: \
         {presented:?}"
    );
    // Once a document, and a later request answers the same report.
    let again: Vec<Event> = viewer.handle(Command::Presented).collect();
    assert!(said(&again).is_empty(), "{again:?}");
    let reported: Vec<Event> = viewer.handle(Command::Report).collect();
    assert_eq!(said(&reported), verified);
}

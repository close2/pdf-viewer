//! §7.6.5.1's reader identity, supplied by a host and used by an open (ADR 2822).
//!
//! ISO 32000-2 §7.6.5.1: "the PDF reader shall scan the recipient list for which the content is
//! encrypted and shall attempt to find a match with a certificate that belongs to the user." The
//! fixtures are `pdf-syntax`'s: `adbe-pkcs7-s4.pdf` is enveloped by this tree's own generator to
//! the `PDFBox` `testnutzer` certificate (`doc/third-party-data.md`), and `test` is a certificate no
//! recipient list in it names.

#![expect(
    clippy::expect_used,
    reason = "test code: a fixture that cannot exercise the rule must fail loudly rather than \
              pass by doing nothing"
)]

use std::path::{Path, PathBuf};

use viewer_core::{Command, DocumentId, Event, RecipientSecret, Viewer};

fn witnesses() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../pdf-syntax/tests/public_key_witnesses")
}

fn read(name: &str) -> Vec<u8> {
    std::fs::read(witnesses().join(name)).expect("a fixture")
}

/// `certificate`'s DER with `key`'s private key.
fn recipient(certificate: &str, key: &str) -> RecipientSecret {
    RecipientSecret::new(
        read(&format!("{certificate}.certificate.der")),
        read(&format!("{key}.key.der")),
    )
}

/// Opens the s4 fixture under `DocumentId(1)` and answers what came back.
fn open(viewer: &mut Viewer) -> Vec<Event> {
    viewer
        .handle(Command::Open {
            id: DocumentId(1),
            bytes: read("adbe-pkcs7-s4.pdf").into(),
            password: None,
            fragment: None,
        })
        .collect()
}

fn opened(events: &[Event]) -> bool {
    events
        .iter()
        .any(|event| matches!(event, Event::Opened { document, .. } if *document == DocumentId(1)))
}

fn recipient_required(events: &[Event]) -> bool {
    events.iter().any(|event| {
        matches!(event, Event::RecipientRequired { document } if *document == DocumentId(1))
    })
}

/// With no key supplied, §7.6.5.1's scan finds no match and the window is asked for one: the
/// prompt, not a failure, and nothing opens.
#[test]
fn a_document_encrypted_to_recipients_asks_for_a_key_when_none_is_held() {
    let mut viewer = Viewer::new(200, 200, 1.0);
    let events = open(&mut viewer);
    assert!(recipient_required(&events), "{events:?}");
    assert!(!opened(&events), "{events:?}");
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, Event::OpenFailed { .. })),
        "a key nobody supplied is not a broken file: {events:?}"
    );
}

/// The certificate a recipient list names, with its own key: the match is found and the file
/// opens.
#[test]
fn the_named_certificate_and_its_key_open_the_document() {
    let mut viewer = Viewer::new(200, 200, 1.0);
    drop(viewer.handle(Command::Recipient(Some(recipient(
        "testnutzer",
        "testnutzer",
    )))));
    let events = open(&mut viewer);
    assert!(opened(&events), "{events:?}");
}

/// A certificate no recipient list names is §7.6.5.1's scan finding no match: asked again, as
/// with no key at all.
#[test]
fn a_certificate_no_recipient_list_names_is_asked_again() {
    let mut viewer = Viewer::new(200, 200, 1.0);
    drop(viewer.handle(Command::Recipient(Some(recipient("test", "test")))));
    let events = open(&mut viewer);
    assert!(recipient_required(&events), "{events:?}");
    assert!(!opened(&events), "{events:?}");
}

/// The named certificate with a key that is not its own: the match is found and the envelope does
/// not open, which another prompt would not help — so it is a failure, in
/// `SyntaxError::RecipientKey`'s words.
#[test]
fn the_named_certificate_with_another_key_is_said_as_the_key() {
    let mut viewer = Viewer::new(200, 200, 1.0);
    drop(viewer.handle(Command::Recipient(Some(recipient("testnutzer", "test")))));
    let events = open(&mut viewer);
    let reason = events
        .iter()
        .find_map(|event| match event {
            Event::OpenFailed { reason, .. } => Some(reason.as_str()),
            _ => None,
        })
        .unwrap_or_else(|| panic!("an OpenFailed: {events:?}"));
    assert!(
        reason.starts_with("the recipient key cannot be used"),
        "{reason}"
    );
    assert!(!recipient_required(&events), "{events:?}");
}

/// `Command::Recipient(None)` withdraws the key: the next open asks again.
#[test]
fn a_withdrawn_key_is_no_key() {
    let mut viewer = Viewer::new(200, 200, 1.0);
    drop(viewer.handle(Command::Recipient(Some(recipient(
        "testnutzer",
        "testnutzer",
    )))));
    drop(viewer.handle(Command::Recipient(None)));
    let events = open(&mut viewer);
    assert!(recipient_required(&events), "{events:?}");
}

/// An elliptic-curve recipient (RFC 5652 section 6.2.2, ADR 2821) opens through the same command.
#[test]
fn an_elliptic_curve_key_opens_its_document() {
    let mut viewer = Viewer::new(200, 200, 1.0);
    drop(viewer.handle(Command::Recipient(Some(recipient("ec-p256", "ec-p256")))));
    let events: Vec<Event> = viewer
        .handle(Command::Open {
            id: DocumentId(1),
            bytes: read("key-agreement-s5.pdf").into(),
            password: None,
            fragment: None,
        })
        .collect();
    assert!(opened(&events), "{events:?}");
}

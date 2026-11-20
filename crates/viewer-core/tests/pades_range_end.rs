//! The report's whole-file sentence for a `PAdES` signature, on both of its sides (ADR 3230).
//!
//! Table 255 says an `ETSI.CAdES.detached` range "shall cover the entire PDF file", and §12.8.1
//! says which bytes that is in a file of several revisions: the range runs "to the end of the
//! \"%%EOF\" comment, possibly followed by an optional EOL marker, terminating the incremental
//! update that adds the digital signature dictionary to the document". So a later revision
//! after the signature is the ordinary case and draws the plain sentence, and a range that stops
//! anywhere else inside the file draws the departure.
//!
//! The fixture is `current_crl_report_spec_audit`'s: one `ETSI.CAdES.detached` signature whose
//! range covers the whole file, the last ten-digit member of its `/ByteRange` its tail's length.
use std::collections::BTreeMap;

use pdf_syntax::object::Object;
use pdf_syntax::write::incremental_update;
use viewer_core::{Command, DocumentId, Event, Viewer};

const DOCUMENT: DocumentId = DocumentId(1);
const SIGNED: &[u8] = include_bytes!("fixtures/current_crl_report/current.pdf");
/// The report's word for a range Table 255 holds to the whole file and which falls short of it.
const BREAKS: &str = "breaks that requirement";
/// The `PAdES` departure list's sentence for the same rule, §12.8.3.4.2.
const PADES_RANGE: &str = "§12.8.3.4.2 requires its /ByteRange to cover the entire file";

fn report(bytes: Vec<u8>) -> Vec<String> {
    let mut viewer = Viewer::new(800, 1000, 1.0);
    viewer
        .handle(Command::Open {
            id: DOCUMENT,
            bytes: bytes.into(),
            password: None,
            fragment: None,
        })
        .for_each(drop);
    viewer
        .handle(Command::Report)
        .filter_map(|event| match event {
            Event::Reported {
                page: None, notes, ..
            } => Some(notes),
            _ => None,
        })
        .flatten()
        .collect()
}

#[test]
fn a_later_revision_draws_the_plain_sentence_and_no_departure() {
    let original = pdf_syntax::Document::open(SIGNED.to_vec()).expect("the fixture opens");
    let Some(Object::Reference(root)) = original.trailer().get("Root").cloned() else {
        unreachable!("the fixture's catalog is indirect")
    };
    let catalog = original.catalog().expect("a catalog");
    let updated = incremental_update(
        &original,
        &BTreeMap::from([(root, Object::Dictionary(catalog))]),
    )
    .expect("the update writes");
    let notes = report(updated);
    assert!(
        notes
            .iter()
            .any(|note| note.contains("appended after that signature")),
        "{notes:?}"
    );
    assert!(
        !notes
            .iter()
            .any(|note| note.contains(BREAKS) || note.contains(PADES_RANGE)),
        "{notes:?}"
    );
}

#[test]
fn a_range_that_stops_inside_its_revision_draws_the_departure() {
    // The last member of `/ByteRange`, written ten digits wide, shortened by four: the range now
    // ends inside the `%%EOF` line rather than after it, and the file is otherwise the same.
    let mut bytes = SIGNED.to_vec();
    let close = bytes
        .windows(b"/ByteRange [".len())
        .position(|window| window == b"/ByteRange [")
        .and_then(|at| {
            bytes[at..]
                .iter()
                .position(|byte| *byte == b']')
                .map(|offset| at.saturating_add(offset))
        })
        .expect("a /ByteRange array");
    let digits = close.saturating_sub(10)..close;
    let stated: u64 = std::str::from_utf8(bytes.get(digits.clone()).expect("ten digits"))
        .expect("ASCII digits")
        .parse()
        .expect("a length");
    let shorter = format!("{:010}", stated.saturating_sub(4));
    bytes.splice(digits, shorter.bytes());

    let notes = report(bytes);
    assert!(notes.iter().any(|note| note.contains(BREAKS)), "{notes:?}");
    assert!(
        notes.iter().any(|note| note.contains(PADES_RANGE)),
        "{notes:?}"
    );
}

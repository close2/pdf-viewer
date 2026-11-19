//! Review 2128's audit of ISO 32000-2 §12.8 (`doc/reviews/2128-specification-audit-12-8.md`),
//! finding 2128-B07: a `PAdES` signature followed by the incremental update §12.8.4.3 itself
//! prescribes is reported as a file breaking Table 255.
//!
//! Table 255 says an `ETSI.CAdES.detached` range "shall cover the entire PDF file", and §12.8.1
//! says which file that is once more than one revision exists: "In case of multiple digital
//! signatures this range shall be the sequence of bytes starting from the \"%PDF-\" comment at the
//! beginning of the PDF document to the end of the \"%%EOF\" comment, possibly followed by an
//! optional EOL marker, terminating the incremental update that adds the digital signature
//! dictionary to the document." §12.8.4.3 then requires the later update: "If the signature
//! dictionary to which a VRI dictionary applies is itself in an incremental update section, the
//! DSS/VRI update shall be done later than the signature update." So an update appended after a
//! `PAdES` signature is the standard's own long-term-validation shape, and not a departure.

use std::collections::BTreeMap;

use pdf_syntax::object::Object;
use pdf_syntax::write::incremental_update;
use viewer_core::{Command, DocumentId, Event, Viewer};

const DOCUMENT: DocumentId = DocumentId(1);
/// `current_crl_report_spec_audit.rs`'s fixture: one `ETSI.CAdES.detached` signature whose
/// revision also carries a `/DSS`, its range covering the whole file.
const SIGNED: &[u8] = include_bytes!("fixtures/current_crl_report/current.pdf");

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
fn a_dss_update_after_a_pades_signature_is_not_reported_as_a_departure_spec_audit() {
    let original = pdf_syntax::Document::open(SIGNED.to_vec()).expect("the signed fixture opens");
    let Some(Object::Reference(root)) = original.trailer().get("Root").cloned() else {
        unreachable!("the fixture's catalog is indirect")
    };
    let catalog = original.catalog().expect("a catalog");
    assert!(catalog.get("DSS").is_some(), "the fixture carries a /DSS");
    // The update §12.8.4.3 prescribes: the catalog, with its `/DSS`, written again in a later
    // revision than the signature's.
    let updated = incremental_update(
        &original,
        &BTreeMap::from([(root, Object::Dictionary(catalog))]),
    )
    .expect("the update writes");
    assert!(updated.len() > SIGNED.len(), "a revision was appended");

    let notes = report(updated);
    assert!(
        notes
            .iter()
            .any(|note| note.contains("appended after that signature")),
        "control: the report sees the appended revision: {notes:?}"
    );
    assert!(
        !notes
            .iter()
            .any(|note| note.contains("breaks that requirement")),
        "§12.8.1 and §12.8.4.3 make a later DSS update the conforming shape: {notes:?}"
    );
}

//! Public regression for Table 273 /V and §12.11.4 minimum-version semantics.
//!
//! A /V /2.1 requirement names a forward PDF minimum while this reader advertises PDF 2.0.
//! The source permits dotted version names and describes /V as minimum support needed. The
//! absent-/V and /V /2.0 cases are controls: absent means disregard version, and 2.0 is the
//! reader's supported version (`pdf_syntax::Version::SUPPORTED`).

#![expect(
    clippy::arithmetic_side_effects,
    reason = "small generated PDF xref offsets and object numbers"
)]
#![expect(
    clippy::expect_used,
    reason = "the public report and opening behavior are required observations"
)]

use std::fmt::Write as _;

use pdf_syntax::Document;
use viewer_core::{Command, DocumentId, Event, Viewer};

const DOC: DocumentId = DocumentId(1);

fn pdf(requirement: &str) -> Vec<u8> {
    let objects = [
        format!("<< /Type /Catalog /Pages 2 0 R /Requirements [{requirement}] >>"),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Resources << >> >>".to_owned(),
    ];
    let mut bytes = String::from("%PDF-2.0\n");
    let mut offsets = Vec::with_capacity(objects.len());
    for (index, object) in objects.iter().enumerate() {
        offsets.push(bytes.len());
        let _ = writeln!(bytes, "{} 0 obj\n{object}\nendobj", index + 1);
    }
    let xref_at = bytes.len();
    let size = objects.len() + 1;
    let _ = writeln!(bytes, "xref\n0 {size}\n0000000000 65535 f ");
    for offset in offsets {
        let _ = writeln!(bytes, "{offset:010} 00000 n ");
    }
    let _ = write!(
        bytes,
        "trailer\n<< /Size {size} /Root 1 0 R \
         /ID [<0123456789abcdef0123456789abcdef> <fedcba9876543210fedcba9876543210>] >>\n\
         startxref\n{xref_at}\n%%EOF\n"
    );
    bytes.into_bytes()
}

fn report(bytes: Vec<u8>) -> Vec<String> {
    let parsed = Document::open(bytes.clone()).expect("synthetic PDF opens");
    assert!(
        !parsed.was_recovered(),
        "xref offsets are generated from exact bytes"
    );
    let mut viewer = Viewer::new(300, 300, 1.0);
    let opened: Vec<Event> = viewer
        .handle(Command::Open {
            id: DOC,
            bytes: bytes.into(),
            password: None,
            fragment: None,
        })
        .collect();
    assert!(
        opened
            .iter()
            .any(|event| matches!(event, Event::Opened { .. })),
        "the supported baseline opens: {opened:?}"
    );
    let reported: Vec<_> = viewer.handle(Command::Report).collect();
    // Empty supported documents can have no report event. The unknown-type control below
    // proves that this route exposes unmet requirements instead of passing vacuously.
    reported
        .into_iter()
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
fn a_requirement_minimum_version_affects_the_public_unmet_report() {
    assert_eq!(pdf_syntax::Version::SUPPORTED.to_string(), "2.0");
    // Negative control: the report actually exposes unsupported requirements.
    let unknown = report(pdf("<< /S /FutureFeature /Penalty 100 >>"));
    assert!(
        unknown
            .iter()
            .any(|note| note.contains("requires FutureFeature")),
        "{unknown:?}"
    );
    // Positive control: absent /V explicitly means satisfaction ignores version.
    let absent = report(pdf("<< /S /Navigation /Penalty 100 >>"));
    assert!(
        absent
            .iter()
            .all(|note| !note.contains("requires Navigation")),
        "this viewer's Navigation support satisfies an unversioned requirement: {absent:?}"
    );

    // Positive control: PDF 2.0 is the version the reader implements.
    let current = report(pdf("<< /S /Navigation /V /2.0 /Penalty 100 >>"));
    assert!(
        current
            .iter()
            .all(|note| !note.contains("requires Navigation")),
        "the current supported version is sufficient: {current:?}"
    );

    // Negative case: a requirement specifying PDF 2.1 cannot be met by this PDF 2.0
    // processor, so the document-level report should identify Navigation as unmet.
    let future = report(pdf("<< /S /Navigation /V /2.1 /Penalty 100 >>"));
    assert!(
        future
            .iter()
            .any(|note| note.contains("requires Navigation")),
        "the present minimum version must affect requirement satisfaction: {future:?}"
    );
}

#[test]
fn requirement_versions_reach_the_enabled_processing_policy() {
    use pdf_model::restriction::Operation;
    use viewer_core::{RestrictionLevel, RestrictionPolicy, RestrictionScope};
    let opening = |requirements: &str| -> Vec<Event> {
        let bytes = pdf(requirements);
        let parsed = Document::open(bytes.clone()).expect("valid generated PDF");
        assert!(!parsed.was_recovered());
        let mut viewer = Viewer::new(300, 300, 1.0);
        viewer
            .handle(Command::Restrict(RestrictionScope::Window(
                RestrictionPolicy::default().with(Operation::Process, RestrictionLevel::On),
            )))
            .for_each(drop);
        viewer
            .handle(Command::Open {
                id: DOC,
                bytes: bytes.into(),
                password: None,
                fragment: None,
            })
            .collect()
    };
    // Two supported features at the advertised version contribute zero penalty.
    let current = opening(
        "<< /S /Navigation /V /2.0 /Penalty 60 >> << /S /Attachment /V /2.0 /Penalty 60 >>",
    );
    assert!(
        current
            .iter()
            .any(|event| matches!(event, Event::Opened { .. })),
        "{current:?}"
    );
    // Two unknown types prove the enabled policy really refuses a total above100.
    let unknown = opening("<< /S /FutureOne /Penalty 60 >> << /S /FutureTwo /Penalty 60 >>");
    assert!(
        unknown.iter().any(|event| matches!(
            event,
            Event::Refused {
                operation: Operation::Process,
                ..
            }
        )),
        "{unknown:?}"
    );
    assert!(
        !unknown
            .iter()
            .any(|event| matches!(event, Event::Opened { .. })),
        "{unknown:?}"
    );
    // The same sum must apply when the known features require an unsupported minimum.
    let future = opening(
        "<< /S /Navigation /V /2.1 /Penalty 60 >> << /S /Attachment /V /2.1 /Penalty 60 >>",
    );
    assert!(
        future.iter().any(|event| matches!(
            event,
            Event::Refused {
                operation: Operation::Process,
                ..
            }
        )),
        "minimum-version penalties must reach Process: {future:?}"
    );
    assert!(
        !future
            .iter()
            .any(|event| matches!(event, Event::Opened { .. })),
        "{future:?}"
    );
}

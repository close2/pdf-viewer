//! Public viewer path for Table 246's required-to-display `/Status` string.

#![expect(
    clippy::expect_used,
    reason = "complete source and supplied FDF syntax are preconditions"
)]

use std::fmt::Write as _;

use viewer_core::{Command, DocumentId, Event, Purpose, Viewer};

const DOCUMENT: DocumentId = DocumentId(31);

fn source() -> Vec<u8> {
    let objects = [
        b"<< /Type /Catalog /Pages 2 0 R /AcroForm << /Fields [] >> >>".as_slice(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".as_slice(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 100] /Resources << >> /Annots [4 0 R] >>".as_slice(),
        b"<< /Type /Annot /Subtype /Link /Rect [10 10 100 40] /F 4 /A << /Type /Action /S /ImportData /F (answer.fdf) >> >>".as_slice(),
    ];
    let mut out = b"%PDF-2.0\n".to_vec();
    let mut offsets = Vec::new();
    for (index, object) in objects.iter().enumerate() {
        offsets.push(out.len());
        out.extend_from_slice(format!("{} 0 obj\n", index.saturating_add(1)).as_bytes());
        out.extend_from_slice(object);
        out.extend_from_slice(b"\nendobj\n");
    }
    let xref_at = out.len();
    let size = objects.len().saturating_add(1);
    out.extend_from_slice(format!("xref\n0 {size}\n0000000000 65535 f \n").as_bytes());
    for offset in offsets {
        out.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    out.extend_from_slice(
        format!(
            "trailer\n<< /Size {size} /Root 1 0 R /ID \
             [<30313233343536373839616263646566> <66656463626139383736353433323130>] >>\n\
             startxref\n{xref_at}\n%%EOF\n"
        )
        .as_bytes(),
    );
    out
}

fn fdf(status: Option<&str>) -> Vec<u8> {
    let status = status.map_or_else(String::new, |value| format!("/Status ({value}) "));
    let header = "%FDF-1.2\n";
    let mut bytes = format!("{header}1 0 obj\n<< /FDF << {status} >> >>\nendobj\n");
    let xref = bytes.len();
    let offset = header.len();
    let _ = write!(
        bytes,
        "xref\n0 2\n0000000000 65535 f \n{offset:010} 00000 n \ntrailer\n<< /Size 2 /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n"
    );
    bytes.into_bytes()
}

fn import(status: Option<&str>) -> Vec<Event> {
    let source = source();
    let parsed = pdf_syntax::Document::open(source.clone()).expect("source PDF syntax");
    assert!(!parsed.was_recovered(), "source xref is exact");
    let supplied = fdf(status);
    let parsed = pdf_syntax::Document::open(supplied.clone()).expect("supplied FDF syntax");
    assert!(!parsed.was_recovered(), "supplied FDF xref is exact");
    let mut viewer = Viewer::new(400, 300, 1.0);
    viewer
        .handle(Command::Open {
            id: DOCUMENT,
            bytes: source.into(),
            password: None,
            fragment: None,
        })
        .for_each(drop);
    let asked: Vec<Event> = viewer
        .handle(Command::Activate(pdf_syntax::ObjectId::new(4, 0)))
        .collect();
    assert!(
        asked.iter().any(|event| matches!(event, Event::NeedsFile {
            purpose: Purpose::ImportData,
            name,
            ..
        } if name == "answer.fdf")),
        "the actual link activation asks for the named FDF: {asked:?}"
    );
    viewer
        .handle(Command::Supply {
            purpose: Purpose::ImportData,
            bytes: Some(supplied),
        })
        .collect()
}

fn reported(events: &[Event]) -> Vec<String> {
    events
        .iter()
        .flat_map(|event| match event {
            Event::Reported { notes, .. } => notes.clone(),
            _ => Vec::new(),
        })
        .collect()
}

/// Table 246 says a stated Status shall be displayed. Exercise the reachable action and import
/// path, then contrast an FDF with no optional Status entry.
#[test]
fn importing_fdf_displays_status_and_absence_has_no_status_message() {
    let with_status = import(Some("The server accepted the form"));
    let notes = reported(&with_status);
    assert!(
        notes
            .iter()
            .any(|note| note.contains("status — The server accepted the form")),
        "the supplied FDF's status reaches the host-facing report: {notes:?}"
    );

    let without_status = import(None);
    let notes = reported(&without_status);
    assert!(
        notes.iter().all(|note| !note.contains("status —")),
        "the absence of optional Status does not invent a result: {notes:?}"
    );
}

//! ISO 32000-2 Tables 198 and 197: initial catalog, page and annotation action order.
#![expect(
    clippy::arithmetic_side_effects,
    clippy::expect_used,
    reason = "bounded exact-xref fixture and explicit reader checks"
)]

use std::fmt::Write as _;

use viewer_core::{Command, DocumentId, Event, Viewer};

const DOCUMENT: DocumentId = DocumentId(1);

fn exact_xref_pdf(open_action: bool, page_action: bool, annotation_action: bool) -> Vec<u8> {
    let catalog_open = if open_action {
        "/OpenAction << /S /URI /URI (https://example.invalid/catalog-open) >>"
    } else {
        ""
    };
    let page_open = if page_action {
        "/AA << /O << /S /URI /URI (https://example.invalid/page-open) >> >>"
    } else {
        ""
    };
    let annotation_open = if annotation_action {
        "/AA << /PO << /S /URI /URI (https://example.invalid/annotation-open) >> >>"
    } else {
        ""
    };
    let bodies = [
        format!("<< /Type /Catalog /Pages 2 0 R {catalog_open} >>"),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] /Resources << >> \
             /Annots [4 0 R] {page_open} >>"
        ),
        format!(
            "<< /Type /Annot /Subtype /Square /Rect [0 0 10 10] /AP << /N 5 0 R >> {annotation_open} >>"
        ),
        "<< /Type /XObject /Subtype /Form /BBox [0 0 10 10] /Resources << >> /Length 0 >>\nstream\n\nendstream".to_owned(),
    ];

    let mut out = String::from("%PDF-1.7\n");
    let mut offsets = Vec::with_capacity(bodies.len());
    for (index, body) in bodies.iter().enumerate() {
        offsets.push(out.len());
        let _ = write!(out, "{} 0 obj\n{body}\nendobj\n", index + 1);
    }
    let xref_at = out.len();
    let size = bodies.len() + 1;
    let _ = write!(out, "xref\n0 {size}\n0000000000 65535 f \n");
    for offset in offsets {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    let _ = write!(
        out,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n"
    );
    out.into_bytes()
}

fn uris(events: &[Event]) -> Vec<String> {
    events
        .iter()
        .filter_map(|event| match event {
            Event::OpenUri { uri, .. } => Some(uri.clone()),
            _ => None,
        })
        .collect()
}

fn open_and_present(bytes: Vec<u8>) -> Vec<String> {
    let document = pdf_syntax::Document::open(bytes.clone()).expect("valid fixture");
    assert!(!document.was_recovered());
    let mut viewer = Viewer::new(200, 200, 1.0);
    let opened: Vec<Event> = viewer
        .handle(Command::Open {
            id: DOCUMENT,
            bytes: bytes.into(),
            password: None,
            fragment: None,
        })
        .collect();
    assert!(
        opened
            .iter()
            .any(|event| matches!(event, Event::Opened { .. })),
        "fixture must open without recovery or parse failure: {opened:#?}"
    );
    assert!(
        !opened
            .iter()
            .any(|event| matches!(event, Event::Reported { .. })),
        "the valid exact-xref fixture must not rely on recovery: {opened:#?}"
    );
    let mut observed = uris(&opened);
    let presented: Vec<Event> = viewer.handle(Command::Presented).collect();
    observed.extend(uris(&presented));
    let repeated: Vec<Event> = viewer.handle(Command::Presented).collect();
    assert!(uris(&repeated).is_empty(), "open actions run once");
    observed
}

#[test]
fn catalog_open_action_precedes_initial_page_and_annotation_open_actions() {
    // Independent no-catalog-action control: page and annotation triggers still run in order.
    assert_eq!(
        open_and_present(exact_xref_pdf(false, true, true)),
        [
            "https://example.invalid/page-open",
            "https://example.invalid/annotation-open"
        ]
    );

    assert_eq!(
        open_and_present(exact_xref_pdf(true, false, false)),
        ["https://example.invalid/catalog-open"]
    );
    // Table 198 orders page O after catalog OpenAction; Table 197 orders annotation PO
    // after both. Absolute URIs expose dispatch order without an external network operation.
    assert_eq!(
        open_and_present(exact_xref_pdf(true, true, true)),
        [
            "https://example.invalid/catalog-open",
            "https://example.invalid/page-open",
            "https://example.invalid/annotation-open"
        ]
    );
}

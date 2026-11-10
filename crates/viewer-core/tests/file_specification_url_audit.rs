//! Viewer-core integration test for §§7.11.2.2 and 7.11.5 URL handling.
//!
//! This drives a `GoToE` action through the public `Viewer` interface and observes the exact
//! `NeedsFile` name handed to the host.
#![expect(clippy::expect_used, reason = "complete generated fixtures must open")]

use std::fmt::Write as _;
use viewer_core::{Command, DocumentId, Event, Purpose, Viewer};

const DOCUMENT: DocumentId = DocumentId(41);

fn source(specification: &str) -> Vec<u8> {
    let objects = [
        "<< /Type /Catalog /Pages 2 0 R /URI << /Base \
         (https://example.test/root/document.pdf) >> >>",
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Resources << >> \
         /Annots [4 0 R] >>",
        &format!(
            "<< /Type /Annot /Subtype /Link /Rect [0 0 50 50] /A \
             << /Type /Action /S /GoToE /D [0 /Fit] /F {specification} \
             /T << /R /C /N (child.pdf) >> >> >>"
        ),
    ];
    let mut bytes = String::from("%PDF-1.7\n");
    let mut offsets = Vec::with_capacity(objects.len());
    for (index, body) in objects.iter().enumerate() {
        offsets.push(bytes.len());
        writeln!(bytes, "{} 0 obj\n{body}\nendobj", index.saturating_add(1))
            .expect("String writes cannot fail");
    }
    let xref_offset = bytes.len();
    writeln!(
        bytes,
        "xref\n0 {}\n0000000000 65535 f ",
        objects.len().saturating_add(1)
    )
    .expect("String writes cannot fail");
    for offset in offsets {
        writeln!(bytes, "{offset:010} 00000 n ").expect("String writes cannot fail");
    }
    write!(
        bytes,
        "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref_offset}\n%%EOF\n",
        objects.len().saturating_add(1)
    )
    .expect("String writes cannot fail");
    bytes.into_bytes()
}

fn asked(specification: &str) -> Vec<Event> {
    let bytes = source(specification);
    let parsed = pdf_syntax::Document::open(bytes.clone()).expect("a valid PDF 1.7 fixture");
    assert!(!parsed.was_recovered(), "the written xref is used");
    assert_eq!(
        pdf_model::Pages::new(&parsed).len(),
        1,
        "the page-tree traversal finds one leaf"
    );

    let mut viewer = Viewer::new(400, 400, 1.0);
    viewer
        .handle(Command::Open {
            id: DOCUMENT,
            bytes: bytes.into(),
            password: None,
            fragment: None,
        })
        .for_each(drop);
    viewer
        .handle(Command::Activate(pdf_syntax::ObjectId::new(4, 0)))
        .collect()
}

fn target_name(events: &[Event]) -> Option<String> {
    events.iter().find_map(|event| match event {
        Event::NeedsFile {
            purpose: Purpose::TargetRoot,
            name,
            ..
        } => Some(name.clone()),
        _ => None,
    })
}

#[test]
fn go_to_e_url_target_resolves_a_valid_rfc3986_path_before_the_host_request() {
    // Positive control first: an ASCII path reaches the host as the URI resolved against /Base.
    let control = asked("<< /Type /Filespec /FS /URL /F (images/figure.pdf) >>");
    assert_eq!(
        target_name(&control).as_deref(),
        Some("https://example.test/root/images/figure.pdf"),
        "the ASCII control reaches the host request"
    );

    // §7.11.5 says /FS /URL /F is already an RFC 3986 URL, with PDFDocEncoding's 7-bit subset.
    // Preserve valid percent escapes while resolving the relative URL. This case deliberately
    // avoids asserting §7.11.2.2's conversion for an ordinary byte-string file spec.
    let encoded = asked("<< /Type /Filespec /FS /URL /F (images/caf%C3%A9%20pic.pdf) >>");
    assert_eq!(
        target_name(&encoded).as_deref(),
        Some("https://example.test/root/images/caf%C3%A9%20pic.pdf"),
        "the host request preserves a valid URL's percent escapes"
    );
}

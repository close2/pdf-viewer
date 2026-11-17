//! Public consumer control: §7.3.10 object-representation control at Table 31 /AA and page /O.
#![expect(
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    reason = "the exact-xref fixture and public action result are the assertion"
)]

use std::fmt::Write as _;

use pdf_syntax::Document;
use viewer_core::{Command, DocumentId, Event, Viewer};

const DOC: DocumentId = DocumentId(1);

/// Same page-open URI action, directly represented or through indirect /AA and /O objects.
fn fixture(indirect: bool) -> Vec<u8> {
    let page = if indirect {
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] /Resources << >> /AA 4 0 R >>"
    } else {
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] /Resources << >> \
         /AA << /O << /S /URI /URI (https://example.invalid/page-open) >> >> >>"
    };
    let mut objects = vec![
        "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        page.to_owned(),
    ];
    if indirect {
        objects.push("<< /O 5 0 R >>".to_owned());
        objects.push("<< /S /URI /URI (https://example.invalid/page-open) >>".to_owned());
    }

    let mut bytes = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for (index, object) in objects.iter().enumerate() {
        offsets.push(bytes.len());
        let _ = writeln!(bytes, "{} 0 obj\n{object}\nendobj", index + 1);
    }
    let xref = bytes.len();
    let size = objects.len() + 1;
    let _ = writeln!(bytes, "xref\n0 {size}\n0000000000 65535 f ");
    for offset in offsets {
        let _ = writeln!(bytes, "{offset:010} 00000 n ");
    }
    let _ = write!(
        bytes,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n"
    );
    bytes.into_bytes()
}

fn open_and_present(bytes: Vec<u8>) -> Vec<String> {
    let syntax = Document::open(bytes.clone()).expect("complete exact-xref fixture");
    assert!(
        !syntax.was_recovered(),
        "fixture must not rely on xref recovery"
    );
    let mut viewer = Viewer::new(200, 200, 1.0);
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
            .any(|event| matches!(event, Event::Opened { document, pages: 1 } if *document == DOC))
    );
    let mut uris: Vec<String> = opened
        .iter()
        .filter_map(|event| match event {
            Event::OpenUri { uri, .. } => Some(uri.clone()),
            _ => None,
        })
        .collect();
    let presented: Vec<Event> = viewer.handle(Command::Presented).collect();
    uris.extend(presented.iter().filter_map(|event| match event {
        Event::OpenUri { uri, .. } => Some(uri.clone()),
        _ => None,
    }));
    uris
}

#[test]
fn page_aa_and_open_action_accept_direct_or_indirect_objects() {
    let expected = ["https://example.invalid/page-open"];
    assert_eq!(
        open_and_present(fixture(false)),
        expected,
        "direct positive control"
    );
    assert_eq!(
        open_and_present(fixture(true)),
        expected,
        "indirect /AA and /O equivalent"
    );
}

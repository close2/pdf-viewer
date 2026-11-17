//! Regression for a `GoToE` from an embedded child back to its containing document.
//! Exact xrefs, valid PDF1.7 fixtures and an independently distinguished parent page.

#![expect(
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    reason = "fixture and public event assertions must fail loudly"
)]

use viewer_core::{Answer, Command, DocumentId, Event, Query, Viewer};

const VIEW: DocumentId = DocumentId(41);

fn assembled(objects: &[Vec<u8>], id: &str) -> Vec<u8> {
    let mut out = b"%PDF-1.7\n%\xE2\xE3\xCF\xD3\n".to_vec();
    let mut offsets = Vec::with_capacity(objects.len());
    for (i, object) in objects.iter().enumerate() {
        offsets.push(out.len());
        out.extend_from_slice(format!("{} 0 obj\n", i + 1).as_bytes());
        out.extend_from_slice(object);
        out.extend_from_slice(b"\nendobj\n");
    }
    let xref = out.len();
    let size = objects.len() + 1;
    out.extend_from_slice(format!("xref\n0 {size}\n0000000000 65535 f \n").as_bytes());
    for offset in offsets {
        out.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    out.extend_from_slice(
        format!(
            "trailer\n<< /Size {size} /Root 1 0 R /ID [<{id}><{id}>] >>\nstartxref\n{xref}\n%%EOF\n"
        )
        .as_bytes(),
    );
    out
}

fn child_pdf(id: &str) -> Vec<u8> {
    assembled(
        &[
            b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
            b"<< /Type /Pages /Count 1 /Kids [3 0 R] >>".to_vec(),
            b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Resources << >> /Annots [4 0 R] >>".to_vec(),
            b"<< /Type /Annot /Subtype /Link /Rect [0 0 50 50] /A << /Type /Action /S /GoToE /D [1 /Fit] /T << /R /P >> >> >>".to_vec(),
        ],
        id,
    )
}

fn root_pdf(child: &[u8], return_in_one_walk: bool) -> Vec<u8> {
    let mut embedded = format!(
        "<< /Type /EmbeddedFile /Subtype /application#2Fpdf /Length {} >>\nstream\n",
        child.len()
    )
    .into_bytes();
    embedded.extend_from_slice(child);
    embedded.extend_from_slice(b"\nendstream");
    assembled(
        &[
            b"<< /Type /Catalog /Pages 2 0 R /Names << /EmbeddedFiles << /Names [(child.pdf) 6 0 R] >> >> >>".to_vec(),
            b"<< /Type /Pages /Count 2 /Kids [3 0 R 4 0 R] >>".to_vec(),
            b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Resources << >> /Annots [5 0 R] >>".to_vec(),
            b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Resources << >> >>".to_vec(),
            if return_in_one_walk {
                b"<< /Type /Annot /Subtype /Link /Rect [0 0 50 50] /A << /Type /Action /S /GoToE /D [1 /Fit] /T << /R /C /N (child.pdf) /T << /R /P >> >> >> >>".to_vec()
            } else {
                b"<< /Type /Annot /Subtype /Link /Rect [0 0 50 50] /A << /Type /Action /S /GoToE /D [0 /Fit] /T << /R /C /N (child.pdf) >> >> >>".to_vec()
            },
            b"<< /Type /Filespec /F (child.pdf) /UF (child.pdf) /AFRelationship /Data /EF << /F 7 0 R /UF 7 0 R >> >>".to_vec(),
            embedded,
        ],
        "102132435465768798A9BACBDCEDFE0F",
    )
}

fn assert_exact(bytes: &[u8]) {
    let document = pdf_syntax::Document::open(bytes.to_vec()).expect("valid exact-xref PDF");
    assert!(!document.cross_reference_chain_was_cut());
    assert!(
        !document.was_recovered(),
        "fixture must not depend on recovery"
    );
}

#[test]
fn a_gotoe_from_a_child_can_name_its_parent_as_the_target() {
    let child = child_pdf("00112233445566778899AABBCCDDEEFF");
    let root = root_pdf(&child, false);
    assert_exact(&child);
    assert_exact(&root);

    let mut viewer = Viewer::new(400, 400, 1.0);
    let opened: Vec<Event> = viewer
        .handle(Command::Open {
            id: VIEW,
            bytes: root.into(),
            password: None,
            fragment: None,
        })
        .collect();
    assert!(
        opened
            .iter()
            .any(|e| matches!(e, Event::Opened { pages: 2, .. }))
    );
    assert!(
        !opened.iter().any(|e| matches!(e, Event::Reported { .. })),
        "initial root opens without recovery or reader reports"
    );

    let entered: Vec<Event> = viewer
        .handle(Command::Activate(pdf_syntax::ObjectId::new(5, 0)))
        .collect();
    assert!(
        entered
            .iter()
            .any(|e| matches!(e, Event::Opened { pages: 1, .. })),
        "positive control: the same-root child is entered via /R /C /N"
    );
    assert!(
        entered
            .iter()
            .any(|e| matches!(e, Event::PageChanged { index: 0, .. })),
        "positive control: /D is resolved in the child document"
    );

    // The child is now the current document and its own link walks one /R /P step. Under
    // Table 204/205 terminology the target is the root just left, page index 1 of 2.
    let returned: Vec<Event> = viewer
        .handle(Command::Activate(pdf_syntax::ObjectId::new(4, 0)))
        .collect();
    assert!(
        returned
            .iter()
            .any(|e| matches!(e, Event::Opened { pages: 2, .. })),
        "the parent PDF, not the one-page child, should be reopened: {returned:?}"
    );
    assert!(
        returned
            .iter()
            .any(|e| matches!(e, Event::PageChanged { index: 1, .. })),
        "the second page is independently distinguished from the child and source page"
    );
    assert!(
        !returned.iter().any(|e| matches!(e, Event::Reported { .. })),
        "a valid parent path should not be declined"
    );
    assert!(matches!(
        viewer.query(Query::CurrentPage),
        Answer::Page { index: 1, .. }
    ));
}

/// The action itself descends and ascends, so no ancestry across actions is needed.
#[test]
fn a_gotoe_walk_can_end_back_at_its_starting_parent() {
    let child = child_pdf("00112233445566778899AABBCCDDEEFF");
    let root = root_pdf(&child, true);
    assert_exact(&child);
    assert_exact(&root);
    let mut viewer = Viewer::new(400, 400, 1.0);
    let opened: Vec<Event> = viewer
        .handle(Command::Open {
            id: VIEW,
            bytes: root.into(),
            password: None,
            fragment: None,
        })
        .collect();
    assert!(
        opened
            .iter()
            .any(|e| matches!(e, Event::Opened { pages: 2, .. }))
    );
    assert!(matches!(
        viewer.query(Query::CurrentPage),
        Answer::Page { index: 0, .. }
    ));
    let returned: Vec<Event> = viewer
        .handle(Command::Activate(pdf_syntax::ObjectId::new(5, 0)))
        .collect();
    assert!(
        returned
            .iter()
            .any(|e| matches!(e, Event::PageChanged { index: 1, .. })),
        "C then P ends at the starting two-page root, destination page1: {returned:?}"
    );
    assert!(matches!(
        viewer.query(Query::CurrentPage),
        Answer::Page { index: 1, .. }
    ));
}

//! Public consumer regression for Table 205's child-by-attachment `/P` + `/A` route.

#![expect(
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    reason = "fixture and public event assertions must fail loudly"
)]

use viewer_core::{Answer, Command, DocumentId, Event, Query, Viewer};

const VIEW: DocumentId = DocumentId(42);

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

fn child_pdf() -> Vec<u8> {
    assembled(
        &[
            b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
            b"<< /Type /Pages /Count 2 /Kids [3 0 R 4 0 R] >>".to_vec(),
            b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Resources << >> >>".to_vec(),
            b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Resources << >> >>".to_vec(),
        ],
        "00112233445566778899AABBCCDDEEFF",
    )
}

fn root_pdf(child: &[u8]) -> Vec<u8> {
    let mut embedded = format!(
        "<< /Type /EmbeddedFile /Subtype /application#2Fpdf /Length {} >>\nstream\n",
        child.len()
    )
    .into_bytes();
    embedded.extend_from_slice(child);
    embedded.extend_from_slice(b"\nendstream");
    assembled(
        &[
            b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
            b"<< /Type /Pages /Count 1 /Kids [3 0 R] >>".to_vec(),
            b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Resources << >> /Annots [4 0 R 5 0 R] >>".to_vec(),
            b"<< /Type /Annot /Subtype /Link /Rect [0 0 50 50] /A << /Type /Action /S /GoToE /D [1 /Fit] /T << /R /C /P 0 /A 1 >> >> >>".to_vec(),
            b"<< /Type /Annot /Subtype /FileAttachment /Rect [60 0 90 30] /NM (attached-child) /FS 6 0 R >>".to_vec(),
            // Object 5's FS references this specification; object 7 is its embedded PDF stream.
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
        "fixture must not use xref recovery"
    );
}

#[test]
fn a_gotoe_child_can_be_selected_by_its_file_attachment_page_and_index() {
    let child = child_pdf();
    let root = root_pdf(&child);
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
            .any(|event| matches!(event, Event::Opened { pages: 1, .. }))
    );
    assert!(
        !opened
            .iter()
            .any(|event| matches!(event, Event::Reported { .. })),
        "the one-page parent with page resources is interpreted completely"
    );

    let switched: Vec<Event> = viewer
        .handle(Command::Activate(pdf_syntax::ObjectId::new(4, 0)))
        .collect();
    assert!(
        switched
            .iter()
            .any(|event| matches!(event, Event::Opened { pages: 2, .. })),
        "P=0 selects the only parent page and A=1 selects the second /Annots entry, the FileAttachment"
    );
    assert!(
        switched
            .iter()
            .any(|event| matches!(event, Event::PageChanged { index: 1, .. })),
        "the explicit /D is resolved in the opened two-page child at zero-based page index 1"
    );
    assert!(matches!(
        viewer.query(Query::CurrentPage),
        Answer::Page { index: 1, .. }
    ));
}

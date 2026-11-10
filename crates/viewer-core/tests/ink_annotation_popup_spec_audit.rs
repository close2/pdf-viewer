//! Viewer consumer test for §12.5.6.13's opened-ink popup requirement.
//! The Ink annotation has a stored appearance and a closed popup.

use std::fmt::Write as _;

use viewer_core::{Answer, Command, DocumentId, Query, Viewer};

fn ink_with_popup() -> Vec<u8> {
    let appearance = "0 0 20 20 re S";
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] \
         /Resources << >> /Contents 4 0 R /Annots [5 0 R 6 0 R] >>\nendobj\n\
         4 0 obj\n<< /Length 0 >>\nstream\n\nendstream\nendobj\n\
         5 0 obj\n<< /Type /Annot /Subtype /Ink /Rect [10 80 30 100] /F 4 \
         /InkList [[12 82 28 98]] /Contents (a handwritten note) /T (Mira) \
         /Popup 6 0 R /AP << /N 7 0 R >> >>\nendobj\n\
         6 0 obj\n<< /Type /Annot /Subtype /Popup /Rect [30 50 90 90] \
         /Parent 5 0 R /Open false >>\nendobj\n\
         7 0 obj\n<< /Type /XObject /Subtype /Form /BBox [0 0 20 20] /Resources << >> \
         /Length {} >>\nstream\n{appearance}\nendstream\nendobj\n",
        appearance.len().saturating_add(1)
    );
    assemble(&body)
}

fn assemble(body: &str) -> Vec<u8> {
    let mut out = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for object in body.split_inclusive("endobj\n") {
        offsets.push(out.len());
        out.push_str(object);
    }
    let xref_at = out.len();
    let size = offsets.len().saturating_add(1);
    let _ = writeln!(out, "xref\n0 {size}");
    out.push_str("0000000000 65535 f \n");
    for offset in offsets {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    let _ = write!(
        out,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n"
    );
    out.into_bytes()
}

#[test]
fn activating_an_ink_annotation_opens_a_popup_with_its_note_text() {
    let bytes = ink_with_popup();
    let document = pdf_syntax::Document::open(bytes.clone()).expect("complete PDF");
    assert!(!document.was_recovered(), "exact cross references");
    let mut viewer = Viewer::new(200, 200, 1.0);
    viewer
        .handle(Command::Open {
            id: DocumentId(1),
            bytes: bytes.into(),
            password: None,
            fragment: None,
        })
        .for_each(drop);

    assert!(
        matches!(viewer.query(Query::Popups), Answer::Popups(windows) if windows.is_empty()),
        "the popup starts closed"
    );
    viewer
        .handle(Command::Activate(pdf_syntax::ObjectId::new(5, 0)))
        .for_each(drop);

    let Answer::Popups(windows) = viewer.query(Query::Popups) else {
        panic!("the viewer answers popup queries with windows");
    };
    assert_eq!(windows.len(), 1, "the ink annotation's window is open");
    assert_eq!(
        windows[0].text.as_deref(),
        Some("a handwritten note"),
        "the popup displays the associated Ink annotation's /Contents"
    );
}

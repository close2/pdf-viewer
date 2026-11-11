//! Public viewer regression for §12.5.6.7's line-annotation popup requirement.

use std::fmt::Write as _;

use viewer_core::{Answer, Command, DocumentId, Query, Viewer};

fn line_with_popup() -> Vec<u8> {
    let appearance = "0 0 m 100 100 l S";
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] \
         /Resources << >> /Contents 4 0 R /Annots [5 0 R 6 0 R] >>\nendobj\n\
         4 0 obj\n<< /Length 0 >>\nstream\n\nendstream\nendobj\n\
         5 0 obj\n<< /Type /Annot /Subtype /Line /Rect [0 0 100 100] /F 4 \
         /L [10 10 90 90] /Contents (line measurement) /T (A. Reader) \
         /Popup 6 0 R /AP << /N 7 0 R >> >>\nendobj\n\
         6 0 obj\n<< /Type /Annot /Subtype /Popup /Rect [30 30 90 70] \
         /Parent 5 0 R /Open false >>\nendobj\n\
         7 0 obj\n<< /Type /XObject /Subtype /Form /BBox [0 0 100 100] /Resources << >> \
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
fn activating_a_line_annotation_opens_its_associated_note_popup() {
    let bytes = line_with_popup();
    let document = pdf_syntax::Document::open(bytes.clone()).expect("complete PDF");
    assert!(!document.was_recovered(), "the xref is exact");
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
        "the PDF states `/Open false`"
    );
    viewer
        .handle(Command::Activate(pdf_syntax::ObjectId::new(5, 0)))
        .for_each(drop);

    let Answer::Popups(windows) = viewer.query(Query::Popups) else {
        panic!("the viewer answers popup queries with windows");
    };
    assert_eq!(windows.len(), 1, "opening the line's associated popup");
    assert_eq!(windows[0].annotation, pdf_syntax::ObjectId::new(6, 0));
    assert_eq!(windows[0].parent, Some(pdf_syntax::ObjectId::new(5, 0)));
    assert_eq!(
        windows[0].text.as_deref(),
        Some("line measurement"),
        "the associated note text is presented"
    );
    assert_eq!(windows[0].title.as_deref(), Some("A. Reader"));
}

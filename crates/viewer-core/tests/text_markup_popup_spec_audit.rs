//! Public viewer check for ISO 32000-2 §12.5.6.10's associated-popup requirement.

#![expect(
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "small exact-xref fixtures; a failed premise must fail loudly"
)]

use std::fmt::Write as _;

use viewer_core::{Answer, Command, DocumentId, Query, Viewer};

const SUBTYPES: [&str; 4] = ["Highlight", "Underline", "Squiggly", "StrikeOut"];

fn markup_with_popup(subtype: &str) -> Vec<u8> {
    assert!(SUBTYPES.contains(&subtype));
    let content = "BT /F1 12 Tf 20 15 Td (WORD) Tj ET";
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] \
         /Resources << /Font << /F1 7 0 R >> >> /Contents 4 0 R \
         /Annots [5 0 R 6 0 R] >>\nendobj\n\
         4 0 obj\n<< /Length {} >>\nstream\n{}\nendstream\nendobj\n\
         5 0 obj\n<< /Type /Annot /Subtype /{} /Rect [18 12 60 30] /F 4 \
         /QuadPoints [18 12 60 12 60 30 18 30] /Contents (associated markup note) \
         /T (A. Reader) /Popup 6 0 R >>\nendobj\n\
         6 0 obj\n<< /Type /Annot /Subtype /Popup /Rect [30 30 90 70] \
         /Parent 5 0 R /Open false >>\nendobj\n\
         7 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>\nendobj\n",
        content.len(),
        content,
        subtype
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
    let size = offsets.len() + 1;
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
fn opening_each_text_markup_annotation_displays_its_associated_note() {
    for subtype in SUBTYPES {
        let bytes = markup_with_popup(subtype);
        let document = pdf_syntax::Document::open(bytes.clone()).expect("complete PDF");
        assert!(!document.was_recovered(), "{subtype}: exact xref");

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
            "{subtype}: /Open false is the initial-state control"
        );
        viewer
            .handle(Command::Activate(pdf_syntax::ObjectId::new(5, 0)))
            .for_each(drop);

        let Answer::Popups(windows) = viewer.query(Query::Popups) else {
            panic!("{subtype}: the viewer answers popup queries with windows");
        };
        assert_eq!(windows.len(), 1, "{subtype}: one associated popup");
        assert_eq!(windows[0].annotation, pdf_syntax::ObjectId::new(6, 0));
        assert_eq!(windows[0].parent, Some(pdf_syntax::ObjectId::new(5, 0)));
        assert_eq!(windows[0].text.as_deref(), Some("associated markup note"));
        assert_eq!(windows[0].title.as_deref(), Some("A. Reader"));
    }
}

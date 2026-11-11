//! Stamp popup activation with and without an explicit Popup dictionary.
//!
//! The Stamp has a stored normal appearance, so this does not test fallback artwork. It tests the
//! separate §12.5.6.12 SHALL that opening a stamp displays its associated note in a popup.

#![expect(
    clippy::panic,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "test fixture failures must not silently turn this regression into a pass"
)]

use std::fmt::Write as _;

use viewer_core::{Answer, Command, DocumentId, PointerAction, Query, Viewer};

const DOCUMENT: DocumentId = DocumentId(1);

/// Complete PDF 1.7 document with one visible Stamp, stored normal appearance, and optionally the
/// explicit Table172 Popup relationship. Stamp is a markup annotation (Table171), so the latter is
/// a valid control, not an invented dictionary entry.
fn stamp_with_note(explicit_popup: bool) -> Vec<u8> {
    let appearance = "1 0 0 rg 0 0 20 20 re f";
    let mut out = String::from("%PDF-1.7\n");
    let mut offsets = vec![0usize];
    let annots = if explicit_popup {
        "[5 0 R 7 0 R]"
    } else {
        "[5 0 R]"
    };
    let popup_key = if explicit_popup { "/Popup 7 0 R" } else { "" };
    let mut objects = vec![
        "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Resources << >> /Contents 4 0 R /Annots {annots} >>"
        ),
        "<< /Length 0 >>\nstream\n\nendstream".to_owned(),
        format!(
            "<< /Type /Annot /Subtype /Stamp /Rect [10 80 30 100] /F 4 /Name /Approved /Contents (associated stamp note) /AP << /N 6 0 R >> {popup_key} >>"
        ),
        format!(
            "<< /Type /XObject /Subtype /Form /BBox [0 0 20 20] /Resources << >> \
             /Length {} >>\nstream\n{}\nendstream",
            appearance.len(),
            appearance
        ),
    ];
    if explicit_popup {
        objects.push(
            "<< /Type /Annot /Subtype /Popup /Rect [30 20 90 70] /Parent 5 0 R /Open false >>"
                .to_owned(),
        );
    }
    for (index, body) in objects.iter().enumerate() {
        offsets.push(out.len());
        writeln!(out, "{} 0 obj\n{}\nendobj", index + 1, body).expect("String write");
    }
    let xref = out.len();
    writeln!(out, "xref\n0 {}", offsets.len()).expect("String write");
    out.push_str("0000000000 65535 f \n");
    for offset in offsets.iter().skip(1) {
        writeln!(out, "{offset:010} 00000 n ").expect("String write");
    }
    write!(
        out,
        "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
        offsets.len()
    )
    .expect("String write");
    out.into_bytes()
}

fn click_stamp(explicit_popup: bool) -> Viewer {
    let bytes = stamp_with_note(explicit_popup);
    let document = pdf_syntax::Document::open(bytes.clone()).expect("complete PDF fixture");
    assert!(!document.was_recovered(), "exact cross references");
    let page = pdf_model::Pages::new(&document).get(0).expect("one page");
    let interpreted = pdf_model::interpret(&document, &page);
    assert!(interpreted.is_complete(), "stored AP is supported");
    assert!(
        !interpreted.display_list.commands().is_empty(),
        "the stamp has visible artwork"
    );
    let mut viewer = Viewer::new(100, 100, 1.0);
    viewer
        .handle(Command::Open {
            id: DOCUMENT,
            bytes: bytes.into(),
            password: None,
            fragment: None,
        })
        .for_each(drop);

    // The stored AP is ordinary visible stamp artwork; its Popup starts closed where one is given.
    assert!(matches!(viewer.query(Query::Popups), Answer::Popups(windows) if windows.is_empty()));

    let Answer::Geometry(geometry) = viewer.query(Query::PageGeometry(0)) else {
        panic!("the stamp's page is on screen");
    };
    // Center of Rect [10, 80, 30, 100] mapped through the reported page placement.
    let at = (
        geometry.origin.0 + 20.0 * geometry.scale,
        geometry.origin.1 + (100.0 - 90.0) * geometry.scale,
    );
    for action in [PointerAction::Pressed, PointerAction::Released] {
        viewer
            .handle(Command::Pointer { at, action })
            .for_each(drop);
    }
    viewer
}

#[test]
fn an_explicit_stamp_popup_is_the_positive_control() {
    let viewer = click_stamp(true);
    let Answer::Popups(windows) = viewer.query(Query::Popups) else {
        panic!("popup query returns the explicitly associated stamp window");
    };
    assert_eq!(
        windows.len(),
        1,
        "opening the stamp opens its explicit window"
    );
    assert_eq!(windows[0].text.as_deref(), Some("associated stamp note"));
}

#[test]
fn opening_a_stamp_without_a_popup_reference_still_displays_its_associated_note() {
    // §12.5.1 defines clicking as exhibiting the associated object; §12.5.6.12 requires
    // the associated note popup on opening. Table172 makes Popup optional, so its omission
    // does not remove the provided Contents note. No artwork construction is involved.
    let viewer = click_stamp(false);
    let Answer::Popups(windows) = viewer.query(Query::Popups) else {
        panic!("popup query returns the opened stamp note");
    };
    assert_eq!(
        windows.len(),
        1,
        "the clause says opening a stamp displays a popup"
    );
    assert_eq!(windows[0].text.as_deref(), Some("associated stamp note"));
}

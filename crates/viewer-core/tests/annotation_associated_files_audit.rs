//! ISO 32000-2 §14.13.9 annotation-associated files through actual Viewer pointer handling.
//!
//! This is deliberately a `/Square`, not `/FileAttachment`: the annotation has no `/FS`
//! entry, so extraction can only come from the annotation's `/AF`. The page-carrier control
//! distinguishes reading the selected annotation from searching unrelated dictionary carriers.

#![expect(
    clippy::expect_used,
    clippy::panic,
    clippy::arithmetic_side_effects,
    reason = "tiny fixture offsets and pointer coordinates; failures must not silently skip a control"
)]

use viewer_core::{Answer, Command, DocumentId, Event, PointerAction, Query, Viewer};

const DOCUMENT: DocumentId = DocumentId(1);

fn annotation_af_pdf(annotation_af: bool, page_af: bool) -> Vec<u8> {
    use std::fmt::Write as _;
    const AP: &str = "0 0 1 rg 0 0 60 60 re f";
    const PAYLOAD: &str = "annotation-associated payload";
    let length = PAYLOAD.len();
    let params = format!("<< /Size {length} /ModDate (D:20240102030405Z) >>");
    let file_spec = "<< /Type /Filespec /F (annotation.txt) /UF (annotation.txt) \
         /AFRelationship /Data /EF << /F 7 0 R >> >>";
    let page_files = if page_af { " /AF [6 0 R]" } else { "" };
    let annotation_files = if annotation_af { " /AF [6 0 R]" } else { "" };
    let body = format!(
        concat!(
            "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n",
            "2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n",
            "3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 100] ",
            "/Resources << >> /Contents 4 0 R /Annots [5 0 R]{page_files} >>\nendobj\n",
            "4 0 obj\n<< /Length 0 >>\nstream\n\nendstream\nendobj\n",
            "5 0 obj\n<< /Type /Annot /Subtype /Square /Rect [20 20 80 80] ",
            "/F 4{annotation_files} /AP << /N 8 0 R >> >>\nendobj\n",
            "6 0 obj\n{file_spec}\nendobj\n",
            "7 0 obj\n<< /Type /EmbeddedFile /Subtype /text#2Fplain ",
            "/Params {params} /Length {length} >>\nstream\n{PAYLOAD}\nendstream\nendobj\n",
            "8 0 obj\n<< /Type /XObject /Subtype /Form /BBox [0 0 60 60] /Resources << >> /Length {ap_length} >>\nstream\n{AP}\nendstream\nendobj\n"
        ),
        page_files = page_files,
        annotation_files = annotation_files,
        file_spec = file_spec,
        params = params,
        length = length,
        PAYLOAD = PAYLOAD,
        AP = AP,
        ap_length = AP.len()
    );

    let mut out = String::from("%PDF-2.0\n");
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
        "trailer\n<< /Size {size} /Root 1 0 R /ID [<00112233445566778899aabbccddeeff> <ffeeddccbbaa99887766554433221100>] >>\nstartxref\n{xref_at}\n%%EOF\n"
    );
    out.into_bytes()
}

fn click_square(viewer: &mut Viewer) -> Vec<Event> {
    let Answer::Geometry(geometry) = viewer.query(Query::PageGeometry(0)) else {
        panic!("the one-page document has page geometry");
    };
    let (x, y) = (
        geometry.origin.0 + 50.0 * geometry.scale,
        geometry.origin.1 + (100.0 - 50.0) * geometry.scale,
    );
    viewer
        .handle(Command::Pointer {
            at: (x, y),
            action: PointerAction::Pressed,
        })
        .for_each(drop);
    viewer
        .handle(Command::Pointer {
            at: (x, y),
            action: PointerAction::Released,
        })
        .collect()
}

fn open(bytes: Vec<u8>) -> Viewer {
    let document = pdf_syntax::Document::open(bytes.clone()).expect("fixture opens");
    assert!(!document.was_recovered(), "fixture has valid xref");
    let mut viewer = Viewer::new(400, 300, 1.0);
    viewer
        .handle(Command::Open {
            id: DOCUMENT,
            bytes: bytes.into(),
            password: None,
            fragment: None,
        })
        .for_each(drop);
    viewer
}

fn extracted(events: &[Event]) -> Vec<(String, Vec<u8>)> {
    events
        .iter()
        .filter_map(|event| match event {
            Event::Extracted { name, bytes, .. } => Some((name.clone(), bytes.clone())),
            _ => None,
        })
        .collect()
}

#[test]
fn clicking_annotation_reads_its_af_without_requiring_fileattachment_fs() {
    let mut viewer = open(annotation_af_pdf(true, false));
    let events = click_square(&mut viewer);
    assert_eq!(
        extracted(&events),
        vec![(
            "annotation.txt".to_owned(),
            b"annotation-associated payload".to_vec()
        )],
        "the Square has /AF and no /FS: {events:?}"
    );
}

#[test]
fn clicking_annotation_does_not_inherit_af_from_the_page() {
    let mut viewer = open(annotation_af_pdf(false, true));
    let events = click_square(&mut viewer);
    assert!(
        extracted(&events).is_empty(),
        "the selected annotation has no /AF or /FS; its page's /AF is unrelated: {events:?}"
    );
}

#[test]
fn clicking_annotation_without_af_or_fs_extracts_nothing() {
    let mut viewer = open(annotation_af_pdf(false, false));
    let events = click_square(&mut viewer);
    assert!(
        extracted(&events).is_empty(),
        "a Square with neither /AF nor /FS has no file to extract: {events:?}"
    );
}

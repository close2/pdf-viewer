//! Complete-PDF consumer tests for §12.5.6.2 markup reply imports and paragraph writing.

#![expect(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "fixed fixtures and exact object assertions fail loudly"
)]

use std::fmt::Write as _;

use pdf_model::{forms_data::FormsData, view::ViewState};
use pdf_syntax::{Document, Object, ObjectId};

/// Assemble complete, cross-reference-valid PDF bytes from object bodies numbered from one.
fn complete_pdf(objects: &[String]) -> Vec<u8> {
    let mut out = String::from("%PDF-1.7\n");
    let mut offsets = vec![0usize];
    for (index, object) in objects.iter().enumerate() {
        offsets.push(out.len());
        let _ = writeln!(out, "{} 0 obj\n{}\nendobj", index + 1, object);
    }
    let xref_at = out.len();
    let _ = writeln!(out, "xref\n0 {}", offsets.len());
    out.push_str("0000000000 65535 f \n");
    for offset in offsets.iter().skip(1) {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    let _ = write!(
        out,
        "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n",
        offsets.len()
    );
    out.into_bytes()
}

/// A valid two-page target. The named target note can be on either page, with an exact xref.
fn target_pdf(target_page: usize) -> Document {
    let page0_annots = if target_page == 0 { "[5 0 R]" } else { "[]" };
    let page1_annots = if target_page == 1 { "[5 0 R]" } else { "[]" };
    let objects = [
        "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
        "<< /Type /Pages /Kids [3 0 R 4 0 R] /Count 2 >>".to_owned(),
        format!("<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 100] /Resources << >> /Annots {page0_annots} >>"),
        format!("<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 100] /Resources << >> /Annots {page1_annots} >>"),
        "<< /Type /Annot /Subtype /Text /Rect [10 10 30 30] /NM (target-note) /Contents (target) >>".to_owned(),
    ];
    open_exact(complete_pdf(&objects))
}

/// FDF cross-reference tables are optional under §12.7.8.1; keep the source PDF complete above.
fn fdf_with_reply(irt: &str) -> Document {
    let bytes = format!(
        "%FDF-1.2\n1 0 obj\n<< /FDF << /Annots [<< /Type /Annot /Subtype /Text /Page 0 /NM (reply-note) /Rect [40 10 60 30] /Contents (reply) /IRT {irt} /RT /R >>] >> >>\nendobj\ntrailer\n<< /Root 1 0 R >>\n%%EOF\n"
    );
    Document::open(bytes.into_bytes()).expect("valid FDF file; xref is optional")
}

fn open_exact(bytes: Vec<u8>) -> Document {
    let document = Document::open(bytes).expect("complete PDF parses");
    assert!(!document.was_recovered(), "exact cross references");
    document
}

fn imported_reply(document: &Document) -> (ObjectId, pdf_syntax::Dictionary) {
    let page_object = document.get(ObjectId::new(3, 0));
    let page = page_object.as_dict().expect("page zero");
    document
        .get_key(page, "Annots")
        .as_array()
        .expect("page annotations")
        .iter()
        .filter_map(Object::as_reference)
        .find_map(|id| {
            let object = document.get(id);
            let dict = object.as_dict()?;
            (document.get_key(dict, "NM").as_string() == Some(b"reply-note"))
                .then(|| (id, dict.clone()))
        })
        .expect("the imported reply is on page zero")
}

#[test]
fn fdf_irt_nm_resolves_to_the_named_annotation_on_the_same_page() {
    let target = target_pdf(0);
    let fdf = fdf_with_reply("(target-note)");
    let data = FormsData::read(&fdf).expect("FDF contents");
    let mut view = ViewState::of(&target);
    let outcome = view.import(&target, &data);
    assert_eq!(outcome.annotations, 1);
    assert!(outcome.refused.is_empty(), "{:?}", outcome.refused);

    let saved = view
        .save(&target)
        .expect("write import as incremental update");
    let reopened = open_exact(saved.bytes);
    let (_, reply) = imported_reply(&reopened);
    assert_eq!(
        reply.get("IRT").and_then(Object::as_reference),
        Some(ObjectId::new(5, 0)),
        "the FDF NM string becomes a PDF reference to the exact target"
    );
    assert_eq!(
        reopened
            .get_key(&reply, "RT")
            .as_name()
            .map(pdf_syntax::Name::as_bytes),
        Some(&b"R"[..])
    );
}

#[test]
fn fdf_irt_missing_name_and_cross_page_name_are_refused_with_rt_removed() {
    for (target_page, name, why) in [
        (0, "missing-note", "holds nowhere"),
        (1, "target-note", "this page holds nowhere"),
    ] {
        let target = target_pdf(target_page);
        let fdf = fdf_with_reply(&format!("({name})"));
        let data = FormsData::read(&fdf).expect("FDF contents");
        let mut view = ViewState::of(&target);
        let outcome = view.import(&target, &data);
        assert_eq!(outcome.annotations, 1);
        assert_eq!(outcome.refused.len(), 1, "{outcome:?}");
        assert!(outcome.refused[0].contains(why), "{:?}", outcome.refused);

        let saved = view
            .save(&target)
            .expect("write import as incremental update");
        let reopened = open_exact(saved.bytes);
        let (_, reply) = imported_reply(&reopened);
        assert!(reply.get("IRT").is_none(), "unresolved relation is omitted");
        assert!(reply.get("RT").is_none(), "RT is meaningful only with IRT");
    }
}

/// One-page PDF with a Text note; complete xref and required page Resources are present.
fn note_pdf() -> Document {
    let objects = [
        "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 100] /Resources << >> /Annots [4 0 R] >>"
            .to_owned(),
        "<< /Type /Annot /Subtype /Text /Rect [10 10 30 30] /Contents (before) >>".to_owned(),
    ];
    open_exact(complete_pdf(&objects))
}

fn saved_note_contents(document: &Document) -> String {
    let note_object = document.get(ObjectId::new(4, 0));
    let note = note_object.as_dict().expect("note annotation");
    let contents = document.get_key(note, "Contents");
    pdf_syntax::text_string(contents.as_string().expect("Contents string"))
}

#[test]
fn writer_uses_cr_for_contents_paragraphs() {
    let document = note_pdf();
    let mut view = ViewState::of(&document);
    assert!(view.set_note_text(&document, ObjectId::new(4, 0), "first\nsecond"));
    let written = view.save(&document).expect("incremental note update");
    let reopened = open_exact(written.bytes);
    assert_eq!(
        saved_note_contents(&reopened),
        "first\rsecond",
        "EC3 #90 scopes the CR instruction to a producer providing Contents"
    );
}

#[test]
fn writer_preserves_single_line_note_text() {
    let document = note_pdf();
    let mut view = ViewState::of(&document);
    assert!(view.set_note_text(&document, ObjectId::new(4, 0), "single line"));
    let written = view.save(&document).expect("incremental note update");
    let reopened = open_exact(written.bytes);
    assert_eq!(saved_note_contents(&reopened), "single line");
}

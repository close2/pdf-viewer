//! Annex Q detection regressions. These call the public archive `Survey` API with exact-xref files.

#![expect(clippy::expect_used, reason = "standalone exact-xref fixture")]

use std::fmt::Write as _;

use pdf_archive::survey::Survey;
use pdf_syntax::Document;

fn document(catalog_tail: &str, page_tail: &str, content: &str, extra: &[String]) -> Document {
    let stream = format!(
        "<< /Length {} >>\nstream\n{content}\nendstream",
        content.len().saturating_add(1)
    );
    let mut objects = vec![
        format!("<< /Type /Catalog /Pages 2 0 R {catalog_tail} >>"),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] /Contents 4 0 R \
             /Resources << >> {page_tail} >>"
        ),
        stream,
    ];
    objects.extend_from_slice(extra);

    let mut bytes = String::from("%PDF-2.0\n");
    let mut offsets = Vec::new();
    for (index, body) in objects.iter().enumerate() {
        offsets.push(bytes.len());
        let _ = writeln!(bytes, "{} 0 obj\n{body}\nendobj", index.saturating_add(1));
    }
    let xref = bytes.len();
    let size = objects.len().saturating_add(1);
    let _ = write!(bytes, "xref\n0 {size}\n0000000000 65535 f \n");
    for offset in offsets {
        let _ = writeln!(bytes, "{offset:010} 00000 n ");
    }
    let _ = write!(
        bytes,
        "trailer\n<< /Size {size} /Root 1 0 R /ID [<00112233445566778899aabbccddeeff> <ffeeddccbbaa99887766554433221100>] >>\nstartxref\n{xref}\n%%EOF\n"
    );
    let document = Document::open(bytes.into_bytes()).expect("the exact-xref PDF opens");
    assert!(
        !document.was_recovered(),
        "the fixture xref is authoritative"
    );
    document
}

fn widget_annotation(entries: &str) -> (String, String) {
    let field = format!(
        "<< /Type /Annot /Subtype /Widget /FT /Btn /Ff 65536 /T (push) \
         /P 3 0 R /Rect [0 0 10 10] {entries} /AP << /N 6 0 R >> >>"
    );
    let marks = "0 0 10 10 re f";
    let appearance = format!(
        "<< /Type /XObject /Subtype /Form /BBox [0 0 10 10] /Resources << >> \
         /Length {} >>\nstream\n{marks}\nendstream",
        marks.len().saturating_add(1)
    );
    (field, appearance)
}

/// Table 166 says annotation CA/ca are for appearance regeneration and “shall not be used” when
/// an appearance stream exists. Issue #51 separately adds annotation BM to Annex Q. A present,
/// opaque pushbutton appearance must therefore remain opaque under CA alone.
#[test]
fn annotation_ca_does_not_override_an_explicit_opaque_appearance() {
    for (entries, expected) in [("", false), ("/CA 1", false), ("/BM /Multiply", true)] {
        let (widget, appearance) = widget_annotation(entries);
        let control = document(
            "/AcroForm << /Fields [5 0 R] /NeedAppearances false >>",
            "/Annots [5 0 R]",
            "",
            &[widget, appearance],
        );
        let survey = Survey::of(&control);
        assert!(!survey.stopped_short());
        assert_eq!(survey.page_is_transparent(0), expected, "control {entries}");
    }
    let (widget, appearance) = widget_annotation("/CA 0.5");
    let doc = document(
        "/AcroForm << /Fields [5 0 R] /NeedAppearances false >>",
        "/Annots [5 0 R]",
        "",
        &[widget, appearance],
    );
    let survey = Survey::of(&doc);
    assert!(!survey.stopped_short());
    assert!(
        !survey.page_is_transparent(0),
        "the explicit AP controls its transparency; annotation CA is not used"
    );
}

/// The same annotation key that does affect painting: EC3 Issue #51 says a non-Normal annotation
/// BM is transparency even with an appearance stream. This is the independent positive control.
#[test]
fn annotation_bm_still_marks_an_explicit_appearance_transparent() {
    let (widget, appearance) = widget_annotation("/BM /Multiply");
    let doc = document(
        "/AcroForm << /Fields [5 0 R] /NeedAppearances false >>",
        "/Annots [5 0 R]",
        "",
        &[widget, appearance],
    );
    assert!(Survey::of(&doc).page_is_transparent(0));
}

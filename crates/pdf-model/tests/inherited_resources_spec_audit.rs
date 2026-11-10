//! ISO 32000-2 §7.7.3.4: inherited composite values are used as-is.
//!
//! A page with no `/Resources` inherits the nearest dictionary in its page-tree ancestry.
//! Even though `/Resources` is composite, dictionaries at different levels are not merged.
//! This checks the parsed page model directly; it does not claim that a named resource was
//! exercised by the content interpreter or rendered.

#![expect(
    clippy::float_cmp,
    reason = "integer fixture coordinates are exactly representable and undergo no arithmetic"
)]

use std::fmt::Write as _;

use pdf_syntax::Document;

/// A complete PDF with a root and intermediate page-tree node, each stating different resources.
fn inherited_resources_pdf(local_resources: bool) -> Vec<u8> {
    let body = concat!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n",
        "2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 ",
        "/MediaBox [0 0 200 100] ",
        "/Resources << /Font << /F1 5 0 R >> >> >>\nendobj\n",
        "3 0 obj\n<< /Type /Pages /Parent 2 0 R /Kids [4 0 R] /Count 1 ",
        "/Resources << /ExtGState << /GS1 6 0 R >> >> >>\nendobj\n",
        "4 0 obj\n<< /Type /Page /Parent 3 0 R >>\nendobj\n",
        "5 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>\nendobj\n",
        "6 0 obj\n<< /Type /ExtGState /ca 0.5 >>\nendobj\n",
    );
    let body = if local_resources {
        body.replace(
            "/Type /Page /Parent 3 0 R",
            "/Type /Page /Parent 3 0 R /Resources << >>",
        )
    } else {
        body.to_owned()
    };
    let mut bytes = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for object in body.split_inclusive("endobj\n") {
        offsets.push(bytes.len());
        bytes.push_str(object);
    }

    let xref_at = bytes.len();
    let size = offsets.len().saturating_add(1);
    let _ = writeln!(bytes, "xref\n0 {size}");
    bytes.push_str("0000000000 65535 f \n");
    for offset in &offsets {
        let _ = writeln!(bytes, "{offset:010} 00000 n ");
    }
    let _ = write!(
        bytes,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n"
    );
    bytes.into_bytes()
}

#[test]
fn inherited_resources_use_the_nearest_dictionary_without_merging() {
    let document = Document::open(inherited_resources_pdf(false)).expect("complete fixture opens");
    assert!(
        !document.was_recovered(),
        "fixture uses a complete xref table"
    );

    let pages = pdf_model::Pages::new(&document);
    assert_eq!(pages.len(), 1);
    let page = pages.get(0).expect("one page is present in the tree");
    assert_eq!(page.media_box, [0.0, 0.0, 200.0, 100.0]);

    assert!(
        page.resources.get("ExtGState").is_some(),
        "the nearer /Resources on the intermediate Pages node is inherited"
    );
    assert!(
        page.resources.get("Font").is_none(),
        "the ancestor's /Font entry is not merged into the nearer dictionary"
    );
}

/// A present empty dictionary stops inheritance just as a populated dictionary does.
#[test]
fn an_explicit_empty_page_resource_dictionary_overrides_ancestors() {
    let document = Document::open(inherited_resources_pdf(true)).expect("complete fixture opens");
    assert!(!document.was_recovered());
    let pages = pdf_model::Pages::new(&document);
    assert_eq!(pages.len(), 1);
    let page = pages.get(0).expect("one page");
    assert_eq!(page.media_box, [0.0, 0.0, 200.0, 100.0]);
    assert!(
        page.resources.is_empty(),
        "present empty dictionary overrides both ancestor dictionaries"
    );
}

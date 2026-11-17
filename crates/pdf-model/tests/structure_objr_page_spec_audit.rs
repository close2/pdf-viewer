//! Regression for effective ISO 32000-2 Table 358 `/Pg` precedence.
//!
//! Table 358 says a present OBJR `/Pg` overrides the containing structure element's `/Pg` and
//! is required if the element has no `/Pg`. The existing mixed-content fixture omits child Pg
//! and tests inheritance, so it cannot catch a reader that always inherits. `Tree::logical_order` is the model
//! consumer: the page association controls which page's logical content includes the OBJR.
//!

#![expect(
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    reason = "test fixture assertions and exact xref offsets"
)]

use std::fmt::Write as _;

use pdf_model::structure::{Child, Tree};
use pdf_syntax::{Document, ObjectId};

const PAGE_1: ObjectId = ObjectId::new(3, 0);
const PAGE_2: ObjectId = ObjectId::new(4, 0);
const ANNOTATION: ObjectId = ObjectId::new(8, 0);

/// Make a two-page tagged PDF with one annotation and a single OBJR.
///
/// `annotation_page` states the page's `/Annots` and the annotation's `/P`.
/// `element_page` supplies the containing structure element's `/Pg` when present.
/// `objr_page` supplies the OBJR's own `/Pg` when present.
fn fixture(annotation_page: u8, element_page: Option<u8>, objr_page: Option<u8>) -> Vec<u8> {
    assert!(annotation_page == 3 || annotation_page == 4);
    let page1_annots = if annotation_page == 3 {
        "[8 0 R]"
    } else {
        "[]"
    };
    let page2_annots = if annotation_page == 4 {
        "[8 0 R]"
    } else {
        "[]"
    };
    let element_pg = element_page
        .map(|page| format!("/Pg {page} 0 R "))
        .unwrap_or_default();
    let objr_pg = objr_page
        .map(|page| format!("/Pg {page} 0 R "))
        .unwrap_or_default();
    let annotation = format!(
        "<< /Type /Annot /Subtype /Text /P {annotation_page} 0 R /F 4 \
         /Rect [10 10 20 20] /Contents (tagged note) /StructParent 0 \
         /AP << /N 9 0 R >> >>"
    );
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R /MarkInfo << /Marked true >> \
         /StructTreeRoot 5 0 R >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R 4 0 R] /Count 2 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] \
         /Resources << >> /Annots {page1_annots} >>\nendobj\n\
         4 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] \
         /Resources << >> /Annots {page2_annots} >>\nendobj\n\
         5 0 obj\n<< /Type /StructTreeRoot /K [10 0 R] /ParentTree 7 0 R \
         /ParentTreeNextKey 1 >>\nendobj\n\
         6 0 obj\n<< /Type /StructElem /S /Annot /P 10 0 R {element_pg} \
         /K << /Type /OBJR /Obj 8 0 R {objr_pg} >> >>\nendobj\n\
         7 0 obj\n<< /Nums [0 6 0 R] >>\nendobj\n\
         8 0 obj\n{annotation}\nendobj\n\
         9 0 obj\n<< /Type /XObject /Subtype /Form /FormType 1 /BBox [0 0 10 10] \
         /Resources << >> /Length 0 >>\nstream\nendstream\nendobj\n\
         10 0 obj\n<< /Type /StructElem /S /Document /P 5 0 R /K [6 0 R] >>\nendobj\n"
    );

    let mut bytes = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for object in body.split_inclusive("endobj\n") {
        offsets.push(bytes.len());
        bytes.push_str(object);
    }
    let startxref = bytes.len();
    let size = offsets.len() + 1;
    let _ = writeln!(bytes, "xref\n0 {size}");
    bytes.push_str("0000000000 65535 f \n");
    for offset in offsets {
        let _ = writeln!(bytes, "{offset:010} 00000 n ");
    }
    let _ = write!(
        bytes,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{startxref}\n%%EOF\n"
    );
    bytes.into_bytes()
}

fn object_item(document: &Document, page: ObjectId) -> Vec<Child> {
    let tree = Tree::of(document).expect("the catalog declares the structure tree");
    let result = tree.logical_order(document, page);
    assert!(
        !result.truncated,
        "the one-element structure tree is complete"
    );
    result.items
}

#[test]
fn objr_page_overrides_element_page_with_inheritance_and_requiredness_controls() {
    // The element says page 1, but the annotation and explicit OBJR `/Pg` say page 2.
    let override_doc =
        Document::open(fixture(4, Some(3), Some(4))).expect("exact-xref override fixture opens");
    assert!(!override_doc.was_recovered(), "the xref is exact");
    assert!(
        !override_doc.cross_reference_chain_was_cut(),
        "complete xref chain"
    );
    assert!(object_item(&override_doc, PAGE_1).is_empty());
    assert_eq!(
        object_item(&override_doc, PAGE_2),
        vec![Child::Object {
            object: ANNOTATION,
            page: Some(PAGE_2),
        }],
        "the Table 358 item page takes precedence over the enclosing element page"
    );

    // When the entry is omitted, the element page is the stated page.
    let inherited_doc =
        Document::open(fixture(3, Some(3), None)).expect("exact-xref inheritance control opens");
    assert!(!inherited_doc.was_recovered(), "the control xref is exact");
    assert!(
        !inherited_doc.cross_reference_chain_was_cut(),
        "complete xref chain"
    );
    assert_eq!(
        object_item(&inherited_doc, PAGE_1),
        vec![Child::Object {
            object: ANNOTATION,
            page: Some(PAGE_1),
        }]
    );
    assert!(object_item(&inherited_doc, PAGE_2).is_empty());

    // Table 358 requires an OBJR `/Pg` if the containing element has none.
    let required_doc =
        Document::open(fixture(4, None, Some(4))).expect("exact-xref required-page control opens");
    assert!(!required_doc.was_recovered(), "the control xref is exact");
    assert!(
        !required_doc.cross_reference_chain_was_cut(),
        "complete xref chain"
    );
    assert!(object_item(&required_doc, PAGE_1).is_empty());
    assert_eq!(
        object_item(&required_doc, PAGE_2),
        vec![Child::Object {
            object: ANNOTATION,
            page: Some(PAGE_2),
        }]
    );
}

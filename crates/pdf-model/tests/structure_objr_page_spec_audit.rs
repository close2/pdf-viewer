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

/// Make a two-page tagged PDF with a target OBJR and a parent-page sibling when needed.
///
/// `annotation_page` states the page's `/Annots` and the annotation's `/P`.
/// `element_page` supplies the containing structure element's `/Pg` when present.
/// `objr_page` supplies the OBJR's own `/Pg` when present.
fn fixture(annotation_page: u8, element_page: Option<u8>, objr_page: Option<u8>) -> Vec<u8> {
    assert!(annotation_page == 3 || annotation_page == 4);
    // Table 355 says the element /Pg identifies a page where some or all K content is
    // rendered. If the principal OBJR overrides that page, add a second genuine annotation
    // and OBJR on the element page instead of contradicting the containing element's /Pg.
    let parent_sibling = element_page.is_some_and(|page| page != annotation_page);
    let mut page1_items = Vec::new();
    let mut page2_items = Vec::new();
    if annotation_page == 3 {
        page1_items.push("8 0 R");
    } else {
        page2_items.push("8 0 R");
    }
    if parent_sibling {
        if element_page == Some(3) {
            page1_items.push("11 0 R");
        } else {
            page2_items.push("11 0 R");
        }
    }
    let page1_annots = format!("[{}]", page1_items.join(" "));
    let page2_annots = format!("[{}]", page2_items.join(" "));
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
    let sibling_objr = if parent_sibling {
        format!(
            " << /Type /OBJR /Obj 11 0 R /Pg {} 0 R >>",
            element_page.expect("sibling needs parent page")
        )
    } else {
        String::new()
    };
    let element_k = if parent_sibling {
        format!("/K [<< /Type /OBJR /Obj 8 0 R {objr_pg} >>{sibling_objr}]")
    } else {
        format!("/K << /Type /OBJR /Obj 8 0 R {objr_pg} >>")
    };
    let parent_tree = if parent_sibling {
        "/Nums [0 6 0 R 1 6 0 R]"
    } else {
        "/Nums [0 6 0 R]"
    };
    let parent_tree_next_key = if parent_sibling { 2 } else { 1 };
    let sibling_annotation = if parent_sibling {
        format!(
            "11 0 obj\n<< /Type /Annot /Subtype /Text /P {} 0 R /F 4 /Rect [30 10 40 20] /Contents (parent-page note) /StructParent 1 /AP << /N 9 0 R >> >>\nendobj\n",
            element_page.expect("sibling needs parent page")
        )
    } else {
        String::new()
    };
    let appearance = "1 0 0 rg 0 0 10 10 re f\n";
    let appearance_length = appearance.len();
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R /MarkInfo << /Marked true >> \
         /StructTreeRoot 5 0 R >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R 4 0 R] /Count 2 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] \
         /Resources << >> /Annots {page1_annots} >>\nendobj\n\
         4 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] \
         /Resources << >> /Annots {page2_annots} >>\nendobj\n\
         5 0 obj\n<< /Type /StructTreeRoot /K [10 0 R] /ParentTree 7 0 R \
         /ParentTreeNextKey {parent_tree_next_key} >>\nendobj\n\
         6 0 obj\n<< /Type /StructElem /S /Annot /P 10 0 R {element_pg} \
         {element_k} >>\nendobj\n\
         7 0 obj\n<< {parent_tree} >>\nendobj\n\
         8 0 obj\n{annotation}\nendobj\n\
         9 0 obj\n<< /Type /XObject /Subtype /Form /FormType 1 /BBox [0 0 10 10] \
         /Resources << >> /Length {appearance_length} >>\nstream\n{appearance}endstream\nendobj\n\
         10 0 obj\n<< /Type /StructElem /S /Document /P 5 0 R /K [6 0 R] >>\nendobj\n{sibling_annotation}"
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
    // The element owns annotations on both pages; the target OBJR overrides its `/Pg`.
    let override_doc =
        Document::open(fixture(4, Some(3), Some(4))).expect("exact-xref override fixture opens");
    assert!(!override_doc.was_recovered(), "the xref is exact");
    assert!(
        !override_doc.cross_reference_chain_was_cut(),
        "complete xref chain"
    );
    assert_eq!(
        object_item(&override_doc, PAGE_1),
        vec![Child::Object {
            object: ObjectId::new(11, 0),
            page: Some(PAGE_1),
        }],
        "the containing element /Pg has its own page-local object reference"
    );
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

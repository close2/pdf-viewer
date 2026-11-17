//! Regression for effective ISO 32000-2 Table 357 `/Pg` precedence.
//!
//! The MCR page must override the containing element's page. When MCR `/Pg` is absent, the
//! sequence belongs to the element's stated page; when the element has no `/Pg`, an MCR `/Pg`
//! is required. This uses a real page-content MCID and `ParentTree` entry, with no font dependency.
//!

#![expect(
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    reason = "test fixture assertions and exact xref offsets"
)]

use std::fmt::Write as _;

use pdf_model::content::ContentStream;
use pdf_model::structure::{Child, Tree};
use pdf_syntax::{Document, ObjectId};

const PAGE_1: ObjectId = ObjectId::new(3, 0);
const PAGE_2: ObjectId = ObjectId::new(4, 0);

const CONTENT: &str = "/Figure << /MCID 0 >> BDC\n\
    1 0 0 rg 10 10 20 20 re f\n\
    EMC\n";

/// A two-page tagged PDF with MCID 0 on the target page and a parent-page sibling when needed.
///
/// `sequence_page` is where the marked sequence and its `ParentTree` key reside.
/// `element_page` is the containing Figure's `/Pg`, if stated.
/// `mcr_page` is the MCR dictionary's `/Pg`, if stated.
fn fixture(sequence_page: u8, element_page: Option<u8>, mcr_page: Option<u8>) -> Vec<u8> {
    assert!(sequence_page == 3 || sequence_page == 4);
    let page1_sequence = sequence_page == 3;
    // Table 355 says the element's /Pg represents a page on which some or all K content
    // is rendered. When the primary MCR overrides that page, add a second real page-local
    // MCID under the same Figure so the fixture still satisfies the parent's /Pg claim.
    let parent_sibling = element_page.is_some_and(|page| page != sequence_page);
    let (page1_contents, page2_contents) = if page1_sequence {
        (
            "/Contents 8 0 R /StructParents 0".to_owned(),
            if parent_sibling {
                "/Contents 10 0 R /StructParents 1".to_owned()
            } else {
                String::new()
            },
        )
    } else {
        (
            if parent_sibling {
                "/Contents 10 0 R /StructParents 1".to_owned()
            } else {
                String::new()
            },
            "/Contents 8 0 R /StructParents 0".to_owned(),
        )
    };
    let element_pg = element_page
        .map(|page| format!("/Pg {page} 0 R "))
        .unwrap_or_default();
    let mcr_pg = mcr_page
        .map(|page| format!("/Pg {page} 0 R "))
        .unwrap_or_default();
    let parent_mcr = if parent_sibling {
        format!(
            " << /Type /MCR /Pg {} 0 R /MCID 1 >>",
            element_page.expect("sibling needs parent page")
        )
    } else {
        String::new()
    };
    let figure_k = if parent_sibling {
        format!("/K [<< /Type /MCR {mcr_pg}/MCID 0 >>{parent_mcr}]")
    } else {
        format!("/K << /Type /MCR {mcr_pg}/MCID 0 >>")
    };
    let parent_tree = if parent_sibling {
        "/Nums [0 [7 0 R] 1 [null 7 0 R]]"
    } else {
        "/Nums [0 [7 0 R]]"
    };
    let parent_tree_next_key = if parent_sibling { 2 } else { 1 };
    let sibling_stream = if parent_sibling {
        format!(
            "10 0 obj\n<< /Length {} >>\nstream\n/Figure << /MCID 1 >> BDC\n1 0 0 rg 40 40 20 20 re f\nEMC\nendstream\nendobj\n",
            "/Figure << /MCID 1 >> BDC\n1 0 0 rg 40 40 20 20 re f\nEMC\n".len()
        )
    } else {
        String::new()
    };
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R /MarkInfo << /Marked true >> \
         /StructTreeRoot 5 0 R >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R 4 0 R] /Count 2 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] \
         /Resources << >> {page1_contents} >>\nendobj\n\
         4 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] \
         /Resources << >> {page2_contents} >>\nendobj\n\
         5 0 obj\n<< /Type /StructTreeRoot /K [6 0 R] /ParentTree 9 0 R \
         /ParentTreeNextKey {parent_tree_next_key} >>\nendobj\n\
         6 0 obj\n<< /Type /StructElem /S /Document /P 5 0 R /K [7 0 R] >>\nendobj\n\
         7 0 obj\n<< /Type /StructElem /S /Figure /P 6 0 R /Alt (red square) {element_pg} \
         {figure_k} >>\nendobj\n\
         8 0 obj\n<< /Length {} >>\nstream\n{CONTENT}endstream\nendobj\n\
         9 0 obj\n<< {parent_tree} >>\nendobj\n{s}",
        CONTENT.len(),
        s = sibling_stream,
    );

    let mut bytes = String::from("%PDF-2.0\n");
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
        "trailer\n<< /Size {size} /Root 1 0 R \
         /ID [<00112233445566778899AABBCCDDEEFF> \
         <00112233445566778899AABBCCDDEEFF>] >>\nstartxref\n{startxref}\n%%EOF\n"
    );
    bytes.into_bytes()
}

fn sequence_item(document: &Document, page: ObjectId) -> Vec<Child> {
    let tree = Tree::of(document).expect("the catalog declares the structure tree");
    let result = tree.logical_order(document, page);
    assert!(!result.truncated, "the bounded structure tree is complete");
    result.items
}

fn assert_real_sequence(document: &Document, page_index: usize, mcid: i64) {
    let page = pdf_model::Pages::new(document)
        .get(page_index)
        .expect("the selected page exists");
    let interpretation = pdf_model::interpret(document, &page);
    assert!(
        interpretation.is_complete(),
        "the target page is completely interpreted"
    );
    assert_eq!(
        interpretation
            .marked
            .iter()
            .map(|span| (span.mcid, span.stream))
            .collect::<Vec<_>>(),
        vec![(mcid, ContentStream::Page)],
        "the selected page contains the fixture's actual page-stream MCID"
    );
}

#[test]
fn mcr_page_overrides_element_page_with_inheritance_and_requiredness_controls() {
    // The Figure includes content on both pages; its target MCR overrides the element page.
    let override_doc =
        Document::open(fixture(4, Some(3), Some(4))).expect("exact-xref override fixture opens");
    assert!(
        !override_doc.was_recovered(),
        "the xref and trailer are exact"
    );
    assert!(!override_doc.cross_reference_chain_was_cut());
    assert_real_sequence(&override_doc, 1, 0);
    assert_real_sequence(&override_doc, 0, 1);
    assert_eq!(
        sequence_item(&override_doc, PAGE_1),
        vec![Child::MarkedContent {
            mcid: 1,
            page: Some(PAGE_1),
            stream: None,
            owner: None,
        }],
        "the containing Figure /Pg has its own real page-local content item"
    );
    assert_eq!(
        sequence_item(&override_doc, PAGE_2),
        vec![Child::MarkedContent {
            mcid: 0,
            page: Some(PAGE_2),
            stream: None,
            owner: None,
        }],
        "Table 357 MCR /Pg takes precedence over its Figure's /Pg"
    );

    // With no MCR `/Pg`, the same sequence is on the Figure's stated page.
    let inherited_doc =
        Document::open(fixture(3, Some(3), None)).expect("exact-xref inheritance control opens");
    assert!(!inherited_doc.was_recovered(), "the control xref is exact");
    assert!(!inherited_doc.cross_reference_chain_was_cut());
    assert_real_sequence(&inherited_doc, 0, 0);
    assert_eq!(
        sequence_item(&inherited_doc, PAGE_1),
        vec![Child::MarkedContent {
            mcid: 0,
            page: Some(PAGE_1),
            stream: None,
            owner: None,
        }]
    );
    assert!(sequence_item(&inherited_doc, PAGE_2).is_empty());

    // Without an element `/Pg`, Table 357 requires the MCR `/Pg` that names the sequence page.
    let required_doc =
        Document::open(fixture(4, None, Some(4))).expect("exact-xref required-page control opens");
    assert!(!required_doc.was_recovered(), "the control xref is exact");
    assert!(!required_doc.cross_reference_chain_was_cut());
    assert_real_sequence(&required_doc, 1, 0);
    assert!(sequence_item(&required_doc, PAGE_1).is_empty());
    assert_eq!(
        sequence_item(&required_doc, PAGE_2),
        vec![Child::MarkedContent {
            mcid: 0,
            page: Some(PAGE_2),
            stream: None,
            owner: None,
        }]
    );
}

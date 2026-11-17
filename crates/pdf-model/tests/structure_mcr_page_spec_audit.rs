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

/// A two-page tagged PDF with MCID 0 in exactly one page's content stream.
///
/// `sequence_page` is where the marked sequence and its `ParentTree` key reside.
/// `element_page` is the containing Figure's `/Pg`, if stated.
/// `mcr_page` is the MCR dictionary's `/Pg`, if stated.
fn fixture(sequence_page: u8, element_page: Option<u8>, mcr_page: Option<u8>) -> Vec<u8> {
    assert!(sequence_page == 3 || sequence_page == 4);
    let page1_sequence = sequence_page == 3;
    let page1_contents = if page1_sequence {
        "/Contents 8 0 R /StructParents 0"
    } else {
        ""
    };
    let page2_contents = if page1_sequence {
        ""
    } else {
        "/Contents 8 0 R /StructParents 0"
    };
    let element_pg = element_page
        .map(|page| format!("/Pg {page} 0 R "))
        .unwrap_or_default();
    let mcr_pg = mcr_page
        .map(|page| format!("/Pg {page} 0 R "))
        .unwrap_or_default();
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R /MarkInfo << /Marked true >> \
         /StructTreeRoot 5 0 R >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R 4 0 R] /Count 2 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] \
         /Resources << >> {page1_contents} >>\nendobj\n\
         4 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] \
         /Resources << >> {page2_contents} >>\nendobj\n\
         5 0 obj\n<< /Type /StructTreeRoot /K [6 0 R] /ParentTree 9 0 R \
         /ParentTreeNextKey 1 >>\nendobj\n\
         6 0 obj\n<< /Type /StructElem /S /Document /P 5 0 R /K [7 0 R] >>\nendobj\n\
         7 0 obj\n<< /Type /StructElem /S /Figure /P 6 0 R /Alt (red square) {element_pg} \
         /K << /Type /MCR {mcr_pg}/MCID 0 >> >>\nendobj\n\
         8 0 obj\n<< /Length {} >>\nstream\n{CONTENT}endstream\nendobj\n\
         9 0 obj\n<< /Nums [0 [7 0 R]] >>\nendobj\n",
        CONTENT.len(),
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
    assert!(!result.truncated, "the one-item structure tree is complete");
    result.items
}

fn assert_real_sequence(document: &Document, page_index: usize) {
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
        vec![(0, ContentStream::Page)],
        "the selected page contains the fixture's actual page-stream MCID"
    );
}

#[test]
fn mcr_page_overrides_element_page_with_inheritance_and_requiredness_controls() {
    // The Figure says page 1, but the MCR and the actual MCID sequence say page 2.
    let override_doc =
        Document::open(fixture(4, Some(3), Some(4))).expect("exact-xref override fixture opens");
    assert!(
        !override_doc.was_recovered(),
        "the xref and trailer are exact"
    );
    assert!(!override_doc.cross_reference_chain_was_cut());
    assert_real_sequence(&override_doc, 1);
    assert!(sequence_item(&override_doc, PAGE_1).is_empty());
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
    assert_real_sequence(&inherited_doc, 0);
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
    assert_real_sequence(&required_doc, 1);
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

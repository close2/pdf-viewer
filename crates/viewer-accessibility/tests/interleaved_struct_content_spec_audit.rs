//! ISO 32000-2 §14.8.2.5.1: interleaved parent content and child elements retain logical order.
//! Exercises the public viewer query and AccessKit tree; it does not drive a platform bus.
#![expect(
    clippy::panic,
    clippy::expect_used,
    reason = "test fixtures must supply the requested public answer"
)]

use std::fmt::Write as _;

use accesskit::{NodeId, Role};
use viewer_accessibility::{DocumentView, PageView, tree};
use viewer_core::{Answer, Command, DocumentId, Query, Viewer};

fn interleaved_pdf(interleave_child: bool) -> Vec<u8> {
    let content = "\
        /Span << /MCID 0 >> BDC BT /F1 12 Tf 10 80 Td (A) Tj ET EMC\n\
        /Span << /MCID 1 >> BDC BT /F1 12 Tf 10 60 Td (B) Tj ET EMC\n\
        /Span << /MCID 2 >> BDC BT /F1 12 Tf 10 40 Td (C) Tj ET EMC\
    ";
    let page_kids = if interleave_child {
        "[<< /Type /MCR /Pg 3 0 R /MCID 0 >> 8 0 R << /Type /MCR /Pg 3 0 R /MCID 2 >>]"
    } else {
        "[<< /Type /MCR /Pg 3 0 R /MCID 0 >> << /Type /MCR /Pg 3 0 R /MCID 1 >> \
         << /Type /MCR /Pg 3 0 R /MCID 2 >>]"
    };
    let parent_tree = if interleave_child {
        "[7 0 R 8 0 R 7 0 R]"
    } else {
        "[7 0 R 7 0 R 7 0 R]"
    };
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R /StructTreeRoot 6 0 R \
         /MarkInfo << /Marked true >> >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] \
         /StructParents 0 /Resources << /Font << /F1 5 0 R >> >> /Contents 4 0 R >>\nendobj\n\
         4 0 obj\n<< /Length {} >>\nstream\n{content}\nendstream\nendobj\n\
         5 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>\nendobj\n\
         6 0 obj\n<< /Type /StructTreeRoot /K [7 0 R] /ParentTree 9 0 R /ParentTreeNextKey 1 >>\nendobj\n\
         7 0 obj\n<< /Type /StructElem /S /P /P 6 0 R /Pg 3 0 R /K {page_kids} >>\nendobj\n\
         8 0 obj\n<< /Type /StructElem /S /Span /P 7 0 R /Pg 3 0 R /K 1 >>\nendobj\n\
         9 0 obj\n<< /Nums [0 {parent_tree}] >>\nendobj\n",
        content.len().saturating_add(1),
    );

    let mut out = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for object in body.split_inclusive("endobj\n") {
        offsets.push(out.len());
        out.push_str(object);
    }
    let xref_at = out.len();
    let size = offsets.len().saturating_add(1);
    let _ = write!(out, "xref\n0 {size}\n0000000000 65535 f \n");
    for offset in offsets {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    let _ = write!(
        out,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n"
    );
    out.into_bytes()
}

fn text_runs(update: &accesskit::TreeUpdate, id: NodeId, out: &mut Vec<String>) {
    let current = update
        .nodes
        .iter()
        .find_map(|(node_id, node)| (*node_id == id).then_some(node))
        .expect("every named child is in this complete tree");
    if current.role() == Role::TextRun
        && let Some(value) = current.value()
    {
        out.push(value.to_owned());
    }
    for child in current.children() {
        text_runs(update, *child, out);
    }
}

fn accessibility_text(pdf: Vec<u8>) -> String {
    let mut viewer = Viewer::new(200, 200, 1.0);
    let _ = viewer
        .handle(Command::Open {
            id: DocumentId(1),
            bytes: pdf.into(),
            password: None,
            fragment: None,
        })
        .collect::<Vec<_>>();
    let Answer::Accessibility(pages) = viewer.query(Query::AccessibilityTree) else {
        panic!("the opened page has an accessibility answer");
    };
    let [structure] = pages.as_slice() else {
        panic!("one page is on screen");
    };
    let page = PageView {
        page: structure.page,
        label: None,
        bounds: [0.0, 0.0, 100.0, 100.0],
        nodes: &structure.nodes,
        widgets: &structure.widgets,
        tagging: structure.tagging,
        reports: &[],
        readback: pdf_model::content::Shortfall::default(),
    };
    let update = tree::build(&DocumentView {
        window: "window",
        document: "document",
        pages: 1,
        viewport: (200.0, 200.0),
        origin: (0.0, 0.0),
        shown: &[page],
    });
    let root = update.tree.as_ref().expect("complete tree").root;
    let mut pieces = Vec::new();
    text_runs(&update, root, &mut pieces);
    pieces.join("")
}

#[test]
fn accesskit_preserves_interleaving_of_parent_content_and_child_elements() {
    // Positive control: one element's MCR list is emitted in its own order.
    assert_eq!(accessibility_text(interleaved_pdf(false)), "ABC");

    // The stream order and logical `/K` order both say A, B, C. The ParentTree routes each
    // MCID to the element owning that MCR. This is a conforming mixed-content tree, not a
    // producer-invalid nesting probe.
    let pdf = interleaved_pdf(true);
    let document = pdf_syntax::Document::open(pdf.clone()).expect("valid exact-xref tagged PDF");
    assert!(!document.was_recovered());
    assert!(!document.cross_reference_chain_was_cut());
    let page = pdf_model::Pages::new(&document).get(0).expect("one page");
    let interpretation = pdf_model::interpret(&document, &page);
    assert!(interpretation.is_complete(), "supported text-only page");
    let structure = pdf_model::structure::Tree::of(&document).expect("tagged structure");
    let logical = structure
        .logical_text(&document, pdf_syntax::ObjectId::new(3, 0), &interpretation)
        .expect("the structure names all page text");
    assert_eq!(
        logical
            .chars()
            .filter(char::is_ascii_alphabetic)
            .collect::<String>(),
        "ABC"
    );

    assert_eq!(
        accessibility_text(pdf),
        "ABC",
        "depth-first accessibility traversal must retain /K order across parent MCR, child StructElem, and the next parent MCR"
    );
}

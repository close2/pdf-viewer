//! ISO 32000-2 Annex O: structure identifier hierarchy order and empty-element fallback.

#![expect(
    clippy::expect_used,
    clippy::panic,
    clippy::arithmetic_side_effects,
    reason = "small synthetic PDF fixtures and query helpers must fail loudly"
)]

use pdf_syntax::Document;
use std::fmt::Write as _;
use viewer_core::{Answer, Command, DocumentId, Query, Viewer};

fn assembled(objects: &[String]) -> Vec<u8> {
    let mut out = String::from("%PDF-2.0\n");
    let mut offsets = Vec::new();
    for (index, body) in objects.iter().enumerate() {
        offsets.push(out.len());
        let _ = write!(out, "{} 0 obj\n{body}\nendobj\n", index + 1);
    }
    let xref_at = out.len();
    let size = objects.len() + 1;
    let _ = write!(out, "xref\n0 {size}\n0000000000 65535 f \n");
    for offset in offsets {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    let _ = write!(
        out,
        "trailer\n<< /Size {size} /Root 1 0 R /ID [<00112233445566778899AABBCCDDEEFF> <00112233445566778899AABBCCDDEEFF>] >>\nstartxref\n{xref_at}\n%%EOF\n"
    );
    out.into_bytes()
}

fn structure_pdf(children: &str) -> Vec<u8> {
    let content = "/P << /MCID 0 >> BDC 0 0 10 10 re f EMC";
    let stream = format!(
        "<< /Length {} >>\nstream\n{content}\nendstream",
        content.len()
    );
    let mut objects: Vec<String> = [
        "<< /Type /Catalog /Pages 2 0 R /StructTreeRoot 5 0 R /MarkInfo << /Marked true >> >>",
        "<< /Type /Pages /Count 2 /Kids [3 0 R 4 0 R] >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Resources << >> /Contents 12 0 R /StructParents 0 >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Resources << >> /Contents 13 0 R /StructParents 1 >>",
        "<< /Type /StructTreeRoot /K [6 0 R] /IDTree 9 0 R /ParentTree 14 0 R >>",
        "", // Target's children are varied, keeping both actual content pages unchanged.
        "<< /Type /StructElem /S /P /P 6 0 R /K [10 0 R] >>",
        "<< /Type /StructElem /S /P /P 6 0 R /K [11 0 R] >>",
        "<< /Names [(Target) 6 0 R] >>",
        "<< /Type /MCR /Pg 4 0 R /MCID 0 >>",
        "<< /Type /MCR /Pg 3 0 R /MCID 0 >>",
    ].into_iter().map(str::to_owned).collect();
    *objects.get_mut(5).expect("target slot") =
        format!("<< /Type /StructElem /S /Document /P 5 0 R /ID (Target) /K [{children}] >>");
    objects.extend([
        stream.clone(),
        stream,
        "<< /Nums [0 [8 0 R] 1 [7 0 R]] >>".to_owned(),
    ]);
    assembled(&objects)
}

fn checked(bytes: Vec<u8>) -> Document {
    let document = Document::open(bytes).expect("a valid PDF");
    assert!(!document.was_recovered(), "xref is used without repair");
    for index in 0..2 {
        let page = pdf_model::Pages::new(&document)
            .get(index)
            .expect("two pages");
        assert!(pdf_model::interpret(&document, &page).is_complete());
    }
    document
}

#[test]
fn structure_id_uses_first_content_in_nested_hierarchy_order() {
    for (children, expected) in [("7 0 R 8 0 R", 1), ("8 0 R 7 0 R", 0)] {
        let bytes = structure_pdf(children);
        let doc = checked(bytes.clone());
        let tree = pdf_model::structure::Tree::of(&doc).expect("structure tree");
        let element = tree
            .element_by_id(&doc, b"Target")
            .expect("ID tree resolves Target");
        assert_eq!(
            opened_page(bytes, "structelem=Target"),
            expected,
            "fragment opens the first content page"
        );
        let pages = pdf_model::Pages::new(&doc);
        assert_eq!(
            pdf_model::destination::structure_element_page(&doc, &element, &pages),
            Some(expected)
        );
    }
}

fn empty_pdf() -> Vec<u8> {
    let objects = [
        "<< /Type /Catalog /Pages 2 0 R /StructTreeRoot 5 0 R >>",
        "<< /Type /Pages /Count 2 /Kids [3 0 R 4 0 R] >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Resources << >> >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Resources << >> >>",
        "<< /Type /StructTreeRoot /K [6 0 R] /IDTree 7 0 R >>",
        "<< /Type /StructElem /S /Document /P 5 0 R /ID (Empty) /K [] >>",
        "<< /Names [(Empty) 6 0 R] >>",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect::<Vec<_>>();
    assembled(&objects)
}

fn opened_page(bytes: Vec<u8>, fragment: &str) -> usize {
    let mut viewer = Viewer::new(800, 1000, 1.0);
    let _ = viewer
        .handle(Command::Open {
            id: DocumentId(1),
            bytes: bytes.into(),
            password: None,
            fragment: Some(fragment.to_owned()),
        })
        .collect::<Vec<_>>();
    let Answer::Page { index, .. } = viewer.query(Query::CurrentPage) else {
        panic!("an opened document has a current page");
    };
    index
}

#[test]
fn matching_empty_structure_id_falls_back_from_page_two_to_page_one() {
    let bytes = empty_pdf();
    let doc = checked(bytes.clone());
    let tree = pdf_model::structure::Tree::of(&doc).expect("tree exists");
    let element = tree
        .element_by_id(&doc, b"Empty")
        .expect("matching ID exists");
    assert_eq!(
        pdf_model::destination::structure_element_page(
            &doc,
            &element,
            &pdf_model::Pages::new(&doc)
        ),
        None
    );
    assert_eq!(
        opened_page(bytes.clone(), "page=2"),
        1,
        "positive control really opens page two"
    );
    assert_eq!(
        opened_page(bytes, "page=2&structelem=Empty"),
        0,
        "matching empty element selects page one"
    );
}

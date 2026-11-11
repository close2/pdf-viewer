//! ISO 32000-2 §7.7.2 Table 29 and §7.7.3.3 Table 31: generated Catalog and Page objects
//! state their required type and page-parent link.

#![expect(
    clippy::expect_used,
    reason = "test code: fixture and writer failures must fail loudly, and all mandatory output entries must be inspected"
)]

use std::fmt::Write as _;

use pdf_syntax::Document;
use pdf_transform::range::Selection;
use pdf_transform::split::{Pieces, SplitPlan};
use pdf_transform::{Budget, MemorySinks, Plan, Policy, Source, apply};

/// A complete, two-page source document with classic xref offsets.
fn two_page_pdf() -> Vec<u8> {
    let body = concat!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n",
        "2 0 obj\n<< /Type /Pages /Kids [3 0 R 5 0 R] /Count 2 >>\nendobj\n",
        "3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] ",
        "/Resources << >> /Contents 4 0 R >>\nendobj\n",
        "4 0 obj\n<< /Length 0 >>\nstream\n\nendstream\nendobj\n",
        "5 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 20 30] ",
        "/Resources << >> /Contents 4 0 R >>\nendobj\n",
    );
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
fn split_output_catalog_has_the_required_catalog_type() {
    let source_bytes = two_page_pdf();
    let source = Document::open(source_bytes.clone()).expect("valid source fixture opens");
    assert!(
        !source.was_recovered(),
        "source has its complete xref table"
    );
    assert_eq!(
        source
            .catalog()
            .expect("source trailer resolves its catalog")
            .get("Type")
            .and_then(pdf_syntax::Object::as_name)
            .map(pdf_syntax::Name::as_bytes),
        Some(&b"Catalog"[..]),
        "positive control: the source states the required Catalog type"
    );

    let sinks = MemorySinks::new();
    let report = apply(
        &Plan::Split(SplitPlan {
            source: 0,
            pages: "2"
                .parse::<Selection>()
                .expect("second-page selection parses"),
            pieces: Pieces::EachPage,
            names: "piece-%d.pdf".parse().expect("output pattern parses"),
        }),
        &[Source::new(source_bytes)],
        &sinks,
        &Policy::default(),
        &Budget::default(),
    )
    .expect("the valid second-page split applies");
    assert!(report.refused.is_empty(), "valid page is split");
    let outputs = sinks.into_outputs();
    assert_eq!(outputs.len(), 1);

    let (_, output_bytes) = outputs.into_iter().next().expect("split wrote one output");
    let output = Document::open(output_bytes).expect("split output opens");
    assert!(!output.was_recovered(), "split output has valid xref data");
    assert_output_page(&output);
}

fn assert_output_page(output: &Document) {
    let catalog = output
        .catalog()
        .expect("output trailer resolves its catalog");
    assert_eq!(
        output
            .get_key(&catalog, "Type")
            .as_name()
            .map(pdf_syntax::Name::as_bytes),
        Some(&b"Catalog"[..]),
        "writer must state the Catalog's required /Type"
    );
    assert!(
        output
            .get(page_tree_id_from_catalog(&catalog))
            .as_dict()
            .is_some(),
        "Catalog's /Pages reaches the output page tree"
    );
    let page_tree_id = page_tree_id_from_catalog(&catalog);
    let page_tree = output.get(page_tree_id);
    let page_tree = page_tree
        .as_dict()
        .expect("output page tree is a dictionary");
    assert_eq!(
        page_tree
            .get("Type")
            .and_then(pdf_syntax::Object::as_name)
            .map(pdf_syntax::Name::as_bytes),
        Some(&b"Pages"[..])
    );
    assert_eq!(
        page_tree
            .get("Count")
            .and_then(pdf_syntax::Object::as_integer),
        Some(1)
    );
    let kids = output.get_key(page_tree, "Kids");
    let kids = kids.as_array().expect("Kids is an array");
    assert_eq!(kids.len(), 1, "only the selected page is emitted");
    let page_id = kids
        .first()
        .and_then(pdf_syntax::Object::as_reference)
        .expect("child is indirect");
    let page = output.get(page_id);
    let page = page.as_dict().expect("output page is a dictionary");
    assert_eq!(
        output
            .get_key(page, "Type")
            .as_name()
            .map(pdf_syntax::Name::as_bytes),
        Some(&b"Page"[..]),
        "writer must state the Page object's required /Type"
    );
    assert_eq!(
        page.get("Parent")
            .and_then(pdf_syntax::Object::as_reference),
        Some(page_tree_id),
        "writer must point the Page at its immediate page-tree parent"
    );
    assert!(
        output.get_key(page, "Resources").as_dict().is_some(),
        "split materializes the selected page resources"
    );
    let media = output.get_key(page, "MediaBox");
    let media = media.as_array().expect("output page has its media box");
    assert_eq!(
        media
            .iter()
            .map(pdf_syntax::Object::as_integer)
            .collect::<Vec<_>>(),
        [Some(0), Some(0), Some(20), Some(30)],
        "selected second page geometry is preserved"
    );
}

fn page_tree_id_from_catalog(catalog: &pdf_syntax::Dictionary) -> pdf_syntax::ObjectId {
    catalog
        .get("Pages")
        .and_then(pdf_syntax::Object::as_reference)
        .expect("Catalog /Pages names the page tree by reference")
}

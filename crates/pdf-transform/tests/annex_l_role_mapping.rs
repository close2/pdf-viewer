//! Integration test for Annex L's role-mapped population.
//!
//! The source
//! PDFs are complete one-page tagged files, and the test reads the merged structure tree back.

#![expect(
    clippy::expect_used,
    reason = "test code: a fixture that cannot exercise the rule must fail loudly"
)]

use std::fmt::Write as _;

use pdf_syntax::{Document, Object};
use pdf_transform::merge::{Input, MergePlan};
use pdf_transform::range::Selection;
use pdf_transform::{Budget, MemorySinks, Plan, Policy, Source, apply};

/// The namespace names the root explicitly identifies.
fn namespace_names(document: &Document, root: &pdf_syntax::Dictionary) -> Vec<String> {
    let namespaces = document.get_key(root, "Namespaces");
    let namespaces = namespaces
        .as_array()
        .expect("Table 354 Namespaces is an array");
    namespaces
        .iter()
        .filter_map(|value| {
            let namespace = document.resolve(value);
            namespace
                .as_dict()
                .and_then(|dict| pdf_model::structure::Namespace::read(document, dict))
                .map(|namespace| namespace.name)
        })
        .collect()
}

/// A complete one-page PDF whose top-level structure element maps to PDF 2.0 `Document`.
fn role_mapped_pdf(first_id: &str, second_id: &str) -> Vec<u8> {
    let content = "/P << /MCID 0 >> BDC\n0 0 10 10 re f\nEMC\n";
    let body = format!(
        r"1 0 obj
<< /Type /Catalog /Pages 2 0 R /StructTreeRoot 5 0 R /MarkInfo << /Marked true >> >>
endobj
2 0 obj
<< /Type /Pages /Kids [3 0 R] /Count 1 >>
endobj
3 0 obj
<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 100] /Resources << >> /Contents 4 0 R /StructParents 0 >>
endobj
4 0 obj
<< /Length {len} >>
stream
{content}endstream
endobj
5 0 obj
<< /Type /StructTreeRoot /K [6 0 R] /ParentTree << /Nums [0 [7 0 R]] >> /ParentTreeNextKey 1 /Namespaces [8 0 R 9 0 R] >>
endobj
6 0 obj
<< /Type /StructElem /S /DocAlias /NS 8 0 R /P 5 0 R /K [7 0 R] >>
endobj
7 0 obj
<< /Type /StructElem /S /P /NS 9 0 R /P 6 0 R /Pg 3 0 R /K [0] >>
endobj
8 0 obj
<< /Type /Namespace /NS (https://example.org/pdf-viewer-audit) /RoleMapNS << /DocAlias [/Document 9 0 R] >> >>
endobj
9 0 obj
<< /Type /Namespace /NS (http://iso.org/pdf2/ssn) >>
endobj
",
        len = content.len(),
    );
    let mut out = String::from("%PDF-2.0\n");
    let mut offsets = Vec::new();
    for object in body.split_inclusive("endobj\n") {
        offsets.push(out.len());
        out.push_str(object);
    }
    let xref_at = out.len();
    let size = offsets.len().saturating_add(1);
    let _ = writeln!(out, "xref\n0 {size}");
    out.push_str("0000000000 65535 f \n");
    for offset in &offsets {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    let _ = write!(
        out,
        "trailer\n<< /Size {size} /Root 1 0 R /ID [<{first_id}> <{second_id}>] >>\nstartxref\n{xref_at}\n%%EOF\n"
    );
    out.into_bytes()
}

/// Merge complete one-page PDFs and return the serialized result.
fn merge_two(first: &[u8], second: &[u8]) -> Vec<u8> {
    let plan = Plan::Merge(MergePlan {
        inputs: [first, second]
            .into_iter()
            .enumerate()
            .map(|(source, _)| Input {
                source,
                pages: "1".parse::<Selection>().expect("one-page selection"),
            })
            .collect(),
        collate: false,
        information: Vec::new(),
        names: "merged.pdf".parse().expect("valid output name pattern"),
    });
    let sources = [Source::new(first.to_vec()), Source::new(second.to_vec())];
    let sinks = MemorySinks::new();
    let _report = apply(
        &plan,
        &sources,
        &sinks,
        &Policy::default(),
        &Budget::default(),
    )
    .expect("the valid PDFs merge");
    let mut outputs = sinks.into_outputs();
    assert_eq!(outputs.len(), 1, "the merge writes one PDF");
    outputs.remove(0).1
}

/// Establishes the source role, namespace and complete page before checking the writer.
fn assert_source_population(input: &[u8]) {
    let source = Document::open(input).expect("the complete source PDF opens");
    assert!(!source.was_recovered(), "the source xref is valid");
    let pages = pdf_model::Pages::new(&source);
    assert_eq!(pages.len(), 1);
    let page = pages.get(0).expect("the source has its complete page");
    assert!(pdf_model::interpret(&source, &page).is_complete());
    let catalog = source.catalog().expect("source catalog");
    let root = source.get_key(&catalog, "StructTreeRoot");
    let root = root.as_dict().expect("source structure tree root");
    let namespaces = namespace_names(&source, root);
    assert!(
        namespaces
            .iter()
            .any(|name| name == "https://example.org/pdf-viewer-audit"),
        "the source root identifies the custom namespace"
    );
    assert!(
        namespaces
            .iter()
            .any(|name| name == pdf_model::structure::STANDARD_NAMESPACE_2_0),
        "the source root identifies the mapping target namespace"
    );
    let top = source.get_key(root, "K");
    let top = top
        .as_array()
        .expect("the source names its one top-level element");
    let element = source.resolve(&top[0]);
    let element = element.as_dict().expect("source structure element");
    let tree = pdf_model::structure::Tree::of(&source).expect("source structure model");
    assert_eq!(
        element
            .get("S")
            .and_then(Object::as_name)
            .map(pdf_syntax::Name::as_bytes),
        Some(&b"DocAlias"[..]),
        "the fixture starts with a custom role name"
    );
    assert_eq!(
        tree.namespace(&source, element).as_deref(),
        Some("https://example.org/pdf-viewer-audit"),
        "the source role is in its custom namespace"
    );
    assert_eq!(
        tree.role(&source, element).as_deref(),
        Some("Document"),
        "the custom namespace's RoleMapNS maps DocAlias to Document"
    );
    assert_eq!(
        tree.standard_role(&source, element),
        Some(pdf_model::structure::StandardType::Document)
    );
    assert!(tree.in_pdf_2_0_namespace(&source, element));
}

/// Annex L applies Table L.2 to elements role mapped into the PDF 2.0 standard namespace.
///
/// Both source roots expose a custom-namespace `DocAlias` whose `/RoleMapNS` maps to `/Document`
/// in the explicitly referenced PDF 2.0 namespace.
/// The merge therefore must treat both as top-level PDF 2.0 Documents and place them beneath the
/// one `Document` required by Table L.2's `StructTreeRoot` row. This fails if wrapper selection
/// checks only the literal `/S` name or ignores namespace role mapping.
#[test]
fn role_mapped_pdf_2_documents_are_written_inside_one_document() {
    let first = role_mapped_pdf(
        "00112233445566778899AABBCCDDEEFF",
        "FFEEDDCCBBAA99887766554433221100",
    );
    let second = role_mapped_pdf(
        "112233445566778899AABBCCDDEEFF00",
        "00FFEEDDCCBBAA998877665544332211",
    );
    for input in [&first, &second] {
        assert_source_population(input);
    }
    let bytes = merge_two(&first, &second);
    let merged = Document::open(bytes).expect("the merged file reopens");
    assert!(!merged.was_recovered(), "the output xref is valid");
    let catalog = merged.catalog().expect("catalog");
    let root = merged.get_key(&catalog, "StructTreeRoot");
    let root = root.as_dict().expect("structure tree root");
    let namespaces = namespace_names(&merged, root);
    assert!(
        namespaces
            .iter()
            .any(|name| name == pdf_model::structure::STANDARD_NAMESPACE_2_0),
        "the output root identifies the wrapper's explicit namespace"
    );
    let top = merged.get_key(root, "K");
    let Object::Array(top) = top else {
        panic!("Table L.2's root has its one Document child");
    };
    assert_eq!(top.len(), 1, "Table L.2 allows one root Document");

    let tree = pdf_model::structure::Tree::of(&merged).expect("structure tree model");
    let wrapper = merged.resolve(&top[0]);
    let wrapper = wrapper
        .as_dict()
        .expect("the root child is a structure element");
    assert_eq!(
        tree.standard_role(&merged, wrapper),
        Some(pdf_model::structure::StandardType::Document)
    );
    assert!(tree.in_pdf_2_0_namespace(&merged, wrapper));

    let children = merged.get_key(wrapper, "K");
    let Object::Array(children) = children else {
        panic!("the wrapper contains the two source Documents");
    };
    assert_eq!(children.len(), 2);
    for child in children {
        let element = merged.resolve(&child);
        let element = element.as_dict().expect("source Document element");
        assert_eq!(
            element
                .get("S")
                .and_then(Object::as_name)
                .map(pdf_syntax::Name::as_bytes),
            Some(&b"DocAlias"[..]),
            "the merge preserves the producer's role name"
        );
        assert_eq!(
            tree.standard_role(&merged, element),
            Some(pdf_model::structure::StandardType::Document),
            "namespace role mapping gives this child the Document type"
        );
        assert_eq!(
            tree.namespace(&merged, element).as_deref(),
            Some("https://example.org/pdf-viewer-audit"),
            "the custom namespace dictionary crosses with the element"
        );
        assert!(tree.in_pdf_2_0_namespace(&merged, element));
        let parent = merged.resolve(element.get("P").expect("Table 355 requires /P"));
        assert_eq!(parent.as_dict(), Some(wrapper));
    }
    assert_eq!(pdf_model::Pages::new(&merged).len(), 2);
}

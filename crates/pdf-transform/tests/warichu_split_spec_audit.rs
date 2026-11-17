//! PDF 2.0 structure sequence preservation through page splitting.
//!
//! Table 369 (physical780/printed765 in EC3) requires Warichu children WP, WT, WP.
//! Table 355 (physical740/printed725) makes a structure element's K optional; WP text
//! is only typical under Table 369. The two WP boundaries therefore omit K. Both WT
//! lines stay on page one beside the base text. A second independent paragraph is on
//! page two. No page-spanning Warichu or empty K array is used.
//!
//! This tests the successful split's output structure. An explicit refusal policy would
//! need its own contract and assertion; a successful output must retain the sequence.

#![expect(
    clippy::expect_used,
    clippy::panic,
    reason = "test fixtures and output are required for the regression to be meaningful"
)]

use std::fmt::Write as _;

use pdf_syntax::{Document, Limits};
use pdf_transform::split::{Pieces, SplitPlan};
use pdf_transform::{Budget, MemorySinks, Plan, Policy, Source, apply};

#[expect(
    clippy::too_many_lines,
    clippy::arithmetic_side_effects,
    reason = "one fixed 22-object fixture keeps its exact references visible; offsets are only a few kilobytes"
)]
fn warichu_document(boundary_text: bool) -> Vec<u8> {
    let mut content = [
        "/P <</MCID 0>> BDC BT /F1 16 Tf 5 54 Td (A) Tj ET EMC\n\
         /WT <</MCID 1>> BDC BT /F1 6 Tf 20 61 Td (A) Tj ET EMC\n\
         /WT <</MCID 2>> BDC BT /F1 6 Tf 20 55 Td (B) Tj ET EMC\n"
            .to_owned(),
        "/P <</MCID 0>> BDC BT /F1 16 Tf 5 54 Td (B) Tj ET EMC\n".to_owned(),
    ];
    if boundary_text {
        // Hex strings avoid confusing the literal punctuation with PDF delimiters.
        content[0].push_str(
            "/WP <</MCID 3>> BDC BT /F1 12 Tf 16 55 Td <28> Tj ET EMC\n\
            /WP <</MCID 4>> BDC BT /F1 12 Tf 25 55 Td <29> Tj ET EMC\n",
        );
    }
    let mut body = String::new();
    let mut offsets = [0_usize; 23];
    let mut add = |id: usize, value: String| {
        offsets[id] = body.len();
        let _ = write!(body, "{id} 0 obj\n{value}\nendobj\n");
    };

    add(
        1,
        "<< /Type /Catalog /Pages 2 0 R /StructTreeRoot 7 0 R /MarkInfo << /Marked true >> >>"
            .into(),
    );
    add(2, "<< /Type /Pages /Kids [3 0 R 4 0 R] /Count 2 >>".into());
    for (index, stream) in content.iter().enumerate() {
        let page_id = 3 + index;
        let stream_id = 5 + index;
        add(
            page_id,
            format!(
                "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Resources << /Font << /F1 21 0 R >> >> /Contents {stream_id} 0 R /StructParents {index} >>"
            ),
        );
        add(
            stream_id,
            format!("<< /Length {} >>\nstream\n{stream}endstream", stream.len()),
        );
    }
    add(
        7,
        "<< /Type /StructTreeRoot /K [10 0 R] /ParentTree 8 0 R /ParentTreeNextKey 2 /Namespaces [9 0 R] >>".into(),
    );
    add(
        8,
        if boundary_text {
            "<< /Nums [0 [11 0 R 14 0 R 14 0 R 13 0 R 15 0 R] 1 [18 0 R]] >>"
        } else {
            "<< /Nums [0 [11 0 R 14 0 R 14 0 R] 1 [18 0 R]] >>"
        }
        .into(),
    );
    add(
        9,
        "<< /Type /Namespace /NS (http://iso.org/pdf2/ssn) >>".into(),
    );
    add(
        10,
        "<< /Type /StructElem /S /Document /NS 9 0 R /P 7 0 R /K [11 0 R 18 0 R] >>".into(),
    );
    add(
        11,
        "<< /Type /StructElem /S /P /NS 9 0 R /P 10 0 R /K [19 0 R 12 0 R] >>".into(),
    );
    add(
        12,
        "<< /Type /StructElem /S /Warichu /NS 9 0 R /P 11 0 R /K [13 0 R 14 0 R 15 0 R] >>".into(),
    );
    add(
        13,
        format!(
            "<< /Type /StructElem /S /WP /NS 9 0 R /P 12 0 R {} >>",
            if boundary_text {
                "/K << /Type /MCR /Pg 3 0 R /MCID 3 >>"
            } else {
                ""
            }
        ),
    );
    add(
        14,
        "<< /Type /StructElem /S /WT /NS 9 0 R /P 12 0 R /K [16 0 R 17 0 R] >>".into(),
    );
    add(
        15,
        format!(
            "<< /Type /StructElem /S /WP /NS 9 0 R /P 12 0 R {} >>",
            if boundary_text {
                "/K << /Type /MCR /Pg 3 0 R /MCID 4 >>"
            } else {
                ""
            }
        ),
    );
    add(16, "<< /Type /MCR /Pg 3 0 R /MCID 1 >>".into());
    add(17, "<< /Type /MCR /Pg 3 0 R /MCID 2 >>".into());
    add(
        18,
        "<< /Type /StructElem /S /P /NS 9 0 R /P 10 0 R /K [20 0 R] >>".into(),
    );
    add(19, "<< /Type /MCR /Pg 3 0 R /MCID 0 >>".into());
    add(20, "<< /Type /MCR /Pg 4 0 R /MCID 0 >>".into());
    // Standard-14 Helvetica metrics, contiguous FirstChar..LastChar interval 40..66.
    add(
        21,
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding /FirstChar 40 /LastChar 66 /Widths [333 333 389 584 278 333 278 278 556 556 556 556 556 556 556 556 556 556 278 278 584 584 584 556 1015 667 667] /FontDescriptor 22 0 R >>".into(),
    );
    add(
        22,
        "<< /Type /FontDescriptor /FontName /Helvetica /Flags 32 /FontBBox [-166 -225 1000 931] /ItalicAngle 0 /Ascent 718 /Descent -207 /CapHeight 718 /StemV 88 >>".into(),
    );

    let xref_at = body.len();
    let size = offsets.len();
    let mut output = String::from("%PDF-2.0\n");
    let shift = output.len();
    output.push_str(&body);
    let _ = writeln!(output, "xref\n0 {size}");
    output.push_str("0000000000 65535 f \n");
    for offset in offsets.iter().skip(1) {
        let _ = writeln!(output, "{:010} 00000 n ", offset + shift);
    }
    let _ = write!(
        output,
        "trailer\n<< /Size {size} /Root 1 0 R /ID [<00112233445566778899AABBCCDDEEFF> <00112233445566778899AABBCCDDEEFF>] >>\nstartxref\n{}\n%%EOF\n",
        xref_at + shift
    );
    output.into_bytes()
}

fn split_each_page(bytes: &[u8]) -> Vec<(String, Vec<u8>)> {
    let sinks = MemorySinks::new();
    apply(
        &Plan::Split(SplitPlan {
            source: 0,
            pages: "1-2".parse().expect("two pages"),
            pieces: Pieces::EachPage,
            names: "piece-%d.pdf".parse().expect("name pattern"),
        }),
        &[Source::new(bytes.to_vec())],
        &sinks,
        &Policy::default(),
        &Budget::default(),
    )
    .expect("splitting two pages");
    sinks.into_outputs()
}

fn element(
    tree: &pdf_model::structure::Tree,
    document: &Document,
    parent: Option<&pdf_syntax::Dictionary>,
    index: usize,
) -> pdf_syntax::Dictionary {
    let children = tree.children(document, parent);
    match children.get(index).expect("the expected child exists") {
        pdf_model::structure::Child::Element(dict) => dict.clone(),
        other => panic!("expected a structure element child, got {other:?}"),
    }
}

fn role(
    tree: &pdf_model::structure::Tree,
    document: &Document,
    element: &pdf_syntax::Dictionary,
) -> String {
    tree.role(document, element).expect("standard PDF 2.0 role")
}

fn child_with_role(
    tree: &pdf_model::structure::Tree,
    document: &Document,
    parent: &pdf_syntax::Dictionary,
    expected: &str,
) -> pdf_syntax::Dictionary {
    tree.children(document, Some(parent))
        .into_iter()
        .find_map(|child| match child {
            pdf_model::structure::Child::Element(dict)
                if role(tree, document, &dict) == expected =>
            {
                Some(dict)
            }
            _ => None,
        })
        .unwrap_or_else(|| panic!("no {expected} child under {parent:?}"))
}

fn assert_split_sequence(boundary_text: bool) {
    let source_bytes = warichu_document(boundary_text);
    let source = Document::open_with_limits(source_bytes.clone(), Limits::DEFAULT)
        .expect("the exact-xref source opens");
    assert!(!source.was_recovered(), "source xref offsets are exact");
    assert!(!source.cross_reference_chain_was_cut());
    let source_tree = pdf_model::structure::Tree::of(&source).expect("source structure tree");
    let source_document = element(&source_tree, &source, None, 0);
    let source_paragraph = child_with_role(&source_tree, &source, &source_document, "P");
    let source_warichu = child_with_role(&source_tree, &source, &source_paragraph, "Warichu");
    let source_roles: Vec<String> = source_tree
        .children(&source, Some(&source_warichu))
        .iter()
        .filter_map(|child| match child {
            pdf_model::structure::Child::Element(dict) => Some(role(&source_tree, &source, dict)),
            _ => None,
        })
        .collect();
    assert_eq!(source_roles, vec!["WP", "WT", "WP"]);
    for index in 0..2 {
        let page = pdf_model::Pages::new(&source)
            .get(index)
            .expect("each page");
        let interpretation = pdf_model::interpret(&source, &page);
        assert!(
            interpretation.is_complete(),
            "source page {index}: {:?}",
            interpretation.unsupported
        );
        assert_eq!(
            interpretation.glyphs,
            if index == 0 {
                if boundary_text { 5 } else { 3 }
            } else {
                1
            },
            "source marks survive interpretation"
        );
        let parents = pdf_model::structure::ParentTree::for_page(&source, &page.dict);
        assert!(parents.element(&source, 0).is_some(), "page {index} MCID 0");
        if index == 0 {
            assert!(parents.element(&source, 1).is_some(), "WT line 1 MCID");
            assert!(parents.element(&source, 2).is_some(), "WT line 2 MCID");
            if boundary_text {
                assert!(parents.element(&source, 3).is_some(), "opening WP MCID");
                assert!(parents.element(&source, 4).is_some(), "closing WP MCID");
            }
        }
        assert_eq!(
            source.get_key(&page.dict, "StructParents").as_integer(),
            Some(i64::try_from(index).expect("page index fits"))
        );
    }

    let pieces = split_each_page(&source_bytes);
    assert_eq!(pieces.len(), 2);
    let first = pieces
        .iter()
        .find(|(name, _)| name == "piece-1.pdf")
        .expect("page one output");
    let output = Document::open_with_limits(first.1.clone(), Limits::DEFAULT).expect("output PDF");
    assert!(!output.was_recovered());
    assert!(!output.cross_reference_chain_was_cut());
    let page = pdf_model::Pages::new(&output).get(0).expect("split page");
    let interpretation = pdf_model::interpret(&output, &page);
    assert!(
        interpretation.is_complete(),
        "output: {:?}",
        interpretation.unsupported
    );
    assert_eq!(interpretation.glyphs, if boundary_text { 5 } else { 3 });
    let tree = pdf_model::structure::Tree::of(&output).expect("output structure tree");
    let document = element(&tree, &output, None, 0);
    let paragraph = child_with_role(&tree, &output, &document, "P");
    let warichu = child_with_role(&tree, &output, &paragraph, "Warichu");
    let roles: Vec<String> = tree
        .children(&output, Some(&warichu))
        .iter()
        .filter_map(|child| match child {
            pdf_model::structure::Child::Element(dict) => Some(role(&tree, &output, dict)),
            _ => None,
        })
        .collect();
    assert_eq!(
        roles,
        vec!["WP", "WT", "WP"],
        "successful page-one split must retain the required Table 369 sequence"
    );
}

#[test]
fn each_page_split_keeps_warichu_boundaries_with_text() {
    assert_split_sequence(true);
}

#[test]
fn each_page_split_keeps_empty_warichu_boundary_elements() {
    assert_split_sequence(false);
}

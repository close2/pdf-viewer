//! ISO 32000-2 §14.7.7's worked example, run as a document.
//!
//! The clause spends four pages on one file: a structure tree root with a role map, a class map,
//! a parent tree and an ID tree; a chapter containing a heading and a paragraph; a paragraph that
//! **spans two pages** through a marked-content reference; and attributes attached three
//! different ways. Every one of those is a mechanism some other subclause states in the abstract,
//! and this is the one place the standard shows them working together.
//!
//! So it is a test. The fixture below is the clause's own objects, with the object numbers and
//! the entries it writes — including its `101 1 obj`, a generation number of 1, which is the only
//! non-zero generation in this tree's whole test corpus and is exactly the sort of thing a reader
//! quietly assumes away.
//!
//! What it checks is that the *pieces the clause demonstrates* are the pieces this crate reads:
//! the role map's three mappings, the class map's attributes under an element's own override, the
//! parent tree in both of its array forms, the ID tree, and the logical order across two pages
//! with a paragraph that continues onto the second.

#![expect(
    clippy::arithmetic_side_effects,
    clippy::too_many_lines,
    reason = "test code: a fixture that cannot exercise what the test is about is a failure, \
              and the fixture is one clause's example written out object by object"
)]

use std::fmt::Write as _;

use pdf_model::structure::{Child, ParentTree, StandardType, Tree};
use pdf_syntax::{Document, Object, ObjectId};

/// §14.7.7's EXAMPLE, as bytes.
///
/// The content streams are the clause's, shortened to the text they show — what this test is
/// about is the structure over them, and the glyph metrics of a 30-point Helvetica are
/// §9.2's business.
fn example() -> Vec<u8> {
    let first = "BT /Span << /MCID 0 >> BDC /F1 1 Tf 30 0 0 30 18 732 Tm \
                 (This is a first level heading. Hello world: ) Tj EMC \
                 /Span << /MCID 1 >> BDC /F12 1 Tf 14 0 0 14 18 660.8 Tm \
                 (This is the first paragraph, which spans pages.) Tj EMC ET";
    let second = "BT /Para << /MCID 0 >> BDC /F12 1 Tf 14 0 0 14 18 732 Tm \
                  (This is the very last sentence of the first paragraph.) Tj EMC \
                  /Span << /MCID 1 >> BDC 14 0 0 14 18 570.8 Tm \
                  (This is the second paragraph.) Tj EMC \
                  /Span << /MCID 2 >> BDC 14 0 0 14 18 550.8 Tm \
                  (The very last sentence of the second paragraph.) Tj EMC ET";
    let objects: Vec<(u32, u16, String)> =
        vec![
        (1, 0, "<< /Type /Catalog /Pages 100 0 R /StructTreeRoot 300 0 R >>".to_owned()),
        (100, 0, "<< /Type /Pages /Kids [101 1 R 102 0 R] /Count 2 >>".to_owned()),
        (
            101,
            1,
            "<< /Type /Page /Parent 100 0 R /Resources << /Font << /F1 6 0 R /F12 7 0 R >> >> \
             /MediaBox [0 0 612 792] /Contents 201 0 R /StructParents 0 >>"
                .to_owned(),
        ),
        (
            102,
            0,
            "<< /Type /Page /Parent 100 0 R /Resources << /Font << /F1 6 0 R /F12 7 0 R >> >> \
             /MediaBox [0 0 612 792] /Contents 202 0 R /StructParents 1 >>"
                .to_owned(),
        ),
        (
            201,
            0,
            format!("<< /Length {} >>\nstream\n{first}\nendstream", first.len() + 1),
        ),
        (
            202,
            0,
            format!("<< /Length {} >>\nstream\n{second}\nendstream", second.len() + 1),
        ),
        (6, 0, "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_owned()),
        (7, 0, "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_owned()),
        (
            300,
            0,
            "<< /Type /StructTreeRoot /K [301 0 R 304 0 R] \
             /RoleMap << /Chap /Sect /Head1 /H /Para /P >> \
             /ClassMap << /Normal 305 0 R >> /ParentTree 400 0 R /ParentTreeNextKey 2 \
             /IDTree 403 0 R >>"
                .to_owned(),
        ),
        (
            301,
            0,
            "<< /Type /StructElem /S /Chap /ID (Chap1) /T (Chapter 1) /P 300 0 R \
             /K [302 0 R 303 0 R] >>"
                .to_owned(),
        ),
        (
            302,
            0,
            "<< /Type /StructElem /S /Head1 /ID (Sec1.1) /T (Section 1.1) /P 301 0 R /Pg 101 1 R \
             /A << /O /Layout /SpaceAfter 25 /SpaceBefore 0 /TextIndent 12.5 >> /K 0 >>"
                .to_owned(),
        ),
        (
            303,
            0,
            "<< /Type /StructElem /S /Para /ID (Para1) /P 301 0 R /Pg 101 1 R /C /Normal \
             /K [1 << /Type /MCR /Pg 102 0 R /MCID 0 >>] >>"
                .to_owned(),
        ),
        (
            304,
            0,
            "<< /Type /StructElem /S /Para /P 300 0 R /Pg 102 0 R /C /Normal \
             /A << /O /Layout /TextAlign /Justify >> /K [1 2] >>"
                .to_owned(),
        ),
        (
            305,
            0,
            "<< /O /Layout /EndIndent 0 /StartIndent 0 /WritingMode /LrTb /TextAlign /Start >>"
                .to_owned(),
        ),
        (400, 0, "<< /Nums [0 401 0 R 1 402 0 R] >>".to_owned()),
        (401, 0, "[302 0 R 303 0 R]".to_owned()),
        (402, 0, "[303 0 R 304 0 R 304 0 R]".to_owned()),
        (403, 0, "<< /Kids [404 0 R] >>".to_owned()),
        (
            404,
            0,
            "<< /Limits [(Chap1) (Sec1.3)] /Names [(Chap1) 301 0 R (Sec1.1) 302 0 R \
             (Sec1.2) 303 0 R (Sec1.3) 304 0 R] >>"
                .to_owned(),
        ),
    ];

    let mut out = String::from("%PDF-1.7\n");
    let mut offsets: std::collections::BTreeMap<u32, (usize, u16)> =
        std::collections::BTreeMap::new();
    for (number, generation, body) in &objects {
        offsets.insert(*number, (out.len(), *generation));
        let _ = write!(out, "{number} {generation} obj\n{body}\nendobj\n");
    }
    let xref_at = out.len();
    let size = offsets.keys().copied().max().unwrap_or(0) + 1;
    let _ = write!(out, "xref\n0 {size}\n0000000000 65535 f \n");
    for number in 1..size {
        match offsets.get(&number) {
            Some((offset, generation)) => {
                let _ = writeln!(out, "{offset:010} {generation:05} n ");
            }
            None => {
                let _ = writeln!(out, "0000000000 65535 f ");
            }
        }
    }
    let _ = write!(
        out,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n"
    );
    out.into_bytes()
}

/// The clause's role map, class map, parent tree, ID tree and cross-page paragraph, all at once.
#[test]
fn the_clauses_worked_example_reads_as_the_structure_it_describes() {
    let document = Document::open(example()).expect("§14.7.7's example is a valid file");
    let tree = Tree::of(&document).expect("a structure tree");

    // The role map: three of the document's own type names, mapped to standard ones. The clause
    // says the elements "are mapped to the standard structure types specified in tagged PDF …
    // by means of the role map specified in the structure tree root".
    let children = tree.children(&document, None);
    let [Child::Element(chapter), Child::Element(paragraph)] = children.as_slice() else {
        panic!("two children of the root, got {children:?}");
    };
    assert_eq!(tree.role(&document, chapter).as_deref(), Some("Sect"));
    assert_eq!(
        tree.standard_role(&document, chapter),
        Some(StandardType::Section)
    );
    assert_eq!(tree.role(&document, paragraph).as_deref(), Some("P"));

    let inside = tree.children(&document, Some(chapter));
    let [Child::Element(heading), Child::Element(spanning)] = inside.as_slice() else {
        panic!("a heading and a paragraph, got {inside:?}");
    };
    assert_eq!(
        tree.standard_role(&document, heading),
        Some(StandardType::UnnumberedHeading),
        "/Head1 maps to /H, which is a standard type without a level"
    );

    // Attributes three ways: the heading's own `/A`, the class map's `/Normal` reached through
    // `/C`, and §14.7.6.2's precedence where an element states both.
    assert_eq!(
        tree.attribute(&document, heading, "TextIndent"),
        Some(Object::Real(12.5))
    );
    assert_eq!(
        tree.attribute(&document, spanning, "WritingMode")
            .and_then(|value| value.as_name().map(ToString::to_string)),
        Some("/LrTb".to_owned()),
        "from the class map, which the element reaches through /C"
    );
    assert_eq!(
        tree.attribute(&document, paragraph, "TextAlign")
            .and_then(|value| value.as_name().map(ToString::to_string)),
        Some("/Justify".to_owned()),
        "the element's own /A overrides the class map's /Start"
    );

    // The ID tree: four identifiers, one of which the example never attaches to an element.
    for (id, expected) in [
        (&b"Chap1"[..], "Sect"),
        (&b"Sec1.1"[..], "H"),
        (&b"Sec1.2"[..], "P"),
    ] {
        let element = tree
            .element_by_id(&document, id)
            .unwrap_or_else(|| panic!("{} is in the ID tree", String::from_utf8_lossy(id)));
        assert_eq!(tree.role(&document, &element).as_deref(), Some(expected));
    }
    assert!(
        tree.element_by_id(&document, b"Chap2").is_none(),
        "an identifier the tree does not hold"
    );

    // The parent tree, in both of the forms Table 354's `/Nums` takes: an array per page, whose
    // entries are the elements owning each `/MCID` in order. The second page's array names 304
    // twice, because one element owns two of its sequences.
    let pages = pdf_model::Pages::new(&document);
    let first = pages.get(0).expect("the first page");
    let parents = ParentTree::for_page(&document, &first.dict);
    assert_eq!(
        parents
            .element(&document, 0)
            .and_then(|element| tree.role(&document, &element)),
        Some("H".to_owned()),
        "sequence 0 of the first page belongs to the heading"
    );
    let second = pages.get(1).expect("the second page");
    let parents = ParentTree::for_page(&document, &second.dict);
    assert_eq!(
        parents
            .element(&document, 1)
            .and_then(|element| tree.role(&document, &element)),
        Some("P".to_owned())
    );

    // The paragraph that spans pages: its `/K` holds an integer for the sequence on its own
    // `/Pg` and a marked-content reference that names the *other* page.
    let items = tree.children(&document, Some(spanning));
    assert_eq!(
        items,
        vec![
            Child::MarkedContent {
                mcid: 1,
                page: Some(ObjectId::new(101, 1)),
            },
            Child::MarkedContent {
                mcid: 0,
                page: Some(ObjectId::new(102, 0)),
            },
        ],
        "one item on each page, the second through a /MCR that moves the page"
    );

    // And the whole of it in logical order, per page. The first page's logical order is the
    // heading then the start of the paragraph; the second page's is the paragraph's
    // continuation and then the second paragraph's two sequences — which is the clause's own
    // point about a logical object extending over more than one page.
    assert_eq!(
        tree.logical_order(&document, ObjectId::new(101, 1)).items,
        vec![
            Child::MarkedContent {
                mcid: 0,
                page: Some(ObjectId::new(101, 1)),
            },
            Child::MarkedContent {
                mcid: 1,
                page: Some(ObjectId::new(101, 1)),
            },
        ]
    );
    assert_eq!(
        tree.logical_order(&document, ObjectId::new(102, 0))
            .items
            .iter()
            .map(|item| match item {
                Child::MarkedContent { mcid, .. } => *mcid,
                _ => -1,
            })
            .collect::<Vec<_>>(),
        vec![0, 1, 2],
        "the first paragraph's continuation, then the second paragraph's two sequences"
    );

    let interpretation = pdf_model::interpret(&document, &second);
    let logical = tree
        .logical_text(&document, ObjectId::new(102, 0), &interpretation)
        .expect("the fixture's tree is far below the walk's bound");
    assert!(
        logical.starts_with("This is the very last sentence of the first paragraph."),
        "the second page reads the first paragraph's end first: {logical:?}"
    );
}

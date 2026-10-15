//! A copy off a tagged page: §14.8.2.5's logical order, the structure's paragraph and cell breaks,
//! and §14.8.2.2's artifacts left out (ADR 1948).
//!
//! Both halves are here, over one fixture: the structure's — which pieces of the readback, in
//! which order, with which boundary between them, which `pdf_model::structure::Tree::logical_range`
//! states — and the text [`Event::Copied`] carries when a person selects the whole page and presses
//! copy, which is where the boundaries become characters.
//!
//! The fixture is built here rather than found, because no corpus page states all four things at
//! once — a stream order that is not the logical one, a `Span` inside a word, a table, and a
//! running head and folio tagged `/Artifact` — and each is a sentence of the clause to hold the copy
//! to.

#![expect(
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    reason = "test code: a fixture that cannot exercise the rule must fail loudly, and the offsets \
              are within a fixture written in this file"
)]

use std::fmt::Write as _;

use pdf_render::Rasterizer;
use render_cpu::CpuRasterizer;
use viewer_core::{Command, DocumentId, Event, Rendered, Selection, Viewer};

/// The document every test here opens.
const DOCUMENT: DocumentId = DocumentId(1);

/// A one-page tagged document: `content` as the page's stream, `structure` as objects 10 onwards
/// with object 10 the `/StructTreeRoot`.
fn tagged(content: &str, structure: &str) -> Vec<u8> {
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R /StructTreeRoot 10 0 R \
         /MarkInfo << /Marked true >> >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 300 300] \
         /Resources << /Font << /F1 5 0 R >> >> /Contents 4 0 R /StructParents 0 >>\nendobj\n\
         4 0 obj\n<< /Length {} >>\nstream\n{content}\nendstream\nendobj\n\
         5 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>\nendobj\n\
         {structure}",
        content.len() + 1,
    );
    let mut out = String::from("%PDF-1.7\n");
    let mut offsets: std::collections::BTreeMap<usize, usize> = std::collections::BTreeMap::new();
    let mut cursor = out.len();
    for object in body.split_inclusive("endobj\n") {
        let number: usize = object
            .split_whitespace()
            .next()
            .and_then(|word| word.parse().ok())
            .expect("every object states its number");
        offsets.insert(number, cursor);
        cursor += object.len();
    }
    out.push_str(&body);
    let xref_at = out.len();
    let size = offsets.keys().copied().max().unwrap_or(0) + 1;
    let _ = write!(out, "xref\n0 {size}\n0000000000 65535 f \n");
    for number in 1..size {
        match offsets.get(&number) {
            Some(offset) => {
                let _ = writeln!(out, "{offset:010} 00000 n ");
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

/// What a person gets who opens `bytes`, selects the whole of page one and presses copy: the
/// logical order where the core gave one, and the page order beside it.
fn copy_all(bytes: Vec<u8>) -> (Option<String>, String) {
    let mut viewer = Viewer::new(800, 1000, 1.0);
    let events: Vec<Event> = viewer
        .handle(Command::Open {
            id: DOCUMENT,
            bytes: bytes.into(),
            password: None,
            fragment: None,
        })
        .collect();
    let request = events
        .iter()
        .find_map(|event| match event {
            Event::NeedsRender(request) => Some(request.clone()),
            _ => None,
        })
        .expect("a render was asked for");
    let raster = CpuRasterizer::new()
        .rasterize(&request.list, request.target)
        .expect("the CPU backend draws this page");
    viewer
        .handle(Command::RenderReady {
            token: request.token,
            rendered: Rendered::Raster(raster),
        })
        .for_each(drop);
    viewer
        .handle(Command::Select(Selection::All))
        .for_each(drop);
    viewer
        .handle(Command::Copy)
        .find_map(|event| match event {
            Event::Copied {
                logical,
                page_order,
                ..
            } => Some((logical, page_order)),
            _ => None,
        })
        .expect("a page with text selected copies")
}

/// The page every test below reads, whose stream order is not its logical order.
///
/// Drawn: a running head tagged `/Artifact` with Table 363's `/Pagination`, the *second*
/// paragraph, the first paragraph — whose last word is split across two sequences, the second a
/// `Span` — a two-by-two table, and a folio tagged with EXAMPLE 1's generic `/Artifact BMC`.
/// Stated: a `Document` holding the two paragraphs in the other order and then the table.
const PAGE: &str = "/Artifact << /Type /Pagination /Subtype /Header >> BDC \
     BT /F1 10 Tf 20 280 Td (Running head) Tj ET EMC \
     BT /F1 12 Tf 20 200 Td /P << /MCID 0 >> BDC (Second paragraph.) Tj EMC ET \
     BT /F1 12 Tf 20 240 Td /P << /MCID 1 >> BDC (First para) Tj EMC \
     /Span << /MCID 2 >> BDC (graph.) Tj EMC ET \
     BT /F1 12 Tf 20 160 Td /TD << /MCID 3 >> BDC (A1) Tj EMC 60 0 Td \
     /TD << /MCID 4 >> BDC (B1) Tj EMC ET \
     BT /F1 12 Tf 20 140 Td /TD << /MCID 5 >> BDC (A2) Tj EMC 60 0 Td \
     /TD << /MCID 6 >> BDC (B2) Tj EMC ET \
     /Artifact BMC BT /F1 10 Tf 140 20 Td (7) Tj ET EMC";

/// The structure [`PAGE`] states, `extra` spliced into the `Document`'s `/K` after the table.
fn structure(extra: &str, objects: &str) -> String {
    format!(
        "10 0 obj\n<< /Type /StructTreeRoot /K [11 0 R] /ParentTree 12 0 R >>\nendobj\n\
         11 0 obj\n<< /Type /StructElem /S /Document /P 10 0 R /K [13 0 R 15 0 R 16 0 R {extra}] \
         >>\nendobj\n\
         12 0 obj\n<< /Nums [0 [15 0 R 13 0 R 14 0 R 19 0 R 20 0 R 21 0 R 22 0 R]] >>\nendobj\n\
         13 0 obj\n<< /Type /StructElem /S /P /P 11 0 R /Pg 3 0 R /K [1 14 0 R] >>\nendobj\n\
         14 0 obj\n<< /Type /StructElem /S /Span /P 13 0 R /Pg 3 0 R /K 2 >>\nendobj\n\
         15 0 obj\n<< /Type /StructElem /S /P /P 11 0 R /Pg 3 0 R /K 0 >>\nendobj\n\
         16 0 obj\n<< /Type /StructElem /S /Table /P 11 0 R /K [17 0 R 18 0 R] >>\nendobj\n\
         17 0 obj\n<< /Type /StructElem /S /TR /P 16 0 R /K [19 0 R 20 0 R] >>\nendobj\n\
         18 0 obj\n<< /Type /StructElem /S /TR /P 16 0 R /K [21 0 R 22 0 R] >>\nendobj\n\
         19 0 obj\n<< /Type /StructElem /S /TD /P 17 0 R /Pg 3 0 R /K 3 >>\nendobj\n\
         20 0 obj\n<< /Type /StructElem /S /TD /P 17 0 R /Pg 3 0 R /K 4 >>\nendobj\n\
         21 0 obj\n<< /Type /StructElem /S /TD /P 18 0 R /Pg 3 0 R /K 5 >>\nendobj\n\
         22 0 obj\n<< /Type /StructElem /S /TD /P 18 0 R /Pg 3 0 R /K 6 >>\nendobj\n\
         {objects}"
    )
}

/// The four rules at once, on a page that states all four.
///
/// - §14.8.2.5.1: "[l]ogical content order -the ordering for semantic purposes -shall be defined
///   by a depth-first traversal of the document's logical structure hierarch y", so the first
///   paragraph comes first although the stream drew it second.
/// - §14.8.3.3: "BLSEs shall be stacked within a reference area in block-progression order", so
///   each paragraph and each table row begins a line — and Table 371's cells are cells, a tab
///   apart.
/// - §14.8.2.2.2's two marked-content artifacts, the running head and the folio, are not in it.
/// - §14.8.4.7.1 makes the `Span` part of its paragraph's content, so the word it divides is one
///   word.
#[test]
fn a_tagged_page_copies_in_its_structure_s_order_with_its_breaks_and_without_its_artifacts() {
    let (logical, page_order) = copy_all(tagged(PAGE, &structure("", "")));
    assert!(
        page_order.contains("Running head") && page_order.contains('7'),
        "the readback keeps what the page drew, artifacts included: {page_order:?}"
    );
    assert_eq!(
        logical.as_deref(),
        Some("First paragraph.\nSecond paragraph.\nA1\tB1\nA2\tB2"),
        "page order was {page_order:?}"
    );
}

/// §14.8.2.2.1's element form: "[w]here artifacts are to be included in the structure tree, they
/// shall be included through the Artifact structure element type, and shall not be considered real
/// content."
///
/// A line number drawn in the stream and reached by the tree through an `Artifact` element — the
/// clause's NOTE 1 names line numbers as the content this form is for — is accounted for, so the
/// copy is still in logical order, and is not in it.
#[test]
fn content_under_an_artifact_element_is_accounted_for_and_left_out() {
    let page = format!("{PAGE} BT /F1 8 Tf 5 240 Td /Span << /MCID 7 >> BDC (12) Tj EMC ET");
    let structure = structure(
        "23 0 R",
        "23 0 obj\n<< /Type /StructElem /S /Artifact /P 11 0 R /Pg 3 0 R /K 7 >>\nendobj\n",
    )
    .replace("22 0 R]] >>", "22 0 R 23 0 R]] >>");
    let (logical, page_order) = copy_all(tagged(&page, &structure));
    assert!(page_order.contains("12"), "{page_order:?}");
    assert_eq!(
        logical.as_deref(),
        Some("First paragraph.\nSecond paragraph.\nA1\tB1\nA2\tB2")
    );
}

/// Text the tree does not reach and nobody declared an artifact keeps the whole copy in page
/// order: §14.8.2.2.2's "[a]ny content that is not included in the structure tree is an artifact"
/// is this reader's inference from an absence, and a copy that acted on it could lose a paragraph
/// a producer's tree forgot.
#[test]
fn text_nothing_reaches_keeps_the_copy_in_page_order() {
    let page = format!("{PAGE} BT /F1 12 Tf 20 100 Td (Forgotten.) Tj ET");
    let (logical, page_order) = copy_all(tagged(&page, &structure("", "")));
    assert!(page_order.contains("Forgotten."), "{page_order:?}");
    assert_eq!(
        logical, None,
        "nothing claims the last line, so no order is claimed"
    );
}

/// The structure's half of the first test, asked of `pdf-model` directly: the pieces in the
/// tree's order, the boundary before each, and a range that is artifacts and nothing else.
///
/// The last is `None` rather than an empty copy: a person who dragged across the running head
/// and pressed copy asked for the running head, and the page order gives it to them.
#[test]
fn the_structure_states_the_pieces_and_what_lies_between_them() {
    use pdf_model::structure::{Boundary, Tree};

    let document = pdf_syntax::Document::open(tagged(PAGE, &structure("", "")))
        .expect("the fixture is a valid file");
    let pages = pdf_model::Pages::new(&document);
    let interpretation = pdf_model::interpret(&document, &pages.get(0).expect("one page"));
    let tree = Tree::of(&document).expect("the fixture states a structure tree");
    let page = pdf_syntax::ObjectId {
        number: 3,
        generation: 0,
    };
    let text = &interpretation.text;
    let pieces = tree
        .logical_range(
            &document,
            page,
            &interpretation.marked,
            &interpretation.artifacts,
            0..text.len(),
        )
        .expect("every byte is reached or declared an artifact");
    let read: Vec<(&str, Boundary)> = pieces
        .iter()
        .map(|piece| {
            (
                text.get(piece.range.clone()).unwrap_or_default().trim(),
                piece.after,
            )
        })
        .collect();
    assert_eq!(
        read,
        vec![
            ("First para", Boundary::Flow),
            ("graph.", Boundary::Flow),
            ("Second paragraph.", Boundary::Block),
            ("A1", Boundary::Block),
            ("B1", Boundary::Cell),
            ("A2", Boundary::Block),
            ("B2", Boundary::Cell),
        ]
    );

    let head = text.find("Running head").expect("the stream drew it");
    assert_eq!(
        tree.logical_range(
            &document,
            page,
            &interpretation.marked,
            &interpretation.artifacts,
            head..head + "Running head".len(),
        ),
        None
    );
}

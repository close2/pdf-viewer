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
use viewer_core::{Answer, Command, DocumentId, Event, Query, Rendered, Selection, Viewer};

/// The document every test here opens.
const DOCUMENT: DocumentId = DocumentId(1);

/// A one-page tagged document: `content` as the page's stream, `structure` as objects 10 onwards
/// with object 10 the `/StructTreeRoot`.
fn tagged(content: &str, structure: &str) -> Vec<u8> {
    tagged_page(content, "", structure)
}

/// [`tagged`], with `page` spliced into the page dictionary — an `/Annots` array.
fn tagged_page(content: &str, page: &str, structure: &str) -> Vec<u8> {
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R /StructTreeRoot 10 0 R \
         /MarkInfo << /Marked true >> >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 300 300] \
         /Resources << /Font << /F1 5 0 R >> >> /Contents 4 0 R /StructParents 0 {page} >>\nendobj\n\
         4 0 obj\n<< /Length {} >>\nstream\n{content}\nendstream\nendobj\n\
         5 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica \
         /Encoding << /Differences [173 /softhyphen] >> >>\nendobj\n\
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

/// A viewer that has opened `bytes` and drawn page one, which is when its readback exists.
fn drawn(bytes: Vec<u8>) -> Viewer {
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
}

/// What a person gets who opens `bytes`, selects the whole of page one and presses copy: the
/// logical order where the core gave one, and the page order beside it.
fn copy_all(bytes: Vec<u8>) -> (Option<String>, String) {
    let mut viewer = drawn(bytes);
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
    use pdf_model::structure::{Boundary, ContentSpans, Tree};

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
            ContentSpans::of(&interpretation),
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
            ContentSpans::of(&interpretation),
            head..head + "Running head".len(),
        ),
        None
    );
}

/// [`PAGE`] with a text field between its two paragraphs, which the tree names by §14.7.5.3's
/// object reference `times` times over: a `P` holding a `Form` whose `/K` is the `OBJR`, and the
/// widget's `/StructParent` keyed to that `Form` (Table 359).
fn with_a_field(times: usize) -> Vec<u8> {
    let reference = "<< /Type /OBJR /Obj 25 0 R /Pg 3 0 R >> ".repeat(times);
    let appearance = "BT /F1 12 Tf 2 5 Td (Field value.) Tj ET";
    let structure = structure(
        "",
        &format!(
            "24 0 obj\n<< /Type /StructElem /S /Form /P 26 0 R /K [{reference}] >>\nendobj\n\
             25 0 obj\n<< /Type /Annot /Subtype /Widget /FT /Tx /T (f) /V (Field value.) \
             /Rect [20 100 200 120] /F 4 /P 3 0 R /StructParent 1 /AP << /N 27 0 R >> >>\n\
             endobj\n\
             26 0 obj\n<< /Type /StructElem /S /P /P 11 0 R /K [24 0 R] >>\nendobj\n\
             27 0 obj\n<< /Type /XObject /Subtype /Form /BBox [0 0 180 20] \
             /Resources << /Font << /F1 5 0 R >> >> /Length {} >>\nstream\n{appearance}\n\
             endstream\nendobj\n",
            appearance.len() + 1
        ),
    )
    .replace("/K [13 0 R 15 0 R", "/K [13 0 R 26 0 R 15 0 R")
    .replace("22 0 R]] >>", "22 0 R] 1 24 0 R] >>");
    tagged_page(PAGE, "/Annots [25 0 R]", &structure)
}

/// §14.8.2.5.2: an annotation "is not interleaved within the page's content stream", and "[t]he
/// position of an annotation in the logical content order is determined from the document's
/// logical structure".
///
/// The field's value reads back after everything the page's stream drew, and the copy puts it
/// where the tree's object reference does: between the two paragraphs, a block of its own because
/// its `P` is one. Without the object's span nothing would account for those bytes, and the copy
/// would be in page order (ADR 1970).
#[test]
fn an_annotation_the_tree_names_is_copied_where_the_tree_names_it() {
    let (logical, page_order) = copy_all(with_a_field(1));
    assert!(
        page_order.trim_end().ends_with("Field value."),
        "the appearance reads back after the page's content: {page_order:?}"
    );
    assert_eq!(
        logical.as_deref(),
        Some("First paragraph.\nField value.\nSecond paragraph.\nA1\tB1\nA2\tB2"),
        "page order was {page_order:?}"
    );
}

/// Table 358's NOTE 2: "if it is rendered multiple times on the same page, just a single object
/// reference suffices to identify all of them" — so a second reference to the same annotation
/// names the same content, and the copy holds it once.
#[test]
fn an_annotation_named_twice_is_copied_once() {
    let (logical, _) = copy_all(with_a_field(2));
    assert_eq!(
        logical.as_deref(),
        Some("First paragraph.\nField value.\nSecond paragraph.\nA1\tB1\nA2\tB2")
    );
}

/// [`PAGE`] with a `Private` element after the table: a two-cell table of its own on one line, and
/// a paragraph on the next.
///
/// Drawn by the stream after everything [`PAGE`] draws, and stated by the tree after the table, so
/// both orders put it last.
fn with_a_private_element() -> Vec<u8> {
    let page = format!(
        "{PAGE} BT /F1 12 Tf 20 100 Td /TD << /MCID 7 >> BDC (C1) Tj EMC 60 0 Td \
         /TD << /MCID 8 >> BDC (D1) Tj EMC ET \
         BT /F1 12 Tf 20 80 Td /P << /MCID 9 >> BDC (Kept.) Tj EMC ET"
    );
    let structure = structure(
        "23 0 R",
        "23 0 obj\n<< /Type /StructElem /S /Private /P 11 0 R /K [24 0 R 28 0 R] >>\nendobj\n\
         24 0 obj\n<< /Type /StructElem /S /Table /P 23 0 R /K [25 0 R] >>\nendobj\n\
         25 0 obj\n<< /Type /StructElem /S /TR /P 24 0 R /K [26 0 R 27 0 R] >>\nendobj\n\
         26 0 obj\n<< /Type /StructElem /S /TD /P 25 0 R /Pg 3 0 R /K 7 >>\nendobj\n\
         27 0 obj\n<< /Type /StructElem /S /TD /P 25 0 R /Pg 3 0 R /K 8 >>\nendobj\n\
         28 0 obj\n<< /Type /StructElem /S /P /P 23 0 R /Pg 3 0 R /K 9 >>\nendobj\n",
    )
    .replace("22 0 R]] >>", "22 0 R 26 0 R 27 0 R 28 0 R]] >>");
    tagged(&page, &structure)
}

/// ISO 32000-1 Table 333's `Private`: "[n]either the Private element nor any of its descendants
/// shall be interpreted or exported to other document formats" — and a copy is not the second,
/// because §14.8.1's NOTE names "[e]xtraction of text and graphics for pasting into other
/// applications" and "[c]onversion to other common file formats" as two things (ADR 2042).
///
/// So the subtree's text is in the copy, where the traversal reaches it and as the page order has
/// it, and none of its types is read: the table inside it states no cell's end and no row's, and
/// the paragraph no block's, so the readback's own separators stand between them. The table
/// *before* it is outside it and still ends as a table does.
#[test]
fn a_private_element_is_copied_and_its_types_are_not_read() {
    use pdf_model::structure::{Boundary, ContentSpans, Tree};

    let bytes = with_a_private_element();
    let (logical, page_order) = copy_all(bytes.clone());
    assert!(
        page_order.contains("C1") && page_order.contains("Kept."),
        "{page_order:?}"
    );
    assert_eq!(
        logical.as_deref(),
        Some("First paragraph.\nSecond paragraph.\nA1\tB1\nA2\tB2\nC1 D1\nKept."),
        "the outer table ends as a table, and the cells inside the `Private` element are the \
         readback's: page order was {page_order:?}"
    );

    let document = pdf_syntax::Document::open(bytes).expect("the fixture is a valid file");
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
            ContentSpans::of(&interpretation),
            0..text.len(),
        )
        .expect("every byte is reached or declared an artifact");
    let private: Vec<(&str, Boundary)> = pieces
        .iter()
        .skip(7)
        .map(|piece| {
            (
                text.get(piece.range.clone()).unwrap_or_default().trim(),
                piece.after,
            )
        })
        .collect();
    assert_eq!(
        private,
        vec![
            ("C1", Boundary::Block),
            ("D1", Boundary::Flow),
            ("Kept.", Boundary::Flow),
        ],
        "the block before the subtree is the outer table's end; nothing inside it is a boundary"
    );
}

/// §14.8.2.3's soft hyphen at a line's end, in both of a copy's orders: the break after it is the
/// incidental division and goes, the character is what the producer mapped and stays, so the
/// pasted word is the word a search finds (ADR 1971). The paragraph is one element whose content
/// the stream divided across two lines; code 173 is the `/Differences` array's `softhyphen`, which
/// the Adobe Glyph List names U+00AD — Annex D's own route for it.
#[test]
fn a_word_a_soft_hyphen_divided_is_copied_whole_in_both_orders() {
    let page = format!(
        "{PAGE} BT /F1 12 Tf 20 100 Td /P << /MCID 7 >> BDC (Transparen\\255) Tj 0 -14 Td \
         (cy matters.) Tj EMC ET"
    );
    let structure = structure(
        "23 0 R",
        "23 0 obj\n<< /Type /StructElem /S /P /P 11 0 R /Pg 3 0 R /K 7 >>\nendobj\n",
    )
    .replace("22 0 R]] >>", "22 0 R 23 0 R]] >>");
    let (logical, page_order) = copy_all(tagged(&page, &structure));
    assert!(
        page_order.ends_with("Transparen\u{ad}cy matters."),
        "{page_order:?}"
    );
    assert_eq!(
        logical.as_deref(),
        Some("First paragraph.\nSecond paragraph.\nA1\tB1\nA2\tB2\nTransparen\u{ad}cy matters.")
    );
}

/// §14.9.4's own EXAMPLE, read aloud: a replacement is spoken once, however many codes it covers.
///
/// > The ActualText value shall be used as a replacement, not a description, for the content,
/// > providing text that is equivalent to what a person would see when viewing the content.
///
/// The section `/Span <</ActualText (c)>> BDC (k-) Tj EMC` shows two codes and is replaced by one
/// character, so the paragraph's first line holds `Der Druc` — the readback's own bytes for the
/// codes on it, which is `TextLine`'s rule: its text is exactly its characters' readback joined.
/// Each code inside the section reads back as the whole replacement (ADR 1515), so a line that
/// spoke each code's readback said the `c` twice. The replacement is one character whose box
/// covers both glyphs (ADR 2086). The page is the one `doc/teaching/how-text-is-selected-and-copied.md`
/// walks through.
#[test]
fn a_replacement_over_two_codes_is_one_character_of_its_line() {
    let content = "/P <</MCID 0>> BDC BT /F1 12 Tf 14 TL 20 240 Td \
         (Der Dru) Tj /Span <</ActualText (c)>> BDC (k-) Tj EMC \
         (ker) ' [-250 (druckt eine Seiten\\255)] TJ (zahl.) ' ET EMC \
         /H1 <</MCID 1>> BDC BT /F1 16 Tf 20 270 Td (Drucken) Tj ET EMC \
         /Artifact BMC BT /F1 10 Tf 140 20 Td (1) Tj ET EMC";
    let structure = "10 0 obj\n<< /Type /StructTreeRoot /K [11 0 R] /ParentTree 12 0 R >>\nendobj\n\
         11 0 obj\n<< /Type /StructElem /S /Document /P 10 0 R /K [13 0 R 14 0 R] >>\nendobj\n\
         12 0 obj\n<< /Nums [0 [14 0 R 13 0 R]] >>\nendobj\n\
         13 0 obj\n<< /Type /StructElem /S /H1 /P 11 0 R /Pg 3 0 R /K 1 >>\nendobj\n\
         14 0 obj\n<< /Type /StructElem /S /P /P 11 0 R /Pg 3 0 R /K 0 >>\nendobj\n";
    let viewer = drawn(tagged(content, structure));
    let Answer::Accessibility(pages) = viewer.query(Query::AccessibilityTree) else {
        panic!("an open document answers the question");
    };
    let paragraph = pages
        .first()
        .and_then(|page| page.nodes.iter().find(|node| node.role == "P"))
        .expect("the paragraph is in the tree");
    let first = paragraph.lines.first().expect("the paragraph has lines");
    assert_eq!(first.text, "Der Druc", "{:?}", paragraph.lines);
    assert_eq!(
        first.characters.len(),
        "Der Druc".chars().count(),
        "one character per code outside the section, and one for the section: {first:?}"
    );
    let replaced = first.characters.last().expect("the line has characters");
    let before = first
        .characters
        .get(first.characters.len() - 2)
        .expect("and more than one");
    assert!(
        replaced.bounds[2] - replaced.bounds[0] > before.bounds[2] - before.bounds[0],
        "the replacement's box covers both of its glyphs, `k` and `-`, which together are wider \
         than the `u` before them: {first:?}"
    );
}

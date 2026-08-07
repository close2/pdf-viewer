//! Where each character the page shows actually is.
//!
//! `Interpretation::text_layer` is the geometry `Interpretation::text` does not have: one entry
//! per character code, carrying the range of the readback it produced and the quadrilateral its
//! glyph occupies. Nothing in ISO 32000-2 asks for it — selecting text is not something the
//! standard describes — so what can be checked is not a clause but a set of invariants, and the
//! invariants are what a defect in this would break.
//!
//! Four things are asserted here, and each of them fails against a different mistake:
//!
//! - **the spans tile the readback in order**, which fails if a code's text is recorded against
//!   the wrong code, or if §14.8.2.5.3's reversal pairs a piece with another glyph's box;
//! - **the boxes are on the page**, which fails if the text rendering matrix is composed in the
//!   wrong order;
//! - **a line of horizontal text advances along x and shares its y**, which fails if the box is
//!   built in the wrong space — glyph space rather than text space, most obviously;
//! - **the invisible text of an OCR layer is placed too**, which is the case the layer exists
//!   for and the one a "draw it and remember where" implementation would miss.

#![expect(
    clippy::panic,
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "test code: a fixture that cannot exercise the rule must fail loudly rather than \
              pass by doing nothing"
)]

use std::path::{Path, PathBuf};

use pdf_model::content::{Placed, interpret};
use pdf_model::{Interpretation, Pages};
use pdf_syntax::Document;

/// A document committed in `doc/`, which every checkout has.
fn committed(name: &str) -> Document {
    let path: PathBuf = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../doc")
        .join(name);
    let bytes =
        std::fs::read(&path).unwrap_or_else(|e| panic!("{} is committed: {e}", path.display()));
    Document::open(bytes).unwrap()
}

/// A corpus document, or `None` when the submodule is not checked out.
fn corpus(name: &str) -> Option<Document> {
    let path: PathBuf = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../doc/pdf.js/test/pdfs")
        .join(name);
    Document::open(std::fs::read(path).ok()?).ok()
}

/// Interprets a page of a document.
fn page(document: &Document, index: usize) -> Interpretation {
    let pages = Pages::new(document);
    let page = pages.get(index).expect("the page exists");
    interpret(document, &page)
}

/// The mid-point of a placement's box.
fn centre(placed: &Placed) -> (f32, f32) {
    let quad = placed.quad;
    (
        f32::midpoint(
            f32::midpoint(quad[0], quad[2]),
            f32::midpoint(quad[4], quad[6]),
        ),
        f32::midpoint(
            f32::midpoint(quad[1], quad[3]),
            f32::midpoint(quad[5], quad[7]),
        ),
    )
}

#[test]
fn every_span_is_in_order_and_inside_the_readback() {
    // The readback and the layer are built by one pass over one string of codes, so a span that
    // runs backwards, overlaps its neighbour or points past the end is a pairing that has come
    // apart — which is precisely what §14.8.2.5.3's reversed strings could do and what nothing
    // else in the tree would notice.
    for name in [
        "PDF20_AN001-BPC.pdf",
        "PDF20_AN002-AF.pdf",
        "Tagged-PDF-Best-Practice-Guide.pdf",
    ] {
        let document = committed(name);
        let interpretation = page(&document, 0);
        assert!(!interpretation.text_layer.is_empty(), "{name} shows text");
        let mut previous = 0;
        for placed in &interpretation.text_layer {
            assert!(placed.span.start >= previous, "{name}: out of order");
            assert!(placed.span.end >= placed.span.start, "{name}: backwards");
            assert!(
                interpretation.text.get(placed.span.clone()).is_some(),
                "{name}: {:?} is not a character boundary of the readback",
                placed.span
            );
            previous = placed.span.end;
        }
        assert!(previous <= interpretation.text.len());
    }
}

#[test]
fn the_boxes_are_on_the_page_and_read_left_to_right() {
    // The specification note is set in horizontal writing on an A4 page, so two things must be
    // true of every run of it: the glyphs are inside the page, and consecutive glyphs of one
    // word advance along x at a constant y. A box built in the wrong space — or a text rendering
    // matrix composed the wrong way round — breaks both.
    let document = committed("PDF20_AN001-BPC.pdf");
    let interpretation = page(&document, 0);
    let size = interpretation.display_list.page_size;

    for placed in &interpretation.text_layer {
        let (x, y) = centre(placed);
        assert!(
            (0.0..=size.width).contains(&x) && (0.0..=size.height).contains(&y),
            "{:?} is off a {size:?} page",
            centre(placed)
        );
    }

    // The first run of at least four consecutive placements whose readback is all alphabetic —
    // a word — and what it must look like.
    let word: Vec<&Placed> = interpretation
        .text_layer
        .windows(4)
        .find(|window| {
            window.iter().all(|placed| {
                interpretation.text[placed.span.clone()]
                    .chars()
                    .all(char::is_alphabetic)
                    && !placed.span.is_empty()
            })
        })
        .expect("the page has a word on it")
        .iter()
        .collect();
    for pair in word.windows(2) {
        let (first, second) = (centre(pair[0]), centre(pair[1]));
        assert!(second.0 > first.0, "a word advances: {first:?} {second:?}");
        assert!(
            (second.1 - first.1).abs() < 0.01,
            "on one line: {first:?} {second:?}"
        );
    }
}

#[test]
fn a_glyphs_box_is_as_wide_as_its_advance_and_as_tall_as_the_font() {
    // The box is the advance by the font's own reach above and below the baseline (Table 120's
    // `/Ascent` and `/Descent`), mapped by §9.4.4's text rendering matrix. On an unrotated page
    // at the default scale that makes each box exactly the glyph's advance wide, so the boxes of
    // a word abut: the right edge of one is the left edge of the next, with no gap and no
    // overlap. That is a stronger statement than "increasing x" and it is what a caret between
    // two characters needs.
    let document = committed("PDF20_AN001-BPC.pdf");
    let (mut abutting, mut comparable) = (0, 0);
    for index in 0..5 {
        let interpretation = page(&document, index);
        for pair in interpretation.text_layer.windows(2) {
            let (left, right) = (pair[0].quad, pair[1].quad);
            // Same line, same size: only then are the two boxes comparable.
            if (left[1] - right[1]).abs() > 0.01 || (left[5] - right[5]).abs() > 0.01 {
                continue;
            }
            comparable += 1;
            if (left[2] - right[0]).abs() < 0.01 {
                abutting += 1;
            }
        }
    }
    // **Not most of them**, and the number is worth writing down: 1485 of 5898 on this document,
    // because §9.3.2's character spacing and a `TJ` array's own offsets both move the next glyph
    // and this producer uses both heavily. What makes the assertion discriminating is the
    // counterfactual, which was measured rather than assumed: giving every box the em width
    // instead of the glyph's advance takes it to **0 of 5898**.
    println!("{abutting} of {comparable} consecutive pairs abut");
    assert!(comparable > 4000, "enough pairs to mean something");
    assert!(
        abutting > 1000,
        "{abutting} of {comparable} pairs abut; the em-box mistake gives none at all"
    );
}

#[test]
fn invisible_text_is_placed_too() {
    // Rendering mode 3 draws nothing, and an OCR layer under a scanned page is nothing but mode
    // 3 — which is the text a person most wants to select. `issue1155.pdf` is such a page.
    let Some(document) = corpus("issue1155.pdf") else {
        eprintln!("skipped: doc/pdf.js is not checked out");
        return;
    };
    let interpretation = page(&document, 0);
    assert!(
        !interpretation.text.is_empty(),
        "the page reads back as text"
    );
    assert_eq!(
        interpretation.glyphs, 0,
        "and marks the page with no glyphs at all"
    );
    assert!(
        !interpretation.text_layer.is_empty(),
        "so a layer built from what was *drawn* would be empty"
    );
    let size = interpretation.display_list.page_size;
    for placed in &interpretation.text_layer {
        let (x, y) = centre(placed);
        assert!(
            (0.0..=size.width).contains(&x) && (0.0..=size.height).contains(&y),
            "{:?} is off a {size:?} page",
            centre(placed)
        );
    }
}

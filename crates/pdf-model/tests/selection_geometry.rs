//! Where a selection highlight goes over a scanned page's OCR layer.
//!
//! # The blind spot this closes
//!
//! Two gates in this tree look at text and neither can see this. `tests/text_extraction.rs`
//! compares the *characters* a page reads back as and never asks where they are; the oracle
//! compares pixels, and an OCR layer under a scanned image is drawn in §9.3.6 Table 104's mode 3
//! and marks no pixel at all. So the quadrilateral a person's drag is turned into — the one thing
//! that decides whether the highlight lands on the words — was measured by nothing, and a wrong
//! one would have shipped without a number moving anywhere.
//!
//! # What is asserted
//!
//! The height of a selection box is not in the file. A text object states a baseline, a size, a
//! matrix, `Tz`, `Ts` and `TL`, and never a line box, so a viewer builds one from §9.8.1's Table
//! 120 — and the project owner's own report (ADR 0216) is of what other viewers do when that
//! table lies: no highlight appears, and the cursor has to be aimed half a line below the text.
//!
//! Four descriptors, each a shape the corpus actually holds (`font_metric_census`), and one
//! invariant that holds over all of them: **the box straddles the baseline and is about as tall
//! as the text is set**. Beside them, the invariant that makes a scanned page selectable at all:
//! Table 104's invisible modes place their glyphs like any other mode.

#![expect(
    clippy::float_cmp,
    reason = "the exact comparisons here are the assertion: an invisible mode must place a glyph \
              in the *same* place a visible one does, not in a nearby one"
)]
#![expect(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "test code: a malformed fixture must fail loudly, and this page is 200 units square \
              where no index can overflow"
)]

use std::fmt::Write as _;

use pdf_model::content::Placed;
use pdf_syntax::Document;

/// The font size every fixture here sets, in unscaled text space units.
const SIZE: f32 = 20.0;

/// The baseline every fixture here puts its text on, in default user space.
const BASELINE: f32 = 100.0;

/// A one-page fixture: one line of text on a known baseline, in a font whose descriptor states
/// exactly what the caller asks it to.
///
/// `descriptor` is spliced into the font descriptor, so a test writes `/Ascent 0 /Descent -205`
/// and gets a document that states it. The font is `/Helvetica`, which §9.6.2.2 makes a font
/// every processor has and which `pdf_font::standard` compiles in, so the metrics under test are
/// the document's rather than this machine's.
fn fixture(mode: u8, descriptor: &str) -> Vec<u8> {
    let content = format!("BT /F1 {SIZE} Tf {mode} Tr 20 {BASELINE} Td (Selectable text) Tj ET");
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] \
         /Resources << /Font << /F1 5 0 R >> >> /Contents 4 0 R >>\nendobj\n\
         4 0 obj\n<< /Length {} >>\nstream\n{content}\nendstream\nendobj\n\
         5 0 obj\n<< /Type /Font /Subtype /TrueType /BaseFont /Helvetica /FirstChar 32 \
         /LastChar 122 /Widths {} /FontDescriptor 6 0 R >>\nendobj\n\
         6 0 obj\n<< /Type /FontDescriptor /FontName /Helvetica /Flags 32 \
         /FontBBox [0 -200 1000 900] /ItalicAngle 0 /StemV 80 {descriptor} >>\nendobj\n",
        content.len() + 1,
        widths(),
    );
    assemble(&body)
}

/// Wraps a body of numbered objects in a header, a cross-reference table and a trailer.
fn assemble(body: &str) -> Vec<u8> {
    let mut out = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for object in body.split_inclusive("endobj\n") {
        offsets.push(out.len());
        out.push_str(object);
    }
    let xref_at = out.len();
    let size = offsets.len() + 1;
    let _ = writeln!(out, "xref\n0 {size}");
    out.push_str("0000000000 65535 f \n");
    for offset in &offsets {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    let _ = write!(
        out,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n"
    );
    out.into_bytes()
}

/// A `/Widths` covering codes 32 to 122, every glyph half an em wide.
///
/// §9.6.2's displacement comes from this array and never from the glyph program, so a fixed
/// value makes every box's *width* predictable while the tests are about its height.
fn widths() -> String {
    let mut out = String::from("[");
    for _ in 32..=122 {
        out.push_str("500 ");
    }
    out.push(']');
    out
}

/// Interprets one fixture's page.
fn page(mode: u8, descriptor: &str) -> pdf_model::Interpretation {
    let document = Document::open(fixture(mode, descriptor)).expect("the fixture is a valid PDF");
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    pdf_model::interpret(&document, &page)
}

/// The top and bottom edge of a placement's box, in default user space.
///
/// The fixture's text is unrotated, so the quadrilateral's first corner is its lower left and
/// its third its upper right (`content::glyph_quad`).
fn edges(placed: &Placed) -> (f32, f32) {
    (placed.quad[5], placed.quad[1])
}

/// Every placement of a page, insisting there is text to place.
fn placements(interpretation: &pdf_model::Interpretation) -> &[Placed] {
    assert!(
        !interpretation.text_layer.is_empty(),
        "the fixture shows text, so a layer built from it is not empty"
    );
    &interpretation.text_layer
}

/// A descriptor stating a face's real measurements, believed exactly as written.
///
/// The control for the three below: a band that threw away true statements would fail here, and
/// this is the case every well-made file in the corpus is in — 1320 of its 1629 font
/// dictionaries.
#[test]
fn a_measured_descriptor_is_believed_to_the_number() {
    for placed in placements(&page(0, "/Ascent 718 /Descent -207")) {
        let (top, bottom) = edges(placed);
        assert!(
            (top - (BASELINE + 0.718 * SIZE)).abs() < 0.01,
            "the file's own ascent places the top edge: {top}"
        );
        assert!(
            (bottom - (BASELINE - 0.207 * SIZE)).abs() < 0.01,
            "and its own descent the bottom: {bottom}"
        );
    }
}

/// Three descriptors no measurement of a face could produce, and the box each one gets.
///
/// Each row is a shape `font_metric_census` counts in `doc/pdf.js/test/pdfs`, and each was
/// **accepted** by the `ascent > descent` guard this replaced:
///
/// - `/Ascent 0 /Descent -205` is `zero_descent.pdf`, and it used to give a box entirely below
///   the baseline — the project owner's "the cursor had to be aimed half a line below the text";
/// - `/Ascent 8 /Descent -2` is `bug868745.pdf`, a face measured in some unit that is not
///   §9.2.4's, and it used to give a sliver a hundredth of the text's height;
/// - `/Ascent 4000 /Descent -1140` is `PDFJS-9279-reduced.pdf`, and it used to give a box five
///   ems tall, swallowing the lines above and below.
///
/// The assertion is one invariant rather than three expected numbers, because what a highlight
/// owes a person is not a particular height: it is to contain the text it is highlighting.
#[test]
fn a_descriptor_that_cannot_be_a_measurement_gets_the_em_box() {
    for descriptor in [
        "/Ascent 0 /Descent -205",
        "/Ascent 8 /Descent -2",
        "/Ascent 4000 /Descent -1140",
    ] {
        for placed in placements(&page(3, descriptor)) {
            let (top, bottom) = edges(placed);
            assert!(
                top >= BASELINE + 0.5 * SIZE,
                "{descriptor}: the box must reach the top of the glyphs, and reaches {top} \
                 above a baseline at {BASELINE}"
            );
            assert!(
                bottom <= BASELINE,
                "{descriptor}: the box must reach the baseline, and stops at {bottom}"
            );
            assert!(
                top - bottom <= 2.4 * SIZE,
                "{descriptor}: the box must not swallow the lines around it: {}",
                top - bottom
            );
        }
    }
}

/// A `/Descent` written without Table 120's sign is read as the depth it states.
///
/// `/Ascent 905 /Descent 211` is Arial's real metrics with the negative sign dropped, and it is
/// the corpus's commonest malformed shape: 42 font dictionaries in 23 documents. Read as written
/// it describes a font no glyph of which touches the baseline, and the highlight then floats
/// above the text. ADR 0216 argues why the magnitude is the measurement and the sign is the
/// convention.
#[test]
fn a_positive_descent_is_the_depth_it_states() {
    for placed in placements(&page(3, "/Ascent 905 /Descent 211")) {
        let (top, bottom) = edges(placed);
        assert!(
            (top - (BASELINE + 0.905 * SIZE)).abs() < 0.01,
            "the ascent is untouched: {top}"
        );
        assert!(
            (bottom - (BASELINE - 0.211 * SIZE)).abs() < 0.01,
            "and the descent is a depth below the baseline: {bottom}"
        );
    }
}

/// §9.3.6's invisible rendering modes place their glyphs like every other mode.
///
/// **This is what makes a scanned document selectable at all**, and until this test nothing in
/// the tree asserted it: an OCR layer is nothing but Table 104's mode 3, and a `text_layer` built
/// from what was *drawn* would be empty for every scanned page in the world. §9.3.6 states the
/// requirement for the position:
///
/// > The e and f components of Tm shall be updated for each glyph drawn when using text rendering
/// > mode 3 or 7 in exactly the same way as would be done for other text rendering modes.
///
/// So mode 3 and mode 7 are compared against mode 0 over the same string: the same readback, the
/// same number of placements, and every box in the same place to the last bit — while the display
/// list gets no glyph at all.
#[test]
fn invisible_modes_place_every_glyph_they_do_not_draw() {
    let visible = page(0, "/Ascent 718 /Descent -207");
    assert!(visible.glyphs > 0, "mode 0 draws the glyphs");
    for mode in [3, 7] {
        let invisible = page(mode, "/Ascent 718 /Descent -207");
        assert_eq!(
            invisible.glyphs, 0,
            "mode {mode} marks the page with no glyphs"
        );
        assert_eq!(
            invisible.text, visible.text,
            "mode {mode} reads back the same"
        );
        assert_eq!(
            invisible.text_layer.len(),
            visible.text_layer.len(),
            "mode {mode} places every code"
        );
        for (one, other) in invisible.text_layer.iter().zip(&visible.text_layer) {
            assert_eq!(one.span, other.span, "mode {mode}: the same readback range");
            assert_eq!(
                one.quad, other.quad,
                "mode {mode}: the same place on the page"
            );
        }
    }
}

/// A Type 3 font's box is the em box in *text* space, and the font matrix does not touch it.
///
/// ADR 0216 found this unverified, and it is the one case where getting the space wrong
/// would be invisible in the common file: §9.6.4's Table 110 NOTE calls `[0.001 0 0 0.001 0 0]`
/// "[a] common practice", and against that matrix a box put through it once too often is
/// indistinguishable from — no, is a thousandth of — the right answer only where the fixture
/// says so. This one states `[0.01 0 0 0.01 0 0]`, so the two readings differ by a hundred.
///
/// The answer is that the em box is already in text space, where §9.4.4 makes one unit the font
/// size, and the font matrix maps *glyph* space to text space (Table 110). Putting the box
/// through it would be converting a quantity that has already arrived. What does go through it
/// is the advance, because Table 110 says so of `/Widths` — "[t]hese widths shall be interpreted
/// in glyph space as specified by `FontMatrix` (unlike the widths of a Type 1 font, which are in
/// thousandths of a unit of text space)" — and that is asserted here beside the box, since a
/// test that checked only the height would pass on an implementation that put *neither* through
/// the matrix.
#[test]
fn a_type_3_fonts_box_is_the_em_box_in_text_space() {
    let document = Document::open(type3_fixture()).expect("the fixture is a valid PDF");
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    let interpretation = pdf_model::interpret(&document, &page);
    for placed in placements(&interpretation) {
        let (top, bottom) = edges(placed);
        assert!(
            (top - (BASELINE + SIZE)).abs() < 0.01,
            "one em above the baseline, not one em of glyph space: {top}"
        );
        assert!(
            (bottom - BASELINE).abs() < 0.01,
            "and the baseline: {bottom}"
        );
        assert!(
            (placed.quad[2] - placed.quad[0] - 0.5 * SIZE).abs() < 0.01,
            "the advance *is* mapped by /FontMatrix: 0.01 × 50 of an em"
        );
    }
}

/// A one-page fixture whose only font is a Type 3 with a font matrix that is not the common one.
///
/// The glyph is §9.6.4's own EXAMPLE square, scaled to the hundred-unit glyph space this matrix
/// implies, and `/Widths [50]` makes the advance half an em through it.
fn type3_fixture() -> Vec<u8> {
    const GLYPH: &str = "50 0 0 0 100 100 d1\n0 0 100 100 re f";
    let content = format!("BT /FT3 {SIZE} Tf 20 {BASELINE} Td (aaa) Tj ET");
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] \
         /Resources << /Font << /FT3 5 0 R >> >> /Contents 4 0 R >>\nendobj\n\
         4 0 obj\n<< /Length {} >>\nstream\n{content}\nendstream\nendobj\n\
         5 0 obj\n<< /Type /Font /Subtype /Type3 /FontBBox [0 0 100 100] \
         /FontMatrix [0.01 0 0 0.01 0 0] /CharProcs 7 0 R /Encoding 6 0 R \
         /FirstChar 97 /LastChar 97 /Widths [50] >>\nendobj\n\
         6 0 obj\n<< /Type /Encoding /Differences [97 /square] >>\nendobj\n\
         7 0 obj\n<< /square 8 0 R >>\nendobj\n\
         8 0 obj\n<< /Length {} >>\nstream\n{GLYPH}\nendstream\nendobj\n",
        content.len() + 1,
        GLYPH.len() + 1,
    );
    assemble(&body)
}

/// The boxes of one line abut, whatever the descriptor says about the line's height.
///
/// The horizontal half of the same question, and the reason ADR 0216 separates the two: a
/// wrong `/Widths` moves the glyphs *and* the boxes together, so they stay consistent, while a
/// wrong `/Ascent` moves only the box. Every glyph here is half an em wide by the fixture's own
/// `/Widths`, so consecutive boxes must meet exactly — which is what a caret between two
/// characters needs, and what a box built from the em rather than from the advance would break.
#[test]
fn consecutive_boxes_meet_at_the_advance() {
    let interpretation = page(3, "/Ascent 0 /Descent -205");
    for pair in placements(&interpretation).windows(2) {
        let (left, right) = (pair[0].quad, pair[1].quad);
        assert!(
            (left[2] - right[0]).abs() < 0.01,
            "the boxes of one line abut: {} then {}",
            left[2],
            right[0]
        );
        assert!(
            (right[2] - right[0] - 0.5 * SIZE).abs() < 0.01,
            "and each is the advance the file states wide"
        );
    }
}

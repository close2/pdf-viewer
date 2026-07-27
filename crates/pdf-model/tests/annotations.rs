//! Annotation appearance streams, checked by where their marks land.
//!
//! An appearance is a form `XObject`, so the question this file asks is never "did it
//! draw" — the interpreter has run forms since before annotations existed — but "did it
//! draw *there*". ISO 32000-2 §12.5.5 defines a matrix from the stream's `/BBox` and
//! `/Matrix` to the annotation's `/Rect`, and every way of getting that wrong leaves an
//! appearance on the page: scaled by the wrong factor, translated by its own offset, or
//! placed correctly at one size and wrongly at another. So the fixtures here deliberately
//! give `/BBox` and `/Rect` different origins *and* different sizes, which is the only
//! shape of test that can tell a correct placement from an accidentally-agreeing one.

#![expect(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "test code: a malformed fixture or an out-of-range pixel should fail loudly, \
              and the fixtures are small enough that no index can overflow"
)]

use std::fmt::Write as _;

use pdf_render::{Rasterizer, TargetSpec};
use pdf_syntax::Document;
use render_cpu::CpuRasterizer;

/// Pixel budget, far above the 100×100 pages these tests build.
const GENEROUS: u64 = 1 << 30;

/// Assembles a one-page PDF whose page carries one annotation.
///
/// The page's own content stream is empty, so every mark in the raster came from the
/// appearance and nothing has to be subtracted to see it.
fn pdf_with(annotation: &str, appearance_dict: &str, appearance: &str) -> Vec<u8> {
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] \
         /Resources << >> /Contents 4 0 R /Annots [5 0 R] >>\nendobj\n\
         4 0 obj\n<< /Length 0 >>\nstream\n\nendstream\nendobj\n\
         5 0 obj\n{annotation}\nendobj\n\
         6 0 obj\n<< /Type /XObject /Subtype /Form {appearance_dict} /Length {} >>\n\
         stream\n{appearance}\nendstream\nendobj\n",
        appearance.len().saturating_add(1)
    );

    let mut out = String::from("%PDF-1.7\n");
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
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n"
    );
    out.into_bytes()
}

/// Renders a fixture at one pixel per unit onto a transparent background.
fn render(bytes: Vec<u8>) -> pdf_render::Raster {
    let document = Document::open(bytes).expect("the fixture is a valid PDF");
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    let interpretation = pdf_model::interpret(&document, &page);
    assert!(
        interpretation.is_complete(),
        "the fixture should draw completely: {:?}",
        interpretation.unsupported
    );
    let list = interpretation.display_list;
    let target = TargetSpec::for_page(&list, 1.0, GENEROUS).expect("valid target");
    CpuRasterizer::new()
        .with_background(pdf_render::Color::TRANSPARENT)
        .rasterize(&list, target)
        .expect("supported")
}

/// Whether anything was painted at a point, given in PDF coordinates.
///
/// The raster's rows run downward and PDF's y runs upward, so the flip happens here once
/// rather than in every assertion, where it would be easy to get right by accident.
fn painted(raster: &pdf_render::Raster, x: u32, y: u32) -> bool {
    let row = raster.height.saturating_sub(1).saturating_sub(y);
    let at = ((row.saturating_mul(raster.width)).saturating_add(x) as usize).saturating_mul(4);
    raster.data[at + 3] > 0
}

/// Reports which of an annotation's edges the appearance actually reached.
fn extent(raster: &pdf_render::Raster) -> (u32, u32, u32, u32) {
    let (mut min_x, mut min_y, mut max_x, mut max_y) = (u32::MAX, u32::MAX, 0, 0);
    for y in 0..raster.height {
        for x in 0..raster.width {
            if painted(raster, x, y) {
                min_x = min_x.min(x);
                min_y = min_y.min(y);
                max_x = max_x.max(x);
                max_y = max_y.max(y);
            }
        }
    }
    (min_x, min_y, max_x, max_y)
}

/// A square annotation whose appearance fills its whole bounding box.
///
/// `/BBox` is `[0 0 10 10]` and `/Rect` is `[20 30 60 70]`, so a correct placement scales by
/// four and translates to (20, 30). Both numbers matter: a reader that ignores `/BBox` and
/// draws in the appearance's own units leaves a 10×10 mark at the origin, and one that
/// translates without scaling leaves a 10×10 mark at (20, 30). Neither survives this.
#[test]
fn an_appearance_is_scaled_and_translated_onto_the_annotation_rectangle() {
    let raster = render(pdf_with(
        "<< /Type /Annot /Subtype /Square /Rect [20 30 60 70] /F 4 /AP << /N 6 0 R >> >>",
        "/BBox [0 0 10 10]",
        "1 0 0 rg 0 0 10 10 re f",
    ));

    assert_eq!(extent(&raster), (20, 30, 59, 69));
    assert!(painted(&raster, 40, 50), "the middle of /Rect");
    assert!(!painted(&raster, 5, 5), "the appearance's own origin");
    assert!(!painted(&raster, 19, 50), "just left of /Rect");
    assert!(!painted(&raster, 60, 50), "just right of /Rect");
}

/// A `/BBox` away from the origin is measured, not just translated.
///
/// The appearance draws at (100, 200) in its own space and says so in its `/BBox`. Step 2
/// of the algorithm maps the box's *lower-left corner* onto the rectangle's, so the offset
/// cancels. A reader that treats `/BBox` as if it started at the origin puts this mark 100
/// units off the page.
#[test]
fn a_bounding_box_away_from_the_origin_is_brought_back_to_the_rectangle() {
    let raster = render(pdf_with(
        "<< /Type /Annot /Subtype /Square /Rect [10 10 50 50] /F 4 /AP << /N 6 0 R >> >>",
        "/BBox [100 200 120 220]",
        "0 0 1 rg 100 200 20 20 re f",
    ));

    assert_eq!(extent(&raster), (10, 10, 49, 49));
}

/// `/Matrix` is applied before the box is measured, and again to the content.
///
/// A 90° rotation turns a 20×10 box into a 10×20 one, so §12.5.5 step 1 measures 10×20 and
/// step 2 scales *that* onto the rectangle. A reader that measures the untransformed
/// `/BBox` gets the two axes' scales the wrong way round, which on a square `/Rect` — the
/// case most fixtures use — is invisible. This rectangle is deliberately not square.
#[test]
fn a_rotating_matrix_is_measured_before_the_scale_is_computed() {
    let raster = render(pdf_with(
        "<< /Type /Annot /Subtype /Square /Rect [0 0 30 60] /F 4 /AP << /N 6 0 R >> >>",
        // Rotates 90° anticlockwise, then shifts the result back into positive coordinates.
        "/BBox [0 0 20 10] /Matrix [0 1 -1 0 10 0]",
        "0 1 0 rg 0 0 20 10 re f",
    ));

    assert_eq!(extent(&raster), (0, 0, 29, 59));
}

/// An appearance drawing outside its `/BBox` is clipped to it.
///
/// §8.10.2 makes the bounding box the clip for a form `XObject`'s content, and §12.5.5's
/// whole algorithm assumes it: the box is what gets mapped onto `/Rect`, so content beyond
/// the box would land beyond the annotation. Here the stream fills the whole page in its
/// own coordinates and only the box's worth may survive.
#[test]
fn an_appearance_may_not_draw_outside_its_bounding_box() {
    let raster = render(pdf_with(
        "<< /Type /Annot /Subtype /Square /Rect [40 40 60 60] /F 4 /AP << /N 6 0 R >> >>",
        "/BBox [0 0 10 10]",
        "1 0 1 rg -100 -100 300 300 re f",
    ));

    assert_eq!(extent(&raster), (40, 40, 59, 59));
}

/// The `Hidden` and `NoView` flags mean nothing is drawn, and nothing is reported.
///
/// Table 167 bit 2 is "do not render the annotation ... regardless of its annotation type",
/// and bit 6 is "do not render the annotation on the screen". A viewer is a screen. Both are
/// the document's instruction rather than a gap, so neither may reach `unsupported` — an
/// annotation the file asked to hide is not something we failed to draw.
#[test]
fn the_hidden_and_no_view_flags_draw_nothing_and_report_nothing() {
    for flags in [2, 32, 34] {
        let bytes = pdf_with(
            &format!(
                "<< /Type /Annot /Subtype /Square /Rect [20 20 80 80] /F {flags} \
                 /AP << /N 6 0 R >> >>"
            ),
            "/BBox [0 0 10 10]",
            "1 0 0 rg 0 0 10 10 re f",
        );
        let document = Document::open(bytes).expect("the fixture is a valid PDF");
        let page = pdf_model::Pages::new(&document).get(0).expect("page one");
        let interpretation = pdf_model::interpret(&document, &page);
        assert!(
            interpretation.unsupported.is_empty(),
            "/F {flags} is an instruction, not a gap: {:?}",
            interpretation.unsupported
        );
        assert!(
            interpretation.display_list.commands().is_empty(),
            "/F {flags} must draw nothing"
        );
    }
}

/// `/AS` chooses among an appearance dictionary's states, and an unmatched one draws
/// nothing.
///
/// §12.5.5: where `/N` is a subdictionary rather than a stream, `/AS` names the state. The
/// clause also asks for "reasonable behaviour (such as displaying nothing)" where `/AS`
/// names a state with no appearance, which is what the second half checks — and it must
/// stay quiet, because a check box in its `Off` state having no `Off` appearance is a
/// document that is drawn correctly by drawing nothing.
#[test]
fn an_appearance_state_is_selected_by_as() {
    let on = render(pdf_with(
        "<< /Type /Annot /Subtype /Widget /Rect [20 20 60 60] /F 4 /AS /On \
         /AP << /N << /On 6 0 R >> >> >>",
        "/BBox [0 0 10 10]",
        "1 0 0 rg 0 0 10 10 re f",
    ));
    assert_eq!(extent(&on), (20, 20, 59, 59));

    let bytes = pdf_with(
        "<< /Type /Annot /Subtype /Widget /Rect [20 20 60 60] /F 4 /AS /Off \
         /AP << /N << /On 6 0 R >> >> >>",
        "/BBox [0 0 10 10]",
        "1 0 0 rg 0 0 10 10 re f",
    );
    let document = Document::open(bytes).expect("the fixture is a valid PDF");
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    let interpretation = pdf_model::interpret(&document, &page);
    assert!(
        interpretation.display_list.commands().is_empty(),
        "/AS naming an undefined state draws nothing"
    );
    assert!(
        interpretation.unsupported.is_empty(),
        "and says nothing: {:?}",
        interpretation.unsupported
    );
}

/// An annotation with no appearance stream is reported rather than invented.
///
/// Synthesising one from `/IC`, `/C` and `/BS` is a separate job — a different routine per
/// subtype — and drawing a guess would put a mark on the page the document never described.
/// This is the rule that keeps the corpus gate's counts meaningful.
#[test]
fn an_annotation_with_no_appearance_is_reported() {
    let bytes = pdf_with(
        "<< /Type /Annot /Subtype /Square /Rect [20 20 60 60] /F 4 /IC [1 0 0] >>",
        "/BBox [0 0 10 10]",
        "1 0 0 rg 0 0 10 10 re f",
    );
    let document = Document::open(bytes).expect("the fixture is a valid PDF");
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    let interpretation = pdf_model::interpret(&document, &page);
    assert!(
        interpretation.display_list.commands().is_empty(),
        "nothing may be invented for it"
    );
    assert!(
        !interpretation.is_complete(),
        "and its absence must be reported"
    );
}

/// `/Rect` written upper-right first is normalised rather than sent off the page.
///
/// §7.9.5 requires a processor to normalise a rectangle. Without it the scale computed in
/// step 2 comes out negative and the appearance is mirrored off the annotation entirely.
#[test]
fn a_reversed_rectangle_is_normalised() {
    let raster = render(pdf_with(
        "<< /Type /Annot /Subtype /Square /Rect [60 70 20 30] /F 4 /AP << /N 6 0 R >> >>",
        "/BBox [0 0 10 10]",
        "1 0 0 rg 0 0 10 10 re f",
    ));

    assert_eq!(extent(&raster), (20, 30, 59, 69));
}

/// Annotations draw over the page content, in `/Annots` order.
///
/// §12.5 puts them above the page; nothing in the content stream refers to them, so the
/// only thing fixing their order is that they come last.
#[test]
fn an_annotation_draws_over_the_page_content() {
    let bytes = pdf_with(
        "<< /Type /Annot /Subtype /Square /Rect [0 0 100 100] /F 4 /AP << /N 6 0 R >> >>",
        "/BBox [0 0 10 10]",
        "0 0 1 rg 0 0 10 10 re f",
    );
    // Replace the empty content stream with one that fills the page, keeping the object
    // numbering intact by writing the same object with a body.
    let bytes = String::from_utf8(bytes).expect("the fixture is ASCII");
    let content = "1 0 0 rg 0 0 100 100 re f";
    let bytes = bytes.replace(
        "4 0 obj\n<< /Length 0 >>\nstream\n\nendstream\nendobj\n",
        &format!(
            "4 0 obj\n<< /Length {} >>\nstream\n{content}\nendstream\nendobj\n",
            content.len().saturating_add(1)
        ),
    );
    // The cross-reference offsets are now stale, which `Document::open` recovers from by
    // scanning — the same path a damaged file takes, and one the corpus exercises daily.
    let document = Document::open(bytes.into_bytes()).expect("recoverable");
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    let list = pdf_model::interpret(&document, &page).display_list;
    let target = TargetSpec::for_page(&list, 1.0, GENEROUS).expect("valid target");
    let raster = CpuRasterizer::new()
        .rasterize(&list, target)
        .expect("supported");

    let at = ((50 * raster.width + 50) as usize).saturating_mul(4);
    assert_eq!(
        (raster.data[at], raster.data[at + 1], raster.data[at + 2]),
        (0, 0, 255),
        "the annotation's blue must cover the page's red"
    );
}

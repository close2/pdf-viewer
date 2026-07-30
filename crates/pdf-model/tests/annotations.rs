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

/// Interprets a fixture without demanding that it drew completely.
fn interpret(bytes: Vec<u8>) -> pdf_model::Interpretation {
    let document = Document::open(bytes).expect("the fixture is a valid PDF");
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    pdf_model::interpret(&document, &page)
}

/// The index of a pixel's first channel, given a point in PDF coordinates.
///
/// The raster's rows run downward and PDF's y runs upward, so the flip happens here once
/// rather than in every assertion, where it would be easy to get right by accident.
fn at(raster: &pdf_render::Raster, x: u32, y: u32) -> usize {
    let row = raster.height.saturating_sub(1).saturating_sub(y);
    ((row.saturating_mul(raster.width)).saturating_add(x) as usize).saturating_mul(4)
}

/// Whether anything was painted at a point, given in PDF coordinates.
fn painted(raster: &pdf_render::Raster, x: u32, y: u32) -> bool {
    raster.data[at(raster, x, y) + 3] > 0
}

/// A pixel's opacity, which is what a fixture testing `/ca` and `/CA` has to look at.
fn alpha_at(raster: &pdf_render::Raster, x: u32, y: u32) -> u8 {
    raster.data[at(raster, x, y) + 3]
}

/// A pixel's colour, ignoring its opacity.
fn colour_at(raster: &pdf_render::Raster, x: u32, y: u32) -> (u8, u8, u8) {
    let index = at(raster, x, y);
    (
        raster.data[index],
        raster.data[index + 1],
        raster.data[index + 2],
    )
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

/// An annotation whose clause states no appearance is reported rather than invented.
///
/// A `Text` annotation displays an *icon*, and §12.5.6.4 names the icons — `/Comment`,
/// `/Key`, `/Note` — without stating one line of their artwork. Drawing a guess would put a
/// mark on the page the document never described, so the report is the answer, and it is what
/// keeps the corpus gate's counts meaningful.
///
/// §12.5.6.10's four text markups were here until the thirty-fourth session, and the reason
/// they left is worth the distinction: that clause states the *mark* — "shall appear as
/// highlights, underlines, strikeouts … or jagged ('squiggly') underlines" — its region and
/// its orientation, and leaves only a thickness. An icon's clause states nothing at all.
#[test]
fn an_annotation_whose_appearance_is_not_stated_is_reported() {
    let interpretation = interpret(pdf_with(
        "<< /Type /Annot /Subtype /Text /Rect [20 20 60 60] /F 4 /C [1 1 0] \
         /Name /Comment >>",
        "/BBox [0 0 10 10]",
        "1 0 0 rg 0 0 10 10 re f",
    ));
    assert!(
        interpretation.display_list.commands().is_empty(),
        "nothing may be invented for it"
    );
    assert!(
        !interpretation.is_complete(),
        "and its absence must be reported"
    );
}

/// §12.5.6.10's four marks are constructed, and each is the mark its subtype's name is.
///
/// The clause states the kind, the region and the orientation and leaves a thickness, so
/// this checks the three things that distinguish the four rather than any dimension: a
/// highlight covers the whole quadrilateral, a strikeout crosses its middle, an underline and
/// a squiggle sit at its foot. See `appearance::text_markup` for what is a reading and what
/// is a choice.
#[test]
fn each_text_markup_draws_its_own_mark() {
    // A 40-unit quadrilateral, drawn over a red square, on a page this test reads back.
    let markup = |subtype: &str| {
        render(pdf_with(
            &format!(
                "<< /Type /Annot /Subtype /{subtype} /Rect [20 20 60 60] /F 4 /C [0 0 1] \
                 /QuadPoints [20 60 60 60 20 20 60 20] >>"
            ),
            "/BBox [0 0 10 10]",
            "1 0 0 rg 0 0 10 10 re f",
        ))
    };

    // The page is 100 by 100 at one pixel per unit and y-down in the raster, so the
    // quadrilateral occupies rows 40..80 and columns 20..60.
    let blue_rows = |raster: &pdf_render::Raster| -> Vec<u32> {
        (40..80)
            .filter(|row| {
                let at = ((row * raster.width + 40) * 4) as usize;
                raster.data[at + 2] > raster.data[at]
            })
            .collect()
    };

    let highlight = blue_rows(&markup("Highlight"));
    assert!(
        highlight.len() > 30,
        "a highlight covers its quadrilateral, not {} rows",
        highlight.len()
    );

    let strikeout = blue_rows(&markup("StrikeOut"));
    assert!(
        !strikeout.is_empty() && strikeout.len() < 10,
        "a strikeout is a bar, not {} rows",
        strikeout.len()
    );
    let middle = strikeout.iter().sum::<u32>() / u32::try_from(strikeout.len()).unwrap_or(1);
    assert!(
        (58..62).contains(&middle),
        "and it crosses the middle, not row {middle}"
    );

    for subtype in ["Underline", "Squiggly"] {
        let rows = blue_rows(&markup(subtype));
        assert!(!rows.is_empty(), "{subtype} draws nothing");
        let lowest = rows.iter().copied().max().unwrap_or(0);
        assert!(
            lowest > 74,
            "{subtype} sits at the foot of its quadrilateral, not at row {lowest}"
        );
    }
}

/// A subtype this crate knows nothing about still draws its normal appearance./// A subtype this crate knows nothing about still draws its normal appearance.
///
/// §12.5.5: "If a PDF processor does not have native support for a particular annotation type,
/// the PDF processor shall render the annotation with its normal (N) appearance." So the
/// placement path may not switch on `/Subtype` — and Table 171's list is consulted for exactly
/// one thing, the `Invisible` flag, whose own wording is conditional on it.
#[test]
fn an_unknown_subtype_still_draws_its_normal_appearance() {
    let raster = render(pdf_with(
        "<< /Type /Annot /Subtype /SomethingFromThePDF3Era /Rect [20 30 60 70] /F 4 \
         /AP << /N 6 0 R >> >>",
        "/BBox [0 0 10 10]",
        "1 0 0 rg 0 0 10 10 re f",
    ));

    assert_eq!(extent(&raster), (20, 30, 59, 69));
}

/// A square with no appearance stream is drawn from Table 180's `/IC`.
///
/// §12.5.6.8: the rectangle "shall be inscribed within the annotation rectangle defined by the
/// annotation dictionary's Rect entry". With no border to inset it, the fill reaches every edge
/// of `/Rect` and nothing outside it.
#[test]
fn a_square_with_no_appearance_is_drawn_from_its_interior_colour() {
    let raster = render(pdf_with(
        "<< /Type /Annot /Subtype /Square /Rect [20 30 60 70] /F 4 /IC [1 0 0] /Border [0 0 0] >>",
        "/BBox [0 0 10 10]",
        "",
    ));

    assert_eq!(extent(&raster), (20, 30, 59, 69));
    assert!(painted(&raster, 40, 50), "the middle of /Rect");
}

/// A circle is an ellipse inscribed in the rectangle, which its corners prove.
///
/// The distinction this test exists for is the one a square `/Rect` and a filled rectangle
/// cannot tell apart: an ellipse touches the middle of each edge and leaves all four corners
/// empty. A reader that drew `re` instead would fill them.
#[test]
fn a_circle_is_an_ellipse_inscribed_in_its_rectangle() {
    let raster = render(pdf_with(
        "<< /Type /Annot /Subtype /Circle /Rect [10 10 90 90] /F 4 /IC [0 0 1] /Border [0 0 0] >>",
        "/BBox [0 0 10 10]",
        "",
    ));

    assert!(painted(&raster, 50, 50), "the centre");
    assert!(painted(&raster, 50, 11), "the bottom of the ellipse");
    assert!(painted(&raster, 11, 50), "the left of the ellipse");
    for (x, y) in [(12, 12), (87, 12), (12, 87), (87, 87)] {
        assert!(
            !painted(&raster, x, y),
            "({x}, {y}) is a corner of /Rect, which an ellipse does not reach"
        );
    }
}

/// A link's border is §12.5.4's rectangle, in Table 166's `/C`, inside `/Rect`.
///
/// Table 166 makes `/C` "a colour used for ... The border of a link annotation" and §12.5.4
/// requires that "the border shall be drawn completely inside the annotation rectangle" — so a
/// four-unit border covers the rectangle's own edge and the four units within it, and leaves
/// the middle alone. A reader that centred the stroke on `/Rect` would paint two units outside
/// it; one that ignored the width would paint one.
#[test]
fn a_link_border_is_drawn_inside_its_rectangle_in_its_own_colour() {
    let raster = render(pdf_with(
        "<< /Type /Annot /Subtype /Link /Rect [20 20 80 60] /C [0 1 0] /Border [0 0 4] >>",
        "/BBox [0 0 10 10]",
        "",
    ));

    assert_eq!(extent(&raster), (20, 20, 79, 59));
    assert_eq!(colour_at(&raster, 50, 20), (0, 255, 0), "the bottom edge");
    assert!(
        painted(&raster, 50, 23),
        "the inner limit of a 4-unit border"
    );
    assert!(!painted(&raster, 50, 24), "just inside the border");
    assert!(!painted(&raster, 50, 40), "the middle of the annotation");
    assert!(!painted(&raster, 19, 40), "outside /Rect");
}

/// A zero-width border, or one with no colour, draws nothing and says nothing.
///
/// Table 166: "if the border width is 0, no border is drawn", and an empty `/C` is "No colour;
/// transparent". Both are the document specifying nothing rather than this crate failing —
/// which matters at the scale of the corpus, where most links are written `/Border [0 0 0]`.
#[test]
fn a_link_with_nothing_to_draw_reports_nothing() {
    for annotation in [
        "<< /Type /Annot /Subtype /Link /Rect [20 20 80 60] /C [0 1 0] /Border [0 0 0] >>",
        "<< /Type /Annot /Subtype /Link /Rect [20 20 80 60] /C [] /Border [0 0 4] >>",
        "<< /Type /Annot /Subtype /Link /Rect [20 20 80 60] /Border [0 0 4] >>",
        "<< /Type /Annot /Subtype /Link /Rect [20 20 80 60] /C [0 1 0] \
         /BS << /W 0 /S /S >> >>",
    ] {
        let interpretation = interpret(pdf_with(annotation, "/BBox [0 0 10 10]", ""));
        assert!(
            interpretation.display_list.commands().is_empty(),
            "{annotation} states no border: {:?}",
            interpretation.display_list.commands()
        );
        assert!(
            interpretation.unsupported.is_empty(),
            "{annotation} is not a gap: {:?}",
            interpretation.unsupported
        );
    }
}

/// A widget stating no background, no border and holding no value draws nothing, quietly.
///
/// This is the commonest widget in the corpus — an empty text field — and it is the reason
/// this decision matters more than any drawing routine: Table 192 is where a widget's
/// background and border come from, so a widget without one states no appearance at all, and
/// reporting it named 23 corpus documents for a gap that is not one.
#[test]
fn a_widget_stating_no_appearance_characteristics_draws_nothing_and_reports_nothing() {
    let interpretation = interpret(pdf_with(
        "<< /Type /Annot /Subtype /Widget /Rect [20 20 80 40] /F 4 /FT /Tx /T (name) \
         /DA (/Helv 9 Tf 0 g) >>",
        "/BBox [0 0 10 10]",
        "",
    ));
    assert!(
        interpretation.display_list.commands().is_empty(),
        "an empty field with no /MK draws nothing"
    );
    assert!(
        interpretation.unsupported.is_empty(),
        "and is not a gap: {:?}",
        interpretation.unsupported
    );
}

/// A widget with a background draws it *and* reports the value it cannot set.
///
/// Table 192's `/BG` is derivable and §12.7.4.3's variable text is not, so both statements are
/// made: the frame is on the page and the text is named as missing. Suppressing either would
/// lose information — the same pairing `/NeedAppearances` and `/Matte` already use.
#[test]
fn a_widget_draws_its_background_and_reports_its_field_value() {
    let interpretation = interpret(pdf_with(
        "<< /Type /Annot /Subtype /Widget /Rect [20 20 80 40] /F 4 /FT /Tx /T (name) \
         /V (Ada) /MK << /BG [0 0 1] >> >>",
        "/BBox [0 0 10 10]",
        "",
    ));
    assert!(
        !interpretation.display_list.commands().is_empty(),
        "the background is stated and must be drawn"
    );
    let reported = format!("{:?}", interpretation.unsupported);
    assert!(
        reported.contains("12.7.4.3"),
        "the value it cannot lay out must be named: {reported}"
    );
}

/// A polygon's `/IC` fills the shape; a polyline's does not.
///
/// Table 181 states the difference outright — for a polyline "the value of the IC key is used
/// to fill only the line ending", and this crate draws no line endings — so the same fixture
/// under two subtypes must differ in the middle of the shape. Nothing else in either clause
/// separates them.
#[test]
fn a_polygons_interior_colour_fills_it_and_a_polylines_does_not() {
    let vertices = "/Vertices [20 20 80 20 80 80 20 80]";
    let polygon = render(pdf_with(
        &format!(
            "<< /Type /Annot /Subtype /Polygon /Rect [10 10 90 90] /F 4 /IC [1 0 0] \
             /C [0 0 1] /BS << /W 1 >> {vertices} >>"
        ),
        "/BBox [0 0 10 10]",
        "",
    ));
    assert!(painted(&polygon, 50, 50), "a polygon's /IC fills it");

    let polyline = render(pdf_with(
        &format!(
            "<< /Type /Annot /Subtype /PolyLine /Rect [10 10 90 90] /F 4 /IC [1 0 0] \
             /C [0 0 1] /BS << /W 1 >> {vertices} >>"
        ),
        "/BBox [0 0 10 10]",
        "",
    ));
    assert!(
        !painted(&polyline, 50, 50),
        "a polyline's /IC is for its line endings, not its interior"
    );
    assert!(painted(&polyline, 50, 20), "and its line is still stroked");
}

/// An ink annotation is stroked along `/InkList`, one subpath per entry.
#[test]
fn an_ink_annotation_is_stroked_along_its_ink_list() {
    let raster = render(pdf_with(
        "<< /Type /Annot /Subtype /Ink /Rect [10 10 90 90] /F 4 /C [0 0 0] /BS << /W 2 >> \
         /InkList [[20 20 80 20] [20 80 80 80]] >>",
        "/BBox [0 0 10 10]",
        "",
    ));

    assert!(painted(&raster, 50, 20), "the first stroke");
    assert!(painted(&raster, 50, 80), "the second stroke");
    assert!(
        !painted(&raster, 50, 50),
        "and nothing between them: the subpaths are separate, not one polyline"
    );
}

/// A stored appearance stream states its own transparency; the annotation's `/CA` is ignored.
///
/// Table 166 of `/CA`: it "shall not be used if the annotation has an appearance stream ... in
/// that case, the appearance stream shall specify any transparency". §12.5.2 lists it among the
/// keys a reader "shall ignore" when an appearance dictionary is present. §12.5.5 says the
/// opposite in one sentence, and that is the reading this tree followed until the twenty-first
/// session; `highlight.pdf` shows why the other two win — it writes `/CA 0.8` *and* `ca 0.8`
/// inside its stream, so applying both would darken the highlight the producer specified.
///
/// **No corpus document can tell the two readings apart**, which was measured: every one of the
/// twelve that carries a `/CA` beside an appearance stream also sets its own alpha inside it, so
/// all 1794 oracle verdicts are identical either way. This test is the only thing in the tree
/// holding the clause to its words.
#[test]
fn a_stored_appearance_ignores_the_annotations_opacity() {
    let raster = render(pdf_with(
        "<< /Type /Annot /Subtype /Square /Rect [20 20 80 80] /F 4 /CA 0.5 \
         /AP << /N 6 0 R >> >>",
        "/BBox [0 0 10 10]",
        "1 0 0 rg 0 0 10 10 re f",
    ));

    assert_eq!(
        alpha_at(&raster, 50, 50),
        255,
        "the stream painted opaquely, so the annotation's /CA may not thin it"
    );
}

/// A constructed appearance *does* use `/ca` and `/CA`, which is what Table 166 defines them
/// for: the opacity "when regenerating the annotation's appearance stream".
///
/// The two are separate operations — `/ca` for nonstroking, `/CA` for stroking — and the
/// clause's fallback is one-directional: "If a ca entry is not present in this dictionary, then
/// the value of this CA entry shall also be used for nonstroking operations as well." So a
/// `/CA` alone thins a fill, and a `/ca` beside it overrides that fill without touching the
/// stroke.
#[test]
fn a_constructed_appearance_takes_its_opacity_from_the_annotation() {
    let from_stroking_alpha = render(pdf_with(
        "<< /Type /Annot /Subtype /Square /Rect [20 20 80 80] /F 4 /CA 0.5 /IC [1 0 0] \
         /Border [0 0 0] >>",
        "/BBox [0 0 10 10]",
        "",
    ));
    assert_eq!(
        alpha_at(&from_stroking_alpha, 50, 50),
        128,
        "/CA stands in for /ca when there is no /ca"
    );

    let with_own_alpha = render(pdf_with(
        "<< /Type /Annot /Subtype /Square /Rect [20 20 80 80] /F 4 /CA 0.5 /ca 0.25 \
         /IC [1 0 0] /Border [0 0 0] >>",
        "/BBox [0 0 10 10]",
        "",
    ));
    assert_eq!(
        alpha_at(&with_own_alpha, 50, 50),
        64,
        "/ca is the nonstroking opacity and outranks /CA for a fill"
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

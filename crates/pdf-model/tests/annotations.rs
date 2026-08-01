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
/// A `Stamp`, a `FileAttachment` and a `Sound` display an *icon*, and §12.5.6.12, §12.5.6.15
/// and §12.5.6.16 each name theirs — `/Approved`, `/Paperclip`, `/Speaker` — without stating
/// one line of their artwork. Each says a reader "**should** provide predefined icon
/// appearances", which is a recommendation: drawing a guess would put a mark on the page the
/// document never described and no clause requires, so the report is the answer, and it is
/// what keeps the corpus gate's counts meaningful.
///
/// §12.5.6.4's text annotation is the one that left this list, in the hundred-and-twentieth
/// session, and the word that moved it is *shall*. §12.5.6.10's four text markups left it in
/// the thirty-fourth, and the reason is a different one worth the distinction: that clause
/// states the *mark* — "shall appear as highlights, underlines, strikeouts … or jagged
/// ('squiggly') underlines" — its region and its orientation, and leaves only a thickness.
#[test]
fn an_annotation_whose_appearance_is_only_recommended_is_reported() {
    for (subtype, name) in [
        ("Stamp", "Approved"),
        ("FileAttachment", "Paperclip"),
        ("Sound", "Speaker"),
    ] {
        let interpretation = interpret(pdf_with(
            &format!(
                "<< /Type /Annot /Subtype /{subtype} /Rect [20 20 60 60] /F 4 /C [1 1 0] \
                 /Name /{name} >>"
            ),
            "/BBox [0 0 10 10]",
            "1 0 0 rg 0 0 10 10 re f",
        ));
        assert!(
            interpretation.display_list.commands().is_empty(),
            "{subtype}: nothing may be invented for it"
        );
        assert!(
            !interpretation.is_complete(),
            "{subtype}: and its absence must be reported"
        );
    }
}

/// A text annotation with no appearance stream draws the icon Table 175's `/Name` selects.
///
/// §12.5.6.4 states the obligation — "Interactive PDF processors shall provide predefined icon
/// appearances for at least the following standard names" — and none of the artwork, so what
/// this can assert is what the clause says and not what the picture is: every one of the seven
/// names draws *something*, and no two of them draw the *same* something. A processor that
/// supplied one shape under seven names would satisfy the sentence's letter and tell a reader
/// nothing, which is the failure worth a test.
#[test]
fn each_of_the_seven_standard_icon_names_draws_its_own_shape() {
    let icon = |name: &str| {
        let raster = render(pdf_with(
            &format!("<< /Type /Annot /Subtype /Text /Rect [10 10 90 90] /F 4 /Name /{name} >>"),
            "/BBox [0 0 10 10]",
            "",
        ));
        raster.data.clone()
    };
    let names = [
        "Comment",
        "Key",
        "Note",
        "Help",
        "NewParagraph",
        "Paragraph",
        "Insert",
    ];
    let drawn: Vec<Vec<u8>> = names.iter().map(|name| icon(name)).collect();
    for (name, pixels) in names.iter().zip(&drawn) {
        assert!(
            pixels.chunks_exact(4).any(|pixel| pixel[3] > 0),
            "/{name} drew nothing"
        );
    }
    for (first, one) in names.iter().enumerate() {
        for (second, other) in names.iter().enumerate().skip(first + 1) {
            assert_ne!(
                drawn[first], drawn[second],
                "/{one} and /{other} draw the same icon"
            );
        }
    }
}

/// Table 175: "Default value: Note ." — an absent `/Name` is the note, not a refusal.
#[test]
fn an_absent_icon_name_is_the_note() {
    let named = render(pdf_with(
        "<< /Type /Annot /Subtype /Text /Rect [10 10 90 90] /F 4 /Name /Note >>",
        "/BBox [0 0 10 10]",
        "",
    ));
    let unnamed = render(pdf_with(
        "<< /Type /Annot /Subtype /Text /Rect [10 10 90 90] /F 4 >>",
        "/BBox [0 0 10 10]",
        "",
    ));
    assert_eq!(named.data, unnamed.data);
}

/// A `/Name` outside the seven is reported by name, not drawn as the default.
///
/// "Additional names may be supported as well" is a permission, so an unrecognised icon is a
/// gap this processor has rather than one the document has — and Table 175's default of `Note`
/// answers an *absent* entry, not an unrecognised one. Substituting the note for it would draw
/// a picture whose meaning the file did not ask for and report nothing, which is trap 5's
/// failure exactly.
#[test]
fn an_icon_name_outside_the_seven_is_reported_by_name() {
    let interpretation = interpret(pdf_with(
        "<< /Type /Annot /Subtype /Text /Rect [10 10 90 90] /F 4 /Name /Bookmark >>",
        "/BBox [0 0 10 10]",
        "",
    ));
    assert!(
        interpretation.display_list.commands().is_empty(),
        "nothing may be invented for a name the clause does not require"
    );
    let reports = format!("{:?}", interpretation.unsupported);
    assert!(
        reports.contains("Bookmark"),
        "the report must name it: {reports}"
    );
}

/// Table 166's `/C` is "The background of the annotation's icon when closed".
///
/// The two halves are one test because each is the other's control: with a `/C` the icon's
/// corner is that colour, and with none it is transparent — Table 166's own word for an empty
/// or absent array. A reader that invented a background would fail the second half, and one
/// that ignored `/C` would fail the first.
#[test]
fn table_166_s_colour_is_the_icon_s_background() {
    let coloured = render(pdf_with(
        "<< /Type /Annot /Subtype /Text /Rect [10 10 90 90] /F 4 /C [1 0 0] /Name /Insert >>",
        "/BBox [0 0 10 10]",
        "",
    ));
    // Just inside the field's rounded corner, where no symbol reaches.
    assert_eq!(colour_at(&coloured, 25, 25), (255, 0, 0));

    let bare = render(pdf_with(
        "<< /Type /Annot /Subtype /Text /Rect [10 10 90 90] /F 4 /Name /Insert >>",
        "/BBox [0 0 10 10]",
        "",
    ));
    assert!(
        !painted(&bare, 25, 25),
        "an absent /C is Table 166's transparent, not an invented field"
    );
}

/// The icon keeps its proportions in a rectangle that has none, and stays centred in it.
///
/// A `/Rect` three times as wide as it is tall: the artwork is drawn on a square, so a reader
/// that stretched it onto the rectangle would reach both vertical edges. What must happen
/// instead is that the marks span the height and sit in the middle of the width — which is
/// this tree's choice rather than the clause's, since §12.5.6.4 would rather the icon not
/// scale at all (`NoZoom`, §12.5.3).
#[test]
fn an_icon_is_square_and_centred_in_a_rectangle_that_is_not() {
    let raster = render(pdf_with(
        "<< /Type /Annot /Subtype /Text /Rect [5 30 95 60] /F 4 /C [1 0 0] /Name /Insert >>",
        "/BBox [0 0 10 10]",
        "",
    ));
    let (min_x, min_y, max_x, max_y) = extent(&raster);
    assert_eq!(
        (min_y, max_y),
        (30, 59),
        "the square is the rectangle's height"
    );
    // 30 units wide, centred in 90: from 35 to 65.
    assert_eq!((min_x, max_x), (35, 64), "and centred across its width");
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

/// §12.5.5's three appearances: the pointer decides which one is drawn.
///
/// > The normal appearance shall be used when the annotation is not interacting with the
/// > user. … The down appearance shall be used when the mouse button is pressed or held down
/// > within the annotation's active area.
///
/// The appearance streams here paint different colours, so the assertion is a pixel: nothing
/// about the display list distinguishes "we picked `/N`" from "we picked `/D` and it happens
/// to look the same". Which one is asked for is `ViewState`'s, because the cursor is not in
/// the document — the same division §12.6.4's actions are read under.
///
/// The last case is the one Table 170 decides: `/R` is optional, so an annotation with the
/// pointer over it and no rollover appearance shows its normal one rather than nothing.
#[test]
fn the_pointer_chooses_between_an_annotations_appearances() {
    let bytes = pdf_with(
        "<< /Type /Annot /Subtype /Widget /Rect [10 10 90 90] /F 4 \
         /AP << /N 6 0 R /D 7 0 R >> >>",
        "/BBox [0 0 80 80]",
        "1 0 0 rg 0 0 80 80 re f",
    );
    // Object 7 is the down appearance, appended to the fixture: blue where /N is red.
    let bytes = with_down_appearance(bytes);
    let document = Document::open(bytes).expect("the fixture is a valid PDF");
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    let annotation = pdf_syntax::ObjectId {
        number: 5,
        generation: 0,
    };

    let colour = |state: &pdf_model::view::ViewState| {
        let list = pdf_model::content::interpret_with(&document, &page, state).display_list;
        let target = TargetSpec::for_page(&list, 1.0, 1 << 20).expect("target");
        let raster = CpuRasterizer::new()
            .rasterize(&list, target)
            .expect("supported");
        let at = ((50 * raster.width) + 50) as usize * 4;
        (raster.data[at], raster.data[at + 1], raster.data[at + 2])
    };

    let mut state = pdf_model::view::ViewState::of(&document);
    assert_eq!(colour(&state), (255, 0, 0), "nothing is interacting: /N");

    state.set_pointer(Some((annotation, pdf_model::view::Pointer::Down)));
    assert_eq!(colour(&state), (0, 0, 255), "a button pressed on it: /D");

    state.set_pointer(Some((annotation, pdf_model::view::Pointer::Over)));
    assert_eq!(
        colour(&state),
        (255, 0, 0),
        "the cursor over it with no /R stated: /N, which Table 170 makes the only required one"
    );

    let elsewhere = pdf_syntax::ObjectId {
        number: 9,
        generation: 0,
    };
    state.set_pointer(Some((elsewhere, pdf_model::view::Pointer::Down)));
    assert_eq!(
        colour(&state),
        (255, 0, 0),
        "pressed on some other annotation"
    );
}

/// Appends object 7, a blue appearance stream, to a fixture built by [`pdf_with`].
///
/// Written as a rebuild rather than a splice because the cross-reference table's offsets are
/// what a `Document` reads, and an appended object nothing points at is not in it.
fn with_down_appearance(bytes: Vec<u8>) -> Vec<u8> {
    let text = String::from_utf8(bytes).expect("the fixture is ASCII");
    let body: String = text
        .split_inclusive("endobj\n")
        .filter(|part| part.contains(" 0 obj"))
        .collect();
    let down = "0 0 1 rg 0 0 80 80 re f";
    let body = format!(
        "{body}7 0 obj\n<< /Type /XObject /Subtype /Form /BBox [0 0 80 80] /Length {} >>\n\
         stream\n{down}\nendstream\nendobj\n",
        down.len().saturating_add(1)
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

/// §12.5.6.7's leader lines: `/L` is where the leaders start, not where the line is.
///
/// Table 178 makes `/L` "the endpoints of the leader lines rather than the endpoints of the
/// line itself" whenever `/LL` is present, and states where the line goes: the leaders "extend
/// from each endpoint of the line perpendicular to the line itself", and a positive `/LL`
/// "shall mean that the leader lines appear in the direction that is clockwise when traversing
/// the line from its starting point to its ending point".
///
/// The fixture states a horizontal `/L` from (20, 50) to (80, 50) with `/LL 20`. Traversing
/// left to right in this y-up space, clockwise is *downwards* — the same quarter turn
/// §7.7.3.3's `/Rotate` takes — so the line proper is at y = 30 and nothing is drawn along
/// y = 50 except the two leaders at its ends. Drawing `/L` itself —
/// which is what this tree did until the eighty-fifth session, by refusing — would paint the
/// middle of y = 50 and leave y = 30 blank, so the two assertions are the two readings.
#[test]
fn a_line_annotations_leader_lines_put_the_line_where_the_clause_says() {
    let raster = render(pdf_with(
        "<< /Type /Annot /Subtype /Line /Rect [0 0 100 100] /F 4 /C [0 0 0] /BS << /W 2 >> \
         /L [20 50 80 50] /LL 20 >>",
        "/BBox [0 0 10 10]",
        "",
    ));

    assert!(
        painted(&raster, 50, 30),
        "the line proper is 20 units clockwise of /L, which is below it in a y-up space"
    );
    assert!(
        !painted(&raster, 50, 50),
        "and /L itself is not the line: only its two leaders touch that row"
    );
    assert!(
        painted(&raster, 20, 48),
        "the first leader starts at /L and runs from there"
    );
    assert!(painted(&raster, 80, 40), "and runs to the line proper");
}

/// A negative `/LL` puts the line on the other side, which is the entry's whole sign rule.
///
/// "[A] negative value shall indicate the opposite direction." Same fixture, same geometry,
/// one minus sign — and a reader that took the absolute value would pass the test above and
/// fail this one.
#[test]
fn a_negative_leader_length_reverses_the_side() {
    let raster = render(pdf_with(
        "<< /Type /Annot /Subtype /Line /Rect [0 0 100 100] /F 4 /C [0 0 0] /BS << /W 2 >> \
         /L [20 50 80 50] /LL -20 >>",
        "/BBox [0 0 10 10]",
        "",
    ));
    assert!(painted(&raster, 50, 70), "20 units anticlockwise of /L");
    assert!(!painted(&raster, 50, 30), "and not on the clockwise side");
}

/// `/LLE` extends each leader past the line, and `/LLO` leaves a gap before it starts.
///
/// Table 178: `/LLE` is "the length of leader line extensions that extend from the line proper
/// 180 degrees from the leader lines", and `/LLO` "the amount of empty space between the
/// endpoints of the annotation and the beginning of the leader lines". So with `/LL 20`,
/// `/LLE 10` and `/LLO 5` the leader occupies the band 5 to 30 units below `/L` — y = 45 down
/// to y = 20 — and the rows either side of that band are blank.
#[test]
fn a_leader_line_has_an_offset_before_it_and_an_extension_beyond_it() {
    let raster = render(pdf_with(
        "<< /Type /Annot /Subtype /Line /Rect [0 0 100 100] /F 4 /C [0 0 0] /BS << /W 2 >> \
         /L [20 50 80 50] /LL 20 /LLE 10 /LLO 5 >>",
        "/BBox [0 0 10 10]",
        "",
    ));
    assert!(
        !painted(&raster, 20, 49),
        "/LLO leaves the first units below /L empty"
    );
    assert!(
        painted(&raster, 20, 40),
        "the leader runs from there to the line"
    );
    assert!(
        painted(&raster, 20, 22),
        "/LLE carries it past the line at y = 30"
    );
    assert!(
        !painted(&raster, 20, 15),
        "and it stops: 20 + 10 units from /L is as far as the clause states"
    );
}

/// An entry that states no shape must not erase the shape the clause does state.
///
/// ISO 32000-2 §12.5.6.7 makes `/L` required and `/LE` optional with a default of
/// `[/None /None]`, and Table 179 gives its nine endings names and not one dimension. So an
/// annotation naming an ending has still stated a line, and this pair is that difference: the
/// same line with and without an ending it cannot be given a size, drawn identically, with the
/// ending named beside it rather than instead of it.
///
/// It was a whole-annotation refusal until the hundred-and-sixteenth session — the same shape
/// ADR 0075 removed from `/LL` one entry over, where the refusal fired on an entry's presence
/// and took the line with it.
#[test]
fn a_line_ending_that_cannot_be_sized_is_named_beside_the_line_it_decorates() {
    let line = |extra: &str| {
        format!(
            "<< /Type /Annot /Subtype /Line /Rect [0 0 100 100] /F 4 /C [0 0 0] \
             /BS << /W 2 >> /L [20 50 80 50] {extra} >>"
        )
    };
    let of = |extra: &str| interpret(pdf_with(&line(extra), "/BBox [0 0 10 10]", ""));

    let plain = of("");
    assert!(plain.is_complete(), "a line with no /LE owes nothing");

    let ended = of("/LE [/OpenArrow /OpenArrow]");
    assert!(
        !ended.display_list.commands().is_empty(),
        "the line the clause requires is drawn whatever its ends ask for"
    );
    assert!(
        !ended.is_complete(),
        "and the ending nobody can size is reported"
    );
    let reported = format!("{:?}", ended.unsupported);
    assert!(
        reported.contains("line endings state no size"),
        "by name: {reported}"
    );

    // Same for §12.5.6.7's caption, and both at once name both — a report that hides another
    // report is trap 11's other edge.
    let both = of("/LE [/Square /Square] /Cap true");
    let reported = format!("{:?}", both.unsupported);
    assert!(
        reported.contains("line endings state no size") && reported.contains("/Cap"),
        "both owed entries are named, not just the first: {reported}"
    );

    // And a polyline's ends are the same rule: Table 181 makes `/Vertices` required.
    let polyline = interpret(pdf_with(
        "<< /Type /Annot /Subtype /PolyLine /Rect [0 0 100 100] /F 4 /C [0 0 0] \
         /BS << /W 2 >> /Vertices [20 20 80 80] /LE [/ClosedArrow /ClosedArrow] >>",
        "/BBox [0 0 10 10]",
        "",
    ));
    assert!(
        !polyline.display_list.commands().is_empty(),
        "the polyline is drawn"
    );
    assert!(!polyline.is_complete(), "and its endings are named");
}

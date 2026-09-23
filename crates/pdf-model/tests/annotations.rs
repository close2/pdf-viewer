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

use pdf_model::appearance::Intent;
use pdf_render::{Rasterizer, TargetSpec};
use pdf_syntax::Document;
use render_cpu::CpuRasterizer;

/// Pixel budget, far above the 100×100 pages these tests build.
const GENEROUS: u64 = 1 << 30;

/// Wraps a body of objects in a header, a cross-reference table and a trailer.
///
/// Every fixture in this file is one page, a few objects long and numbered from 1, so the offsets
/// are whatever the objects land at — which is what §7.5.4 asks a cross-reference table to say.
fn assemble(body: &str) -> Vec<u8> {
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

/// Assembles a one-page PDF whose page carries one annotation.
///
/// The page's own content stream is empty, so every mark in the raster came from the
/// appearance and nothing has to be subtracted to see it.
fn pdf_with(annotation: &str, appearance_dict: &str, appearance: &str) -> Vec<u8> {
    pdf_over("", annotation, appearance_dict, appearance)
}

/// The same, with content on the page under the annotation.
///
/// §12.5.5 composites an appearance's transparency group "with a backdrop consisting of the page
/// content along with any previously painted annotations", so a fixture about what the group
/// composites *onto* needs a page that has something on it. `page` is drawn in the page's own
/// default user space.
fn pdf_over(page: &str, annotation: &str, appearance_dict: &str, appearance: &str) -> Vec<u8> {
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] \
         /Resources << >> /Contents 4 0 R /Annots [5 0 R] >>\nendobj\n\
         4 0 obj\n<< /Length {} >>\nstream\n{page}\nendstream\nendobj\n\
         5 0 obj\n{annotation}\nendobj\n\
         6 0 obj\n<< /Type /XObject /Subtype /Form {appearance_dict} /Length {} >>\n\
         stream\n{appearance}\nendstream\nendobj\n",
        page.len().saturating_add(1),
        appearance.len().saturating_add(1)
    );

    assemble(&body)
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
        .with_medium(pdf_render::Medium::NONE)
        .rasterize(&list, target)
        .expect("supported")
}

/// Rasterises an interpretation that reported something, which [`render`] refuses to.
fn render_incomplete(interpretation: &pdf_model::Interpretation) -> pdf_render::Raster {
    let list = &interpretation.display_list;
    let target = TargetSpec::for_page(list, 1.0, GENEROUS).expect("valid target");
    CpuRasterizer::new()
        .with_medium(pdf_render::Medium::NONE)
        .rasterize(list, target)
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

/// An appearance stream with no `/BBox` is placed by §12.7.4.3's default box, and named.
///
/// §8.10.2 makes `/BBox` required and §12.5.5's algorithm starts by transforming it, so a
/// stream without one states no box to map onto `/Rect`. The box used instead is the one the
/// standard itself states for an appearance stream, in the one place it states any — §12.7.4.3,
/// on the form dictionary a processor builds for a field:
///
/// > The lower-left corner of the bounding box ( BBox ) is set to coordinates (0, 0) in the
/// > form coordinate system. The box's top and right coordinates are taken from the dimensions
/// > of the annotation rectangle (the Rect entry in the widget annotation dictionary).
///
/// The fixture's stream fills a 10×10 square at its own origin and `/Rect` is 40×40 at
/// (20, 30), so the box is [0 0 40 40], the placement is a *translation* — the box already has
/// the rectangle's size — and the square lands in the rectangle's lower-left corner at its own
/// scale. That is the discriminating value: with the entry present and equal to the square,
/// `an_appearance_is_scaled_and_translated_onto_the_annotation_rectangle` gets the same square
/// scaled by four across the whole rectangle instead. A reader that took `/Rect` itself as the
/// box would leave the mark at the *page's* corner and clip all of it away, which is the blank
/// the earlier refusal produced by another route.
#[test]
fn an_appearance_with_no_bounding_box_is_placed_by_the_clauses_default() {
    let interpretation = interpret(pdf_with(
        "<< /Type /Annot /Subtype /Square /Rect [20 30 60 70] /F 4 /AP << /N 6 0 R >> >>",
        "",
        "1 0 0 rg 0 0 10 10 re f",
    ));
    let reports = format!("{:?}", interpretation.unsupported);
    assert!(
        reports.contains("/BBox"),
        "the missing entry is named: {reports}"
    );

    // Not `render`, which insists the page drew completely: this one deliberately reports.
    let list = interpretation.display_list;
    let target = TargetSpec::for_page(&list, 1.0, GENEROUS).expect("valid target");
    let raster = CpuRasterizer::new()
        .with_medium(pdf_render::Medium::NONE)
        .rasterize(&list, target)
        .expect("supported");
    assert_eq!(extent(&raster), (20, 30, 29, 39));
}

/// An annotation with no `/Rect` is placed where its appearance's own box puts it.
///
/// The mirror of the test above, and the same rule read the other way: §12.5.5 maps the
/// appearance's transformed bounding box onto `/Rect`, the two are the same kind of thing, and
/// a missing operand makes the map the identity whichever operand it is. Here the stream draws
/// a 20×30 rectangle at (10, 20) in its own space and says so in its `/BBox`, and that is where
/// it lands — where a reader that refused would leave the page blank.
#[test]
fn an_annotation_with_no_rectangle_is_placed_by_its_appearances_box() {
    let raster = render(pdf_with(
        "<< /Type /Annot /Subtype /Square /F 4 /AP << /N 6 0 R >> >>",
        "/BBox [10 20 30 50]",
        "0 0 1 rg 10 20 20 30 re f",
    ));
    assert_eq!(extent(&raster), (10, 20, 29, 49));
}

/// An appearance whose own box covers no area draws nothing, and says nothing.
///
/// `issue14438.pdf` states four ink annotations with no `/Rect` and appearance streams whose
/// `/BBox` is `[0 0 0 0]`, and it used to be reported for the missing rectangle — the one entry
/// that could not have changed the picture. What draws nothing here is §12.5.5's own arithmetic:
/// the box is scaled onto the rectangle, and a scale onto no extent leaves no mark.
///
/// **Table 166's `/AP` bullet is not the reason, and this comment said it was** until the
/// seven-hundred-and-thirty-fourth session. The bullet frees a *writer* from supplying an
/// appearance only where both of `/Rect`'s pairs are equal — a point — which this fixture's
/// `[0 0 0 0]` happens to be and which `annotation::is_empty` is wider than.
#[test]
fn an_appearance_box_covering_no_area_draws_nothing_and_reports_nothing() {
    let interpretation = interpret(pdf_with(
        "<< /Type /Annot /Subtype /Ink /F 4 /AP << /N 6 0 R >> >>",
        "/BBox [0 0 0 0]",
        "0 0 1 rg 10 20 20 30 re f",
    ));
    assert!(interpretation.display_list.commands().is_empty());
    assert!(
        interpretation.is_complete(),
        "an annotation the file gives no area is not a gap: {:?}",
        interpretation.unsupported
    );
}

/// A `/Rect` covering no area does not unstate marks the clause puts in default user space.
///
/// The rule the fixture above turns on is §12.5.5's arithmetic over a **stored** stream, and it
/// was being applied to constructions that go through none of it. §12.5.6.10's Table 182 states
/// `/QuadPoints` as "the coordinates of n quadrilaterals in default user space" and gives them no
/// fallback to `/Rect` — where the standard means one it says so, which is exactly what
/// §12.5.6.5's Table 176 does for a *link's* activation region:
///
/// > If this entry is not present, or the PDF processor does not recognise it, or if any
/// > coordinates in the QuadPoints array lie outside the region specified by Rect then the
/// > activation region for the link annotation shall be defined by its Rect entry.
///
/// Table 166's `/AP` row is evidence the same way: it frees a writer from supplying an appearance
/// dictionary for "[a]nnotations where the value of the Rect key consists of an array where the
/// value at index 1 is equal to the value at index 3 and the value at index 2 is equal to the
/// value at index 4" — this fixture, and `sumatrapdf-LINK-1618-0.pdf`'s thirteen — so a point
/// `/Rect` beside a whole `/QuadPoints` is a file the standard anticipates rather than a file
/// that has stated nothing. ADR 0825.
#[test]
fn a_text_markups_quadrilaterals_are_drawn_where_its_rect_covers_no_area() {
    let interpretation = interpret(pdf_with(
        "<< /Type /Annot /Subtype /Underline /Rect [20 20 20 20] /F 4 \
         /QuadPoints [20 60 80 60 20 40 80 40] /C [0 0 0] >>",
        "/BBox [0 0 10 10]",
        "",
    ));
    assert!(
        interpretation.is_complete(),
        "the quadrilaterals state the mark whole: {:?}",
        interpretation.unsupported
    );
    let raster = render_incomplete(&interpretation);
    // §12.5.6.10's underline is a bar on the quadrilateral's bottom edge — y 40, and x across the
    // quadrilateral rather than at the point `/Rect` names.
    assert!(painted(&raster, 50, 41), "the bar is on the bottom edge");
    assert!(!painted(&raster, 50, 55), "and nowhere else in the quad");
    assert!(
        !painted(&raster, 5, 41),
        "the bar stops at the quadrilateral's own edge"
    );
}

/// §12.5.6.7's `/L` is in default user space too, so a point `/Rect` does not erase the line.
#[test]
fn a_line_annotation_is_drawn_where_its_rect_covers_no_area() {
    let interpretation = interpret(pdf_with(
        "<< /Type /Annot /Subtype /Line /Rect [0 0 0 0] /F 4 \
         /L [20 50 80 50] /C [0 0 0] /BS << /W 3 >> >>",
        "/BBox [0 0 10 10]",
        "",
    ));
    assert!(
        interpretation.is_complete(),
        "the endpoints state the line whole: {:?}",
        interpretation.unsupported
    );
    let raster = render_incomplete(&interpretation);
    assert!(painted(&raster, 50, 50), "the line is where /L puts it");
    assert!(!painted(&raster, 50, 20), "and not at the origin");
}

/// A subtype whose marks *are* `/Rect`'s still draws nothing, and still says nothing.
///
/// §12.5.6.8's square is "inscribed within the annotation rectangle", so a rectangle of no area
/// inscribes nothing — the exemption above is the six subtypes whose own clause states default
/// user space, and widening it to every construction would put this document back to reporting a
/// gap that is Table 166's `/AP` row rather than ours.
#[test]
fn a_construction_that_rect_places_draws_nothing_where_rect_covers_no_area() {
    let interpretation = interpret(pdf_with(
        "<< /Type /Annot /Subtype /Square /Rect [20 20 20 20] /F 4 /IC [0 0 1] /C [0 0 0] >>",
        "/BBox [0 0 10 10]",
        "",
    ));
    assert!(
        interpretation.display_list.commands().is_empty(),
        "a square inscribed in no area is no square"
    );
    assert!(
        interpretation.is_complete(),
        "and it is not a gap: {:?}",
        interpretation.unsupported
    );
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

/// A `Stamp`'s appearance is reported rather than invented, and the other two are drawn.
///
/// All three clauses say a reader "**should** provide predefined icon appearances", and the
/// two-hundred-and-sixty-sixth session split them on what the *names* are. §12.5.6.15's `Graph`,
/// `PushPin`, `Paperclip` and `Tag` and §12.5.6.16's `Speaker` and `Mic` name **objects**, which
/// is more than §12.5.6.4's mandatory seven give — `NewParagraph` and `Insert` had to be invented
/// out of a typographer's convention — so the artwork is argued from the clause's own word.
/// §12.5.6.12's Table 184 names `Approved`, `Experimental`, `NotApproved`, `Draft` and the rest:
/// **legends rather than symbols**, so drawing one means choosing typography and a border, and a
/// reader would see a word this program picked in a face this program picked. A recommendation is
/// not a licence to invent a different kind of thing from the one the name names.
///
/// §12.5.6.4's text annotation left this list in the hundred-and-twentieth session and the word
/// that moved it is *shall*. §12.5.6.10's four text markups left it in the thirty-fourth, for a
/// different reason worth the distinction: that clause states the *mark* — "shall appear as
/// highlights, underlines, strikeouts … or jagged ('squiggly') underlines" — its region and its
/// orientation, and leaves only a thickness.
#[test]
fn a_stamps_appearance_is_reported_and_the_other_two_icons_are_drawn() {
    let interpret_icon = |subtype: &str, name: &str| {
        interpret(pdf_with(
            &format!(
                "<< /Type /Annot /Subtype /{subtype} /Rect [20 20 60 60] /F 4 /C [1 1 0] \
                 /Name /{name} >>"
            ),
            "/BBox [0 0 10 10]",
            "1 0 0 rg 0 0 10 10 re f",
        ))
    };

    let stamp = interpret_icon("Stamp", "Approved");
    assert!(
        stamp.display_list.commands().is_empty(),
        "a legend is not a symbol and nothing may be invented for it"
    );
    assert!(!stamp.is_complete(), "and its absence must be reported");

    for (subtype, names) in [
        (
            "FileAttachment",
            ["Graph", "PushPin", "Paperclip", "Tag"].as_slice(),
        ),
        ("Sound", ["Speaker", "Mic"].as_slice()),
    ] {
        for name in names {
            let drawn = interpret_icon(subtype, name);
            assert!(
                !drawn.display_list.commands().is_empty(),
                "{subtype} /{name}: the clause names an object and this one is drawn"
            );
            assert!(
                drawn.is_complete(),
                "{subtype} /{name}: and nothing is owed: {:?}",
                drawn.unsupported
            );
        }
        // A name outside the clause's list is still reported: a default is what an *absent*
        // entry means, not what an unrecognised one means.
        let odd = interpret_icon(subtype, "Rhubarb");
        assert!(!odd.is_complete(), "{subtype} /Rhubarb must be reported");
    }
}

/// A caret with no appearance stream is reported rather than drawn, `/Sy` or no `/Sy`.
///
/// §12.5.6.11 says a caret annotation "is a visual symbol that indicates the presence of text
/// edits" and states no artwork for the caret, so there is nothing to derive — the same
/// position §12.5.6.12's stamp is in and for the same reason.
///
/// **The second half of the assertion is the one worth having, and the ledger row was wrong
/// about it until the five-hundred-and-eighty-ninth session.** Table 183 does state a symbol,
/// by name and by character: "P A new paragraph symbol (¶) shall be associated with the
/// caret" — a `shall` and a code point, which is more than §12.5.6.4's seven icons get. What keeps the
/// refusal whole is trap 5's additive-or-substitutive test, read off `/RD`'s own sentence in the
/// same table — the difference "can occur. When a paragraph symbol specified by Sy is displayed
/// along with the caret" — so the pilcrow accompanies the caret rather than standing for it,
/// and drawing it alone would put a mark on the page beside the mark nobody can derive.
///
/// No corpus document states `/Sy` as a name (0 of 974, `examples/witness_census`), and all four
/// that state `/Caret` carry an `/AP`, so nothing but reading Table 183 reaches this.
#[test]
fn a_caret_is_reported_whether_or_not_it_asks_for_a_paragraph_symbol() {
    for entries in [
        "<< /Type /Annot /Subtype /Caret /Rect [20 20 60 60] /F 4 >>",
        "<< /Type /Annot /Subtype /Caret /Rect [20 20 60 60] /F 4 /Sy /P >>",
        "<< /Type /Annot /Subtype /Caret /Rect [20 20 60 60] /F 4 /Sy /None \
         /RD [2 2 2 2] >>",
    ] {
        let caret = interpret(pdf_with(
            entries,
            "/BBox [0 0 10 10]",
            "1 0 0 rg 0 0 10 10 re f",
        ));
        assert!(
            caret.display_list.commands().is_empty(),
            "{entries}: the caret's own shape is stated nowhere, so nothing may be drawn"
        );
        assert!(
            !caret.is_complete(),
            "{entries}: and its absence is reported"
        );
        // **And the report says why in this clause's own terms**, which it did not until the
        // six-hundred-and-twenty-second session: a caret fell to `construct`'s catch-all and a
        // person was told "its clause states no geometry" about a table that states four numbers
        // of it, in the order `appearance::insets` reads for §12.5.6.8 and §12.5.6.6. The
        // behaviour was right and the sentence was not, and only the sentence reaches a reader —
        // which is why this test now asserts one. ADR 0457.
        let said = format!("{:?}", caret.unsupported);
        assert!(
            said.contains("no artwork for the caret itself")
                && said.contains("along with the caret rather than instead of it"),
            "{entries}: the report is the catch-all's rather than this clause's: {said}"
        );
        assert!(
            !said.contains("states no geometry"),
            "{entries}: Table 183 states /RD, which is geometry: {said}"
        );
    }
}

/// A redaction with no appearance stream is reported rather than drawn, and the report is
/// §12.5.6.23's rather than the catch-all's.
///
/// **The caret's defect a second time in the same arm** (ADR 0461). §12.5.6.23 states the region
/// twice — Table 195's `/QuadPoints` is an array "specifying the coordinates of n quadrilaterals in
/// default user space", referred by that row to Table 182, which is the entry §12.5.6.10 is drawn
/// from here; and "If this entry is not present, the Rect entry denotes the content region that is
/// intended to be removed". So a person was told "its clause states no geometry" about a clause
/// that states it in two places.
///
/// What is genuinely unstated is any artwork for the annotation *before* it is applied, which is
/// the only state this program sees one in: every overlay entry in Table 195 begins "after the
/// affected content has been removed". The behaviour is therefore unchanged and only the sentence
/// moves — the half a person reads.
///
/// Both spellings are asserted, with and without `/QuadPoints`, because the two are the clause's
/// own alternative and neither may fall to the catch-all.
#[test]
fn a_redaction_is_reported_in_its_own_clause_s_terms() {
    for entries in [
        "<< /Type /Annot /Subtype /Redact /Rect [20 20 60 60] /F 4 >>",
        "<< /Type /Annot /Subtype /Redact /Rect [20 20 60 60] /F 4 \
         /QuadPoints [20 60 60 60 20 20 60 20] >>",
        "<< /Type /Annot /Subtype /Redact /Rect [20 20 60 60] /F 4 /IC [1 0 0] \
         /OverlayText (gone) /Q 1 >>",
    ] {
        let redaction = interpret(pdf_with(
            entries,
            "/BBox [0 0 10 10]",
            "1 0 0 rg 0 0 10 10 re f",
        ));
        assert!(
            redaction.display_list.commands().is_empty(),
            "{entries}: an unapplied redaction's mark is stated nowhere, so nothing may be drawn"
        );
        assert!(
            !redaction.is_complete(),
            "{entries}: and its absence is reported"
        );
        let said = format!("{:?}", redaction.unsupported);
        assert!(
            said.contains("the region to be removed rather than a mark to draw"),
            "{entries}: the report is the catch-all's rather than this clause's: {said}"
        );
        assert!(
            !said.contains("states no geometry"),
            "{entries}: Table 195 states /QuadPoints and names /Rect in its place: {said}"
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
    // Somewhere in the field rather than at a stated pixel: the square the icon is drawn on is
    // §12.5.6.4's fixed size hung from `/Rect`'s corner, so a test that named a coordinate would
    // be pinning that arithmetic here as well as the entry this one is about.
    let field = (0..coloured.height)
        .flat_map(|y| (0..coloured.width).map(move |x| (x, y)))
        .find(|&(x, y)| colour_at(&coloured, x, y) == (255, 0, 0));
    let Some((x, y)) = field else {
        panic!("/C [1 0 0] must paint the icon's background");
    };

    let bare = render(pdf_with(
        "<< /Type /Annot /Subtype /Text /Rect [10 10 90 90] /F 4 /Name /Insert >>",
        "/BBox [0 0 10 10]",
        "",
    ));
    assert!(
        !painted(&bare, x, y),
        "an absent /C is Table 166's transparent, not an invented field"
    );
}

/// The icon keeps its proportions in a rectangle that has none, and stays centred in it.
///
/// A `/Rect` three times as wide as it is tall: the artwork is drawn on a square, so a reader
/// that stretched it onto the rectangle would reach both vertical edges. What must happen
/// instead is that the marks span the height and sit in the middle of the width.
///
/// **§12.5.6.15's file attachment and not §12.5.6.4's text annotation**, which this test used
/// until the six-hundred-and-fortieth session and which is now the wrong subtype to ask: a text
/// annotation is "attached to a point" and holds a size fixed on the screen, so its icon does not
/// take a size from `/Rect` at all. Table 187 asks for a paperclip and a push pin with a
/// **should** and states no geometry, so inscribing them is this tree's choice — which is what
/// this test pins, by rendering the same annotation twice: once in the wide rectangle and once in
/// the square that rectangle's centre implies. A reader that stretched the artwork, or one that
/// pushed it into a corner, separates the two.
#[test]
fn an_icon_is_square_and_centred_in_a_rectangle_that_is_not() {
    let icon = |rect: &str| {
        let raster = render(pdf_with(
            &format!(
                "<< /Type /Annot /Subtype /FileAttachment /Rect {rect} /F 4 /Name /PushPin >>"
            ),
            "/BBox [0 0 10 10]",
            "",
        ));
        extent(&raster)
    };
    // 90 by 30, so the square is 30 across and starts 30 units in: 35 to 65.
    assert_eq!(icon("[5 30 95 60]"), icon("[35 30 65 60]"));
    // And the equality is not vacuous: the square at either end of the same rectangle is a
    // different picture, which is what "centred" excludes.
    assert_ne!(icon("[5 30 95 60]"), icon("[5 30 35 60]"));
    assert_ne!(icon("[5 30 95 60]"), icon("[65 30 95 60]"));
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

/// A subtype this crate knows nothing about still draws its normal appearance.
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

/// The same subtype with **no** appearance dictionary draws nothing and is not a gap.
///
/// The other half of the sentence the test above asserts, and it was reported as unsupported
/// until the seven-hundred-and-thirty-fourth session — with the detail "its clause states no
/// geometry", asserted of a subtype that has no clause. Table 167's `Invisible` row says what a
/// reader owes here and says it with a condition: "If clear, render such an unknown annotation
/// using an appearance stream specified by its appearance dictionary, if any". There is none, so
/// the requirement is met by drawing nothing, and a report would name a shortfall that is not
/// one. Errata Collection 3's Issue #1 is what sends §12.5.1's sentence about an unrecognised
/// type to this row and to §12.5.5 rather than to §12.5.2.
///
/// **A standard subtype this program does not construct is the control**, because it must keep
/// its report: `Movie` is Table 171's, and what refuses it is `CLAUDE.md`'s clause 13 exclusion
/// rather than a silence — §13.4's Table 306 states a poster image, which is why the sentence
/// this used to call it by was corrected in the nine-hundred-and-thirty-third session. A page
/// that silently loses it is exactly what trap 5 exists to prevent. An annotation with no
/// `/Subtype` at all keeps its own report too, for Table 166's reason — the entry is required —
/// and `issue7446.pdf` is the corpus witness for that one.
#[test]
fn an_unknown_subtype_with_no_appearance_dictionary_draws_nothing_and_reports_nothing() {
    let interpretation = interpret(pdf_with(
        "<< /Type /Annot /Subtype /SomethingFromThePDF3Era /Rect [20 30 60 70] /F 4 >>",
        "/BBox [0 0 10 10]",
        "",
    ));
    assert!(
        interpretation.display_list.commands().is_empty(),
        "nothing states an appearance for it"
    );
    assert!(
        interpretation.is_complete(),
        "a type the standard does not define, with no /AP, asks for nothing: {:?}",
        interpretation.unsupported
    );

    let movie = interpret(pdf_with(
        "<< /Type /Annot /Subtype /Movie /Rect [20 30 60 70] /F 4 >>",
        "/BBox [0 0 10 10]",
        "",
    ));
    assert!(
        !movie.is_complete(),
        "a Table 171 subtype this program does not construct stays loud"
    );

    let anonymous = interpret(pdf_with(
        "<< /Type /Annot /Rect [20 30 60 70] /F 4 >>",
        "/BBox [0 0 10 10]",
        "",
    ));
    assert!(
        !anonymous.is_complete(),
        "Table 166 makes /Subtype required, so its absence is still a defect"
    );
}

/// Every one of Table 171's twenty-eight types is recognised by name (§12.5.1).
///
/// > A PDF processor shall provide annotation handlers for all of the conforming annotation types.
///
/// The sentence after it is what says what *recognised* means, because it is the case this one
/// excludes: "An interactive PDF processor shall provide certain expected behaviour for all
/// annotation types that it does not recognise". So the pair divides the world in two, and the
/// `shall` above is met for a type when the processor answers **from that type's own clause**
/// rather than from the fallback for a type it has never heard of.
///
/// That is what this asserts, and it asserts it where the division is actually made:
/// `appearance::construct` has one arm per Table 171 subtype, each carrying that subtype's own
/// reading — a construction where the clause states a shape, a refusal quoting the clause's reason
/// where it does not — and one catch-all whose sentence is about Table 166's required `/Subtype`
/// being absent. A named type that reached the catch-all would be a type with no handler.
///
/// **The control is inside the test** (trap 13): the anonymous annotation is the one input the
/// catch-all is for, so a run in which nothing reaches it would be a run that proved nothing. It
/// fires, and no named subtype does.
#[test]
fn every_annotation_type_table_171_defines_is_answered_by_its_own_clause() {
    /// ISO 32000-2 Table 171's first column, in the table's own order.
    const TABLE_171: [&str; 28] = [
        "Text",
        "Link",
        "FreeText",
        "Line",
        "Square",
        "Circle",
        "Polygon",
        "PolyLine",
        "Highlight",
        "Underline",
        "Squiggly",
        "StrikeOut",
        "Caret",
        "Stamp",
        "Ink",
        "Popup",
        "FileAttachment",
        "Sound",
        "Movie",
        "Screen",
        "Widget",
        "PrinterMark",
        "TrapNet",
        "Watermark",
        "3D",
        "Redact",
        "Projection",
        "RichMedia",
    ];
    /// The catch-all's own sentence, which is about an annotation that names no type at all.
    const NO_TYPE: &str = "Table 166 makes /Subtype required";

    for subtype in TABLE_171 {
        let interpretation = interpret(pdf_with(
            &format!("<< /Type /Annot /Subtype /{subtype} /Rect [20 30 60 70] /F 4 >>"),
            "/BBox [0 0 10 10]",
            "",
        ));
        let reports = format!("{:?}", interpretation.unsupported);
        assert!(
            !reports.contains(NO_TYPE),
            "/{subtype} is one of Table 171's and reached the answer for a type nobody \
             recognises: {reports}"
        );
    }

    let anonymous = interpret(pdf_with(
        "<< /Type /Annot /Rect [20 30 60 70] /F 4 >>",
        "/BBox [0 0 10 10]",
        "",
    ));
    let reports = format!("{:?}", anonymous.unsupported);
    assert!(
        reports.contains(NO_TYPE),
        "the control must reach the catch-all, or the loop above asserted nothing: {reports}"
    );
}

/// A screen annotation with no `/AP` draws nothing and reports nothing, and one with an `/AP`
/// draws it.
///
/// §12.5.6.18, in the prose after Table 190 rather than in the table:
///
/// > If AP is not present, the screen annotation shall not have a default visual appearance and
/// > shall not be printed.
///
/// So the absence is the clause's own answer and there is no shortfall to name. Until the
/// nine-hundred-and-thirtieth session `crate::appearance::construct`'s catch-all reported "its
/// clause states no geometry" here, which is a claim about a clause that states one — trap 11,
/// and the correction §12.5.6.11's caret and §12.5.6.23's redaction each took out of the same
/// arm. ADR 0901.
///
/// **Both halves are asserted because only the pair discriminates**: an implementation that
/// dropped every `Screen` annotation outright would pass the first assertion alone, and the
/// clause's `/AP` sentence — the appearance "shall be used for printing and default display when
/// a media clip is not being played" — is the half that keeps the annotation on the page.
/// Calibrated by planting the defect this replaces (the `Screen` arm removed, so the catch-all
/// answers): the first assertion fails and the second passes.
#[test]
fn a_screen_annotation_without_an_appearance_draws_nothing_and_is_not_a_gap() {
    let bare = interpret(pdf_with(
        "<< /Type /Annot /Subtype /Screen /Rect [20 30 60 70] /F 4 /T (a clip) >>",
        "/BBox [0 0 10 10]",
        "",
    ));
    assert!(
        bare.display_list.commands().is_empty(),
        "the clause states that there is no default visual appearance"
    );
    assert!(
        bare.is_complete(),
        "a screen annotation with no /AP asks for nothing: {:?}",
        bare.unsupported
    );

    let stated = render(pdf_with(
        "<< /Type /Annot /Subtype /Screen /Rect [20 30 60 70] /F 4 /AP << /N 6 0 R >> >>",
        "/BBox [0 0 10 10]",
        "1 0 0 rg 0 0 10 10 re f",
    ));
    assert_eq!(
        extent(&stated),
        (20, 30, 59, 69),
        "an /AP is drawn like any other annotation's"
    );
}

/// No subtype ISO 32000-2 Table 171 defines is told that "its clause states no geometry".
///
/// **The population is Table 171's own list rather than a list somebody wrote in a comment**
/// (trap 25), and it is here because the sentence this asserts against was wrong four separate
/// times: §12.5.6.11's caret (ADR 0457), §12.5.6.23's redaction (ADR 0461), §12.5.6.18's screen
/// annotation (ADR 0901) and then, in the sweep this test comes from, §12.5.6.17's movie and
/// §12.5.6.22's watermark. Each correction was one member of a group defined by a sentence
/// nobody had checked, so the fifth was only ever a matter of somebody reading the fifth clause.
/// A group like that cannot be swept by a ledger sweep — a catch-all names no blocker, no
/// missing vocabulary and no absent architecture — which is why the guard is a test.
///
/// The verdict for each of the twenty-eight, from its own subclause:
///
/// | drawn from what the clause states | `Text` `Link` `FreeText` `Line` `Square` `Circle` `Polygon` `PolyLine` `Highlight` `Underline` `Squiggly` `StrikeOut` `Ink` `Widget` `FileAttachment` `Sound` |
/// |---|---|
/// | the clause states the outcome, so nothing is drawn and nothing owed | `Popup` (§12.5.6.14) `Projection` (§12.5.6.24) `Screen` (§12.5.6.18) |
/// | refused in the clause's own terms | `Stamp` `Caret` `Redact` `Movie` `Watermark` `PrinterMark` `TrapNet` `3D` `RichMedia` |
///
/// `FileAttachment` and `Sound` are in the first group because §12.5.6.15's and §12.5.6.16's
/// icon names — `Graph`, `PushPin`, `Paperclip`, `Tag`, `Speaker`, `Mic` — name **objects**, so
/// their artwork is argued from the clause's own words and drawn as a choice under a
/// recommendation. What they refuse is a `/Name` outside those lists, which is a different
/// sentence and has its own test.
///
/// Calibrated by planting each of the four arms this sweep added, one at a time — the arm
/// deleted so that the subtype falls to the catch-all — under which this test fails naming that
/// subtype and every other assertion here still passes.
#[test]
fn no_table_171_subtype_is_refused_with_the_catch_all_s_sentence() {
    // Table 171 - Annotation types, first column, in the standard's own order.
    const TABLE_171: [&str; 28] = [
        "Text",
        "Link",
        "FreeText",
        "Line",
        "Square",
        "Circle",
        "Polygon",
        "PolyLine",
        "Highlight",
        "Underline",
        "Squiggly",
        "StrikeOut",
        "Caret",
        "Stamp",
        "Ink",
        "Popup",
        "FileAttachment",
        "Sound",
        "Movie",
        "Screen",
        "Widget",
        "PrinterMark",
        "TrapNet",
        "Watermark",
        "3D",
        "Redact",
        "Projection",
        "RichMedia",
    ];

    for subtype in TABLE_171 {
        let interpretation = interpret(pdf_with(
            &format!("<< /Type /Annot /Subtype /{subtype} /Rect [20 30 60 70] /F 4 >>"),
            "/BBox [0 0 10 10]",
            "",
        ));
        let said = format!("{:?}", interpretation.unsupported);
        assert!(
            !said.contains("states no geometry"),
            "/{subtype} is one of Table 171's, so it has a clause and this report is about \
             some other clause: {said}"
        );
    }

    // The four arms this sweep added, each asserted on a phrase no other arm contains — a
    // substring shared with a neighbour would pass for the neighbour's answer (trap 27).
    for (subtype, phrase) in [
        ("Movie", "poster image"),
        ("Watermark", "transform the annotation rectangle"),
        ("PrinterMark", "whole of the mark's visual presentation"),
        ("TrapNet", "whole of the mark's visual presentation"),
        ("3D", "the annotation's inactive state"),
        ("RichMedia", "the annotation's inactive state"),
    ] {
        let interpretation = interpret(pdf_with(
            &format!("<< /Type /Annot /Subtype /{subtype} /Rect [20 30 60 70] /F 4 >>"),
            "/BBox [0 0 10 10]",
            "",
        ));
        assert!(
            interpretation.display_list.commands().is_empty(),
            "/{subtype}: nothing states a mark for it, so nothing may be drawn"
        );
        let said = format!("{:?}", interpretation.unsupported);
        assert!(
            said.contains(phrase),
            "/{subtype}: the report is not this clause's — it should name {phrase:?}: {said}"
        );
    }

    // What the catch-all is left with, and the only case that can reach it: Table 166 makes
    // `/Subtype` required, so an annotation without one has no subtype clause at all.
    let anonymous = interpret(pdf_with(
        "<< /Type /Annot /Rect [20 30 60 70] /F 4 >>",
        "/BBox [0 0 10 10]",
        "",
    ));
    let said = format!("{:?}", anonymous.unsupported);
    assert!(
        said.contains("Table 166 makes /Subtype required"),
        "an annotation with no /Subtype is the catch-all's whole population: {said}"
    );
}

/// §12.5.6.22: a watermark's `/FixedPrint` places it against the media, not on its own `/Rect`.
///
/// > The annotation's rectangle (as specified by its Rect entry) shall be translated to the
/// > origin and transformed by the Matrix entry of its FixedPrint dictionary to produce a
/// > quadrilateral with arbitrary orientation.
///
/// > The transformed annotation rectangle shall be defined as the smallest upright rectangle that
/// > encompasses this quadrilateral; it shall be used in place of the annotation rectangle
/// > referred to in steps 2 and 3 of "Algorithm: appearance streams"
///
/// with Table 194's `/H` and `/V` "as a percentage of the width of the target media (or if
/// unknown, the width of the page's MediaBox )", and §12.5.6.22 saying which media a screen has:
/// "When displaying a watermark annotation on-screen, interactive PDF processors shall use the
/// dimensions of the media box".
///
/// Every number is one addition on a 100 × 100 media box, checkable by hand:
///
/// ```text
/// /Rect [20 30 60 70] translated to the origin   x  0..40   y  0..40
/// transformed by /Matrix [1 0 0 1 5 5]           x  5..45   y  5..45
/// /H 0.25 and /V 0.5 of 100 by 100               x 30..70   y 55..95
/// ```
///
/// **Both halves are asserted because only the pair discriminates.** Table 193 makes the entry
/// optional — "[i]f this entry is not present, the annotation shall be drawn without any special
/// consideration for the dimensions of the target media" — so a watermark without one stays on
/// its `/Rect`, and a reader that moved every watermark would pass the first assertion alone.
#[test]
#[expect(
    clippy::doc_markdown,
    reason = "verbatim quotations: §12.5.6.22 and Table 194 spell FixedPrint and MediaBox \
              without backticks"
)]
fn a_fixed_print_watermark_is_placed_against_the_media() {
    let fixed = render(pdf_watermark(
        "[0 0 100 100]",
        "[20 30 60 70]",
        "/FixedPrint << /Type /FixedPrint /Matrix [1 0 0 1 5 5] /H 0.25 /V 0.5 >>",
    ));
    assert_eq!(
        extent(&fixed),
        (30, 55, 69, 94),
        "the appearance goes onto the transformed annotation rectangle"
    );

    let plain = render(pdf_watermark("[0 0 100 100]", "[20 30 60 70]", ""));
    assert_eq!(
        extent(&plain),
        (20, 30, 59, 69),
        "and a watermark with no /FixedPrint is placed on /Rect like any other annotation"
    );
}

/// §12.5.6.22's transformed rectangle is *upright*, so a rotating `/Matrix` reshapes the mark.
///
/// The clause's second bullet defines it as "the smallest upright rectangle that encompasses this
/// quadrilateral", and §12.5.5's steps 2 and 3 then map the appearance's box onto that rectangle's
/// corners — a scale and a translation, never a rotation. So a `/FixedPrint` `/Matrix` that turns
/// the rectangle a quarter turn exchanges the mark's width and height and leaves it upright,
/// which is the one visible difference between reading the bullet and applying the matrix to the
/// appearance instead:
///
/// ```text
/// /Rect [10 10 50 30] translated to the origin      x   0..40   y  0..20
/// through [0 1 -1 0 0 0], (x, y) -> (-y, x)         x -20..0    y  0..40
/// /H 0.5 and /V 0.25 of 100 by 100                  x  30..50   y 25..65
/// ```
#[test]
fn a_fixed_prints_rotating_matrix_gives_an_upright_rectangle() {
    let turned = render(pdf_watermark(
        "[0 0 100 100]",
        "[10 10 50 30]",
        "/FixedPrint << /Type /FixedPrint /Matrix [0 1 -1 0 0 0] /H 0.5 /V 0.25 >>",
    ));
    assert_eq!(
        extent(&turned),
        (30, 25, 49, 64),
        "the quadrilateral's own bounding box is what the appearance is scaled onto"
    );
}

/// `/H` and `/V` are measured from the media's own corner, which need not be the origin.
///
/// §8.3.2.3's NOTE 1 is why this is a case at all: "In the PostScript language, the origin of
/// default user space always corresponds to the lower-left corner of the output medium. While
/// this convention is common in PDF documents as well, it is not required". A percentage of the
/// media's width is a distance, so it is measured from that corner — which is what is left of
/// §12.5.6.22's third sentence on a screen, where the media box *is* the media and B's scale and
/// rotation are therefore the identity.
///
/// The fixture is the first test's, on a media box moved to `[10 20 110 120]`. Its dimensions are
/// unchanged, so `/H` and `/V` contribute what they did there and the mark lands on the same
/// place *on the medium* — which is the same place in the raster, because the page transform
/// subtracts the same corner again. A reader that dropped the term would put it ten units left
/// and twenty units down.
#[test]
fn a_fixed_print_is_measured_from_the_media_boxs_own_corner() {
    let moved = render(pdf_watermark(
        "[10 20 110 120]",
        "[20 30 60 70]",
        "/FixedPrint << /Type /FixedPrint /Matrix [1 0 0 1 5 5] /H 0.25 /V 0.5 >>",
    ));
    assert_eq!(
        extent(&moved),
        (30, 55, 69, 94),
        "the media's corner enters the placement and leaves through the page's transform"
    );
}

/// The one document in this tree that states a `/FixedPrint`, rebuilt from its own numbers.
///
/// `isartor-6-5-2-t01-fail-d.pdf` of the Isartor PDF/A-1b suite is the only witness under `doc/`
/// — `examples/fixed_print_census` over the 4172 PDFs there that open — and what makes it worth
/// a test is that its producer wrote a `/Rect` equal to the rectangle the clause computes:
///
/// ```text
/// /Rect [148.75 272.25 446.25 569.75] to the origin    x    0..297.5     y    0..297.5
/// through /Matrix [1 0 0 1 -148.75 -148.75]            x -148.75..148.75 y -148.75..148.75
/// /H 0.5 and /V 0.5 of 595 by 842                      x 148.75..446.25  y 272.25..569.75
/// ```
///
/// So all four terms — the corner that goes to the origin, the matrix, the two percentages and
/// the media they are percentages of — have to be right for the mark to come out where the file's
/// own rectangle already is, and a sign error in any of them moves it.
///
/// **Calibrated, and the result is worth stating rather than hiding.** Planting the defect this
/// replaces — `annotation::fixed_print` answering `None`, which is what this tree did until the
/// nine-hundred-and-forty-second session — fails the three tests above and leaves *this* one
/// green. That is the witness's own property: its producer wrote the two rectangles equal, so no
/// picture of this page can rank an implementation of the clause. What this test guards is the
/// other direction, which is the one a real file can lose — an implementation that moves a mark
/// the standard leaves where it is.
#[test]
fn the_one_witness_transforms_onto_its_own_rectangle() {
    let witness = render(pdf_watermark(
        "[0 0 595 842]",
        "[148.75 272.25 446.25 569.75]",
        "/FixedPrint << /Type /FixedPrint /V 0.5 /H 0.5 /Matrix [1 0 0 1 -148.75 -148.75] >>",
    ));
    assert_eq!(
        extent(&witness),
        (148, 272, 446, 569),
        "the transformed rectangle is the file's own /Rect, to the pixel it partly covers"
    );
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

/// A square's `/BS` gives it a width and a dash and no style, so a `B` there raises nothing.
///
/// Table 180 gives this subtype two of Table 168's entries — "specifying the line width and dash
/// pattern that shall be used in drawing the rectangle or ellipse" — and §12.5.4 states the same
/// restriction for four subtypes at once: "[s]uch dictionaries may also be used to specify the
/// width and dash pattern for the lines drawn by line, square, circle, and ink annotations". The
/// mark is the annotation's own rectangle, not §12.5.4's border around one, so there is no
/// relief to draw inside it.
///
/// **The pair is the discriminating part**: the identical `/BS` on a link *is* §12.5.4's border,
/// where Table 168's `S` entry applies in full and `B` is "[a] simulated embossed rectangle that
/// appears to be raised above the surface of the page" — the relief ADR 1061 chooses, inside the
/// line. A reader that asked the border dictionary the same question for both subtypes draws two
/// reliefs or none.
///
/// `/Rect [20 20 80 60]` and a line four wide: the line is the band within four of each side and
/// the relief the band within eight, so (26, 40) is on the left band, (74, 40) on the right,
/// (50, 54) on the top and (50, 26) on the bottom. The light falls from the upper left, so a
/// raised rectangle's top and left are lit and its bottom and right are shaded.
#[test]
fn a_squares_border_style_raises_nothing_and_a_links_is_drawn_in_relief() {
    let square = render(pdf_with(
        "<< /Type /Annot /Subtype /Square /Rect [20 20 80 60] /F 4 /C [0 1 0] \
         /BS << /W 4 /S /B >> >>",
        "/BBox [0 0 10 10]",
        "",
    ));
    assert_eq!(
        colour_at(&square, 22, 40),
        (0, 255, 0),
        "Table 180's rectangle"
    );
    for (x, y) in [(26, 40), (74, 40), (50, 54), (50, 26)] {
        assert!(
            !painted(&square, x, y),
            "({x}, {y}) is inside a square's line, where its /BS states nothing"
        );
    }

    let link = render(pdf_with(
        "<< /Type /Annot /Subtype /Link /Rect [20 20 80 60] /C [0 1 0] \
         /BS << /W 4 /S /B >> >>",
        "/BBox [0 0 10 10]",
        "",
    ));
    assert_eq!(
        colour_at(&link, 22, 40),
        (0, 255, 0),
        "the line is Table 166's /C"
    );
    assert_eq!(
        colour_at(&link, 26, 40),
        (255, 255, 255),
        "the left of a raised rectangle is lit"
    );
    assert_eq!(
        colour_at(&link, 50, 54),
        (255, 255, 255),
        "and so is its top"
    );
    assert!(
        shaded(colour_at(&link, 74, 40)),
        "its right is in shade: {:?}",
        colour_at(&link, 74, 40)
    );
    assert!(
        shaded(colour_at(&link, 50, 26)),
        "and so is its bottom: {:?}",
        colour_at(&link, 50, 26)
    );
    assert!(!painted(&link, 50, 40), "the relief is a frame, not a fill");
}

/// Whether a pixel is the relief's mid-grey, which eight-bit arithmetic puts at 127 or 128.
fn shaded(colour: (u8, u8, u8)) -> bool {
    colour.0 == colour.1 && colour.1 == colour.2 && (127..=128).contains(&colour.0)
}

/// Table 168's `I` is the same relief the other way round, on a widget's Table 192 border.
///
/// "A simulated engraved rectangle that appears to be recessed below the surface of the page":
/// under the same light, a recessed rectangle's top and left are in shade and its bottom and
/// right are lit. The widget's `/BG` is what the relief sits on — the shaded band is opaque grey
/// over the yellow, and the lit band is white over it, not yellow — because the illusion is of
/// the page's surface and not a tint of the background (ADR 1061).
///
/// Same rectangle and width as the link above, so the same four probes.
#[test]
fn an_inset_border_is_shaded_on_the_top_and_left_and_lit_on_the_bottom_and_right() {
    let widget = interpret(pdf_with(
        "<< /Type /Annot /Subtype /Widget /FT /Btn /Ff 65536 /T (b) /Rect [20 20 80 60] /F 4 \
         /MK << /BC [0 0 1] /BG [1 1 0] >> /BS << /W 4 /S /I >> >>",
        "/BBox [0 0 10 10]",
        "",
    ));
    assert!(
        widget.unsupported.is_empty(),
        "an inset border is drawn, not reported: {:?}",
        widget.unsupported
    );
    let widget = render_incomplete(&widget);
    assert_eq!(
        colour_at(&widget, 22, 40),
        (0, 0, 255),
        "the line is Table 192's /BC"
    );
    assert!(
        shaded(colour_at(&widget, 26, 40)),
        "the left of a recessed rectangle is in shade: {:?}",
        colour_at(&widget, 26, 40)
    );
    assert!(
        shaded(colour_at(&widget, 50, 54)),
        "and so is its top: {:?}",
        colour_at(&widget, 50, 54)
    );
    assert_eq!(
        colour_at(&widget, 74, 40),
        (255, 255, 255),
        "its right is lit"
    );
    assert_eq!(
        colour_at(&widget, 50, 26),
        (255, 255, 255),
        "and so is its bottom"
    );
    assert_eq!(
        colour_at(&widget, 50, 40),
        (255, 255, 0),
        "the background shows inside the relief"
    );
}

/// A bevelled border's relief is part of the border, so a field's text starts inside it.
///
/// §12.5.4 puts the border "completely inside the annotation rectangle", and what a text field
/// has left for §12.7.4.3's layout is the rectangle inset by the whole of it. With `/S /S` and a
/// width of four the text may begin at 24; with `/S /B` the lit band occupies 24 to 28 and the
/// first glyph begins past it. Same field, one name changed.
#[test]
fn a_fields_text_begins_inside_the_relief_of_a_bevelled_border() {
    let leftmost_ink = |style: &str| {
        let widget = interpret(pdf_with(
            &format!(
                "<< /Type /Annot /Subtype /Widget /FT /Tx /T (t) /V (M) /Rect [20 20 80 60] /F 4 \
                 /DA (/Helv 12 Tf 0 g) /MK << /BC [0 0 1] >> /BS << /W 4 /S /{style} >> >>"
            ),
            "/BBox [0 0 10 10]",
            "",
        ));
        let raster = render_incomplete(&widget);
        // Black is the text: the line is blue and the relief is white or grey.
        (0..raster.width)
            .find(|&x| {
                (0..raster.height)
                    .any(|y| colour_at(&raster, x, y) == (0, 0, 0) && painted(&raster, x, y))
            })
            .expect("the field's value is drawn")
    };
    let solid = leftmost_ink("S");
    let bevelled = leftmost_ink("B");
    assert!(
        (24..28).contains(&solid),
        "text begins just inside a solid border: {solid}"
    );
    assert!(
        bevelled >= 28,
        "and past a bevelled border's relief: {bevelled}"
    );
    assert_eq!(bevelled, solid + 4, "moved by exactly the relief's width");
}

/// Table 169's cloudy border is drawn as scallops inside the square, not refused and not straight.
///
/// §12.5.4 makes the effect a `shall` — a `/BE` "specifies an effect that shall be applied to the
/// border of the annotations" — and Table 169 states its look in words alone: "a series of convex
/// curved line segments in a manner that simulates the appearance of a cloud". The geometry is
/// ADR 1057's choice; what this test pins is the reading, with the arithmetic that follows from it.
///
/// `/Rect [10 10 90 90]`, a line two wide and `/I 1`: the line's path is the rectangle inset by
/// one, `[11 11 89 89]`, the cusps sit a radius of four inside that, `[15 15 85 85]`, and each
/// seventy-point edge takes nine scallops of radius 3.89. So the middle of the left edge is an
/// apex reaching back out to x ≈ 11.1 — exactly where the straight line was — while the corner
/// `(11, 11)` a straight square inks is four points from the nearest cusp and bare, and so is the
/// inscribed line just below the corner cusp, where the first scallop of the left edge has not
/// yet bulged out to it. **A straight square paints both of those pixels**, which is what makes
/// them the discriminating half; the apex is the half that says the cloud reaches the inscribed
/// rectangle and no further.
#[test]
fn a_cloudy_square_is_scalloped_inside_its_rectangle() {
    let raster = render(pdf_with(
        "<< /Type /Annot /Subtype /Square /Rect [10 10 90 90] /F 4 /C [1 0 0] \
         /BS << /W 2 >> /BE << /S /C /I 1 >> >>",
        "/BBox [0 0 10 10]",
        "",
    ));
    assert!(
        painted(&raster, 11, 50),
        "an apex reaches the inscribed line"
    );
    assert!(
        painted(&raster, 14, 22),
        "an arc leaves the cusp four points inside it"
    );
    assert!(
        !painted(&raster, 11, 11),
        "the corner a straight square inks is bare"
    );
    assert!(
        !painted(&raster, 11, 14),
        "and so is the inscribed line just below the corner cusp"
    );
}

/// A cloudy circle's cusps lie on an ellipse a radius inside the line, spaced by arc length.
///
/// With no `/I` the intensity is Table 169's default 0 and the radius two, so the cusp ellipse is
/// `[13 13 87 87]` and its first cusp, at angle zero, is `(87, 50)` — a pixel the straight
/// ellipse, whose line runs from x = 88 to 90 there, leaves bare.
#[test]
fn a_cloudy_circle_has_a_cusp_where_the_straight_ellipse_has_none() {
    let raster = render(pdf_with(
        "<< /Type /Annot /Subtype /Circle /Rect [10 10 90 90] /F 4 /C [0 0 1] \
         /BS << /W 2 >> /BE << /S /C >> >>",
        "/BBox [0 0 10 10]",
        "",
    ));
    assert!(painted(&raster, 87, 50), "the cusp at angle zero");
    assert!(!painted(&raster, 84, 50), "and nothing further in");
}

/// A polygon's cloud bulges outward from its own vertices; a polyline's `/BE` is not read.
///
/// Table 181's `/BE` is "(Optional; meaningful only for polygon annotations)", so the same entry
/// on a polyline is a key the standard defines nothing for: the polyline is drawn straight and
/// nothing is reported, where refusing it would name a gap the clause does not state (trap 11).
/// The polygon is a sixty-point square at `/I 2`, radius six: five scallops an edge, the first
/// apex of the bottom edge at `(26, 14)`, and the straight edge at `(26, 20)` bare.
#[test]
fn a_cloudy_polygon_bulges_outward_and_a_polylines_effect_is_not_meaningful() {
    let polygon = render(pdf_with(
        "<< /Type /Annot /Subtype /Polygon /Rect [0 0 100 100] /F 4 /C [0 0 1] \
         /Vertices [20 20 80 20 80 80 20 80] /BS << /W 2 >> /BE << /S /C /I 2 >> >>",
        "/BBox [0 0 10 10]",
        "",
    ));
    assert!(
        painted(&polygon, 26, 13),
        "an apex six points outside the edge"
    );
    assert!(
        !painted(&polygon, 26, 20),
        "the straight edge under it is bare"
    );

    let polyline = render(pdf_with(
        "<< /Type /Annot /Subtype /PolyLine /Rect [0 0 100 100] /F 4 /C [0 0 1] \
         /Vertices [20 20 80 20 80 80 20 80] /BS << /W 2 >> /BE << /S /C /I 2 >> >>",
        "/BBox [0 0 10 10]",
        "",
    ));
    assert!(
        painted(&polyline, 26, 20),
        "a polyline's edge stays straight"
    );
    assert!(!painted(&polyline, 26, 13), "and nothing bulges from it");
}

/// A cloudy `/Path` polygon is scalloped too, its curve flattened into one edge of the outline.
///
/// Table 181's `/Path` supersedes `/Vertices`; this one is a square whose top side is a curve
/// bowing upward to y = 95 at its middle, and the cloud follows the curve rather than a chord
/// of it. The flattened curve is 73.1 long, so at `/I 1` it takes nine scallops of radius 4.06
/// and its middle is an apex, at about y = 99; the chord's middle, y = 80, is bare.
#[test]
fn a_cloudy_path_polygon_follows_its_curve() {
    let raster = render(pdf_with(
        "<< /Type /Annot /Subtype /Polygon /Rect [0 0 100 120] /F 4 /C [0 0 1] \
         /Path [[20 20] [80 20] [80 80] [80 100 20 100 20 80]] /BS << /W 2 >> \
         /BE << /S /C /I 1 >> >>",
        "/BBox [0 0 10 10]",
        "",
    ));
    assert!(painted(&raster, 50, 98), "an apex above the curve's crown");
    assert!(
        !painted(&raster, 50, 80),
        "the chord a straight reading would draw is bare"
    );
}

/// §12.5.6.22: "Watermark annotations shall have no popup window nor other interactive
/// elements." So the pointer does not land on one — `annotation_at`, which every press, hover
/// and §12.6.3 trigger goes through, answers `None` over a watermark's rectangle — and the same
/// rectangle under a square, whose flags are identical, is found. The `/AP` `/D` the fixture
/// gives both is the interactive element the sentence forbids a watermark to have.
#[test]
fn a_watermark_is_not_under_the_pointer_and_a_square_is() {
    let found = |subtype: &str| {
        let bytes = pdf_with(
            &format!(
                "<< /Type /Annot /Subtype /{subtype} /Rect [10 10 90 90] /F 4 \
                 /AP << /N 6 0 R /D 6 0 R >> >>"
            ),
            "/BBox [0 0 80 80]",
            "1 0 0 rg 0 0 80 80 re f",
        );
        let document = Document::open(bytes).expect("the fixture is a valid PDF");
        let page = pdf_model::Pages::new(&document).get(0).expect("page one");
        let view = pdf_model::view::ViewState::of(&document);
        pdf_model::view::annotation_at(&document, &page, &view, 50.0, 50.0).is_some()
    };
    assert!(
        !found("Watermark"),
        "a watermark has no interactive elements"
    );
    assert!(
        found("Square"),
        "and the same rectangle on a square is live"
    );
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

/// A border as wide as its rectangle covers the rectangle, and still nothing outside it.
///
/// §12.5.4 puts the border "completely inside the annotation rectangle" and Table 168 makes `/W`
/// "[t]he border width in points", so the border of width *w* is the part of the rectangle within
/// *w* of its boundary. Once *w* reaches either dimension the bands from opposite sides meet and
/// that region *is* the rectangle — there is no frame left, and this is the width at which a
/// stroke stops being able to state it: the path inset by *w*/2 has a zero-length pair of sides,
/// and a stroke of those two sides covers nothing.
///
/// The three widths below are the three regimes, and the middle one is the boundary rather than a
/// round number: 40 is exactly the rectangle's height.
///
/// **Each of the last two drew the wrong picture until the seven-hundred-and-fifty-sixth
/// session** — a band as wide as the rectangle minus the width where the region is the whole
/// rectangle, and, past both dimensions, nothing at all. `bug1552113.pdf` is the corpus's own
/// witness, and its content stream says "this text should be visible" under an annotation whose
/// `/Border` is 112 units on a 20-unit-tall rectangle (ADR 0674).
#[test]
fn a_border_as_wide_as_its_rectangle_covers_it_and_no_more() {
    for width in ["20", "40", "400"] {
        let raster = render(pdf_with(
            &format!(
                "<< /Type /Annot /Subtype /Link /Rect [20 20 80 60] /C [0 1 0] \
                 /Border [0 0 {width}] >>"
            ),
            "/BBox [0 0 10 10]",
            "",
        ));

        // A 20-unit border on a 60 × 40 rectangle still leaves a 20 × 0 hole, which is no hole:
        // every width from the height upwards paints the rectangle and only the rectangle.
        assert_eq!(extent(&raster), (20, 20, 79, 59), "/Border [0 0 {width}]");
        assert_eq!(
            colour_at(&raster, 50, 40),
            (0, 255, 0),
            "the middle of the rectangle, under a /Border [0 0 {width}]"
        );
        assert!(!painted(&raster, 19, 40), "left of /Rect: {width}");
        assert!(!painted(&raster, 80, 40), "right of /Rect: {width}");
        assert!(!painted(&raster, 50, 19), "above /Rect: {width}");
        assert!(!painted(&raster, 50, 60), "below /Rect: {width}");
    }
}

/// The same width on §12.5.6.8's square, whose `/BS` states the line and not a border style.
///
/// Table 180 gives that entry "the line width and dash pattern that shall be used in drawing the
/// rectangle or ellipse", so this subtype's wide line is not §12.5.4's border — but §12.5.4's
/// sentence still binds where the ink may fall, and the geometry that follows is the same one:
/// a line as wide as the shape covers the shape. `/IC` is under it whole and is not seen.
#[test]
fn a_square_whose_line_is_wider_than_it_is_covered_by_that_line() {
    let raster = render(pdf_with(
        "<< /Type /Annot /Subtype /Square /Rect [20 20 80 60] /C [0 1 0] /IC [1 0 0] \
         /BS << /W 400 >> >>",
        "/BBox [0 0 10 10]",
        "",
    ));

    assert_eq!(extent(&raster), (20, 20, 79, 59));
    assert_eq!(
        colour_at(&raster, 50, 40),
        (0, 255, 0),
        "the line's own colour, not /IC's"
    );
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

/// A widget with a background draws it *and* reports the value it cannot lay out.
///
/// Table 192's `/BG` is derivable and this fixture's value is not, so both statements are made:
/// the frame is on the page and the text is named as missing. Suppressing either would lose
/// information — the same pairing `/NeedAppearances` and `/Matte` already use.
///
/// **What makes the value underivable here is the absence of a `/DA`, not §12.7.4.3.** This
/// comment said "§12.7.4.3's variable text is not [derivable]" until the
/// three-hundred-and-eighty-seventh session, and it had been false since the twenty-third:
/// `appearance::field_text` lays a text field's `/V` out. The fixture states no `/DA` anywhere up
/// its chain, which is `variable_text::Owed::NoFont` — one of the eight cases that still report —
/// and the clause named in the report is §12.7.4.3 because that is the clause requiring the entry:
/// "[a]t a minimum, the string shall include a Tf (text font) operator along with its two
/// operands". §12.5.6.19's ledger row carried the same misreading for the same reasons.
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

/// PDF 2.0's `/Path` on an ink annotation, drawn as the operands Table 185 defines.
///
/// ISO 32000-2 §12.5.6.13, Table 185:
///
/// > An array of n arrays, each supplying the operands for a path building operator ( m , l
/// > or c ). … The first array shall be of length 2 and specifies the operand of a moveto
/// > operator which establishes a current point. Subsequent arrays of length 2 specify the
/// > operands of lineto operators. Arrays of length 6 specify the operands for curveto
/// > operators.
///
/// # The precedence here is a choice, and Table 185 does not state it
///
/// §12.5.6.9's Table 181 makes `/Vertices` "(Required unless a Path key is present, in which
/// case it shall be ignored)". **Table 185 says no such thing of `/InkList`**: it is
/// "(Required)" flatly, and `/Path` is "(Optional; PDF 2.0)" beside it, with no sentence
/// ordering the two. So a file stating both leaves a processor to decide, and this one draws
/// the `/Path` — the entry that can carry curves, and the same answer §12.5.6.9 is *told* to
/// give — rather than both, which would put the scribble on the page twice. The second half
/// of this test is what makes that choice visible rather than assumed.
#[test]
fn an_ink_annotations_path_is_drawn_and_outranks_its_ink_list() {
    // A moveto, a lineto, and a curveto whose four control points are collinear, so the
    // curve is the straight segment from (50, 20) to (80, 20) and a pixel on it is an
    // assertion about the operands rather than about a flattening tolerance.
    let path = "/Path [[20 20] [50 20] [60 20 70 20 80 20]]";
    let raster = render(pdf_with(
        &format!(
            "<< /Type /Annot /Subtype /Ink /Rect [10 10 90 90] /F 4 /C [0 0 0] \
             /BS << /W 2 >> {path} >>"
        ),
        "/BBox [0 0 10 10]",
        "",
    ));

    assert!(painted(&raster, 35, 20), "the lineto's segment");
    assert!(painted(&raster, 70, 20), "the curveto's");
    assert!(
        !painted(&raster, 50, 60),
        "and nothing where no operand goes"
    );

    let both = render(pdf_with(
        &format!(
            "<< /Type /Annot /Subtype /Ink /Rect [10 10 90 90] /F 4 /C [0 0 0] \
             /BS << /W 2 >> /InkList [[20 80 80 80]] {path} >>"
        ),
        "/BBox [0 0 10 10]",
        "",
    ));
    assert!(painted(&both, 35, 20), "the /Path is drawn");
    assert!(
        !painted(&both, 50, 80),
        "and the /InkList beside it is not, which is this crate's choice and not Table 185's rule"
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

/// Table 191's last `/H` sentence: a mode other than `P` takes the down appearance away.
///
/// ISO 32000-2 §12.5.6.19:
///
/// > A highlighting mode other than P shall override any down appearance defined for the
/// > annotation.
///
/// So a widget stating `/H /N` beside a `/D` shows its **normal** stream while the button is
/// down and draws nothing over it, where the same widget with no `/H` shows the `/D` its
/// producer wrote (`the_pointer_chooses_between_an_annotations_appearances`, the pair this is
/// read against). `/H /O` is the other direction of the same sentence: the normal stream again,
/// with §11.3.5.2's Difference border over it, which is the clause's own `f(x) = 1 - x`.
///
/// **The sentence is a widget's alone**, which is why the fixture states `/Subtype /Widget`:
/// Table 176 gives a link's `/H` four modes with no such sentence and a `P` that names no
/// appearance stream, so §12.5.5's "down appearance shall be used when the mouse button is
/// pressed" is not overridden anywhere else.
///
/// No corpus document can decide this (trap 8): `examples/push_button_census` finds 7 of the
/// corpus's 833 widgets stating an `/H` at all — six `P` and one `N` — and not one of them
/// states a `/D` for a mode to override.
#[test]
fn a_widgets_highlighting_mode_overrides_its_down_appearance() {
    let colour_under = |highlight: &str, x: u32, y: u32| {
        let bytes = with_down_appearance(pdf_with(
            &format!(
                "<< /Type /Annot /Subtype /Widget /Rect [10 10 90 90] /F 4 {highlight} \
                 /AP << /N 6 0 R /D 7 0 R >> >>"
            ),
            "/BBox [0 0 80 80]",
            "1 0 0 rg 0 0 80 80 re f",
        ));
        let document = Document::open(bytes).expect("the fixture is a valid PDF");
        let page = pdf_model::Pages::new(&document).get(0).expect("page one");
        let mut state = pdf_model::view::ViewState::of(&document);
        state.set_pointer(Some((
            pdf_syntax::ObjectId {
                number: 5,
                generation: 0,
            },
            pdf_model::view::Pointer::Down,
        )));
        let list = pdf_model::content::interpret_with(&document, &page, &state).display_list;
        let target = TargetSpec::for_page(&list, 1.0, 1 << 20).expect("target");
        let raster = CpuRasterizer::new()
            .rasterize(&list, target)
            .expect("supported");
        colour_at(&raster, x, y)
    };

    assert_eq!(
        colour_under("/H /N", 50, 50),
        (255, 0, 0),
        "`N` is no highlighting, so the press shows the normal stream and nothing over it"
    );
    assert_eq!(
        colour_under("/H /O", 50, 50),
        (255, 0, 0),
        "`O` strokes the border, so the middle of the normal stream is untouched"
    );
    assert_eq!(
        colour_under("/H /I", 50, 50),
        (0, 255, 255),
        "`I` inverts the normal stream's red rather than showing the blue `/D`"
    );
    assert_eq!(
        colour_under("", 50, 50),
        (0, 0, 255),
        "the control: no `/H` at all beside a `/D` is read as `P`, and the `/D` is shown"
    );
}

/// §12.5.3's `ToggleNoView`, Table 167 bit 9:
///
/// > If set, invert the interpretation of the NoView flag for annotation selection and mouse
/// > hovering, causing the annotation to be visible when the mouse pointer hovers over the
/// > annotation or when the annotation is selected.
///
/// So the flag is a pointer-dependent *reading* of `NoView` rather than a second suppression,
/// and the exclusive-or is the sentence. Both directions are checked here, because the clause
/// states an inversion and not a reveal: `NoView` with the toggle appears under the cursor, and
/// the toggle alone disappears under it.
///
/// **No corpus document states bit 9**, on a scan of every uncompressed `/F` in all 974, so this
/// is a hand-built fixture and says so. What made the flag unreachable was never the corpus: a
/// `NoView` annotation could not be hovered, because `annotation_at` filters by
/// `annotation::interacts` and that returned `false` for it — a flag whose whole effect is
/// conditioned on a hover it prevented.
#[test]
fn toggle_no_view_inverts_no_view_under_the_pointer() {
    let annotation = pdf_syntax::ObjectId {
        number: 5,
        generation: 0,
    };
    // Bit 6 is NoView (32) and bit 9 is ToggleNoView (256); bit 3, Print (4), is set on every
    // case so that the flag words differ in nothing else.
    let drawn = |flags: u32, pointer: Option<pdf_model::view::Pointer>| {
        let bytes = pdf_with(
            &format!(
                "<< /Type /Annot /Subtype /Square /Rect [10 10 90 90] /F {flags} \
                 /AP << /N 6 0 R >> >>"
            ),
            "/BBox [0 0 80 80]",
            "1 0 0 rg 0 0 80 80 re f",
        );
        let document = Document::open(bytes).expect("the fixture is a valid PDF");
        let page = pdf_model::Pages::new(&document).get(0).expect("page one");
        let mut state = pdf_model::view::ViewState::of(&document);
        state.set_pointer(pointer.map(|pointer| (annotation, pointer)));
        let list = pdf_model::content::interpret_with(&document, &page, &state).display_list;
        let target = TargetSpec::for_page(&list, 1.0, 1 << 20).expect("target");
        let raster = CpuRasterizer::new()
            .rasterize(&list, target)
            .expect("supported");
        let at = ((50 * raster.width) + 50) as usize * 4;
        (raster.data[at], raster.data[at + 1], raster.data[at + 2])
    };
    let red = (255, 0, 0);
    let blank = drawn(4 | 32, None);
    assert_ne!(
        blank, red,
        "NoView draws nothing: the flag this one inverts"
    );

    // NoView alone: nothing, wherever the pointer is.
    assert_eq!(drawn(4 | 32, Some(pdf_model::view::Pointer::Over)), blank);

    // NoView and ToggleNoView: nothing until the cursor arrives, which is the clause's own
    // "typical use" and the only half a reader is likely to meet.
    assert_eq!(drawn(4 | 32 | 256, None), blank);
    assert_eq!(
        drawn(4 | 32 | 256, Some(pdf_model::view::Pointer::Over)),
        red
    );
    assert_eq!(
        drawn(4 | 32 | 256, Some(pdf_model::view::Pointer::Down)),
        red
    );

    // ToggleNoView alone: the inversion the other way, which the sentence states and no producer
    // is likely to write. Drawn normally, and *not* while the cursor is on it.
    assert_eq!(drawn(4 | 256, None), red);
    assert_eq!(drawn(4 | 256, Some(pdf_model::view::Pointer::Over)), blank);

    // And a plain annotation is drawn whatever the pointer does.
    assert_eq!(drawn(4, None), red);
    assert_eq!(drawn(4, Some(pdf_model::view::Pointer::Over)), red);
}

/// §12.5.6.4's icon is one size whatever the `/Rect` states, because the clause states a point.
///
/// > A text annotation represents a "sticky note" attached to a point in the PDF document. When
/// > closed, the annotation shall appear as an icon
///
/// A `shall` about the icon, and a *point* rather than a rectangle — so a `/Rect` with no area is
/// not this annotation saying it covers nothing, the way a `Square`'s would be. The same clause's
/// next sentence gives the size:
///
/// > Text annotations shall not scale and rotate with the page; they shall behave as if the
/// > NoZoom and NoRotate annotation flags (see "Table 167 -Annotation flags") were always set.
///
/// and §12.5.3 says what that flag means:
///
/// > If the NoZoom flag is set, the annotation shall always maintain the same fixed size on the
/// > screen and shall be unaffected by the magnification level at which the page itself is
/// > displayed.
///
/// A size fixed on the screen is by definition not `/Rect`'s, which is stated in default user
/// space. **Neither sentence mentions the rectangle's area**, so neither does this: the icon is
/// the same square hung from Table 167's own fixed point — "the upper-left corner of its
/// annotation rectangle" — for a degenerate `/Rect` and for a four-hundred-unit one alike.
///
/// **How big is a choice**, like the artwork itself, and it is twenty units. What is *not* a
/// choice is that something is drawn: `rc_annotation.pdf` is one corpus witness with
/// `/Rect [50 50 50 50]`, and this tree drew nothing for it inside an `ambiguous` verdict until
/// `doc/todo/00`'s step 7 sweep put it at −1.783 of 255 — two of four references draw an icon.
/// The other end of the same rule is `1407194.pdf`, whose `/Rect [0 542 400 792]` had a
/// 250-unit icon over a quarter of a book cover.
#[test]
fn a_text_annotations_icon_is_one_size_whatever_its_rect_states() {
    let ink = |rect: &str| {
        let bytes = pdf_with(
            &format!("<< /Type /Annot /Subtype /Text /Name /Note /Rect {rect} /F 4 >>"),
            "/BBox [0 0 80 80]",
            "",
        );
        let document = Document::open(bytes).expect("the fixture is a valid PDF");
        let page = pdf_model::Pages::new(&document).get(0).expect("page one");
        let interpretation = pdf_model::interpret(&document, &page);
        assert!(
            interpretation.is_complete(),
            "{:?}",
            interpretation.unsupported
        );
        let list = interpretation.display_list;
        let target = TargetSpec::for_page(&list, 4.0, 1 << 22).expect("target");
        let raster = CpuRasterizer::new()
            .rasterize(&list, target)
            .expect("supported");
        raster
            .data
            .chunks_exact(4)
            .filter(|pixel| pixel[0] < 128)
            .count()
    };

    // The corpus witness's shape: a point, with the icon hanging below and right of it.
    let point = ink("[50 50 50 50]");
    assert!(point > 0, "a text annotation attached to a point draws");

    // And a rectangle with an area is the same icon, because the clause gives the size and the
    // rectangle gives only the corner it hangs from. `1407194.pdf`'s is the second of these.
    // The page these fixtures state is 100 by 100, so a rectangle is chosen for its shape rather
    // than for `1407194.pdf`'s own numbers: wide, tall, and one already the icon's size.
    for rect in ["[10 10 90 90]", "[10 40 90 60]", "[10 60 30 80]"] {
        assert_eq!(
            ink(rect),
            point,
            "{rect} draws the same icon as a point does"
        );
    }

    // A subtype whose clause states no fixed size keeps the general rule — no area, no marks.
    let square = pdf_with(
        "<< /Type /Annot /Subtype /Square /Rect [50 50 50 50] /F 4 /IC [1 0 0] >>",
        "/BBox [0 0 80 80]",
        "",
    );
    let document = Document::open(square).expect("the fixture is a valid PDF");
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    assert!(
        pdf_model::interpret(&document, &page)
            .display_list
            .commands()
            .is_empty(),
        "a Square covering no area draws nothing, which is Table 166's own excuse"
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

    assemble(&body)
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

/// An entry that states no *size* must not erase the shape the clause does state.
///
/// ISO 32000-2 §12.5.6.7 makes `/L` required and `/LE` optional with a default of
/// `[/None /None]`, and Table 179 gives its ten endings names and not one dimension. Both halves
/// of that were once a refusal and each was removed by the same argument, six rounds apart: the
/// line is drawn whatever its ends ask for (session 116), and since the
/// three-hundred-and-fourteenth the ends are drawn too, at a size taken from the only length the
/// annotation supplies — §12.5.4's border width (ADR 0192).
///
/// `/Cap` was the third of these and is drawn since the five-hundred-and-seventy-fourth
/// session; what is asserted below is that an *empty* one owes nothing, because Table 178
/// replicates "the text specified by the Contents or RC entries" and this fixture states neither.
#[test]
fn a_line_ending_is_drawn_and_an_empty_caption_owes_nothing() {
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
        ended.is_complete(),
        "and one with an ending owes nothing either now: {:?}",
        ended.unsupported
    );
    assert!(
        ended.display_list.commands().len() > plain.display_list.commands().len(),
        "the arrowheads are marks the plain line does not have"
    );

    // §12.5.6.7's caption replicates text this fixture does not state, so it asks for nothing.
    let captioned = of("/LE [/Square /Square] /Cap true");
    assert!(
        !captioned.display_list.commands().is_empty(),
        "the line the clause requires is drawn whatever the caption asks for"
    );
    assert!(
        captioned.is_complete(),
        "a /Cap with no /Contents and no /RC has nothing to replicate: {:?}",
        captioned.unsupported
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
    assert!(polyline.is_complete(), "and its endings are drawn with it");
}

/// §12.5.6.19's `/H`, which is a clause about a *moment* and became reachable when this program
/// grew a pointer.
///
/// ISO 32000-2 §12.5.6.19, Table 192:
///
/// > The annotation's highlighting mode , the visual effect that shall be used when the mouse
/// > button is pressed or held down inside its active area: N (None) No highlighting. I (Invert)
/// > Invert the colours used to display the contents of the annotation rectangle.
///
/// The clause states the effect as arithmetic — `f(x) = 1 - x` on every channel — so the
/// assertion is the arithmetic: a red widget under a press with `/H /I` is cyan, exactly.
/// Nothing about the display list would distinguish "we drew the mark" from "we drew it in the
/// wrong colour", which is why this is a pixel.
#[test]
fn a_press_inverts_a_widget_whose_highlighting_mode_says_to() {
    let colour_of = |annotation: &str, pointer: Option<pdf_model::view::Pointer>| {
        let bytes = pdf_with(annotation, "/BBox [0 0 80 80]", "1 0 0 rg 0 0 80 80 re f");
        let document = Document::open(bytes).expect("the fixture is a valid PDF");
        let page = pdf_model::Pages::new(&document).get(0).expect("page one");
        let mut state = pdf_model::view::ViewState::of(&document);
        if let Some(pointer) = pointer {
            state.set_pointer(Some((
                pdf_syntax::ObjectId {
                    number: 5,
                    generation: 0,
                },
                pointer,
            )));
        }
        let list = pdf_model::content::interpret_with(&document, &page, &state).display_list;
        let target = TargetSpec::for_page(&list, 1.0, 1 << 20).expect("target");
        let raster = CpuRasterizer::new()
            .rasterize(&list, target)
            .expect("supported");
        let at = ((50 * raster.width) + 50) as usize * 4;
        (raster.data[at], raster.data[at + 1], raster.data[at + 2])
    };

    let inverting = "<< /Type /Annot /Subtype /Widget /Rect [10 10 90 90] /F 4 /H /I \
                     /AP << /N 6 0 R >> >>";
    assert_eq!(
        colour_of(inverting, None),
        (255, 0, 0),
        "nothing is pressing it"
    );
    assert_eq!(
        colour_of(inverting, Some(pdf_model::view::Pointer::Over)),
        (255, 0, 0),
        "the clause is about the button being *pressed*, not about the cursor being there"
    );
    assert_eq!(
        colour_of(inverting, Some(pdf_model::view::Pointer::Down)),
        (0, 255, 255),
        "f(x) = 1 - x on every channel"
    );

    // `/H /N` asks for nothing, which is the one mode that is not the default.
    let none = "<< /Type /Annot /Subtype /Widget /Rect [10 10 90 90] /F 4 /H /N \
                /AP << /N 6 0 R >> >>";
    assert_eq!(
        colour_of(none, Some(pdf_model::view::Pointer::Down)),
        (255, 0, 0),
        "/H /N: no highlighting"
    );

    // And the default is `I`, for an annotation that states no down appearance to have meant
    // `P` by — which is the reading `annotation::highlight` argues for and this pins.
    let unstated = "<< /Type /Annot /Subtype /Widget /Rect [10 10 90 90] /F 4 \
                    /AP << /N 6 0 R >> >>";
    assert_eq!(
        colour_of(unstated, Some(pdf_model::view::Pointer::Down)),
        (0, 255, 255),
        "Table 192: default value I"
    );

    // **And the default belongs to the entry, not to annotations in general.** Two tables
    // define `/H` — Table 176's link and Table 192's widget — so a subtype whose clause states
    // no such entry has no mode to default and a press draws no mark on it. Unreachable until
    // the two-hundred-and-fifty-third session took the pressed annotation from the whole page
    // rather than from its links, which is exactly when a latent default becomes a wrong pixel.
    let square = "<< /Type /Annot /Subtype /Square /Rect [10 10 90 90] /F 4 \
                  /AP << /N 6 0 R >> >>";
    assert_eq!(
        colour_of(square, Some(pdf_model::view::Pointer::Down)),
        (255, 0, 0),
        "a Square states no /H, so Table 192's default is not its default"
    );
}

/// `/H /O` inverts the border rather than the contents, at §12.5.4's width and inside the
/// rectangle.
///
/// ISO 32000-2 §12.5.6.19, Table 192, of the `O` mode: "Stroke the colours used to display the
/// annotation border."
///
/// Two pixels rather than one: the middle of the annotation is untouched and a pixel on the
/// border is inverted. A version that filled the rectangle would pass the second and fail the
/// first, which is the mistake worth catching.
#[test]
fn a_press_with_the_outline_mode_inverts_the_border_alone() {
    let bytes = pdf_with(
        "<< /Type /Annot /Subtype /Widget /Rect [10 10 90 90] /F 4 /H /O \
         /Border [0 0 4] /AP << /N 6 0 R >> >>",
        "/BBox [0 0 80 80]",
        "1 0 0 rg 0 0 80 80 re f",
    );
    let document = Document::open(bytes).expect("the fixture is a valid PDF");
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    let mut state = pdf_model::view::ViewState::of(&document);
    state.set_pointer(Some((
        pdf_syntax::ObjectId {
            number: 5,
            generation: 0,
        },
        pdf_model::view::Pointer::Down,
    )));
    let list = pdf_model::content::interpret_with(&document, &page, &state).display_list;
    let target = TargetSpec::for_page(&list, 1.0, 1 << 20).expect("target");
    let raster = CpuRasterizer::new()
        .rasterize(&list, target)
        .expect("supported");
    let pixel = |x: u32, y: u32| {
        let at = ((y * raster.width) + x) as usize * 4;
        (raster.data[at], raster.data[at + 1], raster.data[at + 2])
    };

    // The raster's y runs down from the page's top, so the annotation's [10 10 90 90] covers
    // rows 10 to 90 either way and its border is two units inside each edge.
    assert_eq!(pixel(50, 50), (255, 0, 0), "the contents are not inverted");
    assert_eq!(pixel(50, 12), (0, 255, 255), "the border is");
}

/// A fixture whose page states a `/Rotate`, and whose annotation's `/F` the caller chooses.
fn pdf_rotated(rotate: u16, flags: i64, appearance: &str) -> Vec<u8> {
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Rotate {rotate} \
         /Resources << >> /Contents 4 0 R /Annots [5 0 R] >>\nendobj\n\
         4 0 obj\n<< /Length 0 >>\nstream\n\nendstream\nendobj\n\
         5 0 obj\n<< /Type /Annot /Subtype /Square /Rect [40 40 70 70] /F {flags} \
         /AP << /N 6 0 R >> >>\nendobj\n\
         6 0 obj\n<< /Type /XObject /Subtype /Form /BBox [0 0 30 30] /Length {} >>\n\
         stream\n{appearance}\nendstream\nendobj\n",
        appearance.len().saturating_add(1)
    );

    assemble(&body)
}

/// Renders one of those, at a magnification the caller states or at none.
fn render_at(bytes: Vec<u8>, magnification: Option<f32>) -> pdf_render::Raster {
    let document = Document::open(bytes).expect("the fixture is a valid PDF");
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    let mut state = pdf_model::view::ViewState::of(&document);
    state.set_magnification(magnification);
    let interpretation = pdf_model::content::interpret_with(&document, &page, &state);
    assert!(
        interpretation.is_complete(),
        "the fixture should draw completely: {:?}",
        interpretation.unsupported
    );
    let list = interpretation.display_list;
    let target = TargetSpec::for_page(&list, 1.0, GENEROUS).expect("valid target");
    CpuRasterizer::new()
        .with_medium(pdf_render::Medium::NONE)
        .rasterize(&list, target)
        .expect("supported")
}

/// A one-page fixture whose only annotation is a watermark, built from the caller's numbers.
///
/// `/BBox` is `[0 0 10 10]` and the appearance fills it, so the mark in the raster *is* the
/// rectangle §12.5.5's algorithm placed the appearance onto — which is what §12.5.6.22 replaces.
///
/// **Hand-built, and `examples/fixed_print_census` is the reason.** One document under `doc/`
/// states a `/FixedPrint` at all, out of the 4172 that open — `isartor-6-5-2-t01-fail-d.pdf` of
/// the Isartor PDF/A-1b suite — and its transformed rectangle works out to exactly its own
/// `/Rect`, so applying the clause moves nothing on the one page in reach. The gate corpus states
/// none. A fixture that has to move is therefore built rather than found (trap 8).
fn pdf_watermark(media_box: &str, rect: &str, fixed_print: &str) -> Vec<u8> {
    let appearance = "0 0 0 rg 0 0 10 10 re f";
    assemble(&format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox {media_box} \
         /Resources << >> /Contents 4 0 R /Annots [5 0 R] >>\nendobj\n\
         4 0 obj\n<< /Length 0 >>\nstream\n\nendstream\nendobj\n\
         5 0 obj\n<< /Type /Annot /Subtype /Watermark /Rect {rect} /F 4 \
         /AP << /N 6 0 R >> {fixed_print} >>\nendobj\n\
         6 0 obj\n<< /Type /XObject /Subtype /Form /BBox [0 0 10 10] /Length {} >>\n\
         stream\n{appearance}\nendstream\nendobj\n",
        appearance.len().saturating_add(1)
    ))
}

/// §12.5.3's `NoRotate`: the annotation stays upright while the page turns under it.
///
/// > Similarly, if the NoRotate flag is set, the annotation shall retain its original
/// > orientation on the screen when the page is rotated (by changing the Rotate entry in the
/// > page object; see 7.7.3, "Page tree").
///
/// and Figure 78's NOTE names the fixed point: "[t]he upper-left corner of the annotation
/// remains at the same point in default user space; the annotation pivots around that point."
///
/// The fixture is a 100×100 page at `/Rotate 90` with a `/Rect [40 40 70 70]` — upper-left
/// corner (40, 70) — whose appearance fills only the **left half** of its box, so the mark is
/// asymmetric and a rotation of it is visible. Every number below is one composition of two
/// matrices and is checkable by hand:
///
/// ```text
/// the mark, in default user space         x 40..55   y 40..70
/// /Rotate 90 alone, (x, y) -> (y, 100-x)  x 40..70   y 45..60
/// pivoted first, (x, y) -> (110-y, x+30)  x 40..70   y 70..85
///   and then rotated                      x 70..85   y 30..60
/// ```
#[test]
fn a_no_rotate_annotation_pivots_about_its_own_corner() {
    // 4 is Print; 16 is NoRotate.
    let turned = render_at(pdf_rotated(90, 4, "0 0 0 rg 0 0 15 30 re f"), None);
    let upright = render_at(pdf_rotated(90, 20, "0 0 0 rg 0 0 15 30 re f"), None);

    assert!(
        painted(&turned, 50, 50) && !painted(&turned, 75, 45),
        "without the flag the mark turns with the page: {:?}",
        extent(&turned)
    );
    assert!(
        painted(&upright, 75, 45) && !painted(&upright, 50, 50),
        "with it the mark is where the counter-rotation puts it: {:?}",
        extent(&upright)
    );
    assert_eq!(
        extent(&upright),
        (70, 30, 84, 59),
        "and the whole mark is the quadrilateral the two matrices give"
    );
    // The clause's fixed point: (40, 70) in default user space is (70, 60) after `/Rotate 90`,
    // and it is a corner of the mark in both renders.
    assert_eq!(extent(&turned).2, 69, "the pivot is the mark's own corner");
}

/// §12.5.3's `NoZoom`: the annotation keeps its size while the page is magnified.
///
/// > If the NoZoom flag is set, the annotation shall always maintain the same fixed size on the
/// > screen and shall be unaffected by the magnification level at which the page itself is
/// > displayed.
///
/// A 30×30 `/Rect` at a magnification of 2 must be drawn 15 units across in the space that is
/// about to be doubled, so that it comes out 30 pixels — and it must hang off the same corner,
/// which is (40, 70).
///
/// **A magnification nobody stated is not 1.0**, which the third case checks: the corpus gate
/// and the oracle render a page at its own scale and say nothing about a zoom, and under that
/// the flag changes nothing at all.
#[test]
fn a_no_zoom_annotation_keeps_its_size_when_the_page_is_magnified() {
    // 4 is Print; 8 is NoZoom.
    let fixture = || pdf_rotated(0, 12, "0 0 0 rg 0 0 30 30 re f");

    let unstated = render_at(fixture(), None);
    assert_eq!(
        extent(&unstated),
        (40, 40, 69, 69),
        "no magnification stated, so the annotation is its own /Rect"
    );

    let doubled = render_at(fixture(), Some(2.0));
    assert_eq!(
        extent(&doubled),
        (40, 55, 54, 69),
        "at 2x it is drawn half as large, hanging off the upper-left corner (40, 70)"
    );

    let halved = render_at(fixture(), Some(0.5));
    assert_eq!(
        extent(&halved),
        (40, 10, 99, 69),
        "and at half it is drawn twice as large, off the same corner"
    );

    // Without the flag the magnification is not the interpreter's business at all.
    let plain = render_at(pdf_rotated(0, 4, "0 0 0 rg 0 0 30 30 re f"), Some(2.0));
    assert_eq!(extent(&plain), (40, 40, 69, 69));
}

/// §12.5.3's `NoZoom` does not reach a text markup annotation, and it is a choice.
///
/// Two `shall`s that cannot both hold at a magnification other than 1, with no precedence stated
/// between them. §12.5.3: "the annotation shall always maintain the same fixed size on the
/// screen". §12.5.6.10:
///
/// > Text markup annotations shall appear as highlights, underlines, strikeouts (all PDF 1.3), or
/// > jagged ("squiggly") underlines ( PDF 1.4 ) in the text of a document.
///
/// Obey the first and a strike-out at 200% covers half the words it struck out, at which point
/// the second is false. This tree obeys the second, because §12.5.6.10 says what the annotation
/// *is* and §12.5.3 offers a display option annotations have in general — argued in ADR 0172 and
/// counted first: 211 of the corpus's 511 text markup annotations carry `NoZoom` and all 211 are
/// in one document, at one flag value.
///
/// The fixture is `/F 220`, which is `ISO_32000-2_sponsored_EC3.pdf`'s own value on every one of
/// its 211 strike-outs: Locked, `ReadOnly`, `NoRotate`, **`NoZoom`**, Print.
#[test]
fn a_text_markup_annotation_scales_with_the_text_it_marks() {
    for subtype in ["StrikeOut", "Highlight", "Underline", "Squiggly"] {
        let fixture = || {
            pdf_with(
                &format!(
                    "<< /Type /Annot /Subtype /{subtype} /Rect [40 40 70 70] /F 220 \
                     /QuadPoints [40 70 70 70 40 40 70 40] /AP << /N 6 0 R >> >>"
                ),
                "/BBox [0 0 30 30]",
                "0 0 0 rg 0 0 30 30 re f",
            )
        };
        assert_eq!(
            extent(&render_at(fixture(), Some(2.0))),
            (40, 40, 69, 69),
            "{subtype} at 2x is still its own /Rect, so it still covers its words"
        );
        assert_eq!(
            extent(&render_at(fixture(), None)),
            (40, 40, 69, 69),
            "{subtype} unchanged where no magnification is stated"
        );
    }

    // And the exclusion is by subtype rather than by flag: a Square with the same /F 220 still
    // shrinks, which is what keeps this a reading of §12.5.6.10 and not a repeal of §12.5.3.
    let square = pdf_with(
        "<< /Type /Annot /Subtype /Square /Rect [40 40 70 70] /F 220 /AP << /N 6 0 R >> >>",
        "/BBox [0 0 30 30]",
        "0 0 0 rg 0 0 30 30 re f",
    );
    assert_eq!(
        extent(&render_at(square, Some(2.0))),
        (40, 55, 54, 69),
        "a Square keeps §12.5.3's behaviour"
    );
}

/// A page says whether its marks depend on the magnification, so a host knows when to re-ask.
///
/// The flag is what makes `NoZoom` affordable: a zoom re-rasterises the same display list, and
/// re-interpreting on every zoom step to catch 124 annotations in 51 documents would pay for
/// them on all 974. `NoRotate` is deliberately *not* here — the page's `/Rotate` is in the file,
/// so an annotation setting only that flag is as pure a function of the document as anything
/// else on the page.
#[test]
fn a_page_says_whether_its_marks_depend_on_the_magnification() {
    let view_dependent = |flags: i64| {
        let bytes = pdf_rotated(0, flags, "0 0 0 rg 0 0 30 30 re f");
        let document = Document::open(bytes).expect("the fixture is a valid PDF");
        let page = pdf_model::Pages::new(&document).get(0).expect("page one");
        pdf_model::interpret(&document, &page).view_dependent
    };
    assert!(!view_dependent(4), "Print alone");
    assert!(
        !view_dependent(20),
        "NoRotate alone is the file's own business"
    );
    assert!(view_dependent(12), "NoZoom");
    assert!(view_dependent(28), "and both");
}

/// §12.5.6.4 makes a text annotation behave as though both flags were set, whatever its `/F`.
///
/// > Text annotations shall not scale and rotate with the page; they shall behave as if the
/// > NoZoom and NoRotate annotation flags (see "Table 167 -Annotation flags") were always set.
///
/// A `shall` about the *subtype*, so the file's `/F` cannot turn it off — which is the only
/// thing this checks, since the arithmetic is `a_no_zoom_annotation_keeps_its_size_when_the_page_is_magnified`'s.
/// The fixture is a `Text` annotation with `/F 4`: Print alone, neither flag set.
#[test]
fn a_text_annotation_behaves_as_though_both_flags_were_set() {
    let text = |flags: i64, magnification: Option<f32>| {
        let bytes = pdf_with(
            &format!(
                "<< /Type /Annot /Subtype /Text /Rect [40 40 70 70] /F {flags} \
                 /AP << /N 6 0 R >> >>"
            ),
            "/BBox [0 0 30 30]",
            "0 0 0 rg 0 0 30 30 re f",
        );
        let document = Document::open(bytes).expect("the fixture is a valid PDF");
        let page = pdf_model::Pages::new(&document).get(0).expect("page one");
        let mut state = pdf_model::view::ViewState::of(&document);
        state.set_magnification(magnification);
        let interpretation = pdf_model::content::interpret_with(&document, &page, &state);
        let list = interpretation.display_list;
        let target = TargetSpec::for_page(&list, 1.0, GENEROUS).expect("valid target");
        let raster = CpuRasterizer::new()
            .with_medium(pdf_render::Medium::NONE)
            .rasterize(&list, target)
            .expect("supported");
        (extent(&raster), interpretation.view_dependent)
    };

    assert_eq!(
        text(4, Some(2.0)).0,
        (40, 55, 54, 69),
        "the subtype sets NoZoom, so a doubled page draws it half as large off (40, 70)"
    );
    assert_eq!(
        text(4, None).0,
        (40, 40, 69, 69),
        "and an unstated magnification still changes nothing"
    );
    assert!(
        text(4, None).1,
        "a page with a text annotation depends on the magnification whatever its /F says"
    );
}

/// Table 179's ten styles, each drawing its own shape and none drawing nothing.
///
/// The same argument as the seven icons one clause over: the table *describes* ten shapes and
/// gives not one dimension, so what is checked is that each name reaches a different picture —
/// a golden image would pin a size the standard does not state.
#[test]
fn each_of_table_179_s_line_endings_draws_its_own_shape() {
    let ending = |name: &str| {
        let raster = render(pdf_with(
            &format!(
                "<< /Type /Annot /Subtype /Line /Rect [0 0 100 100] /F 4 /L [30 50 70 50] \
                 /C [0 0 0] /IC [1 0 0] /BS << /W 3 >> /LE [/{name} /{name}] >>"
            ),
            "/BBox [0 0 10 10]",
            "",
        ));
        raster.data.clone()
    };
    let plain = ending("None");
    let names = [
        "Square",
        "Circle",
        "Diamond",
        "OpenArrow",
        "ClosedArrow",
        "ROpenArrow",
        "RClosedArrow",
        "Butt",
        "Slash",
    ];
    let drawn: Vec<Vec<u8>> = names.iter().map(|name| ending(name)).collect();
    for (name, pixels) in names.iter().zip(&drawn) {
        assert_ne!(*pixels, plain, "/{name} drew the line and nothing else");
    }
    for (first, one) in names.iter().enumerate() {
        for (second, other) in names.iter().enumerate().skip(first + 1) {
            assert_ne!(
                drawn[first], drawn[second],
                "/{one} and /{other} draw the same ending"
            );
        }
    }
}

/// Table 179 fills five of its ten with `/IC` and leaves the other five to the stroke.
///
/// **This said four and asserted five**, which is the arithmetic `Ending::filled`'s own doc
/// comment carried too: the published table names the fill on four rows and Errata Collection 3
/// Issue #515 puts it on `RClosedArrow` as well, which is the arm this crate had already derived
/// from "in the reverse direction from" `ClosedArrow`. Every one of the ten is named below now, so
/// the sentence and the assertions are the same list; `/None` is stated rather than asserted, and
/// honestly so — `draw_ending` returns before it looks at a colour, so that one arm cannot fail.
///
/// **And this test could not see the arrowheads at all until the same session.** `draw_ending`
/// decided their fill for itself, three arms below the one that asks `Ending::filled`, so removing
/// `RClosedArrow` from `filled` left every assertion here passing. Calibrated per trap 13 after
/// the two expressions became one: with that arm removed the run fails on `/RClosedArrow` by name.
#[test]
fn only_the_five_endings_table_179_fills_use_the_interior_colour() {
    let ending = |name: &str, interior: &str| {
        let raster = render(pdf_with(
            &format!(
                "<< /Type /Annot /Subtype /Line /Rect [0 0 100 100] /F 4 /L [30 50 70 50] \
                 /C [0 0 0] {interior} /BS << /W 3 >> /LE [/{name} /None] >>"
            ),
            "/BBox [0 0 10 10]",
            "",
        ));
        raster.data.clone()
    };
    for name in ["Square", "Circle", "Diamond", "ClosedArrow", "RClosedArrow"] {
        assert_ne!(
            ending(name, "/IC [1 0 0]"),
            ending(name, ""),
            "Table 179 fills /{name} with the annotation's interior colour"
        );
    }
    for name in ["OpenArrow", "ROpenArrow", "Butt", "Slash", "None"] {
        assert_eq!(
            ending(name, "/IC [1 0 0]"),
            ending(name, ""),
            "Table 179 does not fill /{name}, so /IC changes nothing"
        );
    }
}

/// The reverse styles are Table 179's own way of asking for the other direction.
#[test]
fn a_reversed_arrowhead_is_not_the_one_it_reverses() {
    let ending = |name: &str| {
        render(pdf_with(
            &format!(
                "<< /Type /Annot /Subtype /Line /Rect [0 0 100 100] /F 4 /L [30 50 70 50] \
                 /C [0 0 0] /BS << /W 3 >> /LE [/{name} /None] >>"
            ),
            "/BBox [0 0 10 10]",
            "",
        ))
        .data
        .clone()
    };
    assert_ne!(ending("OpenArrow"), ending("ROpenArrow"));
    assert_ne!(ending("ClosedArrow"), ending("RClosedArrow"));
}

/// A `/LE` naming something Table 179 does not is reported rather than dropped.
#[test]
fn a_line_ending_style_the_table_does_not_have_is_reported() {
    let interpretation = interpret(pdf_with(
        "<< /Type /Annot /Subtype /Line /Rect [0 0 100 100] /F 4 /L [30 50 70 50] /C [0 0 0] \
         /LE [/Wedge /None] >>",
        "/BBox [0 0 10 10]",
        "",
    ));
    assert!(
        format!("{:?}", interpretation.unsupported).contains("Table 179"),
        "{:?}",
        interpretation.unsupported
    );
}

/// Table 178's `/Cap` puts the annotation's own words on the line, which is a `shall`.
///
/// ISO 32000-2 §12.5.6.7, Table 178:
///
/// > If true , the text specified by the Contents or RC entries shall be replicated as a caption
/// > in the appearance of the line
///
/// The fixture is a horizontal line from (20, 50) to (80, 50) and the caption is centred on its
/// midpoint, so the assertion is about the *rows* the two placements mark: `Top` puts ink above
/// y = 50 and leaves the row below it clean, and `Inline` straddles the line. A reader that drew
/// no caption at all fails both.
#[test]
fn a_captioned_line_draws_its_contents_where_cp_states() {
    let captioned = |extra: &str| {
        interpret(pdf_with(
            &format!(
                "<< /Type /Annot /Subtype /Line /Rect [0 0 100 100] /F 4 /C [0 0 0] \
                 /BS << /W 1 >> /L [20 50 80 50] /Cap true /Contents (Hill) {extra} >>"
            ),
            "/BBox [0 0 10 10]",
            "",
        ))
    };

    let plain = interpret(pdf_with(
        "<< /Type /Annot /Subtype /Line /Rect [0 0 100 100] /F 4 /C [0 0 0] \
         /BS << /W 1 >> /L [20 50 80 50] /Contents (Hill) >>",
        "/BBox [0 0 10 10]",
        "",
    ));
    let top = captioned("/CP /Top");
    assert!(
        top.is_complete(),
        "the caption is drawn rather than named: {:?}",
        top.unsupported
    );
    assert!(
        top.display_list.commands().len() > plain.display_list.commands().len(),
        "a caption is glyphs the same line without /Cap does not have"
    );

    let ink = |interpretation: &pdf_model::Interpretation, from: u32, to: u32| {
        let raster = render_incomplete(interpretation);
        (from..to)
            .flat_map(|y| (0..100).map(move |x| (x, y)))
            .filter(|&(x, y)| painted(&raster, x, y))
            .count()
    };
    // The line itself is one unit wide about y = 50, so the bands either side of it are the
    // caption's alone.
    assert!(
        ink(&top, 52, 70) > 0,
        "'Top , meaning the caption shall be on top of the line'"
    );
    assert_eq!(
        ink(&top, 30, 48),
        0,
        "and nothing of it falls below the line"
    );

    let inline = captioned("");
    assert!(
        ink(&inline, 30, 48) > 0,
        "'Inline , meaning the caption shall be centred inside the line', which is the default"
    );

    // `/CO`'s second number is "the vertical offset perpendicular to the annotation line, with a
    // positive value indicating a shift up", so a shift moves the whole caption off the line.
    let shifted = captioned("/CO [0 20]");
    assert_eq!(
        ink(&shifted, 30, 48),
        0,
        "a caption shifted twenty units up leaves the rows it used to occupy"
    );
    assert!(
        ink(&shifted, 52, 80) > ink(&inline, 52, 80),
        "and marks the rows twenty units higher"
    );
}

/// The two entries that place the caption are refused by value, and the line survives both.
///
/// Table 178 gives `/CP` exactly two names and `/CO` exactly two numbers. A file outside either
/// has asked for a caption and said nothing about where it goes, so the caption is named and the
/// line — which `/L` makes required — is still drawn. ADR 0106's rule, and [`CALLOUT_SHAPE`]'s
/// reasoning one subtype over.
#[test]
fn a_caption_this_table_cannot_place_is_named_and_the_line_is_still_drawn() {
    let of = |extra: &str| {
        interpret(pdf_with(
            &format!(
                "<< /Type /Annot /Subtype /Line /Rect [0 0 100 100] /F 4 /C [0 0 0] \
                 /BS << /W 1 >> /L [20 50 80 50] /Cap true /Contents (Hill) {extra} >>"
            ),
            "/BBox [0 0 10 10]",
            "",
        ))
    };

    for (entry, expected) in [("/CP /Middle", "/CP"), ("/CO [1 2 3]", "/CO")] {
        let interpretation = of(entry);
        assert!(
            !interpretation.display_list.commands().is_empty(),
            "{entry}: the line the clause requires is still drawn"
        );
        let reported = format!("{:?}", interpretation.unsupported);
        assert!(
            reported.contains(expected),
            "{entry}: the caption is named — {reported}"
        );
    }

    // A line of no length has no axes for `/CO` to be measured in, and says so beside the dot
    // §8.5.3.2 still puts on the page.
    let degenerate = interpret(pdf_with(
        "<< /Type /Annot /Subtype /Line /Rect [0 0 100 100] /F 4 /C [0 0 0] \
         /BS << /W 4 >> /L [50 50 50 50] /Cap true /Contents (Hill) >>",
        "/BBox [0 0 10 10]",
        "",
    ));
    assert!(
        format!("{:?}", degenerate.unsupported).contains("same point"),
        "{:?}",
        degenerate.unsupported
    );
}

/// §12.5.6.9's polyline takes its endings from its own first and last legs.
#[test]
fn a_polylines_endings_follow_its_first_and_last_leg() {
    let bent = render(pdf_with(
        "<< /Type /Annot /Subtype /PolyLine /Rect [0 0 100 100] /F 4 \
         /Vertices [20 20 50 80 80 20] /C [0 0 0] /BS << /W 3 >> /LE [/OpenArrow /OpenArrow] >>",
        "/BBox [0 0 10 10]",
        "",
    ));
    let plain = render(pdf_with(
        "<< /Type /Annot /Subtype /PolyLine /Rect [0 0 100 100] /F 4 \
         /Vertices [20 20 50 80 80 20] /C [0 0 0] /BS << /W 3 >> /LE [/None /None] >>",
        "/BBox [0 0 10 10]",
        "",
    ));
    assert_ne!(bent.data, plain.data, "the arrowheads are drawn");
}

/// A polygon's ends meet, so §12.5.6.9 gives it none to decorate — and it says so.
#[test]
fn a_polygons_line_endings_are_reported_because_it_has_no_ends() {
    let interpretation = interpret(pdf_with(
        "<< /Type /Annot /Subtype /Polygon /Rect [0 0 100 100] /F 4 \
         /Vertices [20 20 50 80 80 20] /C [0 0 0] /BS << /W 3 >> /LE [/OpenArrow /OpenArrow] >>",
        "/BBox [0 0 10 10]",
        "",
    ));
    assert!(
        format!("{:?}", interpretation.unsupported).contains("no two points"),
        "{:?}",
        interpretation.unsupported
    );
}

/// A push-button with no appearance stream draws Table 192's `/I` as its icon.
///
/// §12.5.6.19's Table 191 makes the appearance characteristics dictionary the input to
/// *constructing* a stream, so this is the only route by which the entry can reach a pixel: a widget with its own
/// `/AP` is drawn from that and never asks. `/TP 1` is the table's "No caption; icon only", which
/// is what makes the mark here the icon's and nothing else's.
///
/// The fixture's icon is object 6, the same form `XObject` every other test here hands to `/AP` —
/// a red square on `/BBox [0 0 10 10]`. Table 250's defaults scale it "Always", "Proportional",
/// centred, so it fills the rectangle inside §12.5.4's one-unit border.
#[test]
fn a_push_buttons_normal_icon_is_drawn_where_it_has_no_appearance_stream() {
    let raster = render(pdf_with(
        "<< /Type /Annot /Subtype /Widget /Rect [10 10 90 90] /F 4 /FT /Btn /Ff 65536 \
         /T (go) /MK << /I 6 0 R /TP 1 >> >>",
        "/BBox [0 0 10 10]",
        "1 0 0 rg 0 0 10 10 re f",
    ));
    assert_eq!(
        colour_at(&raster, 50, 50),
        (255, 0, 0),
        "the icon fills the rectangle Table 250's defaults fit it into"
    );
    assert!(painted(&raster, 15, 15), "and reaches its corner");
    assert!(!painted(&raster, 5, 5), "and not past /Rect");
}

/// Table 250's `/SW N` — "Never scale" — leaves the icon its own size, and `/A` places it.
///
/// The icon's `/BBox` is 10 by 10 in a rectangle 78 by 78 inside the border, so an unscaled icon
/// occupies a hundredth of it and where that hundredth sits is entirely `/A`'s answer: the
/// default `[0.5 0.5]` centres it, `[0.0 0.0]` puts it "at the bottom-left corner". Two fixtures
/// differing in one entry, because a single one cannot tell a placement from a scale.
#[test]
fn table_250s_never_scale_keeps_the_icons_own_size_and_a_places_it() {
    let centred = render(pdf_with(
        "<< /Type /Annot /Subtype /Widget /Rect [10 10 90 90] /F 4 /FT /Btn /Ff 65536 \
         /T (go) /MK << /I 6 0 R /TP 1 /IF << /SW /N >> >> >>",
        "/BBox [0 0 10 10]",
        "1 0 0 rg 0 0 10 10 re f",
    ));
    assert!(painted(&centred, 50, 50), "the default /A centres it");
    assert!(!painted(&centred, 20, 20), "and it is not scaled up");

    let cornered = render(pdf_with(
        "<< /Type /Annot /Subtype /Widget /Rect [10 10 90 90] /F 4 /FT /Btn /Ff 65536 \
         /T (go) /MK << /I 6 0 R /TP 1 /IF << /SW /N /A [0 0] >> >> >>",
        "/BBox [0 0 10 10]",
        "1 0 0 rg 0 0 10 10 re f",
    ));
    assert!(
        painted(&cornered, 15, 15),
        "/A [0.0 0.0] puts it at the bottom-left corner"
    );
    assert!(!painted(&cornered, 50, 50), "and nowhere near the centre");
}

/// Table 192's `/TP 0` is "No icon; caption only", and it is the table's default.
///
/// A widget stating an `/I` and no `/TP` states a button the standard says shows its caption, so
/// drawing the icon there would put a mark on the page the file asked not to have. The fixture
/// states neither a caption nor a background, so obeying the code leaves the page blank.
#[test]
fn table_192s_default_caption_position_draws_no_icon() {
    let raster = render(pdf_with(
        "<< /Type /Annot /Subtype /Widget /Rect [10 10 90 90] /F 4 /FT /Btn /Ff 65536 \
         /T (go) /MK << /I 6 0 R >> >>",
        "/BBox [0 0 10 10]",
        "1 0 0 rg 0 0 10 10 re f",
    ));
    assert!(
        !painted(&raster, 50, 50),
        "code 0 is 'No icon; caption only', and this button has no caption"
    );
}

/// The icon entries are Table 192's "push-button fields only", and a check box is not one.
///
/// Table 229 bit 17 is what separates them, and it is inheritable — so this is the same fixture
/// with `/Ff 0`, which makes the field a check box and the `/I` an entry that applies to nothing.
#[test]
fn a_check_box_does_not_draw_table_192s_push_button_icon() {
    let interpretation = interpret(pdf_with(
        "<< /Type /Annot /Subtype /Widget /Rect [10 10 90 90] /F 4 /FT /Btn /Ff 0 \
         /T (tick) /V /Off /MK << /I 6 0 R /TP 1 >> >>",
        "/BBox [0 0 10 10]",
        "1 0 0 rg 0 0 10 10 re f",
    ));
    assert!(
        interpretation.display_list.commands().is_empty(),
        "the entries are for push-buttons and this field is a check box"
    );
}

/// Table 192's codes 2 to 5 put the caption on one side of the icon, at a share this program
/// chose, and say so.
///
/// "Caption below the icon", "Caption above the icon", "Caption to the right of the icon" and
/// "Caption to the left of the icon" fix a relation and no proportion; `doc/questions/A72` rules
/// that such a quantity is chosen and reported as ours (ADR 1299). The caption takes a third of
/// the rectangle on its side, so the icon is fitted into the other two thirds: a square icon in
/// the 80-point square `/Rect [10 10 90 90]` becomes a square of about 53 points, against the
/// side away from the caption's band. The points below are PDF's, y upward. The fixture states
/// no caption text, so what reports is the share rather than a caption with no `/DA`.
#[test]
fn table_192s_beside_codes_divide_the_rectangle_at_a_chosen_share() {
    let widget = |code: u8| {
        format!(
            "<< /Type /Annot /Subtype /Widget /Rect [10 10 90 90] /F 4 /FT /Btn /Ff 65536 \
             /T (go) /MK << /I 6 0 R /TP {code} >> >>"
        )
    };
    // (code, a point inside the icon's two thirds, a point inside the caption's third)
    for (code, icon, caption) in [
        (2, (50, 70), (50, 20)),
        (3, (50, 30), (50, 80)),
        (4, (30, 50), (80, 50)),
        (5, (70, 50), (20, 50)),
    ] {
        let interpretation = interpret(pdf_with(
            &widget(code),
            "/BBox [0 0 10 10]",
            "1 0 0 rg 0 0 10 10 re f",
        ));
        let raster = render_incomplete(&interpretation);
        assert_eq!(
            colour_at(&raster, icon.0, icon.1),
            (255, 0, 0),
            "/TP {code}: the icon is in its two thirds"
        );
        assert!(
            !painted(&raster, caption.0, caption.1),
            "/TP {code}: and not in the caption's third"
        );
        let reported = format!("{:?}", interpretation.unsupported);
        assert!(
            reported.contains(&format!("/TP {code}")) && reported.contains("this program chose"),
            "the share is named as this program's: {reported}"
        );
    }
}

/// A one-page fixture whose widget states the `/MK` entries given, with three icons to pick from.
///
/// Objects 6, 7 and 8 are form `XObject`s filling their whole `/BBox` in red, green and blue, so
/// which of Table 192's three icon entries reached the stream is a colour rather than a count.
fn widget_with_three_icons(characteristics: &str) -> Vec<u8> {
    let mut body = String::from(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] \
         /Resources << >> /Contents 4 0 R /Annots [5 0 R] >>\nendobj\n\
         4 0 obj\n<< /Length 0 >>\nstream\n\nendstream\nendobj\n",
    );
    // `/H /N` is "[n]o highlighting", so the down state shows the down *icon* rather than
    // Table 191's default `I`, which would invert every colour asserted below.
    let _ = write!(
        body,
        "5 0 obj\n<< /Type /Annot /Subtype /Widget /Rect [10 10 90 90] /F 4 /H /N \
         /FT /Btn /Ff 65536 /T (go) /MK << {characteristics} /TP 1 >> >>\nendobj\n"
    );
    for (number, colour) in [(6, "1 0 0"), (7, "0 1 0"), (8, "0 0 1")] {
        let content = format!("{colour} rg 0 0 10 10 re f");
        let _ = write!(
            body,
            "{number} 0 obj\n<< /Type /XObject /Subtype /Form /BBox [0 0 10 10] /Length {} >>\n\
             stream\n{content}\nendstream\nendobj\n",
            content.len().saturating_add(1)
        );
    }
    assemble(&body)
}

/// Table 192's three icons are chosen by what the pointer is doing, as the table's own rows say.
///
/// `/I` is "the widget annotation's normal icon , which shall be displayed when it is not
/// interacting with the user", `/RI` the rollover icon shown "when the user rolls the cursor into
/// its active area without pressing the mouse button", and `/IX` the alternate one shown "when
/// the mouse button is pressed within its active area" — the same three conditions §12.5.5 states
/// for `/N`, `/R` and `/D`, which is why the state that selects among those selects among these.
///
/// The assertion is a pixel for [`the_pointer_chooses_between_an_annotations_appearances`]'s
/// reason: nothing in the display list distinguishes *we picked `/I`* from *we picked `/RI` and
/// it happens to look the same*.
///
/// **The second half is the fallback, which is a decision** (ADR 1090): Table 192 makes all three
/// optional and states no fallback, so a rollover with no `/RI` is read the way Table 170 reads a
/// rollover with no `/R` — the normal one. **No corpus document states `/RI` or `/IX` at all**, so
/// this fixture is hand-built and is the only thing defending either rule (trap 8).
#[test]
fn table_192s_three_icons_are_chosen_by_what_the_pointer_is_doing() {
    let colour_under = |characteristics: &str, pointer: Option<pdf_model::view::Pointer>| {
        let document =
            Document::open(widget_with_three_icons(characteristics)).expect("a valid PDF");
        let page = pdf_model::Pages::new(&document).get(0).expect("page one");
        let mut state = pdf_model::view::ViewState::of(&document);
        state.set_pointer(pointer.map(|pointer| {
            (
                pdf_syntax::ObjectId {
                    number: 5,
                    generation: 0,
                },
                pointer,
            )
        }));
        let interpretation = pdf_model::content::interpret_with(&document, &page, &state);
        let reported = format!("{:?}", interpretation.unsupported);
        let list = interpretation.display_list;
        let target = TargetSpec::for_page(&list, 1.0, GENEROUS).expect("valid target");
        let raster = CpuRasterizer::new()
            .with_medium(pdf_render::Medium::NONE)
            .rasterize(&list, target)
            .expect("supported");
        let index = at(&raster, 50, 50);
        (
            (
                raster.data[index],
                raster.data[index.saturating_add(1)],
                raster.data[index.saturating_add(2)],
            ),
            reported,
        )
    };

    let all_three = "/I 6 0 R /RI 7 0 R /IX 8 0 R";
    assert_eq!(
        colour_under(all_three, None).0,
        (255, 0, 0),
        "nothing is interacting: /I"
    );
    assert_eq!(
        colour_under(all_three, Some(pdf_model::view::Pointer::Over)).0,
        (0, 255, 0),
        "the cursor rolled in without a button pressed: /RI"
    );
    assert_eq!(
        colour_under(all_three, Some(pdf_model::view::Pointer::Down)).0,
        (0, 0, 255),
        "the mouse button pressed within its active area: /IX"
    );

    // The entry the file does not state, in both directions: an absent `/RI` falls back to `/I`,
    // and an absent `/IX` does the same, rather than erasing the button as the pointer crosses it.
    let normal_only = "/I 6 0 R";
    for pointer in [
        pdf_model::view::Pointer::Over,
        pdf_model::view::Pointer::Down,
    ] {
        let (colour, reported) = colour_under(normal_only, Some(pointer));
        assert_eq!(colour, (255, 0, 0), "with no entry for {pointer:?}: /I");
        assert!(
            !reported.contains("/RI") && !reported.contains("/IX"),
            "and nothing is owed for an entry the file never stated: {reported}"
        );
    }

    // And the four entries this clause's row called owed are no longer reported at all.
    let (_, reported) = colour_under(all_three, Some(pdf_model::view::Pointer::Over));
    assert!(
        reported.contains("[]"),
        "a widget stating /RI and /IX draws them and owes nothing: {reported}"
    );
}

/// §12.5.4's underline border is inside the rectangle, not centred on its bottom edge.
///
/// Table 168 calls the `U` style "[a] single line along the bottom of the annotation
/// rectangle", and §12.5.4's own sentence binds it exactly as it binds the four rectangular
/// styles: "If present, the border shall be drawn completely inside the annotation rectangle."
/// A stroke straddles its path, so a path *on* the bottom edge puts half the ink below `/Rect`
/// — which is what this crate drew until the four-hundred-and-fifty-eighth session, on a
/// comment that described where a stroke sits relative to its path and mistook that for where
/// the path goes.
///
/// **What the reader saw was a thin line rather than ink outside the rectangle**, and that is
/// worth stating because it is why nothing noticed: `Constructed::bounded` clips a link's
/// construction to `/Rect`, so the half that fell outside was cut away and a `/W` 4 underline
/// arrived 2 units thick. A departure that loses half a mark inside a clip looks like a mark.
///
/// One corpus document states a `U` border on an annotation with no appearance stream —
/// `annotation-border-styles.pdf`, whose object 29 is a `/Subtype /Link` with
/// `/BS << /S /U /W 1 >>` — so the departure was on a real page, half a point of it.
#[test]
fn an_underline_border_is_drawn_inside_the_rectangle_rather_than_across_its_edge() {
    let raster = render(pdf_with(
        "<< /Type /Annot /Subtype /Link /Rect [20 20 80 60] /C [0 1 0] \
         /BS << /W 4 /S /U >> >>",
        "/BBox [0 0 10 10]",
        "",
    ));

    // A 4-unit line whose path is the bottom edge raised by half its width covers exactly the
    // four rows inside the rectangle, and none outside it.
    assert_eq!(extent(&raster), (20, 20, 79, 23));
    assert!(!painted(&raster, 50, 19), "below /Rect");
    assert!(painted(&raster, 50, 20), "the bottom edge itself");
    assert!(painted(&raster, 50, 23), "the inner limit of a 4-unit line");
    assert!(!painted(&raster, 50, 24), "just inside the line");
    assert_eq!(colour_at(&raster, 50, 21), (0, 255, 0), "Table 166's /C");
}

/// A `/BS` entry ignores `/Border` whole, corner radii included.
///
/// Table 166: "If an annotation dictionary includes the BS entry, then the Border entry is
/// ignored" — sharpened by Errata Collection 3 Issue #287 to *shall be ignored*. Table 168 has
/// no entry for a corner radius, which is why reading `/Border`'s first two elements beside a
/// `/BS` looked like completeness; it is a border the standard says is square, drawn round.
///
/// The pair is the point: two annotations differing only in whether `/BS` is present, so the
/// same `/Border [12 12 4]` rounds one corner and is ignored on the other. No corpus document
/// states both a `/BS` and a non-zero `/Border` radius on an annotation this crate constructs a
/// border for — 6 do state both, and all 6 are ink annotations, whose mark is `/InkList` — so
/// this fixture is the only witness there is.
#[test]
fn a_border_style_dictionary_ignores_the_border_arrays_corner_radii() {
    let rounded = render(pdf_with(
        "<< /Type /Annot /Subtype /Link /Rect [20 20 80 60] /C [0 1 0] /Border [12 12 4] >>",
        "/BBox [0 0 10 10]",
        "",
    ));
    assert!(
        !painted(&rounded, 21, 21),
        "/Border's radii round the corner when nothing overrides them"
    );

    let square = render(pdf_with(
        "<< /Type /Annot /Subtype /Link /Rect [20 20 80 60] /C [0 1 0] /Border [12 12 4] \
         /BS << /W 4 /S /S >> >>",
        "/BBox [0 0 10 10]",
        "",
    ));
    assert!(
        painted(&square, 21, 21),
        "a /BS entry ignores /Border, so the corner is square"
    );
    assert_eq!(extent(&square), (20, 20, 79, 59), "still inside /Rect");
}

/// §12.5.5's second transparency sentence: a `/Group` on the appearance decides which group.
///
/// > If the appearance's stream dictionary does not contain a Group entry, it shall be treated
/// > as a non-isolated, non-knockout transparency group. Otherwise, the isolated and knockout
/// > values specified in the group dictionary (see 11.6.6, "Transparency group XObjects") shall
/// > be used.
///
/// Every appearance is a group, so each of the two stated values is asserted against the group
/// the sentence names by default — the same content twice, once with the entry and once without.
/// The default one is what §11.4.4's NOTE 5 flattens, which is why the pair is the measurement:
/// a reader that ignored the entry would draw both halves of each pair the same.
///
/// §11.4.6 is what the knockout half asks for — "each individual element shall be composited
/// with the group's initial backdrop rather than with the stack of preceding elements in the
/// group" — so where the two half-opaque fills overlap, the later one is composited with the
/// page as though the earlier were not there, and the overlap is the colour the later fill has
/// where it lies over the page alone.
///
/// §11.4.4's NOTE 2 is what the isolated half asks for: an element's blend with the backdrop is
/// "[w]hat distinguishes non-isolated groups from isolated groups", so a `Multiply` fill inside
/// an isolated group multiplies with transparency — which leaves its own colour — and the same
/// fill in the default group multiplies with the page.
///
/// **The corpus has no witness and the crawl does**, which is why the fixture is a pair rather
/// than a document (trap 8): `examples/appearance_transparency_census` finds five appearance
/// streams stating a `/Group` across the curated population's 1452 documents, every one of them
/// non-isolated and non-knockout, against 95 isolated and 1 knockout over `CC-MAIN-2021-31`'s
/// 65 720.
#[test]
fn an_appearance_group_is_isolated_and_knocked_out_as_the_file_states() {
    // The page under the annotation: blue everywhere the appearance will draw.
    const PAGE: &str = "0 0 1 rg 0 0 100 100 re f";
    const ANNOTATION: &str =
        "<< /Type /Annot /Subtype /Square /Rect [0 0 100 100] /F 4 /AP << /N 6 0 R >> >>";
    // Two half-opaque fills, red then green, overlapping over the middle of the page.
    const OVERLAPPING: &str = "/G0 gs 1 0 0 rg 10 10 50 50 re f 0 1 0 rg 40 40 50 50 re f";
    const HALF: &str = "/Resources << /ExtGState << /G0 << /ca 0.5 >> >> >>";
    // A single element that blends, and the state that makes it blend (§11.4.4 NOTE 2, below).
    const BLENDING: &str = "/G1 gs 1 0 0 rg 10 10 80 80 re f";
    const MULTIPLY: &str = "/Resources << /ExtGState << /G1 << /BM /Multiply >> >> >>";

    let knockout = render(pdf_over(
        PAGE,
        ANNOTATION,
        &format!("/BBox [0 0 100 100] /Group << /S /Transparency /I true /K true >> {HALF}"),
        OVERLAPPING,
    ));
    let green_alone = colour_at(&knockout, 70, 70);
    assert_eq!(
        colour_at(&knockout, 50, 50),
        green_alone,
        "§11.4.6: in the overlap the later element is composited with the group's initial \
         backdrop, so it is the colour it has where nothing precedes it"
    );

    let stacked = render(pdf_over(
        PAGE,
        ANNOTATION,
        &format!("/BBox [0 0 100 100] /Group << /S /Transparency /I true >> {HALF}"),
        OVERLAPPING,
    ));
    assert_eq!(
        colour_at(&stacked, 70, 70),
        green_alone,
        "the control differs only in the overlap"
    );
    assert_ne!(
        colour_at(&stacked, 50, 50),
        green_alone,
        "without /K the later element composites over the earlier one"
    );

    // §11.4.4's NOTE 2, on a single blending element: the isolated group's initial backdrop is
    // transparent, so `Multiply` has nothing to multiply with and the element keeps its colour.
    let isolated = render(pdf_over(
        PAGE,
        ANNOTATION,
        &format!("/BBox [0 0 100 100] /Group << /S /Transparency /I true >> {MULTIPLY}"),
        BLENDING,
    ));
    assert_eq!(
        colour_at(&isolated, 50, 50),
        (255, 0, 0),
        "§11.4.5: an isolated group's elements composite onto transparency, so Multiply leaves \
         the source colour"
    );

    let joined = render(pdf_over(
        PAGE,
        ANNOTATION,
        &format!("/BBox [0 0 100 100] /Group << /S /Transparency >> {MULTIPLY}"),
        BLENDING,
    ));
    assert_eq!(
        colour_at(&joined, 50, 50),
        (0, 0, 0),
        "the default group is non-isolated, so the same fill multiplies with the blue page"
    );
}

/// The annotation dictionary of a one-page fixture that carries exactly one.
///
/// `/Rect` and an appearance are beside the point for both entries this builds a document for:
/// §12.5.2's list of keys a reader ignores while rendering a stored appearance names neither
/// `/IT` nor `/Measure`, so they are read from the dictionary whatever is drawn.
fn annotation_of(annotation: &str) -> (Document, pdf_syntax::Dictionary) {
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] \
         /Resources << >> /Annots [4 0 R] >>\nendobj\n\
         4 0 obj\n{annotation}\nendobj\n"
    );
    let document = Document::open(assemble(&body)).expect("the fixture is a valid PDF");
    let dict = document
        .get(pdf_syntax::ObjectId {
            number: 4,
            generation: 0,
        })
        .as_dict()
        .expect("object 4 is the annotation")
        .clone();
    (document, dict)
}

/// §12.5.6.7's `/IT` has two valid values and §12.5.6.9's three, and which table applies is the
/// annotation's own `/Subtype`.
///
/// Table 178: "Valid values shall be LineArrow , which means that the annotation is intended to
/// function as an arrow, and LineDimension , which means that the annotation is intended to
/// function as a dimension line." Table 181: "The following values shall be valid:" —
/// `PolygonCloud`, `PolyLineDimension`, `PolygonDimension`.
///
/// The rows that make this a test of the *tables* rather than of five string comparisons are the
/// crossed ones: a `Line` stating `PolygonCloud` and a `Polygon` stating `LineArrow` have each
/// named something their own table does not define, and are kept as the names they are rather
/// than dropped — a file that named a purpose may not read as one that named none.
///
/// The last row is the entry this clause shares its spelling with: §12.5.6.6's free text callout
/// intent, whose reader is `appearance::callout` and which this one does not answer for.
///
/// Calibrated (trap 13) by making [`pdf_model::appearance::intent`] ignore the `/Subtype`: the
/// two crossed rows fail and the `FreeText` row fails with them.
#[expect(
    clippy::doc_markdown,
    reason = "the comment quotes Table 178 verbatim, and a quotation is not marked up"
)]
#[test]
fn an_intent_is_read_against_the_table_its_own_subtype_names() {
    let cases = [
        ("/Line", "/LineArrow", Some(Intent::LineArrow)),
        ("/Line", "/LineDimension", Some(Intent::LineDimension)),
        ("/Polygon", "/PolygonCloud", Some(Intent::PolygonCloud)),
        (
            "/PolyLine",
            "/PolyLineDimension",
            Some(Intent::PolyLineDimension),
        ),
        (
            "/Polygon",
            "/PolygonDimension",
            Some(Intent::PolygonDimension),
        ),
        (
            "/Line",
            "/PolygonCloud",
            Some(Intent::Unknown("PolygonCloud".to_owned())),
        ),
        (
            "/Polygon",
            "/LineArrow",
            Some(Intent::Unknown("LineArrow".to_owned())),
        ),
        ("/FreeText", "/FreeTextCallout", None),
    ];
    for (subtype, stated, expected) in cases {
        let (document, annotation) = annotation_of(&format!(
            "<< /Type /Annot /Subtype {subtype} /Rect [0 0 100 100] /IT {stated} >>"
        ));
        assert_eq!(
            pdf_model::appearance::intent(&document, &annotation),
            expected,
            "{subtype} stating /IT {stated}"
        );
    }

    // An annotation of a subtype this reader answers for that states no `/IT` at all: the entry
    // is "(Optional)" in both tables, so its absence is not a value.
    let (document, annotation) =
        annotation_of("<< /Type /Annot /Subtype /Line /Rect [0 0 100 100] /L [0 0 1 1] >>");
    assert_eq!(pdf_model::appearance::intent(&document, &annotation), None);
}

/// §12.5.6.7's and §12.5.6.9's `/Measure`, applied to the geometry each clause states.
///
/// Table 178 gives a line annotation "[a] measure dictionary … that shall specify the scale and
/// units that apply to the line annotation", and Table 181 says the same of a polygon's and a
/// polyline's. Table 267 then states the arithmetic and, twice, its *order*: the scale factors
/// from `/X`, `/Y` and `/CYX` "shall be used to convert from default user space to the
/// appropriate units before applying the distance function", and the same sentence again for the
/// area function.
///
/// **The `/X` factor of 2 is what makes this a test of that order.** A 3 by 4 line is 5 units
/// long in user space and 10 metres long in this drawing's; a reader that took the distance first
/// and converted afterwards would answer `5 m`, because `/D`'s own conversion is 1. The same
/// factor squares for the area: a 3 by 4 rectangle is 12 in user space and 48 here.
///
/// The perimeter rows are §12.5.6.9's own sentence about what closes a shape — a polyline is a
/// polygon "except that the first and last vertex are not implicitly connected" — so the polygon
/// walks 6 + 8 + 6 + 8 and the polyline, over the identical vertices, walks 6 + 8 + 6.
///
/// Calibrated (trap 13) by converting after the distance rather than before: every row fails.
#[test]
fn an_annotations_own_measure_states_the_units_its_geometry_is_in() {
    const MEASURE: &str = "/Measure << /Subtype /RL /R (1 pt = 2 m) \
                           /X [<< /U (m) /C 2 >>] /D [<< /U (m) /C 1 >>] \
                           /A [<< /U (sqm) /C 1 >>] >>";

    let (document, line) = annotation_of(&format!(
        "<< /Type /Annot /Subtype /Line /Rect [0 0 100 100] /L [0 0 3 4] {MEASURE} >>"
    ));
    let measured = pdf_model::measurement::annotation_measurement(&document, &line)
        .expect("a line annotation stating a rectilinear /Measure measures");
    assert_eq!(measured.length.as_deref(), Some("10 m"));
    assert_eq!(measured.area, None, "a line encloses nothing");

    let vertices = "/Vertices [0 0 3 0 3 4 0 4]";
    let (document, polygon) = annotation_of(&format!(
        "<< /Type /Annot /Subtype /Polygon /Rect [0 0 100 100] {vertices} {MEASURE} >>"
    ));
    let measured = pdf_model::measurement::annotation_measurement(&document, &polygon)
        .expect("a polygon stating a rectilinear /Measure measures");
    assert_eq!(measured.length.as_deref(), Some("28 m"));
    assert_eq!(measured.area.as_deref(), Some("48 sqm"));

    let (document, polyline) = annotation_of(&format!(
        "<< /Type /Annot /Subtype /PolyLine /Rect [0 0 100 100] {vertices} {MEASURE} >>"
    ));
    let measured = pdf_model::measurement::annotation_measurement(&document, &polyline)
        .expect("a polyline stating a rectilinear /Measure measures");
    assert_eq!(
        measured.length.as_deref(),
        Some("20 m"),
        "the same vertices, with the leg back to the first not walked"
    );
    assert_eq!(measured.area, None, "an open shape encloses nothing");

    // Table 178's `/LL` makes `/L` "the endpoints of the leader lines rather than the endpoints
    // of the line itself", and the line proper is that segment translated perpendicular to
    // itself — so the length is the same one.
    let (document, leadered) = annotation_of(&format!(
        "<< /Type /Annot /Subtype /Line /Rect [0 0 100 100] /L [0 0 3 4] /LL 20 {MEASURE} >>"
    ));
    let measured = pdf_model::measurement::annotation_measurement(&document, &leadered)
        .expect("leader lines do not stop a line from being measured");
    assert_eq!(measured.length.as_deref(), Some("10 m"));

    // An annotation stating no `/Measure` has stated no units, and one stating a `/Path` has
    // stated curves Table 267's conversions do not describe.
    let (document, plain) = annotation_of(
        "<< /Type /Annot /Subtype /Polygon /Rect [0 0 100 100] /Vertices [0 0 3 0 3 4] >>",
    );
    assert_eq!(
        pdf_model::measurement::annotation_measurement(&document, &plain),
        None
    );
    let (document, curved) = annotation_of(&format!(
        "<< /Type /Annot /Subtype /Polygon /Rect [0 0 100 100] \
         /Path [[0 0] [3 0 3 4 0 4]] {MEASURE} >>"
    ));
    assert_eq!(
        pdf_model::measurement::annotation_measurement(&document, &curved),
        None,
        "the length of a cubic Bezier is not a quantity Table 267 states a conversion for"
    );
}

/// §12.5.6.2's `/DS` is Table 177's entry, not Table 172's, and the clause names it once.
///
/// The group-attribute sentence is the only place §12.5.6.2 mentions it — "[t]hese entries are
/// Contents (or RC and DS ), M , C , T , Popup , CreationDate , Subj , and Open" — and Table 177
/// is where it is defined: "[a] default style string, as described in Adobe XML Architecture, XML
/// Forms Architecture (XFA) Specification, version 3.3". That specification is one `CLAUDE.md`
/// excludes, so the style is not applied and the note's text is laid out under Table 177's `/DA`
/// instead. The departure is **reported** rather than shown in silence, which is the whole of
/// what this asserts — a pair, so that the report fires on the entry and not on every free text
/// annotation (trap 11). ADR 1224.
#[expect(
    clippy::doc_markdown,
    reason = "verbatim quotations: §12.5.6.2 and Table 177 spell the entry names without \
              backticks"
)]
#[test]
fn a_note_whose_default_style_is_not_applied_says_so() {
    let note = |extra: &str| {
        format!(
            "<< /Type /Annot /Subtype /FreeText /Rect [10 10 90 90] /F 4 \
             /Contents (a note) /DA (/Helv 10 Tf 0 g) {extra} >>"
        )
    };
    let plain = interpret(pdf_with(&note(""), "", ""));
    let reports = format!("{:?}", plain.unsupported);
    assert!(
        !reports.contains("/DS"),
        "a note stating no /DS owes nothing: {reports}"
    );

    let styled = interpret(pdf_with(&note("/DS (font: 12pt Helvetica)"), "", ""));
    let reports = format!("{:?}", styled.unsupported);
    assert!(reports.contains("/DS"), "the entry is named: {reports}");
    assert!(
        reports.contains("XFA"),
        "and why it is not applied: {reports}"
    );
}

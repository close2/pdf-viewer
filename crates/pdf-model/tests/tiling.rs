//! Tiling patterns, checked by where the tiles actually land.
//!
//! A tiling pattern is the one paint whose correctness is about *position*: the cell is
//! anchored to the page rather than to the path being filled, and its phase comes from the
//! pattern matrix. Get that wrong and the page still looks patterned — just not the pattern
//! the document asked for, offset by whatever the current transform happened to be.

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

/// Pixel budget, far above the small pages these tests build.
const GENEROUS: u64 = 1 << 30;

/// Assembles a one-page PDF whose `/Pattern` resource is the given object.
fn pdf_with(pattern: &str, content: &str) -> Vec<u8> {
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] \
         /Resources << /Pattern << /P0 5 0 R >> >> /Contents 4 0 R >>\nendobj\n\
         4 0 obj\n<< /Length {} >>\nstream\n{content}\nendstream\nendobj\n\
         5 0 obj\n{pattern}\nendobj\n",
        content.len().saturating_add(1)
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

fn pixel(raster: &pdf_render::Raster, x: u32, y: u32) -> (u8, u8, u8, u8) {
    let at = ((y.saturating_mul(raster.width)).saturating_add(x) as usize).saturating_mul(4);
    (
        raster.data[at],
        raster.data[at + 1],
        raster.data[at + 2],
        raster.data[at + 3],
    )
}

/// A cell holding one red 10×10 square, stepped every 20 units.
///
/// The gap between step and cell size is deliberate: `/XStep` may exceed the bounding box,
/// which is how a pattern tiles with space around each figure, and a reader that uses the
/// bounding box as the step instead draws them touching.
fn dotted_cell(paint_type: i32, colour: &str) -> String {
    let content = format!("{colour} 0 0 10 10 re f");
    format!(
        "<< /PatternType 1 /PaintType {paint_type} /TilingType 1 /BBox [0 0 20 20] \
         /XStep 20 /YStep 20 /Resources << >> /Length {} >>\nstream\n{content}\nendstream",
        content.len().saturating_add(1)
    )
}

#[test]
fn a_tiling_pattern_repeats_its_cell_across_the_filled_path() {
    let raster = render(pdf_with(
        &dotted_cell(1, "1 0 0 rg"),
        "/Pattern cs /P0 scn 0 0 100 100 re f",
    ));

    // The page is 100 units and the step is 20, so cells sit at 0, 20, 40, 60 and 80 —
    // and the raster's y runs the other way from PDF's, so the first row is at the bottom.
    for step in 0..5u32 {
        let across = step * 20 + 4;
        let down = 99 - across;
        let (red, green, blue, alpha) = pixel(&raster, across, down);
        assert_eq!(alpha, 255, "a cell should be painted at ({across},{down})");
        assert!(
            red > 240 && green < 15 && blue < 15,
            "and be red: {red},{green},{blue}"
        );
    }

    // The cell is 10 wide and the step is 20, so the space between cells is untouched.
    assert_eq!(
        pixel(&raster, 15, 85).3,
        0,
        "the gap between cells must not be painted"
    );
}

/// The pattern is anchored to the page, not to the path.
///
/// Two different paths filled with one pattern must show the same phase, or the tiling
/// slides about as the shapes move.
#[test]
fn tiles_line_up_between_separately_filled_paths() {
    let whole = render(pdf_with(
        &dotted_cell(1, "1 0 0 rg"),
        "/Pattern cs /P0 scn 0 0 100 100 re f",
    ));
    // The same pattern, but reaching the page through two smaller rectangles drawn at
    // different times. The tiles must land in exactly the same places.
    let split = render(pdf_with(
        &dotted_cell(1, "1 0 0 rg"),
        "/Pattern cs /P0 scn 0 0 100 50 re f 0 50 100 50 re f",
    ));

    for y in (2..100).step_by(7) {
        for x in (2..100).step_by(7) {
            assert_eq!(
                pixel(&whole, x, y),
                pixel(&split, x, y),
                "the tiling shifted at ({x},{y})"
            );
        }
    }
}

/// A fill must not paint outside its own path, however the pattern tiles.
#[test]
fn a_pattern_fill_stays_inside_its_path() {
    let raster = render(pdf_with(
        &dotted_cell(1, "1 0 0 rg"),
        "/Pattern cs /P0 scn 0 0 50 50 re f",
    ));

    // Inside the rectangle, in raster coordinates the lower left.
    assert_eq!(
        pixel(&raster, 4, 95).3,
        255,
        "the pattern must paint inside"
    );
    // Outside it, nothing — even though the tiling itself would continue.
    assert_eq!(
        pixel(&raster, 64, 35).3,
        0,
        "a tiling pattern must not escape the path it fills"
    );
}

/// `/PaintType 2` is a stencil: the cell carries no colour and `scn` supplies it.
#[test]
fn an_uncoloured_pattern_takes_its_colour_from_the_operator() {
    // The cell's content sets no colour at all.
    let raster = render(pdf_with(
        &dotted_cell(2, ""),
        "/Pattern cs 0 0 1 /P0 scn 0 0 100 100 re f",
    ));

    let (r, g, b, a) = pixel(&raster, 4, 95);
    assert_eq!(a, 255, "the stencil should paint");
    assert!(
        b > 240 && r < 15 && g < 15,
        "and take the colour given to scn, got {r},{g},{b}"
    );
}

/// An uncoloured cell that *does* set a colour is ignored, not obeyed.
///
/// ISO 32000-2 §8.6.8 names two circumstances in which the colour operators "shall be
/// ignored", and this is the second of them: "in the content stream of an uncoloured tiling
/// pattern (see 8.7.3.3, "Uncoloured tiling patterns") and to all other content streams
/// invoked from within the uncoloured tiling pattern stream". A cell setting green while
/// `scn` supplies blue therefore paints blue.
///
/// The rule is one sentence away from the `d1` glyph description rule in the same clause, and
/// this tree implements both through one flag; `tests/type3.rs` holds the other half.
#[test]
fn an_uncoloured_cell_that_sets_a_colour_is_ignored() {
    let raster = render(pdf_with(
        &dotted_cell(2, "0 1 0 rg"),
        "/Pattern cs 0 0 1 /P0 scn 0 0 100 100 re f",
    ));

    let (r, g, b, a) = pixel(&raster, 4, 95);
    assert_eq!(a, 255, "the stencil should paint");
    assert!(
        b > 240 && r < 15 && g < 15,
        "the cell's own `rg` is ignored, so this stays the scn blue: got {r},{g},{b}"
    );
}

/// The pattern matrix sets the tiling's phase.
#[test]
fn the_pattern_matrix_moves_the_tiling() {
    let shifted = dotted_cell(1, "1 0 0 rg")
        .replace("/PatternType 1", "/PatternType 1 /Matrix [1 0 0 1 10 0]");
    let raster = render(pdf_with(&shifted, "/Pattern cs /P0 scn 0 0 100 100 re f"));

    // Shifting by ten moves each cell into what was the gap, and empties what was a cell.
    assert_eq!(
        pixel(&raster, 14, 95).3,
        255,
        "the cell should have moved right by ten"
    );
    assert_eq!(
        pixel(&raster, 4, 95).3,
        0,
        "and left the old position empty"
    );
}

/// A pattern small enough to need more tiles than the bound allows must be reported.
#[test]
fn an_unreasonable_number_of_tiles_is_reported_rather_than_drawn() {
    let tiny = dotted_cell(1, "1 0 0 rg").replace("/XStep 20 /YStep 20", "/XStep 0.05 /YStep 0.05");
    let bytes = pdf_with(&tiny, "/Pattern cs /P0 scn 0 0 100 100 re f");
    let document = Document::open(bytes).expect("valid PDF");
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    let interpretation = pdf_model::interpret(&document, &page);

    let reported = format!("{:?}", interpretation.unsupported);
    assert!(
        reported.contains("MAX_TILES"),
        "four million tiles should be refused and said so: {reported}"
    );
}

/// A cell's content is clipped to its `/BBox`, in every cell.
///
/// Table 74, of the four numbers in `/BBox`: "These boundaries shall be used to clip the
/// pattern cell." The cell here paints a 30-unit square inside a 10-unit box stepped every 20,
/// so the clause's answer and the unclipped one are visible in different places: **inside the
/// box** every renderer paints, in the **gap** between box and step only an unclipped one
/// does, and in the **next cell's box** an unclipped one paints twice over.
///
/// `tiling-pattern-large-steps.pdf` is the corpus page that found this: its cell draws to
/// x = 4000 inside a box that ends at 3950, and poppler, ghostscript and hayro stop at the box
/// while this tree ran on to the end of the page.
#[test]
fn a_cell_is_clipped_to_its_bounding_box() {
    let content = "1 0 0 rg 0 0 30 30 re f";
    let pattern = format!(
        "<< /PatternType 1 /PaintType 1 /TilingType 1 /BBox [0 0 10 10] \
         /XStep 20 /YStep 20 /Resources << >> /Length {} >>\nstream\n{content}\nendstream",
        content.len().saturating_add(1)
    );
    let raster = render(pdf_with(&pattern, "/Pattern cs /P0 scn 0 0 100 100 re f"));

    // Inside the first cell's box: painted.
    let (red, _, _, alpha) = pixel(&raster, 5, 99 - 5);
    assert_eq!((red > 240, alpha), (true, 255), "inside the box");

    // Between the box's edge at 10 and the next cell's origin at 20: the cell's own square
    // covers it and the box does not, so this is the pixel the clause decides.
    assert_eq!(
        pixel(&raster, 15, 99 - 15).3,
        0,
        "a cell's content past its /BBox is clipped away"
    );

    // And the same in the horizontal gap alone, where the unclipped square would still reach.
    assert_eq!(pixel(&raster, 15, 99 - 5).3, 0, "clipped in x as well as y");
}

/// A pattern used inside a form `XObject` is anchored to the *form's* space, not the page's.
///
/// §8.7.2 says a pattern matrix maps pattern space to "the default coordinate system of the
/// pattern's parent content stream", and then says what that is here:
///
/// > Similarly, if a pattern is used within a form XObject (see 8.10, "Form XObjects" ), the
/// > pattern matrix maps pattern space to the form's default user space (that is, the form
/// > coordinate space at the time the form is painted with the Do operator).
///
/// The fixture makes the two readings land in different pixels: the form's `/Matrix` shifts it
/// 10 units right, and the cell paints a 5-unit square every 20. Anchored to the form, the
/// squares start at x = 10; anchored to the page, at x = 0 — and 10 is where the clause and
/// three reference renderers put them.
#[test]
fn a_pattern_inside_a_form_is_anchored_to_the_forms_space() {
    let cell = "1 0 0 rg 0 0 5 5 re f";
    let pattern = format!(
        "<< /PatternType 1 /PaintType 1 /TilingType 1 /BBox [0 0 20 20] \
         /XStep 20 /YStep 20 /Resources << >> /Length {} >>\nstream\n{cell}\nendstream",
        cell.len().saturating_add(1)
    );
    let form_content = "/Pattern cs /P0 scn 0 0 100 100 re f";
    let form = format!(
        "<< /Type /XObject /Subtype /Form /FormType 1 /BBox [0 0 100 100] \
         /Matrix [1 0 0 1 10 0] /Resources << /Pattern << /P0 5 0 R >> >> /Length {} >>\
         \nstream\n{form_content}\nendstream",
        form_content.len().saturating_add(1)
    );

    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] \
         /Resources << /XObject << /Fm0 6 0 R >> >> /Contents 4 0 R >>\nendobj\n\
         4 0 obj\n<< /Length 8 >>\nstream\n/Fm0 Do\nendstream\nendobj\n\
         5 0 obj\n{pattern}\nendobj\n\
         6 0 obj\n{form}\nendobj\n"
    );
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

    let raster = render(out.into_bytes());
    // Two pixels, and they disagree under the two readings: (12, ·) is inside the square the
    // form's space puts at x = 10..15, and (2, ·) is inside the one the page's space would
    // put at x = 0..5.
    assert_eq!(pixel(&raster, 12, 99 - 2).3, 255, "anchored to the form");
    assert_eq!(
        pixel(&raster, 2, 99 - 2).3,
        0,
        "and not to the page, which would start the cells 10 units to the left"
    );
}

/// A glyph filled with a tiling pattern is its outline tiled, not a solid fill.
///
/// §8.7.2: "All patterns shall be treated as colours; a Pattern colour space shall be
/// established with the CS or cs operator just like other colour spaces" — so a pattern set as
/// the *fill* colour applies to text exactly as it applies to a path, and the glyph's outline
/// is what the cell is clipped to. `pattern_text_embedded_font.pdf` is the corpus page that
/// found this: three references draw a checkerboard line of `AbCdEf` and this tree drew
/// nothing at all there, because a glyph took `fill_paint()`, which a tiling pattern
/// deliberately leaves alone.
///
/// The fixture uses one of the standard fonts, so no font program is embedded and the test is
/// about the paint rather than about glyph outlines: a large `A` in Helvetica over a cell that
/// paints alternate 5-unit squares. What is asserted is that ink lands inside the glyph and
/// that the gaps between cells stay empty *inside the glyph* — the two answers a solid fill
/// cannot both give.
#[test]
fn a_glyph_filled_with_a_tiling_pattern_is_tiled() {
    let cell = "1 0 0 rg 0 0 5 5 re f";
    let pattern = format!(
        "<< /PatternType 1 /PaintType 1 /TilingType 1 /BBox [0 0 10 10] \
         /XStep 10 /YStep 10 /Resources << >> /Length {} >>\nstream\n{cell}\nendstream",
        cell.len().saturating_add(1)
    );
    let content = "BT /F0 120 Tf /Pattern cs /P0 scn 5 10 Td (A) Tj ET";
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] \
         /Resources << /Pattern << /P0 5 0 R >> \
         /Font << /F0 << /Type /Font /Subtype /Type1 /BaseFont /Helvetica >> >> >> \
         /Contents 4 0 R >>\nendobj\n\
         4 0 obj\n<< /Length {} >>\nstream\n{content}\nendstream\nendobj\n\
         5 0 obj\n{pattern}\nendobj\n",
        content.len().saturating_add(1)
    );
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

    let raster = render(out.into_bytes());
    let mut painted = 0u32;
    let mut clear = 0u32;
    for y in 0..100u32 {
        for x in 0..100u32 {
            let (red, _, _, alpha) = pixel(&raster, x, y);
            if alpha == 255 && red > 240 {
                painted += 1;
            } else if alpha == 0 {
                clear += 1;
            }
        }
    }
    // A 120-point `A` covers a large part of a 100-unit page, and the cell paints a quarter of
    // its area — so a tiled glyph leaves far more of the page clear than a solidly filled one,
    // and both numbers have to be substantial for the pattern to have been the paint.
    assert!(painted > 200, "the glyph is painted somewhere: {painted}");
    assert!(
        clear > 7000,
        "and three quarters of every cell it covers is left clear: {clear}"
    );
}

/// A coloured tiling pattern's cell — §8.7.3.2's `/PaintType 1` — starts from the *initial*
/// graphics state, which Table 75 says in ISO 32000-2 §8.7.3.1:
///
/// > The current colours in use when the PDF processor begins processing the content stream
/// > are the ones initially in effect in the pattern's parent content stream.
///
/// "Initially in effect" rather than "in effect": the cell inherits the state the parent
/// stream *started* with, not the state at the fill. The distinction is invisible on a
/// nonstroking colour, because setting `/Pattern cs` has already replaced that one — so this
/// fixture asks it of the *stroking* colour, which a fill with a pattern never touches. The
/// page sets a red stroke and the cell strokes without setting one; the clause says the line
/// is black.
///
/// A reader that runs the cell under the state at the fill draws it red, and nothing reports
/// that: it is a plausible colour, painted in the right place, in the right shape.
#[test]
fn a_coloured_cell_starts_from_the_streams_initial_colours_rather_than_the_fills() {
    let content = "4 w 2 10 m 18 10 l S";
    let pattern = format!(
        "<< /PatternType 1 /PaintType 1 /TilingType 1 /BBox [0 0 20 20] \
         /XStep 20 /YStep 20 /Resources << >> /Length {} >>\nstream\n{content}\nendstream",
        content.len().saturating_add(1)
    );
    let raster = render(pdf_with(
        &pattern,
        "1 0 0 RG /Pattern cs /P0 scn 0 0 100 100 re f",
    ));

    // The stroke sits at pattern-space y = 10, which is page y = 10 in the first row of
    // cells and device row 89 once the page is flipped.
    let (red, green, blue, alpha) = pixel(&raster, 10, 89);
    assert_eq!(alpha, 255, "the cell's line should be painted here");
    assert!(
        red < 15 && green < 15 && blue < 15,
        "and be the initial black rather than the page's red: {red},{green},{blue}"
    );
}

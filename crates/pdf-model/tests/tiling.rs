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
    with_extra_object(pattern, content, "")
}

/// [`pdf_with`], plus one more numbered object appended after the pattern.
///
/// Object 6 onwards, so a pattern's own resources can name something the page does not — which
/// is what a cell holding an `/ExtGState` needs.
fn with_extra_object(pattern: &str, content: &str, extra: &str) -> Vec<u8> {
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] \
         /Resources << /Pattern << /P0 5 0 R >> /ColorSpace << /CS0 6 0 R >> \
         /ExtGState << /Half << /ca 0.5 >> >> >> /Contents 4 0 R >>\nendobj\n\
         4 0 obj\n<< /Length {} >>\nstream\n{content}\nendstream\nendobj\n\
         5 0 obj\n{pattern}\nendobj\n{extra}",
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
        .with_medium(pdf_render::Medium::NONE)
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

/// The stencil's colour is read in the *underlying* space, not in one guessed from the operands.
///
/// ISO 32000-2 §8.7.3.3 makes the underlying space a `shall` and says where it is written:
///
/// > A Pattern colour space representing an uncoloured tiling pattern shall have a parameter: an
/// > object identifying the underlying colour space in which the actual colour of the pattern
/// > shall be specified. The underlying colour space shall be given as the second element of the
/// > array that defines the Pattern colour space.
///
/// **The two tests above cannot see that half of the clause**, and nothing else in this tree could
/// either: they write a bare `/Pattern cs`, for which there is no stated space and the operand
/// count is the only evidence — so `content::pattern`'s fallback picks `DeviceGray`, `DeviceRGB` or
/// `DeviceCMYK` by arity, and for a *device* base that fallback and the stated space agree on every
/// value. Dropping the base from `ColourSpace::parse_at` altogether failed no test in the workspace,
/// which is how this gap was found.
///
/// A `Separation` base is what tells them apart: one operand, so the fallback would read it as a
/// `DeviceGray` level, while the clause reads it as a tint through the space's own transform. The
/// tint of 1.0 is red here and the grey of 1.0 is white, which is also the page's background — so
/// the assertion is on a channel rather than on a pixel being marked, and a stencil that painted
/// the fallback's colour would be invisible rather than merely wrong.
#[test]
fn an_uncoloured_patterns_colour_is_read_in_its_underlying_space() {
    let underlying = "6 0 obj\n[/Pattern 7 0 R]\nendobj\n\
                      7 0 obj\n[/Separation /Spot /DeviceRGB 8 0 R]\nendobj\n\
                      8 0 obj\n<< /FunctionType 2 /Domain [0 1] /C0 [1 1 1] /C1 [1 0 0] /N 1 >>\n\
                      endobj\n";
    let raster = render(with_extra_object(
        &dotted_cell(2, ""),
        "/CS0 cs 1 /P0 scn 0 0 100 100 re f",
        underlying,
    ));

    let (r, g, b, a) = pixel(&raster, 4, 95);
    assert_eq!(a, 255, "the stencil should paint");
    assert!(
        r > 240 && g < 15 && b < 15,
        "a tint of 1.0 in the Separation base is that space's red, not a DeviceGray white: \
         got {r},{g},{b}"
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

/// A transfer function inside an uncoloured cell is ignored, like every other colour entry.
///
/// ISO 32000-2 §8.6.8 states the rule as a list rather than as a principle, and `/TR` is on it:
///
/// > All of the following entries, if present in the graphics state parameter dictionary of a gs
/// > operator shall be ignored:
///
/// followed by a table whose members are `TR`, `TR2`, `BG`, `BG2`, `UCR`, `UCR2`, `HT` and
/// `UseBlackPtComp`. §10.5's transfer function arrived in the three-hundred-and-fifty-eighth
/// session and its `/ExtGState` reader was not put behind that flag, so an uncoloured cell could
/// decide a colour §8.6.8 reserves for whoever uses the pattern — found in the
/// three-hundred-and-seventy-fifth by a sweep of the phrase the same clause's comment used.
///
/// The fixture's function is `{ pop 0 }`: every component to zero, which would paint the cell
/// black. `scn` supplies blue, and blue is what must come out.
#[test]
fn an_uncoloured_cell_that_sets_a_transfer_function_is_ignored() {
    let function = "6 0 obj\n<< /FunctionType 4 /Domain [0 1] /Range [0 1] /Length 10 >>\n\
                    stream\n{ pop 0 }\nendstream\nendobj\n";
    let cell = dotted_cell(2, "/Dark gs").replace(
        "/Resources << >>",
        "/Resources << /ExtGState << /Dark << /TR 6 0 R >> >> >>",
    );
    let raster = render(with_extra_object(
        &cell,
        "/Pattern cs 0 0 1 /P0 scn 0 0 100 100 re f",
        function,
    ));

    let (r, g, b, a) = pixel(&raster, 4, 95);
    assert_eq!(a, 255, "the stencil should paint");
    assert!(
        b > 240 && r < 15 && g < 15,
        "§8.6.8 ignores the cell's own /TR, so this stays the scn blue: got {r},{g},{b}"
    );
}

/// The same fixture *outside* an uncoloured cell, where the transfer function does apply.
///
/// The pair is what makes the test above a statement about §8.6.8 rather than about
/// `Transfer::read` having been broken: the identical function on a coloured tiling turns its
/// blue cell black, which is §10.5 working.
#[test]
fn the_same_transfer_function_applies_to_a_coloured_cell() {
    let function = "6 0 obj\n<< /FunctionType 4 /Domain [0 1] /Range [0 1] /Length 10 >>\n\
                    stream\n{ pop 0 }\nendstream\nendobj\n";
    let cell = dotted_cell(1, "/Dark gs 0 0 1 rg").replace(
        "/Resources << >>",
        "/Resources << /ExtGState << /Dark << /TR 6 0 R >> >> >>",
    );
    let raster = render(with_extra_object(
        &cell,
        "/Pattern cs /P0 scn 0 0 100 100 re f",
        function,
    ));

    let (r, g, b, a) = pixel(&raster, 4, 95);
    assert_eq!(a, 255, "the cell should paint");
    assert!(
        r < 15 && g < 15 && b < 15,
        "every component maps to zero, so the blue cell is black: got {r},{g},{b}"
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

/// Every site a fill states is painted, however many that is, while the budget affords them.
///
/// §8.7.3.1 asks the processor to "paint the cell on the current page as many times as necessary
/// to fill an area". Until the eight-hundred-and-eighty-second session a constant, `MAX_TILES`,
/// capped that at 4096 sites whatever the cell held — so this fixture, a unit cell at a unit
/// step over a 100-unit fill, which [`span`](../src/content/pattern.rs)'s floor and ceil make 102
/// columns by 102 rows, painted its lowest forty rows and reported the rest (ADR 0477, on
/// `7803372.pdf` of the crawl, two hatched table columns this tree left white). ADR 0810 retired
/// the constant: the sites are copies charged to `MAX_OPERATIONS` and to the tiling's own
/// `MAX_TILE_COPIES` (ADR 0430, ADR 0810), an empty cell loops nothing, a site the fill cannot
/// reach is not copied, and the count a file states is bounded by the commands it costs rather than
/// by a number of its own. So the whole square carries the pattern and nothing is reported — the top row that
/// used to be the control for "past the budget" is now the control for "the budget was not the
/// question". `hostile_budgets.rs` holds the case that *does* reach the budget.
#[test]
fn every_site_the_fill_states_is_painted() {
    let unit = dotted_cell(1, "1 0 0 rg")
        .replace("/BBox [0 0 20 20]", "/BBox [0 0 1 1]")
        .replace("/XStep 20 /YStep 20", "/XStep 1 /YStep 1")
        .replace("0 0 10 10 re f", "0 0 1 1 re f");
    let bytes = pdf_with(&unit, "/Pattern cs /P0 scn 0 0 100 100 re f");
    let document = Document::open(bytes).expect("valid PDF");
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    let interpretation = pdf_model::interpret(&document, &page);

    let reported = format!("{:?}", interpretation.unsupported);
    assert_eq!(
        reported, "[]",
        "ten thousand sites of a one-command cell are inside the operations budget and refused \
         nowhere: {reported}"
    );

    let list = interpretation.display_list;
    let target = TargetSpec::for_page(&list, 1.0, GENEROUS).expect("valid target");
    let raster = CpuRasterizer::new()
        .with_medium(pdf_render::Medium::NONE)
        .rasterize(&list, target)
        .expect("supported");

    assert_eq!(
        pixel(&raster, 50, 90),
        (255, 0, 0, 255),
        "the bottom of the square carries the cell, as it always did"
    );
    assert_eq!(
        pixel(&raster, 50, 10),
        (255, 0, 0, 255),
        "and so does the top, which the retired count used to leave unpainted"
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

/// [`pdf_with`], with a standard font named `/F0` so that a fixture can show text.
///
/// No font program is embedded, so a test built on this is about the *paint* rather than about
/// glyph outlines.
fn pdf_with_font(pattern: &str, content: &str) -> Vec<u8> {
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

/// A cell painting the lower-left quarter of every 10-unit square, in red.
///
/// The gaps are what a *pattern* has and a solid colour does not, so a test asserting a clear
/// pixel inside the mark is asserting that the paint was the pattern.
fn quartered_cell() -> String {
    let content = "1 0 0 rg 0 0 5 5 re f";
    format!(
        "<< /PatternType 1 /PaintType 1 /TilingType 1 /BBox [0 0 10 10] \
         /XStep 10 /YStep 10 /Resources << >> /Length {} >>\nstream\n{content}\nendstream",
        content.len().saturating_add(1)
    )
}

/// A *stroke* whose colour is a tiling pattern is the cell cut to the stroke's own outline.
///
/// ISO 32000-2 §8.7.2 makes a stroking pattern a colour for `SCN` exactly as a nonstroking one
/// is for `scn`:
///
/// > All patterns shall be treated as colours; a Pattern colour space shall be established with
/// > the CS or cs operator just like other colour spaces, and a particular pattern shall be
/// > installed as the current colour with the SCN or scn operator
///
/// It was reported rather than drawn until the eight-hundred-and-second session, on ADR 0028's
/// reason — the outline is the backends' to compute — which reaches the construction that tiles
/// an outline *as a path* and not the one used here, where the outline is a soft mask each
/// backend derives with the expander it already has (ADR 0735).
///
/// The fixture is a diagonal rule, chosen so that the three pixels below each fail for a
/// different reason. The stroke runs from (10, 10) to (90, 90) at width 10, so its mark is a
/// band about the line `y = x` while its *bounding box* is most of the page.
#[test]
fn a_stroke_whose_colour_is_a_tiling_pattern_is_tiled_along_its_outline() {
    // `0 1 0 RG` before the pattern: if the tiling is skipped the stroke falls back to the last
    // solid stroking colour, and green appears nowhere in the pattern.
    let raster = render(pdf_with(
        &quartered_cell(),
        "0 1 0 RG /Pattern CS /P0 SCN 10 w 10 10 m 90 90 l S",
    ));
    // On the line and inside a painted quarter (50..55 in both axes).
    assert_eq!(
        pixel(&raster, 51, 99 - 51),
        (255, 0, 0, 255),
        "the cell paints where the stroke's outline covers it"
    );
    // On the line and inside the cell's *gap* (55..60), which a solid stroke would have filled.
    assert_eq!(
        pixel(&raster, 56, 99 - 56).3,
        0,
        "and leaves the gaps between figures clear, which a solid colour would not"
    );
    // Inside a painted quarter (10..15 × 80..85) and nowhere near the line: this is the pixel
    // the stroke's shape has to remove, and the one that fails if the tiles are cut to the
    // stroke's bounding box instead of to its outline.
    assert_eq!(
        pixel(&raster, 13, 99 - 83).3,
        0,
        "and paints nothing outside the outline, only inside its bounding box"
    );
    let green = (0..100u32)
        .flat_map(|y| (0..100u32).map(move |x| (x, y)))
        .filter(|&(x, y)| {
            let (red, green, blue, alpha) = pixel(&raster, x, y);
            alpha > 0 && green > red && green > blue
        })
        .count();
    assert_eq!(green, 0, "and never in the solid colour SCN replaced");
}

/// A glyph *stroked* in a tiling pattern is tiled too, ISO 32000-2 §9.3.6's modes 1, 2, 5 and 6.
///
/// §8.7.2 makes a stroking pattern a colour, and a glyph's outline is stroked by the same
/// parameters a path's is. **`path.rs` reported this and `text.rs` did not**, which is what
/// §8.7.3's ledger row hid for as long as it named one corpus document on the path route and
/// read as though the gap were covered everywhere: a `Tr 1` glyph was outlined in whatever solid
/// colour was last set, silently (session 630). Both routes tile since ADR 0735, and this test
/// is what fails if only one of them does.
#[test]
fn a_glyph_stroked_in_a_tiling_pattern_is_tiled() {
    // `1 Tr` is stroke-only, so nothing but the stroking colour decides what this glyph gets,
    // and the green is the fallback this test exists to catch.
    let raster = render(pdf_with_font(
        &quartered_cell(),
        "BT /F0 120 Tf 0 1 0 RG /Pattern CS /P0 SCN 3 w 1 Tr 5 10 Td (A) Tj ET",
    ));
    let mut red = 0u32;
    let mut green = 0u32;
    for y in 0..100u32 {
        for x in 0..100u32 {
            let (r, g, b, alpha) = pixel(&raster, x, y);
            if alpha > 0 && r > g && r > b {
                red += 1;
            } else if alpha > 0 && g > r && g > b {
                green += 1;
            }
        }
    }
    assert!(
        red > 20,
        "the glyph's outline is stroked in the cell: {red}"
    );
    assert_eq!(
        green, 0,
        "and never in the solid colour the pattern replaced"
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
    let raster = render(pdf_with_font(
        &pattern,
        "BT /F0 120 Tf /Pattern cs /P0 scn 5 10 Td (A) Tj ET",
    ));
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

/// ISO 32000-2 §11.6.7: the alpha constant applies to the pattern, not to each of its marks.
///
/// > the pattern definition shall be treated as if it were implicitly enclosed in a
/// > non-isolated transparency group: a non-knockout group for tiling patterns, a knockout
/// > group for shading patterns. The definition shall not inherit the current values of the
/// > graphics state parameters at the time it is evaluated; those parameters shall take effect
/// > only when the resulting pattern is later used to paint an object.
///
/// A cell drawing two overlapping shapes is what makes the difference visible, and the visible
/// quantity is the *alpha* rather than the colour: with the constant on each mark, the second
/// composites over the first and the overlap reaches an alpha of 0.75; with it on the finished
/// pattern, the marks composite opaquely inside the group and the whole thing arrives at 0.5.
/// A cell with one shape gives the same answer under either model, which is why this fixture
/// has two.
///
/// NOTE 2 asks for the same construction from the other end: "[i]n a raster-based
/// implementation of tiling, it is advisable to treat all tiles as a single transparency group.
/// This avoids artifacts due to multiple marking of pixels along the boundaries between
/// adjacent tiles."
///
/// **No corpus document reaches this**: all 122 tiling-pattern paints in the 974 documents are
/// under a default alpha, blend mode and soft mask, measured. So the rule is here on the
/// clause's evidence, as trap 8 describes.
#[test]
fn the_alpha_constant_applies_to_the_finished_pattern_rather_than_to_each_mark() {
    let content = "1 0 0 rg 0 0 12 12 re f 0 0 1 rg 6 6 12 12 re f";
    let pattern = format!(
        "<< /PatternType 1 /PaintType 1 /TilingType 1 /BBox [0 0 20 20] \
         /XStep 20 /YStep 20 /Resources << >> /Length {} >>\nstream\n{content}\nendstream",
        content.len().saturating_add(1)
    );
    let raster = render(pdf_with(
        &pattern,
        "/Half gs /Pattern cs /P0 scn 0 0 100 100 re f",
    ));

    // Page (9, 9) is inside both rectangles; the raster's rows run the other way.
    let (_, _, blue, alpha) = pixel(&raster, 9, 90);
    assert!(
        (120..=136).contains(&alpha),
        "the overlap is the pattern at 0.5, not two marks at 0.5 over each other \
         (which would be 0.75, or about 191): got {alpha}"
    );
    assert!(
        blue > 200,
        "and the topmost mark is what survives inside the group: {blue}"
    );

    // Either rectangle alone is the same under both models, which is the control.
    assert!(
        (120..=136).contains(&pixel(&raster, 2, 97).3),
        "a single mark is the pattern's own alpha either way"
    );
}

/// A pattern painted with everything at its default costs no group at all.
///
/// §11.4.4's NOTE 5: "the effect of compositing objects as a group is the same as that of
/// compositing them separately (without grouping)" where the group is non-isolated,
/// non-knockout, Normal, alpha 1.0 and unmasked — which is every tiling pattern in the corpus.
/// Building one anyway would put a page-sized buffer behind 122 corpus paints for nothing, so
/// this checks the commands stay inline.
#[test]
fn a_pattern_that_composites_trivially_is_not_wrapped_in_a_group() {
    let document = Document::open(pdf_with(
        &dotted_cell(1, "1 0 0 rg"),
        "/Pattern cs /P0 scn 0 0 100 100 re f",
    ))
    .expect("the fixture is a valid PDF");
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    let list = pdf_model::interpret(&document, &page).display_list;
    assert!(
        !list
            .commands()
            .iter()
            .any(|command| matches!(command, pdf_render::Command::Group { .. })),
        "nothing composites, so §11.4.4's NOTE 5 says there is no group to build"
    );
}

/// The cell's content stream is interpreted **once**, however many sites the tiling has.
///
/// §8.7.3.1 defines the cell as "the painting operators needed to paint one instance of the
/// cell" and then replicates *the cell*, not the reading of it — so a site is the cell's marks
/// moved. This asserts the difference where it is observable: the operators inside a cell are
/// charged to `MAX_OPERATIONS` once rather than once per site, so a cell of two hundred thousand
/// operators over a hundred and sixty-nine sites is thirty-four million operators when it is
/// re-read and two hundred thousand when it is read once. The first refuses the page by name;
/// the second draws it.
///
/// It is the observable that matters rather than the arithmetic: a route decision is invisible
/// in its output (`nested_content_window.rs` says the same about the window), and this is the
/// one thing a page can be asked that answers it.
#[test]
fn a_cells_operators_are_read_once_and_not_once_per_site() {
    let mut content = String::from("1 0 0 rg 0 0 10 10 re f");
    // `q`/`Q` mark nothing and cost an operator each, which is what makes this a test about the
    // reading rather than about the drawing.
    for _ in 0..100_000 {
        content.push_str(" q Q");
    }
    let pattern = format!(
        "<< /PatternType 1 /PaintType 1 /TilingType 1 /BBox [0 0 20 20] \
         /XStep 10 /YStep 10 /Resources << >> /Length {} >>\nstream\n{content}\nendstream",
        content.len().saturating_add(1)
    );
    let document = Document::open(pdf_with(&pattern, "/Pattern cs /P0 scn 0 0 100 100 re f"))
        .expect("the fixture is a valid PDF");
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    let interpretation = pdf_model::interpret(&document, &page);

    assert!(
        interpretation.is_complete(),
        "the cell is read once, so its operators are counted once: {:?}",
        interpretation.unsupported
    );
    let fills = interpretation
        .display_list
        .commands()
        .iter()
        .filter(|command| matches!(command, pdf_render::Command::Fill { .. }))
        .count();
    assert_eq!(
        fills, 169,
        "and every site is still drawn: thirteen columns by thirteen rows, which is a \
         hundred-unit path at a step of ten plus the sites whose twenty-unit box reaches it"
    );
}

/// Every site is the cell's figure translated by the lattice, and by nothing else.
///
/// The direct statement of what a copy means, on the geometry rather than on the raster: the
/// commands come out in painting order, so site *n*'s fill is site zero's with `/XStep` and
/// `/YStep` added — through the pattern matrix, which here is the identity.
#[test]
fn each_site_is_the_cells_marks_displaced_by_the_lattice() {
    let document = Document::open(pdf_with(
        &dotted_cell(1, "1 0 0 rg"),
        "/Pattern cs /P0 scn 0 0 60 60 re f",
    ))
    .expect("the fixture is a valid PDF");
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    let list = pdf_model::interpret(&document, &page).display_list;
    let placements: Vec<(f32, f32)> = list
        .commands()
        .iter()
        .filter_map(|command| match command {
            pdf_render::Command::Fill { transform, .. } => Some((transform.e, transform.f)),
            _ => None,
        })
        .collect();

    // A 60-unit path at a step of 20 reaches five columns and five rows: `span` measures from
    // the cell's own extent, so the site one step below the path is reached too.
    assert_eq!(placements.len(), 25, "five columns by five rows");
    let (x0, y0) = placements[0];
    let mut sites = placements.iter();
    for row in [0.0_f32, 1.0, 2.0, 3.0, 4.0] {
        for column in [0.0_f32, 1.0, 2.0, 3.0, 4.0] {
            let (x, y) = sites.next().expect("a site per lattice point");
            assert!(
                (x - column.mul_add(20.0, x0)).abs() < 1e-3
                    && (y - row.mul_add(20.0, y0)).abs() < 1e-3,
                "the site at column {column}, row {row} sits at ({x},{y}) rather than at the \
                 lattice point it belongs to"
            );
        }
    }
}

/// A pattern named inside a cell is anchored to **the cell**, not to the page (§8.7.2).
///
/// > A pattern can be used within another pattern
///
/// — and the sentence finishes by saying that the inner pattern's matrix defines its
/// relationship to the pattern space of the *outer* pattern. (The standard breaks
/// "relationship" across a line, so only the first clause is quoted.)
///
/// §8.7.3.1's own picture is what makes that observable: "the effect is as if the figure were
/// painted on the surface of a clear glass tile, identical copies of which were then laid down
/// in an array". A gradient anchored to the page instead of to the cell makes the tiles differ
/// from one another, which is not an array of identical copies. `issue8565.pdf` is the corpus
/// document that states one — a page-sized cell whose fill is a radial shading pattern under a
/// luminosity mask — and it drew a flat colour until the five-hundred-and-ninety-fifth session
/// noticed the anchoring while checking something else.
///
/// Asserted by putting the gradient's own space beside the marks it paints: the inner pattern
/// states no `/Matrix`, so its space *is* the cell's, and a gradient anchored to the cell has
/// exactly the transform the cell's fill has. Anchored to the page it has the page's, which is
/// the same for every site and is what made the first site's gradient belong to a cell at the
/// origin rather than to the cell being painted.
#[test]
fn a_shading_pattern_inside_a_cell_is_anchored_to_the_cell() {
    let cell = "/Pattern cs /Inner scn 0 0 20 20 re f";
    let pattern = format!(
        "<< /PatternType 1 /PaintType 1 /TilingType 1 /BBox [0 0 20 20] \
         /XStep 20 /YStep 20 /Resources << /Pattern << /Inner 6 0 R >> >> /Length {} >>\n\
         stream\n{cell}\nendstream",
        cell.len().saturating_add(1)
    );
    let inner = "6 0 obj\n<< /PatternType 2 /Shading << /ShadingType 2 /ColorSpace /DeviceRGB \
                 /Coords [0 0 20 0] /Function << /FunctionType 2 /Domain [0 1] /C0 [1 0 0] \
                 /C1 [0 0 1] /N 1 >> /Extend [true true] >> >>\nendobj\n";
    let document = Document::open(with_extra_object(
        &pattern,
        "/Pattern cs /P0 scn 0 0 60 60 re f",
        inner,
    ))
    .expect("the fixture is a valid PDF");
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    let list = pdf_model::interpret(&document, &page).display_list;
    let sites: Vec<((f32, f32), (f32, f32))> = list
        .commands()
        .iter()
        .filter_map(|command| match command {
            pdf_render::Command::Fill {
                transform,
                paint: pdf_render::Paint::Shading(shading),
                ..
            } => Some((
                (transform.e, transform.f),
                (shading.transform.e, shading.transform.f),
            )),
            _ => None,
        })
        .collect();

    assert_eq!(sites.len(), 25, "one gradient per site");
    for (index, (marks, gradient)) in sites.iter().enumerate() {
        assert!(
            (marks.0 - gradient.0).abs() < 1e-3 && (marks.1 - gradient.1).abs() < 1e-3,
            "site {index} paints its marks at {marks:?} out of a gradient anchored at \
             {gradient:?}"
        );
    }
    let first = sites[0].1;
    let end = sites[24].1;
    assert!(
        (end.0 - (first.0 + 80.0)).abs() < 1e-3 && (end.1 - (first.1 + 80.0)).abs() < 1e-3,
        "and the lattice still separates the first gradient from the last: {first:?} {end:?}"
    );
}

/// A cell that stays inside its own box is not clipped to it.
///
/// Table 74 says a cell's box "shall be used to clip the pattern cell", and where the cell
/// draws nothing outside it that clip removes no geometry — so it is taken back off, one
/// clip for the whole tiling instead of one per cell. The saving is not why: an anti-aliased
/// clip mask removes *coverage* from a mark lying on its boundary even when it removes no
/// geometry, which is what the next test measures.
///
/// Asserted on the clips the display list actually references, because that is the thing that
/// changed; the picture is the next test's business.
#[test]
fn a_cell_that_stays_inside_its_box_is_not_clipped_to_it() {
    let content = "1 0 0 rg 2 2 6 6 re f";
    let pattern = format!(
        "<< /PatternType 1 /PaintType 1 /TilingType 1 /BBox [0 0 10 10] \
         /XStep 10 /YStep 10 /Resources << >> /Length {} >>\nstream\n{content}\nendstream",
        content.len().saturating_add(1)
    );
    let document = Document::open(pdf_with(&pattern, "/Pattern cs /P0 scn 0 0 100 100 re f"))
        .expect("the fixture is a valid PDF");
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    let list = pdf_model::interpret(&document, &page).display_list;

    let clips: std::collections::BTreeSet<_> = list
        .commands()
        .iter()
        .map(pdf_render::Command::clip)
        .collect();
    assert_eq!(
        clips.len(),
        1,
        "one clip — the filled path's — for a hundred cells that never reach their own edges"
    );
    assert!(
        list.commands().len() >= 100,
        "the cells are still drawn: {} commands",
        list.commands().len()
    );
}

/// And the ink it deposits is the ink its geometry states.
///
/// This is `issue16038.pdf`'s own geometry in a fixture: a rule spanning **exactly** its cell,
/// 0.3985 wide, repeated every 2.98883 units, which is a cell about three device pixels
/// across. Every cell boundary falls at a fraction of a pixel, and clipping each cell to a box
/// its rule reaches exactly makes the two halves of every boundary pixel composite as
/// `1 − (1−a)(1−b)` instead of adding — 15% of the page's ink, measured on the real file
/// before this was fixed (`AMBIGUOUS_TILING_CELL_CLIP`).
///
/// The expected coverage is the geometry's own: a rule 0.3985 wide every 2.98883 units covers
/// `0.3985 / 2.98883` of the page, and the tolerance is a fiftieth of that. It is a
/// discriminating test in both directions — with the clip applied the coverage is 0.111, and
/// `a_cell_is_clipped_to_its_bounding_box` above fails if the clip is dropped where it is
/// load-bearing.
#[test]
fn a_rule_spanning_its_whole_cell_deposits_the_ink_its_geometry_states() {
    let content = "0 0 0 RG 0.3985 w -1.49442 0 m 1.49442 0 l S";
    let pattern = format!(
        "<< /PatternType 1 /PaintType 1 /TilingType 1 \
         /BBox [-1.49442 -1.49442 1.49442 1.49442] /XStep 2.98883 /YStep 2.98883 \
         /Resources << >> /Length {} >>\nstream\n{content}\nendstream",
        content.len().saturating_add(1)
    );
    let raster = render(pdf_with(&pattern, "/Pattern cs /P0 scn 0 0 100 100 re f"));

    let ink: f64 = raster
        .data
        .chunks_exact(4)
        .map(|pixel| f64::from(pixel[3]) / 255.0)
        .sum();
    let covered = ink / f64::from(raster.width.saturating_mul(raster.height));
    let expected = 0.3985 / 2.98883;
    assert!(
        (covered - expected).abs() < expected / 50.0,
        "the rules cover {covered:.4} of the page where their own width and spacing say \
         {expected:.4}"
    );
}

/// The cell is where its content draws it, and the tiles are that cell stepped from *there*.
///
/// §8.7.3.1 places the pattern cell where its own content stream draws it and replicates that at
/// multiples of `/XStep` and `/YStep`. So the offsets needed to cover a path are measured from
/// the cell's own extent — and until the two-hundred-and-eighteenth session they were measured
/// from the pattern space's origin, which is the same answer for every pattern whose `/BBox` is
/// within one step of it and a wrong one for the rest.
///
/// This cell sits at `[60 60 80 80]` with a step of 20, which is three steps out. The mark must
/// still land on the path, and on the same lattice as a cell drawn at the origin — because that
/// is what "stepped from there" means: 60 is a multiple of 20, so the two patterns tile
/// identically and only a reader measuring from the wrong place can tell them apart.
#[test]
fn a_cell_far_from_the_patterns_origin_still_tiles_onto_the_path() {
    let far = format!(
        "<< /PatternType 1 /PaintType 1 /TilingType 1 /BBox [60 60 80 80] \
         /XStep 20 /YStep 20 /Resources << >> /Length {} >>\nstream\n{}\nendstream",
        "1 0 0 rg 60 60 10 10 re f".len().saturating_add(1),
        "1 0 0 rg 60 60 10 10 re f"
    );
    let raster = render(pdf_with(&far, "/Pattern cs /P0 scn 0 0 100 100 re f"));

    // The same five rows and columns `a_tiling_pattern_repeats_its_cell_across_the_filled_path`
    // checks, because 60 is three whole steps: the lattice is the same one.
    for step in 0..5u32 {
        let across = step * 20 + 4;
        let down = 99 - across;
        let (red, green, blue, alpha) = pixel(&raster, across, down);
        assert_eq!(
            alpha, 255,
            "a cell three steps from the origin must still reach ({across},{down})"
        );
        assert!(red > 240 && green < 15 && blue < 15, "{red},{green},{blue}");
    }
    assert_eq!(pixel(&raster, 15, 85).3, 0, "and the gaps are still gaps");
}

/// A rule stated on both edges of its cell is one rule of the tiling, and weighs one rule.
///
/// This is `issue16038.pdf`'s `/pgfpat22`: the cell strokes a line along the bottom of its box
/// and another along the top, so Table 74's clip keeps half of each and the halves meet. In
/// geometry they do; on the raster the boundary pixel keeps a fraction of one half and a
/// fraction of the other, and two fractions composite as `1 − (1−a)(1−b)` rather than adding.
///
/// The expected coverage is the geometry's own — one rule 0.3985 wide every 2.98883 units — and
/// it is the same number `a_rule_spanning_its_whole_cell_deposits_the_ink_its_geometry_states`
/// checks for the other phase of the same figure, which is the point: the two patterns state
/// the same rules and must weigh the same.
#[test]
fn a_rule_stated_at_both_cell_edges_weighs_one_rule() {
    let content = "0 0 0 RG 0.3985 w 0 0 m 2.98883 0 l 0 2.98883 m 2.98883 2.98883 l S";
    let pattern = format!(
        "<< /PatternType 1 /PaintType 1 /TilingType 1 \
         /BBox [0 0 2.98883 2.98883] /XStep 2.98883 /YStep 2.98883 \
         /Resources << >> /Length {} >>\nstream\n{content}\nendstream",
        content.len().saturating_add(1)
    );
    let raster = render(pdf_with(&pattern, "/Pattern cs /P0 scn 0 0 100 100 re f"));

    let ink: f64 = raster
        .data
        .chunks_exact(4)
        .map(|pixel| f64::from(pixel[3]) / 255.0)
        .sum();
    let covered = ink / f64::from(raster.width.saturating_mul(raster.height));
    let expected = 0.3985 / 2.98883;
    assert!(
        (covered - expected).abs() < expected / 50.0,
        "the rules cover {covered:.4} of the page where their own width and spacing say \
         {expected:.4}"
    );
}

/// And the whole figure — the rules *and* the border they end under — weighs its own area.
///
/// The two tests above measure a tiling's interior, which is the quantity a rule's width over
/// its step states. This one measures `issue16038.pdf` itself, because that page is the corpus's
/// closed form for a whole figure: two squares, each `B`, filled with an uncoloured pattern of
/// rules and stroked at the same 0.3985. Nothing else is on it, so the area it asks for is
/// arithmetic — and until the eight-hundred-and-sixth session the arithmetic was wrong.
///
/// # The area, term by term
///
/// Twenty rules of `28.3468 × 0.3985`, plus a `0.3985` stroke around each of two `28.3468`
/// squares — which is `4 × side × width` exactly, an outer square less an inner one — **less what
/// the two share**. Each rule is a pattern mark clipped to its square's fill path, so it runs to
/// `x = 0` and `x = 28.3468`; the border is a stroke of that same path and §8.4.3.2 puts half its
/// width on each side of it. The shared region is `2 × (w/2) × w = w²` per rule, 0.15881, and
/// there are twenty:
///
/// ```text
/// 20 × 28.3468 × 0.3985  +  2 × 4 × 28.3468 × 0.3985  −  20 × 0.3985²  =  313.117
/// ```
///
/// `AMBIGUOUS_TILING_CELL_CLIP` carried the first two terms without the third — 316.29 — from the
/// three-hundred-and-seventy-fourth session to the eight-hundred-and-sixth, and every percentage
/// in that note was against it. ADR 0738.
///
/// # What it discriminates, at which scale, and what it does not
///
/// Ink is a geometric quantity a rasteriser approaches as the pixels shrink, so the scale is part
/// of the claim: at 24× this tree deposits 313.016 and at the page's own scale 299.86, which is
/// §10.7.4's anti-aliasing departure on a rule half a device pixel wide. The tolerance is half a
/// percent at 24×, and the wrong answer this test exists for is outside it — the closed form
/// without its third term is 1.03% away, and running the test against it is where the figure in
/// the failure message came from.
///
/// **It does not see the defect ADR 0155 fixed, and that was checked rather than assumed.** With
/// `unclip_redundant_cell` made to return `false` — the redundant per-cell box back on — this
/// page deposits 312.975 against 313.016, a movement of 0.013%, because a clip's cost is the
/// anti-aliased seam at a cell boundary and at 24× a boundary pixel is a fortieth of what it is
/// at 1×. That defect was 15% of the ink *at the page's own scale*, and what holds it is
/// `a_rule_spanning_its_whole_cell_deposits_the_ink_its_geometry_states` above, which fails under
/// the same mutation. Two tests, two scales, and neither is the other's spare.
#[test]
fn the_page_that_is_a_closed_form_weighs_what_the_closed_form_says() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../doc/pdf.js/test/pdfs/issue16038.pdf");
    let Ok(bytes) = std::fs::read(&path) else {
        println!("skipped: the doc/pdf.js submodule is not checked out");
        return;
    };

    let document = Document::open(bytes).expect("issue16038.pdf is a valid PDF");
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    let interpretation = pdf_model::interpret(&document, &page);
    assert!(
        interpretation.is_complete(),
        "the page draws completely: {:?}",
        interpretation.unsupported
    );
    let list = interpretation.display_list;
    let scale = 24.0_f32;
    // `Medium::NONE` leaves the page's own alpha, so alpha *is* coverage and no colour
    // weighting enters — which on a page whose rules are pure blue is worth a quarter of a
    // level (ADR 0738).
    let target = TargetSpec::for_page(&list, scale, GENEROUS).expect("valid target");
    let raster = CpuRasterizer::new()
        .with_medium(pdf_render::Medium::NONE)
        .rasterize(&list, target)
        .expect("supported");

    let covered: f64 = raster
        .data
        .chunks_exact(4)
        .map(|pixel| f64::from(pixel[3]) / 255.0)
        .sum();
    let ink = covered / f64::from(scale * scale);

    let rules = 20.0 * 28.3468 * 0.3985;
    let borders = 2.0 * 4.0 * 28.3468 * 0.3985;
    let shared = 20.0 * 0.3985 * 0.3985;
    let area = rules + borders - shared;
    assert!(
        (ink - area).abs() < area / 200.0,
        "the page deposits {ink:.3} square points where its own geometry states {area:.3} \
         ({rules:.3} of rules plus {borders:.3} of border less {shared:.3} they share)"
    );
}

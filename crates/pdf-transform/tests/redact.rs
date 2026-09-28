//! Applying a redaction, ISO 32000-2 §12.5.6.23 (`doc/questions/A64`).
//!
//! The calibration trap 13 asks for: a page with text inside and outside a `/QuadPoints`
//! region, where after application the inside text is **gone from the content stream** and
//! unrecoverable by extraction, the outside byte-identical, and the file re-opens and
//! re-extracts. Plus the overlay departure, the refusals, and the census — a `/Redact` planted
//! inside an object stream (§7.5.7), which a byte-grep cannot see and the parsed census must.

#![expect(
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    clippy::cast_precision_loss,
    reason = "test code: a fixture that cannot be built must fail loudly, its byte offsets are \
              small integers a panic on overflow would only make louder, and a raster coordinate \
              on a 200-unit page is far inside f32's exact range"
)]

use std::fmt::Write as _;

use pdf_model::colour::Conversion;
use pdf_render::Color;
use pdf_syntax::serialize::{Assembly, Form, ObjectStreams, Options, Streams, flate_encode};
use pdf_syntax::{Document, Limits, Object};
use pdf_transform::optimize::OptimizePlan;
use pdf_transform::redact::RedactPlan;
use pdf_transform::{
    Budget, Departure, MemorySinks, Origin, Plan, Policy, Protect, Refusal, Secret, Source, apply,
    apply_protected,
};

mod support;

/// Assembles a one-page PDF: Helvetica in `/F1`, a 200×200 media box, the given content stream
/// and the given annotation dictionaries in `/Annots`.
fn build(content: &str, annotations: &[&str]) -> Vec<u8> {
    let mut objects: Vec<String> = vec![
        "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        String::new(), // page, filled once the annotation object numbers are known
        format!(
            "<< /Length {} >>\nstream\n{content}\nendstream",
            content.len() + 1
        ),
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_owned(),
    ];
    let mut annot_refs = String::new();
    for annotation in annotations {
        let number = objects.len() + 1;
        let _ = write!(annot_refs, "{number} 0 R ");
        objects.push((*annotation).to_owned());
    }
    let annots = if annotations.is_empty() {
        String::new()
    } else {
        format!(" /Annots [{}]", annot_refs.trim_end())
    };
    objects[2] = format!(
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] /Resources << /Font << /F1 5 0 R \
         >> >> /Contents 4 0 R{annots} >>"
    );
    assemble(&objects)
}

/// Writes a flat object list into a §7.5.4 cross-referenced file, objects numbered from one.
fn assemble(objects: &[String]) -> Vec<u8> {
    let mut out = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for (index, object) in objects.iter().enumerate() {
        offsets.push(out.len());
        let _ = writeln!(out, "{} 0 obj {object} endobj", index + 1);
    }
    let at = out.len();
    let size = objects.len() + 1;
    let _ = writeln!(out, "xref\n0 {size}\n0000000000 65535 f ");
    for offset in &offsets {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    let _ = write!(
        out,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{at}\n%%EOF\n"
    );
    out.into_bytes()
}

/// Applies a redaction to the bytes, returning the report and the one output file.
fn redact(bytes: &[u8]) -> (pdf_transform::Report, Vec<u8>) {
    redact_within(bytes, None)
}

/// [`redact`] with the operator's `--image-samples` budget stated.
fn redact_within(bytes: &[u8], image_samples: Option<u64>) -> (pdf_transform::Report, Vec<u8>) {
    let sinks = MemorySinks::new();
    let report = apply(
        &Plan::Redact(RedactPlan {
            source: 0,
            names: "out.pdf".parse().expect("a pattern"),
            image_samples,
        }),
        &[Source::new(bytes.to_vec())],
        &sinks,
        &Policy::default(),
        &Budget::default(),
    )
    .expect("the redaction applies");
    let mut outputs = sinks.into_outputs();
    assert_eq!(outputs.len(), 1, "one input, one output");
    (report, outputs.remove(0).1)
}

/// The extracted text of a document's first page — the same string
/// `pdf_model::interpret(...).text` gives, which is what `pdf-retrieve` returns by default.
fn page_text(bytes: &[u8]) -> String {
    let document = Document::open_with_limits(bytes.to_vec(), Limits::DEFAULT).expect("it opens");
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    pdf_model::interpret(&document, &page).text
}

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    haystack
        .windows(needle.len())
        .any(|window| window == needle)
}

/// The calibration: inside a `/QuadPoints` region the text is gone from the content stream and
/// unrecoverable; outside it the bytes are identical; the file re-opens and re-extracts.
#[test]
fn text_inside_a_quadpoints_region_is_removed_and_outside_it_is_byte_identical() {
    // "KEEP" sits at y = 150, "SECRET" at y = 50; the region is the band [10,40]–[130,66].
    let content = "BT /F1 12 Tf 20 150 Td (KEEP) Tj ET\nBT /F1 12 Tf 20 50 Td (SECRET) Tj ET";
    let bytes = build(
        content,
        &["<< /Type /Annot /Subtype /Redact /Rect [10 40 130 66] \
           /QuadPoints [10 66 130 66 10 40 130 40] >>"],
    );
    assert!(contains(&bytes, b"SECRET"), "the fixture holds the secret");

    let (report, out) = redact(&bytes);

    assert!(
        report.refused.is_empty(),
        "nothing was refused: {:?}",
        report.refused
    );
    assert!(
        !contains(&out, b"SECRET"),
        "the removed text is gone from the file, not merely covered"
    );
    assert!(
        contains(&out, b"(KEEP)"),
        "the surviving show operator is byte-identical"
    );
    assert!(
        !contains(&out, b"/Redact"),
        "§12.5.6.23: the redaction annotation is removed too"
    );

    let text = page_text(&out);
    assert!(
        text.contains("KEEP"),
        "the outside text re-extracts: {text:?}"
    );
    assert!(
        !text.contains("SECRET"),
        "the removed text is unrecoverable by extraction: {text:?}"
    );

    let Some(Origin::Redacted {
        annotations,
        glyphs,
        ..
    }) = report.outputs.first().map(|output| output.origin.clone())
    else {
        panic!("the report states a redacted origin");
    };
    assert_eq!(annotations, 1, "one annotation applied");
    assert_eq!(glyphs, 6, "the six glyphs of SECRET were removed");
}

/// Per-glyph removal inside one show operator: `[(A) -5000 (B)] TJ` places B far to the right,
/// where a narrow region catches it and leaves A. The advance restoration (§9.4.4's `w0`, read
/// from the placed quad) keeps A where it was.
#[test]
fn one_glyph_is_removed_from_the_middle_of_a_show_operator() {
    let content = "BT /F1 12 Tf 20 100 Td [(A) -5000 (B)] TJ ET";
    let bytes = build(
        content,
        &["<< /Type /Annot /Subtype /Redact /Rect [78 94 130 112] >>"],
    );
    let (report, out) = redact(&bytes);

    assert!(
        report.refused.is_empty(),
        "nothing refused: {:?}",
        report.refused
    );
    let Some(Origin::Redacted { glyphs, .. }) =
        report.outputs.first().map(|output| output.origin.clone())
    else {
        panic!("a redacted origin");
    };
    assert_eq!(glyphs, 1, "exactly the one glyph in the region went");
    let text = page_text(&out);
    assert_eq!(text.trim(), "A", "B is gone and A survives: {text:?}");
    assert!(contains(&out, b"TJ"), "the operator was rebuilt as a TJ");
}

/// A redaction that states `/OverlayText` gets the removal and a reported departure: the content
/// is gone, the overlay is not composed (A64/A65's provenance fence).
#[test]
fn an_overlay_is_a_reported_departure_not_a_drawn_mark() {
    let content = "BT /F1 12 Tf 20 50 Td (SECRET) Tj ET";
    let bytes = build(
        content,
        &["<< /Type /Annot /Subtype /Redact /Rect [10 40 130 66] \
           /IC [0 0 0] /OverlayText (REDACTED) /DA (/Helv 0 Tf 0 g) >>"],
    );
    let (report, out) = redact(&bytes);

    assert!(!contains(&out, b"SECRET"), "the content is still removed");
    assert!(
        !contains(&out, b"REDACTED"),
        "the overlay text is not composed onto the page"
    );
    let departures: Vec<&Departure> = report.departures.iter().collect();
    assert_eq!(departures.len(), 1, "the departure is reported once");
    assert_eq!(departures[0].page, Some(1), "on the page it happened");
    assert!(
        departures[0].detail.contains("overlay"),
        "the departure says what was not composed: {:?}",
        departures[0]
    );
}

/// A page whose region also holds a painted path is refused by name — the content and the
/// annotation are left as the file wrote them, never cut wrong (trap 5).
/// Every mark the display list carries, as its extent in the display list's own space.
///
/// A redaction's proof is that no mark survives whose extent meets the region, so the walk has to
/// see *every* command — a group's elements as much as a page-level fill, since a group draws
/// what its elements draw. Clips are not applied: a mark clipped away would read here as a mark,
/// which is the safe direction for a test asserting that nothing is left.
fn mark_extents(commands: &[pdf_render::Command]) -> Vec<pdf_render::geom::Rect> {
    let mut out = Vec::new();
    for command in commands {
        match command {
            pdf_render::Command::Fill {
                path, transform, ..
            }
            | pdf_render::Command::Stroke {
                path, transform, ..
            } => out.extend(path.bounds(*transform)),
            pdf_render::Command::Image { transform, .. } => {
                // §8.9.5.2 places an image in the unit square of user space; its extent is that
                // square under the placing transform.
                out.push(pdf_render::geom::Rect::from_corners(
                    transform.apply(pdf_render::geom::Point::new(0.0, 0.0)),
                    transform.apply(pdf_render::geom::Point::new(1.0, 1.0)),
                ));
            }
            pdf_render::Command::Group { commands, .. } => {
                out.extend(mark_extents(commands));
            }
            pdf_render::Command::Shaped { object, .. } => {
                out.extend(mark_extents(std::slice::from_ref(object)));
            }
            // The enum is non-exhaustive; a command this walk does not know is a mark it cannot
            // measure, so it is reported as covering everything and the caller's assertion fails
            // — never as nothing, which would make a new command read as a clean region.
            _ => out.push(pdf_render::geom::Rect::from_corners(
                pdf_render::geom::Point::new(f32::MIN, f32::MIN),
                pdf_render::geom::Point::new(f32::MAX, f32::MAX),
            )),
        }
    }
    out
}

/// Asserts that no mark in the output's first page reaches into the user-space region.
///
/// The region is mapped through `base_transform` into the display list's own space, which is the
/// space the marks are in — `doc/traps` 12a: the display list's space is not the page's, and the
/// flip lives in that one function.
/// The colour at one page point of a rasterised page, as three eight-bit components.
///
/// The fixtures' page is `PAGE_POINTS` square and the reference rasterises it at its own
/// resolution, so the point is scaled by the raster's width rather than assumed to be a pixel.
/// Integer arithmetic throughout, because every point asked about here is a whole number.
fn page_pixel(raster: &pdf_render::Raster, x: u32, y: u32) -> [u8; 3] {
    let column = x.saturating_mul(raster.width) / PAGE_POINTS;
    let row = y.saturating_mul(raster.height) / PAGE_POINTS;
    let scan = raster.height.saturating_sub(1).saturating_sub(row);
    let at = usize::try_from(scan.saturating_mul(raster.width).saturating_add(column))
        .expect("a raster index fits a usize")
        .saturating_mul(4);
    [raster.data[at], raster.data[at + 1], raster.data[at + 2]]
}

/// The side of the square page every fixture in this file draws on, in points.
const PAGE_POINTS: u32 = 200;

fn no_mark_meets(bytes: &[u8], region: [f32; 4]) {
    let document = Document::open_with_limits(bytes.to_vec(), Limits::DEFAULT).expect("it opens");
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    let base = pdf_model::content::base_transform(&page);
    let corners = [
        base.apply(pdf_render::geom::Point::new(region[0], region[1])),
        base.apply(pdf_render::geom::Point::new(region[2], region[3])),
    ];
    let quad = pdf_render::geom::Rect::from_corners(corners[0], corners[1]);
    let list = pdf_model::interpret(&document, &page).display_list;
    for extent in mark_extents(list.commands()) {
        assert!(
            extent.intersection(quad).is_none(),
            "a mark at {extent:?} still reaches the redacted region {quad:?}"
        );
    }
}

/// A painted path partly under the region keeps the part outside it and loses the part inside:
/// §12.5.6.23 asks for the content to be *removed*, so the geometry that described the removed
/// marks is gone from the content stream rather than covered or clipped.
#[test]
fn a_painted_path_is_cut_to_the_region_s_complement() {
    // A black bar from x = 20 to x = 120 at y = 40..60; the region takes everything from x = 60.
    let content = "0 0 0 rg 20 40 100 20 re f";
    let bytes = build(
        content,
        &["<< /Type /Annot /Subtype /Redact /Rect [60 30 140 80] >>"],
    );
    let (report, out) = redact(&bytes);

    assert!(
        report.refused.is_empty(),
        "the path is cut, not refused: {:?}",
        report.refused
    );
    let Some(Origin::Redacted { paths, .. }) =
        report.outputs.first().map(|output| output.origin.clone())
    else {
        panic!("a redacted origin");
    };
    assert_eq!(paths, 1, "one painted path was cut");
    assert!(
        !contains(&out, b"20 40 100 20 re"),
        "the rectangle that described the removed marks is gone from the file"
    );
    no_mark_meets(&out, [60.0, 30.0, 140.0, 80.0]);

    // What survives is the bar's left end, and it is still painted.
    let document = Document::open_with_limits(out.clone(), Limits::DEFAULT).expect("it opens");
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    let extents = mark_extents(
        pdf_model::interpret(&document, &page)
            .display_list
            .commands(),
    );
    assert_eq!(extents.len(), 1, "one surviving mark: {extents:?}");
    let base = pdf_model::content::base_transform(&page);
    let expected = pdf_render::geom::Rect::from_corners(
        base.apply(pdf_render::geom::Point::new(20.0, 40.0)),
        base.apply(pdf_render::geom::Point::new(60.0, 60.0)),
    );
    let survivor = extents[0];
    assert!(
        (survivor.min.x - expected.min.x).abs() < 0.05
            && (survivor.max.x - expected.max.x).abs() < 0.05
            && (survivor.min.y - expected.min.y).abs() < 0.05
            && (survivor.max.y - expected.max.y).abs() < 0.05,
        "the survivor is the bar's left end: {survivor:?} vs {expected:?}"
    );
}

/// The pixels a page draws, at 150 dpi through the correctness-oracle backend.
fn pixels(bytes: &[u8]) -> pdf_render::Raster {
    support::oracle(bytes, 0)
}

/// The cut proved in pixels rather than in operators: outside the region the redacted page is
/// identical to the original, and inside it nothing is drawn.
///
/// The comparison skips a one-pixel band around the region, because the cut deliberately reaches
/// [`REGION_PAD`]'s hundredth of a point past the quad so that writing the new vertices back and
/// reading them as §7.3.3 reals cannot leave a sliver of the removed marks alive. A boundary
/// pixel therefore carries a fraction of a percent less coverage, which is the safe direction and
/// not a difference the comparison is about.
#[test]
fn the_cut_page_is_pixel_identical_outside_the_region_and_empty_inside_it() {
    let content = "0 0 0 rg 20 40 100 20 re f\n0 0 1 rg 20 150 30 10 re f";
    let bytes = build(
        content,
        &["<< /Type /Annot /Subtype /Redact /Rect [60 30 140 80] >>"],
    );
    let (report, out) = redact(&bytes);
    assert!(report.refused.is_empty(), "{:?}", report.refused);

    let before = pixels(&bytes);
    let after = pixels(&out);
    assert_eq!((before.width, before.height), (after.width, after.height));
    assert_eq!(before.format, after.format);

    // The region in the raster's own space: 150 dpi over 72 units to the inch, and y measured
    // down from the top of a 200-unit page (trap 12a).
    let scale = 150.0_f32 / 72.0;
    let left = 60.0 * scale;
    let right = 140.0 * scale;
    let top = (200.0 - 80.0) * scale;
    let bottom = (200.0 - 30.0) * scale;

    let stride = before.width as usize * 4;
    let mut inside = 0usize;
    let mut marked = false;
    for y in 0..before.height {
        for x in 0..before.width {
            let at = y as usize * stride + x as usize * 4;
            let (fx, fy) = (x as f32, y as f32);
            // A one-pixel band either side of the boundary is skipped: the cut reaches a
            // hundredth of a point past the quad by construction (`paths::REGION_PAD`), and a
            // boundary pixel's coverage is a rasteriser's business rather than this test's.
            let well_inside =
                fx > left + 1.0 && fx < right - 1.0 && fy > top + 1.0 && fy < bottom - 1.0;
            let well_outside =
                fx < left - 1.0 || fx > right + 1.0 || fy < top - 1.0 || fy > bottom + 1.0;
            if well_inside {
                inside += 1;
                marked |= before.data[at] != 0xFF;
                // Nothing is drawn there: the oracle's page starts white and stays white.
                assert_eq!(
                    &after.data[at..at + 3],
                    &[0xFF, 0xFF, 0xFF],
                    "a pixel at ({x}, {y}) inside the region is still marked"
                );
            } else if well_outside {
                assert_eq!(
                    &after.data[at..at + 4],
                    &before.data[at..at + 4],
                    "a pixel at ({x}, {y}) outside the region changed"
                );
            }
        }
    }
    assert!(inside > 1000, "the region covers a real part of the page");
    // And the comparison could have failed: the original does mark the region.
    assert!(
        marked,
        "the original page marks the region, so the test can fail"
    );
}

/// A painted path wholly inside the region loses every one of its marks, and the painting
/// operator goes with the geometry — an empty path painted is not what the producer wrote.
#[test]
fn a_painted_path_inside_the_region_is_deleted_entirely() {
    let content = "0 0 0 rg 20 40 30 10 re f\n0 0 0 rg 20 150 30 10 re f";
    let bytes = build(
        content,
        &["<< /Type /Annot /Subtype /Redact /Rect [10 30 140 80] >>"],
    );
    let (report, out) = redact(&bytes);
    assert!(report.refused.is_empty(), "{:?}", report.refused);
    assert!(
        !contains(&out, b"20 40 30 10 re"),
        "the deleted path's geometry is gone"
    );
    assert!(
        contains(&out, b"20 150 30 10 re"),
        "the path clear of the region crosses the output byte for byte"
    );
    no_mark_meets(&out, [10.0, 30.0, 140.0, 80.0]);
}

/// A painted path nowhere near the region is left exactly as the producer wrote it.
#[test]
fn a_painted_path_clear_of_the_region_is_byte_identical() {
    let content = "0 0 0 rg 20 150 30 10 re f\nBT /F1 12 Tf 20 50 Td (SECRET) Tj ET";
    let bytes = build(
        content,
        &["<< /Type /Annot /Subtype /Redact /Rect [10 40 130 66] >>"],
    );
    let (report, out) = redact(&bytes);
    assert!(report.refused.is_empty(), "{:?}", report.refused);
    assert!(!contains(&out, b"SECRET"), "the text under the region went");
    assert!(
        contains(&out, b"20 150 30 10 re f"),
        "the path's own bytes are untouched"
    );
    let Some(Origin::Redacted { paths, .. }) =
        report.outputs.first().map(|output| output.origin.clone())
    else {
        panic!("a redacted origin");
    };
    assert_eq!(paths, 0, "nothing was cut");
}

/// A stroked path meeting the region is cut as the **outline** it marks (§8.5.3.2), and the
/// proof is in pixels: outside the region the page is identical, inside it nothing is drawn.
///
/// The fixture's outline is *exact*, which is what lets the assertion be byte equality: a
/// straight segment offset by half the line width, closed by butt caps and turned by miter
/// joins, is computed in closed form. An outline holding an arc is fitted within a bound instead,
/// and `a_stroke_whose_outline_holds_an_arc_is_cut_at_the_region_edge` holds it to that.
#[test]
fn a_stroked_path_is_cut_as_the_outline_it_marks() {
    let content = "0 0 0 RG 2 w 0 J 0 j 20 50 m 120 50 l S";
    let bytes = build(
        content,
        &["<< /Type /Annot /Subtype /Redact /Rect [60 30 140 80] >>"],
    );
    let (report, out) = redact(&bytes);
    assert!(report.refused.is_empty(), "{:?}", report.refused);
    let Some(Origin::Redacted { paths, .. }) =
        report.outputs.first().map(|output| output.origin.clone())
    else {
        panic!("a redacted origin");
    };
    assert_eq!(paths, 1, "one stroked path was cut");
    assert!(
        !contains(&out, b"20 50 m 120 50 l S"),
        "the centre line that described the removed marks is gone from the file"
    );
    no_mark_meets(&out, [60.0, 30.0, 140.0, 80.0]);

    let before = pixels(&bytes);
    let after = pixels(&out);
    let scale = 150.0_f32 / 72.0;
    let (left, right) = (60.0 * scale, 140.0 * scale);
    let (top, bottom) = ((200.0 - 80.0) * scale, (200.0 - 30.0) * scale);
    let stride = before.width as usize * 4;
    let mut marked = false;
    for y in 0..before.height {
        for x in 0..before.width {
            let at = y as usize * stride + x as usize * 4;
            let (fx, fy) = (x as f32, y as f32);
            if fx > left + 1.0 && fx < right - 1.0 && fy > top + 1.0 && fy < bottom - 1.0 {
                marked |= before.data[at] != 0xFF;
                assert_eq!(
                    &after.data[at..at + 3],
                    &[0xFF, 0xFF, 0xFF],
                    "a pixel at ({x}, {y}) inside the region is still marked"
                );
            } else if fx < left - 1.0 || fx > right + 1.0 || fy < top - 1.0 || fy > bottom + 1.0 {
                assert_eq!(
                    &after.data[at..at + 4],
                    &before.data[at..at + 4],
                    "a pixel at ({x}, {y}) outside the region changed"
                );
            }
        }
    }
    assert!(
        marked,
        "the original stroke marks the region, so the test can fail"
    );
}

/// The distance from `point` to the segment `from`–`to`, all in default user space.
fn distance_to_segment(point: (f64, f64), from: (f64, f64), to: (f64, f64)) -> f64 {
    let (dx, dy) = (to.0 - from.0, to.1 - from.1);
    let along = ((point.0 - from.0) * dx + (point.1 - from.1) * dy) / (dx * dx + dy * dy);
    let t = along.clamp(0.0, 1.0);
    (point.0 - (from.0 + t * dx)).hypot(point.1 - (from.1 + t * dy))
}

/// The eight-bit grey a pixel of [`pixels`]' raster owes, from the fraction of it `inside`
/// covers — black ink on white paper, sampled on a 32 × 32 grid, which is a quarter of an
/// eight-bit step at worst. `inside` is asked in default user space (trap 12a's flip).
fn exact_grey(x: u32, y: u32, inside: &dyn Fn(f64, f64) -> bool) -> f64 {
    let to_user = 72.0 / 150.0;
    let mut covered = 0_u32;
    for row in 0..32 {
        for column in 0..32 {
            let px = f64::from(x) + (f64::from(column) + 0.5) / 32.0;
            let py = f64::from(y) + (f64::from(row) + 0.5) / 32.0;
            if inside(px * to_user, 200.0 - py * to_user) {
                covered += 1;
            }
        }
    }
    255.0 - 255.0 * f64::from(covered) / 1024.0
}

/// Holds the redacted page to the clause's own geometry outside the region, and asserts that
/// every pixel inside it is unmarked and that the original did mark it.
///
/// `inside` is the stroke §8.5.3.2 describes, stated from §8.4.3's parameters directly — the
/// set of points within half the line width of the path, closed off as the cap style says — so
/// the reference is the clause and neither backend. Every pixel outside the region that the
/// redaction changed has to be within two steps of that reference, and no further from it than
/// the original page was: the redacted page may re-resolve an edge pixel towards the geometry,
/// and a mark that moved fails both.
fn held_to_the_geometry_outside(before: &[u8], after: &[u8], inside: &dyn Fn(f64, f64) -> bool) {
    let before = pixels(before);
    let after = pixels(after);
    let scale = 150.0_f32 / 72.0;
    let (left, right) = (60.0 * scale, 140.0 * scale);
    let (top, bottom) = ((200.0 - 80.0) * scale, (200.0 - 30.0) * scale);
    let stride = before.width as usize * 4;
    let mut marked = false;
    for y in 0..before.height {
        for x in 0..before.width {
            let at = y as usize * stride + x as usize * 4;
            let (fx, fy) = (x as f32, y as f32);
            if fx > left + 1.0 && fx < right - 1.0 && fy > top + 1.0 && fy < bottom - 1.0 {
                marked |= before.data[at] != 0xFF;
                assert_eq!(
                    &after.data[at..at + 3],
                    &[0xFF, 0xFF, 0xFF],
                    "a pixel at ({x}, {y}) inside the region is still marked"
                );
            } else if (fx < left - 1.0 || fx > right + 1.0 || fy < top - 1.0 || fy > bottom + 1.0)
                && after.data[at..at + 4] != before.data[at..at + 4]
            {
                let exact = exact_grey(x, y, inside);
                let drawn = f64::from(after.data[at]);
                let original = f64::from(before.data[at]);
                assert!(
                    (drawn - exact).abs() <= 2.0
                        && (drawn - exact).abs() <= (original - exact).abs() + 1.0,
                    "a pixel at ({x}, {y}) outside the region moved from {original} to {drawn}, \
                     where the stroke's own geometry covers it to {exact:.1}"
                );
            }
        }
    }
    assert!(
        marked,
        "the original marks the region, so the test can fail"
    );
}

/// What a stroke marks, as a predicate on a point of default user space.
type Marks<'a> = &'a dyn Fn(f64, f64) -> bool;

/// A stroke whose outline holds an arc is cut as that outline (§8.4.3.3, §8.4.3.4, §8.5.2.2):
/// a round cap, a round join and a stroked curve each lose their marks inside the region, and
/// the centre line that described the removed marks is gone from the file.
///
/// Outside the region the arc is written back as cubics within `redact::ARC_TOLERANCE` of it,
/// and the test holds those pixels to the stroke the clauses describe rather than to the
/// oracle's own expansion of it: §8.4.3.3's "semicircular arc with a diameter equal to the line
/// width" around the endpoint, and for the curve every point within half the width of §8.5.2.2's
/// polynomial, sampled finely enough that the chord departs from it by a thousandth of a pixel.
/// Where the two pages differ, it is the redacted one that is the nearer to that geometry.
#[test]
fn a_stroke_whose_outline_holds_an_arc_is_cut_at_the_region_edge() {
    // The curve's centre line, 400 chords of §8.5.2.2's cubic.
    let bezier = |t: f64| {
        let points = [(20.0, 40.0), (30.0, 75.0), (50.0, 75.0), (100.0, 45.0)];
        let u = 1.0 - t;
        let weights = [u * u * u, 3.0 * u * u * t, 3.0 * u * t * t, t * t * t];
        points
            .iter()
            .zip(weights)
            .fold((0.0, 0.0), |sum, (point, weight)| {
                (sum.0 + weight * point.0, sum.1 + weight * point.1)
            })
    };
    let chords: Vec<(f64, f64)> = (0..=400)
        .map(|step| bezier(f64::from(step) / 400.0))
        .collect();
    let round_cap =
        |x: f64, y: f64| distance_to_segment((x, y), (20.0, 50.0), (120.0, 50.0)) <= 3.0;
    let round_join = |x: f64, y: f64| {
        let first = ((20.0, 20.0), (60.0, 55.0));
        let second = ((60.0, 55.0), (20.0, 90.0));
        // Butt caps: a point is marked where it lies within half the width of a segment and
        // between the two perpendiculars through its ends, or within the join's disc.
        let beside = |(from, to): ((f64, f64), (f64, f64))| {
            let (dx, dy) = (to.0 - from.0, to.1 - from.1);
            let along = ((x - from.0) * dx + (y - from.1) * dy) / (dx * dx + dy * dy);
            (0.0..=1.0).contains(&along) && distance_to_segment((x, y), from, to) <= 3.0
        };
        beside(first) || beside(second) || (x - 60.0).hypot(y - 55.0) <= 3.0
    };
    let curve = |x: f64, y: f64| {
        chords
            .windows(2)
            .any(|chord| distance_to_segment((x, y), chord[0], chord[1]) <= 3.0)
    };
    let cases: [(&str, &str, &str, Marks); 3] = [
        (
            "a round cap",
            "0 0 0 RG 6 w 1 J 0 j 20 50 m 120 50 l S",
            "20 50 m 120 50 l S",
            &round_cap,
        ),
        (
            "a round join",
            "0 0 0 RG 6 w 0 J 1 j 20 20 m 60 55 l 20 90 l S",
            "20 20 m 60 55 l 20 90 l S",
            &round_join,
        ),
        (
            "a stroked curve",
            "0 0 0 RG 6 w 0 J 0 j 20 40 m 30 75 50 75 100 45 c S",
            "20 40 m 30 75 50 75 100 45 c S",
            &curve,
        ),
    ];
    for (what, content, centre_line, inside) in cases {
        let bytes = build(
            content,
            &["<< /Type /Annot /Subtype /Redact /Rect [60 30 140 80] >>"],
        );
        let (report, out) = redact(&bytes);
        assert!(report.refused.is_empty(), "{what}: {:?}", report.refused);
        let Some(Origin::Redacted { paths, .. }) =
            report.outputs.first().map(|output| output.origin.clone())
        else {
            panic!("{what}: a redacted origin");
        };
        assert_eq!(paths, 1, "{what}: one stroked path was cut");
        assert!(
            !contains(&out, centre_line.as_bytes()),
            "{what}: the centre line that described the removed marks is gone from the file"
        );
        no_mark_meets(&out, [60.0, 30.0, 140.0, 80.0]);
        held_to_the_geometry_outside(&bytes, &out, inside);
    }
}

/// §8.4.3.2's zero line width is refused: it "shall denote the thinnest line that can be
/// rendered at device resolution", which is a width in device pixels and not one an outline in
/// user space can be written from.
#[test]
fn a_zero_width_stroke_in_the_region_refuses_the_page() {
    let content = "0 0 0 RG 0 w 0 J 20 50 m 120 50 l S";
    let bytes = build(
        content,
        &["<< /Type /Annot /Subtype /Redact /Rect [60 30 140 80] >>"],
    );
    let (report, _out) = redact(&bytes);
    assert_eq!(report.refused.len(), 1, "{:?}", report.refused);
    assert!(
        report.refused[0].detail.contains("§8.4.3.2"),
        "the refusal names the clause: {}",
        report.refused[0].detail
    );
}

/// The surviving outline is painted in the **stroking** colour, replayed under Table 74's
/// non-stroking operator with the producer's own operand bytes.
#[test]
fn the_surviving_outline_is_filled_in_the_stroking_colour() {
    let content = "1 0 0 rg 0 0 1 RG 2 w 0 J 0 j 20 50 m 120 50 l S";
    let bytes = build(
        content,
        &["<< /Type /Annot /Subtype /Redact /Rect [60 30 140 80] >>"],
    );
    let (report, out) = redact(&bytes);
    assert!(report.refused.is_empty(), "{:?}", report.refused);
    assert!(
        contains(&out, b"0 0 1 rg"),
        "the stroking colour is replayed as the non-stroking one"
    );
    // And it is balanced, so the fill colour the producer set is what comes back afterwards.
    assert!(contains(&out, b"q\n0 0 1 rg"), "the replay is inside a q");

    // In pixels: what survives is blue, which is the stroke's colour and not the fill's.
    let after = pixels(&out);
    // 150 dpi over 72 units to the inch, and y measured down from the top of a 200-unit page
    // (trap 12a). A point in the surviving half of the bar, left of the region.
    let stride = after.width as usize * 4;
    let x = 40 * 150 / 72;
    let y = (200 - 50) * 150 / 72;
    let at = y * stride + x * 4;
    assert_eq!(
        &after.data[at..at + 3],
        &[0x00, 0x00, 0xFF],
        "the survivor is the stroke's blue"
    );
}

/// §8.4.3.6's dash pattern is applied **before** the outline is cut: a dashed stroke marks the
/// on-stretches alone, so the outline that is cut is the dashes' and each is its own contour.
#[test]
fn a_dashed_stroke_is_dashed_before_the_outline_is_cut() {
    let content = "0 0 0 RG 2 w 0 J 0 j [10 10] 0 d 20 50 m 120 50 l S";
    let bytes = build(
        content,
        &["<< /Type /Annot /Subtype /Redact /Rect [60 30 140 80] >>"],
    );
    let (report, out) = redact(&bytes);
    assert!(report.refused.is_empty(), "{:?}", report.refused);
    no_mark_meets(&out, [60.0, 30.0, 140.0, 80.0]);

    // Two dashes lie clear of the region — 20..30 and 40..50 — so the cut outline is two
    // contours, and a solid stroke would have been one.
    let document = Document::open_with_limits(out.clone(), Limits::DEFAULT).expect("it opens");
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    let extents = mark_extents(
        pdf_model::interpret(&document, &page)
            .display_list
            .commands(),
    );
    assert_eq!(extents.len(), 1, "one painted mark: {extents:?}");
    let content = String::from_utf8_lossy(&page.content(&document)).to_string();
    assert_eq!(
        content.matches(" m\n").count(),
        2,
        "two dashes survive as two contours: {content}"
    );
}

/// A path with a §8.5.2.2 Bézier segment is cut at the parameters where the curve meets the
/// region's edges, which is a root rather than a sample: the surviving piece is still a curve,
/// and it is the producer's own curve restricted to a sub-interval of itself.
#[test]
fn a_curved_path_is_cut_at_the_roots_where_it_meets_the_region() {
    let content = "0 0 0 rg 20 40 m 60 90 100 90 120 40 c h f";
    let bytes = build(
        content,
        &["<< /Type /Annot /Subtype /Redact /Rect [60 30 140 80] >>"],
    );
    let (report, out) = redact(&bytes);
    assert!(report.refused.is_empty(), "{:?}", report.refused);
    assert!(
        !contains(&out, b"60 90 100 90 120 40 c"),
        "the control points that described the removed marks are gone from the file"
    );
    no_mark_meets(&out, [60.0, 30.0, 140.0, 80.0]);

    let document = Document::open_with_limits(out.clone(), Limits::DEFAULT).expect("it opens");
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    let content = String::from_utf8_lossy(&page.content(&document)).to_string();
    assert!(
        content.contains(" c\n"),
        "what survives is still a curve, not a chord: {content}"
    );
}

/// The curved cut proved in pixels: inside the region nothing is drawn, and outside it no mark
/// has moved.
///
/// **What "no mark has moved" is asserted as, and why it is not byte equality.** The cut writes
/// back the producer's own curve restricted to a sub-interval of itself, which is the same curve
/// — but the rasteriser flattens a cubic adaptively, and its subdivision of a sub-curve is not
/// the same subdivision it gave that stretch of the whole curve. So a pixel *on the curve's own
/// edge* can resolve one eight-bit step differently. The assertion is therefore the property
/// that discriminates a moved mark from a re-subdivided one: a pixel the original painted fully,
/// or left untouched, is identical, and a pixel that differs had **partial coverage** in the
/// original and differs by at most one step. A mark that actually moved would turn a white pixel
/// grey or a black one lighter, and both of those fail here. The stroke's cut is byte-identical
/// outside the region, because its outline is made of straight lines and has nothing to flatten.
#[test]
fn no_mark_moves_outside_the_region_when_a_curve_is_cut() {
    let content = "0 0 0 rg 20 40 m 60 90 100 90 120 40 c h f";
    let bytes = build(
        content,
        &["<< /Type /Annot /Subtype /Redact /Rect [60 30 140 80] >>"],
    );
    let (report, out) = redact(&bytes);
    assert!(report.refused.is_empty(), "{:?}", report.refused);

    let before = pixels(&bytes);
    let after = pixels(&out);
    let scale = 150.0_f32 / 72.0;
    let (left, right) = (60.0 * scale, 140.0 * scale);
    let (top, bottom) = ((200.0 - 80.0) * scale, (200.0 - 30.0) * scale);
    let stride = before.width as usize * 4;
    let mut marked = false;
    let mut differing = 0usize;
    for y in 0..before.height {
        for x in 0..before.width {
            let at = y as usize * stride + x as usize * 4;
            let (fx, fy) = (x as f32, y as f32);
            if fx > left + 1.0 && fx < right - 1.0 && fy > top + 1.0 && fy < bottom - 1.0 {
                marked |= before.data[at] != 0xFF;
                assert_eq!(
                    &after.data[at..at + 3],
                    &[0xFF, 0xFF, 0xFF],
                    "a pixel at ({x}, {y}) inside the region is still marked"
                );
            } else if fx < left - 1.0 || fx > right + 1.0 || fy < top - 1.0 || fy > bottom + 1.0 {
                for channel in 0..4 {
                    let delta =
                        i32::from(after.data[at + channel]) - i32::from(before.data[at + channel]);
                    if delta == 0 {
                        continue;
                    }
                    differing += 1;
                    assert!(
                        delta.abs() <= 1,
                        "a pixel at ({x}, {y}) outside the region moved by {delta}"
                    );
                    let coverage = before.data[at];
                    assert!(
                        coverage > 0 && coverage < 0xFF,
                        "a pixel at ({x}, {y}) the original painted whole changed"
                    );
                }
            }
        }
    }
    assert!(
        differing > 0,
        "the two rasters are not identical, which is what the bound above is about"
    );
    assert!(
        marked,
        "the original curve marks the region, so the test can fail"
    );
}

/// A path that is also §8.5.4's clipping boundary is refused: cutting its geometry would move
/// the boundary every mark after the painting operator is held to, which is content the
/// annotation did not identify.
/// A path that is also §8.5.4's clipping boundary keeps the boundary and loses the marks.
///
/// The clause separates the two acts in time, which is what lets a cut take one without the
/// other:
///
/// > Although the clipping path operator appears before the painting operator, it shall not
/// > alter the clipping path at the point where it appears. Rather, it shall modify the effect
/// > of the succeeding painting operator. After the path has been painted, the clipping path in
/// > the graphics state shall be set to the intersection of the current clipping path and the
/// > newly constructed path.
///
/// So the marks are painted under the clip already in force, and the boundary is set afterwards
/// from the path the producer constructed. The removal writes exactly that back — the cut marks,
/// then the producer's own bytes through the clipping operator, then `n`, which "shall cause no
/// marks to be placed on the page, but can be used with a clipping path operator to establish a
/// new clipping path". The fixture proves both halves at once: a black bar is clipped to itself
/// and then a blue fill covers the whole page, so the blue can only stay inside `20 40 100 20`
/// if the boundary survived, and the black can only leave the region if its marks were cut.
#[test]
fn a_clipping_path_keeps_its_boundary_while_its_marks_are_cut() {
    let content = "0 0 0 rg 20 40 100 20 re W f\n0 0 1 rg 0 0 200 200 re f";
    let bytes = build(
        content,
        &["<< /Type /Annot /Subtype /Redact /Rect [60 30 140 80] >>"],
    );
    let (report, out) = redact(&bytes);
    assert!(report.refused.is_empty(), "{:?}", report.refused);
    // In pixels, because the clip is not a mark and a bounding box cannot see it. The blue fill
    // covers the whole page and may only show through the boundary the black bar established, so
    // a point well outside that boundary is the page's own white either way; a point inside it
    // and outside the region is blue; and a point inside both is nothing at all, which is the
    // marks having been cut.
    let raster = pixels(&out);
    let at = |x: u32, y: u32| page_pixel(&raster, x, y);
    assert_eq!(
        at(160, 150),
        [255, 255, 255],
        "the clip still keeps the blue fill off the rest of the page"
    );
    assert_eq!(
        at(30, 50),
        [0, 0, 255],
        "and still lets it through where the producer's path put it"
    );
    assert_eq!(
        at(80, 50),
        [255, 255, 255],
        "inside the region both paths lost their marks"
    );
}

/// A clipping operator before the last construction still bounds the whole path.
///
/// §8.5.4 only *permits* the tidy order — the operator "may appear after the last path
/// construction operator and before the path-painting operator that terminates a path object" —
/// and what it bounds is "the newly constructed path", which is whole at the painting operator
/// and not before it. So the boundary's bytes are taken there, and a second rectangle built
/// after the `W` is part of the boundary exactly as the clause says. Without that, the second
/// rectangle would be missing from the re-stated boundary and the blue fill would not reach it.
#[test]
fn a_clipping_operator_before_the_last_construction_still_bounds_the_whole_path() {
    let content = "0 0 0 rg 20 40 100 20 re W 140 40 20 20 re f\n0 0 1 rg 0 0 200 200 re f";
    let bytes = build(
        content,
        &["<< /Type /Annot /Subtype /Redact /Rect [60 30 140 80] >>"],
    );
    let (report, out) = redact(&bytes);
    assert!(report.refused.is_empty(), "{:?}", report.refused);

    let raster = pixels(&out);
    let at = |x: u32, y: u32| page_pixel(&raster, x, y);
    assert_eq!(
        at(30, 50),
        [0, 0, 255],
        "the part of the boundary built before the clipping operator is in it"
    );
    assert_eq!(
        at(150, 50),
        [0, 0, 255],
        "and so is the part built after it"
    );
    assert_eq!(
        at(180, 150),
        [255, 255, 255],
        "the boundary is still a boundary"
    );
    assert_eq!(
        at(80, 50),
        [255, 255, 255],
        "and the marks inside the region are gone"
    );
}

/// A document with no `/Redact` annotation is written unchanged, and its text re-extracts.
#[test]
fn a_document_without_redactions_is_written_and_unchanged() {
    let content = "BT /F1 12 Tf 20 100 Td (HELLO) Tj ET";
    let bytes = build(content, &[]);
    let (report, out) = redact(&bytes);
    assert!(report.refused.is_empty());
    assert!(report.departures.is_empty());
    assert_eq!(page_text(&out).trim(), "HELLO");
    let Some(Origin::Redacted { glyphs, .. }) =
        report.outputs.first().map(|output| output.origin.clone())
    else {
        panic!("a redacted origin");
    };
    assert_eq!(glyphs, 0, "nothing was removed");
}

/// The census: a `/Redact` planted inside an object stream (§7.5.7) is invisible to a byte-grep
/// and must be seen by the parsed census — and applied.
#[test]
fn a_redaction_hidden_in_an_object_stream_is_found_and_applied() {
    let bytes = object_stream_fixture();
    assert!(
        !contains(&bytes, b"/Redact"),
        "the planted annotation is not visible to a byte-grep"
    );
    // The parsed reader sees it: it resolves the annotation and its region.
    let document = Document::open_with_limits(bytes.clone(), Limits::DEFAULT).expect("opens");
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    let annots = document.get_key(&page.dict, "Annots");
    let has_redact = annots.as_array().is_some_and(|items| {
        items.iter().any(|item| {
            document
                .resolve(item)
                .as_dict()
                .and_then(|dict| document.get_key(dict, "Subtype").as_name().cloned())
                .is_some_and(|name| name.as_bytes() == b"Redact")
        })
    });
    assert!(
        has_redact,
        "the census resolves the object-stream-hidden /Redact"
    );

    let (report, out) = redact(&bytes);
    assert!(
        report.refused.is_empty(),
        "it applies: {:?}",
        report.refused
    );
    assert!(
        !contains(&out, b"SECRET"),
        "and the hidden redaction removes the content"
    );
}

/// A §7.5.7 file whose `/Redact` annotation lives in an object stream, so the annotation is
/// compressed out of a byte-grep's sight.
///
/// Built by writing a plain fixture and running it through this suite's own `optimize` verb with
/// [`ObjectStreams::DEFAULT`], which is §7.5.8's cross-reference stream and §7.5.7's object
/// streams — a real object-stream file rather than one this test hand-rolls.
fn object_stream_fixture() -> Vec<u8> {
    let content = "BT /F1 12 Tf 20 50 Td (SECRET) Tj ET";
    let plain = build(
        content,
        &["<< /Type /Annot /Subtype /Redact /Rect [10 40 130 66] \
           /QuadPoints [10 66 130 66 10 40 130 40] >>"],
    );
    let sinks = MemorySinks::new();
    apply(
        &Plan::Optimize(OptimizePlan {
            source: 0,
            names: "objstm.pdf".parse().expect("a pattern"),
            prune: true,
            object_streams: ObjectStreams::DEFAULT,
            streams: Streams::DEFAULT,
            linearize: false,
        }),
        &[Source::new(plain)],
        &sinks,
        &Policy::default(),
        &Budget::default(),
    )
    .expect("the object-stream rewrite applies");
    sinks.into_outputs().remove(0).1
}

/// The Isartor `6.5.2` witness — a `-fail-` PDF/A-1b fixture carrying a `/Redact` inside an
/// object stream — opens, its region resolves, and `quorra-transform`'s verb runs on it.
#[test]
fn the_isartor_witness_opens_and_its_region_resolves() {
    let path = support::committed(
        "veraPDF-corpus/Isartor test files/PDFA-1b/6.5 Annotations/6.5.2 Annotation \
         types/isartor-6-5-2-t01-fail-h.pdf",
    );
    let Ok(bytes) = std::fs::read(&path) else {
        // The corpus submodule is not checked out in this tree; the census's other half stands.
        return;
    };
    let document =
        Document::open_with_limits(bytes.clone(), Limits::DEFAULT).expect("the witness opens");
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    let annots = document.get_key(&page.dict, "Annots");
    let region_resolves = annots.as_array().is_some_and(|items| {
        items.iter().any(|item| {
            let resolved = document.resolve(item);
            let Some(dict) = resolved.as_dict() else {
                return false;
            };
            document
                .get_key(dict, "Subtype")
                .as_name()
                .is_some_and(|name| name.as_bytes() == b"Redact")
                && !matches!(document.get_key(dict, "Rect"), Object::Null)
        })
    });
    assert!(region_resolves, "the witness's /Redact region resolves");

    // End to end through `apply`: it either applies or refuses by name, never panics or leaks.
    let (report, _out) = redact(&bytes);
    assert!(
        report.outputs.len() == 1,
        "the verb wrote a document for the witness"
    );
}

/// Assembles a §7.5.4 cross-referenced file from byte objects, numbered from one — the binary
/// counterpart of [`assemble`], for a fixture whose image stream is not valid UTF-8.
fn assemble_bytes(objects: &[Vec<u8>]) -> Vec<u8> {
    let mut out = Vec::from(&b"%PDF-1.7\n"[..]);
    let mut offsets = Vec::new();
    for (index, object) in objects.iter().enumerate() {
        offsets.push(out.len());
        out.extend_from_slice(format!("{} 0 obj ", index + 1).as_bytes());
        out.extend_from_slice(object);
        out.extend_from_slice(b" endobj\n");
    }
    let at = out.len();
    let size = objects.len() + 1;
    out.extend_from_slice(format!("xref\n0 {size}\n0000000000 65535 f \n").as_bytes());
    for offset in &offsets {
        out.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    out.extend_from_slice(
        format!("trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{at}\n%%EOF\n").as_bytes(),
    );
    out
}

/// An 8×8 `DeviceGray` image whose sample `(col, row)` is `col*8 + row + 1` — every sample
/// distinct and non-zero, so a cleared sample (zero) is unmistakable and a mixed-up row would be
/// caught.
fn distinct_samples() -> Vec<u8> {
    let mut samples = Vec::with_capacity(64);
    for row in 0..8u8 {
        for col in 0..8u8 {
            samples.push(col * 8 + row + 1);
        }
    }
    samples
}

/// The one image `XObject` as a byte object, `/Filter` and data given.
fn image_object(filter: &str, data: &[u8]) -> Vec<u8> {
    let mut object = format!(
        "<< /Type /XObject /Subtype /Image /Width 8 /Height 8 /ColorSpace /DeviceGray \
         /BitsPerComponent 8 /Filter /{filter} /Length {} >>\nstream\n",
        data.len()
    )
    .into_bytes();
    object.extend_from_slice(data);
    object.extend_from_slice(b"\nendstream");
    object
}

/// The output's one image `XObject` decoded back to its packed samples.
fn read_back_samples(bytes: &[u8]) -> Vec<u8> {
    read_back_samples_on(bytes, 0).0
}

/// The same, on the given page, also answering which object the page's `/Im1` names — the second
/// half of what a shared image's copy has to prove.
fn read_back_samples_on(bytes: &[u8], index: usize) -> (Vec<u8>, Option<pdf_syntax::ObjectId>) {
    let document = Document::open_with_limits(bytes.to_vec(), Limits::DEFAULT).expect("it opens");
    let page = pdf_model::Pages::new(&document)
        .get(index)
        .expect("the page");
    let xobjects = document.get_key(&page.resources, "XObject");
    let entry = xobjects
        .as_dict()
        .and_then(|dict| dict.get("Im1"))
        .cloned()
        .expect("/Im1 in the resources");
    let image = document.resolve(&entry);
    let stream = image.as_stream().expect("the image is a stream");
    // Re-interpret the page too: the destroyed image must not stop it drawing (it renders).
    let _ = pdf_model::interpret(&document, &page);
    let samples = document
        .image_stream(stream)
        .expect("the image decodes to samples")
        .data
        .to_vec();
    (samples, entry.as_reference())
}

/// The calibration trap 13 asks for on the image case (§12.5.6.23, "that portion of the image
/// data shall be destroyed"): an 8×8 image is placed over the page's [50,150]² square, and a
/// `/QuadPoints` region covers its left half. After application the left four columns are zero in
/// the decoded image and unrecoverable, the right four columns are byte-identical, and the file
/// re-opens and its image decodes.
#[test]
fn image_samples_inside_a_quadpoints_region_are_destroyed_and_outside_intact() {
    let samples = distinct_samples();
    let encoded = flate_encode(&samples, 6).expect("the fixture image deflates");
    let objects = vec![
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] /Resources << /XObject << /Im1 6 \
          0 R >> >> /Contents 4 0 R /Annots [5 0 R] >>"
            .to_vec(),
        b"<< /Length 33 >>\nstream\nq 100 0 0 100 50 50 cm /Im1 Do Q\nendstream".to_vec(),
        b"<< /Type /Annot /Subtype /Redact /Rect [50 50 100 150] \
          /QuadPoints [50 150 100 150 100 50 50 50] >>"
            .to_vec(),
        image_object("FlateDecode", &encoded),
    ];
    let bytes = assemble_bytes(&objects);

    let (report, out) = redact(&bytes);
    assert!(
        report.refused.is_empty(),
        "the image is cleared, not refused: {:?}",
        report.refused
    );
    // Principle 1: the original stream bytes are gone from the file, not left as an orphan the
    // new one sits beside — the destruction is a destruction.
    assert!(
        !contains(&out, &encoded),
        "the original image stream is not left in the file"
    );

    let out_samples = read_back_samples(&out);
    assert_eq!(out_samples.len(), 64, "the grid survives");
    for row in 0..8usize {
        for col in 0..8usize {
            let got = out_samples[row * 8 + col];
            if col < 4 {
                assert_eq!(
                    got, 0,
                    "col {col} row {row} is in the region: its sample is destroyed"
                );
            } else {
                let want = u8::try_from(col * 8 + row + 1).expect("small");
                assert_eq!(
                    got, want,
                    "col {col} row {row} is outside the region: its sample is byte-identical"
                );
            }
        }
    }

    let Some(Origin::Redacted { images, glyphs, .. }) =
        report.outputs.first().map(|output| output.origin.clone())
    else {
        panic!("the report states a redacted origin");
    };
    assert_eq!(images, 1, "one image had samples destroyed");
    assert_eq!(glyphs, 0, "no text was in this fixture");
}

/// An 8×8 one-component JPEG 2000 codestream, every sample [`JPX_SAMPLE`].
///
/// A bare codestream — SOC, SIZ, COD, QCD, SOT, SOD, EOC — with no JP2 boxes. Its `SIZ` states
/// one component of eight unsigned bits and its `COD` the reversible 5/3 wavelet, so it is
/// lossless and the value comes back exactly. The same bytes `pdf-model`'s image-mask tests
/// carry, generated rather than written because a JPEG 2000 codestream cannot be written by hand
/// legibly:
///
/// ```sh
/// python3 -c "import numpy as np; np.full(64, 200, np.uint8).tofile('gray.raw')"
/// opj_compress -i gray.raw -o gray.j2k -F 8,8,1,8,u -n 1 -r 1
/// ```
const JPX_ONE_COMPONENT: &[u8] = &[
    0xff, 0x4f, 0xff, 0x51, 0x00, 0x29, 0x00, 0x00, 0x00, 0x00, 0x00, 0x08, 0x00, 0x00, 0x00, 0x08,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x08, 0x00, 0x00, 0x00, 0x08,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x07, 0x01, 0x01, 0xff, 0x52, 0x00,
    0x0c, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x04, 0x04, 0x00, 0x01, 0xff, 0x5c, 0x00, 0x04, 0x40,
    0x40, 0xff, 0x90, 0x00, 0x0a, 0x00, 0x00, 0x00, 0x00, 0x00, 0x23, 0x00, 0x01, 0xff, 0x93, 0xcf,
    0xb4, 0x48, 0x14, 0x00, 0x5c, 0xa3, 0x65, 0x5d, 0xb0, 0x00, 0x03, 0x09, 0x08, 0xd5, 0x0a, 0x18,
    0x48, 0x4b, 0xff, 0x7f, 0xff, 0xd9,
];

/// The one sample value [`JPX_ONE_COMPONENT`] carries, in every pixel.
const JPX_SAMPLE: u8 = 200;

/// The page, the region and the image object the three `JPXDecode` fixtures share.
fn jpx_page(dictionary_extra: &str, data: &[u8]) -> Vec<u8> {
    let mut image = format!(
        "<< /Type /XObject /Subtype /Image /Width 8 /Height 8 /ColorSpace /DeviceGray \
         /Filter /JPXDecode {dictionary_extra} /Length {} >>\nstream\n",
        data.len()
    )
    .into_bytes();
    image.extend_from_slice(data);
    image.extend_from_slice(b"\nendstream");
    assemble_bytes(&[
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] /Resources << /XObject << /Im1 6 \
          0 R >> >> /Contents 4 0 R /Annots [5 0 R] >>"
            .to_vec(),
        b"<< /Length 33 >>\nstream\nq 100 0 0 100 50 50 cm /Im1 Do Q\nendstream".to_vec(),
        b"<< /Type /Annot /Subtype /Redact /Rect [50 50 100 150] \
          /QuadPoints [50 150 100 150 100 50 50 50] >>"
            .to_vec(),
        image,
    ])
}

/// A `JPXDecode` image on its own grid is decoded, cleared and re-encoded as `FlateDecode`, its
/// own samples in its own colour space (ADR 1333).
///
/// §12.5.6.23 asks one thing of an image — "[i]f a portion of an image is contained in a
/// redaction region, that portion of the image data shall be destroyed; clipping or image masks
/// shall not be used to hide that data" — and states nothing about the encoding the rest of it
/// survives in. Decoding the codestream, zeroing the region's samples and writing the whole grid
/// back under `FlateDecode` is therefore the destruction the clause asks for: the removed samples
/// are in the output in no form at all, and the ones outside the region are the values a reader
/// decoded before, carried losslessly. The assertions are both halves of that — the left four
/// columns zero, the right four still [`JPX_SAMPLE`], and the original codestream gone from the
/// file so there is no orphan to recover it from.
#[test]
fn a_jpx_image_in_the_region_is_decoded_cleared_and_reencoded_as_flate() {
    // In-process decode: the same routine the confined worker runs, with no worker binary needed.
    pdf_sandbox::set_isolation(pdf_sandbox::Isolation::InProcess);
    let bytes = jpx_page("", JPX_ONE_COMPONENT);

    let (report, out) = redact(&bytes);
    assert!(
        report.refused.is_empty(),
        "an eight-bit codestream on its own grid is cleared, not refused: {:?}",
        report.refused
    );
    assert!(
        !contains(&out, JPX_ONE_COMPONENT),
        "the original codestream is not left in the file"
    );

    let (width, height, rgba, _) = read_back_codec_image(&out);
    assert_eq!((width, height), (8, 8), "the grid survives the re-encode");
    assert_eq!(
        image_entries(&out),
        ("FlateDecode".to_owned(), "DeviceGray".to_owned(), 8),
        "the samples are written as themselves: the stated space, eight bits, no codec"
    );
    let samples = read_back_samples(&out);
    for row in 0..8usize {
        for col in 0..8usize {
            let red = rgba[(row * 8 + col) * 4];
            let own = samples[row * 8 + col];
            if col < 4 {
                assert_eq!((red, own), (0, 0), "destroyed at row {row}, column {col}");
            } else {
                assert_eq!(
                    (red, own),
                    (JPX_SAMPLE, JPX_SAMPLE),
                    "intact at row {row}, column {col}"
                );
            }
        }
    }
}

/// The filter, the colour space's name and the depth the output's `/Im1` states.
fn image_entries(bytes: &[u8]) -> (String, String, i64) {
    let document = Document::open_with_limits(bytes.to_vec(), Limits::DEFAULT).expect("it opens");
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    let entry = document
        .get_key(&page.resources, "XObject")
        .as_dict()
        .and_then(|dict| dict.get("Im1"))
        .cloned()
        .expect("/Im1 in the resources");
    let image = document.resolve(&entry);
    let stream = image.as_stream().expect("the image is a stream");
    let name = |key: &str| {
        let value = document.get_key(&stream.dict, key);
        match &value {
            Object::Name(name) => String::from_utf8_lossy(name.as_bytes()).into_owned(),
            Object::Array(items) => items
                .first()
                .and_then(Object::as_name)
                .map(|name| String::from_utf8_lossy(name.as_bytes()).into_owned())
                .unwrap_or_default(),
            _ => String::new(),
        }
    };
    let bits = document
        .get_key(&stream.dict, "BitsPerComponent")
        .as_integer()
        .unwrap_or(0);
    (name("Filter"), name("ColorSpace"), bits)
}

/// The numbers of the output's `/Im1` entry `key`, or `None` where it is not an array of them.
fn image_numbers(bytes: &[u8], key: &str) -> Option<Vec<f64>> {
    let document = Document::open_with_limits(bytes.to_vec(), Limits::DEFAULT).expect("it opens");
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    let entry = document
        .get_key(&page.resources, "XObject")
        .as_dict()
        .and_then(|dict| dict.get("Im1"))
        .cloned()
        .expect("/Im1 in the resources");
    let image = document.resolve(&entry);
    let stream = image.as_stream().expect("the image is a stream");
    let value = document.get_key(&stream.dict, key);
    value.as_array().map(|items| {
        items
            .iter()
            .filter_map(|item| document.resolve(item).as_number())
            .collect()
    })
}

/// An 8×8 one-component JPEG 2000 codestream of **sixteen** unsigned bits, sample `(r, c)` equal
/// to [`sixteen_bit_sample`] — every one distinct, and each differing from its neighbours below
/// the eighth bit, so a re-encode that narrowed them to a byte could not keep them apart. Lossless
/// 5/3; the raw samples are big-endian, which is what `opj_compress` reads:
///
/// ```sh
/// python3 -c "import struct; open('g16.raw','wb').write(b''.join(struct.pack('>H', \
///   0x8000 + 2000*r + 13*c) for r in range(8) for c in range(8)))"
/// opj_compress -i g16.raw -o g16.j2k -F 8,8,1,16,u -n 1 -r 1
/// ```
const JPX_SIXTEEN_BIT_RAMP: &[u8] = &[
    0xff, 0x4f, 0xff, 0x51, 0x00, 0x29, 0x00, 0x00, 0x00, 0x00, 0x00, 0x08, 0x00, 0x00, 0x00, 0x08,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x08, 0x00, 0x00, 0x00, 0x08,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x0f, 0x01, 0x01, 0xff, 0x52, 0x00,
    0x0c, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x04, 0x04, 0x00, 0x01, 0xff, 0x5c, 0x00, 0x04, 0x40,
    0x80, 0xff, 0x64, 0x00, 0x25, 0x00, 0x01, 0x43, 0x72, 0x65, 0x61, 0x74, 0x65, 0x64, 0x20, 0x62,
    0x79, 0x20, 0x4f, 0x70, 0x65, 0x6e, 0x4a, 0x50, 0x45, 0x47, 0x20, 0x76, 0x65, 0x72, 0x73, 0x69,
    0x6f, 0x6e, 0x20, 0x32, 0x2e, 0x35, 0x2e, 0x34, 0xff, 0x90, 0x00, 0x0a, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x76, 0x00, 0x01, 0xff, 0x93, 0xc7, 0xfe, 0x0c, 0xc8, 0x3a, 0x62, 0x4b, 0x03, 0xf3, 0x01,
    0xbc, 0xc5, 0x89, 0xc7, 0x35, 0xbe, 0x4c, 0xdc, 0xb0, 0x20, 0x1d, 0x06, 0xfc, 0xd2, 0xd2, 0x37,
    0x92, 0xfb, 0xb0, 0xb3, 0xdc, 0x50, 0xfe, 0x4d, 0x93, 0x1c, 0x38, 0xb5, 0x76, 0xf7, 0xee, 0x71,
    0x5f, 0x0b, 0x8c, 0xa0, 0xee, 0x27, 0x71, 0xbe, 0x3b, 0xbf, 0xa8, 0x92, 0x75, 0x92, 0xd5, 0x1c,
    0x92, 0x2b, 0x54, 0x8d, 0x53, 0x4a, 0xd1, 0x97, 0xfc, 0xe2, 0xba, 0xed, 0x0b, 0x29, 0xaa, 0x67,
    0x50, 0x4a, 0x05, 0x60, 0x1b, 0x86, 0xf5, 0x63, 0x6d, 0x7b, 0x8b, 0x5c, 0x27, 0x5b, 0xeb, 0x69,
    0x75, 0x6d, 0x23, 0x59, 0x7f, 0x37, 0xcd, 0xf3, 0x7c, 0xdf, 0x37, 0xcd, 0xff, 0x7f, 0xff, 0xd9,
];

/// The sample [`JPX_SIXTEEN_BIT_RAMP`] carries at row `row`, column `col`.
fn sixteen_bit_sample(row: usize, col: usize) -> u16 {
    u16::try_from(0x8000 + 2000 * row + 13 * col).expect("under 65536")
}

/// The same recipe at **twelve** bits, sample `(r, c)` equal to [`twelve_bit_sample`]:
///
/// ```sh
/// python3 -c "import struct; open('g12.raw','wb').write(b''.join(struct.pack('>H', \
///   1000 + 37*r + 400*c) for r in range(8) for c in range(8)))"
/// opj_compress -i g12.raw -o g12.j2k -F 8,8,1,12,u -n 1 -r 1
/// ```
const JPX_TWELVE_BIT_RAMP: &[u8] = &[
    0xff, 0x4f, 0xff, 0x51, 0x00, 0x29, 0x00, 0x00, 0x00, 0x00, 0x00, 0x08, 0x00, 0x00, 0x00, 0x08,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x08, 0x00, 0x00, 0x00, 0x08,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x0b, 0x01, 0x01, 0xff, 0x52, 0x00,
    0x0c, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x04, 0x04, 0x00, 0x01, 0xff, 0x5c, 0x00, 0x04, 0x40,
    0x60, 0xff, 0x64, 0x00, 0x25, 0x00, 0x01, 0x43, 0x72, 0x65, 0x61, 0x74, 0x65, 0x64, 0x20, 0x62,
    0x79, 0x20, 0x4f, 0x70, 0x65, 0x6e, 0x4a, 0x50, 0x45, 0x47, 0x20, 0x76, 0x65, 0x72, 0x73, 0x69,
    0x6f, 0x6e, 0x20, 0x32, 0x2e, 0x35, 0x2e, 0x34, 0xff, 0x90, 0x00, 0x0a, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x6a, 0x00, 0x01, 0xff, 0x93, 0xcf, 0xe5, 0x64, 0x12, 0x0a, 0xbe, 0x4f, 0xb1, 0x27, 0x4b,
    0x45, 0xe1, 0xe7, 0x6f, 0x80, 0xd8, 0xe1, 0xc4, 0x37, 0x47, 0xd1, 0xbe, 0xc2, 0xb3, 0x4f, 0x72,
    0xcd, 0x90, 0xff, 0x75, 0xc8, 0x05, 0xba, 0xd7, 0xa7, 0x5a, 0x1f, 0x4b, 0x34, 0xd5, 0x5e, 0x50,
    0x17, 0xef, 0x6c, 0xcb, 0xa1, 0x0e, 0x32, 0xb4, 0x7a, 0x59, 0x0b, 0x6c, 0x2c, 0x7e, 0xed, 0x06,
    0xb4, 0xf7, 0x29, 0x3c, 0x62, 0x54, 0x41, 0x7d, 0x56, 0xa6, 0x76, 0x6f, 0x34, 0x4a, 0x7d, 0xe9,
    0x18, 0x08, 0x20, 0x4a, 0x77, 0x80, 0x46, 0x94, 0xcd, 0x53, 0x95, 0x50, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x03, 0xff, 0xd9,
];

/// The sample [`JPX_TWELVE_BIT_RAMP`] carries at row `row`, column `col`.
fn twelve_bit_sample(row: usize, col: usize) -> u16 {
    u16::try_from(1000 + 37 * row + 400 * col).expect("under 4096")
}

/// The output's `/Im1` samples as sixteen-bit big-endian integers.
fn read_back_wide_samples(bytes: &[u8]) -> Vec<u16> {
    read_back_samples(bytes)
        .chunks_exact(2)
        .map(|pair| u16::from_be_bytes([pair[0], pair[1]]))
        .collect()
}

/// A sixteen-bit `JPXDecode` image is cleared at its own precision (ADR 1333).
///
/// Table 87 leaves the depth to the processor — "[t]he bit depth is determined by the PDF processor
/// in the process of decoding the JPEG 2000 image" — and the decoder determines the codestream's,
/// so the samples outside the region are written back as the sixteen-bit integers they are: the
/// annotation identified the left half, and an eight-bit re-encode would have coarsened the right
/// half as well. The assertions are the written integers themselves, against the fixture's recipe.
#[test]
fn a_sixteen_bit_jpx_image_is_cleared_at_its_own_precision() {
    pdf_sandbox::set_isolation(pdf_sandbox::Isolation::InProcess);
    let bytes = jpx_page("", JPX_SIXTEEN_BIT_RAMP);

    let (report, out) = redact(&bytes);
    assert!(
        report.refused.is_empty(),
        "a sixteen-bit codestream is cleared, not refused: {:?}",
        report.refused
    );
    assert!(
        !contains(&out, JPX_SIXTEEN_BIT_RAMP),
        "the codestream is gone"
    );
    assert_eq!(
        image_entries(&out),
        ("FlateDecode".to_owned(), "DeviceGray".to_owned(), 16)
    );
    assert_eq!(
        image_numbers(&out, "Decode"),
        Some(vec![0.0, 1.0]),
        "sixteen bits read and written at sixteen: the pair is the space's default"
    );
    let samples = read_back_wide_samples(&out);
    for row in 0..8usize {
        for col in 0..8usize {
            let want = if col < 4 {
                0
            } else {
                sixteen_bit_sample(row, col)
            };
            assert_eq!(samples[row * 8 + col], want, "row {row}, column {col}");
        }
    }
}

/// A twelve-bit `JPXDecode` image keeps its integers at sixteen bits, with its `/Decode` restated.
///
/// A `FlateDecode` stream takes 1, 2, 4, 8 or 16 bits a component, so twelve-bit integers are
/// written in sixteen-bit fields unchanged, and §8.9.5.2's map — `D min + x × (D max − D min) ÷
/// (2^n − 1)` — is kept by moving the pair's far end to `4095 ÷ 4095 × 65535 ÷ 4095`: every
/// integer then means what it meant at twelve bits. The page drawn from the output is compared
/// against the page drawn from the source outside the region, which is the claim.
#[test]
fn a_twelve_bit_jpx_image_keeps_its_integers_under_a_widened_decode() {
    pdf_sandbox::set_isolation(pdf_sandbox::Isolation::InProcess);
    let bytes = jpx_page("", JPX_TWELVE_BIT_RAMP);
    let original = decode_fixture_rgba(&bytes);

    let (report, out) = redact(&bytes);
    assert!(report.refused.is_empty(), "cleared: {:?}", report.refused);
    assert_eq!(
        image_entries(&out),
        ("FlateDecode".to_owned(), "DeviceGray".to_owned(), 16)
    );
    let decode = image_numbers(&out, "Decode").expect("a /Decode is stated");
    assert_eq!(decode.len(), 2);
    assert!(
        decode[0] == 0.0 && (decode[1] - 65535.0 / 4095.0).abs() < 1e-4,
        "the far end carries twelve bits' map into sixteen: {decode:?}"
    );
    let samples = read_back_wide_samples(&out);
    let (_, _, rgba, _) = read_back_codec_image(&out);
    for row in 0..8usize {
        for col in 0..8usize {
            let at = row * 8 + col;
            if col < 4 {
                assert_eq!(samples[at], 0, "destroyed at row {row}, column {col}");
            } else {
                assert_eq!(
                    samples[at],
                    twelve_bit_sample(row, col),
                    "row {row}, column {col}"
                );
                assert_eq!(
                    &rgba[at * 4..at * 4 + 4],
                    &original[at * 4..at * 4 + 4],
                    "drawn as before at row {row}, column {col}"
                );
            }
        }
    }
}

/// A `JPXDecode` image whose dictionary states no colour space is written in the codestream's.
///
/// §7.4.9: where `/ColorSpace` is absent the codestream's own colour specification decides, and a
/// bare codestream that states none falls back on its channel count — one channel, `DeviceGray`. A
/// `FlateDecode` image has no codestream to ask, so the space the samples were read in is stated.
#[test]
fn a_jpx_image_stating_no_colour_space_is_written_in_the_codestreams_own() {
    pdf_sandbox::set_isolation(pdf_sandbox::Isolation::InProcess);
    let mut image = format!(
        "<< /Type /XObject /Subtype /Image /Width 8 /Height 8 /Filter /JPXDecode /Length {} \
         >>\nstream\n",
        JPX_SIXTEEN_BIT_RAMP.len()
    )
    .into_bytes();
    image.extend_from_slice(JPX_SIXTEEN_BIT_RAMP);
    image.extend_from_slice(b"\nendstream");
    let bytes = masked_page(image, b"null".to_vec());

    let (report, out) = redact(&bytes);
    assert!(report.refused.is_empty(), "cleared: {:?}", report.refused);
    assert_eq!(
        image_entries(&out),
        ("FlateDecode".to_owned(), "DeviceGray".to_owned(), 16)
    );
    assert_eq!(read_back_wide_samples(&out)[7], sixteen_bit_sample(0, 7));
}

/// A `JPXDecode` image larger than the budget is refused by name rather than reduced (ADR 1333).
///
/// §7.4.9 NOTE 3 lets a viewer "select and decode only the data making up a lower-resolution
/// version" of a codestream too large for it, and this tree's viewer does. A redaction cannot:
/// it writes the grid back, and a reduced one would resample every sample outside the region. So
/// the decode is asked for at full resolution within a stated number of samples, and a codestream
/// over it is refused naming the budget and the flag that raises it — `--image-samples`, the
/// operator's, since the memory and time a larger decode costs are the machine's. The fixture's
/// grid is 64 samples: a budget of 63 refuses it and a budget of 64 clears it, which puts the line
/// exactly at the grid.
#[test]
fn a_jpx_image_over_the_operators_budget_is_refused_rather_than_reduced() {
    pdf_sandbox::set_isolation(pdf_sandbox::Isolation::InProcess);
    let bytes = jpx_page("", JPX_SIXTEEN_BIT_RAMP);

    let (report, out) = redact_within(&bytes, Some(63));
    let refused = report
        .refused
        .iter()
        .find(|declined| declined.page == Some(1))
        .expect("the page is refused");
    assert!(
        refused.detail.contains("§7.4.9")
            && refused.detail.contains("beyond the 63")
            && refused.detail.contains("--image-samples"),
        "the refusal names the clause, the budget and the flag: {}",
        refused.detail
    );
    assert!(
        contains(&out, JPX_SIXTEEN_BIT_RAMP),
        "the refused page keeps its image"
    );

    let (report, out) = redact_within(&bytes, Some(64));
    assert!(
        report.refused.is_empty(),
        "within the budget: {:?}",
        report.refused
    );
    assert_eq!(read_back_wide_samples(&out)[0], 0, "and cleared");
}

/// A colour key on a `JPXDecode` image is carried, because its samples are (ADR 1333).
///
/// §8.9.6.4's integers are "colour values before decoding with the Decode array", and the samples
/// the output carries are the same integers in the same domain — so the ranges still pick out the
/// samples they were written against. The key names one twelve-bit sample on the right half,
/// which is masked before and after; a key the re-encode had carried into another domain would
/// mask something else or nothing.
#[test]
fn a_colour_key_on_a_jpx_image_is_carried_with_its_samples() {
    pdf_sandbox::set_isolation(pdf_sandbox::Isolation::InProcess);
    let keyed = twelve_bit_sample(2, 6);
    let bytes = jpx_page(&format!("/Mask [{keyed} {keyed}]"), JPX_TWELVE_BIT_RAMP);
    let original = decode_fixture_rgba(&bytes);
    let masked = (2 * 8 + 6) * 4 + 3;
    assert_eq!(original[masked], 0, "the fixture masks the sample it names");

    let (report, out) = redact(&bytes);
    assert!(report.refused.is_empty(), "cleared: {:?}", report.refused);
    assert_eq!(
        image_numbers(&out, "Mask"),
        Some(vec![f64::from(keyed), f64::from(keyed)]),
        "the ranges are carried as the file wrote them"
    );
    let (_, _, rgba, _) = read_back_codec_image(&out);
    for row in 0..8usize {
        for col in 4..8usize {
            let at = (row * 8 + col) * 4 + 3;
            assert_eq!(
                rgba[at], original[at],
                "the key's answer at row {row}, column {col}"
            );
        }
    }
}

/// An 8×8 JP2 file with one grey component of 200 and an opacity channel (a `cdef` box names it)
/// whose sample at row `r`, column `c` is `40 + 25c + r` — every value distinct, so a cleared
/// sample is unmistakable. Lossless 5/3, so the values come back exactly:
///
/// ```sh
/// python3 - <<'EOF'
/// from PIL import Image
/// im = Image.new('LA', (8, 8))
/// for y in range(8):
///     for x in range(8):
///         im.putpixel((x, y), (200, 40 + x * 25 + y))
/// im.save('ga.png')
/// EOF
/// opj_compress -i ga.png -o ga.jp2 -n 1
/// ```
const JPX_GREY_WITH_OPACITY: &[u8] = &[
    0x00, 0x00, 0x00, 0x0c, 0x6a, 0x50, 0x20, 0x20, 0x0d, 0x0a, 0x87, 0x0a, 0x00, 0x00, 0x00, 0x14,
    0x66, 0x74, 0x79, 0x70, 0x6a, 0x70, 0x32, 0x20, 0x00, 0x00, 0x00, 0x00, 0x6a, 0x70, 0x32, 0x20,
    0x00, 0x00, 0x00, 0x43, 0x6a, 0x70, 0x32, 0x68, 0x00, 0x00, 0x00, 0x16, 0x69, 0x68, 0x64, 0x72,
    0x00, 0x00, 0x00, 0x08, 0x00, 0x00, 0x00, 0x08, 0x00, 0x02, 0x07, 0x07, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x0f, 0x63, 0x6f, 0x6c, 0x72, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x11, 0x00, 0x00, 0x00,
    0x16, 0x63, 0x64, 0x65, 0x66, 0x00, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x01, 0x00,
    0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0xcc, 0x6a, 0x70, 0x32, 0x63, 0xff, 0x4f, 0xff, 0x51, 0x00,
    0x2c, 0x00, 0x00, 0x00, 0x00, 0x00, 0x08, 0x00, 0x00, 0x00, 0x08, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x08, 0x00, 0x00, 0x00, 0x08, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x02, 0x07, 0x01, 0x01, 0x07, 0x01, 0x01, 0xff, 0x52, 0x00, 0x0c, 0x00,
    0x00, 0x00, 0x01, 0x00, 0x00, 0x04, 0x04, 0x00, 0x01, 0xff, 0x5c, 0x00, 0x04, 0x40, 0x40, 0xff,
    0x64, 0x00, 0x25, 0x00, 0x01, 0x43, 0x72, 0x65, 0x61, 0x74, 0x65, 0x64, 0x20, 0x62, 0x79, 0x20,
    0x4f, 0x70, 0x65, 0x6e, 0x4a, 0x50, 0x45, 0x47, 0x20, 0x76, 0x65, 0x72, 0x73, 0x69, 0x6f, 0x6e,
    0x20, 0x32, 0x2e, 0x35, 0x2e, 0x34, 0xff, 0x90, 0x00, 0x0a, 0x00, 0x00, 0x00, 0x00, 0x00, 0x57,
    0x00, 0x01, 0xff, 0x93, 0xcf, 0xb4, 0x48, 0x14, 0x00, 0x5c, 0xa3, 0x65, 0x5d, 0xb0, 0x00, 0x03,
    0x09, 0x08, 0xd5, 0x0a, 0x18, 0x48, 0x4b, 0xff, 0x7f, 0xcf, 0xb4, 0xc4, 0x11, 0x64, 0xe3, 0x72,
    0x5c, 0x73, 0x0a, 0x36, 0x7d, 0xf5, 0x15, 0xaf, 0x90, 0x0f, 0x1e, 0x69, 0x46, 0x5a, 0x68, 0xf8,
    0x25, 0x20, 0x3b, 0xec, 0x40, 0x15, 0x6f, 0x13, 0xbd, 0x37, 0xfd, 0x3c, 0x3d, 0xbc, 0x6e, 0x4e,
    0xf3, 0xc1, 0x1c, 0x0e, 0xed, 0xb5, 0xda, 0xaa, 0xaa, 0xaa, 0xaa, 0xaa, 0xa7, 0xff, 0xd9,
];

/// The opacity sample [`JPX_GREY_WITH_OPACITY`] carries at `row`, `col`.
fn jpx_opacity(row: usize, col: usize) -> u8 {
    u8::try_from(40 + col * 25 + row).expect("under 256")
}

/// A `JPXDecode` image whose codestream carries its opacity has both cleared, the opacity written
/// as the soft-mask image Table 87 names.
///
/// Table 87's `/SMaskInData` 1: "[t]he image's data stream includes encoded soft -mask values. A
/// PDF processor shall create a soft-mask image from the information to be used as a source of
/// mask shape or mask opacity in the transparency imaging model." The opaque `FlateDecode`
/// re-encode has no channel for it, so the removal writes that soft-mask image as an image
/// dictionary's `/SMask` beside the re-encode — and because the opacity channel is image data,
/// §12.5.6.23's "that portion of the image data shall be destroyed" reaches it as well: the left
/// four columns are zero in both, the right four columns keep the grey and the opacity the
/// codestream carried. ADR 1277.
#[test]
fn a_jpx_image_whose_samples_carry_opacity_is_cleared_with_its_opacity_channel() {
    pdf_sandbox::set_isolation(pdf_sandbox::Isolation::InProcess);
    let bytes = jpx_page("/SMaskInData 1", JPX_GREY_WITH_OPACITY);

    let (report, out) = redact(&bytes);
    assert!(
        report.refused.is_empty(),
        "the opacity channel travels as a soft mask, so nothing is refused: {:?}",
        report.refused
    );
    assert!(
        !contains(&out, JPX_GREY_WITH_OPACITY),
        "the original codestream is not left in the file"
    );
    let (width, height, rgba, _) = read_back_codec_image(&out);
    assert_eq!((width, height), (8, 8), "the grid survives the re-encode");
    assert_eq!(
        image_entries(&out),
        ("FlateDecode".to_owned(), "DeviceGray".to_owned(), 8),
        "the picture's own grey samples under FlateDecode"
    );
    let mask = read_back_mask(&out, "SMask").expect("the picture names a soft mask");
    assert_eq!(
        mask.len(),
        64,
        "one opacity sample a pixel, on the picture's grid"
    );
    for row in 0..8usize {
        for col in 0..8usize {
            let at = (row * 8 + col) * 4;
            if col < 4 {
                assert_eq!(rgba[at], 0, "grey destroyed at row {row}, column {col}");
                assert_eq!(mask[row * 8 + col], 0, "opacity destroyed at {row}, {col}");
            } else {
                assert_eq!(
                    rgba[at], JPX_SAMPLE,
                    "grey intact at row {row}, column {col}"
                );
                assert_eq!(
                    mask[row * 8 + col],
                    jpx_opacity(row, col),
                    "opacity intact at row {row}, column {col}"
                );
                assert_eq!(
                    rgba[at + 3],
                    jpx_opacity(row, col),
                    "a reader sees it as alpha"
                );
            }
        }
    }
}

/// A `/SMaskInData` code Table 87 does not define refuses the page by name.
///
/// The table defines 0, 1 and 2 and nothing else, so what a third code says the samples carry is
/// not something this removal can read off the standard; it is refused rather than guessed.
#[test]
fn an_undefined_smaskindata_code_refuses_the_page() {
    pdf_sandbox::set_isolation(pdf_sandbox::Isolation::InProcess);
    let bytes = jpx_page("/SMaskInData 3", JPX_ONE_COMPONENT);

    let (report, _out) = redact(&bytes);
    let refused = report
        .refused
        .iter()
        .find(|declined| declined.page == Some(1))
        .expect("the page is refused");
    assert!(
        refused.detail.contains("/SMaskInData 3"),
        "the refusal names the code: {}",
        refused.detail
    );
}

/// Data that is not JPEG 2000 at all refuses the page rather than being re-encoded blind.
///
/// The precision gate reads the codestream's own statement, so data that states nothing is data
/// this removal cannot show its re-encode preserves. Trap 5: refused by name rather than cleared
/// on a guess.
#[test]
fn a_jpx_image_that_states_no_precision_refuses_the_page() {
    let bytes = jpx_page("", b"\x00\x00\x00\x0cjP  ");

    let (report, _out) = redact(&bytes);
    let refused = report
        .refused
        .iter()
        .find(|declined| declined.page == Some(1))
        .expect("the page is refused");
    assert!(
        refused.detail.contains("§7.4.9"),
        "the refusal names the clause the precision comes from: {}",
        refused.detail
    );
}

/// An image carrying §8.9.5.4 `/Alternates` loses the entry, and the variants with it.
///
/// §12.5.6.23 asks for every trace to go — "remove all traces of the specified content" — and
/// §8.9.5.4 makes an alternate "an array of alternate image dictionaries specifying variant
/// representations of the base image", reached from that entry and from nowhere else. So the
/// redacted page's copy of the image states no `/Alternates`, the variant is reached from
/// nothing, and the closure the writer copies never sees it: its bytes are not in the output at
/// all. The fixture gives the alternate samples of its own, so the assertion is about that
/// object rather than about the base's.
#[test]
fn an_image_carrying_alternates_loses_the_entry_and_its_variants() {
    let samples = distinct_samples();
    let encoded = flate_encode(&samples, 6).expect("the fixture image deflates");
    // A variant of the same picture, distinguishable byte for byte from the base.
    let variant: Vec<u8> = samples.iter().map(|sample| 255 - sample).collect();
    let variant = flate_encode(&variant, 6).expect("the fixture variant deflates");
    let mut base = image_object("FlateDecode", &encoded);
    // The `/Alternates` entry, spliced into the image dictionary before its `>>`.
    let at = base
        .windows(4)
        .position(|window| window == b">>\ns")
        .expect("the dictionary ends before the stream");
    base.splice(
        at..at,
        b"/Alternates [ << /Image 7 0 R >> ] ".iter().copied(),
    );
    let objects = vec![
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] /Resources << /XObject << /Im1 6 \
          0 R >> >> /Contents 4 0 R /Annots [5 0 R] >>"
            .to_vec(),
        b"<< /Length 33 >>\nstream\nq 100 0 0 100 50 50 cm /Im1 Do Q\nendstream".to_vec(),
        b"<< /Type /Annot /Subtype /Redact /Rect [50 50 100 150] \
          /QuadPoints [50 150 100 150 100 50 50 50] >>"
            .to_vec(),
        base,
        image_object("FlateDecode", &variant),
    ];
    let bytes = assemble_bytes(&objects);

    let (report, out) = redact(&bytes);
    assert!(report.refused.is_empty(), "{:?}", report.refused);
    assert!(
        !contains(&out, &variant),
        "the variant's samples are not in the output"
    );
    assert!(
        !contains(&out, b"/Alternates"),
        "and neither is the entry that reached them"
    );
    // The base is still the base, destroyed in the region: the left four columns are zero.
    let cleared = read_back_samples(&out);
    for row in 0..8usize {
        for col in 0..4usize {
            assert_eq!(
                cleared[row * 8 + col],
                0,
                "the region's samples are destroyed at row {row}, column {col}"
            );
        }
    }
}

/// The one image `XObject` as a byte object with a stated grid, colour space, bit depth, filter
/// and (optional) decode parameters — for the codec fixtures whose grid is not the 8×8 default.
fn codec_image_object(
    width: u32,
    height: u32,
    colour_space: &str,
    bits: u32,
    filter: &str,
    parms: &str,
    data: &[u8],
) -> Vec<u8> {
    let mut object = format!(
        "<< /Type /XObject /Subtype /Image /Width {width} /Height {height} /ColorSpace \
         /{colour_space} /BitsPerComponent {bits} /Filter /{filter}{parms} /Length {} >>\n\
         stream\n",
        data.len()
    )
    .into_bytes();
    object.extend_from_slice(data);
    object.extend_from_slice(b"\nendstream");
    object
}

/// The output's one image `XObject`, decoded to straight-alpha `RGBA8` the interpreter's own way,
/// with whether it is a `FlateDecode` `DeviceRGB` image (the codec re-encode's shape).
fn read_back_codec_image(bytes: &[u8]) -> (u32, u32, Vec<u8>, bool) {
    let document = Document::open_with_limits(bytes.to_vec(), Limits::DEFAULT).expect("it opens");
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    let entry = document
        .get_key(&page.resources, "XObject")
        .as_dict()
        .and_then(|dict| dict.get("Im1"))
        .cloned()
        .expect("/Im1 in the resources");
    let image = document.resolve(&entry);
    let stream = image.as_stream().expect("the image is a stream");
    let is_flate_rgb = {
        let filter = document.get_key(&stream.dict, "Filter");
        let space = document.get_key(&stream.dict, "ColorSpace");
        let flate = matches!(filter, Object::Name(name) if name.as_bytes() == b"FlateDecode");
        let rgb = matches!(space, Object::Name(name) if name.as_bytes() == b"DeviceRGB");
        flate && rgb
    };
    // The redacted page must still draw (its interpretation must not fault on the new image).
    let _ = pdf_model::interpret(&document, &page);
    let flattened = pdf_model::image::decode(
        &document,
        stream,
        &page.resources,
        Color::BLACK,
        &Conversion::device(),
    )
    .expect("the output image decodes");
    (
        flattened.image.width,
        flattened.image.height,
        flattened.image.data.to_vec(),
        is_flate_rgb,
    )
}

/// The calibration trap 13 asks for on the codec case: a `DCTDecode` image (§7.4.8) over the
/// page's [50,150]² square, a `/QuadPoints` region on its left half. After application the region
/// pixels are the zero constant in the re-read image, the rest byte-identical to the original
/// decode, and the output is a `FlateDecode` `DeviceRGB` image — no codec, so the cleared region
/// cannot round-trip back through the lossy filter that would leak it.
#[test]
fn a_dct_image_in_the_region_is_decoded_cleared_and_reencoded_as_flate() {
    let objects = vec![
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] /Resources << /XObject << /Im1 6 \
          0 R >> >> /Contents 4 0 R /Annots [5 0 R] >>"
            .to_vec(),
        b"<< /Length 33 >>\nstream\nq 100 0 0 100 50 50 cm /Im1 Do Q\nendstream".to_vec(),
        b"<< /Type /Annot /Subtype /Redact /Rect [50 50 100 150] \
          /QuadPoints [50 150 100 150 100 50 50 50] >>"
            .to_vec(),
        codec_image_object(16, 16, "DeviceRGB", 8, "DCTDecode", "", DCT_JPEG_16X16),
    ];
    let bytes = assemble_bytes(&objects);

    // The original image decoded the interpreter's way: what "the rest is intact" is measured
    // against. A JPEG decodes deterministically, so the output's untouched pixels are these.
    let original = decode_fixture_rgba(&bytes);

    let (report, out) = redact(&bytes);
    assert!(
        report.refused.is_empty(),
        "the codec image is cleared, not refused: {:?}",
        report.refused
    );
    // Principle 1: the original codec bytes are gone from the file — no orphan to recover.
    assert!(
        !contains(&out, DCT_JPEG_16X16),
        "the original DCT stream is not left in the file"
    );

    let (width, height, rgba, is_flate_rgb) = read_back_codec_image(&out);
    assert_eq!((width, height), (16, 16), "the grid survives the re-encode");
    assert!(
        is_flate_rgb,
        "the output image is a FlateDecode DeviceRGB stream, not a codec"
    );
    for row in 0..16usize {
        for col in 0..16usize {
            let at = (row * 16 + col) * 4;
            if col < 8 {
                assert_eq!(
                    &rgba[at..at + 3],
                    &[0, 0, 0],
                    "col {col} row {row} is in the region: its sample is the zero constant"
                );
            } else {
                assert_eq!(
                    &rgba[at..at + 3],
                    &original[at..at + 3],
                    "col {col} row {row} is outside the region: byte-identical to the decode"
                );
            }
        }
    }
    // The two halves of the original differ, so a preserved right half is not a cleared one.
    assert_ne!(
        &original[8 * 4..8 * 4 + 3],
        &original[0..3],
        "the fixture's halves differ, so 'intact' is a real assertion"
    );

    let Some(Origin::Redacted { images, .. }) =
        report.outputs.first().map(|output| output.origin.clone())
    else {
        panic!("the report states a redacted origin");
    };
    assert_eq!(images, 1, "one image had samples destroyed");
}

/// The same calibration for a `CCITTFaxDecode` image (§7.4.6). The bilevel image is decoded
/// through the codec (in-process here, the confined worker in the program — principle 3), its
/// region cleared, and the output written as a `FlateDecode` `DeviceRGB` raster.
#[test]
fn a_ccitt_image_in_the_region_is_decoded_cleared_and_reencoded_as_flate() {
    // In-process decode: the same routine the confined worker runs, with no worker binary needed.
    pdf_sandbox::set_isolation(pdf_sandbox::Isolation::InProcess);
    let objects = vec![
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] /Resources << /XObject << /Im1 6 \
          0 R >> >> /Contents 4 0 R /Annots [5 0 R] >>"
            .to_vec(),
        b"<< /Length 33 >>\nstream\nq 100 0 0 100 50 50 cm /Im1 Do Q\nendstream".to_vec(),
        b"<< /Type /Annot /Subtype /Redact /Rect [50 50 100 150] \
          /QuadPoints [50 150 100 150 100 50 50 50] >>"
            .to_vec(),
        codec_image_object(
            16,
            16,
            "DeviceGray",
            1,
            "CCITTFaxDecode",
            " /DecodeParms << /K -1 /Columns 16 /Rows 16 >>",
            CCITT_G4_16X16,
        ),
    ];
    let bytes = assemble_bytes(&objects);
    let original = decode_fixture_rgba(&bytes);

    let (report, out) = redact(&bytes);
    assert!(
        report.refused.is_empty(),
        "the CCITT image is cleared, not refused: {:?}",
        report.refused
    );
    assert!(
        !contains(&out, CCITT_G4_16X16),
        "the original CCITT stream is not left in the file"
    );

    let (width, height, rgba, is_flate_rgb) = read_back_codec_image(&out);
    assert_eq!((width, height), (16, 16), "the grid survives the re-encode");
    assert!(
        is_flate_rgb,
        "the output image is a FlateDecode DeviceRGB stream, not a codec"
    );
    for row in 0..16usize {
        for col in 0..16usize {
            let at = (row * 16 + col) * 4;
            if col < 8 {
                assert_eq!(
                    &rgba[at..at + 3],
                    &[0, 0, 0],
                    "col {col} row {row} is in the region: the zero constant"
                );
            } else {
                assert_eq!(
                    &rgba[at..at + 3],
                    &original[at..at + 3],
                    "col {col} row {row} is outside the region: byte-identical to the decode"
                );
            }
        }
    }
    // Left half and right half of the fixture differ (black vs white), so the intact assertion
    // is real rather than vacuous.
    assert_ne!(
        &original[8 * 4..8 * 4 + 3],
        &original[0..3],
        "the fixture's halves differ, so 'intact' is a real assertion"
    );
}

/// The output's one image `XObject`, decoded to straight-alpha `RGBA8`, with whether it is the
/// bilevel re-encode's shape: a `FlateDecode` 1-bit `DeviceGray` image (ADR 1143).
fn read_back_bilevel_image(bytes: &[u8]) -> (u32, u32, Vec<u8>, bool) {
    let document = Document::open_with_limits(bytes.to_vec(), Limits::DEFAULT).expect("it opens");
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    let entry = document
        .get_key(&page.resources, "XObject")
        .as_dict()
        .and_then(|dict| dict.get("Im1"))
        .cloned()
        .expect("/Im1 in the resources");
    let image = document.resolve(&entry);
    let stream = image.as_stream().expect("the image is a stream");
    let is_flate_gray_1bit = {
        let filter = document.get_key(&stream.dict, "Filter");
        let space = document.get_key(&stream.dict, "ColorSpace");
        let bits = document.get_key(&stream.dict, "BitsPerComponent");
        let flate = matches!(filter, Object::Name(name) if name.as_bytes() == b"FlateDecode");
        let gray = matches!(space, Object::Name(name) if name.as_bytes() == b"DeviceGray");
        flate && gray && matches!(bits, Object::Integer(1))
    };
    // The redacted page must still draw: its interpretation must not fault on the new image.
    let _ = pdf_model::interpret(&document, &page);
    let flattened = pdf_model::image::decode(
        &document,
        stream,
        &page.resources,
        Color::BLACK,
        &Conversion::device(),
    )
    .expect("the output image decodes");
    (
        flattened.image.width,
        flattened.image.height,
        flattened.image.data.to_vec(),
        is_flate_gray_1bit,
    )
}

/// The calibration trap 13 asks for on the bilevel-codec case: the ISO 32000-2 §7.4.7 worked
/// example (a 52×66 `JBIG2Decode` image, a letter C drawn twice) over the page's [50,150]² square,
/// a `/QuadPoints` region on its left half. The image is decoded through the codec (in-process
/// here, the confined worker in the program — principle 3), its region cleared, and the output
/// written as a `FlateDecode` **1-bit `DeviceGray`** raster (ADR 1143): a lossless, non-codec
/// filter at the bilevel image's own depth, so the cleared region is exactly the one-bit zero
/// constant and cannot round-trip back through the codec. The bytes are the specification's own,
/// never another implementation's output (principle 5).
#[test]
fn a_jbig2_image_in_the_region_is_decoded_cleared_and_reencoded_as_flate() {
    // In-process decode: the same routine the confined worker runs, with no worker binary needed.
    pdf_sandbox::set_isolation(pdf_sandbox::Isolation::InProcess);
    let mut image = format!(
        "<< /Type /XObject /Subtype /Image /Width 52 /Height 66 /ColorSpace /DeviceGray \
         /BitsPerComponent 1 /Filter /JBIG2Decode /DecodeParms << /JBIG2Globals 7 0 R >> \
         /Length {} >>\nstream\n",
        JBIG2_IMAGE.len()
    )
    .into_bytes();
    image.extend_from_slice(JBIG2_IMAGE);
    image.extend_from_slice(b"\nendstream");
    let mut globals = format!("<< /Length {} >>\nstream\n", JBIG2_GLOBALS.len()).into_bytes();
    globals.extend_from_slice(JBIG2_GLOBALS);
    globals.extend_from_slice(b"\nendstream");
    let objects = vec![
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] /Resources << /XObject << /Im1 6 \
          0 R >> >> /Contents 4 0 R /Annots [5 0 R] >>"
            .to_vec(),
        b"<< /Length 33 >>\nstream\nq 100 0 0 100 50 50 cm /Im1 Do Q\nendstream".to_vec(),
        b"<< /Type /Annot /Subtype /Redact /Rect [50 50 100 150] \
          /QuadPoints [50 150 100 150 100 50 50 50] >>"
            .to_vec(),
        image,
        globals,
    ];
    let bytes = assemble_bytes(&objects);
    let original = decode_fixture_rgba(&bytes);

    let (report, out) = redact(&bytes);
    assert!(
        report.refused.is_empty(),
        "the JBIG2 image is cleared, not refused: {:?}",
        report.refused
    );
    // Principle 1: the original JBIG2 segments are gone from the file — no orphan to recover.
    assert!(
        !contains(&out, JBIG2_IMAGE),
        "the original JBIG2 image stream is not left in the file"
    );
    assert!(
        !contains(&out, JBIG2_GLOBALS),
        "the orphaned JBIG2 globals stream is not left in the file"
    );

    let (width, height, rgba, is_flate_gray_1bit) = read_back_bilevel_image(&out);
    assert_eq!((width, height), (52, 66), "the grid survives the re-encode");
    assert!(
        is_flate_gray_1bit,
        "the output image is a FlateDecode 1-bit DeviceGray stream, not a codec"
    );
    // The region is the left half of the placement: sample centres whose column maps into [50,100]
    // in user space, which is columns 0..26 of the 52-wide image (all rows).
    let (mut cleared_had_white, mut intact_had_white) = (false, false);
    for row in 0..66usize {
        for col in 0..52usize {
            let at = (row * 52 + col) * 4;
            if col < 26 {
                assert_eq!(
                    &rgba[at..at + 3],
                    &[0, 0, 0],
                    "col {col} row {row} is in the region: the one-bit zero constant (black)"
                );
                cleared_had_white |= original[at] >= 0x80;
            } else {
                assert_eq!(
                    &rgba[at..at + 3],
                    &original[at..at + 3],
                    "col {col} row {row} is outside the region: byte-identical to the decode"
                );
                intact_had_white |= original[at] >= 0x80;
            }
        }
    }
    // The cleared region held white (background) pixels the redaction turned black, so clearing
    // changed real content; the intact region held white pixels too, so 'byte-identical' is not a
    // vacuous statement about an all-black image (a bilevel page is mostly white).
    assert!(
        cleared_had_white,
        "the cleared region held white pixels the redaction destroyed"
    );
    assert!(
        intact_had_white,
        "the intact region held white pixels, so 'intact' is a real assertion"
    );

    let Some(Origin::Redacted { images, .. }) =
        report.outputs.first().map(|output| output.origin.clone())
    else {
        panic!("the report states a redacted origin");
    };
    assert_eq!(images, 1, "one image had samples destroyed");
}

/// Decodes the fixture's one image `XObject` to straight-alpha `RGBA8` from the input bytes,
/// giving the pixels the redaction's untouched region must still equal.
fn decode_fixture_rgba(bytes: &[u8]) -> Vec<u8> {
    let document = Document::open_with_limits(bytes.to_vec(), Limits::DEFAULT).expect("it opens");
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    let entry = document
        .get_key(&page.resources, "XObject")
        .as_dict()
        .and_then(|dict| dict.get("Im1"))
        .cloned()
        .expect("/Im1 in the resources");
    let image = document.resolve(&entry);
    let stream = image.as_stream().expect("the image is a stream");
    pdf_model::image::decode(
        &document,
        stream,
        &page.resources,
        Color::BLACK,
        &Conversion::device(),
    )
    .expect("the fixture image decodes")
    .image
    .data
    .to_vec()
}

/// A shared image is **copied** for the redacted page, not replaced: §12.5.6.23 asks for the
/// content the annotation identified to be removed, and the marks the *other* page's placement
/// draws are content it did not identify. So the redacted page gets its own image object with the
/// region's samples destroyed, and the original keeps the picture the unredacted page draws.
#[test]
fn a_shared_image_is_copied_for_the_redacted_page() {
    let samples = distinct_samples();
    let encoded = flate_encode(&samples, 6).expect("the fixture image deflates");
    let objects = vec![
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R 7 0 R] /Count 2 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] /Resources << /XObject << /Im1 6 \
          0 R >> >> /Contents 4 0 R /Annots [5 0 R] >>"
            .to_vec(),
        b"<< /Length 33 >>\nstream\nq 100 0 0 100 50 50 cm /Im1 Do Q\nendstream".to_vec(),
        b"<< /Type /Annot /Subtype /Redact /Rect [50 50 100 150] \
          /QuadPoints [50 150 100 150 100 50 50 50] >>"
            .to_vec(),
        image_object("FlateDecode", &encoded),
        // A second page placing the very same image object 6.
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] /Resources << /XObject << /Im1 6 \
          0 R >> >> /Contents 8 0 R >>"
            .to_vec(),
        b"<< /Length 33 >>\nstream\nq 100 0 0 100 50 50 cm /Im1 Do Q\nendstream".to_vec(),
    ];
    let bytes = assemble_bytes(&objects);

    let (report, out) = redact(&bytes);
    assert!(
        report.refused.is_empty(),
        "the shared image is copied, not refused: {:?}",
        report.refused
    );

    let (redacted, redacted_id) = read_back_samples_on(&out, 0);
    let (untouched, untouched_id) = read_back_samples_on(&out, 1);
    assert_ne!(
        redacted_id, untouched_id,
        "the two pages name two objects, so neither placement decides the other's picture"
    );
    assert_eq!(
        untouched,
        distinct_samples(),
        "the unredacted page's picture is exactly the producer's"
    );
    for row in 0..8 {
        for column in 0..8 {
            let at = row * 8 + column;
            if column < 4 {
                assert_eq!(redacted[at], 0, "sample {at} is inside the region");
            } else {
                assert_eq!(
                    redacted[at], untouched[at],
                    "sample {at} is outside the region"
                );
            }
        }
    }
}

/// A fixture whose pages each draw the one form `XObject`, which holds all of the text.
///
/// `pages` is how many pages name the form — one for a form the redacted page owns, two for a
/// form it shares. Page one carries the redaction; the form's own `/Resources` names the font, so
/// its names resolve in its own dictionary rather than the page's (§8.10.2 Table 93).
fn form_fixture(form_content: &str, pages: usize, annotation: &str) -> Vec<u8> {
    let mut objects: Vec<String> = vec![
        "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
        String::new(), // the page tree, filled once the page numbers are known
        format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] /Resources << /XObject << /Fm1 \
             5 0 R >> >> /Contents 4 0 R /Annots [6 0 R] >>"
        ),
        "<< /Length 12 >>\nstream\n/Fm1 Do\nendstream".to_owned(),
        format!(
            "<< /Type /XObject /Subtype /Form /BBox [0 0 200 200] /Resources << /Font << /F1 7 0 \
             R >> >> /Length {} >>\nstream\n{form_content}\nendstream",
            form_content.len() + 1
        ),
        annotation.to_owned(),
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_owned(),
    ];
    let mut kids = String::from("3 0 R");
    if pages > 1 {
        let number = objects.len() + 1;
        let _ = write!(kids, " {number} 0 R");
        objects.push(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] /Resources << /XObject << /Fm1 \
             5 0 R >> >> /Contents 4 0 R >>"
                .to_owned(),
        );
    }
    objects[1] = format!("<< /Type /Pages /Kids [{kids}] /Count {pages} >>");
    assemble(&objects)
}

/// The form object the given page's `/Fm1` names, and its decoded content.
fn form_of(bytes: &[u8], index: usize) -> (pdf_syntax::ObjectId, Vec<u8>) {
    let document = Document::open_with_limits(bytes.to_vec(), Limits::DEFAULT).expect("it opens");
    let page = pdf_model::Pages::new(&document)
        .get(index)
        .expect("the page");
    let xobjects = document.get_key(&page.resources, "XObject");
    let entry = xobjects
        .as_dict()
        .and_then(|dict| dict.get("Fm1"))
        .cloned()
        .expect("/Fm1 in the resources");
    let id = entry
        .as_reference()
        .expect("the form is an indirect object");
    let object = document.resolve(&entry);
    let stream = object.as_stream().expect("the form is a stream");
    let data = document
        .decoded_stream_data(stream)
        .expect("the form decodes");
    (id, data.to_vec())
}

/// A form `XObject` the redacted page owns is **entered**, and the marks it draws under the
/// region are removed from its own content stream — §8.10.1 makes a form "a self-contained
/// description of any sequence of graphics objects", so that is where those marks are described.
#[test]
fn a_form_the_page_owns_has_its_own_content_redacted() {
    let inner = "BT /F1 12 Tf 20 150 Td (KEEP) Tj ET\nBT /F1 12 Tf 20 50 Td (SECRET) Tj ET";
    let bytes = form_fixture(
        inner,
        1,
        "<< /Type /Annot /Subtype /Redact /Rect [10 40 130 66] \
         /QuadPoints [10 66 130 66 10 40 130 40] >>",
    );
    assert!(contains(&bytes, b"SECRET"), "the fixture holds the secret");

    let (report, out) = redact(&bytes);
    assert!(report.refused.is_empty(), "{:?}", report.refused);
    assert!(
        !contains(&out, b"SECRET"),
        "the text the form drew under the region is gone from the file"
    );
    assert!(
        contains(&out, b"KEEP"),
        "the text the form drew outside it stayed"
    );
    assert_eq!(page_text(&out).trim(), "KEEP");
    no_mark_meets(&out, [10.0, 40.0, 130.0, 66.0]);
    let Some(Origin::Redacted { glyphs, .. }) =
        report.outputs.first().map(|output| output.origin.clone())
    else {
        panic!("a redacted origin");
    };
    assert_eq!(glyphs, 6, "the six codes of SECRET");
}

/// A form two pages draw is **copied** for the redacted page: the marks the other page's
/// placement draws are content the annotation did not identify, so the original keeps them.
#[test]
fn a_shared_form_is_copied_for_the_redacted_page() {
    let inner = "BT /F1 12 Tf 20 150 Td (KEEP) Tj ET\nBT /F1 12 Tf 20 50 Td (SECRET) Tj ET";
    let bytes = form_fixture(
        inner,
        2,
        "<< /Type /Annot /Subtype /Redact /Rect [10 40 130 66] \
         /QuadPoints [10 66 130 66 10 40 130 40] >>",
    );
    let (report, out) = redact(&bytes);
    assert!(report.refused.is_empty(), "{:?}", report.refused);

    let (redacted_id, redacted) = form_of(&out, 0);
    let (shared_id, shared) = form_of(&out, 1);
    assert_ne!(
        redacted_id, shared_id,
        "the redacted page names a form of its own"
    );
    assert!(
        !contains(&redacted, b"SECRET"),
        "the redacted page's form lost the marks under the region"
    );
    assert!(
        contains(&shared, b"SECRET"),
        "the other page's form is exactly the producer's"
    );
    assert_eq!(
        shared.strip_suffix(b"\n").unwrap_or(&shared),
        inner.as_bytes(),
        "byte for byte"
    );
    let document = Document::open_with_limits(out.clone(), Limits::DEFAULT).expect("it opens");
    let pages = pdf_model::Pages::new(&document);
    let second = pages.get(1).expect("page two");
    assert!(
        pdf_model::interpret(&document, &second)
            .text
            .contains("SECRET"),
        "and it still draws what it drew"
    );
}

/// A painted path inside a form is cut in the **form's** own space: §8.10.1's step b) puts the
/// form's `/Matrix` in front of the transform at the `Do`, so where the region falls in the
/// form's coordinates is decided by both.
#[test]
fn a_path_inside_a_form_is_cut_in_the_form_s_own_space() {
    // The form's matrix moves it 100 units right, so its bar at form x 0..100 lands on page x
    // 100..200; the region takes the page from x = 150, which is form x = 50.
    let inner = "0 0 0 rg 0 40 100 20 re f";
    let objects: Vec<String> = vec![
        "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] /Resources << /XObject << /Fm1 5 0 \
         R >> >> /Contents 4 0 R /Annots [6 0 R] >>"
            .to_owned(),
        "<< /Length 12 >>\nstream\n/Fm1 Do\nendstream".to_owned(),
        format!(
            "<< /Type /XObject /Subtype /Form /BBox [0 0 200 200] /Matrix [1 0 0 1 100 0] \
             /Length {} >>\nstream\n{inner}\nendstream",
            inner.len() + 1
        ),
        "<< /Type /Annot /Subtype /Redact /Rect [150 30 200 80] >>".to_owned(),
    ];
    let bytes = assemble(&objects);
    let (report, out) = redact(&bytes);
    assert!(report.refused.is_empty(), "{:?}", report.refused);
    let Some(Origin::Redacted { paths, .. }) =
        report.outputs.first().map(|output| output.origin.clone())
    else {
        panic!("a redacted origin");
    };
    assert_eq!(paths, 1, "the form's path was cut");

    let (_id, content) = form_of(&out, 0);
    assert!(
        !contains(&content, b"0 40 100 20 re"),
        "the rectangle that described the removed marks is gone"
    );
    let text = String::from_utf8_lossy(&content);
    assert!(
        text.contains("49.99") || text.contains("50 "),
        "the cut is at the form's x = 50, not the page's: {text}"
    );
    no_mark_meets(&out, [150.0, 30.0, 200.0, 80.0]);
}

/// A form nested inside a **shared** form is copied too, and the outer copy names the inner one.
///
/// The inner form is referenced exactly once, so counting references alone would call it the
/// redacted page's and replace it — and the other page, which reaches it through the shared outer
/// form, would have lost content the annotation never identified. Reachability through a shared
/// form is what decides ownership, and every copy's slot is taken before any of them is built, so
/// the outer copy can name the inner one (ADR 1196).
#[test]
fn a_form_nested_in_a_shared_form_is_copied_with_it() {
    let inner = "BT /F1 12 Tf 20 150 Td (KEEP) Tj ET\nBT /F1 12 Tf 20 50 Td (SECRET) Tj ET";
    let objects: Vec<String> = vec![
        "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
        "<< /Type /Pages /Kids [3 0 R 8 0 R] /Count 2 >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] /Resources << /XObject << /Fm1 5 0 \
         R >> >> /Contents 4 0 R /Annots [6 0 R] >>"
            .to_owned(),
        "<< /Length 12 >>\nstream\n/Fm1 Do\nendstream".to_owned(),
        // The outer form: shared by both pages, and drawing the inner one.
        "<< /Type /XObject /Subtype /Form /BBox [0 0 200 200] /Resources << /XObject << /Fm2 9 0 \
         R >> >> /Length 12 >>\nstream\n/Fm2 Do\nendstream"
            .to_owned(),
        "<< /Type /Annot /Subtype /Redact /Rect [10 40 130 66] \
         /QuadPoints [10 66 130 66 10 40 130 40] >>"
            .to_owned(),
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] /Resources << /XObject << /Fm1 5 0 \
         R >> >> /Contents 4 0 R >>"
            .to_owned(),
        // The inner form: referenced once, from the outer form's resources.
        format!(
            "<< /Type /XObject /Subtype /Form /BBox [0 0 200 200] /Resources << /Font << /F1 7 0 \
             R >> >> /Length {} >>\nstream\n{inner}\nendstream",
            inner.len() + 1
        ),
    ];
    let bytes = assemble(&objects);
    let (report, out) = redact(&bytes);
    assert!(report.refused.is_empty(), "{:?}", report.refused);

    let document = Document::open_with_limits(out.clone(), Limits::DEFAULT).expect("it opens");
    let pages = pdf_model::Pages::new(&document);
    let first = pages.get(0).expect("page one");
    let second = pages.get(1).expect("page two");
    assert!(
        !pdf_model::interpret(&document, &first)
            .text
            .contains("SECRET"),
        "the redacted page draws nothing of the removed text"
    );
    assert!(
        pdf_model::interpret(&document, &second)
            .text
            .contains("SECRET"),
        "the other page, reaching the inner form through the shared outer one, is untouched"
    );
    assert_eq!(
        pdf_model::interpret(&document, &first).text.trim(),
        "KEEP",
        "and the redacted page still draws what the region did not cover"
    );
    let (outer_one, _) = form_of(&out, 0);
    let (outer_two, _) = form_of(&out, 1);
    assert_ne!(
        outer_one, outer_two,
        "the outer form was copied for the page"
    );
}

/// A page whose text lives inside a form the region does not reach is applied, not refused: the
/// interpreter runs a form's content inline, so a walk that did not enter the form would disagree
/// with the placed-code count that calibrates it.
#[test]
fn text_inside_a_form_clear_of_the_region_does_not_refuse_the_page() {
    let inner = "BT /F1 12 Tf 20 150 Td (KEEP) Tj ET";
    let bytes = form_fixture(
        inner,
        1,
        "<< /Type /Annot /Subtype /Redact /Rect [10 20 130 40] >>",
    );
    let (report, out) = redact(&bytes);
    assert!(
        report.refused.is_empty(),
        "the form's codes are counted: {:?}",
        report.refused
    );
    assert_eq!(page_text(&out).trim(), "KEEP");
    assert!(
        contains(&out, inner.as_bytes()),
        "the form's bytes are untouched"
    );
}

/// A form that draws itself is refused by name rather than walked for ever.
#[test]
fn a_form_that_draws_itself_is_refused_by_name() {
    let inner = "BT /F1 12 Tf 20 50 Td (SECRET) Tj ET\n/Fm1 Do";
    let mut objects: Vec<String> = vec![
        "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] /Resources << /XObject << /Fm1 5 0 \
         R >> >> /Contents 4 0 R /Annots [6 0 R] >>"
            .to_owned(),
        "<< /Length 12 >>\nstream\n/Fm1 Do\nendstream".to_owned(),
        format!(
            "<< /Type /XObject /Subtype /Form /BBox [0 0 200 200] /Resources << /Font << /F1 7 0 \
             R >> /XObject << /Fm1 5 0 R >> >> /Length {} >>\nstream\n{inner}\nendstream",
            inner.len() + 1
        ),
        "<< /Type /Annot /Subtype /Redact /Rect [10 40 130 66] >>".to_owned(),
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_owned(),
    ];
    objects.truncate(7);
    let bytes = assemble(&objects);
    let (report, out) = redact(&bytes);
    assert_eq!(report.refused.len(), 1, "{:?}", report.refused);
    assert!(
        report.refused[0].detail.contains("draws itself"),
        "the refusal names the recursion: {}",
        report.refused[0].detail
    );
    assert!(
        contains(&out, b"SECRET"),
        "a refused page keeps its content"
    );
}

/// The first offset of `needle` in `haystack`, or `None`.
fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

/// A one-page fixture whose content places an 8×8 `DeviceGray` inline image over the page's
/// [50,150]² square, with the given filter/data spelling written between `ID` and `EI`. `prefix`
/// and `suffix` bracket the `BI`…`EI` run so a test can prove they cross the output byte for byte.
fn inline_fixture(image: &[u8], quadpoints: &str) -> Vec<u8> {
    let mut content = b"q 100 0 0 100 50 50 cm\n".to_vec();
    content.extend_from_slice(image);
    content.extend_from_slice(b"\nQ");

    let mut content_object = format!("<< /Length {} >>\nstream\n", content.len()).into_bytes();
    content_object.extend_from_slice(&content);
    content_object.extend_from_slice(b"\nendstream");

    let objects = vec![
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] /Resources << >> /Contents 4 0 R \
          /Annots [5 0 R] >>"
            .to_vec(),
        content_object,
        format!(
            "<< /Type /Annot /Subtype /Redact /Rect [50 50 100 150] /QuadPoints [{quadpoints}] >>"
        )
        .into_bytes(),
    ];
    assemble_bytes(&objects)
}

/// The first-page content stream of `bytes`, the inline image's decoded samples, and the content
/// split around the `BI`\u{2026}`EI` run: everything before `BI` and everything after `EI`.
fn read_back_inline(bytes: &[u8]) -> (Vec<u8>, Vec<u8>, Vec<u8>, Vec<u8>) {
    let document = Document::open_with_limits(bytes.to_vec(), Limits::DEFAULT).expect("it opens");
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    let content = page.content(&document);
    // The spliced image must still draw: interpreting the page must not stumble on the new run.
    let _ = pdf_model::interpret(&document, &page);
    let bi = find(&content, b"BI").expect("a BI operator in the content");
    let scan = pdf_model::inline_image::scan(&document, &content, bi + 2, &page.resources, true);
    let stream = scan.image.expect("the inline image reads back");
    let samples = document
        .image_stream(&stream)
        .expect("the inline image decodes to samples")
        .data
        .to_vec();
    let before = content[..bi].to_vec();
    let after = content[scan.resume.min(content.len())..].to_vec();
    (content, samples, before, after)
}

/// The calibration trap 13 asks for on the inline-image case (§8.9.7, §12.5.6.23): an 8×8
/// unfiltered inline image is placed over the page's [50,150]² square and a `/QuadPoints` region
/// covers its left half. After application the left four columns are zero in the spliced image
/// and unrecoverable, the right four columns are byte-identical, every byte of the content stream
/// outside the `BI`…`EI` run is unchanged, and the file re-opens and the image decodes and draws.
#[test]
fn inline_image_samples_inside_a_quadpoints_region_are_destroyed_and_the_stream_is_spliced() {
    let samples = distinct_samples();
    let mut image = b"BI /W 8 /H 8 /BPC 8 /CS /G ID\n".to_vec();
    image.extend_from_slice(&samples);
    image.extend_from_slice(b"\nEI");
    let bytes = inline_fixture(&image, "50 150 100 150 100 50 50 50");

    let (report, out) = redact(&bytes);
    // The same fixture, read before redaction, gives the surrounding bytes to compare against.
    let (_before_content, in_samples, in_before, in_after) = read_back_inline(&bytes);
    assert_eq!(
        in_samples,
        distinct_samples(),
        "the input image reads back whole"
    );

    let (content, out_samples, out_before, out_after) = read_back_inline(&out);
    assert!(
        report.refused.is_empty(),
        "the inline image is spliced, not refused: {:?}",
        report.refused
    );
    assert_eq!(out_samples.len(), 64, "the grid survives the splice");
    for row in 0..8usize {
        for col in 0..8usize {
            let got = out_samples[row * 8 + col];
            if col < 4 {
                assert_eq!(got, 0, "col {col} row {row} is in the region: destroyed");
            } else {
                let want = u8::try_from(col * 8 + row + 1).expect("small");
                assert_eq!(got, want, "col {col} row {row} is outside: byte-identical");
            }
        }
    }

    // The surrounding stream is byte-identical: everything before BI, and everything after EI up
    // to the trailing white space the content reader appends afresh on each read (which a
    // redaction round-trip bakes into the stored stream, ADR 1124's apply-on-reader-output).
    assert_eq!(
        out_before, in_before,
        "the content before the run is byte-identical"
    );
    let trim = |b: &[u8]| {
        let end = b
            .iter()
            .rposition(|c| !c.is_ascii_whitespace())
            .map_or(0, |i| i + 1);
        b[..end].to_vec()
    };
    assert_eq!(
        trim(&out_after),
        trim(&in_after),
        "the content after the run is byte-identical but for reader trailing white space"
    );
    // Principle 1: the original sample block is gone from the decoded content, not covered \u2014 the
    // left columns are the constant and the block never appears whole again.
    assert!(
        find(&content, &samples).is_none(),
        "the original inline sample block is not left in the spliced content"
    );

    let Some(Origin::Redacted { images, glyphs, .. }) =
        report.outputs.first().map(|output| output.origin.clone())
    else {
        panic!("the report states a redacted origin");
    };
    assert_eq!(images, 1, "one inline image had its samples destroyed");
    assert_eq!(glyphs, 0, "no text was in this fixture");
}

/// An inline image behind a lossy codec (`DCTDecode`) meeting the region is refused by name, never
/// spliced — its samples are behind a codec this removal does not re-encode (trap 5, principle 1).
#[test]
fn an_inline_image_behind_a_codec_is_refused_by_name() {
    // The four bytes are never decoded: the codec is seen from `/F` before any decode is tried.
    let image = b"BI /W 8 /H 8 /BPC 8 /CS /G /F /DCT /L 4 ID\n\xff\xd8\xff\xd9\nEI".to_vec();
    let bytes = inline_fixture(&image, "50 150 100 150 100 50 50 50");

    let (report, _out) = redact(&bytes);
    let refused = report
        .refused
        .iter()
        .find(|declined| declined.page == Some(1))
        .expect("the page is refused");
    assert!(
        refused.detail.contains("DCTDecode") && refused.detail.contains("codec"),
        "the refusal names the codec: {}",
        refused.detail
    );
}

/// An inline image the redaction does not touch crosses the output byte for byte: the run is
/// skipped, never spliced, when its placement is nowhere near a region.
#[test]
fn an_inline_image_clear_of_the_region_is_left_untouched() {
    let samples = distinct_samples();
    let mut image = b"BI /W 8 /H 8 /BPC 8 /CS /G ID\n".to_vec();
    image.extend_from_slice(&samples);
    image.extend_from_slice(b"\nEI");
    // The region is the page's top-right corner [150,190]², clear of the image's [50,150]² square.
    let bytes = inline_fixture(&image, "150 190 190 190 190 150 150 150");

    let (report, out) = redact(&bytes);
    assert!(
        report.refused.is_empty(),
        "no page is refused: {:?}",
        report.refused
    );
    let (_content, out_samples, _before, _after) = read_back_inline(&out);
    assert_eq!(
        out_samples, samples,
        "the untouched inline image is byte-identical"
    );
    let Some(Origin::Redacted { images, .. }) =
        report.outputs.first().map(|output| output.origin.clone())
    else {
        panic!("the report states a redacted origin");
    };
    assert_eq!(images, 0, "no image was destroyed");
}

// --- Codec fixtures (generated with PIL; see doc/history/1136) ---
// There is no JPEG or CCITT encoder in the tree, so a valid codec stream cannot be built from
// pixels in-test; these are captured bytes. Each is decoded through the real codec, and the test
// asserts against that decode — never against a hand-predicted pixel — so what they encode is
// self-checking. Both are 16×16, left half distinct from the right so an intact right half is not
// a cleared one.

const DCT_JPEG_16X16: &[u8] = &[
    // 653 bytes  16x16 RGB baseline JPEG (left red, right blue), q90 4:4:4
    0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10, 0x4A, 0x46, 0x49, 0x46, 0x00, 0x01, 0x01, 0x00, 0x00, 0x01,
    0x00, 0x01, 0x00, 0x00, 0xFF, 0xDB, 0x00, 0x43, 0x00, 0x03, 0x02, 0x02, 0x03, 0x02, 0x02, 0x03,
    0x03, 0x03, 0x03, 0x04, 0x03, 0x03, 0x04, 0x05, 0x08, 0x05, 0x05, 0x04, 0x04, 0x05, 0x0A, 0x07,
    0x07, 0x06, 0x08, 0x0C, 0x0A, 0x0C, 0x0C, 0x0B, 0x0A, 0x0B, 0x0B, 0x0D, 0x0E, 0x12, 0x10, 0x0D,
    0x0E, 0x11, 0x0E, 0x0B, 0x0B, 0x10, 0x16, 0x10, 0x11, 0x13, 0x14, 0x15, 0x15, 0x15, 0x0C, 0x0F,
    0x17, 0x18, 0x16, 0x14, 0x18, 0x12, 0x14, 0x15, 0x14, 0xFF, 0xDB, 0x00, 0x43, 0x01, 0x03, 0x04,
    0x04, 0x05, 0x04, 0x05, 0x09, 0x05, 0x05, 0x09, 0x14, 0x0D, 0x0B, 0x0D, 0x14, 0x14, 0x14, 0x14,
    0x14, 0x14, 0x14, 0x14, 0x14, 0x14, 0x14, 0x14, 0x14, 0x14, 0x14, 0x14, 0x14, 0x14, 0x14, 0x14,
    0x14, 0x14, 0x14, 0x14, 0x14, 0x14, 0x14, 0x14, 0x14, 0x14, 0x14, 0x14, 0x14, 0x14, 0x14, 0x14,
    0x14, 0x14, 0x14, 0x14, 0x14, 0x14, 0x14, 0x14, 0x14, 0x14, 0x14, 0x14, 0x14, 0x14, 0xFF, 0xC0,
    0x00, 0x11, 0x08, 0x00, 0x10, 0x00, 0x10, 0x03, 0x01, 0x11, 0x00, 0x02, 0x11, 0x01, 0x03, 0x11,
    0x01, 0xFF, 0xC4, 0x00, 0x1F, 0x00, 0x00, 0x01, 0x05, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09,
    0x0A, 0x0B, 0xFF, 0xC4, 0x00, 0xB5, 0x10, 0x00, 0x02, 0x01, 0x03, 0x03, 0x02, 0x04, 0x03, 0x05,
    0x05, 0x04, 0x04, 0x00, 0x00, 0x01, 0x7D, 0x01, 0x02, 0x03, 0x00, 0x04, 0x11, 0x05, 0x12, 0x21,
    0x31, 0x41, 0x06, 0x13, 0x51, 0x61, 0x07, 0x22, 0x71, 0x14, 0x32, 0x81, 0x91, 0xA1, 0x08, 0x23,
    0x42, 0xB1, 0xC1, 0x15, 0x52, 0xD1, 0xF0, 0x24, 0x33, 0x62, 0x72, 0x82, 0x09, 0x0A, 0x16, 0x17,
    0x18, 0x19, 0x1A, 0x25, 0x26, 0x27, 0x28, 0x29, 0x2A, 0x34, 0x35, 0x36, 0x37, 0x38, 0x39, 0x3A,
    0x43, 0x44, 0x45, 0x46, 0x47, 0x48, 0x49, 0x4A, 0x53, 0x54, 0x55, 0x56, 0x57, 0x58, 0x59, 0x5A,
    0x63, 0x64, 0x65, 0x66, 0x67, 0x68, 0x69, 0x6A, 0x73, 0x74, 0x75, 0x76, 0x77, 0x78, 0x79, 0x7A,
    0x83, 0x84, 0x85, 0x86, 0x87, 0x88, 0x89, 0x8A, 0x92, 0x93, 0x94, 0x95, 0x96, 0x97, 0x98, 0x99,
    0x9A, 0xA2, 0xA3, 0xA4, 0xA5, 0xA6, 0xA7, 0xA8, 0xA9, 0xAA, 0xB2, 0xB3, 0xB4, 0xB5, 0xB6, 0xB7,
    0xB8, 0xB9, 0xBA, 0xC2, 0xC3, 0xC4, 0xC5, 0xC6, 0xC7, 0xC8, 0xC9, 0xCA, 0xD2, 0xD3, 0xD4, 0xD5,
    0xD6, 0xD7, 0xD8, 0xD9, 0xDA, 0xE1, 0xE2, 0xE3, 0xE4, 0xE5, 0xE6, 0xE7, 0xE8, 0xE9, 0xEA, 0xF1,
    0xF2, 0xF3, 0xF4, 0xF5, 0xF6, 0xF7, 0xF8, 0xF9, 0xFA, 0xFF, 0xC4, 0x00, 0x1F, 0x01, 0x00, 0x03,
    0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01,
    0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B, 0xFF, 0xC4, 0x00, 0xB5, 0x11, 0x00,
    0x02, 0x01, 0x02, 0x04, 0x04, 0x03, 0x04, 0x07, 0x05, 0x04, 0x04, 0x00, 0x01, 0x02, 0x77, 0x00,
    0x01, 0x02, 0x03, 0x11, 0x04, 0x05, 0x21, 0x31, 0x06, 0x12, 0x41, 0x51, 0x07, 0x61, 0x71, 0x13,
    0x22, 0x32, 0x81, 0x08, 0x14, 0x42, 0x91, 0xA1, 0xB1, 0xC1, 0x09, 0x23, 0x33, 0x52, 0xF0, 0x15,
    0x62, 0x72, 0xD1, 0x0A, 0x16, 0x24, 0x34, 0xE1, 0x25, 0xF1, 0x17, 0x18, 0x19, 0x1A, 0x26, 0x27,
    0x28, 0x29, 0x2A, 0x35, 0x36, 0x37, 0x38, 0x39, 0x3A, 0x43, 0x44, 0x45, 0x46, 0x47, 0x48, 0x49,
    0x4A, 0x53, 0x54, 0x55, 0x56, 0x57, 0x58, 0x59, 0x5A, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68, 0x69,
    0x6A, 0x73, 0x74, 0x75, 0x76, 0x77, 0x78, 0x79, 0x7A, 0x82, 0x83, 0x84, 0x85, 0x86, 0x87, 0x88,
    0x89, 0x8A, 0x92, 0x93, 0x94, 0x95, 0x96, 0x97, 0x98, 0x99, 0x9A, 0xA2, 0xA3, 0xA4, 0xA5, 0xA6,
    0xA7, 0xA8, 0xA9, 0xAA, 0xB2, 0xB3, 0xB4, 0xB5, 0xB6, 0xB7, 0xB8, 0xB9, 0xBA, 0xC2, 0xC3, 0xC4,
    0xC5, 0xC6, 0xC7, 0xC8, 0xC9, 0xCA, 0xD2, 0xD3, 0xD4, 0xD5, 0xD6, 0xD7, 0xD8, 0xD9, 0xDA, 0xE2,
    0xE3, 0xE4, 0xE5, 0xE6, 0xE7, 0xE8, 0xE9, 0xEA, 0xF2, 0xF3, 0xF4, 0xF5, 0xF6, 0xF7, 0xF8, 0xF9,
    0xFA, 0xFF, 0xDA, 0x00, 0x0C, 0x03, 0x01, 0x00, 0x02, 0x11, 0x03, 0x11, 0x00, 0x3F, 0x00, 0xF0,
    0x2A, 0xFC, 0xC8, 0xFE, 0xE3, 0x3C, 0xA6, 0xBF, 0xD3, 0x03, 0xFC, 0xF7, 0x3D, 0x5A, 0xBF, 0xCC,
    0xF3, 0xFD, 0x08, 0x3C, 0xA6, 0xBF, 0xD3, 0x03, 0xFC, 0xF7, 0x3F, 0xFF, 0xD9,
];

const CCITT_G4_16X16: &[u8] = &[
    // 9 bytes  16x16 CCITT G4, photometric=1 (0=WhiteIsZero)
    0x33, 0x17, 0xFF, 0xFF, 0xFF, 0xF0, 0x01, 0x00, 0x10,
];

// --- JBIG2 fixture (ISO 32000-2 §7.4.7's worked example; see doc/history/1143) ---
// There is no JBIG2 encoder in the tree, so a valid embedded stream cannot be built from pixels
// in-test. These are the specification's own worked example, split exactly where §7.4.7 splits it:
// a 52×66 bilevel image — a letter C drawn twice, upper half and lower — whose symbol dictionary is
// the globals stream and whose page-information and text-region segments are the image stream.
// Nothing here was taken from another implementation's output (principle 5); it is the same
// bitstream `pdf-sandbox`'s own §7.4.7 test decodes. The test decodes it through the real codec and
// asserts against that decode, never a hand-predicted pixel, so what it encodes is self-checking.

/// The `/JBIG2Globals` stream: segment 0, a symbol dictionary (ISO 32000-2 §7.4.7, part (b)).
const JBIG2_GLOBALS: &[u8] = &[
    0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x32, 0x00, 0x00, 0x03, 0xFF, 0xFD,
    0xFF, 0x02, 0xFE, 0xFE, 0xFE, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x2A, 0xE2, 0x25,
    0xAE, 0xA9, 0xA5, 0xA5, 0x38, 0xB4, 0xD9, 0x99, 0x9C, 0x5C, 0x8E, 0x56, 0xEF, 0x0F, 0x87, 0x27,
    0xF2, 0xB5, 0x3D, 0x4E, 0x37, 0xEF, 0x79, 0x5C, 0xC5, 0x50, 0x6D, 0xFF, 0xAC,
];

/// The image stream: segment 1, page information, and segment 2, an immediate text region
/// (ISO 32000-2 §7.4.7, part (c)).
const JBIG2_IMAGE: &[u8] = &[
    0x00, 0x00, 0x00, 0x01, 0x30, 0x00, 0x01, 0x00, 0x00, 0x00, 0x13, 0x00, 0x00, 0x00, 0x34, 0x00,
    0x00, 0x00, 0x42, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x40, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x02, 0x06, 0x20, 0x00, 0x01, 0x00, 0x00, 0x00, 0x1E, 0x00, 0x00, 0x00, 0x34, 0x00, 0x00,
    0x00, 0x42, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x02, 0x00, 0x10, 0x00, 0x00, 0x00,
    0x02, 0x31, 0xDB, 0x51, 0xCE, 0x51, 0xFF, 0xAC,
];

/// Encrypts a plaintext fixture with this tree's own writer, so that the tests below have an
/// encrypted source that is not a corpus document.
///
/// §7.6.4's handler at `/V` 5 and `/R` 6, which `pdf_syntax::serialize_encrypted` writes and
/// `crates/pdf-syntax/tests/serialize_encrypted.rs` holds to the clause. Here it is only the
/// input: what these two tests are about is what `redact` does when handed one.
fn encrypt(bytes: &[u8], owner: &str) -> Vec<u8> {
    let source = Document::open_with_limits(bytes.to_vec(), Limits::DEFAULT).expect("it opens");
    let mut assembly = Assembly::new(vec![&source]);
    // The fixtures `build` makes are numbered from one with no gaps, and an object number past
    // the last is `§7.3.10`'s null rather than an error, so the loop stops where `get` does.
    for number in 1..=64 {
        let id = pdf_syntax::ObjectId::new(number, 0);
        if matches!(source.get(id), Object::Null) {
            break;
        }
        assembly.copy(0, id).expect("the assembly takes it");
    }
    assembly.set_root(pdf_syntax::ObjectId::new(1, 0));
    let mut out = Vec::new();
    pdf_syntax::serialize_encrypted(
        &assembly,
        pdf_syntax::Version { major: 1, minor: 7 },
        Options::new(Form::of(&source)),
        &pdf_syntax::Protection::owner_only(owner),
        &mut pdf_syntax::SystemEntropy,
        &mut out,
    )
    .expect("the fixture is encrypted");
    out
}

/// `redact`, with the output's own §7.6 protection supplied — or not.
fn redact_protected(
    bytes: &[u8],
    password: &str,
    protect: Option<&Protect>,
) -> Result<(pdf_transform::Report, Vec<u8>), Refusal> {
    let sinks = MemorySinks::new();
    let source = Source::with_password(
        pdf_syntax::FileBytes::from(bytes.to_vec()),
        Secret::from(password.to_owned()),
    );
    let report = apply_protected(
        &Plan::Redact(RedactPlan {
            source: 0,
            names: "out.pdf".parse().expect("a pattern"),
            image_samples: None,
        }),
        &[&source],
        &sinks,
        &Policy::default(),
        &Budget::default(),
        protect,
    )?;
    let mut outputs = sinks.into_outputs();
    assert_eq!(outputs.len(), 1, "one input, one output");
    Ok((report, outputs.remove(0).1))
}

/// The fixture both encryption tests redact: "KEEP" outside the region, "SECRET" inside it.
fn secret_page() -> Vec<u8> {
    build(
        "BT /F1 12 Tf 20 150 Td (KEEP) Tj ET\nBT /F1 12 Tf 20 50 Td (SECRET) Tj ET",
        &["<< /Type /Annot /Subtype /Redact /Rect [10 40 130 66] \
           /QuadPoints [10 66 130 66 10 40 130 40] >>"],
    )
}

/// An encrypted source with no protection stated for the output is refused, and the refusal
/// says why rather than naming a missing verb.
///
/// §12.5.6.23 asks a redaction to "remove all traces of the specified content"; a person who
/// put a password on a document asked for the rest of it not to be read either. Writing the
/// survivors in the clear would answer the first by breaking the second, so the operation stops
/// and says what would let it proceed. ADR 1162.
#[test]
fn an_encrypted_source_is_refused_where_the_caller_states_no_protection_for_the_output() {
    let encrypted = encrypt(&secret_page(), "keeper");
    let refused = redact_protected(&encrypted, "keeper", None);
    match refused {
        Err(Refusal::Assembly(detail)) => {
            assert!(
                detail.contains("§7.6") && detail.contains("no passwords were supplied"),
                "the refusal names the clause and what would lift it: {detail}"
            );
        }
        other => panic!("an encrypted source is refused: {other:?}"),
    }
}

/// With passwords supplied the redaction happens and its output is protected too.
///
/// The two halves that matter are both here: the removed text is gone from the file — and not
/// merely unreadable, because a reader with the password would read it — and the file that
/// holds the survivors is itself encrypted, so the redaction did not quietly trade one
/// protection for another.
#[test]
fn an_encrypted_source_is_redacted_and_the_redaction_is_encrypted_in_turn() {
    let encrypted = encrypt(&secret_page(), "keeper");
    let (report, out) = redact_protected(
        &encrypted,
        "keeper",
        Some(&Protect::owner_only(Secret::from("newkeeper".to_owned()))),
    )
    .expect("the redaction applies");
    assert!(
        report.refused.is_empty(),
        "nothing was refused: {:?}",
        report.refused
    );
    assert!(
        !contains(&out, b"SECRET"),
        "the removed text is gone from the file"
    );
    assert!(
        !contains(&out, b"(KEEP)"),
        "and the surviving operators are ciphertext, because the output is encrypted"
    );

    let opened = Document::open_with_password(out, Limits::DEFAULT, "newkeeper").expect("it opens");
    assert!(opened.is_encrypted(), "§7.6: the output states an /Encrypt");
    let page = pdf_model::Pages::new(&opened).get(0).expect("page one");
    let text = pdf_model::interpret(&opened, &page).text;
    assert!(text.contains("KEEP"), "the outside text survives: {text:?}");
    assert!(
        !text.contains("SECRET"),
        "and the removed text is unrecoverable even with the password: {text:?}"
    );
}

// --- A mask is image data: §8.9.6.3's explicit mask and §11.6.5.2's soft mask ---

/// The packed samples of the mask the output's `/Im1` names under `key` (`SMask` or `Mask`), read
/// through every filter — or `None` where the picture names none.
fn read_back_mask(bytes: &[u8], key: &str) -> Option<Vec<u8>> {
    let document = Document::open_with_limits(bytes.to_vec(), Limits::DEFAULT).expect("it opens");
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    let entry = document
        .get_key(&page.resources, "XObject")
        .as_dict()
        .and_then(|dict| dict.get("Im1"))
        .cloned()
        .expect("/Im1 in the resources");
    let image = document.resolve(&entry);
    let stream = image.as_stream().expect("the image is a stream");
    let mask = document.get_key(&stream.dict, key);
    let mask = mask.as_stream()?;
    let decoded = document.image_stream(mask).expect("the mask decodes");
    assert!(
        decoded.codec.is_none(),
        "the cleared mask is behind no codec"
    );
    Some(decoded.data.to_vec())
}

/// A 4×4 `DeviceGray` soft-mask image, eight bits a sample, every sample distinct and non-zero:
/// `101 + 4r + c` at row `r`, column `c`.
fn soft_mask_samples() -> Vec<u8> {
    (0..16u8).map(|index| 101 + index).collect()
}

/// A one-page fixture drawing `/Im1` (object 6) over [50,150]², its left half redacted, with
/// object 7 the mask the picture names.
fn masked_page(picture: Vec<u8>, mask: Vec<u8>) -> Vec<u8> {
    assemble_bytes(&[
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] /Resources << /XObject << /Im1 6 \
          0 R >> >> /Contents 4 0 R /Annots [5 0 R] >>"
            .to_vec(),
        b"<< /Length 33 >>\nstream\nq 100 0 0 100 50 50 cm /Im1 Do Q\nendstream".to_vec(),
        b"<< /Type /Annot /Subtype /Redact /Rect [50 50 100 150] \
          /QuadPoints [50 150 100 150 100 50 50 50] >>"
            .to_vec(),
        picture,
        mask,
    ])
}

/// A stream object from a dictionary body and data.
fn stream_object(dictionary: &str, data: &[u8]) -> Vec<u8> {
    let mut object = format!("<< {dictionary} /Length {} >>\nstream\n", data.len()).into_bytes();
    object.extend_from_slice(data);
    object.extend_from_slice(b"\nendstream");
    object
}

/// The 4×4 soft mask's left two columns are zero and its right two what the file wrote.
fn assert_soft_mask_cleared(mask: &[u8]) {
    let original = soft_mask_samples();
    assert_eq!(mask.len(), 16, "the mask keeps its own 4×4 grid");
    for row in 0..4usize {
        for col in 0..4usize {
            let at = row * 4 + col;
            if col < 2 {
                assert_eq!(mask[at], 0, "mask row {row} column {col} is in the region");
            } else {
                assert_eq!(
                    mask[at], original[at],
                    "mask row {row} column {col} is intact"
                );
            }
        }
    }
}

/// A codec-free picture's soft mask is cleared on the mask's own grid, under the picture's region.
///
/// §12.5.6.23: "that portion of the image data shall be destroyed; clipping or image masks shall
/// not be used to hide that data". A §11.6.5.2 soft-mask image is image data of its own — it holds
/// the picture's shape sample by sample — and Table 143 puts it on the picture's unit square
/// "regardless of whether the samples coincide individually", so the picture's placement is the
/// mask's and the region's share of it is the columns whose centres fall in the region on the
/// mask's *own* 4×4 grid, not the picture's 8×8. The picture is cleared exactly as before, the
/// mask's original bytes are gone from the file, and the picture names the cleared mask.
#[test]
fn a_soft_mask_is_cleared_on_its_own_grid_with_its_picture() {
    let picture = flate_encode(&distinct_samples(), 6).expect("the picture deflates");
    let mask = flate_encode(&soft_mask_samples(), 6).expect("the mask deflates");
    let bytes = masked_page(
        stream_object(
            "/Type /XObject /Subtype /Image /Width 8 /Height 8 /ColorSpace /DeviceGray \
             /BitsPerComponent 8 /SMask 7 0 R /Filter /FlateDecode",
            &picture,
        ),
        stream_object(
            "/Type /XObject /Subtype /Image /Width 4 /Height 4 /ColorSpace /DeviceGray \
             /BitsPerComponent 8 /Filter /FlateDecode",
            &mask,
        ),
    );

    let (report, out) = redact(&bytes);
    assert!(
        report.refused.is_empty(),
        "nothing is refused: {:?}",
        report.refused
    );
    assert!(
        !contains(&out, &mask),
        "the original mask bytes are not left in the file"
    );
    let samples = read_back_samples(&out);
    for row in 0..8usize {
        for col in 0..8usize {
            let want = if col < 4 {
                0
            } else {
                u8::try_from(col * 8 + row + 1).expect("small")
            };
            assert_eq!(
                samples[row * 8 + col],
                want,
                "picture row {row} column {col}"
            );
        }
    }
    assert_soft_mask_cleared(&read_back_mask(&out, "SMask").expect("the soft mask is named"));
    let Some(Origin::Redacted { images, .. }) =
        report.outputs.first().map(|output| output.origin.clone())
    else {
        panic!("the report states a redacted origin");
    };
    assert_eq!(
        images, 1,
        "one picture is counted; its mask travels with it"
    );
}

/// A `DCTDecode` picture carrying a soft mask is cleared, and so is its mask, each on its own grid.
///
/// The picture is decoded **without** its mask — the mask is cleared as an image of its own, so
/// multiplying it into the picture's alpha would hide nothing and lose the mask's resolution — and
/// re-encoded as the opaque 8-bit `DeviceRGB` raster ADR 1133 writes for a colour codec, now naming
/// the cleared mask. The assertions are the picture's colour against its own decode, the mask's
/// samples against the file's, and the alpha a reader derives from the two: zero in the region.
#[test]
fn a_dct_image_carrying_a_soft_mask_is_cleared_with_its_mask() {
    let mask = flate_encode(&soft_mask_samples(), 6).expect("the mask deflates");
    let bytes = masked_page(
        codec_image_object(
            16,
            16,
            "DeviceRGB",
            8,
            "DCTDecode",
            " /SMask 7 0 R",
            DCT_JPEG_16X16,
        ),
        stream_object(
            "/Type /XObject /Subtype /Image /Width 4 /Height 4 /ColorSpace /DeviceGray \
             /BitsPerComponent 8 /Filter /FlateDecode",
            &mask,
        ),
    );
    let original = decode_fixture_rgba(&bytes);

    let (report, out) = redact(&bytes);
    assert!(
        report.refused.is_empty(),
        "nothing is refused: {:?}",
        report.refused
    );
    assert!(
        !contains(&out, DCT_JPEG_16X16),
        "the original DCT stream is gone"
    );
    assert!(!contains(&out, &mask), "the original mask bytes are gone");
    let (width, height, rgba, flate_rgb) = read_back_codec_image(&out);
    assert_eq!((width, height), (16, 16), "the grid survives the re-encode");
    assert!(flate_rgb, "the picture is a FlateDecode DeviceRGB stream");
    for row in 0..16usize {
        for col in 0..16usize {
            let at = (row * 16 + col) * 4;
            if col < 8 {
                assert_eq!(
                    &rgba[at..at + 4],
                    &[0, 0, 0, 0],
                    "row {row} column {col} is gone"
                );
            } else {
                assert_eq!(
                    &rgba[at..at + 4],
                    &original[at..at + 4],
                    "row {row} column {col}"
                );
            }
        }
    }
    assert_soft_mask_cleared(&read_back_mask(&out, "SMask").expect("the soft mask is named"));
}

/// An 8×8 explicit mask, one bit a sample: row `r` is the byte `0x5A ^ r`, so every row differs
/// and each has both painted and unpainted samples on both halves.
fn explicit_mask_rows() -> Vec<u8> {
    (0..8u8).map(|row| 0x5A ^ row).collect()
}

/// A `DCTDecode` picture carrying a §8.9.6.3 explicit mask is cleared, and so is the mask.
///
/// §8.9.6.3: "[t]he base image and the image mask need not have the same resolution ( Width and
/// Height values), but since all images shall be defined on the unit square in user space, their
/// boundaries on the page will coincide". The mask is a one-bit image of its own, so its region
/// bits are cleared to the sample domain's zero on its own 8×8 grid, under the picture's 16×16.
#[test]
fn a_dct_image_carrying_an_explicit_mask_is_cleared_with_its_mask() {
    let rows = explicit_mask_rows();
    let bytes = masked_page(
        codec_image_object(
            16,
            16,
            "DeviceRGB",
            8,
            "DCTDecode",
            " /Mask 7 0 R",
            DCT_JPEG_16X16,
        ),
        stream_object(
            "/Type /XObject /Subtype /Image /Width 8 /Height 8 /ImageMask true",
            &rows,
        ),
    );

    let (report, out) = redact(&bytes);
    assert!(
        report.refused.is_empty(),
        "nothing is refused: {:?}",
        report.refused
    );
    let (_width, _height, rgba, flate_rgb) = read_back_codec_image(&out);
    assert!(flate_rgb, "the picture is a FlateDecode DeviceRGB stream");
    assert_eq!(
        &rgba[0..3],
        &[0, 0, 0],
        "the picture's region is the zero constant"
    );
    let mask = read_back_mask(&out, "Mask").expect("the explicit mask is named");
    for (row, (got, source)) in mask.iter().zip(&rows).enumerate() {
        assert_eq!(
            got & 0xF0,
            0,
            "row {row}: the left four bits are in the region"
        );
        assert_eq!(
            got & 0x0F,
            source & 0x0F,
            "row {row}: the right four bits are intact"
        );
    }
}

/// A 16×16 baseline JPEG in four flat 8×8 quadrants — red over green on the left, blue over yellow
/// on the right — so a colour key that names one quadrant's colour masks part of each half and not
/// the rest. Generated with PIL (quality 95, no chroma subsampling), and every assertion is against
/// this tree's own decode of it rather than against the colours it was made from:
///
/// ```sh
/// python3 -c "from PIL import Image; im = Image.new('RGB', (16, 16)); px = im.load(); \
///   [px.__setitem__((x, y), [[(200,30,30),(30,160,40)],[(29,30,200),(220,200,40)]][x//8][y//8]) \
///    for x in range(16) for y in range(16)]; im.save('q.jpg', quality=95, subsampling=0)"
/// ```
const DCT_JPEG_QUADRANTS: &[u8] = &[
    0xff, 0xd8, 0xff, 0xe0, 0x00, 0x10, 0x4a, 0x46, 0x49, 0x46, 0x00, 0x01, 0x01, 0x00, 0x00, 0x01,
    0x00, 0x01, 0x00, 0x00, 0xff, 0xdb, 0x00, 0x43, 0x00, 0x02, 0x01, 0x01, 0x01, 0x01, 0x01, 0x02,
    0x01, 0x01, 0x01, 0x02, 0x02, 0x02, 0x02, 0x02, 0x04, 0x03, 0x02, 0x02, 0x02, 0x02, 0x05, 0x04,
    0x04, 0x03, 0x04, 0x06, 0x05, 0x06, 0x06, 0x06, 0x05, 0x06, 0x06, 0x06, 0x07, 0x09, 0x08, 0x06,
    0x07, 0x09, 0x07, 0x06, 0x06, 0x08, 0x0b, 0x08, 0x09, 0x0a, 0x0a, 0x0a, 0x0a, 0x0a, 0x06, 0x08,
    0x0b, 0x0c, 0x0b, 0x0a, 0x0c, 0x09, 0x0a, 0x0a, 0x0a, 0xff, 0xdb, 0x00, 0x43, 0x01, 0x02, 0x02,
    0x02, 0x02, 0x02, 0x02, 0x05, 0x03, 0x03, 0x05, 0x0a, 0x07, 0x06, 0x07, 0x0a, 0x0a, 0x0a, 0x0a,
    0x0a, 0x0a, 0x0a, 0x0a, 0x0a, 0x0a, 0x0a, 0x0a, 0x0a, 0x0a, 0x0a, 0x0a, 0x0a, 0x0a, 0x0a, 0x0a,
    0x0a, 0x0a, 0x0a, 0x0a, 0x0a, 0x0a, 0x0a, 0x0a, 0x0a, 0x0a, 0x0a, 0x0a, 0x0a, 0x0a, 0x0a, 0x0a,
    0x0a, 0x0a, 0x0a, 0x0a, 0x0a, 0x0a, 0x0a, 0x0a, 0x0a, 0x0a, 0x0a, 0x0a, 0x0a, 0x0a, 0xff, 0xc0,
    0x00, 0x11, 0x08, 0x00, 0x10, 0x00, 0x10, 0x03, 0x01, 0x11, 0x00, 0x02, 0x11, 0x01, 0x03, 0x11,
    0x01, 0xff, 0xc4, 0x00, 0x1f, 0x00, 0x00, 0x01, 0x05, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09,
    0x0a, 0x0b, 0xff, 0xc4, 0x00, 0xb5, 0x10, 0x00, 0x02, 0x01, 0x03, 0x03, 0x02, 0x04, 0x03, 0x05,
    0x05, 0x04, 0x04, 0x00, 0x00, 0x01, 0x7d, 0x01, 0x02, 0x03, 0x00, 0x04, 0x11, 0x05, 0x12, 0x21,
    0x31, 0x41, 0x06, 0x13, 0x51, 0x61, 0x07, 0x22, 0x71, 0x14, 0x32, 0x81, 0x91, 0xa1, 0x08, 0x23,
    0x42, 0xb1, 0xc1, 0x15, 0x52, 0xd1, 0xf0, 0x24, 0x33, 0x62, 0x72, 0x82, 0x09, 0x0a, 0x16, 0x17,
    0x18, 0x19, 0x1a, 0x25, 0x26, 0x27, 0x28, 0x29, 0x2a, 0x34, 0x35, 0x36, 0x37, 0x38, 0x39, 0x3a,
    0x43, 0x44, 0x45, 0x46, 0x47, 0x48, 0x49, 0x4a, 0x53, 0x54, 0x55, 0x56, 0x57, 0x58, 0x59, 0x5a,
    0x63, 0x64, 0x65, 0x66, 0x67, 0x68, 0x69, 0x6a, 0x73, 0x74, 0x75, 0x76, 0x77, 0x78, 0x79, 0x7a,
    0x83, 0x84, 0x85, 0x86, 0x87, 0x88, 0x89, 0x8a, 0x92, 0x93, 0x94, 0x95, 0x96, 0x97, 0x98, 0x99,
    0x9a, 0xa2, 0xa3, 0xa4, 0xa5, 0xa6, 0xa7, 0xa8, 0xa9, 0xaa, 0xb2, 0xb3, 0xb4, 0xb5, 0xb6, 0xb7,
    0xb8, 0xb9, 0xba, 0xc2, 0xc3, 0xc4, 0xc5, 0xc6, 0xc7, 0xc8, 0xc9, 0xca, 0xd2, 0xd3, 0xd4, 0xd5,
    0xd6, 0xd7, 0xd8, 0xd9, 0xda, 0xe1, 0xe2, 0xe3, 0xe4, 0xe5, 0xe6, 0xe7, 0xe8, 0xe9, 0xea, 0xf1,
    0xf2, 0xf3, 0xf4, 0xf5, 0xf6, 0xf7, 0xf8, 0xf9, 0xfa, 0xff, 0xc4, 0x00, 0x1f, 0x01, 0x00, 0x03,
    0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01,
    0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0xff, 0xc4, 0x00, 0xb5, 0x11, 0x00,
    0x02, 0x01, 0x02, 0x04, 0x04, 0x03, 0x04, 0x07, 0x05, 0x04, 0x04, 0x00, 0x01, 0x02, 0x77, 0x00,
    0x01, 0x02, 0x03, 0x11, 0x04, 0x05, 0x21, 0x31, 0x06, 0x12, 0x41, 0x51, 0x07, 0x61, 0x71, 0x13,
    0x22, 0x32, 0x81, 0x08, 0x14, 0x42, 0x91, 0xa1, 0xb1, 0xc1, 0x09, 0x23, 0x33, 0x52, 0xf0, 0x15,
    0x62, 0x72, 0xd1, 0x0a, 0x16, 0x24, 0x34, 0xe1, 0x25, 0xf1, 0x17, 0x18, 0x19, 0x1a, 0x26, 0x27,
    0x28, 0x29, 0x2a, 0x35, 0x36, 0x37, 0x38, 0x39, 0x3a, 0x43, 0x44, 0x45, 0x46, 0x47, 0x48, 0x49,
    0x4a, 0x53, 0x54, 0x55, 0x56, 0x57, 0x58, 0x59, 0x5a, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68, 0x69,
    0x6a, 0x73, 0x74, 0x75, 0x76, 0x77, 0x78, 0x79, 0x7a, 0x82, 0x83, 0x84, 0x85, 0x86, 0x87, 0x88,
    0x89, 0x8a, 0x92, 0x93, 0x94, 0x95, 0x96, 0x97, 0x98, 0x99, 0x9a, 0xa2, 0xa3, 0xa4, 0xa5, 0xa6,
    0xa7, 0xa8, 0xa9, 0xaa, 0xb2, 0xb3, 0xb4, 0xb5, 0xb6, 0xb7, 0xb8, 0xb9, 0xba, 0xc2, 0xc3, 0xc4,
    0xc5, 0xc6, 0xc7, 0xc8, 0xc9, 0xca, 0xd2, 0xd3, 0xd4, 0xd5, 0xd6, 0xd7, 0xd8, 0xd9, 0xda, 0xe2,
    0xe3, 0xe4, 0xe5, 0xe6, 0xe7, 0xe8, 0xe9, 0xea, 0xf2, 0xf3, 0xf4, 0xf5, 0xf6, 0xf7, 0xf8, 0xf9,
    0xfa, 0xff, 0xda, 0x00, 0x0c, 0x03, 0x01, 0x00, 0x02, 0x11, 0x03, 0x11, 0x00, 0x3f, 0x00, 0xf9,
    0x0e, 0xbf, 0x0b, 0x3f, 0xd5, 0x43, 0xe7, 0xfa, 0xff, 0x00, 0x72, 0x0f, 0xf1, 0xcc, 0xfb, 0xa2,
    0xbf, 0xc1, 0x33, 0xf9, 0x6c, 0xfd, 0x44, 0xaf, 0xe4, 0xb3, 0xfd, 0x38, 0x3f, 0xff, 0xd9,
];

/// A colour codec picture's colour key becomes the §8.9.6.3 stencil it is equivalent to (ADR 1333).
///
/// §8.9.6.4 tests "colour values before decoding with the Decode array", which a `DCTDecode`
/// picture re-expressed as eight-bit `DeviceRGB` no longer holds, so the ranges cannot be carried.
/// What they mean can: "[s]amples in the image that fall within this range shall not be painted",
/// and an explicit mask's places are ones that "shall not be" painted either. So the output names
/// a one-bit image mask on the picture's grid that masks exactly what the key masked, and is
/// cleared under the region with the picture. The key is chosen from the decode itself — a range
/// around one sample of the right half — so the stencil has places of both kinds outside the
/// region, and the assertion is the drawn alpha against the source's own, sample by sample.
/// [`DCT_JPEG_QUADRANTS`]'s right half is blue over yellow, and the key is a range around a blue
/// sample.
#[test]
fn a_codec_image_with_a_colour_key_is_cleared_with_the_stencil_the_key_becomes() {
    let plain = masked_page(
        codec_image_object(16, 16, "DeviceRGB", 8, "DCTDecode", "", DCT_JPEG_QUADRANTS),
        b"null".to_vec(),
    );
    let colours = decode_fixture_rgba(&plain);
    let at = (3 * 16 + 12) * 4;
    let range = |channel: usize| {
        let value = i32::from(colours[at + channel]);
        format!("{} {}", (value - 6).max(0), (value + 6).min(255))
    };
    let key = format!(" /Mask [{} {} {}]", range(0), range(1), range(2));
    let bytes = masked_page(
        codec_image_object(
            16,
            16,
            "DeviceRGB",
            8,
            "DCTDecode",
            &key,
            DCT_JPEG_QUADRANTS,
        ),
        b"null".to_vec(),
    );
    let keyed = decode_fixture_rgba(&bytes);
    let right = |rgba: &[u8], alpha: u8| {
        (0..16usize)
            .flat_map(|row| (8..16usize).map(move |col| (row * 16 + col) * 4 + 3))
            .filter(|at| rgba[*at] == alpha)
            .count()
    };
    assert!(
        right(&keyed, 0) > 0 && right(&keyed, 255) > 0,
        "the key masks some of the right half and paints the rest, so the stencil is discriminating"
    );

    let (report, out) = redact(&bytes);
    assert!(
        report.refused.is_empty(),
        "the key becomes a stencil, so nothing is refused: {:?}",
        report.refused
    );
    assert!(
        image_numbers(&out, "Mask").is_none(),
        "the ranges are not carried into a domain they were not written in"
    );
    let stencil = read_back_mask(&out, "Mask").expect("the picture names a stencil");
    assert_eq!(
        stencil.len(),
        32,
        "one bit a sample on the picture's 16×16 grid"
    );
    let (_, _, rgba, flate_rgb) = read_back_codec_image(&out);
    assert!(
        flate_rgb,
        "the picture is re-expressed as FlateDecode DeviceRGB"
    );
    for row in 0..16usize {
        for col in 0..16usize {
            let at = (row * 16 + col) * 4;
            if col < 8 {
                assert_eq!(
                    &rgba[at..at + 4],
                    &[0, 0, 0, 255],
                    "row {row} column {col}: the picture and its stencil are both the zero"
                );
            } else {
                assert_eq!(
                    rgba[at + 3],
                    keyed[at + 3],
                    "row {row} column {col}: masked exactly where the key masked"
                );
                if keyed[at + 3] == 255 {
                    assert_eq!(
                        &rgba[at..at + 3],
                        &colours[at..at + 3],
                        "row {row} column {col}"
                    );
                }
            }
        }
    }
}

/// A 16×16 `DeviceGray` soft mask for a pre-blended picture: every sample distinct and non-zero.
fn matte_mask_samples() -> Vec<u8> {
    (0..=255u8).map(|index| index | 1).collect()
}

/// A picture pre-blended with its soft mask's `/Matte` is cleared to the matte (ADR 1333).
///
/// §11.6.5.2's pre-blending is `c′ = m + α × (c − m)`, and the cleared soft mask states α = 0 over
/// the region, where the formula gives `c′ = m` — so the picture's cleared samples are the matte
/// colour, and the region holds pre-blended data the formula could have produced rather than a
/// zero it could not. Table 143 puts a matted mask on its picture's grid ("[i]f a Matte entry …
/// is present, shall be the same as the Width value of the parent image"), so the two clears
/// cover the same samples. The `DCTDecode` picture is `DeviceRGB`, which its eight-bit re-encode
/// keeps component for component, so the matte `[0.2 0.6 1]` becomes the samples 51, 153, 255
/// by §8.9.5.2's default map and the mask carries its `/Matte` unchanged.
#[test]
fn a_codec_picture_pre_blended_with_a_matte_is_cleared_to_the_matte() {
    let mask = flate_encode(&matte_mask_samples(), 6).expect("the mask deflates");
    let bytes = masked_page(
        codec_image_object(
            16,
            16,
            "DeviceRGB",
            8,
            "DCTDecode",
            " /SMask 7 0 R",
            DCT_JPEG_16X16,
        ),
        stream_object(
            "/Type /XObject /Subtype /Image /Width 16 /Height 16 /ColorSpace /DeviceGray \
             /BitsPerComponent 8 /Matte [0.2 0.6 1] /Filter /FlateDecode",
            &mask,
        ),
    );
    let colours = decode_fixture_rgba(&masked_page(
        codec_image_object(16, 16, "DeviceRGB", 8, "DCTDecode", "", DCT_JPEG_16X16),
        b"null".to_vec(),
    ));

    let (report, out) = redact(&bytes);
    assert!(report.refused.is_empty(), "cleared: {:?}", report.refused);
    let picture = read_back_samples(&out);
    for row in 0..16usize {
        for col in 0..16usize {
            let at = (row * 16 + col) * 3;
            let want: [u8; 3] = if col < 8 {
                [51, 153, 255]
            } else {
                let from = (row * 16 + col) * 4;
                [colours[from], colours[from + 1], colours[from + 2]]
            };
            assert_eq!(
                &picture[at..at + 3],
                &want,
                "picture row {row} column {col}"
            );
        }
    }
    let cleared = read_back_mask(&out, "SMask").expect("the soft mask is named");
    let source = matte_mask_samples();
    for row in 0..16usize {
        for col in 0..16usize {
            let want = if col < 8 { 0 } else { source[row * 16 + col] };
            assert_eq!(cleared[row * 16 + col], want, "mask row {row} column {col}");
        }
    }
    let document = Document::open_with_limits(out.clone(), Limits::DEFAULT).expect("it opens");
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    let picture = document.resolve(
        document
            .get_key(&page.resources, "XObject")
            .as_dict()
            .and_then(|dict| dict.get("Im1"))
            .expect("/Im1"),
    );
    let mask = document.get_key(&picture.as_stream().expect("a stream").dict, "SMask");
    let matte: Vec<f64> = document
        .get_key(&mask.as_stream().expect("a stream").dict, "Matte")
        .as_array()
        .expect("the matte is carried")
        .iter()
        .filter_map(Object::as_number)
        .collect();
    assert_eq!(
        matte,
        vec![0.2, 0.6, 1.0],
        "a DeviceRGB picture keeps its matte as stated"
    );
}

/// A `JPXDecode` picture pre-blended with a matte is cleared to the matte in its own samples.
///
/// The picture is written as its own grey samples ([`jpx_page`]'s `DeviceGray`), so the matte 0.5
/// is carried into them by §8.9.5.2's default map run backwards — 127.5, the nearest sample 128 —
/// and the mask's `/Matte` is carried unchanged, both images still in the picture's own space.
#[test]
fn a_jpx_picture_pre_blended_with_a_matte_is_cleared_to_the_matte() {
    pdf_sandbox::set_isolation(pdf_sandbox::Isolation::InProcess);
    let mask_samples: Vec<u8> = (1..=64u8).collect();
    let mask = flate_encode(&mask_samples, 6).expect("the mask deflates");
    let mut bytes_objects = Vec::new();
    let mut image = format!(
        "<< /Type /XObject /Subtype /Image /Width 8 /Height 8 /ColorSpace /DeviceGray \
         /Filter /JPXDecode /SMask 7 0 R /Length {} >>\nstream\n",
        JPX_ONE_COMPONENT.len()
    )
    .into_bytes();
    image.extend_from_slice(JPX_ONE_COMPONENT);
    image.extend_from_slice(b"\nendstream");
    bytes_objects.push(image);
    let bytes = masked_page(
        bytes_objects.remove(0),
        stream_object(
            "/Type /XObject /Subtype /Image /Width 8 /Height 8 /ColorSpace /DeviceGray \
             /BitsPerComponent 8 /Matte [0.5] /Filter /FlateDecode",
            &mask,
        ),
    );

    let (report, out) = redact(&bytes);
    assert!(report.refused.is_empty(), "cleared: {:?}", report.refused);
    let picture = read_back_samples(&out);
    for row in 0..8usize {
        for col in 0..8usize {
            let want = if col < 4 { 128 } else { JPX_SAMPLE };
            assert_eq!(
                picture[row * 8 + col],
                want,
                "picture row {row} column {col}"
            );
        }
    }
    let cleared = read_back_mask(&out, "SMask").expect("the soft mask is named");
    for row in 0..8usize {
        for col in 0..8usize {
            let want = if col < 4 {
                0
            } else {
                mask_samples[row * 8 + col]
            };
            assert_eq!(cleared[row * 8 + col], want, "mask row {row} column {col}");
        }
    }
}

/// A matte the re-expression cannot keep refuses the page by name.
///
/// A `CalRGB` picture reaches the eight-bit `DeviceRGB` re-encode through §8.6.5.3's conversion,
/// which is not affine in the picture's components, so Table 144's relation between the picture
/// and its mask does not survive it: refused rather than written with a matte that no longer
/// describes the samples.
#[test]
fn a_matte_the_re_expression_cannot_keep_refuses_the_page() {
    let mask = flate_encode(&matte_mask_samples(), 6).expect("the mask deflates");
    let mut picture = format!(
        "<< /Type /XObject /Subtype /Image /Width 16 /Height 16 \
         /ColorSpace [/CalRGB << /WhitePoint [0.9505 1 1.089] /Gamma [2.2 2.2 2.2] >>] \
         /BitsPerComponent 8 /Filter /DCTDecode /SMask 7 0 R /Length {} >>\nstream\n",
        DCT_JPEG_16X16.len()
    )
    .into_bytes();
    picture.extend_from_slice(DCT_JPEG_16X16);
    picture.extend_from_slice(b"\nendstream");
    let bytes = masked_page(
        picture,
        stream_object(
            "/Type /XObject /Subtype /Image /Width 16 /Height 16 /ColorSpace /DeviceGray \
             /BitsPerComponent 8 /Matte [0.2 0.6 1] /Filter /FlateDecode",
            &mask,
        ),
    );

    let (report, _out) = redact(&bytes);
    let refused = report
        .refused
        .iter()
        .find(|declined| declined.page == Some(1))
        .expect("the page is refused");
    assert!(
        refused.detail.contains("§11.6.5.2") && refused.detail.contains("Table 144"),
        "the refusal names the matte and the relation it cannot keep: {}",
        refused.detail
    );
}

/// A §8.9.6.2 image mask behind `CCITTFaxDecode` is decoded, cleared and written as a one-bit
/// stencil under `FlateDecode`.
///
/// §8.9.6.2: "a sample value of 0 shall mark the page with the current colour, and a 1 shall leave
/// the previous contents unchanged". The decode gives the stencil as alpha — painted or not — and
/// the re-expression writes it back as those bits under the default `/Decode`, so outside the
/// region a reader paints exactly what it painted before, and inside it the bits are the sample
/// domain's zero, as every other cleared image's are (ADR 1126).
#[test]
fn a_ccitt_image_mask_in_the_region_is_cleared_as_a_stencil() {
    pdf_sandbox::set_isolation(pdf_sandbox::Isolation::InProcess);
    let bytes = masked_page(
        stream_object(
            "/Type /XObject /Subtype /Image /Width 16 /Height 16 /ImageMask true \
             /Filter /CCITTFaxDecode /DecodeParms << /K -1 /Columns 16 /Rows 16 >>",
            CCITT_G4_16X16,
        ),
        b"null".to_vec(),
    );
    let original = decode_fixture_rgba(&bytes);

    let (report, out) = redact(&bytes);
    assert!(
        report.refused.is_empty(),
        "nothing is refused: {:?}",
        report.refused
    );
    assert!(
        !contains(&out, CCITT_G4_16X16),
        "the original CCITT stream is gone"
    );
    let document = Document::open_with_limits(out.clone(), Limits::DEFAULT).expect("it opens");
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    let image = document.resolve(
        document
            .get_key(&page.resources, "XObject")
            .as_dict()
            .and_then(|dict| dict.get("Im1"))
            .expect("/Im1"),
    );
    let stream = image.as_stream().expect("a stream");
    assert!(
        matches!(
            document.get_key(&stream.dict, "ImageMask"),
            Object::Boolean(true)
        ),
        "the output is still an image mask"
    );
    let rgba = decode_fixture_rgba(&out);
    for row in 0..16usize {
        for col in 0..16usize {
            let alpha = rgba[(row * 16 + col) * 4 + 3];
            if col < 8 {
                assert_eq!(
                    alpha, 0xFF,
                    "row {row} column {col}: the region's zero bit paints"
                );
            } else {
                let before = original[(row * 16 + col) * 4 + 3];
                assert_eq!(alpha, before, "row {row} column {col} paints as it did");
            }
        }
    }
    // The fixture's halves are one painted and one unpainted, so both assertions above could
    // have failed: the region changed, and the intact half is not the region's constant.
    let left = original[3];
    let right = original[8 * 4 + 3];
    assert_eq!(
        (left, right),
        (0, 0xFF),
        "the source leaves its left half unpainted"
    );
}

/// An inline image behind `DCTDecode` is decoded, cleared and spliced back as an 8-bit
/// `DeviceRGB` inline image under `FlateDecode`.
///
/// §8.9.7 lets an inline image use `DCTDecode` — "JBIG2Decode , Crypt and JPXDecode are not listed
/// … because those filters shall not be used with inline images" leaves it in — so the codec is
/// decoded the interpreter's own way and re-expressed as ADR 1133 re-expresses an image `XObject`
/// behind it. The spliced image's left half is the zero constant and its right half the decode's.
#[test]
#[expect(
    clippy::doc_markdown,
    reason = "the comment quotes §8.9.7 verbatim, and a quotation is not marked up"
)]
fn an_inline_dct_image_is_decoded_cleared_and_spliced() {
    let mut image = format!(
        "BI /W 16 /H 16 /BPC 8 /CS /RGB /F /DCT /L {} ID\n",
        DCT_JPEG_16X16.len()
    )
    .into_bytes();
    image.extend_from_slice(DCT_JPEG_16X16);
    image.extend_from_slice(b"\nEI");
    let bytes = inline_fixture(&image, "50 150 100 150 100 50 50 50");
    let original = inline_rgba(&bytes);

    let (report, out) = redact(&bytes);
    assert!(
        report.refused.is_empty(),
        "nothing is refused: {:?}",
        report.refused
    );
    let (content, samples, _before, _after) = read_back_inline(&out);
    assert!(
        find(&content, DCT_JPEG_16X16).is_none(),
        "the DCT data is not left in the content"
    );
    assert!(
        find(&content, b"/DeviceRGB").is_some(),
        "re-expressed as DeviceRGB"
    );
    assert_eq!(
        samples.len(),
        16 * 16 * 3,
        "an 8-bit RGB raster on the image's grid"
    );
    for row in 0..16usize {
        for col in 0..16usize {
            let at = (row * 16 + col) * 3;
            if col < 8 {
                assert_eq!(
                    &samples[at..at + 3],
                    &[0, 0, 0],
                    "row {row} column {col} is gone"
                );
            } else {
                let from = (row * 16 + col) * 4;
                assert_eq!(
                    &samples[at..at + 3],
                    &original[from..from + 3],
                    "{row}, {col}"
                );
            }
        }
    }
}

/// The first page's inline image, decoded the interpreter's way to straight-alpha `RGBA8`.
fn inline_rgba(bytes: &[u8]) -> Vec<u8> {
    let document = Document::open_with_limits(bytes.to_vec(), Limits::DEFAULT).expect("it opens");
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    let content = page.content(&document);
    let bi = find(&content, b"BI").expect("a BI operator");
    let scan = pdf_model::inline_image::scan(&document, &content, bi + 2, &page.resources, true);
    let stream = scan.image.expect("the inline image reads");
    pdf_model::image::decode(
        &document,
        &stream,
        &page.resources,
        Color::BLACK,
        &Conversion::device(),
    )
    .expect("the inline image decodes")
    .image
    .data
    .to_vec()
}

/// An inline image whose colour space is a resource name is spliced under that name.
///
/// §8.9.7: "the value of the ColorSpace entry may also be the name of a colour space in the
/// ColorSpace subdictionary of the current resource dictionary". The name here resolves to a
/// `/Separation` whose tint transform is a §7.3.8 reference no content stream can hold, so the
/// splice writes the producer's *name* again — the resources in force are the same ones — and
/// the samples are cleared as a codec-free image's.
#[test]
#[expect(
    clippy::doc_markdown,
    reason = "the comment quotes §8.9.7 verbatim, and a quotation is not marked up"
)]
fn an_inline_image_naming_a_colour_space_resource_is_spliced_under_that_name() {
    let samples = distinct_samples();
    let mut image = b"BI /W 8 /H 8 /BPC 8 /CS /CS0 ID\n".to_vec();
    image.extend_from_slice(&samples);
    image.extend_from_slice(b"\nEI");
    let mut content = b"q 100 0 0 100 50 50 cm\n".to_vec();
    content.extend_from_slice(&image);
    content.extend_from_slice(b"\nQ");
    let bytes = assemble_bytes(&[
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] /Resources << /ColorSpace << \
          /CS0 [/Separation /Spot /DeviceGray 6 0 R] >> >> /Contents 4 0 R /Annots [5 0 R] >>"
            .to_vec(),
        stream_object("", &content),
        b"<< /Type /Annot /Subtype /Redact /Rect [50 50 100 150] \
          /QuadPoints [50 150 100 150 100 50 50 50] >>"
            .to_vec(),
        b"<< /FunctionType 2 /Domain [0 1] /C0 [1] /C1 [0] /N 1 >>".to_vec(),
    ]);

    let (report, out) = redact(&bytes);
    assert!(
        report.refused.is_empty(),
        "nothing is refused: {:?}",
        report.refused
    );
    let (content, out_samples, _before, _after) = read_back_inline(&out);
    assert!(
        find(&content, b"/ColorSpace /CS0").is_some(),
        "the resource is named again: {}",
        String::from_utf8_lossy(&content)
    );
    for row in 0..8usize {
        for col in 0..8usize {
            let want = if col < 4 {
                0
            } else {
                u8::try_from(col * 8 + row + 1).expect("small")
            };
            assert_eq!(out_samples[row * 8 + col], want, "row {row} column {col}");
        }
    }
}

/// An inline image behind a filter §8.9.7 forbids inline refuses the page by name.
///
/// "JBIG2Decode , Crypt and JPXDecode are not listed … because those filters shall not be used with
/// inline images": such a run is not an inline image the clause describes, so it is refused rather
/// than decoded on the file's word.
#[test]
#[expect(
    clippy::doc_markdown,
    reason = "the comment quotes §8.9.7 verbatim, and a quotation is not marked up"
)]
fn an_inline_image_behind_a_forbidden_filter_refuses_the_page() {
    let image =
        b"BI /W 8 /H 8 /BPC 1 /CS /G /F /JBIG2Decode /L 4 ID\n\x00\x00\x00\x00\nEI".to_vec();
    let bytes = inline_fixture(&image, "50 150 100 150 100 50 50 50");

    let (report, _out) = redact(&bytes);
    let refused = report
        .refused
        .iter()
        .find(|declined| declined.page == Some(1))
        .expect("the page is refused");
    assert!(
        refused
            .detail
            .contains("shall not be used with inline images"),
        "the refusal names the clause: {}",
        refused.detail
    );
}

/// §8.9.7's EXAMPLE: a 17×17 inline image, eight bits a component in `DeviceRGB`, behind
/// `[/A85 /LZW]`, placed at (298, 388) and scaled to 17 units. The clause elides the example's
/// data, so the samples are this fixture's own — [`example_samples`], every one distinct and
/// non-zero — encoded the way the example states, under the example's dictionary and `cm`. The
/// encoder was a few lines of Python: §7.4.4's LZW with nine-bit codes, a clear code emitted
/// before the table reaches 512 entries so the width never changes, then §7.4.3's base-85 without
/// the `<~` prefix. The test reads the input back through the tree's own `ASCII85Decode` and
/// `LZWDecode` before it trusts anything, so the constant is checked against [`example_samples`]
/// rather than believed.
const EXAMPLE_INLINE_A85_LZW: &str = concat!(
    r#"J,hjMY]*8i)\<<E!n!+ZF?^o\ab95YA@Q&()GS7.Z0_OD6mGS9<75nWVc>7uMfn%`Lrj9d>0X9:1aEeF"#,
    r#"#K/rK@(^/5*Y:l\P&X0s>;-o=#1J.QoR0"Kd7Eo(>r\NU.TOY,ZL*%WA2;jJ<l9'u%giM.Je9]4>Vd</"#,
    r#"a947a1n[i#(JI;>;7u+:4s3[XP]B2DRnkH_$FPK9E4o[7=\SOQ?+Ad43^V2E*%YM.*lLWI6G-RF)E>c5"#,
    r#"dKEg70d]f@<`6p7]pUWl\"%>K>JN<^'i0SV&g\*a7k[c"FOS2p$WBVsPP(2PEH>Js=IOD7"-qC'Gs0*3"#,
    r#"iJkd(F=#bd(S`SSja]SRd?"Kg=YcRGWKh6&Rc/S.@^;E(-G+!+jE%pS01X(p<\PGP#G`Om.%X<eQODqC"#,
    r#"8[.2t4J.^KA6M8GQqjRoWhjIPDRs2[)!o8#;6o%t\XdX$S)=>P"cS2r_>+Y*9IQ0a-0#l1eo3X@&#]U\"#,
    r#".2n@BVN^:63jE/$>d'MGAm79mfPS,<Y(pf7lM:R6!ML[XpkD6aPR:uRl%$ok2OofjU\YQ4N(BcfMASCc"#,
    r#"'YB+*+q7YDMCe4TJ:QRnGYg.\OV;9%WBm+VFI++^.-[IDDQ'*XB<3Qb5).jf3b^6^?mH^*$O4NPHDsfH"#,
    r#"@BH$"a1D[&aHa*1X\/J*o&.X#[B@e:bJO#V^OuZ^T]h75-RTVO"g\5octtc`:Ik$i*>ji0-<#d:18S6^"#,
    r#">;"P:2FB?peSR6M:3TXMkk>)Y_*IL_>W<l>$:Vp3d<rlF2B0qsPMu`SM+8i>agD20:p!hO=%h*X*L+lb"#,
    r#"2;=--,Gj2cHN6<]ODmT=)\34n!jR:*\7%039uf&pm/Vc,,>nhIdWIsU<@cXYFlNX]a_*G6c*m5'd-Gp>"#,
    r#"?N;cF/0GZ9#=LmU:qU<t*Z3i`#\!EJMMWY=W^t"H2SP5-j=Q35Q;7"9!G.rRJ:;)](la:qiYeb@GFtQW"#,
    r#"8K'()OK54\.EktR8uUXJ].6[GkVBpkZ&,R"0u3F373rpjkNe9`lBo45DeglorrUE]5T;<<)k#:HH+j8d"#,
    r#"?s#Z@(_2?)U7l8ADE8l%#irj]"9~>"#,
);

/// The 17×17×3 samples [`EXAMPLE_INLINE_A85_LZW`] encodes: component `k` of row `r`, column `c` is
/// `(13c + 7r + 5k) mod 250 + 1`.
fn example_samples() -> Vec<u8> {
    let mut samples = Vec::with_capacity(17 * 17 * 3);
    for row in 0..17usize {
        for col in 0..17usize {
            for component in 0..3usize {
                samples.push(
                    u8::try_from((col * 13 + row * 7 + component * 5) % 250 + 1).expect("small"),
                );
            }
        }
    }
    samples
}

/// §8.9.7's EXAMPLE redacted: its left eight columns are destroyed and the rest survive.
///
/// The example's image sits at (298, 388), scaled to 17 units, so a column's centre is at
/// `298 + c + 0.5`; a region reaching x = 306 takes columns 0 to 7 and leaves column 8, whose
/// centre is 306.5. The spliced image is codec-free, so it keeps the example's grid and colour
/// space and only the encoding changes.
#[test]
fn the_inline_image_example_of_8_9_7_is_redacted() {
    let content = format!(
        concat!(
            "q\n17 0 0 17 298 388 cm\n",
            "BI /W 17 /H 17 /CS /RGB /BPC 8 /L {} /F [/A85 /LZW] ID\n{}\nEI\nQ"
        ),
        EXAMPLE_INLINE_A85_LZW.len(),
        EXAMPLE_INLINE_A85_LZW
    );
    let bytes = assemble_bytes(&[
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Resources << >> /Contents 4 0 R \
          /Annots [5 0 R] >>"
            .to_vec(),
        stream_object("", content.as_bytes()),
        b"<< /Type /Annot /Subtype /Redact /Rect [298 388 306 405] \
          /QuadPoints [298 405 306 405 306 388 298 388] >>"
            .to_vec(),
    ]);
    let (_content, input, _before, _after) = read_back_inline(&bytes);
    assert_eq!(
        input,
        example_samples(),
        "the fixture decodes to the samples it states"
    );

    let (report, out) = redact(&bytes);
    assert!(
        report.refused.is_empty(),
        "nothing is refused: {:?}",
        report.refused
    );
    let (content, samples, _before, _after) = read_back_inline(&out);
    assert!(
        find(&content, EXAMPLE_INLINE_A85_LZW.as_bytes()).is_none(),
        "the example's encoded data is not left in the content"
    );
    let original = example_samples();
    for row in 0..17usize {
        for col in 0..17usize {
            let at = (row * 17 + col) * 3;
            if col < 8 {
                assert_eq!(
                    &samples[at..at + 3],
                    &[0, 0, 0],
                    "row {row} column {col} is gone"
                );
            } else {
                assert_eq!(
                    &samples[at..at + 3],
                    &original[at..at + 3],
                    "row {row} column {col}"
                );
            }
        }
    }
}

// --- Type 3 glyphs, composite codes, `sh`, soft-mask groups and stroke colours (ADRs 1351, 1352)

/// A one-page 200 × 200 PDF: catalog, pages, the page with `resources`, its `content`, the
/// `/Redact` annotation over `rect`, then `extras` numbered from 6.
fn page_with(resources: &str, content: &str, rect: [f32; 4], extras: &[Vec<u8>]) -> Vec<u8> {
    let mut objects = vec![
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] /Resources {resources} \
             /Contents 4 0 R /Annots [5 0 R] >>"
        )
        .into_bytes(),
        stream_object("", content.as_bytes()),
        format!(
            "<< /Type /Annot /Subtype /Redact /Rect [{} {} {} {}] >>",
            rect[0], rect[1], rect[2], rect[3]
        )
        .into_bytes(),
    ];
    objects.extend(extras.iter().cloned());
    assemble_bytes(&objects)
}

/// The rebuilt show operator in the output, for an assertion's message.
fn shown(out: &[u8]) -> String {
    let at = find(out, b"TJ").unwrap_or(0);
    let from = out[..at]
        .iter()
        .rposition(|byte| *byte == b'[')
        .unwrap_or(at);
    String::from_utf8_lossy(&out[from..(at + 2).min(out.len())]).into_owned()
}

/// The redaction's proof in pixels: inside `region` nothing is drawn, and outside `region` and
/// every box in `moved` the page is identical to the original — `moved` being where a removed
/// code's own marks reached past the region, which bounding-box removal takes whole.
///
/// A one-pixel band around each box is skipped, as in
/// [`the_cut_page_is_pixel_identical_outside_the_region_and_empty_inside_it`]; and the original
/// has to mark the region, so the comparison could have failed.
fn held_outside_and_blank_inside(
    before: &[u8],
    after: &[u8],
    region: [f32; 4],
    moved: &[[f32; 4]],
) {
    let before = pixels(before);
    let after = pixels(after);
    assert_eq!((before.width, before.height), (after.width, after.height));
    let scale = before.width as f32 / PAGE_POINTS as f32;
    let to_raster = |[x0, y0, x1, y1]: [f32; 4]| {
        [
            x0 * scale,
            (PAGE_POINTS as f32 - y1) * scale,
            x1 * scale,
            (PAGE_POINTS as f32 - y0) * scale,
        ]
    };
    let inside = |b: [f32; 4], x: f32, y: f32, pad: f32| {
        x > b[0] - pad && x < b[2] + pad && y > b[1] - pad && y < b[3] + pad
    };
    let region_px = to_raster(region);
    let moved_px: Vec<[f32; 4]> = moved.iter().copied().map(to_raster).collect();
    let stride = before.width as usize * 4;
    let mut marked = false;
    for y in 0..before.height {
        for x in 0..before.width {
            let at = y as usize * stride + x as usize * 4;
            let (fx, fy) = (x as f32 + 0.5, y as f32 + 0.5);
            if inside(region_px, fx, fy, -1.0) {
                marked |= before.data[at..at + 3] != [0xFF, 0xFF, 0xFF];
                assert_eq!(
                    &after.data[at..at + 3],
                    &[0xFF, 0xFF, 0xFF],
                    "a pixel at ({x}, {y}) inside the region is still marked"
                );
            } else if !inside(region_px, fx, fy, 1.0)
                && !moved_px.iter().any(|b| inside(*b, fx, fy, 1.0))
            {
                assert_eq!(
                    &after.data[at..at + 4],
                    &before.data[at..at + 4],
                    "a pixel at ({x}, {y}) outside the region changed"
                );
            }
        }
    }
    assert!(
        marked,
        "the original marks the region, so the comparison can fail"
    );
}

/// The Type 3 font of §9.6.4's EXAMPLE, word for word: a `d1` square and a `d0` triangle.
fn example_type3_font() -> Vec<Vec<u8>> {
    vec![
        b"<< /Type /Font /Subtype /Type3 /FontBBox [-36 -36 786 786] \
          /FontMatrix [0.001 0 0 0.001 0 0] /CharProcs 8 0 R /Encoding 7 0 R \
          /FirstChar 97 /LastChar 98 /Widths [1000 1000] >>"
            .to_vec(),
        b"<< /Type /Encoding /Differences [97 /square /triangle] >>".to_vec(),
        b"<< /square 9 0 R /triangle 10 0 R >>".to_vec(),
        stream_object("", b"1000 0 -36 -36 786 786 d1 72 w 0 0 750 750 re B"),
        stream_object(
            "",
            b"1000 0 d0 72 w 0.2 0.6 0.8 rg 0.1 0.3 0.4 RG 0 0 m 375 750 l 750 0 l b",
        ),
    ]
}

/// §9.6.4's EXAMPLE redacted glyph by glyph: the third of four glyphs is removed, its advance
/// restored so the fourth stays where it was, and the glyph descriptions are left as the font's.
///
/// At 15 points a glyph advances 15 units and its declared box reaches from −0.54 to 11.79 past
/// its origin, so in `(abab)` set from x = 30 the third glyph is drawn over x = 59.46…71.79. The
/// region, x = 62…70, meets that glyph's box and no other one's.
#[test]
fn the_type3_example_of_9_6_4_is_redacted_glyph_by_glyph() {
    let content = "0.2 0.8 0.0 rg 0.1 0.4 0.0 RG BT /FT3 15 Tf 30 100 Td (abab) Tj ET";
    let region = [62.0, 90.0, 70.0, 120.0];
    let bytes = page_with(
        "<< /Font << /FT3 6 0 R >> >>",
        content,
        region,
        &example_type3_font(),
    );
    let (report, out) = redact(&bytes);
    assert!(report.refused.is_empty(), "{:?}", report.refused);
    let Some(Origin::Redacted { glyphs, .. }) =
        report.outputs.first().map(|output| output.origin.clone())
    else {
        panic!("a redacted origin");
    };
    assert_eq!(glyphs, 1, "exactly the third glyph went");
    assert!(
        contains(&out, b"[(ab) -1000(b)] TJ"),
        "the third code is gone and its 15-unit advance is a TJ adjustment of 1000 thousandths: {:?}",
        shown(&out)
    );
    assert!(
        contains(&out, b"0 0 750 750 re B"),
        "the glyph description is the font's and stays"
    );
    no_mark_meets(&out, region);
    held_outside_and_blank_inside(&bytes, &out, region, &[[59.0, 99.0, 72.5, 113.0]]);
}

/// A Type 3 glyph whose description marks far past its advance is removed where its **marks**
/// meet the region, although its advance box does not.
///
/// The glyph advances 15 units and declares with `d1` a box three ems wide; set at x = 30 its
/// marks run to x = 75, and the region at x = 68…74 is clear of every advance box on the line.
#[test]
fn a_type3_glyph_whose_marks_reach_the_region_past_its_advance_is_removed() {
    let font = vec![
        b"<< /Type /Font /Subtype /Type3 /FontBBox [0 0 3000 750] \
          /FontMatrix [0.001 0 0 0.001 0 0] /CharProcs 8 0 R /Encoding 7 0 R \
          /FirstChar 97 /LastChar 98 /Widths [1000 1000] >>"
            .to_vec(),
        b"<< /Type /Encoding /Differences [97 /long /square] >>".to_vec(),
        b"<< /long 9 0 R /square 10 0 R >>".to_vec(),
        stream_object("", b"1000 0 0 0 3000 750 d1 0 0 3000 750 re f"),
        stream_object("", b"1000 0 0 0 750 750 d1 0 0 750 750 re f"),
    ];
    let content = "0 0 0 rg BT /FT3 15 Tf 30 100 Td (ab) Tj ET";
    let region = [68.0, 95.0, 74.0, 120.0];
    let bytes = page_with("<< /Font << /FT3 6 0 R >> >>", content, region, &font);
    let (report, out) = redact(&bytes);
    assert!(report.refused.is_empty(), "{:?}", report.refused);
    assert!(
        contains(&out, b"[ -1000(b)] TJ"),
        "the long glyph went and the square stayed at its own origin: {:?}",
        shown(&out)
    );
    no_mark_meets(&out, region);
    held_outside_and_blank_inside(&bytes, &out, region, &[[30.0, 99.0, 75.5, 112.0]]);
}

/// A `d0` glyph in a font whose `/FontBBox` is all zero states nowhere where its marks fall, so
/// the page is refused by name rather than tested against a guess.
#[test]
fn a_type3_glyph_whose_marks_are_stated_nowhere_refuses_the_page() {
    let font = vec![
        b"<< /Type /Font /Subtype /Type3 /FontBBox [0 0 0 0] \
          /FontMatrix [0.001 0 0 0.001 0 0] /CharProcs 8 0 R /Encoding 7 0 R \
          /FirstChar 97 /LastChar 97 /Widths [1000] >>"
            .to_vec(),
        b"<< /Type /Encoding /Differences [97 /square] >>".to_vec(),
        b"<< /square 9 0 R >>".to_vec(),
        stream_object("", b"1000 0 d0 0 0 1 rg 0 0 750 750 re f"),
    ];
    let bytes = page_with(
        "<< /Font << /FT3 6 0 R >> >>",
        "BT /FT3 15 Tf 30 100 Td (a) Tj ET",
        [0.0, 0.0, 10.0, 10.0],
        &font,
    );
    let (report, out) = redact(&bytes);
    let refused = report.refused.first().expect("the page is refused");
    assert!(
        refused.detail.contains("d0") && refused.detail.contains("FontBBox"),
        "the refusal names why: {}",
        refused.detail
    );
    assert!(
        contains(&out, b"/Redact"),
        "the annotation is left as the file wrote it"
    );
}

/// A composite font over an embedded `CMap` of the given codespace, with a `/ToUnicode` naming
/// every code the tests below show, and no program, so the text layer is placed from `/DW`.
fn composite_font(codespace: &str, mappings: &[(&str, &str)]) -> Vec<Vec<u8>> {
    let mut cids = String::new();
    let mut chars = String::new();
    for (index, (code, unicode)) in mappings.iter().enumerate() {
        let _ = writeln!(cids, "<{code}> <{code}> {}", index + 1);
        let _ = writeln!(chars, "<{code}> <{unicode}>");
    }
    let cmap = format!(
        "/CIDInit /ProcSet findresource begin 12 dict begin begincmap\n\
         /CMapName /Test def /CMapType 1 def /WMode 0 def\n\
         /CIDSystemInfo << /Registry (Test) /Ordering (Test) /Supplement 0 >> def\n\
         {codespace}\n{} begincidrange\n{cids}endcidrange\n\
         endcmap CMapName currentdict /CMap defineresource pop end end\n",
        mappings.len()
    );
    let unicode = format!(
        "/CIDInit /ProcSet findresource begin 12 dict begin begincmap\n\
         {codespace}\n{} beginbfchar\n{chars}endbfchar\n\
         endcmap CMapName currentdict /CMap defineresource pop end end\n",
        mappings.len()
    );
    vec![
        b"<< /Type /Font /Subtype /Type0 /BaseFont /Test /Encoding 7 0 R \
          /DescendantFonts [8 0 R] /ToUnicode 10 0 R >>"
            .to_vec(),
        stream_object(
            "/Type /CMap /CMapName /Test /WMode 0 \
             /CIDSystemInfo << /Registry (Test) /Ordering (Test) /Supplement 0 >>",
            cmap.as_bytes(),
        ),
        b"<< /Type /Font /Subtype /CIDFontType0 /BaseFont /Test \
          /CIDSystemInfo << /Registry (Test) /Ordering (Test) /Supplement 0 >> \
          /FontDescriptor 9 0 R /DW 1000 >>"
            .to_vec(),
        b"<< /Type /FontDescriptor /FontName /Test /Flags 4 /ItalicAngle 0 /Ascent 800 \
          /Descent -200 /CapHeight 700 /StemV 80 /FontBBox [0 -200 1000 800] >>"
            .to_vec(),
        stream_object("", unicode.as_bytes()),
    ]
}

/// A composite font whose `CMap` is two bytes wide but not `Identity-H`: the removed code takes
/// its own two bytes and no neighbour's (§9.7.6.2).
///
/// Each code advances one em, 10 units at 10 points, so `<1000 1001 1002>` set from x = 20 puts
/// the second code over x = 30…40; the region at x = 32…38 takes that code alone.
#[test]
fn a_two_byte_cmap_s_code_is_removed_with_its_own_two_bytes() {
    let font = composite_font(
        "1 begincodespacerange\n<0000> <FFFF>\nendcodespacerange",
        &[("1000", "0041"), ("1001", "0042"), ("1002", "0043")],
    );
    let region = [32.0, 95.0, 38.0, 115.0];
    let bytes = page_with(
        "<< /Font << /F0 6 0 R >> >>",
        "BT /F0 10 Tf 20 100 Td <100010011002> Tj ET",
        region,
        &font,
    );
    assert_eq!(page_text(&bytes).trim(), "ABC", "the fixture reads back");
    let (report, out) = redact(&bytes);
    assert!(report.refused.is_empty(), "{:?}", report.refused);
    let text = page_text(&out);
    assert!(
        text.contains('A') && text.contains('C') && !text.contains('B'),
        "the middle code went and its neighbours stayed: {text:?}"
    );
    assert!(
        contains(&out, br"[(\020\000) -1000(\020\002)] TJ"),
        "two bytes went, and the gap is one em: {:?}",
        shown(&out)
    );
}

/// A mixed-length `CMap` — one-byte codes `<00>…<7F>` beside two-byte codes `<8000>…<FFFF>` — is
/// split where its codespace ranges say (§9.7.6.2): removing the two-byte code between two
/// one-byte ones takes exactly its two bytes.
#[test]
fn a_mixed_length_cmap_s_code_is_removed_with_exactly_its_own_bytes() {
    let font = composite_font(
        "2 begincodespacerange\n<00> <7F>\n<8000> <FFFF>\nendcodespacerange",
        &[
            ("41", "0041"),
            ("8142", "0042"),
            ("43", "0043"),
            ("8144", "0044"),
        ],
    );
    // A (x 20…30), B (30…40), C (40…50), D (50…60): the region takes B alone.
    let region = [32.0, 95.0, 38.0, 115.0];
    let bytes = page_with(
        "<< /Font << /F0 6 0 R >> >>",
        "BT /F0 10 Tf 20 100 Td <41814243 8144> Tj ET",
        region,
        &font,
    );
    assert_eq!(page_text(&bytes).trim(), "ABCD", "the fixture reads back");
    let (report, out) = redact(&bytes);
    assert!(report.refused.is_empty(), "{:?}", report.refused);
    let Some(Origin::Redacted { glyphs, .. }) =
        report.outputs.first().map(|output| output.origin.clone())
    else {
        panic!("a redacted origin");
    };
    assert_eq!(glyphs, 1, "one code went");
    let text = page_text(&out);
    assert!(
        text.contains('A') && text.contains("CD") && !text.contains('B'),
        "the two-byte code went, the one-byte codes either side stayed: {text:?}"
    );
    assert!(
        contains(&out, br"[(A) -1000(C\201D)] TJ"),
        "the bytes kept are exactly the three codes that were not removed: {:?}",
        shown(&out)
    );
}

/// An axial shading's exponential function, red to blue across the page.
const AXIAL_SHADING: &[u8] = b"<< /ShadingType 2 /ColorSpace /DeviceRGB /Coords [20 0 180 0] \
    /Function << /FunctionType 2 /Domain [0 1] /C0 [1 0 0] /C1 [0 0 1] /N 1 >> \
    /Extend [true true] >>";

/// `sh` paints the current clip, so its marks within the region are removed by cutting the clip:
/// the shading is painted through the clip intersected with the region's complement, and the
/// page is identical outside the region and blank inside it (§8.7.4.2, ADR 1351).
#[test]
fn a_shading_painted_with_sh_is_cut_by_its_clip() {
    let region = [60.0, 60.0, 100.0, 100.0];
    let bytes = page_with(
        "<< /Shading << /Sh0 6 0 R >> >>",
        "q 20 20 160 160 re W n /Sh0 sh Q",
        region,
        &[AXIAL_SHADING.to_vec()],
    );
    let (report, out) = redact(&bytes);
    assert!(report.refused.is_empty(), "{:?}", report.refused);
    assert!(
        contains(&out, b"W n\n/Sh0 sh\nQ"),
        "the shading is painted through a new clip"
    );
    held_outside_and_blank_inside(&bytes, &out, region, &[]);
}

/// A shading whose clip keeps it clear of the region crosses the output byte for byte.
#[test]
fn a_shading_clipped_clear_of_the_region_is_left_alone() {
    let content = "q 20 120 60 60 re W n /Sh0 sh Q";
    let bytes = page_with(
        "<< /Shading << /Sh0 6 0 R >> >>",
        content,
        [100.0, 20.0, 180.0, 80.0],
        &[AXIAL_SHADING.to_vec()],
    );
    let (report, out) = redact(&bytes);
    assert!(report.refused.is_empty(), "{:?}", report.refused);
    assert!(
        contains(&out, content.as_bytes()),
        "the content is the producer's"
    );
}

/// A shading whose colours are sampled data — a §7.10.2 sampled function behind an axial
/// shading — may hold values only the region reaches; a clip would hide them rather than
/// destroy them, so the page is refused by name.
#[test]
fn a_shading_through_a_sampled_function_refuses_the_page() {
    let shading = b"<< /ShadingType 2 /ColorSpace /DeviceGray /Coords [20 0 180 0] \
        /Function 7 0 R /Extend [true true] >>"
        .to_vec();
    let function = stream_object(
        "/FunctionType 0 /Domain [0 1] /Range [0 1] /Size [4] /BitsPerSample 8",
        &[0, 255, 0, 255],
    );
    let bytes = page_with(
        "<< /Shading << /Sh0 6 0 R >> >>",
        "/Sh0 sh",
        [60.0, 60.0, 100.0, 100.0],
        &[shading, function],
    );
    let (report, _) = redact(&bytes);
    let refused = report.refused.first().expect("the page is refused");
    assert!(refused.detail.contains("sampled"), "{}", refused.detail);
}

/// A soft mask's group is content: text drawn into a `/Luminosity` group is legible through the
/// marks the mask applies to, so it is removed from the group's own stream where the region
/// meets it, and the page is identical outside the region (§12.5.6.23, ADR 1352).
#[test]
fn text_inside_a_soft_mask_group_is_removed() {
    let group = stream_object(
        "/Type /XObject /Subtype /Form /BBox [0 0 200 200] \
         /Group << /S /Transparency /CS /DeviceGray >> /Resources << /Font << /F1 6 0 R >> >>",
        b"1 g 0 0 200 200 re f BT /F1 24 Tf 0 g 60 70 Td (SECRET) Tj ET",
    );
    let region = [55.0, 55.0, 160.0, 100.0];
    let bytes = page_with(
        "<< /ExtGState << /GS0 << /SMask << /Type /Mask /S /Luminosity /G 7 0 R >> >> >> >>",
        "q /GS0 gs 0 0 1 rg 20 20 160 160 re f Q",
        region,
        &[
            b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_vec(),
            group,
        ],
    );
    let (report, out) = redact(&bytes);
    assert!(report.refused.is_empty(), "{:?}", report.refused);
    assert!(contains(&bytes, b"SECRET"), "the fixture holds the secret");
    assert!(
        !contains(&out, b"SECRET"),
        "the mask's text is gone from the file"
    );
    held_outside_and_blank_inside(&bytes, &out, region, &[]);
}

/// A stroke no stroking colour operator has set is painted in Table 51's initial black, and is
/// cut like any other: the surviving outline is filled with `0 g`.
#[test]
fn a_stroke_in_the_initial_colour_is_cut() {
    let region = [80.0, 80.0, 120.0, 120.0];
    let bytes = page_with("<< >>", "10 w 20 100 m 180 100 l S", region, &[]);
    let (report, out) = redact(&bytes);
    assert!(report.refused.is_empty(), "{:?}", report.refused);
    assert!(
        contains(&out, b"0 g\n"),
        "the initial colour is stated for the fill"
    );
    held_outside_and_blank_inside(&bytes, &out, region, &[]);
}

/// `SC` with no `CS` before it sets a colour in the stroking space's initial `DeviceGray`,
/// whatever the non-stroking space has become, so the fill that restates it names that space.
#[test]
fn a_stroking_colour_set_in_the_initial_space_is_restated_in_it() {
    let region = [80.0, 80.0, 120.0, 120.0];
    let bytes = page_with(
        "<< >>",
        "1 0 0 rg 0.5 SC 10 w 20 100 m 180 100 l S 20 150 40 20 re f",
        region,
        &[],
    );
    let (report, out) = redact(&bytes);
    assert!(report.refused.is_empty(), "{:?}", report.refused);
    assert!(
        contains(&out, b"/DeviceGray cs\n0.5 sc\n"),
        "the space is restated before the colour"
    );
    held_outside_and_blank_inside(&bytes, &out, region, &[]);
}

/// A stroke whose `/CA` differs from the fill's `/ca` is cut, and the surviving outline is filled
/// under a graphics state stating the stroke's alpha as the fill's (§11.6.4.4), so it composites
/// exactly as the stroke did: identical outside the region, blank inside.
#[test]
fn a_stroke_whose_alpha_differs_from_the_fill_s_is_cut_at_its_own_alpha() {
    let region = [80.0, 80.0, 120.0, 120.0];
    let bytes = page_with(
        "<< /ExtGState << /GS0 << /CA 0.4 /ca 1 >> >> >>",
        "/GS0 gs 1 0 0 RG 10 w 20 100 m 180 100 l S 0 0 1 rg 20 150 40 20 re f",
        region,
        &[],
    );
    let (report, out) = redact(&bytes);
    assert!(report.refused.is_empty(), "{:?}", report.refused);
    assert!(
        contains(&out, b"/RedactStroke1 gs\n"),
        "the outline is filled under the restated alpha"
    );
    let document = Document::open_with_limits(out.clone(), Limits::DEFAULT).expect("it opens");
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    let states = document.get_key(&page.resources, "ExtGState");
    let state = states
        .as_dict()
        .and_then(|dict| dict.get("RedactStroke1"))
        .map(|state| document.resolve(state))
        .expect("the page's resources gain the state");
    let alpha = state
        .as_dict()
        .and_then(|dict| dict.get("ca"))
        .and_then(Object::as_number)
        .expect("a fill alpha");
    assert!(
        (alpha - 0.4).abs() < 1e-6,
        "the stroke's own alpha: {alpha}"
    );
    assert!(
        states
            .as_dict()
            .is_some_and(|dict| dict.get("GS0").is_some()),
        "the producer's own states are kept"
    );
    held_outside_and_blank_inside(&bytes, &out, region, &[]);
}

/// A stroke drawn inside a form under an overprint the fill does not share is restated in the
/// form's own resources — its content names the new state, so the form states it.
#[test]
fn a_stroke_whose_overprint_differs_is_restated_in_the_form_s_resources() {
    let form = stream_object(
        "/Type /XObject /Subtype /Form /BBox [0 0 200 200] \
         /Resources << /ExtGState << /GS0 << /OP true /op false >> >> >>",
        b"/GS0 gs 0 0 0 1 K 10 w 20 100 m 180 100 l S",
    );
    let region = [80.0, 80.0, 120.0, 120.0];
    let bytes = page_with(
        "<< /XObject << /Fm0 6 0 R >> >>",
        "/Fm0 Do",
        region,
        &[form],
    );
    let (report, out) = redact(&bytes);
    assert!(report.refused.is_empty(), "{:?}", report.refused);
    assert!(
        contains(&out, b"/RedactStroke1 gs\n"),
        "the outline is filled under /op true"
    );
    held_outside_and_blank_inside(&bytes, &out, region, &[]);
}

/// A path filled with a shading pattern whose shading is a gradient law is cut like any fill, and
/// the pattern carried with it holds nothing of the region's own.
#[test]
fn a_path_filled_with_an_axial_shading_pattern_is_cut() {
    let region = [60.0, 60.0, 100.0, 100.0];
    let mut pattern = b"<< /PatternType 2 /Shading ".to_vec();
    pattern.extend_from_slice(AXIAL_SHADING);
    pattern.extend_from_slice(b" >>");
    let bytes = page_with(
        "<< /Pattern << /P0 6 0 R >> >>",
        "/Pattern cs /P0 scn 20 20 160 160 re f",
        region,
        &[pattern],
    );
    let (report, out) = redact(&bytes);
    assert!(report.refused.is_empty(), "{:?}", report.refused);
    held_outside_and_blank_inside(&bytes, &out, region, &[]);
}

/// A path filled with a shading pattern whose colours are sampled data is refused by name: the
/// cut would remove the marks and carry the pattern, with the region's colours in it, past them.
#[test]
fn a_path_filled_with_a_sampled_shading_pattern_refuses_the_page() {
    let pattern = b"<< /PatternType 2 /Shading << /ShadingType 2 /ColorSpace /DeviceGray \
        /Coords [20 0 180 0] /Function 7 0 R >> >>"
        .to_vec();
    let function = stream_object(
        "/FunctionType 0 /Domain [0 1] /Range [0 1] /Size [4] /BitsPerSample 8",
        &[0, 255, 0, 255],
    );
    let bytes = page_with(
        "<< /Pattern << /P0 6 0 R >> >>",
        "/Pattern cs /P0 scn 20 20 160 160 re f",
        [60.0, 60.0, 100.0, 100.0],
        &[pattern, function],
    );
    let (report, _) = redact(&bytes);
    let refused = report.refused.first().expect("the page is refused");
    assert!(
        refused.detail.contains("shading pattern") && refused.detail.contains("sampled"),
        "{}",
        refused.detail
    );
}

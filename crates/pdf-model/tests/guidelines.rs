//! §14.11.2.2's guidelines for the page boundaries, read off `/BoxColorInfo` and drawn.
//!
//! Every expected value below is Table 396's, Table 397's or §14.11.2.1's, cited where it is used;
//! the colours a page does not state are this program's own defaults, which the clause hands it
//! ("the interactive PDF processor shall use its own current default settings instead") and
//! `pdf_model::guidelines::own_default` documents as a choice. ADR 2440.

#![expect(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    clippy::float_cmp,
    reason = "test code: a malformed fixture should fail loudly, the fixtures are small pages \
              where no index can overflow, and every number compared is read from the fixture's \
              own decimal literals and arrives exactly"
)]

use std::fmt::Write as _;

use pdf_model::content::Unsupported;
use pdf_model::guidelines::{self, BoxStyle};
use pdf_model::optional_content::Purpose;
use pdf_model::page::Boundary;
use pdf_model::view::ViewState;
use pdf_render::{Command, Paint, Rasterizer, TargetSpec};
use pdf_syntax::Document;
use render_cpu::CpuRasterizer;

/// Pixel budget, far above the small pages these tests build.
const GENEROUS: u64 = 1 << 30;

/// The trim and bleed boxes every fixture defines, inside a 200 × 100 media box.
const BOXES: &str = "/TrimBox [20 20 180 80] /BleedBox [10 10 190 90]";

/// A one-page PDF with an empty content stream, `node` on the page tree's root node and `page`
/// on the page object.
fn document(node: &str, page: &str) -> Document {
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 {node} >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 100] {page} \
         /Resources << >> /Contents 4 0 R >>\nendobj\n\
         4 0 obj\n<< /Length 0 >>\nstream\n\nendstream\nendobj\n"
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
    Document::open(out.into_bytes()).expect("the fixture is a valid PDF")
}

/// Page one interpreted with the guidelines asked for or not, for the stated purpose.
fn interpreted(
    document: &Document,
    shown: bool,
    purpose: Purpose,
) -> pdf_model::content::Interpretation {
    let page = pdf_model::Pages::new(document).get(0).expect("page one");
    let mut state = ViewState::of(document);
    state.set_guidelines(shown);
    state.set_purpose(purpose);
    pdf_model::content::interpret_with(document, &page, &state)
}

/// The strokes a list carries, as their width, dash array, dash phase and colour.
fn strokes(list: &pdf_render::DisplayList) -> Vec<(f32, Vec<f32>, f32, [f32; 3])> {
    list.commands()
        .iter()
        .filter_map(|command| match command {
            Command::Stroke {
                stroke,
                paint: Paint::Solid(colour),
                ..
            } => Some((
                stroke.width,
                stroke.dash_array.clone(),
                stroke.dash_phase,
                [colour.r, colour.g, colour.b],
            )),
            _ => None,
        })
        .collect()
}

/// Table 396: a style for each boundary the page defines, and none for one it does not.
///
/// The `/ArtBox` style is ignored because "[t]his entry shall be ignored if no art box is defined
/// in the page object"; the crop box carries no guideline because the page defines none, and its
/// default *is* the media box (§14.11.2.1). The bleed box is defined and unstyled, so it is drawn
/// in this program's own default for it; the trim box in the style Table 397 reads off its entry.
#[test]
fn a_guideline_is_drawn_for_each_defined_boundary_in_its_stated_style() {
    let document = document(
        "",
        &format!(
            "{BOXES} /BoxColorInfo << /TrimBox << /C [1 0 0] /W 4 /S /D /D [6 2] >> \
             /ArtBox << /C [0 1 0] >> >>"
        ),
    );
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    let read = guidelines::of(&document, &page);
    assert!(read.malformed.is_empty(), "{:?}", read.malformed);
    let lines: Vec<_> = read
        .lines
        .iter()
        .map(|line| (line.boundary, line.rect, line.stated))
        .collect();
    assert_eq!(
        lines,
        [
            (Boundary::Bleed, [10.0, 10.0, 190.0, 90.0], false),
            (Boundary::Trim, [20.0, 20.0, 180.0, 80.0], true),
        ]
    );
    assert_eq!(
        read.lines[0].style,
        guidelines::own_default(Boundary::Bleed)
    );
    assert_eq!(
        read.lines[1].style,
        BoxStyle {
            colour: [1.0, 0.0, 0.0],
            width: 4.0,
            dash: vec![6.0, 2.0],
        }
    );
}

/// Table 397's defaults, key by key, inside a style dictionary that states nothing:
/// "[0.0 0.0 0.0]", "1", "S" — and `/D`'s "[3]" where only `/S /D` is stated.
#[test]
fn an_absent_key_is_table_397s_default() {
    let document = document(
        "",
        &format!("{BOXES} /BoxColorInfo << /BleedBox << >> /TrimBox << /S /D >> >>"),
    );
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    let read = guidelines::of(&document, &page);
    assert_eq!(read.lines[0].style, BoxStyle::TABLE_397);
    assert!(read.lines[0].stated);
    assert_eq!(
        read.lines[1].style,
        BoxStyle {
            dash: vec![3.0],
            ..BoxStyle::TABLE_397
        }
    );
}

/// Nothing is drawn unless a person asked, and nothing at all on paper: the clause's guidelines
/// are "on the screen", and it is a permission to offer them rather than an instruction to draw.
#[test]
fn guidelines_are_drawn_only_on_a_screen_that_asked_for_them() {
    let document = document(
        "",
        &format!("{BOXES} /BoxColorInfo << /TrimBox << /C [1 0 0] >> >>"),
    );
    assert!(strokes(&interpreted(&document, false, Purpose::View).display_list).is_empty());
    assert!(strokes(&interpreted(&document, true, Purpose::Print).display_list).is_empty());
    let drawn = strokes(&interpreted(&document, true, Purpose::View).display_list);
    assert_eq!(
        drawn,
        [
            (1.0, Vec::new(), 0.0, [0.0, 0.0, 1.0]),
            (1.0, Vec::new(), 0.0, [1.0, 0.0, 0.0]),
        ],
        "the bleed box in this program's blue, the trim box in its stated red"
    );
}

/// Table 397's `/D` is §8.4.3.6's dash array at phase 0 — "[t]he dash phase shall not be
/// specified and shall be assumed to be 0" — so `[6 2]` along the trim box's bottom edge, from its
/// corner at x = 20, is on for x in 20–26, off for 26–28, and so on; the page is rendered and the
/// pixels read, because a count of strokes cannot see a stroke drawn in the wrong place (trap 1).
#[test]
fn a_dashed_guideline_reaches_the_pixels_in_its_colour_and_pattern() {
    let document = document(
        "",
        &format!("{BOXES} /BoxColorInfo << /TrimBox << /C [1 0 0] /W 4 /S /D /D [6 2] >> >>"),
    );
    let list = interpreted(&document, true, Purpose::View).display_list;
    let target =
        TargetSpec::for_page(&list, 1.0, GENEROUS).expect("a small page is a valid target");
    let raster = CpuRasterizer::new()
        .rasterize(&list, target)
        .expect("the display list holds nothing the CPU backend refuses");
    assert_eq!((raster.width, raster.height), (200, 100));
    let pixel = |x: u32, y: u32| {
        let at = ((y * raster.width + x) * 4) as usize;
        (raster.data[at], raster.data[at + 1], raster.data[at + 2])
    };
    // y = 20 in user space is row 80 from the top; x = 103 is 83 units in, 3 into a dash.
    let (r, g, b) = pixel(103, 80);
    assert!(r > 200 && g < 60 && b < 60, "on a dash: {:?}", (r, g, b));
    // x = 107 is 87 units in, 7 into the period, which is the gap's 6–8.
    assert_eq!(pixel(107, 80), (255, 255, 255), "in a gap");
    // The bleed box's left edge at x = 10, one unit wide and centred on it, in the program's blue.
    let (r, _, b) = pixel(9, 50);
    assert!(
        b > r.saturating_add(60),
        "the bleed box's blue: {:?}",
        pixel(9, 50)
    );
    // And nowhere else: the middle of the page is the paper.
    assert_eq!(pixel(100, 50), (255, 255, 255));
}

/// An inherited `/CropBox` is the page object's own under §7.7.3.4, so Table 396's crop box
/// entry applies and the crop box is drawn.
#[test]
fn an_inherited_crop_box_is_defined_in_the_page_object() {
    let document = document("/CropBox [5 5 195 95]", "");
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    let read = guidelines::of(&document, &page);
    assert_eq!(read.lines.len(), 1);
    assert_eq!(read.lines[0].boundary, Boundary::Crop);
    assert_eq!(read.lines[0].rect, [5.0, 5.0, 195.0, 95.0]);
    assert_eq!(read.lines[0].style, guidelines::own_default(Boundary::Crop));
}

/// §14.11.2.1: "If the bounds of the crop, trim, bleed or art box extends outside of the bounds of
/// the media box, a processor shall treat the box as its intersection with the media box" — the
/// guideline is drawn where the box is treated as being.
#[test]
fn a_box_past_the_media_box_is_drawn_as_its_intersection() {
    let document = document("", "/ArtBox [-50 40 150 400]");
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    let read = guidelines::of(&document, &page);
    assert_eq!(read.lines.len(), 1);
    assert_eq!(read.lines[0].rect, [0.0, 40.0, 150.0, 100.0]);
}

/// A value Table 397 does not admit is drawn at that key's default and said, rather than shown as
/// though the page had asked for it: a colour outside 0.0 to 1.0, a style this program does not
/// know ("[o]ther guideline styles may be defined in the future"), and a dash array §8.4.3.6
/// rejects ("the numbers shall be nonnegative and not all zero").
#[test]
fn a_value_table_397_does_not_admit_is_drawn_at_its_default_and_said() {
    let document = document(
        "",
        &format!(
            "{BOXES} /BoxColorInfo << /TrimBox << /C [2 0 0] /S /X >> \
             /BleedBox << /S /D /D [0 0] >> >>"
        ),
    );
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    let read = guidelines::of(&document, &page);
    assert_eq!(
        read.lines[0].style,
        BoxStyle {
            dash: vec![3.0],
            ..BoxStyle::TABLE_397
        },
        "the bleed box's dash falls back to [3]"
    );
    assert_eq!(read.lines[1].style, BoxStyle::TABLE_397);
    assert_eq!(read.malformed.len(), 3, "{:?}", read.malformed);
    let said = interpreted(&document, true, Purpose::View)
        .unsupported
        .iter()
        .filter(|unsupported| matches!(unsupported, Unsupported::Guidelines { .. }))
        .count();
    assert_eq!(said, 3, "each is said beside the page it was drawn on");
    // And nothing is said while nobody asked: the entry decides a display that is not happening.
    let silent = interpreted(&document, false, Purpose::View)
        .unsupported
        .iter()
        .any(|unsupported| matches!(unsupported, Unsupported::Guidelines { .. }));
    assert!(!silent);
}

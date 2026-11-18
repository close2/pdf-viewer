//! A simple font outside ISO 32000-2 §9.6.2.2's fourteen that states no `/Widths`: the window
//! lays it out by the order the owner chose on the file's defect and says so out loud
//! (`doc/questions/A859`, ADR 2898).
//!
//! §9.6.2.1's Table 109 makes `/Widths` "Required; optional in PDF 1.0-1.7 for the standard 14
//! fonts", so these files state no layout. The order is an embedded program's own advances, then
//! the published metrics of the standard font the name resembles, then Table 120's
//! `/MissingWidth`; `pdf-font`'s `substituted_width_tests` pin each step, and these pin what the
//! page shows and says.
#![expect(
    clippy::expect_used,
    reason = "test code: a fixture that stops opening should fail loudly"
)]

use std::fmt::Write as _;

use pdf_model::{Interpretation, Pages, interpret};
use pdf_render::{Medium, Rasterizer, TargetSpec};
use pdf_syntax::Document;
use render_cpu::CpuRasterizer;

/// Page one of `bytes`, interpreted.
fn page_one(bytes: Vec<u8>) -> Interpretation {
    let document = Document::open(bytes).expect("the document opens");
    let page = Pages::new(&document)
        .get(0)
        .expect("the document has a page");
    interpret(&document, &page)
}

/// The leftmost and rightmost columns the page marks, drawn on the processor at 72 dpi with no
/// medium under it.
fn marked_columns(interpretation: &Interpretation) -> (usize, usize) {
    let list = &interpretation.display_list;
    let target = TargetSpec::for_page(list, 1.0, 1 << 22).expect("a bounded target");
    let raster = CpuRasterizer::new()
        .with_medium(Medium::NONE)
        .rasterize(list, target)
        .expect("the page draws");
    let width = usize::try_from(raster.width).expect("a page width fits");
    let columns: Vec<usize> = raster
        .data
        .as_chunks::<4>()
        .0
        .iter()
        .enumerate()
        .filter(|(_, pixel)| pixel[3] != 0)
        .filter_map(|(index, _)| index.checked_rem(width))
        .collect();
    (
        columns.iter().copied().min().unwrap_or(0),
        columns.iter().copied().max().unwrap_or(0),
    )
}

/// A 200 × 100 page showing `(AAAA)` at 48 points from x = 10 through `/F1`, the `TrueType`
/// font `/Arial` with a descriptor and the further entries `entries`, embedding no program.
///
/// `issue5470.pdf`'s shape: a descriptor, a range, no `/Widths` and no `/MissingWidth`.
fn arial(entries: &str) -> Vec<u8> {
    let content = "BT /F1 48 Tf 10 30 Td (AAAA) Tj ET\n";
    let mut out = String::from("%PDF-1.7\n");
    let _ = write!(
        out,
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 100] /Contents 4 0 R \
         /Resources << /Font << /F1 5 0 R >> >> >>\nendobj\n\
         4 0 obj\n<< /Length {} >>\nstream\n{content}endstream\nendobj\n\
         5 0 obj\n<< /Type /Font /Subtype /TrueType /BaseFont /Arial /FirstChar 32 \
         /LastChar 255 /Encoding /WinAnsiEncoding {entries} /FontDescriptor 6 0 R >>\nendobj\n\
         6 0 obj\n<< /Type /FontDescriptor /FontName /Arial /Flags 32 /ItalicAngle 0 \
         /StemV 80 /FontBBox [-665 -325 2000 1006] /Ascent 905 /Descent -212 \
         /CapHeight 716 >>\nendobj\n",
        content.len(),
    );
    out.push_str("trailer\n<< /Root 1 0 R >>\n%%EOF\n");
    out.into_bytes()
}

/// The line keeps its letters apart: four `A`s at Helvetica's published 667 per mille of 48
/// points advance 128 points, so the ink spans more than three advances (96 points) — where
/// `/MissingWidth`'s default of 0 for every code (Table 120) would stack them in one 32-point
/// glyph's width.
#[test]
fn a_font_with_no_widths_lays_its_line_out_by_the_resembling_metrics() {
    let interpretation = page_one(arial(""));
    let (left, right) = marked_columns(&interpretation);
    assert!(right.saturating_sub(left) > 96, "{left}..{right}");
    assert_eq!(interpretation.text.trim(), "AAAA");
}

/// The defect is said (trap 5): the page is not laid out as its producer specified, because the
/// producer specified no layout.
#[test]
fn a_font_with_no_widths_is_reported() {
    let interpretation = page_one(arial(""));
    let said = format!("{:?}", interpretation.unsupported);
    assert!(!interpretation.is_complete(), "the page is incomplete");
    assert!(said.contains("font F1 states no /Widths"), "{said}");
    assert!(said.contains("Helvetica's published metrics"), "{said}");
}

/// The control: the same font stating its `/Widths` is the producer's layout, and nothing about
/// its widths is said.
#[test]
fn a_font_that_states_its_widths_reports_nothing_about_them() {
    let widths = format!("/Widths [{}]", ["667"; 224].join(" "));
    let interpretation = page_one(arial(&widths));
    let said = format!("{:?}", interpretation.unsupported);
    assert!(!said.contains("states no /Widths"), "{said}");
}

//! §12.3.3 Table 151 /F and Table 152 style flags reach visible outline text.
//!
//! Four valid PDF 1.7 documents place the same title at the same pixel position, with
//! F=0,1,2,3. Parser flags and distinct nonempty masks exercise style selection through the
//! `Sidebar`. Equal placement avoids mistaking row-dependent subpixel phase for a font change.

#![expect(
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    clippy::indexing_slicing,
    reason = "the fixture is exact-xref and all raster bands are bounded"
)]

use std::fmt::Write as _;

use pdf_model::outline::Outline;
use pdf_model::page::Pages;
use pdf_render::{Rasterizer as _, TargetSpec, Transform};
use pdf_syntax::Document;
use render_cpu::CpuRasterizer;
use viewer_ui::chrome::{Chrome, Content, Sidebar};

const WIDTH: u32 = 400;
const HEIGHT: u32 = 130;

fn styled_outline_pdf(flags: u8) -> Vec<u8> {
    let item = format!("<< /Title (MMMM) /Parent 4 0 R /F {flags} >>");
    let objects = [
        "<< /Type /Catalog /Pages 2 0 R /Outlines 4 0 R >>",
        "<< /Type /Pages /Count 1 /Kids [3 0 R] >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Resources << >> >>",
        // One leaf has no open descendants, so the root Count is omitted.
        "<< /Type /Outlines /First 5 0 R /Last 5 0 R >>",
        &item,
    ];
    let mut pdf = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for (index, body) in objects.iter().enumerate() {
        offsets.push(pdf.len());
        let _ = writeln!(pdf, "{} 0 obj\n{body}\nendobj", index + 1);
    }
    let xref_at = pdf.len();
    let size = objects.len() + 1;
    let _ = writeln!(pdf, "xref\n0 {size}\n0000000000 65535 f ");
    for offset in offsets {
        let _ = writeln!(pdf, "{offset:010} 00000 n ");
    }
    let _ = write!(
        pdf,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n"
    );
    pdf.into_bytes()
}

static NOTHING: std::sync::LazyLock<pdf_model::metadata::Information> =
    std::sync::LazyLock::new(pdf_model::metadata::Information::default);

fn panel(outline: &Outline) -> pdf_render::DisplayList {
    let content = Content {
        outline,
        layers: &[],
        attachments: &[],
        external_files: &[],
        articles: &[],
        collection: None,
        information: &NOTHING,
        metadata: None,
        page_count: 0,
        pages: None,
    };
    let chrome = Chrome::compiled_in_only().expect("compiled-in faces are available");
    let mut sidebar = Sidebar::default();
    sidebar.toggle();
    sidebar.draw(&chrome, content, HEIGHT, 1.0)
}

fn dark_mask(raster: &pdf_render::Raster, rows: std::ops::Range<u32>) -> Vec<bool> {
    let mut mask = Vec::new();
    for y in rows {
        for x in 12..150 {
            let at = ((y * WIDTH + x) * 4) as usize;
            mask.push(
                raster.data.get(at).is_some_and(|red| *red < 100)
                    && raster.data.get(at + 1).is_some_and(|green| *green < 100)
                    && raster.data.get(at + 2).is_some_and(|blue| *blue < 100),
            );
        }
    }
    mask
}

fn differs(left: &[bool], right: &[bool]) -> usize {
    left.iter().zip(right).filter(|(a, b)| a != b).count()
}

fn style_mask(flags: u8, italic: bool, bold: bool) -> Vec<bool> {
    let document = Document::open(styled_outline_pdf(flags)).expect("valid exact-xref outline PDF");
    assert!(!document.was_recovered(), "the xref offsets are exact");
    let pages = Pages::new(&document);
    let outline = Outline::read(&document, &pages);
    assert_eq!(outline.items.len(), 1);
    assert_eq!(
        (outline.items[0].italic, outline.items[0].bold),
        (italic, bold)
    );
    let display = panel(&outline);
    let raster = CpuRasterizer::new()
        .rasterize(
            &display,
            TargetSpec {
                width: WIDTH,
                height: HEIGHT,
                transform: Transform::IDENTITY,
            },
        )
        .expect("Sidebar uses rasterizable paths");
    let mask = dark_mask(&raster, 26..46);
    assert!(
        mask.iter().any(|ink| *ink),
        "style {flags} must draw visible title ink"
    );
    mask
}

#[test]
fn outline_flags_select_visible_italic_bold_and_combined_styles() {
    // Every document uses the same first row. Comparing different sidebar rows would also
    // measure their different subpixel positions: the row pitch is not an integer.
    let regular = style_mask(0, false, false);
    let italic = style_mask(1, true, false);
    let bold = style_mask(2, false, true);
    let combined = style_mask(3, true, true);
    for (left, right, label) in [
        (&regular, &italic, "regular/italic"),
        (&regular, &bold, "regular/bold"),
        (&regular, &combined, "regular/combined"),
        (&italic, &bold, "italic/bold"),
        (&italic, &combined, "italic/combined"),
        (&bold, &combined, "bold/combined"),
    ] {
        assert!(differs(left, right) > 0, "style masks must differ: {label}");
    }
}

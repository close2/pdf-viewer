//! Public-consumer assertion for §12.3.3 Table 151 `/C`.
//!
//! A valid PDF 1.7 with exact classic-xref offsets presents two equal outline titles: one
//! explicitly red and one with the Table 151 black default. The literal pixel relation checks
//! that parsing and the `Sidebar` drawing consumer preserve the stated colour. It does not pin a
//! font outline or a whole screenshot.

#![expect(
    clippy::arithmetic_side_effects,
    clippy::indexing_slicing,
    reason = "the fixed-size fixture and raster coordinates are bounded by this test"
)]

use std::fmt::Write as _;

use pdf_model::outline::Outline;
use pdf_model::page::Pages;
use pdf_render::{Rasterizer as _, TargetSpec, Transform};
use pdf_syntax::Document;
use render_cpu::CpuRasterizer;
use viewer_ui::chrome::{Chrome, Content, Sidebar};

const WIDTH: u32 = 400;
const HEIGHT: u32 = 100;

fn outline_pdf() -> Vec<u8> {
    let objects: [&[u8]; 6] = [
        b"<< /Type /Catalog /Pages 2 0 R /Outlines 4 0 R >>",
        b"<< /Type /Pages /Count 1 /Kids [3 0 R] >>",
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Resources << >> >>",
        // Both item leaves omit Count, so the root's Count is omitted as Table 150 requires
        // when the document has no open outline items.
        b"<< /Type /Outlines /First 5 0 R /Last 6 0 R >>",
        b"<< /Title (MMMM) /Parent 4 0 R /Next 6 0 R /C [1 0 0] >>",
        b"<< /Title (MMMM) /Parent 4 0 R /Prev 5 0 R >>",
    ];
    let mut pdf = b"%PDF-1.7\n".to_vec();
    let mut offsets = Vec::new();
    for (index, body) in objects.iter().enumerate() {
        offsets.push(pdf.len());
        pdf.extend_from_slice(format!("{} 0 obj\n", index + 1).as_bytes());
        pdf.extend_from_slice(body);
        pdf.extend_from_slice(b"\nendobj\n");
    }
    let xref_at = pdf.len();
    let size = objects.len() + 1;
    let mut xref = format!("xref\n0 {size}\n0000000000 65535 f \n");
    for offset in offsets {
        let _ = writeln!(xref, "{offset:010} 00000 n ");
    }
    let _ = write!(
        xref,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n"
    );
    pdf.extend_from_slice(xref.as_bytes());
    pdf
}

static NOTHING: std::sync::LazyLock<pdf_model::metadata::Information> =
    std::sync::LazyLock::new(pdf_model::metadata::Information::default);

fn red_dominant_pixels(raster: &pdf_render::Raster, y: std::ops::Range<u32>) -> usize {
    let mut count = 0;
    for row in y {
        for x in 0..WIDTH {
            let at = ((row * WIDTH + x) * 4) as usize;
            if let (Some(red), Some(green), Some(blue)) = (
                raster.data.get(at),
                raster.data.get(at + 1),
                raster.data.get(at + 2),
            ) && *red > green.saturating_add(12)
                && *red > blue.saturating_add(12)
            {
                count += 1;
            }
        }
    }
    count
}

fn black_title_pixels(raster: &pdf_render::Raster) -> usize {
    // Interior of the second title, away from tab/row borders and marker gutter.
    let mut count = 0;
    for row in 50..64 {
        for x in 20..65 {
            let at = ((row * WIDTH + x) * 4) as usize;
            if raster.data[at] < 80 && raster.data[at + 1] < 80 && raster.data[at + 2] < 80 {
                count += 1;
            }
        }
    }
    count
}

#[test]
fn explicit_outline_rgb_is_visible_in_the_panel_and_default_remains_black() {
    let document = Document::open(outline_pdf()).expect("valid exact-xref fixture");
    assert!(!document.was_recovered());
    let pages = Pages::new(&document);
    let outline = Outline::read(&document, &pages);
    assert_eq!(outline.items.len(), 2);
    assert_eq!(outline.items[0].colour, [1.0, 0.0, 0.0]);
    assert_eq!(outline.items[1].colour, [0.0, 0.0, 0.0]);

    let content = Content {
        outline: &outline,
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
    let chrome = Chrome::compiled_in_only().expect("the UI uses compiled-in standard faces");
    let mut sidebar = Sidebar::default();
    sidebar.toggle();
    let list = sidebar.draw(&chrome, content, HEIGHT, 1.0);
    let raster = CpuRasterizer::new()
        .rasterize(
            &list,
            TargetSpec {
                width: WIDTH,
                height: HEIGHT,
                transform: Transform::IDENTITY,
            },
        )
        .expect("Sidebar emits paths only");

    assert!(
        red_dominant_pixels(&raster, 26..46) > 10,
        "red title ink is absent"
    );
    assert!(
        black_title_pixels(&raster) > 10,
        "the default-black title must draw actual ink"
    );
    assert_eq!(
        red_dominant_pixels(&raster, 46..66),
        0,
        "the no-/C control uses Table 151's black default"
    );
}

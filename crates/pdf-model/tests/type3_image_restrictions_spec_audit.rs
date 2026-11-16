//! PDF 2.0 §8.6.8 suppresses coloured images in d1 glyphs, including invoked streams.
//! Image masks remain active and paint the inherited colour (NOTE 1).
#![expect(
    clippy::arithmetic_side_effects,
    clippy::expect_used,
    reason = "small exact-xref fixtures and bounded CPU pixel coordinates"
)]

use pdf_render::{Rasterizer, TargetSpec};
use pdf_syntax::Document;
use std::fmt::Write as _;

fn stream(dictionary: &str, data: &str) -> String {
    format!(
        "<< {dictionary} /Length {} >>\nstream\n{data}\nendstream",
        data.len()
    )
}

fn image_page(uncoloured: bool, mask: bool, nested: bool) -> pdf_render::Raster {
    let metric = if uncoloured {
        "1000 0 0 0 1000 1000 d1"
    } else {
        "1000 0 d0"
    };
    let draw = if nested {
        "/Fm Do"
    } else {
        "q 1000 0 0 1000 0 0 cm /Im Do Q"
    };
    let image = if mask {
        stream(
            "/Type /XObject /Subtype /Image /Width 1 /Height 1 /ImageMask true /BitsPerComponent 1 /Decode [0 1] /Filter /ASCIIHexDecode",
            "00>",
        )
    } else {
        stream(
            "/Type /XObject /Subtype /Image /Width 1 /Height 1 /ColorSpace /DeviceRGB /BitsPerComponent 8 /Filter /ASCIIHexDecode",
            "FF0000>",
        )
    };
    let objects = [
        "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] /Resources << /Font << /F3 5 0 R >> >> /Contents 4 0 R >>".to_owned(),
        stream("", "1 1 1 rg 0 0 200 200 re f 0 0 1 rg BT /F3 100 Tf 10 10 Td (a) Tj ET"),
        "<< /Type /Font /Subtype /Type3 /FontBBox [0 0 1000 1000] /FontMatrix [.001 0 0 .001 0 0] /CharProcs << /a 6 0 R >> /Encoding << /Type /Encoding /Differences [97 /a] >> /FirstChar 97 /LastChar 97 /Widths [1000] /Resources << /XObject << /Fm 7 0 R /Im 8 0 R >> >> >>".to_owned(),
        stream("", &format!("{metric} {draw}")),
        stream("/Type /XObject /Subtype /Form /BBox [0 0 1000 1000] /Resources << /XObject << /Im 8 0 R >> >>", "q 1000 0 0 1000 0 0 cm /Im Do Q"),
        image,
    ];
    let mut pdf = "%PDF-2.0\n".to_owned();
    let mut offsets = Vec::new();
    for (index, body) in objects.iter().enumerate() {
        offsets.push(pdf.len());
        let _ = writeln!(pdf, "{} 0 obj\n{body}\nendobj", index + 1);
    }
    let xref = pdf.len();
    let _ = writeln!(pdf, "xref\n0 9\n0000000000 65535 f ");
    for offset in offsets {
        let _ = writeln!(pdf, "{offset:010} 00000 n ");
    }
    let _ = write!(
        pdf,
        "trailer\n<< /Size 9 /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n"
    );
    let document = Document::open(pdf.into_bytes()).expect("exact-xref PDF");
    assert!(!document.was_recovered());
    assert!(!document.cross_reference_chain_was_cut());
    let page = pdf_model::Pages::new(&document).get(0).expect("one page");
    let interpreted = pdf_model::interpret(&document, &page);
    assert!(interpreted.is_complete(), "{:?}", interpreted.unsupported);
    let target = TargetSpec::for_page(&interpreted.display_list, 1.0, 1 << 20).expect("small page");
    render_cpu::CpuRasterizer::new()
        .rasterize(&interpreted.display_list, target)
        .expect("CPU raster")
}

fn sample(raster: &pdf_render::Raster, x: u32, page_y: u32) -> [u8; 4] {
    let row = raster.height - 1 - page_y;
    let at = ((row * raster.width + x) * 4) as usize;
    raster
        .data
        .get(at..at + 4)
        .expect("sample lies inside page")
        .try_into()
        .expect("RGBA pixel")
}

#[test]
fn d1_ignores_coloured_images_in_direct_and_invoked_content() {
    for nested in [false, true] {
        let control = image_page(false, false, nested);
        assert_eq!(
            sample(&control, 60, 60),
            [255, 0, 0, 255],
            "d0 image is red, nested={nested}"
        );
        let uncoloured = image_page(true, false, nested);
        assert_eq!(
            sample(&uncoloured, 60, 60),
            [255, 255, 255, 255],
            "d1 suppresses image, nested={nested}"
        );
    }
}

#[test]
fn d1_image_masks_in_direct_and_invoked_content_paint_inherited_blue() {
    for nested in [false, true] {
        let raster = image_page(true, true, nested);
        assert_eq!(
            sample(&raster, 60, 60),
            [0, 0, 255, 255],
            "stencil keeps inherited colour, nested={nested}"
        );
        assert_eq!(
            sample(&raster, 150, 150),
            [255, 255, 255, 255],
            "stencil does not fill outside glyph"
        );
    }
}

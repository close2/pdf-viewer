//! End-to-end checks for remaining Type 1 pattern semantics in §§8.7.2 and 8.7.3.
//! Complete PDFs with exact stream lengths, resource dictionaries and xref offsets.

#![expect(
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    reason = "a malformed fixture or out-of-raster sample must fail loudly; fixture dimensions are bounded"
)]

use std::fmt::Write as _;

use pdf_render::{Rasterizer, TargetSpec};
use pdf_syntax::Document;
use render_cpu::CpuRasterizer;

const PIXEL_BUDGET: u64 = 1 << 30;

fn stream(body: &str, content: &str) -> String {
    format!(
        "<< {body} /Length {} >>\nstream\n{content}\nendstream",
        content.len().saturating_add(1)
    )
}

fn pdf(
    page_box: &str,
    resources: &str,
    page_content: &str,
    pattern: &str,
    extra: &[String],
) -> Vec<u8> {
    let mut bodies = vec![
        "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox {page_box} /Resources << {resources} >> /Contents 4 0 R >>"
        ),
        stream("", page_content),
        pattern.to_owned(),
    ];
    bodies.extend(extra.iter().cloned());

    let mut bytes = String::from("%PDF-1.7\n");
    let mut offsets = Vec::with_capacity(bodies.len());
    for (index, body) in bodies.iter().enumerate() {
        offsets.push(bytes.len());
        writeln!(bytes, "{} 0 obj\n{body}\nendobj", index + 1)
            .expect("writing a fixture into String cannot fail");
    }
    let xref = bytes.len();
    writeln!(bytes, "xref\n0 {}\n0000000000 65535 f ", bodies.len() + 1)
        .expect("writing a fixture into String cannot fail");
    for offset in offsets {
        writeln!(bytes, "{offset:010} 00000 n ")
            .expect("writing a fixture into String cannot fail");
    }
    writeln!(
        bytes,
        "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF",
        bodies.len() + 1
    )
    .expect("writing a fixture into String cannot fail");
    bytes.into_bytes()
}

fn render(bytes: Vec<u8>) -> pdf_render::Raster {
    let document = Document::open(bytes).expect("complete PDF fixture parses");
    assert!(!document.was_recovered(), "fixture has exact xref offsets");
    let page = pdf_model::Pages::new(&document).get(0).expect("one page");
    let interpretation = pdf_model::interpret(&document, &page);
    assert!(
        interpretation.is_complete(),
        "fixture draws completely: {:?}",
        interpretation.unsupported
    );
    let target = TargetSpec::for_page(&interpretation.display_list, 1.0, PIXEL_BUDGET)
        .expect("small page fits pixel budget");
    CpuRasterizer::new()
        .with_medium(pdf_render::Medium::NONE)
        .rasterize(&interpretation.display_list, target)
        .expect("CPU rasterizer supports these operators")
}

fn pixel(image: &pdf_render::Raster, x: u32, y: u32) -> [u8; 4] {
    let index = (y * image.width + x) as usize * 4;
    image
        .data
        .get(index..index + 4)
        .expect("sample lies inside the raster")
        .try_into()
        .expect("RGBA has four bytes")
}

fn type_one_pattern(
    paint_type: u8,
    cell: &str,
    resources: &str,
    bbox: &str,
    xstep: i16,
    ystep: i16,
) -> String {
    stream(
        &format!(
            "/Type /Pattern /PatternType 1 /PaintType {paint_type} /TilingType 2 \
             /BBox {bbox} /XStep {xstep} /YStep {ystep} /Resources << {resources} >>"
        ),
        cell,
    )
}

#[test]
fn a_cell_state_change_does_not_escape_the_pattern_paint() {
    // A colored cell changes its CTM, stroke color and line width without restoring them.
    // The later parent stroke uses the prior red/4-unit state without resetting it.
    // A green cell pixel proves evaluation ran, and a red pixel two units from the path
    // distinguishes restored state from the cell's green/1-unit stroke or scaled CTM.
    let pattern = type_one_pattern(
        1,
        "5 0 0 5 0 0 cm 0 1 0 rg 0 1 0 RG 1 w 0 0 1 1 re f",
        "",
        "[0 0 10 10]",
        10,
        10,
    );
    let image = render(pdf(
        "[0 0 100 100]",
        "/Pattern << /P 5 0 R >> /ColorSpace << /CS /Pattern >>",
        "1 0 0 RG 4 w /CS cs /P scn 0 0 40 40 re f 70 70 10 10 re S",
        &pattern,
        &[],
    ));
    let cell_mark = pixel(&image, 2, 97);
    assert!(
        cell_mark[1] > 240 && cell_mark[0] < 15 && cell_mark[2] < 15 && cell_mark[3] == 255,
        "the coloured pattern cell paints green at its scaled origin: {cell_mark:?}"
    );
    assert_eq!(
        pixel(&image, 68, 24),
        [255, 0, 0, 255],
        "the later parent stroke keeps its CTM, red color and four-unit width"
    );
    assert_eq!(
        pixel(&image, 75, 24)[3],
        0,
        "the parent stroke does not fill its interior"
    );
}

#[test]
fn an_uncoloured_pattern_cell_can_paint_an_image_mask_through_its_supplied_colour() {
    // §8.7.3.3 allows an image mask in an otherwise uncoloured cell. §8.6.8 also requires
    // image-mask painting to remain active there. A one-bit ASCII-hex image containing zero
    // paints the current stencil colour under §8.9.6.2; the second tile and intervening gap
    // distinguish a real mask-driven cell from a solid fallback or a dropped image operation.
    let cell = "10 0 0 10 0 0 cm /Im Do";
    let pattern = type_one_pattern(2, cell, "/XObject << /Im 7 0 R >>", "[0 0 10 10]", 20, 10);
    let color_space = "[/Pattern /DeviceRGB]".to_owned();
    let image_mask = stream(
        "/Type /XObject /Subtype /Image /Width 1 /Height 1 /ImageMask true \
         /BitsPerComponent 1 /Decode [0 1] /Filter /ASCIIHexDecode",
        "00>",
    );
    let image = render(pdf(
        "[0 0 40 10]",
        "/Pattern << /P 5 0 R >> /ColorSpace << /CS 6 0 R >>",
        "/CS cs 0 0 1 /P scn 0 0 40 10 re f",
        &pattern,
        &[color_space, image_mask],
    ));
    assert_eq!(
        pixel(&image, 4, 5),
        [0, 0, 255, 255],
        "the zero sample paints the supplied blue through the first stencil cell"
    );
    assert_eq!(
        pixel(&image, 24, 5),
        [0, 0, 255, 255],
        "the mask's painted cell repeats at the stated XStep"
    );
    assert_eq!(
        pixel(&image, 14, 5)[3],
        0,
        "the gap between BBox and XStep is not painted"
    );
}

#[test]
fn negative_steps_cover_the_same_pattern_lattice_as_positive_steps() {
    // Signed steps enumerate the same infinite set of cell origins; reversing either axis
    // must not drop a row, reverse cell artwork, or change the phase. Positive-case pixels
    // establish real repeated marks and gaps before the cross-sign comparisons.
    let draw = |xstep, ystep| {
        let pattern = type_one_pattern(1, "0 1 0 rg 0 0 5 5 re f", "", "[0 0 10 10]", xstep, ystep);
        render(pdf(
            "[0 0 40 30]",
            "/Pattern << /P 5 0 R >>",
            "/Pattern cs /P scn 0 0 40 30 re f",
            &pattern,
            &[],
        ))
    };
    let positive = draw(20, 15);
    for (x, y) in [(2, 27), (22, 27), (2, 12), (22, 12)] {
        assert_eq!(pixel(&positive, x, y), [0, 255, 0, 255]);
    }
    for (x, y) in [(12, 27), (12, 12), (22, 7)] {
        assert_eq!(pixel(&positive, x, y)[3], 0, "positive lattice gap");
    }
    for (xstep, ystep) in [(-20, 15), (20, -15), (-20, -15)] {
        assert_eq!(
            draw(xstep, ystep).data,
            positive.data,
            "step signs change enumeration, not cell placement: {xstep}, {ystep}"
        );
    }
}

#[test]
fn page_ctm_changes_do_not_reanchor_a_pattern_selected_before_or_after_them() {
    let pattern = type_one_pattern(1, "0 1 0 rg 0 0 5 5 re f", "", "[0 0 10 10]", 20, 20);
    let draw = |content| {
        render(pdf(
            "[0 0 100 100]",
            "/Pattern << /P 5 0 R >>",
            content,
            &pattern,
            &[],
        ))
    };
    let original = draw("/Pattern cs /P scn 0 0 100 100 re f");
    assert_eq!(pixel(&original, 2, 97), [0, 255, 0, 255]);
    assert_eq!(pixel(&original, 12, 97)[3], 0);
    // The path compensates the page CTM so all three fills cover exactly the same area.
    // The 13/7 translation is deliberately not a multiple of either pattern step.
    for content in [
        "q 1 0 0 1 13 7 cm /Pattern cs /P scn -13 -7 100 100 re f Q",
        "/Pattern cs /P scn q 1 0 0 1 13 7 cm -13 -7 100 100 re f Q",
    ] {
        assert_eq!(
            draw(content).data,
            original.data,
            "the page's initial space anchors the pattern, independently of selection and painting CTM"
        );
    }
}

#[test]
fn a_patterned_glyph_stroke_contains_cell_gaps_not_just_the_cells_colour() {
    let pattern = type_one_pattern(1, "1 0 0 rg 0 0 5 5 re f", "", "[0 0 10 10]", 10, 10);
    let font = "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_owned();
    let image = render(pdf(
        "[0 0 100 100]",
        "/Pattern << /P 5 0 R >> /Font << /F 6 0 R >>",
        "BT /F 120 Tf 0 1 0 RG /Pattern CS /P SCN 3 w 1 Tr 5 10 Td (A) Tj ET",
        &pattern,
        &[font],
    ));
    let mut ink = 0;
    let mut forbidden_gap_ink = 0;
    for y in 0..100 {
        for x in 0..100 {
            let [r, g, b, alpha] = pixel(&image, x, y);
            if alpha > 128 {
                ink += 1;
                assert!(
                    r > 240 && g < 15 && b < 15,
                    "the cell's red replaces prior green"
                );
                if x % 10 >= 5 || (99 - y) % 10 >= 5 {
                    forbidden_gap_ink += 1;
                }
            }
        }
    }
    assert!(ink > 20, "the glyph's patterned stroke actually draws");
    assert_eq!(
        forbidden_gap_ink, 0,
        "a solid stroke in the cell's red would wrongly paint the known periodic gaps"
    );
}

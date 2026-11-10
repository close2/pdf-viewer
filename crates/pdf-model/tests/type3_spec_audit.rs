//! Discriminating PDF-input assertions for ISO 32000-2 §9.6.4 and §9.6.5.3.

#![expect(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "the test PDF is a small, deliberately valid fixture"
)]

use std::fmt::Write as _;

use pdf_render::{Command, Paint, Rasterizer, TargetSpec};
use pdf_syntax::Document;
use render_cpu::CpuRasterizer;

fn type3_page(font_matrix: &str, glyph: &str, content: &str) -> pdf_model::Interpretation {
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 300 300] \
         /Resources << /Font << /F3 5 0 R >> /ExtGState << /Half 8 0 R /NoKnock 9 0 R >> >> \
         /Contents 4 0 R >>\nendobj\n\
         4 0 obj\n<< /Length {} >>\nstream\n{content}\nendstream\nendobj\n\
         5 0 obj\n<< /Type /Font /Subtype /Type3 /FontBBox [0 0 750 750] \
         /FontMatrix {font_matrix} /CharProcs << /square 7 0 R >> /Encoding 6 0 R \
         /FirstChar 97 /LastChar 97 /Widths [1000] >>\nendobj\n\
         6 0 obj\n<< /Type /Encoding /Differences [97 /square] >>\nendobj\n\
         7 0 obj\n<< /Length {} >>\nstream\n{glyph}\nendstream\nendobj\n\
         8 0 obj\n<< /Type /ExtGState /ca 0.5 /CA 0.5 /TK true >>\nendobj\n\
         9 0 obj\n<< /Type /ExtGState /ca 0.5 /CA 0.5 /TK false >>\nendobj\n",
        content.len() + 1,
        glyph.len() + 1,
    );

    let mut bytes = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for object in body.split_inclusive("endobj\n") {
        offsets.push(bytes.len());
        bytes.push_str(object);
    }
    let xref_at = bytes.len();
    let size = offsets.len() + 1;
    let _ = writeln!(bytes, "xref\n0 {size}");
    bytes.push_str("0000000000 65535 f \n");
    for offset in offsets {
        let _ = writeln!(bytes, "{offset:010} 00000 n ");
    }
    let _ = write!(
        bytes,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n"
    );

    let document = Document::open(bytes.into_bytes()).expect("the PDF fixture is valid");
    assert!(
        !document.was_recovered(),
        "the generated PDF fixture is structurally valid"
    );
    let page = pdf_model::Pages::new(&document)
        .get(0)
        .expect("page one exists");
    pdf_model::interpret(&document, &page)
}

fn rgba_at(interpretation: &pdf_model::Interpretation, x: u32, y: u32) -> [u8; 4] {
    let list = &interpretation.display_list;
    let target = TargetSpec::for_page(list, 1.0, 1 << 20).expect("small page target");
    let raster = CpuRasterizer::new()
        .rasterize(list, target)
        .expect("the test page rasterises");
    let index = ((y * raster.width + x) * 4) as usize;
    [
        raster.data[index],
        raster.data[index + 1],
        raster.data[index + 2],
        raster.data[index + 3],
    ]
}

fn rgba_bits(colour: [f32; 4]) -> [u32; 4] {
    colour.map(f32::to_bits)
}

fn fills(interpretation: &pdf_model::Interpretation) -> Vec<(pdf_render::Transform, [f32; 4])> {
    interpretation
        .display_list
        .commands()
        .iter()
        .filter_map(|command| match command {
            Command::Fill {
                transform,
                paint: Paint::Solid(colour),
                ..
            } => Some((*transform, [colour.r, colour.g, colour.b, colour.a])),
            _ => None,
        })
        .collect()
}

/// Table 110: under a rotated `FontMatrix`, only the transformed width's horizontal component
/// advances the text position, so a glyph may rotate while the next code remains at the same
/// horizontal and vertical text position.
#[test]
fn a_rotated_font_matrix_keeps_the_type3_advance_horizontal() {
    let interpretation = type3_page(
        "[0 0.001 -0.002 0 0 0]",
        "1000 0 0 0 750 750 d1\n0 0 750 750 re f",
        "BT /F3 10 Tf 0 0 Td (aa) Tj ET",
    );
    assert!(
        interpretation.is_complete(),
        "the valid Type 3 glyphs should draw: {:?}",
        interpretation.unsupported
    );
    let rotated_fills = fills(&interpretation);
    assert_eq!(rotated_fills.len(), 2, "each shown code invokes its glyph");
    let [first, second] = [rotated_fills[0].0, rotated_fills[1].0];

    // The `FontMatrix` rotates and scales the glyph. Its 1000-unit width has a zero horizontal
    // component after that transform, and Table 110 explicitly keeps the resulting displacement
    // horizontal in text space instead of moving the next glyph vertically.
    assert!(
        first.c.abs() > 0.01,
        "the glyph's CTM is rotated: {first:?}"
    );
    assert!(
        (second.e - first.e).abs() < 1e-4 && (second.f - first.f).abs() < 1e-4,
        "a rotated width advances by its horizontal component only: {first:?} then {second:?}"
    );

    let control = type3_page(
        "[0.001 0 0 0.001 0 0]",
        "1000 0 0 0 750 750 d1\n0 0 750 750 re f",
        "BT /F3 10 Tf 0 0 Td (aa) Tj ET",
    );
    let control_fills = fills(&control);
    assert_eq!(control_fills.len(), 2);
    assert!(
        ((control_fills[1].0.e - control_fills[0].0.e) - 10.0).abs() < 1e-4,
        "the ordinary 1000-unit width advances one em at 10-point text size"
    );
}

/// §9.6.4 step c): a glyph's saved graphics state is restored after the description, including
/// changes made by a valid `d0` glyph that declares its own colour.
#[test]
fn a_d0_glyph_does_not_change_the_callers_graphics_state() {
    let interpretation = type3_page(
        "[0.001 0 0 0.001 0 0]",
        "1000 0 d0\n0 0 1 rg\n0 0 750 750 re f",
        "1 0 0 rg BT /F3 10 Tf 0 0 Td (a) Tj ET 0 0 4 4 re f",
    );
    assert!(
        interpretation.is_complete(),
        "a valid d0 glyph should draw: {:?}",
        interpretation.unsupported
    );
    let fills = fills(&interpretation);
    assert_eq!(
        fills.len(),
        2,
        "the glyph and subsequent page path both draw"
    );
    assert_eq!(
        rgba_bits(fills[0].1),
        rgba_bits([0.0, 0.0, 1.0, 1.0]),
        "d0 sets the glyph's blue"
    );
    assert_eq!(
        rgba_bits(fills[1].1),
        rgba_bits([1.0, 0.0, 0.0, 1.0]),
        "step c restores the red graphics state that invoked the glyph"
    );
}

/// §9.6.4 states that §9.3.8's text-knockout flag controls glyphs from any font type. This
/// valid two-glyph Type 3 text object overlaps at half opacity, the case where knockout changes
/// the compositing result.
#[test]
fn text_knockout_applies_to_overlapping_type3_glyphs() {
    let font_matrix = "[0.001 0 0 0.001 0 0]";
    let glyph = "1000 0 0 0 750 750 d1\n0 0 750 750 re f";
    let overlapping = type3_page(
        font_matrix,
        glyph,
        "/Half gs BT /F3 24 Tf 10 50 Td (a) Tj -8 0 Td (a) Tj ET",
    );
    let one_glyph = type3_page(
        font_matrix,
        glyph,
        "/Half gs BT /F3 24 Tf 10 50 Td (a) Tj ET",
    );
    let ordinary_overdraw = type3_page(
        font_matrix,
        glyph,
        "/NoKnock gs BT /F3 24 Tf 10 50 Td (a) Tj -8 0 Td (a) Tj ET",
    );
    for interpretation in [&overlapping, &one_glyph, &ordinary_overdraw] {
        assert!(
            interpretation.is_complete(),
            "the valid Type 3 fixture should be represented: {:?}",
            interpretation.unsupported
        );
    }

    // Device (15, 240) samples PDF coordinates (15.5, 59.5), inside both glyph squares;
    // (25, 240) lies inside only the first. Under /TK true the overlap matches one glyph,
    // while /TK false composites the same two half-alpha marks.
    assert_eq!(
        rgba_at(&overlapping, 15, 240),
        rgba_at(&one_glyph, 15, 240),
        "the later glyph knocks out the earlier glyph in their overlap"
    );
    let single_pixel = rgba_at(&one_glyph, 15, 240);
    let overdraw_pixel = rgba_at(&ordinary_overdraw, 15, 240);
    assert!(
        (i16::from(single_pixel[0]) - 128).abs() <= 4,
        "one half-opacity black glyph over the white page is near 128 gray: {single_pixel:?}"
    );
    assert!(
        (i16::from(overdraw_pixel[0]) - 64).abs() <= 4,
        "two ordinary half-opacity marks over white are near 64 gray: {overdraw_pixel:?}"
    );
    assert_ne!(
        single_pixel, overdraw_pixel,
        "ordinary alpha compositing keeps both overlapping marks"
    );
    assert_eq!(
        rgba_at(&overlapping, 25, 240),
        rgba_at(&ordinary_overdraw, 25, 240),
        "outside the overlap both models draw just one glyph"
    );
}

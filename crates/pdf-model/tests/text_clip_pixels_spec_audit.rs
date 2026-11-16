//! §9.2.3's public effect: glyph outlines can clip later general graphics operations.
//!
//! The same exact PDF is rendered once with text clipping and once with fill-only text. The
//! latter control paints the whole red rectangle; the former leaves a red sample in the
//! interior of embedded Liberation Sans H and the white backdrop outside the glyph outline.

#![expect(
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    reason = "a malformed exact-xref fixture or missing pixel must fail loudly"
)]

use std::io::Write as _;

use pdf_render::{Rasterizer, TargetSpec};
use pdf_syntax::Document;
use render_cpu::CpuRasterizer;

const SIDE: usize = 50;
const LIMIT: u64 = 1 << 20;

// Full checked-in LiberationSans font: SHA256
// f8ace1f892b2bd9dc1792ba7f097fa7588f84fed48321480e04de5390828221f.
// Independently read with fontTools: U+0048 maps to H/GID43, UPM2048,
// advance1479; its left stem is x168..359,y0..1409. At 40pt and (5,5),
// pixel(9,15) lies strictly inside x8.28125..12.0117,y5..32.5195.
const FONT: &[u8] = include_bytes!("../../../data/standard-fonts/LiberationSans-Regular.ttf");

fn stream(dictionary: &str, data: &[u8]) -> Vec<u8> {
    let mut out = format!("<< {dictionary} /Length {} >>\nstream\n", data.len()).into_bytes();
    out.extend_from_slice(data);
    out.extend_from_slice(b"\nendstream");
    out
}

fn pdf(clipping: bool) -> Vec<u8> {
    let render_mode = if clipping { "7 Tr" } else { "0 Tr" };
    let content = format!(
        "1 g 0 0 {SIDE} {SIDE} re f\nBT /F1 40 Tf 5 5 Td {render_mode} (H) Tj ET\n\
         1 0 0 rg 0 0 {SIDE} {SIDE} re f"
    );
    let objects = [
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        format!("<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {SIDE} {SIDE}] /Resources << /Font << /F1 5 0 R >> >> /Contents 4 0 R >>").into_bytes(),
        stream("", content.as_bytes()),
        b"<< /Type /Font /Subtype /TrueType /BaseFont /LiberationSans /FirstChar 72 /LastChar 72 /Widths [722] /Encoding /WinAnsiEncoding /FontDescriptor 6 0 R >>".to_vec(),
        b"<< /Type /FontDescriptor /FontName /LiberationSans /Flags 32 /FontBBox [-204 -304 1051 911] /ItalicAngle 0 /Ascent 729 /Descent -211 /CapHeight 688 /StemV 80 /FontFile2 7 0 R >>".to_vec(),
        stream(&format!("/Length1 {}", FONT.len()), FONT),
    ];
    let mut out = b"%PDF-1.7\n%\xE2\xE3\xCF\xD3\n".to_vec();
    let mut offsets = Vec::new();
    for (index, body) in objects.iter().enumerate() {
        offsets.push(out.len());
        writeln!(out, "{} 0 obj", index + 1).expect("Vec writes");
        out.extend_from_slice(body);
        out.extend_from_slice(b"\nendobj\n");
    }
    let xref = out.len();
    let size = offsets.len() + 1;
    write!(out, "xref\n0 {size}\n0000000000 65535 f \n").expect("Vec writes");
    for offset in offsets {
        writeln!(out, "{offset:010} 00000 n ").expect("Vec writes");
    }
    write!(
        out,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n"
    )
    .expect("Vec writes");
    out
}

fn pixel(clipping: bool, x: usize, y: usize) -> [u8; 4] {
    let document = Document::open(pdf(clipping)).expect("fixture parses");
    assert!(!document.was_recovered(), "fixture has exact xref offsets");
    let page = pdf_model::Pages::new(&document).get(0).expect("one page");
    let interpreted = pdf_model::interpret(&document, &page);
    assert!(
        interpreted.is_complete(),
        "fixture uses supported embedded TrueType glyph and graphics: {:?}",
        interpreted.unsupported
    );
    let target =
        TargetSpec::for_page(&interpreted.display_list, 1.0, LIMIT).expect("bounded 50x50 target");
    let raster = CpuRasterizer::new()
        .rasterize(&interpreted.display_list, target)
        .expect("page is supported by the CPU rasterizer");
    assert_eq!((raster.width, raster.height), (50, 50));
    // The raster origin is at the top left, while PDF user space has its origin at the bottom
    // left. These samples are intentionally away from glyph and rectangle antialias edges.
    let row = SIDE - 1 - y;
    let width = usize::try_from(raster.width).expect("bounded target width fits usize");
    let start = (row * width + x) * 4;
    raster.data[start..start + 4]
        .try_into()
        .expect("four bytes per pixel")
}

#[test]
fn glyph_clip_restricts_later_graphics_to_the_glyph_outline() {
    let inside = pixel(true, 9, 15);
    let outside = pixel(true, 40, 40);
    let unclip_control = pixel(false, 40, 40);

    assert_eq!((inside[3], outside[3], unclip_control[3]), (255, 255, 255));
    assert!(
        inside[0] > 245 && inside[1] < 10 && inside[2] < 10,
        "inside: {inside:?}"
    );
    assert!(
        outside[0] > 245 && outside[1] > 245 && outside[2] > 245,
        "outside: {outside:?}"
    );
    assert!(
        unclip_control[0] > 245 && unclip_control[1] < 10 && unclip_control[2] < 10,
        "without the text clip the rectangle reaches the same outside sample: {unclip_control:?}"
    );
}

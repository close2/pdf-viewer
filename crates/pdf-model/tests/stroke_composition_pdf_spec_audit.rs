//! Section11.6.2: one PDF stroke composites once, including overlapping subpaths.

#![expect(
    clippy::arithmetic_side_effects,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::expect_used,
    reason = "small bounded target and explicit fixture assertions"
)]

use std::io::Write as _;

use pdf_render::{Rasterizer, TargetSpec};
use pdf_syntax::Document;
use render_cpu::CpuRasterizer;

const PAGE: f32 = 80.0;
const SCALE: f32 = 2.0;

fn pdf(with_degenerate_subpath: bool) -> Vec<u8> {
    let mut content = String::from("1 g 0 0 80 80 re f\nq /GS gs 0 G 8 w 1 J 20 50 m 40 50 l");
    if with_degenerate_subpath {
        // This is a second subpath inside the existing one Stroke object, wholly contained by
        // the ordinary line at the pixel sampled below. No claim depends on whether it draws.
        content.push_str(" 30 50 m h");
    }
    content.push_str(" S Q\n");
    let mut out = b"%PDF-2.0\n%\xE2\xE3\xCF\xD3\n".to_vec();
    let mut offsets = vec![0_u64];
    let mut object = |number: u32, body: &[u8]| {
        offsets.push(out.len() as u64);
        out.extend_from_slice(format!("{number} 0 obj\n").as_bytes());
        out.extend_from_slice(body);
        out.extend_from_slice(b"\nendobj\n");
    };
    object(1, b"<< /Type /Catalog /Pages 2 0 R >>");
    object(2, b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>");
    object(
        3,
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 80 80] /Resources << /ExtGState << /GS 5 0 R >> >> /Contents 4 0 R >>",
    );
    let mut stream = format!("<< /Length {} >>\nstream\n", content.len()).into_bytes();
    stream.extend_from_slice(content.as_bytes());
    stream.extend_from_slice(b"endstream");
    object(4, &stream);
    object(5, b"<< /Type /ExtGState /CA 0.5 >>");

    let xref = out.len();
    let size = offsets.len();
    let _ = write!(out, "xref\n0 {size}\n0000000000 65535 f \n");
    for offset in offsets.iter().skip(1) {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    let _ = write!(
        out,
        "trailer\n<< /Size {size} /Root 1 0 R /ID [<00112233445566778899AABBCCDDEEFF> <00112233445566778899AABBCCDDEEFF>] >>\nstartxref\n{xref}\n%%EOF\n"
    );
    out
}

fn draw(bytes: Vec<u8>) -> (u8, u8) {
    let document = Document::open(bytes).expect("the exact-xref PDF parses");
    assert!(!document.was_recovered(), "fixture must not rely on repair");
    let page = pdf_model::Pages::new(&document).get(0).expect("one page");
    let interpreted = pdf_model::interpret(&document, &page);
    assert!(
        interpreted.unsupported.is_empty(),
        "{:?}",
        interpreted.unsupported
    );
    // The PDF contains one S operation. Its representation may change as long as the
    // visible result preserves one-object compositing; do not constrain command lowering.
    let target = TargetSpec::for_page(&interpreted.display_list, SCALE, 1 << 20)
        .expect("bounded page target fits");
    let raster = CpuRasterizer::new()
        .rasterize(&interpreted.display_list, target)
        .expect("page content is supported");
    let column = (30.25 * SCALE) as usize;
    let row = ((PAGE - 50.25) * SCALE) as usize;
    let start = (row * raster.width as usize + column) * 4;
    (raster.data[start], raster.data[start + 3])
}

#[test]
fn one_pdf_stroke_object_does_not_apply_its_alpha_to_its_own_parts() {
    let (line_red, line_alpha) = draw(pdf(false));
    assert!((127..=128).contains(&line_red), "line control: {line_red}");
    assert_eq!(line_alpha, 255, "white backdrop stays opaque");

    let (same_object_red, same_object_alpha) = draw(pdf(true));
    assert!(
        (127..=128).contains(&same_object_red),
        "PDF-content consumer must composite one translucent Stroke once at its contained line interior: {same_object_red}; repeated composition gives about 64"
    );
    assert_eq!(same_object_alpha, 255);
}

//! Public pixel regression for §12.5.3 flags applied to a §12.5.5 appearance mapping.

#![expect(clippy::expect_used, clippy::arithmetic_side_effects)]

use std::fmt::Write as _;

use pdf_model::{Pages, content, view::ViewState};
use pdf_render::{Rasterizer, TargetSpec};
use pdf_syntax::Document;
use render_cpu::CpuRasterizer;

const BUDGET: u64 = 1 << 20;

fn assemble(body: &str) -> Vec<u8> {
    let mut bytes = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for object in body.split_inclusive("endobj\n") {
        offsets.push(bytes.len());
        bytes.push_str(object);
    }
    let xref = bytes.len();
    let size = offsets.len() + 1;
    let _ = writeln!(bytes, "xref\n0 {size}");
    bytes.push_str("0000000000 65535 f \n");
    for offset in offsets {
        let _ = writeln!(bytes, "{offset:010} 00000 n ");
    }
    let _ = write!(
        bytes,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n"
    );
    bytes.into_bytes()
}

fn fixture(flags: i64) -> Vec<u8> {
    // Rotate the nonzero BBox: transformed extent [10 10 40 30]. The painted
    // source left half becomes its bottom half [10 10 40 20], then maps onto
    // the bottom half of /Rect. Ignoring this Matrix instead paints the left half.
    let appearance = "0 0 0 rg 10 20 10 30 re f";
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Rotate 90 \
         /Resources << >> /Contents 4 0 R /Annots [5 0 R] >>\nendobj\n\
         4 0 obj\n<< /Length 1 >>\nstream\n\nendstream\nendobj\n\
         5 0 obj\n<< /Type /Annot /Subtype /Square /Rect [40 40 72 72] /F {flags} \
         /AP << /N 6 0 R >> >>\nendobj\n\
         6 0 obj\n<< /Type /XObject /Subtype /Form /BBox [10 20 30 50] \
         /Matrix [0 1 -1 0 60 0] /Length {} >>\nstream\n{appearance}\nendstream\nendobj\n",
        appearance.len() + 1
    );
    assemble(&body)
}

fn render(flags: i64) -> pdf_render::Raster {
    let document = Document::open(fixture(flags)).expect("valid exact-xref fixture");
    assert!(!document.was_recovered(), "xref offsets are exact");
    let page = Pages::new(&document).get(0).expect("page one");
    let mut state = ViewState::of(&document);
    state.set_magnification(Some(2.0));
    let interpreted = content::interpret_with(&document, &page, &state);
    assert!(
        interpreted.is_complete(),
        "supported appearance, no recovery: {:?}",
        interpreted.unsupported
    );
    let target =
        TargetSpec::for_page(&interpreted.display_list, 1.0, BUDGET).expect("bounded target");
    CpuRasterizer::new()
        .with_medium(pdf_render::Medium::NONE)
        .rasterize(&interpreted.display_list, target)
        .expect("rasterization")
}

fn extent(raster: &pdf_render::Raster) -> (u32, u32, u32, u32) {
    let mut points = raster
        .data
        .as_chunks::<4>()
        .0
        .iter()
        .enumerate()
        .filter_map(|(index, rgba)| {
            let index = u32::try_from(index).expect("small raster index");
            (rgba[3] != 0).then_some((
                index % raster.width,
                raster.height - 1 - index / raster.width,
            ))
        });
    let first = points.next().expect("painted pixels");
    points.fold(
        (first.0, first.1, first.0, first.1),
        |(l, t, r, b), (x, y)| (l.min(x), t.min(y), r.max(x), b.max(y)),
    )
}

#[test]
fn nozoom_and_norotate_apply_to_the_mapped_appearance_as_a_whole() {
    // The mapped bottom half [40,72] x [40,56] turns clockwise with /Rotate.
    assert_eq!(extent(&render(4)), (40, 28, 55, 59));

    // Both flags at 2x and /Rotate 90. Pivot about /Rect's upper-left (40,72):
    // bottom half -> CCW [56,72] x [72,104] -> half scale [48,56] x [72,88]
    // -> page clockwise [72,88] x [44,52]. Expected bounds are pixel centers.
    assert_eq!(extent(&render(28)), (72, 44, 87, 51));
}

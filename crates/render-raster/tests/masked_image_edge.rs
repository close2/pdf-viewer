//! A soft-masked image's edge, drawn below its own resolution, against the CPU oracle.
//!
//! A soft mask makes a sample transparent without saying anything about the colour the base
//! image holds there, and an encoder is free to leave anything under it. A transparent
//! sample contributes no colour to the page, so a reduction or a filter that reads its
//! colour at full weight has drawn something the file does not contain. The oracle filters
//! premultiplied samples; this holds the device to the same answer at a sweep of
//! magnifications, because the colour under the mask only showed where a device pixel
//! gathered two samples or more (ADR 1287).

#![expect(
    clippy::expect_used,
    clippy::panic,
    reason = "test code: an explanatory panic is the intended failure mode"
)]

use pdf_render::{Rasterizer, TargetSpec};
use render_cpu::CpuRasterizer;
use render_raster::QuorraRasterizer;

/// Source samples per side: enough that every magnification below reduces the image.
const SIDE: usize = 96;

/// Builds a one-page file: a white image whose colour is **black** wherever its `/SMask`
/// is zero, drawn at 40 × 40 points. The black is what makes a leak visible — a correct
/// renderer never shows it, because the mask says those samples are transparent.
fn masked_image_file() -> Vec<u8> {
    // In half-samples, so the disc's centre (47.5, 47.5) is the whole number 95 and its
    // radius of forty samples is 80.
    let inside = |x: usize, y: usize| {
        let (dx, dy) = (
            x.saturating_mul(2).abs_diff(95),
            y.saturating_mul(2).abs_diff(95),
        );
        dx.saturating_mul(dx).saturating_add(dy.saturating_mul(dy)) < 80 * 80
    };
    let mut colour = Vec::with_capacity(SIDE * SIDE * 3);
    let mut mask = Vec::with_capacity(SIDE * SIDE);
    for y in 0..SIDE {
        for x in 0..SIDE {
            let opaque = inside(x, y);
            colour.extend_from_slice(if opaque { &[255, 255, 255] } else { &[0, 0, 0] });
            mask.push(if opaque { 255 } else { 0 });
        }
    }
    let content = b"q 40 0 0 40 10 10 cm /I Do Q".to_vec();
    let stream = |dict: String, data: &[u8]| {
        let mut out = format!("{dict} /Length {} >>\nstream\n", data.len()).into_bytes();
        out.extend_from_slice(data);
        out.extend_from_slice(b"\nendstream");
        out
    };
    let objects: Vec<Vec<u8>> = vec![
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 60 60] \
           /Resources << /XObject << /I 4 0 R >> >> /Contents 6 0 R >>"
            .to_vec(),
        stream(
            format!(
                "<< /Type /XObject /Subtype /Image /Width {SIDE} /Height {SIDE} \
                 /ColorSpace /DeviceRGB /BitsPerComponent 8 /SMask 5 0 R"
            ),
            &colour,
        ),
        stream(
            format!(
                "<< /Type /XObject /Subtype /Image /Width {SIDE} /Height {SIDE} \
                 /ColorSpace /DeviceGray /BitsPerComponent 8"
            ),
            &mask,
        ),
        stream("<<".to_owned(), &content),
    ];
    let mut file = b"%PDF-1.7\n".to_vec();
    let mut offsets = Vec::new();
    for (number, body) in (1_usize..).zip(&objects) {
        offsets.push(file.len());
        file.extend_from_slice(format!("{number} 0 obj\n").as_bytes());
        file.extend_from_slice(body);
        file.extend_from_slice(b"\nendobj\n");
    }
    let xref = file.len();
    let size = objects.len().saturating_add(1);
    file.extend_from_slice(format!("xref\n0 {size}\n0000000000 65535 f \n").as_bytes());
    for offset in offsets {
        file.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    file.extend_from_slice(
        format!("trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n").as_bytes(),
    );
    file
}

/// The largest per-channel difference between the two backends at one scale.
fn worst_difference(
    list: &pdf_render::DisplayList,
    device: &mut QuorraRasterizer,
    scale: f32,
) -> u8 {
    let target = TargetSpec::for_page(list, scale, u64::MAX).expect("the target fits");
    let oracle = CpuRasterizer::new()
        .rasterize(list, target)
        .expect("the CPU oracle draws the page");
    let ours = device
        .rasterize(list, target)
        .unwrap_or_else(|e| panic!("raster refused the page at {scale}: {e}"));
    oracle
        .data
        .iter()
        .zip(ours.data.iter())
        .map(|(a, b)| a.abs_diff(*b))
        .max()
        .unwrap_or(0)
}

#[test]
fn a_masked_images_hidden_colour_does_not_reach_its_edge_at_any_reduction() {
    let document = pdf_syntax::Document::open(masked_image_file()).expect("the fixture is a PDF");
    let pages = pdf_model::Pages::new(&document);
    let page = pages.get(0).expect("the fixture has a page");
    let list = pdf_model::content::interpret(&document, &page).display_list;
    let mut device = QuorraRasterizer::new_headless().unwrap_or_else(|e| {
        panic!("no adapter for raster: {e} — these tests do not skip (ADR 0004)")
    });
    // 40 points is 40 × scale device pixels over 96 samples: factors one to three, at
    // ratios that are and are not whole numbers, which is where a filter reads between
    // texels at all.
    for scale in [0.9_f32, 1.2, 1.35, 1.5, 1.65, 1.8, 2.1] {
        let worst = worst_difference(&list, &mut device, scale);
        assert!(
            worst <= 4,
            "at scale {scale} the device differs from the oracle by {worst} levels along the \
             mask's edge: a transparent sample's colour was filtered in"
        );
    }
}

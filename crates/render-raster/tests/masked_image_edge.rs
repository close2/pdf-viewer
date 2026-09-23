//! A soft-masked image's edge, drawn below its own resolution, against the CPU oracle.
//!
//! A soft mask makes a sample transparent without saying anything about the colour the base
//! image holds there, and an encoder is free to leave anything under it. A transparent
//! sample contributes no colour to the page, so a reduction or a filter that reads its
//! colour at full weight has drawn something the file does not contain. The oracle filters
//! premultiplied samples; this holds the device to the same answer at a sweep of
//! magnifications, because the colour under the mask only showed where a device pixel
//! gathered two samples or more (ADR 1287).
//!
//! The second half is an opaque image's sample grid held against the placement ISO 32000-2
//! §10.7.4 and §8.9.4 put it at, where a pixel centre lands on the edge two samples share —
//! the image's origin on a half pixel — at one device pixel per sample, magnified, and
//! reduced onto one sample per pixel (ADR 1302).

#![expect(
    clippy::expect_used,
    clippy::panic,
    clippy::arithmetic_side_effects,
    reason = "test code: an explanatory panic is the intended failure mode, and the arithmetic \
              is on the fixture's own sides and pixel indices, all far below overflow"
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
    image_file(SIDE, &colour, Some(&mask))
}

/// The colour of sample `(x, y)` of [`opaque_image_file`]'s image: every neighbour differs
/// from it in every channel, so a pixel that reads the wrong sample is never within a
/// tolerance of the right one.
fn opaque_sample(x: usize, y: usize) -> [u8; 3] {
    let level = x.wrapping_mul(37).wrapping_add(y.wrapping_mul(91)) % 256;
    let level = u8::try_from(level).expect("reduced modulo 256");
    [level, level.wrapping_mul(3), 255 - level]
}

/// Builds a one-page file: an opaque `side` × `side` image of [`opaque_sample`]'s colours
/// and no mask at all, drawn at 40 × 40 points.
fn opaque_image_file(side: usize) -> Vec<u8> {
    let mut colour = Vec::with_capacity(side * side * 3);
    for y in 0..side {
        for x in 0..side {
            colour.extend_from_slice(&opaque_sample(x, y));
        }
    }
    image_file(side, &colour, None)
}

/// A one-page 60 × 60 point file drawing a `side` × `side` `DeviceRGB` image, with `mask`
/// as its `/SMask` where one is given, at 40 × 40 points from (10, 10).
fn image_file(side: usize, colour: &[u8], mask: Option<&[u8]>) -> Vec<u8> {
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
                "<< /Type /XObject /Subtype /Image /Width {side} /Height {side} \
                 /ColorSpace /DeviceRGB /BitsPerComponent 8{}",
                if mask.is_some() { " /SMask 5 0 R" } else { "" }
            ),
            colour,
        ),
        stream(
            format!(
                "<< /Type /XObject /Subtype /Image /Width {side} /Height {side} \
                 /ColorSpace /DeviceGray /BitsPerComponent 8"
            ),
            mask.unwrap_or(&[]),
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
    let entries = objects.len().saturating_add(1);
    file.extend_from_slice(format!("xref\n0 {entries}\n0000000000 65535 f \n").as_bytes());
    for offset in offsets {
        file.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    file.extend_from_slice(
        format!("trailer\n<< /Size {entries} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n")
            .as_bytes(),
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

/// Draws `file`'s page at `scale` on both backends.
fn both(file: Vec<u8>, scale: f32) -> (pdf_render::Raster, pdf_render::Raster) {
    let document = pdf_syntax::Document::open(file).expect("the fixture is a PDF");
    let pages = pdf_model::Pages::new(&document);
    let page = pages.get(0).expect("the fixture has a page");
    let list = pdf_model::content::interpret(&document, &page).display_list;
    let target = TargetSpec::for_page(&list, scale, u64::MAX).expect("the target fits");
    let oracle = CpuRasterizer::new()
        .rasterize(&list, target)
        .expect("the CPU oracle draws the page");
    let mut device = QuorraRasterizer::new_headless().unwrap_or_else(|e| {
        panic!("no adapter for raster: {e} — these tests do not skip (ADR 0004)")
    });
    let ours = device
        .rasterize(&list, target)
        .unwrap_or_else(|e| panic!("raster refused the page at {scale}: {e}"));
    (oracle, ours)
}

/// An opaque image's sample grid, placed where the standard puts it, on both backends.
///
/// §10.7.4 colours a pixel of a sampled image from one point: "[t]he position of the centre
/// of such a pixel -in other words, the point whose coordinate values have fractional parts
/// of one-half -shall be mapped back into source space to determine how to colour the
/// pixel", and "[t]here shall not be averaging over the pixel area". §8.9.4 gives source space one unit per
/// sample, "[t]he upper-left corner of the first sample is at coordinates (0, 0)", and the
/// sample read is the one whose square holds the point — the floor, by §10.7.4's own
/// convention for the pixels it names by their minimum corner.
///
/// Each rung puts the image's origin on a half pixel, so every pixel centre lands on the edge
/// between two samples in at least one axis; the sample is then `floor((2i + 1 − 2o) × side ⁄
/// 2E)` for pixel `i`, with the origin `o` and extent `E` in device pixels — written in
/// doubled units so that the arithmetic is exact. Only pixels whose squares lie wholly inside
/// the image's rectangle are asked: the edge pixels are §10.7.4's anti-aliasing departure,
/// which both backends take alike.
///
/// Scale 3.75 puts the 40 × 40 point image on 150 × 150 pixels from (37.5, 37.5): 150 samples
/// is one per pixel, 100 is magnified one and a half times, and 42 at scale 1.05 is one per
/// pixel from (10.5, 10.5). The device filtered the first and third into a four-sample mean
/// and broke the second's ties by float error — up to 239 levels from both the oracle and
/// the clause, opaque as these images are (ADR 1302).
#[test]
fn an_opaque_images_samples_land_where_the_clause_places_them() {
    // (samples per side, scale, twice the origin, twice the extent), all in device pixels.
    for (side, scale, origin2, extent2) in [
        (150_usize, 3.75_f32, 75_usize, 300_usize),
        (100, 3.75, 75, 300),
        (42, 1.05, 21, 84),
    ] {
        let (oracle, ours) = both(opaque_image_file(side), scale);
        let width = oracle.width as usize;
        // The first and one-past-last whole pixels inside the image's rectangle.
        // The far edge is at `(origin2 + extent2) ⁄ 2` device pixels, which is the midpoint of
        // the two doubled numbers.
        let (first, end) = (origin2.div_ceil(2), origin2.midpoint(extent2));
        let sample = |pixel: usize| (2 * pixel + 1 - origin2) * side / extent2;
        for (name, raster) in [("the oracle", &oracle), ("the device", &ours)] {
            for y in first..end {
                for x in first..end {
                    let at = (y * width + x) * 4;
                    let drawn = &raster.data[at..at + 3];
                    let expected = opaque_sample(sample(x), sample(y));
                    let worst = drawn
                        .iter()
                        .zip(expected)
                        .map(|(a, b)| a.abs_diff(b))
                        .max()
                        .unwrap_or(0);
                    assert!(
                        worst <= 1,
                        "{side} samples at scale {scale}: {name} drew {drawn:?} at pixel \
                         ({x}, {y}), where §10.7.4's centre maps back into sample ({}, {}), \
                         {expected:?}",
                        sample(x),
                        sample(y)
                    );
                }
            }
        }
    }
}

/// The same grid reduced onto one sample per pixel: 300 samples on 150 pixels is a reduction
/// by exactly two, and the reduced grid is then drawn at one device pixel per sample, so the
/// clause's point sample decides it on both backends. What each reduced sample holds is the
/// caller's documented area-averaging departure (ADR 0025) rather than a formula of the
/// clause's, so this rung is held against the oracle rather than a closed form.
#[test]
fn a_grid_reduced_onto_one_sample_per_pixel_is_not_filtered_again() {
    let (oracle, ours) = both(opaque_image_file(300), 3.75);
    let worst = oracle
        .data
        .iter()
        .zip(ours.data.iter())
        .map(|(a, b)| a.abs_diff(*b))
        .max()
        .unwrap_or(0);
    assert!(
        worst <= 2,
        "a 2:1 reduction drawn from a half-pixel origin differs from the oracle by {worst} \
         levels: the reduced grid was filtered, or read a sample to one side"
    );
}

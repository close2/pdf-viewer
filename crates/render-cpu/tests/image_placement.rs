//! Which part of an image lands where on the page.
//!
//! # What the expected values come from
//!
//! ISO 32000-2 §8.9.5.2 defines the image coordinate system: an image is mapped to the
//! **unit square** of user space, with its first sample row at the *top* of that square
//! and its first column at the left. Everything else — position, size, rotation, the flip
//! between PDF's y-up space and a raster's y-down rows — is carried by the transform in
//! force, so the whole of an image's placement is that one sentence plus a matrix.
//!
//! These scenes therefore use an image whose four samples are four distinguishable
//! colours. Where each of them lands says whether the mapping is right, and each way of
//! getting it wrong shows up as a different permutation: a missing vertical flip swaps
//! top for bottom, a transposed matrix swaps the diagonal, and a device transform applied
//! twice puts a single sample across the whole square.
//!
//! # Why the scale varies
//!
//! A paint's transform is read by `tiny-skia` in the space of the path it fills, and
//! composing the page-to-device transform into it as well applies that transform twice.
//! At a scale of 1.0 the second application partly cancels, so a case that only ever runs
//! at 1.0 can pass while every real page is wrong. See `shading_placement.rs`, which
//! makes the same argument for gradients.

#![expect(
    clippy::arithmetic_side_effects,
    reason = "test code: the arithmetic is on literal page dimensions that cannot overflow"
)]
#![expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "the casts are page coordinates under 200, in range by construction"
)]

use pdf_render::{
    BlendMode, Command, DisplayList, Image, Point, Raster, Rasterizer, Size, TargetSpec, Transform,
};
use render_cpu::CpuRasterizer;

/// Pixel budget for a target; far above anything these tests request.
const GENEROUS: u64 = 1 << 30;

/// Page side, in PDF units.
const PAGE: f32 = 100.0;

const RED: [u8; 4] = [255, 0, 0, 255];
const GREEN: [u8; 4] = [0, 255, 0, 255];
const BLUE: [u8; 4] = [0, 0, 255, 255];
const YELLOW: [u8; 4] = [255, 255, 0, 255];

/// How far a sampled channel may sit from the sample it is reading.
///
/// The rasteriser filters the image bilinearly, and a device pixel centre lands half a
/// pixel from the exact centre of a magnified sample, so a reading is pulled a few levels
/// towards its neighbour. Far below the 255 that separates these four colours from one
/// another, which is all these assertions need to tell apart.
const TOLERANCE: i32 = 12;

/// A 2×2 image: red and green on the first row, blue and yellow on the second.
fn quadrants() -> Image {
    let mut data = Vec::with_capacity(16);
    for sample in [RED, GREEN, BLUE, YELLOW] {
        data.extend_from_slice(&sample);
    }
    Image {
        width: 2,
        height: 2,
        data: data.into(),
        // §8.9.5.3's default: these tests magnify four samples across a page, so the
        // sampler draws four flat rectangles and a placement error moves an edge rather
        // than shading it.
        interpolate: false,
    }
}

/// The image mapped onto the unit square, carried by `transform` onto the page.
fn image_page(transform: Transform) -> DisplayList {
    let mut list = DisplayList::new(Size::new(PAGE, PAGE));
    list.push(Command::Image {
        image: quadrants(),
        transform,
        alpha: 1.0,
        clip: None,
        blend: BlendMode::Normal,
    });
    list
}

/// Reads the pixel at a *page* point.
fn at_page_point(raster: &Raster, page: Point, scale: f32) -> [u8; 4] {
    let x = (page.x * scale) as u32;
    // Device rows count down from the page's top edge.
    let y = ((PAGE - page.y) * scale) as u32;
    assert!(x < raster.width && y < raster.height, "inside the raster");
    let index = ((y as usize) * (raster.width as usize) + (x as usize)) * 4;
    let p = &raster.data[index..index + 4];
    [p[0], p[1], p[2], p[3]]
}

fn assert_close(what: &str, actual: [u8; 4], wanted: [u8; 4]) {
    let far = actual
        .iter()
        .zip(wanted.iter())
        .any(|(a, b)| (i32::from(*a) - i32::from(*b)).abs() > TOLERANCE);
    assert!(!far, "{what}: got {actual:?}, expected {wanted:?}");
}

/// The image covers the whole page, so each sample owns one quarter of it.
///
/// The first row of samples belongs at the *top* of the unit square, which is the high-y
/// end in page space and the low-y end in device rows.
#[test]
fn the_first_row_of_an_image_lands_at_the_top_of_the_unit_square() {
    let list = image_page(Transform::scale(PAGE, PAGE));

    for scale in [1.0f32, 2.0, 0.5] {
        let target = TargetSpec::for_page(&list, scale, GENEROUS).expect("valid target");
        let raster = CpuRasterizer::new()
            .rasterize(&list, target)
            .expect("an image is supported");

        for (corner, wanted, name) in [
            (Point::new(25.0, 75.0), RED, "first row, first column"),
            (Point::new(75.0, 75.0), GREEN, "first row, second column"),
            (Point::new(25.0, 25.0), BLUE, "second row, first column"),
            (Point::new(75.0, 25.0), YELLOW, "second row, second column"),
        ] {
            assert_close(
                &format!("{name} at scale {scale}"),
                at_page_point(&raster, corner, scale),
                wanted,
            );
        }
    }
}

/// The unit square placed away from the origin, at a size that is not the page's.
///
/// A transform applied twice leaves an image that is still square and still coloured from
/// the same four samples; what it does not survive is being asked where it *is*.
#[test]
fn an_image_lands_where_its_transform_puts_it() {
    // A 40×40 square with its lower-left corner at page (20, 30).
    let placed = Transform::scale(40.0, 40.0).then(Transform::translate(20.0, 30.0));
    let list = image_page(placed);

    for scale in [1.0f32, 2.0] {
        let target = TargetSpec::for_page(&list, scale, GENEROUS).expect("valid target");
        let raster = CpuRasterizer::new()
            .rasterize(&list, target)
            .expect("an image is supported");

        for (point, wanted, name) in [
            (Point::new(30.0, 60.0), RED, "first row, first column"),
            (Point::new(50.0, 60.0), GREEN, "first row, second column"),
            (Point::new(30.0, 40.0), BLUE, "second row, first column"),
            (Point::new(50.0, 40.0), YELLOW, "second row, second column"),
            // Outside the square the page is untouched.
            (Point::new(10.0, 10.0), [255, 255, 255, 255], "clear of it"),
            (Point::new(90.0, 90.0), [255, 255, 255, 255], "clear of it"),
        ] {
            assert_close(
                &format!("{name} at scale {scale}"),
                at_page_point(&raster, point, scale),
                wanted,
            );
        }
    }
}

/// A rotated placement, which a scale and a translation together cannot distinguish.
#[test]
fn an_image_turns_with_its_transform() {
    // A quarter turn anticlockwise about the origin, then back onto the page: the unit
    // square's first row runs down the left-hand edge instead of across the top.
    let turned = Transform::new(0.0, 1.0, -1.0, 0.0, 0.0, 0.0)
        .then(Transform::scale(PAGE, PAGE))
        .then(Transform::translate(PAGE, 0.0));
    let list = image_page(turned);
    let target = TargetSpec::for_page(&list, 1.0, GENEROUS).expect("valid target");
    let raster = CpuRasterizer::new()
        .rasterize(&list, target)
        .expect("an image is supported");

    // The composed map sends a unit-square point (u, v) to page (100 - 100v, 100u), so
    // the first row — which sits at v ≈ 0.75 — lands down the page's *left* edge, and the
    // two columns run up it. Red at (0.25, 0.75) therefore lands at page (25, 25).
    for (point, wanted, name) in [
        (Point::new(25.0, 25.0), RED, "first sample"),
        (Point::new(25.0, 75.0), GREEN, "second sample"),
        (Point::new(75.0, 25.0), BLUE, "third sample"),
        (Point::new(75.0, 75.0), YELLOW, "fourth sample"),
    ] {
        assert_close(name, at_page_point(&raster, point, 1.0), wanted);
    }
}

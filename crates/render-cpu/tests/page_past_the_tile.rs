//! A page drawn more than 8191 pixels wide or tall is drawn as the window of it is — ISO 32000-2
//! §10.7.4, ADR 2058.
//!
//! # What the expected values come from
//!
//! §10.7.4 fixes a pixel as the half-open square `[i, i+1) × [j, j+1)` on integer device
//! coordinates, and this backend paints a partly covered pixel at the area the shape covers there
//! (the ledger row's departure (1)). That area is a function of the shape's position against the
//! grid and of nothing else, so the same shape moved by a whole number of pixels covers the same
//! areas of the moved pixels — and subtracting a whole number from these coordinates is exact in
//! `f32`, so the moved shape is the same shape. A window of a page is that move: the page's marks
//! under its transform with the window's corner taken off. The page drawn whole and its window
//! therefore owe the same pixels, and no renderer enters the expected value.
//!
//! # What broke it, and why both halves are here
//!
//! `tiny-skia`'s fixed-point arithmetic stops at 8191, and two of its consequences reached a page
//! past that size while sparing the window of it, which sits nearer the origin: a mark whose device
//! extent passed 8191 was drawn without anti-aliasing at all, and every rectangle on a surface
//! past 8191 lost its closed form to the library's supersampled converter. On the owner's witness
//! at 9.3 times the fit the second put one glyph's bar edge at 0 on the page and 58 in the window.

#![expect(
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "test code: a panic with a message is the intended failure mode, and the arithmetic \
              is over rasters this file sized itself"
)]

use std::sync::Arc;

use pdf_render::{
    BlendMode, Color, Command, DisplayList, FillRule, Paint, Path, PathCommand, Point, Raster,
    Rasterizer, Size, Stroke, TargetSpec, Transform,
};
use render_cpu::CpuRasterizer;

/// The width of the page every scene is drawn on: past 8191, as the witness's is.
const WIDE: u32 = 11_176;

/// The height of every surface here.
const TALL: u32 = 64;

/// How far the window's corner lies from the page's, in whole pixels.
const CORNER: f32 = 8_000.0;

/// A triangle with oblique edges and its corner at `(x, y)`, every vertex stated as `x` or `y`
/// plus a constant, so that moving `x` by a whole number moves every vertex by it exactly.
fn triangle(x: f32, y: f32) -> Path {
    let mut path = Path::new();
    path.push(PathCommand::MoveTo(Point::new(x, y)));
    path.push(PathCommand::LineTo(Point::new(x + 5.3, y + 0.7)));
    path.push(PathCommand::LineTo(Point::new(x + 1.9, y + 4.6)));
    path.push(PathCommand::Close);
    path
}

/// A closed axis-aligned rectangle.
fn rectangle(x0: f32, y0: f32, x1: f32, y1: f32) -> Path {
    let mut path = Path::new();
    path.push(PathCommand::MoveTo(Point::new(x0, y0)));
    path.push(PathCommand::LineTo(Point::new(x1, y0)));
    path.push(PathCommand::LineTo(Point::new(x1, y1)));
    path.push(PathCommand::LineTo(Point::new(x0, y1)));
    path.push(PathCommand::Close);
    path
}

/// One black mark on a white surface `width` pixels wide, at one device pixel per unit, drawn in
/// one strip.
fn drawn(mark: Command, width: u32) -> Raster {
    let mut list = DisplayList::new(Size::new(width as f32, TALL as f32));
    list.push(mark);
    let target = TargetSpec {
        width,
        height: TALL,
        transform: Transform::IDENTITY,
    };
    CpuRasterizer::new()
        .with_strips(1)
        .rasterize(&list, target)
        .expect("a solid mark draws")
}

fn filled(path: Path) -> Command {
    Command::Fill {
        path: Arc::new(path),
        transform: Transform::IDENTITY,
        fill_rule: FillRule::NonZero,
        paint: Paint::Solid(Color::BLACK),
        clip: None,
        mask: None,
        blend: BlendMode::Normal,
    }
}

fn stroked(path: Path) -> Command {
    Command::Stroke {
        path: Arc::new(path),
        transform: Transform::IDENTITY,
        stroke: Stroke {
            width: 1.7,
            ..Stroke::default()
        },
        paint: Paint::Solid(Color::BLACK),
        clip: None,
        mask: None,
        blend: BlendMode::Normal,
    }
}

/// The red channel of pixel `(x, y)`, which on black over white is the complement of coverage.
fn level(raster: &Raster, x: u32, y: u32) -> u8 {
    raster.data[((y * raster.width + x) * 4) as usize]
}

/// The pixels of `page` from column `from` on that differ from `window`'s, which starts there.
fn differing(page: &Raster, window: &Raster, from: u32) -> Vec<(u32, u32, u8, u8)> {
    let mut found = Vec::new();
    for y in 0..TALL {
        for x in 0..window.width {
            let (a, b) = (level(page, from + x, y), level(window, x, y));
            if a != b {
                found.push((x, y, a, b));
            }
        }
    }
    found
}

/// A mark past 8191 device pixels is measured where it lies, as the same mark moved near the
/// origin is: before ADR 2058 the page drew it without anti-aliasing, every boundary pixel 0 or
/// 255.
#[test]
fn a_mark_past_the_range_is_the_same_mark_moved_into_it() {
    let (x, y) = (9_031.3_f32, 20.27_f32);
    for (what, page, window) in [
        (
            "a fill",
            filled(triangle(x, y)),
            filled(triangle(x - CORNER, y)),
        ),
        (
            "a stroke",
            stroked(triangle(x, y)),
            stroked(triangle(x - CORNER, y)),
        ),
    ] {
        let page = drawn(page, WIDE);
        let window = drawn(window, WIDE - CORNER as u32);
        let found = differing(&page, &window, CORNER as u32);
        assert!(
            found.is_empty(),
            "{what} past the range differs from itself moved into it at {found:?}"
        );
    }
}

/// A rectangle on a surface past 8191 pixels is its closed form, §10.7.4's product of two
/// overlaps, to the rounding of one level — where the library's rectangle call would have handed
/// it to the supersampled converter and rounded each edge to a quarter of a pixel.
#[test]
fn a_rectangle_on_a_surface_past_the_tile_is_its_closed_form() {
    let corners = (6_231.3_f32, 20.227_f32, 6_236.8_f32, 23.4_f32);
    let page = drawn(
        filled(rectangle(corners.0, corners.1, corners.2, corners.3)),
        WIDE,
    );
    let area = pdf_render::Rect::from_corners(
        Point::new(corners.0, corners.1),
        Point::new(corners.2, corners.3),
    );
    for y in 0..TALL {
        for x in 6_225..6_245_u32 {
            let wanted = pdf_render::rectangle_coverage(area, x as f32, y as f32);
            let painted = f32::from(255 - level(&page, x, y)) / 255.0;
            assert!(
                (painted - wanted).abs() <= 1.0 / 255.0,
                "pixel ({x}, {y}) is painted at {painted}, its area is {wanted}"
            );
        }
    }
}

/// And the window of it, drawn on a surface the library's rectangle converter reaches, is the
/// same closed form in that converter's 8.8 fixed point: the two agree to a level of 255, which is
/// what remains between a page and its window here (ADR 2058).
#[test]
fn a_rectangle_and_its_window_agree_to_a_level() {
    let (x0, y0, x1, y1) = (6_231.3_f32, 20.227_f32, 6_236.8_f32, 23.4_f32);
    let page = drawn(filled(rectangle(x0, y0, x1, y1)), WIDE);
    let window = drawn(
        filled(rectangle(x0 - CORNER / 2.0, y0, x1 - CORNER / 2.0, y1)),
        WIDE - CORNER as u32,
    );
    let found = differing(&page, &window, (CORNER / 2.0) as u32);
    assert!(
        found.iter().all(|&(_, _, a, b)| a.abs_diff(b) <= 1),
        "the page and its window differ by more than a level at {found:?}"
    );
}

/// A spike from the page out to `reach` device pixels, which is what a damaged content stream
/// states: ADR 0269's two witnesses reach 10²⁵ on a 368 × 542 page.
fn spike(reach: f32) -> Path {
    let mut path = Path::new();
    path.push(PathCommand::MoveTo(Point::new(10.0, 10.0)));
    path.push(PathCommand::LineTo(Point::new(reach, reach * 200.0)));
    path.push(PathCommand::LineTo(Point::new(30.0, 10.0)));
    path.push(PathCommand::Close);
    path
}

/// A mark whose coordinates are past what `f32` states to a measured coverage is drawn as the
/// library's aliased converter draws it, as it was before ADR 2058: `render_cpu::area` measures a
/// mark past 8191 only while its coordinates keep the precision its integral needs. Drawn with
/// anti-aliasing asked for and with it withdrawn, the two are the same page. Before the bound,
/// `4605705.pdf` lost a third of its ink to the integral of a spike at 10²⁵.
#[test]
fn a_mark_past_what_f32_measures_is_drawn_as_the_library_draws_it() {
    for reach in [1.0e7_f32, 1.0e25] {
        let mut list = DisplayList::new(Size::new(368.0, TALL as f32));
        list.push(filled(spike(reach)));
        let target = TargetSpec {
            width: 368,
            height: TALL,
            transform: Transform::IDENTITY,
        };
        let asked = CpuRasterizer::new()
            .with_strips(1)
            .rasterize(&list, target)
            .expect("the spike draws");
        let aliased = CpuRasterizer::new()
            .with_strips(1)
            .with_anti_alias(false)
            .rasterize(&list, target)
            .expect("the spike draws aliased");
        let differing = asked
            .data
            .chunks_exact(4)
            .zip(aliased.data.chunks_exact(4))
            .filter(|(a, b)| a != b)
            .count();
        assert_eq!(
            differing, 0,
            "the spike to {reach} differs from the library's aliased drawing at {differing} pixels"
        );
    }
}

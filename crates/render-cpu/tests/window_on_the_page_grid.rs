//! A window of a page is the page's pixels there — ISO 32000-2 §10.7.4, ADR 2132.
//!
//! # What the expected values come from
//!
//! §10.7.4 identifies a pixel by its device coordinates alone:
//!
//! > Pixel boundaries always fall on integer coordinates in device space.
//!
//! A window of a page (ADR 1930) is a rectangle of the page's own device grid whose corner is a
//! whole number of its pixels, so every pixel of the window is a pixel of the page and owes what
//! the page drawn whole has there. No renderer enters the expected value: each test draws the
//! page whole and compares the window with the same rectangle of it, byte for byte.
//!
//! # What each test holds
//!
//! Each is one of the places a window's corner reaches a mark's geometry, and each would misplace
//! or redraw the mark if the corner were taken off in another place than the one
//! `CpuRasterizer::rasterize_window` takes it off in: the converter a rectangle takes on a page past
//! `tiny-skia`'s 8191-pixel range (ADR 2116), the sample an unsmoothed image's pixel takes (ADR
//! 2100), and the closed form of a dot inside two by two pixels (ADR 1829). The rectangle's edges
//! are at thirds of a pixel, which the library's 8.8 fixed point cannot hold, so its converter and
//! the closed form cannot agree there by accident (trap 225).

#![expect(
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    clippy::cast_precision_loss,
    reason = "test code: a panic with a message is the intended failure mode, and the arithmetic \
              is over rasters this file sized itself"
)]

use std::sync::Arc;

use pdf_render::{
    BlendMode, Color, Command, DisplayList, FillRule, Image, LineCap, Paint, Path, PathCommand,
    Point, Raster, Rasterizer as _, SampleAlpha, Size, Stroke, TargetSpec, Transform,
};
use render_cpu::{CpuRasterizer, Shown};

/// A page past `tiny-skia`'s range across, as the owner's witness is at 1.25¹⁰ of the fit.
const WIDE: u32 = 11_176;

/// The page's height.
const TALL: u32 = 96;

/// The page's target: one pixel a unit, flipped as `TargetSpec::for_page` flips a page, so that
/// the vertical translation is a page height rather than zero.
fn page_target() -> TargetSpec {
    TargetSpec {
        width: WIDE,
        height: TALL,
        transform: Transform::new(1.0, 0.0, 0.0, -1.0, 0.0, TALL as f32),
    }
}

/// The pixels of `window` of `page` that differ from the page's own there: column and row in the
/// window, and the two pixels.
fn differing(page: &Raster, shown: Shown, window: &Raster) -> Vec<(u32, u32, [u8; 4], [u8; 4])> {
    let pixel = |raster: &Raster, x: u32, y: u32| -> [u8; 4] {
        let at = ((y * raster.width + x) * 4) as usize;
        raster.data[at..at + 4].try_into().expect("four bytes")
    };
    let mut found = Vec::new();
    for y in 0..shown.height {
        for x in 0..shown.width {
            let (a, b) = (
                pixel(page, shown.left + x, shown.top + y),
                pixel(window, x, y),
            );
            if a != b {
                found.push((x, y, a, b));
            }
        }
    }
    found
}

/// `list` drawn whole and as `shown`, each in one strip, and the window's pixels that are not the
/// page's.
fn window_against_page(list: &DisplayList, shown: Shown) -> Vec<(u32, u32, [u8; 4], [u8; 4])> {
    let target = page_target();
    let page = CpuRasterizer::new()
        .with_strips(1)
        .rasterize(list, target)
        .expect("the page draws");
    let window = CpuRasterizer::new()
        .with_strips(1)
        .rasterize_window(list, target, shown)
        .expect("the window draws");
    assert_eq!((window.width, window.height), (shown.width, shown.height));
    differing(&page, shown, &window)
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

/// A triangle, a stroked curve and a stroked diagonal in a window of a page magnified as the owner's
/// witness is at 1.25¹⁰ of the fit, the window's corner where that window's is: each is measured
/// by `crate::area` from vertices on the page's grid, so the window holds the page's levels. From
/// the window's own target, or from the page's matrix with the corner composed in last, a vertex
/// was the same real number rounded at the window's magnitude, and these pixels parted by a level
/// (ADR 2133).
#[test]
fn a_window_s_fills_and_strokes_are_measured_from_the_page_s_vertices() {
    let scale = 4.687_865_3_f32;
    let target = TargetSpec {
        width: WIDE,
        height: TALL,
        transform: Transform::new(scale, 0.0, 0.0, -scale, 0.0, TALL as f32 - 0.635_3),
    };
    let user = |x: f32, y: f32| Point::new(x / scale, (TALL as f32 - y) / scale);
    let mut list = DisplayList::new(Size::new(WIDE as f32 / scale, TALL as f32 / scale));
    let mut triangle = Path::new();
    triangle.push(PathCommand::MoveTo(user(9_001.37, 12.29)));
    triangle.push(PathCommand::LineTo(user(9_037.11, 21.83)));
    triangle.push(PathCommand::LineTo(user(9_009.73, 61.07)));
    triangle.push(PathCommand::Close);
    list.push(filled(triangle));
    let mut curve = Path::new();
    curve.push(PathCommand::MoveTo(user(9_041.3, 70.1)));
    curve.push(PathCommand::CurveTo(
        user(9_052.9, 10.7),
        user(9_071.1, 88.3),
        user(9_083.7, 15.9),
    ));
    curve.push(PathCommand::LineTo(user(9_011.1, 80.2)));
    let mut diagonal = Path::new();
    diagonal.push(PathCommand::MoveTo(user(9_003.9, 88.4)));
    diagonal.push(PathCommand::LineTo(user(9_090.2, 6.7)));
    for (path, width) in [(curve, 0.73), (diagonal, 1.31)] {
        list.push(Command::Stroke {
            path: Arc::new(path),
            transform: Transform::IDENTITY,
            stroke: Stroke {
                width,
                ..Stroke::default()
            },
            paint: Paint::Solid(Color::BLACK),
            clip: None,
            mask: None,
            blend: BlendMode::Normal,
        });
    }
    let page = CpuRasterizer::new()
        .with_strips(1)
        .rasterize(&list, target)
        .expect("the page draws");
    for shown in [
        Shown {
            left: 8_993,
            top: 3,
            width: 101,
            height: 90,
        },
        Shown {
            left: 4_688,
            top: 1,
            width: 4_500,
            height: 94,
        },
    ] {
        let window = CpuRasterizer::new()
            .with_strips(1)
            .rasterize_window(&list, target, shown)
            .expect("the window draws");
        let moved = differing(&page, shown, &window);
        assert!(
            moved.is_empty(),
            "the window {shown:?}'s pixels the page does not have: {moved:?}"
        );
    }
}

/// A rectangle in a window of a page past 8191 pixels is drawn by the closed form the page draws it
/// by: drawn from the window's own target, which is under the range, it took the library's 8.8
/// converter and its edge at a third of a pixel was a level off the page's (ADR 2116's last
/// section).
#[test]
fn a_window_s_rectangle_takes_the_converter_the_page_takes() {
    let mut list = DisplayList::new(Size::new(WIDE as f32, TALL as f32));
    let (x0, x1) = (9_001.0 + 1.0 / 3.0, 9_013.0 + 2.0 / 3.0);
    let (y0, y1) = (20.0 + 1.0 / 3.0, 61.0 + 2.0 / 3.0);
    let mut bar = Path::new();
    bar.push(PathCommand::MoveTo(Point::new(x0, y0)));
    bar.push(PathCommand::LineTo(Point::new(x1, y0)));
    bar.push(PathCommand::LineTo(Point::new(x1, y1)));
    bar.push(PathCommand::LineTo(Point::new(x0, y1)));
    bar.push(PathCommand::Close);
    list.push(filled(bar));
    let shown = Shown {
        left: 8_990,
        top: 7,
        width: 40,
        height: 70,
    };
    let moved = window_against_page(&list, shown);
    assert!(
        moved.is_empty(),
        "the window's pixels the page does not have: {moved:?}"
    );
}

/// An unsmoothed image whose sample edges fall near pixel centres, cut by the window's left and
/// top edges: each pixel takes the sample its centre maps to on the page, so the window's
/// columns are the page's columns only if the window's first column is added back where the
/// centre is stated (ADR 2100).
#[test]
fn a_window_s_image_takes_the_sample_its_centre_maps_to_on_the_page() {
    let mut data = Vec::with_capacity(7 * 10 * 4);
    for index in 0..70_u8 {
        data.extend([index * 3, 255 - index * 3, (index % 7) * 36, 255]);
    }
    let image = Image {
        width: 7,
        height: 10,
        data: data.into(),
        interpolate: false,
        sample_alpha: SampleAlpha::Shape,
    };
    let mut list = DisplayList::new(Size::new(WIDE as f32, TALL as f32));
    // Seven samples across 17.443 units and ten down 24.92: neither a sample's width nor its
    // height is a whole or a dyadic number of pixels, so some centre lies within a rounding of an
    // edge two samples share.
    list.push(Command::Image {
        image: image.into(),
        transform: Transform::new(17.443, 0.0, 0.0, 24.92, 9_100.161, 30.609),
        alpha: 1.0,
        clip: None,
        mask: None,
        blend: BlendMode::Normal,
    });
    for shown in [
        Shown {
            left: 9_105,
            top: 11,
            width: 33,
            height: 60,
        },
        Shown {
            left: 4_096,
            top: 0,
            width: 5_120,
            height: TALL,
        },
    ] {
        let moved = window_against_page(&list, shown);
        assert!(
            moved.is_empty(),
            "the window {shown:?}'s pixels the page does not have: {moved:?}"
        );
    }
}

/// A round dot of a width under a pixel, painted by its closed form inside two by two pixels on
/// the page's grid, lands on the window's pixels the page has it on (ADR 1829).
#[test]
fn a_window_s_thin_dot_is_the_page_s() {
    let mut list = DisplayList::new(Size::new(WIDE as f32, TALL as f32));
    let mut dot = Path::new();
    dot.push(PathCommand::MoveTo(Point::new(
        9_210.0 + 1.0 / 3.0,
        40.0 + 2.0 / 3.0,
    )));
    dot.push(PathCommand::LineTo(Point::new(
        9_210.0 + 1.0 / 3.0,
        40.0 + 2.0 / 3.0,
    )));
    list.push(Command::Stroke {
        path: Arc::new(dot),
        transform: Transform::IDENTITY,
        stroke: Stroke {
            width: 0.9,
            cap: LineCap::Round,
            ..Stroke::default()
        },
        paint: Paint::Solid(Color::BLACK),
        clip: None,
        mask: None,
        blend: BlendMode::Normal,
    });
    let shown = Shown {
        left: 9_200,
        top: 30,
        width: 30,
        height: 30,
    };
    let target = page_target();
    let page = CpuRasterizer::new()
        .with_strips(1)
        .rasterize(&list, target)
        .expect("the page draws");
    let inked = page
        .data
        .chunks_exact(4)
        .filter(|pixel| pixel[0] < 255)
        .count();
    assert!(inked > 0, "the dot drew nothing on the page");
    let moved = window_against_page(&list, shown);
    assert!(
        moved.is_empty(),
        "the window's pixels the page does not have: {moved:?}"
    );
}

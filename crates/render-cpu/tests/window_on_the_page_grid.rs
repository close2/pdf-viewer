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
    BlendMode, Clip, ClipId, Color, Command, DisplayList, FillRule, Image, LineCap, Paint, Path,
    PathCommand, Point, Ramp, Raster, Rasterizer as _, SampleAlpha, Shading, ShadingKind, Size,
    Stroke, TargetSpec, Transform,
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

/// The magnification of the owner's witness at 1.25¹⁰ of the fit, which is neither a whole nor a
/// dyadic number of pixels a unit, and the page's height off by a fraction, so that no vertex a
/// test states lands on the 8.8 or the 16.16 grid by accident (trap 225).
const SCALE: f32 = 4.687_865_3;

/// The page's target at [`SCALE`], flipped as `TargetSpec::for_page` flips a page.
fn magnified() -> TargetSpec {
    TargetSpec {
        width: WIDE,
        height: TALL,
        transform: Transform::new(SCALE, 0.0, 0.0, -SCALE, 0.0, TALL as f32 - 0.635_3),
    }
}

/// The user-space point [`magnified`] carries onto the device point `(x, y)`.
fn user(x: f32, y: f32) -> Point {
    Point::new(x / SCALE, (TALL as f32 - 0.635_3 - y) / SCALE)
}

/// The two windows every test below draws: one near the corner of a window of the owner's
/// witness, under the library's 8191-pixel range as the page is not, and one wider than the range
/// whose corner is a large number of columns from the page's.
const WINDOWS: [Shown; 2] = [
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
];

/// The `index`th of a scatter of device points across the first of [`WINDOWS`], each coordinate a
/// fraction no power of two divides.
fn scattered(index: u32) -> (f32, f32) {
    let golden = |k: f32| (k * 0.618_034 + 1.0 / 3.0).fract();
    let k = index as f32;
    (
        8_996.0 + 90.0 * golden(k),
        6.0 + 80.0 * golden(k * 1.7 + 0.2),
    )
}

/// `list` drawn whole under `target` by `rasterizer` and as each of [`WINDOWS`], each in one strip;
/// a panic naming the window and its pixels wherever a window has pixels the page does not, and
/// another where the page holds no mark at all, since a test of nothing passes.
fn every_window_is_the_page(
    list: &DisplayList,
    target: TargetSpec,
    rasterizer: fn() -> CpuRasterizer,
) {
    let page = rasterizer()
        .with_strips(1)
        .rasterize(list, target)
        .expect("the page draws");
    assert!(
        page.data.chunks_exact(4).any(|pixel| pixel != [255; 4]),
        "the page holds no mark"
    );
    for shown in WINDOWS {
        let window = rasterizer()
            .with_strips(1)
            .rasterize_window(list, target, shown)
            .expect("the window draws");
        let moved = differing(&page, shown, &window);
        assert!(
            moved.is_empty(),
            "the window {shown:?}'s {} pixels the page does not have: {:?}",
            moved.len(),
            &moved[..moved.len().min(12)]
        );
    }
}

/// The rasteriser every test below draws with but one.
fn anti_aliased() -> CpuRasterizer {
    CpuRasterizer::new()
}

/// A stroke of `path` at `width` user units under `cap`, solid black.
fn stroked(path: Path, width: f32, cap: LineCap) -> Command {
    Command::Stroke {
        path: Arc::new(path),
        transform: Transform::IDENTITY,
        stroke: Stroke {
            width,
            cap,
            ..Stroke::default()
        },
        paint: Paint::Solid(Color::BLACK),
        clip: None,
        mask: None,
        blend: BlendMode::Normal,
    }
}

/// A triangle with a corner at the device point `(x, y)`.
fn triangle_at(x: f32, y: f32) -> Path {
    let mut path = Path::new();
    path.push(PathCommand::MoveTo(user(x, y)));
    path.push(PathCommand::LineTo(user(x + 3.37, y + 1.21)));
    path.push(PathCommand::LineTo(user(x + 0.91, y + 4.13)));
    path.push(PathCommand::Close);
    path
}

/// A fill whose path states subpaths with no area beside one that has some: §10.7.4's marks for
/// the first are a pixel thick across and as long as the subpath along, and the window draws them
/// where the page does only if their ends are measured on the page's grid — the marks are
/// `pdf_render::split_collapsed_fill`'s, stated back through the inverse of the placement (ADR
/// 2148).
#[test]
fn a_window_s_marks_for_subpaths_with_no_area_are_the_page_s() {
    let mut list = DisplayList::new(Size::new(WIDE as f32 / SCALE, TALL as f32 / SCALE));
    for index in 0..60 {
        let (x, y) = scattered(index);
        let mut path = triangle_at(x, y);
        path.push(PathCommand::MoveTo(user(x + 0.31, y + 6.29)));
        path.push(PathCommand::LineTo(user(x + 7.77, y + 6.29)));
        path.push(PathCommand::MoveTo(user(x - 1.13, y + 0.47)));
        path.push(PathCommand::LineTo(user(x - 1.13, y + 5.83)));
        list.push(filled(path));
    }
    every_window_is_the_page(&list, magnified(), anti_aliased);
}

/// Bars and rules thinner than a device pixel, which §10.7.4's substitution draws as the whole
/// pixel line they lie in at the coverage their area implies, and a turned rule under one level of
/// a pixel, widened to one pixel at the alpha its width implies: the line and the band are the
/// page's only if they are decided and measured on the page's grid (ADR 2148).
#[test]
fn a_window_s_marks_thinner_than_a_pixel_are_the_page_s() {
    let mut list = DisplayList::new(Size::new(WIDE as f32 / SCALE, TALL as f32 / SCALE));
    for index in 0..40 {
        let (x, y) = scattered(index);
        let mut bar = Path::new();
        let (x1, y1) = (x + 5.71, y + 0.29);
        bar.push(PathCommand::MoveTo(user(x, y)));
        bar.push(PathCommand::LineTo(user(x1, y)));
        bar.push(PathCommand::LineTo(user(x1, y1)));
        bar.push(PathCommand::LineTo(user(x, y1)));
        bar.push(PathCommand::Close);
        list.push(filled(bar));
        let mut rule = Path::new();
        rule.push(PathCommand::MoveTo(user(x + 0.43, y + 2.17)));
        rule.push(PathCommand::LineTo(user(x + 9.29, y + 2.17)));
        list.push(stroked(rule, 0.21 / SCALE, LineCap::Butt));
        let mut turned = Path::new();
        turned.push(PathCommand::MoveTo(user(x + 0.77, y + 3.41)));
        turned.push(PathCommand::LineTo(user(x + 6.13, y + 7.93)));
        list.push(stroked(turned, 0.002 / SCALE, LineCap::Round));
    }
    every_window_is_the_page(&list, magnified(), anti_aliased);
}

/// Fills under clips that are not rectangles: §10.7.4 states the clip as "the set of pixels that
/// would be included by a fill operation", so the window's mask is the page's only if the clip is
/// measured on the page's grid and its band's corner moves only the write (ADR 2148).
#[test]
fn a_window_s_clip_is_measured_on_the_page_s_grid() {
    let mut list = DisplayList::new(Size::new(WIDE as f32 / SCALE, TALL as f32 / SCALE));
    for index in 0..30 {
        let (x, y) = scattered(index);
        let mut region = Path::new();
        region.push(PathCommand::MoveTo(user(x, y)));
        region.push(PathCommand::CurveTo(
            user(x + 4.1, y - 2.3),
            user(x + 7.9, y + 5.7),
            user(x + 3.3, y + 8.9),
        ));
        region.push(PathCommand::LineTo(user(x - 2.7, y + 4.4)));
        region.push(PathCommand::Close);
        let clip: ClipId = list
            .add_clip(Clip {
                path: region,
                transform: Transform::IDENTITY,
                fill_rule: FillRule::NonZero,
                parent: None,
            })
            .expect("a clip");
        let mut cover = Path::new();
        cover.push(PathCommand::MoveTo(user(x - 4.0, y - 4.0)));
        cover.push(PathCommand::LineTo(user(x + 12.0, y - 4.0)));
        cover.push(PathCommand::LineTo(user(x + 12.0, y + 12.0)));
        cover.push(PathCommand::LineTo(user(x - 4.0, y + 12.0)));
        cover.push(PathCommand::Close);
        let mut cover = filled(cover);
        cover.set_clip(Some(clip));
        list.push(cover);
    }
    every_window_is_the_page(&list, magnified(), anti_aliased);
}

/// A radial shading whose circles do not contain one another, which §8.7.4.5.4's "greatest value
/// of s" decides per pixel and this backend draws as a raster through the shape, and an axial one
/// washed with its `/Background`: the raster is evaluated at the page's pixel centres and the
/// shape's edge measured on the page's grid (ADR 2148).
#[test]
fn a_window_s_shading_drawn_as_a_raster_is_the_page_s() {
    let ramp = Ramp::sample(|t| Color::rgb(1.0 - t, 0.3, t));
    let mut list = DisplayList::new(Size::new(WIDE as f32 / SCALE, TALL as f32 / SCALE));
    for index in 0..12 {
        let (x, y) = scattered(index);
        let cone = ShadingKind::Radial {
            start: user(x, y),
            start_radius: 0.43,
            end: user(x + 9.1, y + 2.3),
            end_radius: 1.37,
            ramp: ramp.clone(),
            extend: (true, true),
        };
        let washed = ShadingKind::Axial {
            start: user(x + 2.0, y + 3.0),
            end: user(x + 5.0, y + 7.0),
            ramp: ramp.clone(),
            extend: (false, false),
        };
        for (kind, background) in [(cone, None), (washed, Some(Color::rgb(0.2, 0.7, 0.1)))] {
            let mut shape = triangle_at(x - 2.3, y - 1.9);
            shape.push(PathCommand::MoveTo(user(x + 1.1, y + 0.3)));
            shape.push(PathCommand::LineTo(user(x + 12.9, y + 2.7)));
            shape.push(PathCommand::LineTo(user(x + 4.4, y + 9.9)));
            shape.push(PathCommand::Close);
            list.push(Command::Fill {
                path: Arc::new(shape),
                transform: Transform::IDENTITY,
                fill_rule: FillRule::NonZero,
                paint: Paint::Shading(Arc::new(Shading {
                    background,
                    kind: Arc::new(kind),
                    transform: Transform::IDENTITY,
                })),
                clip: None,
                mask: None,
                blend: BlendMode::Normal,
            });
        }
    }
    every_window_is_the_page(&list, magnified(), anti_aliased);
}

/// Round dots wider than a pixel, §8.5.3.2's "filled circle centred at the single point", which
/// lie in more than two by two pixels and so are filled as discs rather than by the closed form:
/// the mark the owner's witness held where its window parted from its page at 1.25⁸ (ADR 2148).
#[test]
fn a_window_s_dots_past_two_pixels_are_the_page_s() {
    let mut list = DisplayList::new(Size::new(WIDE as f32 / SCALE, TALL as f32 / SCALE));
    for index in 0..80 {
        let (x, y) = scattered(index);
        let mut dot = Path::new();
        dot.push(PathCommand::MoveTo(user(x, y)));
        dot.push(PathCommand::LineTo(user(x, y)));
        list.push(stroked(dot, 1.44 / SCALE, LineCap::Round));
    }
    every_window_is_the_page(&list, magnified(), anti_aliased);
}

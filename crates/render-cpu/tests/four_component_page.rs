//! §11.4.7's page composited in a four-component blending colour space, on the oracle.
//!
//! `CLAUDE.md` keeps `render-cpu` as the correctness oracle, so the numbers the other
//! backends are held to are established here — against the clause's own arithmetic rather
//! than against a golden file. ISO 32000-2 §11.4.7:
//!
//! > All page-level compositing shall be done in the default blending colour space of the
//! > page, and the entire result shall then, if the colour spaces are not equivalent, be
//! > converted to the native colour space of the output device before being composited with
//! > the context-dependent backdrop.
//!
//! and §11.3.4 makes that per component:
//!
//! > The i th component of the result colour 𝐶𝑟 shall be obtained by applying the
//! > compositing formula to the i th components of the constituent colours
//!
//! So four components are two rasters, and the question this file answers is whether the
//! composite really happened in ink. [`test_scenes::four_component_page`] carries the
//! geometry and says what each mark is for.

#![expect(
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    reason = "test code: an explanatory panic is the intended failure mode, and the \
              arithmetic is on literal page dimensions that cannot overflow"
)]

use pdf_render::{Raster, Rasterizer, TargetSpec};
use render_cpu::CpuRasterizer;

/// Pixel budget for a target; far above anything this test requests.
const GENEROUS: u64 = 1 << 30;

/// Reads a pixel as `[r, g, b, a]`.
fn pixel(raster: &Raster, x: u32, y: u32) -> [u8; 4] {
    let index = ((y as usize) * (raster.width as usize) + (x as usize)) * 4;
    let p = &raster.data[index..index + 4];
    [p[0], p[1], p[2], p[3]]
}

/// How far a channel may sit from the arithmetic below.
///
/// One level, and it is the quantisation of a premultiplied byte on the way through
/// [`pdf_render::blending::resolve`]: half of registration black is 127 or 128 of 255 rather
/// than exactly half, so the ink recovered from it is 0.502 rather than 0.500.
const TOLERANCE: i32 = 1;

fn assert_close(what: &str, got: [u8; 4], want: [u8; 4]) {
    let apart = |a: u8, b: u8| i32::from(a) - i32::from(b);
    assert!(
        got.iter()
            .zip(want)
            .all(|(&g, w)| apart(g, w).abs() <= TOLERANCE),
        "{what}: drew {got:?}, and the clause's arithmetic gives {want:?}"
    );
}

fn render(list: &pdf_render::DisplayList) -> Raster {
    let target = TargetSpec::for_page(list, 1.0, GENEROUS).expect("A4 target is valid");
    CpuRasterizer::new()
        .rasterize(list, target)
        .expect("the oracle draws a four-component page")
}

/// The four marks, each against the value §11.3.4 gives it.
///
/// The device coordinates are the page's with y flipped by the target transform (A4 is 842
/// units tall), so page y 600 is device y 242.
#[test]
fn a_four_component_page_composites_in_ink_and_converts_once() {
    let raster = render(&test_scenes::four_component_page());

    // Nothing but paper: no ink at all is the cube's first corner, and the medium under it
    // is white as well, so this pixel says only that the page was drawn.
    assert_close("paper", pixel(&raster, 450, 592), [255, 255, 255, 255]);

    // Half of registration black over paper. Per component the pixel holds ½ of each of the
    // four inks, and the multilinear interpolation of the cube at (½, ½, ½, ½) is the mean of
    // its sixteen corners — (76.0, 66.1, 63.9) of 255.
    assert_close(
        "half registration black over paper",
        pixel(&raster, 190, 242),
        [76, 66, 64, 255],
    );

    // The black component alone. This mark is **white** in the chromatic raster — no cyan, no
    // magenta, no yellow — so it is the one that says the second raster was drawn at all.
    assert_close(
        "process black alone",
        pixel(&raster, 430, 242),
        [35, 31, 32, 255],
    );

    // The chromatic half, unchanged by the recombination: the cube's cyan corner.
    assert_close(
        "process cyan alone",
        pixel(&raster, 190, 562),
        [0, 173, 239, 255],
    );
}

/// The route this fixture replaces: converting each colour first and compositing on the
/// device's three components.
///
/// The same two marks stated the way a page with no `/Group` states them — registration black
/// already converted to `(0, 0, 0)` and paper to `(255, 255, 255)` — so the composite happens
/// in light rather than in ink and half of one over the other is **127.5**. That is 51 of 255
/// away from what the clause gives at the same pixel, and it is the whole reason §11.4.7 is
/// implemented rather than approximated: putting the old route back is what shows the test
/// above is measuring the clause and not the geometry.
#[test]
fn converting_before_compositing_is_a_different_page() {
    use std::sync::Arc;

    use pdf_render::{BlendMode, Color, Command, DisplayList, FillRule, Paint, Transform};
    use test_scenes::{A4, rect};

    let fill = |x0, y0, x1, y1, paint| Command::Fill {
        path: Arc::new(rect(x0, y0, x1, y1)),
        transform: Transform::IDENTITY,
        fill_rule: FillRule::NonZero,
        paint: Paint::Solid(paint),
        clip: None,
        mask: None,
        blend: BlendMode::Normal,
    };
    let mut list = DisplayList::new(A4);
    list.push(fill(
        40.0,
        100.0,
        555.0,
        750.0,
        Color {
            r: 1.0,
            g: 1.0,
            b: 1.0,
            a: 1.0,
        },
    ));
    list.push(fill(
        80.0,
        500.0,
        300.0,
        700.0,
        Color {
            r: 0.0,
            g: 0.0,
            b: 0.0,
            a: 0.5,
        },
    ));

    let drawn = pixel(&render(&list), 190, 242);
    assert!(
        (127..=128).contains(&drawn[0]),
        "averaging two converted colours gives 127.5 of 255: {drawn:?}"
    );
    assert!(
        i32::from(drawn[0]) - 76 >= 50,
        "and it is 51 of 255 from the ink the clause composites: {drawn:?}"
    );
}

//! A group compositing in a blending colour space of its own, and a luminosity mask whose `Y` is
//! a space's own, drawn by this backend and held to the clause's closed form (ADR 1471).
//!
//! ISO 32000-2 §11.7.2: "all blending and compositing computations shall be done in that space",
//! and "[t]he resulting colours shall then be interpreted in the group's colour space when the
//! group is subsequently composited with its backdrop" — so the conversion runs on the
//! *composited* group, which is what makes compositing in the space differ from converting each
//! colour first. §11.5.3 takes a luminosity mask's `Y` of the group composited onto its backdrop,
//! in the group's own space. Every expected value below is that arithmetic worked by hand on the
//! fixture's own numbers; the CPU backend is not asked (trap 9).

#![expect(
    clippy::expect_used,
    clippy::panic,
    clippy::arithmetic_side_effects,
    reason = "test code: an explanatory panic is the intended failure mode, and the pixel \
              indices are inside rasters of a few hundred pixels a side"
)]

use std::sync::Arc;

use pdf_render::{
    BlackHalf, BlendMode, Color, Command, DisplayList, FillRule, Luminance, Paint, Path,
    PathCommand, Point, Raster, Rasterizer, Size, SoftMask, SoftMaskKind, TargetSpec, Transform,
};
use render_raster::{PresentFrame, QuorraRasterizer};

/// Pixel budget for a target; far above anything this file requests.
const GENEROUS: u64 = 1 << 30;

/// How far a channel may sit from the closed form: one level for the 8-bit composite of half
/// a black over paper (127 or 128), and one for the conversion's own rounding.
const LEVELS: u8 = 2;

fn raster() -> QuorraRasterizer {
    QuorraRasterizer::new_headless().expect("a device is available (ADR 0004)")
}

fn draw(list: &DisplayList) -> Raster {
    let target = TargetSpec::for_page(list, 1.0, GENEROUS).expect("the target fits the budget");
    raster()
        .rasterize(list, target)
        .unwrap_or_else(|e| panic!("raster draws a group in its own space: {e}"))
}

fn pixel(raster: &Raster, x: u32, y: u32) -> [u8; 4] {
    let at = usize::try_from((y * raster.width + x) * 4).expect("an index into the raster");
    let mut out = [0; 4];
    out.copy_from_slice(&raster.data[at..at + 4]);
    out
}

fn assert_near(what: &str, got: [u8; 4], want: [u8; 3]) {
    for (channel, (&g, &w)) in got.iter().zip(want.iter()).enumerate() {
        assert!(
            g.abs_diff(w) <= LEVELS,
            "{what}: channel {channel} is {g}, the clause's arithmetic gives {w} ({got:?})"
        );
    }
    assert_eq!(got[3], 255, "{what}: the page is opaque over its medium");
}

/// sRGB's encoding of a linear value, which is the output curve both one- and three-component
/// fixtures state.
fn srgb(linear: f32) -> u8 {
    let encoded = if linear <= 0.003_130_8 {
        linear * 12.92
    } else {
        1.055 * linear.powf(1.0 / 2.4) - 0.055
    };
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "clamped to 0..=255 before the cast"
    )]
    {
        (encoded * 255.0).round().clamp(0.0, 255.0) as u8
    }
}

/// Four components: half of registration black over paper inside the group is half of each
/// ink, and the grid of side two at (½, ½, ½, ½) is the mean of its sixteen corners —
/// `test_scenes`'s process-ink corners average to (76, 66, 64). Paper alone is the grid's first
/// corner, (255, 255, 255). Converting each colour before compositing would give grey 128.
#[test]
fn a_four_component_group_resolves_its_pair_after_compositing() {
    let page = draw(&test_scenes::group_in_its_own_blending_space());
    assert_near("half of every ink", pixel(&page, 50, 50), [76, 66, 64]);
    assert_near(
        "paper inside the group",
        pixel(&page, 15, 50),
        [255, 255, 255],
    );
    assert_near(
        "the page outside the group",
        pixel(&page, 5, 5),
        [255, 255, 255],
    );
}

/// One component through a curve: the composite of half a black over paper is the component
/// ½, and sRGB's curve puts it at 188 (`test_scenes::group_in_a_one_component_blending_space`).
#[test]
fn a_one_component_group_resolves_its_curve_after_compositing() {
    let page = draw(&test_scenes::group_in_a_one_component_blending_space());
    let half = srgb(0.5);
    assert_eq!(half, 188, "the fixture's own figure");
    assert_near("the curve at one half", pixel(&page, 50, 50), [half; 3]);
    assert_near("paper", pixel(&page, 15, 50), [255; 3]);
}

/// Three CIE-based components through a cube that is the identity on linear light with sRGB's
/// curve out: ½ on each component is 188 on each channel.
#[test]
fn a_three_component_group_resolves_its_cube_after_compositing() {
    let page = draw(&test_scenes::group_in_a_three_component_blending_space());
    assert_near("the cube at one half", pixel(&page, 50, 50), [srgb(0.5); 3]);
    assert_near("paper", pixel(&page, 15, 50), [255; 3]);
}

/// The group's own alpha applies to the *converted* result, once (§11.4.4, §11.7.2): at alpha ½
/// over the white page the centre is the mean of 188 and 255.
#[test]
fn a_group_alpha_applies_to_the_converted_result() {
    let mut list = test_scenes::group_in_a_three_component_blending_space();
    let commands: Vec<Command> = list
        .commands()
        .iter()
        .cloned()
        .map(|command| match command {
            Command::Group {
                commands,
                clip,
                mask,
                blend,
                isolated,
                knockout,
                alpha_is_shape,
                blending,
                ..
            } => Command::Group {
                commands,
                alpha: 0.5,
                clip,
                mask,
                blend,
                isolated,
                knockout,
                alpha_is_shape,
                blending,
            },
            other => other,
        })
        .collect();
    let mut halved = DisplayList::new(list.page_size);
    for command in commands {
        halved.push(command);
    }
    list = halved;
    let page = draw(&list);
    let want = u8::try_from((u16::from(srgb(0.5)) + 255).div_ceil(2)).expect("a level");
    assert_near(
        "half of the converted group",
        pixel(&page, 50, 50),
        [want; 3],
    );
}

/// The window's own path, where the scene is built in page space and the viewport places it:
/// the resolved group lands on the same pixels the page does, at two placements in a row — the
/// second a rebuild, since the scene carries this view's pixels (ADR 0702).
#[test]
fn a_window_frame_places_the_resolved_group_where_the_page_is() {
    let list = Arc::new(test_scenes::group_in_a_three_component_blending_space());
    let page = TargetSpec::for_page(&list, 1.0, GENEROUS).expect("the target fits the budget");
    let mut gpu = raster();
    for (dx, dy) in [(20.0_f32, 10.0_f32), (33.0, 17.0)] {
        let target = TargetSpec {
            width: 160,
            height: 140,
            transform: page.transform.then(Transform::translate(dx, dy)),
        };
        let frame = gpu
            .rasterize_frame(&PresentFrame {
                width: 160,
                height: 140,
                pages: &[(&list, target)],
                raster: None,
                overlays: &[],
            })
            .unwrap_or_else(|e| panic!("the window draws the group: {e}"));
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "whole offsets of a few tens of pixels"
        )]
        let (x, y) = (dx as u32, dy as u32);
        assert_near(
            "the cube at one half, placed",
            pixel(&frame, 50 + x, 50 + y),
            [srgb(0.5); 3],
        );
        assert_near("paper, placed", pixel(&frame, 15 + x, 50 + y), [255; 3]);
    }
}

/// The page every mask fixture is drawn on.
const MASK_PAGE: Size = Size {
    width: 64.0,
    height: 64.0,
};

fn square(
    x0: f32,
    y0: f32,
    x1: f32,
    y1: f32,
    paint: Color,
    mask: Option<pdf_render::SoftMaskId>,
) -> Command {
    let mut path = Path::new();
    for command in [
        PathCommand::MoveTo(Point::new(x0, y0)),
        PathCommand::LineTo(Point::new(x1, y0)),
        PathCommand::LineTo(Point::new(x1, y1)),
        PathCommand::LineTo(Point::new(x0, y1)),
        PathCommand::Close,
    ] {
        path.push(command);
    }
    Command::Fill {
        path: Arc::new(path),
        transform: Transform::IDENTITY,
        fill_rule: FillRule::NonZero,
        paint: Paint::Solid(paint),
        clip: None,
        mask,
        blend: BlendMode::Normal,
    }
}

const BLUE: Color = Color {
    r: 0.0,
    g: 0.0,
    b: 1.0,
    a: 1.0,
};

/// A blue page-wide fill under the mask, so the mask value reads off the blue's alpha over the
/// white medium: a value `v` shows as `255 − v` in red and green.
fn masked_page(mask: SoftMask) -> DisplayList {
    let mut list = DisplayList::new(MASK_PAGE);
    let id = list
        .add_soft_mask(mask)
        .expect("the first soft mask of this list");
    list.push(square(0.0, 0.0, 64.0, 64.0, BLUE, Some(id)));
    list
}

/// Three curves summed (§11.5.3's EXAMPLE 1 shape): with curves of 0.2, 0.7 and 0.1 times the
/// component, a square of the first component alone on a black backdrop has `Y` = 0.2, mask 51,
/// and the backdrop alone has `Y` = 0. §10.4.2.2's device weights would give 0.30 instead.
#[test]
fn a_luminosity_of_three_curves_is_the_spaces_own_y() {
    let curves: [[f32; 3]; 256] = std::array::from_fn(|i| {
        let t = f32::from(u8::try_from(i).expect("256 samples")) / 255.0;
        [0.2 * t, 0.7 * t, 0.1 * t]
    });
    let mask = SoftMask {
        commands: vec![square(
            16.0,
            16.0,
            48.0,
            48.0,
            Color {
                r: 1.0,
                g: 0.0,
                b: 0.0,
                a: 1.0,
            },
            None,
        )],
        kind: SoftMaskKind::Luminosity {
            backdrop: Color {
                r: 0.0,
                g: 0.0,
                b: 0.0,
                a: 1.0,
            },
        },
        transfer: None,
        luminance: Some(Luminance::curves(Arc::new(curves))),
        black: None,
    };
    let page = draw(&masked_page(mask));
    // Mask 51 of blue over white: red and green are 255 − 51.
    assert_near(
        "Y = 0.2 inside the square",
        pixel(&page, 32, 32),
        [204, 204, 255],
    );
    assert_near("Y = 0 on the backdrop", pixel(&page, 4, 4), [255, 255, 255]);
}

/// The pair (§11.4.7, §11.5.3 EXAMPLE 2's `DeviceCMYK` branch): an ink of black ½ and no other is
/// a white chromatic half and a half-grey black half, and `Y = 1 − min(1, 0.3C + 0.59M + 0.11Y +
/// K)` = ½, mask 128. Outside the square, paper on both halves, `Y` = 1. Reading the chromatic
/// half alone would give 1 inside the square too.
#[test]
fn a_luminosity_of_the_pair_reads_all_four_components() {
    let grey = Color {
        r: 0.5,
        g: 0.5,
        b: 0.5,
        a: 1.0,
    };
    let mask = SoftMask {
        commands: vec![square(16.0, 16.0, 48.0, 48.0, Color::WHITE, None)],
        kind: SoftMaskKind::Luminosity {
            backdrop: Color::WHITE,
        },
        transfer: None,
        luminance: Some(Luminance::device_ink()),
        black: Some(BlackHalf {
            commands: vec![square(16.0, 16.0, 48.0, 48.0, grey, None)],
            backdrop: Color::WHITE,
        }),
    };
    let page = draw(&masked_page(mask));
    assert_near("K = ½: Y = ½", pixel(&page, 32, 32), [127, 127, 255]);
    assert_near("paper: Y = 1", pixel(&page, 4, 4), [0, 0, 255]);
}

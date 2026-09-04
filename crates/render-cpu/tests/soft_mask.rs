//! What a soft mask does to the pixels beneath it (ISO 32000-2 §11.5).
//!
//! The values here are computed from the clause rather than observed: a mask value is a
//! coverage, so painting an object through one is the ordinary interpolation between what
//! is already there and what the object paints. Getting that arithmetic wrong by a level or
//! two is invisible in any picture and is exactly what an oracle comparison against another
//! renderer trips over, so it is pinned here at the pixel.

#![expect(
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "test code: a panic with a message is the intended failure mode, the arithmetic \
              is on literal page dimensions that cannot overflow, and the casts turn a \
              clamped unit interval into the byte it is compared against"
)]

use std::sync::Arc;

use pdf_render::{
    Color, Command, DisplayList, FillRule, Paint, Path, PathCommand, Point, Rasterizer, Size,
    SoftMask, SoftMaskKind, TargetSpec, Transfer, Transform,
};
use render_cpu::CpuRasterizer;

const PAGE: f32 = 20.0;

/// The whole page as a path.
fn page_rect() -> Path {
    let mut path = Path::new();
    path.push(PathCommand::MoveTo(Point::new(0.0, 0.0)));
    path.push(PathCommand::LineTo(Point::new(PAGE, 0.0)));
    path.push(PathCommand::LineTo(Point::new(PAGE, PAGE)));
    path.push(PathCommand::LineTo(Point::new(0.0, PAGE)));
    path.push(PathCommand::Close);
    path
}

fn fill(colour: Color, mask: Option<pdf_render::SoftMaskId>) -> Command {
    Command::Fill {
        path: Arc::new(page_rect()),
        transform: Transform::IDENTITY,
        fill_rule: FillRule::NonZero,
        paint: Paint::Solid(colour),
        clip: None,
        mask,
        blend: pdf_render::BlendMode::Normal,
    }
}

/// Renders a list at 1:1 and returns the pixel at the middle of the page.
fn centre_pixel(list: &DisplayList) -> [u8; 4] {
    let target = TargetSpec::for_page(list, 1.0, 1 << 20).expect("a valid target");
    let raster = CpuRasterizer::new()
        .rasterize(list, target)
        .expect("the page rasterises");
    let index = ((target.height / 2) * target.width + target.width / 2) as usize * 4;
    let pixel = &raster.data[index..index + 4];
    [pixel[0], pixel[1], pixel[2], pixel[3]]
}

/// A mask of constant value `v` composites `v` of the object over `1 - v` of the backdrop.
///
/// The mask is a luminosity one over an *empty* group, so every pixel takes §11.6.5.1's
/// outside-the-bounding-box value — "the mask value shall be derived by transforming the BC
/// colour to luminosity" — which for a grey backdrop is that grey. A constant is the one
/// mask whose expected pixel can be written down in closed form, which is why the arithmetic
/// is checked here rather than on a gradient.
#[test]
fn a_constant_mask_interpolates_between_the_object_and_the_backdrop() {
    let backdrop = Color::rgb(0.75, 0.75, 0.75);
    let grey = Color::rgb(0.95, 0.95, 0.95);
    let red = Color::rgb(0.85, 0.2, 0.1);

    let mut list = DisplayList::new(Size::new(PAGE, PAGE));
    let mask = list
        .add_soft_mask(SoftMask {
            commands: Vec::new(),
            kind: SoftMaskKind::Luminosity { backdrop },
            transfer: None,
            luminance: None,
            black: None,
        })
        .expect("the first soft mask");
    list.push(fill(grey, None));
    list.push(fill(red, Some(mask)));

    // §11.5.3's luminosity of an achromatic backdrop is that grey, exactly: the three
    // coefficients of EXAMPLE 2 sum to 1. Quantised to the eight bits a mask value holds.
    let value = f32::from((0.75_f32 * 255.0).round() as u8) / 255.0;
    let expected = |over: f32, under: f32| {
        let byte = |channel: f32| (channel * 255.0).round();
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "an interpolation between two bytes is a byte"
        )]
        {
            byte(over)
                .mul_add(value, byte(under) * (1.0 - value))
                .round() as u8
        }
    };

    let pixel = centre_pixel(&list);
    let want = [
        expected(red.r, grey.r),
        expected(red.g, grey.g),
        expected(red.b, grey.b),
        255,
    ];
    assert_eq!(pixel, want, "§11.3.6's weighted average, to the level");
}

/// The same average, at **every** value a mask can take, and it is exact at all 256.
///
/// This is the discriminating form of the test above, and the reason it exists is that the
/// test above used to allow one level of slack — "quantised to the eight bits a mask value
/// holds", which sounded like arithmetic and was a hypothesis. It hid a departure of up to
/// two levels for five hundred sessions, on a page the oracle had listed the whole time
/// (`CONTRADICTED_MASK_QUANTISATION`, ADR 0418). The eight-bit mask was never the cause:
/// `tiny-skia` compiles two raster pipelines and picked the low-precision one, whose
/// `div255(v) = (v + 255) >> 8` is an upper bound on `v ÷ 255` rather than its rounding, twice
/// per pixel and both times in the same direction.
///
/// A single mask value cannot tell one pipeline from the other — at 64 of 255 the two agree —
/// so the sweep is the test. `crate::HIGH_PRECISION_PIPELINE` is what makes it pass;
/// **flipping that constant to `false` makes it fail**, which is what establishes that it
/// guards the decision rather than describing it.
#[test]
fn the_weighted_average_is_exact_at_every_mask_value() {
    let grey = Color::rgb(0.95, 0.95, 0.95);
    let red = Color::rgb(0.85, 0.2, 0.1);
    // The destination is what the first fill actually wrote, which is the byte the raster
    // holds rather than the 0.95 the display list states: §11.3.6's backdrop colour `Cb` is
    // the pixel already there.
    let backdrop = f32::from((0.95_f32 * 255.0).round() as u8) / 255.0;

    for value in 0..=u8::MAX {
        // A `/TR` that answers this value for every input makes the mask exactly `value`
        // everywhere, whatever §11.5.3 derives — which is how one assertion reaches all 256.
        let mut list = DisplayList::new(Size::new(PAGE, PAGE));
        let mask = list
            .add_soft_mask(SoftMask {
                commands: Vec::new(),
                kind: SoftMaskKind::Luminosity {
                    backdrop: Color::WHITE,
                },
                transfer: Some(Transfer::from_samples([value; 256])),
                luminance: None,
                black: None,
            })
            .expect("the first soft mask");
        list.push(fill(grey, None));
        list.push(fill(red, Some(mask)));

        let alpha = f32::from(value) / 255.0;
        let expected =
            |source: f32| (source.mul_add(alpha, backdrop * (1.0 - alpha)) * 255.0).round() as u8;
        assert_eq!(
            centre_pixel(&list),
            [expected(red.r), expected(red.g), expected(red.b), 255],
            "mask value {value} of 255"
        );
    }
}

/// A mask of zero paints nothing, and one of full paints everything.
///
/// The two ends matter more than the middle: §11.6.5.1 makes the area outside a mask
/// group's bounding box the transfer function of the backdrop's luminosity, so a black
/// backdrop hides everything the mask is applied to and a white one hides nothing. Getting
/// the ends the wrong way round is a page that is either blank or unmasked, and both look
/// like a plausible rendering.
#[test]
fn a_black_backdrop_masks_everything_away_and_a_white_one_nothing() {
    let grey = Color::rgb(0.95, 0.95, 0.95);
    let red = Color::rgb(1.0, 0.0, 0.0);

    for (backdrop, want) in [
        (Color::BLACK, [242, 242, 242, 255]),
        (Color::WHITE, [255, 0, 0, 255]),
    ] {
        let mut list = DisplayList::new(Size::new(PAGE, PAGE));
        let mask = list
            .add_soft_mask(SoftMask {
                commands: Vec::new(),
                kind: SoftMaskKind::Luminosity { backdrop },
                transfer: None,
                luminance: None,
                black: None,
            })
            .expect("the first soft mask");
        list.push(fill(grey, None));
        list.push(fill(red, Some(mask)));
        assert_eq!(
            centre_pixel(&list),
            want,
            "a backdrop of {backdrop:?} should leave {want:?}"
        );
    }
}

/// An alpha mask takes the group's alpha and ignores its colour (§11.5.2).
///
/// The group here paints one half-transparent black rectangle over the whole page, so the
/// mask is 0.5 everywhere — and it would be 0.0 if the derivation looked at the colour,
/// which is the mistake this pins.
#[test]
fn an_alpha_mask_reads_the_groups_alpha_rather_than_its_colour() {
    let grey = Color::rgb(1.0, 1.0, 1.0);
    let red = Color::rgb(1.0, 0.0, 0.0);

    let mut list = DisplayList::new(Size::new(PAGE, PAGE));
    let mask = list
        .add_soft_mask(SoftMask {
            commands: vec![fill(Color::rgba(0.0, 0.0, 0.0, 0.5), None)],
            kind: SoftMaskKind::Alpha,
            transfer: None,
            luminance: None,
            black: None,
        })
        .expect("the first soft mask");
    list.push(fill(grey, None));
    list.push(fill(red, Some(mask)));

    let pixel = centre_pixel(&list);
    assert!(
        pixel[0] > 250 && (120..=136).contains(&pixel[1]) && (120..=136).contains(&pixel[2]),
        "half the red over white should be about [255, 128, 128], not {pixel:?}"
    );
}

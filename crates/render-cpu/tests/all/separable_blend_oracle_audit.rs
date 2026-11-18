//! Scratch integration-test draft for ISO 32000-2 §11.3.5.2.
//!
//! Promote to `crates/render-cpu/tests/separable_blend_oracle_audit.rs` only after parent review.
//! This oracle is deliberately independent of render-cpu's mode conversion: it computes the
//! Table 134 values listed below at an interior pixel of an opaque two-fill page. With both
//! alphas equal to one, §11.3.6 reduces the result color to B(Cb,Cs), so compositing does not
//! obscure the mode formula. The values are from the equations in Table 134, not another
//! renderer. Selected Dodge/Burn endpoints and `SoftLight` branches are checked separately.

#![expect(
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    reason = "test fixture is a bounded 64-pixel page and failures should report the expected value"
)]

use std::sync::Arc;

use pdf_render::{
    BlendMode, Color, Command, DisplayList, FillRule, Paint, Path, PathCommand, Point, Raster,
    Rasterizer, Size, TargetSpec, Transform,
};
use render_cpu::CpuRasterizer;

const PIXEL_BUDGET: u64 = 1 << 20;
const SAMPLE_X: u32 = 32;
const SAMPLE_Y: u32 = 32;

fn box_path() -> Arc<Path> {
    let mut path = Path::new();
    path.push(PathCommand::MoveTo(Point::new(8.0, 8.0)));
    path.push(PathCommand::LineTo(Point::new(56.0, 8.0)));
    path.push(PathCommand::LineTo(Point::new(56.0, 56.0)));
    path.push(PathCommand::LineTo(Point::new(8.0, 56.0)));
    path.push(PathCommand::Close);
    Arc::new(path)
}

fn sample(mode: BlendMode) -> [u8; 3] {
    sample_colors(mode, [0.2, 0.4, 0.6], [0.3, 0.7, 0.8])
}

fn sample_colors(mode: BlendMode, backdrop: [f32; 3], source: [f32; 3]) -> [u8; 3] {
    let path = box_path();
    let mut list = DisplayList::new(Size::new(64.0, 64.0));
    list.push(Command::Fill {
        path: Arc::clone(&path),
        transform: Transform::IDENTITY,
        fill_rule: FillRule::NonZero,
        paint: Paint::Solid(Color::rgb(backdrop[0], backdrop[1], backdrop[2])),
        clip: None,
        mask: None,
        blend: BlendMode::Normal,
    });
    list.push(Command::Fill {
        path,
        transform: Transform::IDENTITY,
        fill_rule: FillRule::NonZero,
        paint: Paint::Solid(Color::rgb(source[0], source[1], source[2])),
        clip: None,
        mask: None,
        blend: mode,
    });
    let target = TargetSpec::for_page(&list, 1.0, PIXEL_BUDGET).expect("bounded test page");
    let raster = CpuRasterizer::new()
        .rasterize(&list, target)
        .expect("two solid fills are supported");
    pixel(&raster, SAMPLE_X, SAMPLE_Y)
}

fn pixel(raster: &Raster, x: u32, y: u32) -> [u8; 3] {
    let offset = ((y * raster.width + x) * 4) as usize;
    [
        raster.data[offset],
        raster.data[offset + 1],
        raster.data[offset + 2],
    ]
}

fn assert_near(actual: [u8; 3], expected: [u8; 3], mode: BlendMode) {
    for (channel, (got, want)) in actual.into_iter().zip(expected).enumerate() {
        assert!(
            got.abs_diff(want) <= 2,
            "{mode:?} channel {channel}: got {got}, expected {want} ±2; observed {actual:?}"
        );
    }
}

#[test]
fn opaque_separable_modes_match_table_134_at_an_interior_pixel() {
    // cb=(.2,.4,.6), cs=(.3,.7,.8), encoded to nearest 8-bit component. Expected bytes
    // are direct evaluations of Table 134, rounded to the nearest byte; ±2 accounts for the
    // raster conversion of half-level inputs.
    // Overlay and HardLight intentionally differ here: overlay is .56 in green, hard light is .64.
    // The inset rectangle makes the sample independent of path-edge coverage.
    let cases = [
        (BlendMode::Normal, [77, 179, 204]),
        (BlendMode::Multiply, [15, 71, 122]),
        (BlendMode::Screen, [112, 209, 235]),
        (BlendMode::Overlay, [31, 143, 214]),
        (BlendMode::Darken, [51, 102, 153]),
        (BlendMode::Lighten, [77, 179, 204]),
        (BlendMode::ColorDodge, [73, 255, 255]),
        (BlendMode::ColorBurn, [0, 36, 128]),
        (BlendMode::HardLight, [31, 163, 214]),
        (BlendMode::SoftLight, [35, 126, 180]),
        (BlendMode::Difference, [26, 77, 51]),
        (BlendMode::Exclusion, [97, 138, 112]),
    ];
    for (mode, expected) in cases {
        assert_near(sample(mode), expected, mode);
    }
}

#[test]
fn dodge_burn_piecewise_edges_and_softlight_low_domain_are_formula_derived() {
    // Table 134's PDF 2.0 edge cases are load-bearing: Dodge(cb=0, cs=1) is zero, while
    // Burn(cb=1, cs=0) is one. Each sample also exercises the neighboring piecewise condition.
    assert_near(
        sample_colors(BlendMode::ColorDodge, [0.0, 0.7, 0.2], [1.0, 0.4, 0.1]),
        [0, 255, 57],
        BlendMode::ColorDodge,
    );
    assert_near(
        sample_colors(BlendMode::ColorBurn, [1.0, 0.2, 0.8], [0.0, 0.7, 0.9]),
        [255, 0, 198],
        BlendMode::ColorBurn,
    );

    // SoftLight's D(x) is cubic for x <= .25 and sqrt(x) above .25. For .01 the cubic is
    // .038816, visibly different from sqrt(.01)=.1. The green component takes the low-source
    // formula, and blue at cs=.5 is the shared boundary value cb.
    assert_near(
        sample_colors(BlendMode::SoftLight, [0.01, 0.2, 0.25], [0.8, 0.25, 0.5]),
        [7, 31, 64],
        BlendMode::SoftLight,
    );
}

#[test]
fn nonseparable_rgb_modes_match_independent_table_135_values() {
    // For cb=(.2,.4,.6) and cs=(.3,.7,.8), Table 135 plus Lum weights
    // (.3,.59,.11) gives Hue=(.1292,.4492,.5292), Saturation=(.1595,.4095,.6595),
    // Color=(.071,.471,.571), Luminosity=(.429,.629,.829). The first three preserve
    // backdrop luminosity .362; Luminosity takes source luminosity .591. These values are
    // literal hand-evaluations, not calls into render-cpu's helper functions.
    let cases = [
        (BlendMode::Hue, [33, 115, 135]),
        (BlendMode::Saturation, [41, 104, 168]),
        (BlendMode::Color, [18, 120, 146]),
        (BlendMode::Luminosity, [109, 160, 211]),
    ];
    for (mode, expected) in cases {
        assert_near(sample(mode), expected, mode);
    }
}

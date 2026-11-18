//! Independent formula control for ISO 32000-2 §11.3.3 and §11.3.5.
//!
//! Unlike the opaque Table 134/135 mode tests, this exercises the full basic compositing
//! equation with both source and backdrop alpha strictly between zero and one. Expected
//! channels are calculated directly from the clause; the Normal control distinguishes the
//! alpha-weighting path from an accidentally opaque blend-function-only result.

#![expect(
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    reason = "test fixture uses a bounded raster and fixed unit-interval arithmetic"
)]

use std::sync::Arc;

use pdf_render::{
    BlendMode, Color, Command, DisplayList, FillRule, Paint, Path, PathCommand, Point, Rasterizer,
    Size, TargetSpec, Transform,
};
use render_cpu::CpuRasterizer;

fn rect() -> Arc<Path> {
    let mut path = Path::new();
    path.push(PathCommand::MoveTo(Point::new(8.0, 8.0)));
    path.push(PathCommand::LineTo(Point::new(56.0, 8.0)));
    path.push(PathCommand::LineTo(Point::new(56.0, 56.0)));
    path.push(PathCommand::LineTo(Point::new(8.0, 56.0)));
    path.push(PathCommand::Close);
    Arc::new(path)
}

fn render(mode: BlendMode) -> [u8; 4] {
    let path = rect();
    let mut list = DisplayList::new(Size::new(64.0, 64.0));
    for (colour, blend) in [
        (Color::rgba(0.2, 0.4, 0.6, 0.4), BlendMode::Normal),
        (Color::rgba(0.6, 0.3, 0.8, 0.5), mode),
    ] {
        list.push(Command::Fill {
            path: Arc::clone(&path),
            transform: Transform::IDENTITY,
            fill_rule: FillRule::NonZero,
            paint: Paint::Solid(colour),
            clip: None,
            mask: None,
            blend,
        });
    }
    let target = TargetSpec::for_page(&list, 1.0, 1 << 20).expect("bounded page");
    let raster = CpuRasterizer::new()
        .with_medium(pdf_render::Medium::NONE)
        .rasterize(&list, target)
        .expect("two solid fills are supported");
    let offset = ((32 * raster.width + 32) * 4) as usize;
    [
        raster.data[offset],
        raster.data[offset + 1],
        raster.data[offset + 2],
        raster.data[offset + 3],
    ]
}

fn near(actual: [u8; 4], expected: [u8; 4]) {
    for (channel, (got, want)) in actual.into_iter().zip(expected).enumerate() {
        assert!(
            got.abs_diff(want) <= 1,
            "channel {channel}: observed {actual:?}, expected {expected:?} (±1 byte)"
        );
    }
}

#[test]
fn basic_composite_uses_both_alphas_and_the_overlap_blend_value() {
    // Medium::NONE isolates the composite from the later white-paper operation.
    // alpha_b=.4, alpha_s=.5, alpha_r=.4+.5-(.4*.5)=.7.
    // With Multiply, B(cb,cs)=(.12,.12,.48). Table 133's straight result is
    // (1-.5/.7)*cb + (.5/.7)*((1-.4)*cs + .4*B(cb,cs))
    // = (.348571..., .277142..., .651428...), which rounds to [89,71,166].
    // The result alpha is .7 => 179. The independent Normal control has B=cs and
    // therefore gives [124,84,189,179].
    near(render(BlendMode::Normal), [124, 84, 189, 179]);
    near(render(BlendMode::Multiply), [89, 71, 166, 179]);
    // Nonseparable Color takes the separate compositor route. Table 135 gives
    // Lum(cb)=.362, Lum(cs)=.445, so SetLum(cs,Lum(cb))=(.517,.217,.717).
    // All components stay in range; no ClipColor branch changes them. Substitution
    // into the same full-alpha formula gives (.462,.304857...,.719142...).
    near(render(BlendMode::Color), [118, 78, 183, 179]);
}

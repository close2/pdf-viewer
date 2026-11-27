//! A non-separable blend mode over a backdrop that is white everywhere it is painted leaves the
//! page white: ISO 32000-2 §11.3.5.3's `ClipColor` asked of a neutral colour (ADR 3773).
//!
//! The page is a white mark of fractional coverage under an isolated group of blend mode `Color`
//! that paints white over the whole page. §11.3.5.3 defines the mode as
//! `B(Cb, Cs) = SetLum(Cs, Lum(Cb))`; with both colours white that is white, and §11.3.6's
//! compositing formula of white over white is white whatever the two alphas are. So every pixel of
//! the page is white, opaque, and `255` in every channel.
//!
//! The backdrop's straight colour is its premultiplied colour divided by its alpha, which rounds a
//! hair past `1.0` at some partial alphas, and `ClipColor` then divided zero by zero: `issue18032`
//! showed a black pixel inside a white glyph.
#[path = "../../../tools/conformance/test-support/gpu_lock.rs"]
mod gpu_lock;
#[path = "../../../tools/conformance/test-support/gpu_lock_hook.rs"]
mod gpu_lock_hook;

use pdf_render::{
    BlendMode, Color, Command, DisplayList, FillRule, Medium, Paint, Path, PathCommand, Point,
    Rasterizer as _, Size, TargetSpec, Transform,
};
use render_raster::QuorraRasterizer;
use std::sync::Arc;

const WHITE: Paint = Paint::Solid(Color {
    r: 1.0,
    g: 1.0,
    b: 1.0,
    a: 1.0,
});

fn fill(path: Path, transform: Transform) -> Command {
    Command::Fill {
        path: Arc::new(path),
        transform,
        fill_rule: FillRule::NonZero,
        paint: WHITE,
        clip: None,
        mask: None,
        blend: BlendMode::Normal,
    }
}

fn rectangle(path: &mut Path, (x0, y0): (f32, f32), (x1, y1): (f32, f32)) {
    path.extend(&[
        PathCommand::MoveTo(Point::new(x0, y0)),
        PathCommand::LineTo(Point::new(x1, y0)),
        PathCommand::LineTo(Point::new(x1, y1)),
        PathCommand::LineTo(Point::new(x0, y1)),
        PathCommand::Close,
    ]);
}

/// `issue18032`'s lower-case "d", in its glyph space, as the document's font states it.
fn the_glyph_d() -> Path {
    let p = Point::new;
    let mut path = Path::new();
    path.extend(&[
        PathCommand::MoveTo(p(0.297, 0.113)),
        PathCommand::CurveTo(p(0.225, 0.113), p(0.193, 0.174), p(0.193, 0.263)),
        PathCommand::CurveTo(p(0.193, 0.36), p(0.228, 0.411), p(0.295, 0.411)),
        PathCommand::CurveTo(p(0.337, 0.411), p(0.366, 0.395), p(0.401, 0.362)),
        PathCommand::LineTo(p(0.401, 0.165)),
        PathCommand::CurveTo(p(0.375, 0.138), p(0.338, 0.113), p(0.297, 0.113)),
        PathCommand::Close,
        PathCommand::MoveTo(p(0.237, -0.008)),
        PathCommand::CurveTo(p(0.316, -0.008), p(0.366, 0.032), p(0.401, 0.08)),
        PathCommand::CurveTo(p(0.401, 0.053), p(0.406, 0.013), p(0.413, 0.0)),
        PathCommand::LineTo(p(0.578, 0.0)),
        PathCommand::CurveTo(p(0.566, 0.027), p(0.562, 0.066), p(0.562, 0.13)),
        PathCommand::LineTo(p(0.562, 0.74)),
        PathCommand::LineTo(p(0.401, 0.74)),
        PathCommand::LineTo(p(0.401, 0.466)),
        PathCommand::CurveTo(p(0.363, 0.518), p(0.311, 0.544), p(0.239, 0.544)),
        PathCommand::CurveTo(p(0.117, 0.544), p(0.032, 0.446), p(0.032, 0.262)),
        PathCommand::CurveTo(p(0.032, 0.078), p(0.116, -0.008), p(0.237, -0.008)),
        PathCommand::Close,
    ]);
    path
}

#[test]
fn a_colour_group_over_partly_covered_white_leaves_every_pixel_white() {
    // `issue18032`'s page: the glyph at nine points where the document draws it.
    let size = Size {
        width: 612.0,
        height: 792.0,
    };
    let glyph = fill(
        the_glyph_d(),
        Transform {
            a: 9.0,
            b: 0.0,
            c: 0.0,
            d: 9.0,
            e: 220.8013,
            f: 632.9346,
        },
    );
    // And a ramp of every partial alpha: column `k` of a strip covered `(k + 0.5) / 256` across.
    let mut ramp = Path::new();
    for k in 0..256_u16 {
        let x = f32::from(k) + 300.0;
        rectangle(
            &mut ramp,
            (x, 20.0),
            (x + (f32::from(k) + 0.5) / 256.0, 36.0),
        );
    }
    let mut page = Path::new();
    rectangle(&mut page, (0.0, 0.0), (size.width, size.height));

    let mut list = DisplayList::new(size);
    list.push(glyph);
    list.push(fill(ramp, Transform::IDENTITY));
    list.push(Command::Group {
        commands: vec![fill(page, Transform::IDENTITY)],
        alpha: 1.0,
        clip: None,
        mask: None,
        blend: BlendMode::Color,
        isolated: true,
        knockout: true,
        alpha_is_shape: false,
        blending: None,
    });
    let target = TargetSpec::for_page(&list, 1.0, 64 << 20).expect("a page-sized target");
    gpu_lock_hook::install();
    let mut backend = QuorraRasterizer::new_headless()
        .expect("real adapter")
        .with_medium(Medium::NONE);
    eprintln!("neutral colour blend: {}", backend.adapter_description());
    let frame = backend.rasterize(&list, target).expect("the page");
    let width = frame.width as usize;
    let off: Vec<(usize, usize, &[u8; 4])> = frame
        .data
        .as_chunks::<4>()
        .0
        .iter()
        .enumerate()
        .filter(|(_, pixel)| **pixel != [255, 255, 255, 255])
        .map(|(index, pixel)| (index % width, index / width, pixel))
        .take(8)
        .collect();
    assert!(
        off.is_empty(),
        "white under Color over white is white (§11.3.5.3): {off:?}"
    );
}

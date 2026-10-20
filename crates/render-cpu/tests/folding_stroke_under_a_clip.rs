//! A folding stroke under a clip that removes nothing is drawn as it is drawn alone, to the byte —
//! ISO 32000-2 §10.7.4, ADR 2106.
//!
//! # What the expected values come from
//!
//! §10.7.4's clipping paragraph intersects the mark's set of pixels with the region's:
//!
//! > Subsequent painting operations shall affect a region that is the intersection of the set of
//! > pixels defined by the clipping region with the set of pixels for the region to be painted.
//!
//! so a region containing the mark leaves it as it was. The same clause makes a pixel the
//! half-open square `[i, i+1) × [j, j+1)` on integer device coordinates and says a shape's
//! "coordinates are mapped into device space but not rounded to device pixel boundaries", and
//! nothing in either depends on where a surface the backend draws into begins. So the two drawings
//! below owe the same pixels, and the expected value of each is the other.
//!
//! # The scene is the witness's own
//!
//! The paperclip of veraPDF's `PDF_A-4` `6-3-1-t01-fail-f`, a `0.59 w` round-joined stroke whose
//! top bends more tightly than its half-width at 10x, so `render_cpu` strokes it as ADR 1348's
//! union of convex pieces and `tiny-skia`'s supersampled converter measures them. The target's
//! transform is that page's at 10x, to the bit, and the clip is the appearance's own `/BBox`,
//! which `Do` keeps (§8.10.2).
//!
//! # Why this scene, and what it is red on
//!
//! A clip that removes nothing is neither composed into the mark nor allowed to band its draw
//! (ADR 2106). A band is a surface that starts some rows down the page, and the page's matrix
//! reaches it with those rows taken off its translation — the same real number, rounded in
//! another binade (ADR 0219). This stroke has an edge within an `ulp` of one of the library's
//! sixteen samples at pixel `(798, 567)`: drawn in the 173 rows of its `/BBox` that pixel was 240,
//! and on the page 255.

#![expect(
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    clippy::cast_precision_loss,
    reason = "test code: a panic with a message is the intended failure mode, and the arithmetic \
              is over rasters this file sized itself"
)]

use std::sync::Arc;

use pdf_render::{
    BlendMode, Clip, Color, Command, DisplayList, FillRule, LineCap, LineJoin, Paint, Path,
    PathCommand, Point, Raster, Rasterizer, Size, Stroke, TargetSpec, Transform,
};
use render_cpu::CpuRasterizer;

/// The witness's paperclip, in the appearance stream's own space: its `m`, `c`, `l` and `h`
/// operands, in order.
const PAPERCLIP: &str = "0.51 13.63 m 0.51 13.25 0.48 4.38 0.48 3.74 c 0.48 3.29 0.49 1.93 1.38 \
    1.05 c 1.89 0.55 2.59 0.31 3.45 0.32 c 5.46 0.36 6.60 1.61 6.57 3.76 c 6.56 4.66 6.57 10.39 \
    6.57 10.45 c 6.57 10.70 6.36 10.90 6.11 10.90 c 5.86 10.90 5.65 10.70 5.65 10.44 c 5.65 10.21 \
    5.64 4.66 5.65 3.75 c 5.67 2.09 4.95 1.27 3.44 1.24 c 2.83 1.23 2.35 1.39 2.03 1.71 c 1.40 \
    2.32 1.40 3.39 1.40 3.75 c 1.40 4.37 1.43 13.24 1.43 13.63 c 1.43 13.97 1.52 15.65 3.03 15.65 \
    c 3.91 15.65 4.29 15.09 4.29 13.77 c 4.29 13.63 l 4.30 13.30 4.28 9.30 4.27 7.24 c 4.27 7.23 \
    4.27 7.22 4.27 7.21 c 4.28 7.00 4.25 6.32 3.96 6.03 c 3.87 5.94 3.76 5.89 3.60 5.90 c 2.85 \
    5.91 2.86 7.23 2.86 7.24 c 2.84 10.81 l 2.83 11.07 2.63 11.27 2.37 11.27 c 2.12 11.27 1.92 \
    11.06 1.92 10.81 c 1.94 7.24 l 1.93 6.47 2.26 5.00 3.59 4.98 c 3.99 4.97 4.35 5.12 4.62 5.40 \
    c 5.21 6.01 5.20 7.06 5.19 7.24 c 5.19 7.50 5.22 13.24 5.20 13.66 c 5.20 13.77 l 5.21 14.92 \
    4.92 15.61 4.51 16.03 c 4.08 16.45 3.53 16.57 3.03 16.57 c 1.05 16.57 0.52 14.72 0.51 13.63 c \
    h";

/// The witness page's matrix at 10x, from the appearance's space to the device, to the bit: the
/// annotation's `/Rect` corner `(75.0341, 720.288)` on a page 792 tall, as the interpreter
/// composes it.
const AT_TEN: Transform = Transform {
    a: 10.0,
    b: 0.0,
    c: 0.0,
    d: -10.0,
    e: 750.341_06,
    f: 717.119_6,
};

/// The surface: wide and tall enough for the paperclip, and tall enough that its clip's rows
/// start where the witness's do.
const WIDTH: u32 = 830;
const HEIGHT: u32 = 720;

/// The paperclip as a path, parsed from [`PAPERCLIP`].
fn paperclip() -> Path {
    let mut path = Path::new();
    let mut operands: Vec<f32> = Vec::new();
    for word in PAPERCLIP.split_whitespace() {
        let point = |at: usize| Point::new(operands[at], operands[at + 1]);
        match word {
            "m" => path.push(PathCommand::MoveTo(point(0))),
            "l" => path.push(PathCommand::LineTo(point(0))),
            "c" => path.push(PathCommand::CurveTo(point(0), point(2), point(4))),
            "h" => path.push(PathCommand::Close),
            number => {
                operands.push(number.parse().expect("an operand"));
                continue;
            }
        }
        operands.clear();
    }
    path
}

/// The appearance's `/BBox`, which contains the stroke and its round joins.
fn bounding_box() -> Path {
    let mut path = Path::new();
    path.push(PathCommand::MoveTo(Point::new(0.0, 0.0)));
    path.push(PathCommand::LineTo(Point::new(7.0, 0.0)));
    path.push(PathCommand::LineTo(Point::new(7.0, 17.0)));
    path.push(PathCommand::LineTo(Point::new(0.0, 17.0)));
    path.push(PathCommand::Close);
    path
}

/// The paperclip stroked as the witness strokes it, under its `/BBox` where `clipped`.
fn scene(clipped: bool) -> DisplayList {
    let mut list = DisplayList::new(Size::new(WIDTH as f32, HEIGHT as f32));
    let clip = clipped.then(|| {
        list.add_clip(Clip {
            path: bounding_box(),
            transform: Transform::IDENTITY,
            fill_rule: FillRule::NonZero,
            parent: None,
        })
        .expect("one clip fits in a display list")
    });
    list.push(Command::Stroke {
        path: Arc::new(paperclip()),
        transform: Transform::IDENTITY,
        stroke: Stroke {
            width: 0.59,
            cap: LineCap::Butt,
            join: LineJoin::Round,
            miter_limit: 4.0,
            dash_array: Vec::new(),
            dash_phase: 0.0,
            adjust: false,
        },
        paint: Paint::Solid(Color::BLACK),
        clip,
        mask: None,
        blend: BlendMode::Normal,
    });
    list
}

/// `list` drawn in one strip on a white surface under `transform`.
fn drawn(list: &DisplayList, (width, height): (u32, u32), transform: Transform) -> Raster {
    let target = TargetSpec {
        width,
        height,
        transform,
    };
    CpuRasterizer::new()
        .with_strips(1)
        .rasterize(list, target)
        .expect("the oracle draws one stroke")
}

/// The ink at `(x, y)`, in levels of 255: the black stroke's coverage on white.
fn ink(raster: &Raster, (x, y): (u32, u32)) -> u8 {
    let at = ((y * raster.width + x) * 4) as usize;
    255 - raster.data[at]
}

/// The clause: a region containing the mark leaves every pixel of it as it was.
#[test]
fn a_clip_that_removes_nothing_leaves_a_folding_stroke_as_it_was() {
    let bare = drawn(&scene(false), (WIDTH, HEIGHT), AT_TEN);
    let clipped = drawn(&scene(true), (WIDTH, HEIGHT), AT_TEN);
    let mut worst = (0_u8, (0, 0));
    for y in 0..HEIGHT {
        for x in 0..WIDTH {
            let apart = ink(&bare, (x, y)).abs_diff(ink(&clipped, (x, y)));
            if apart > worst.0 {
                worst = (apart, (x, y));
            }
        }
    }
    assert!(
        worst.0 == 0,
        "pixel {:?} is {} unclipped and {} under a clip that contains the whole stroke — §10.7.4's \
         intersection with a region containing the mark is the mark",
        worst.1,
        ink(&bare, worst.1),
        ink(&clipped, worst.1)
    );
}

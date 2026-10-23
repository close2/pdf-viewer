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
//! union of convex pieces, measured from their union's boundary (ADR 2122). The target's
//! transform is that page's at 10x, to the bit, and the clip is the appearance's own `/BBox`,
//! which `Do` keeps (§8.10.2).
//!
//! # Why this scene, and what it is red on
//!
//! A clip that removes nothing is neither composed into the mark nor allowed to band its draw
//! (ADR 2106). A band is a surface that starts some rows down the page, and the page's matrix
//! reaches it with those rows taken off its translation — the same real number, rounded in
//! another binade (ADR 0219). This stroke has an edge within an `ulp` of one of the library's
//! sixteen samples at pixel `(798, 567)`, so the library's converter, drawing it in the 173 rows of
//! its `/BBox`, reads 240 there, and on the page 255.
//!
//! # And what a folding stroke's pixel is owed at all
//!
//! The same pixel is the witness for the second test. §10.7.4 measures a shape after its curves
//! are flattened:
//!
//! > At this level, curves have been flattened to sequences of straight lines, and all
//! > "insideness" computations have been performed.
//!
//! and §10.7.2 leaves how finely to this processor:
//!
//! > PDF processors may choose to ignore any flatness tolerance specified within a PDF file.
//!
//! So a pixel is owed the area of the **flattened** set over its square, which `render_cpu`
//! measures exactly from the union's boundary (ADR 2122) — `stroke::folding`'s own tests hold that
//! measure to the same pieces drawn sixteen times finer — and the flattened set lies within the
//! chords' flatness, a twentieth of a device pixel (`doc/questions/A559`, ADR 2252), of the curve's.
//! The test here holds the page to the curve by that bound: the stroke drawn sixteen times finer,
//! its own chords sixteen times closer, averaged over the sixteen-by-sixteen block that becomes
//! each pixel.

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

/// The witness pixel's neighbourhood, in device pixels: the fold at the paperclip's top.
const WINDOW: (u32, u32) = (794, 563);

/// Its side, in device pixels.
const WINDOW_SIDE: u32 = 8;

/// How much finer the ladder draws the window.
const LADDER: u32 = 16;

/// The witness pixel itself, whose integral ADR 2106 §1 measured.
const WITNESS: (u32, u32) = (798, 567);

/// How far a pixel may read from its ladder, in levels of 255: the flattening's, not the
/// measure's. A boundary crosses one pixel over at most `√2` of it, so two sets whose boundaries lie
/// within `d` of each other differ inside a pixel by at most `√2 · d` of its area: the page's chords
/// lie within a twentieth of a device pixel of their curve and the ladder's sixteen times closer
/// (ADR 2252), and each drawing rounds its level once, half a level apiece.
const FLATTENING: f64 = 255.0 * std::f64::consts::SQRT_2 * (1.0 / 20.0 + 1.0 / 320.0) + 1.0;

/// §10.7.4 and §10.7.2: a folding stroke covers each pixel by the area of its flattened set there,
/// which lies within the chords' flatness of the curve's, as the stroke drawn [`LADDER`] times finer
/// measures it.
#[test]
fn a_folding_stroke_covers_a_pixel_by_its_area() {
    let page = drawn(&scene(false), (WIDTH, HEIGHT), AT_TEN);
    // The page's matrix scaled by the ladder and moved so that the window's corner is the
    // ladder's origin.
    let side = WINDOW_SIDE * LADDER;
    let scale = LADDER as f32;
    let finer = Transform {
        a: AT_TEN.a * scale,
        b: 0.0,
        c: 0.0,
        d: AT_TEN.d * scale,
        e: (AT_TEN.e - WINDOW.0 as f32) * scale,
        f: (AT_TEN.f - WINDOW.1 as f32) * scale,
    };
    let ladder = drawn(&scene(false), (side, side), finer);
    let mut worst = (0.0_f64, (0, 0), 0_u8, 0.0_f64);
    let mut witness = (0_u8, 0.0_f64);
    for y in 0..WINDOW_SIDE {
        for x in 0..WINDOW_SIDE {
            let mut sum = 0_u32;
            for v in 0..LADDER {
                for u in 0..LADDER {
                    sum += u32::from(ink(&ladder, (x * LADDER + u, y * LADDER + v)));
                }
            }
            let area = f64::from(sum) / f64::from(LADDER * LADDER);
            let drawn = ink(&page, (WINDOW.0 + x, WINDOW.1 + y));
            let apart = (f64::from(drawn) - area).abs();
            if (WINDOW.0 + x, WINDOW.1 + y) == WITNESS {
                witness = (drawn, area);
            }
            if apart > worst.0 {
                worst = (apart, (WINDOW.0 + x, WINDOW.1 + y), drawn, area);
            }
        }
    }
    assert!(
        (f64::from(witness.0) - witness.1).abs() <= FLATTENING,
        "the witness pixel {WITNESS:?} reads {} where the stroke drawn {LADDER} times finer covers \
         {:.1} of 255 — past the flatness §10.7.2 leaves to this processor",
        witness.0,
        witness.1
    );
    assert!(
        worst.0 <= FLATTENING,
        "pixel {:?} reads {} where the stroke drawn {LADDER} times finer covers {:.1} of 255 — \
         past the flatness §10.7.2 leaves to this processor",
        worst.1,
        worst.2,
        worst.3
    );
    // Printed, so that a run states the figures the bound holds and not only that it held.
    println!(
        "witness {WITNESS:?}: {} against {:.1}; worst {:?}: {} against {:.1}, bound {FLATTENING:.1}",
        witness.0, witness.1, worst.1, worst.2, worst.3
    );
}

//! ISO 32000-2 §11.7.4.3's special overprinting blend mode, drawn by this backend
//! (`doc/adr/1295`).
//!
//! The clause's first bullet decides the blend function per component:
//!
//! > If the overprint mode is 1 (nonzero overprint mode) and the current colour space and
//! > group colour space are both DeviceCMYK , then process colour components with nonzero
//! > values shall replace the corresponding component values of the backdrop; components
//! > with zero values leave the existing backdrop value unchanged.
//!
//! `pdf_render::Overprint` carries which channels are left alone, and this backend states them
//! as raster's per-channel compositing operator. The expectations are the clause's closed form
//! over an opaque backdrop: a kept channel is the backdrop's value, every other channel is
//! source-over — `α·Cs + (1 − α)·Cb` — and the pixel stays opaque. A fill takes the operator
//! directly and a stroke through a group of one, so both are drawn.
//!
//! The last test is the first meeting ADR 1295 promised: raster's arithmetic was derived from the
//! clauses without reading the CPU backend's, and here the two draw the same lists — every kept
//! set, over a translucent backdrop, with an oblique edge — and are compared.

#![expect(
    clippy::panic,
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    reason = "test code: a missing adapter is a failure rather than a skip (ADR 0004), the \
              expects live in helpers the allow-expect-in-tests config cannot see, and the \
              arithmetic indexes a raster this file just drew"
)]

use std::sync::Arc;

use pdf_render::display_list::Command;
use pdf_render::{
    BlendMode, Color, DisplayList, FillRule, LineCap, Overprint, Paint, Path, PathCommand, Point,
    Rasterizer, Size, Stroke, TargetSpec, Transform,
};
use render_cpu::CpuRasterizer;
use render_raster::QuorraRasterizer;

/// Pixel budget for a target; far above anything this test requests.
const GENEROUS: u64 = 1 << 30;

/// The page every fixture here is drawn on, one device pixel per unit.
const PAGE: Size = Size {
    width: 16.0,
    height: 16.0,
};

/// The opaque backdrop, every channel different.
const BACKDROP: Color = Color {
    r: 0.9,
    g: 0.2,
    b: 0.5,
    a: 1.0,
};

/// The mark: half opaque, so a replaced channel and a kept one differ by far more than a step.
const MARK: Color = Color {
    r: 0.1,
    g: 0.7,
    b: 0.9,
    a: 0.5,
};

/// Page space with its origin at the top left, so that path coordinates are device pixels.
fn top_down() -> Transform {
    Transform::new(1.0, 0.0, 0.0, -1.0, 0.0, PAGE.height)
}

fn rectangle(x0: f32, y0: f32, x1: f32, y1: f32) -> Arc<Path> {
    let mut path = Path::new();
    path.push(PathCommand::MoveTo(Point::new(x0, y0)));
    path.push(PathCommand::LineTo(Point::new(x1, y0)));
    path.push(PathCommand::LineTo(Point::new(x1, y1)));
    path.push(PathCommand::LineTo(Point::new(x0, y1)));
    path.push(PathCommand::Close);
    Arc::new(path)
}

fn fill(path: Arc<Path>, colour: Color, blend: BlendMode) -> Command {
    Command::Fill {
        path,
        transform: top_down(),
        fill_rule: FillRule::NonZero,
        paint: Paint::Solid(colour),
        clip: None,
        mask: None,
        blend,
    }
}

/// A page of the backdrop with `mark` over it, stating the mode as the interpreter does.
fn page(mark: Command) -> DisplayList {
    let mut list = DisplayList::new(PAGE);
    list.note_overprinting();
    list.push(fill(
        rectangle(0.0, 0.0, PAGE.width, PAGE.height),
        BACKDROP,
        BlendMode::Normal,
    ));
    list.push(mark);
    list.settle_overprinting();
    list
}

/// The raster backend, or a panic naming why the whole suite should fail rather than skip.
fn raster() -> QuorraRasterizer {
    QuorraRasterizer::new_headless().unwrap_or_else(|e| {
        panic!(
            "no adapter available for raster: {e}\n\
             Install a Vulkan driver (mesa-vulkan-drivers for a software one). These tests do \
             not skip, because a skipped suite reports success while verifying nothing."
        )
    })
}

/// One pixel of the drawn page, as the four bytes the raster holds.
fn pixel(raster: &mut QuorraRasterizer, list: &DisplayList, x: usize, y: usize) -> [u8; 4] {
    let target = TargetSpec::for_page(list, 1.0, GENEROUS).expect("valid target");
    let drawn = raster.rasterize(list, target).expect("the mode is drawn");
    let at = (y * drawn.width as usize + x) * 4;
    let mut out = [0; 4];
    out.copy_from_slice(&drawn.data[at..at + 4]);
    out
}

/// §11.7.4.3's value at a fully covered pixel of an opaque backdrop, in bytes.
fn clause(kept: [bool; 3]) -> [f32; 4] {
    let backdrop = [BACKDROP.r, BACKDROP.g, BACKDROP.b];
    let mark = [MARK.r, MARK.g, MARK.b];
    let mut out = [255.0; 4];
    for channel in 0..3 {
        out[channel] = 255.0
            * if kept[channel] {
                backdrop[channel]
            } else {
                MARK.a * mark[channel] + (1.0 - MARK.a) * backdrop[channel]
            };
    }
    out
}

fn assert_clause(drawn: [u8; 4], kept: [bool; 3], what: &str) {
    let want = clause(kept);
    for (channel, (&got, want)) in drawn.iter().zip(want).enumerate() {
        assert!(
            (f32::from(got) - want).abs() <= 1.0,
            "{what}, kept {kept:?}: channel {channel} is {got}, the clause gives {want:.1}"
        );
    }
}

#[test]
fn a_fill_under_the_mode_keeps_the_channels_it_names() {
    let mut raster = raster();
    for kept in [
        [true; 3],
        [false, true, true],
        [true, false, false],
        [false; 3],
    ] {
        let list = page(fill(
            rectangle(4.0, 4.0, 12.0, 12.0),
            MARK,
            BlendMode::Overprint(Overprint::new(kept)),
        ));
        assert!(list.overprints(), "the fixture must state the mode");
        assert_clause(pixel(&mut raster, &list, 8, 8), kept, "a fill");
        assert_clause(
            pixel(&mut raster, &list, 1, 1),
            [true; 3],
            "outside the fill",
        );
    }
}

#[test]
fn a_stroke_under_the_mode_keeps_the_channels_it_names() {
    let mut raster = raster();
    let mut line = Path::new();
    line.push(PathCommand::MoveTo(Point::new(2.0, 8.0)));
    line.push(PathCommand::LineTo(Point::new(14.0, 8.0)));
    let line = Arc::new(line);
    for kept in [[true; 3], [true, false, true]] {
        let list = page(Command::Stroke {
            path: Arc::clone(&line),
            transform: top_down(),
            stroke: Stroke {
                width: 4.0,
                cap: LineCap::Butt,
                ..Stroke::default()
            },
            paint: Paint::Solid(MARK),
            clip: None,
            mask: None,
            blend: BlendMode::Overprint(Overprint::new(kept)),
        });
        assert_clause(pixel(&mut raster, &list, 8, 7), kept, "a stroke");
        assert_clause(
            pixel(&mut raster, &list, 8, 1),
            [true; 3],
            "outside the stroke",
        );
    }
}

/// A wedge with an oblique edge, so that partially covered pixels exist.
fn wedge() -> Arc<Path> {
    let mut path = Path::new();
    path.push(PathCommand::MoveTo(Point::new(2.0, 2.0)));
    path.push(PathCommand::LineTo(Point::new(14.0, 3.0)));
    path.push(PathCommand::LineTo(Point::new(3.0, 14.0)));
    path.push(PathCommand::Close);
    Arc::new(path)
}

/// The two independent readings of §11.7.4.3 draw the same pixels: every kept set, a fill with an
/// oblique edge and a stroke, over a backdrop at alpha 0.6 so that destination-over is not the
/// backdrop unchanged.
#[test]
fn the_two_backends_meet_on_the_mode() {
    let mut raster = raster();
    let mut cpu = CpuRasterizer::new();
    let translucent = Color { a: 0.6, ..BACKDROP };
    let mut worst = 0_u8;
    for bits in 0_u8..8 {
        let kept = [bits & 1 != 0, bits & 2 != 0, bits & 4 != 0];
        let blend = BlendMode::Overprint(Overprint::new(kept));
        let mut list = DisplayList::new(PAGE);
        list.note_overprinting();
        list.push(fill(
            rectangle(0.0, 0.0, PAGE.width, PAGE.height),
            translucent,
            BlendMode::Normal,
        ));
        list.push(fill(wedge(), MARK, blend));
        let mut line = Path::new();
        line.push(PathCommand::MoveTo(Point::new(1.0, 12.5)));
        line.push(PathCommand::LineTo(Point::new(15.0, 11.0)));
        list.push(Command::Stroke {
            path: Arc::new(line),
            transform: top_down(),
            stroke: Stroke {
                width: 2.5,
                ..Stroke::default()
            },
            paint: Paint::Solid(Color { a: 0.7, ..MARK }),
            clip: None,
            mask: None,
            blend,
        });
        list.settle_overprinting();
        let target = TargetSpec::for_page(&list, 4.0, GENEROUS).expect("valid target");
        let oracle = cpu
            .rasterize(&list, target)
            .expect("the CPU backend draws the mode");
        let ours = raster
            .rasterize(&list, target)
            .expect("raster draws the mode");
        let comparison = raster_compare::compare(&oracle, &ours).expect("same dimensions");
        eprintln!(
            "kept {kept:?}: mean {:.4}, max {}, worst tile {:.3}, differing {:.4}",
            comparison.mean_error,
            comparison.max_error,
            comparison.worst_tile_error,
            comparison.differing_fraction
        );
        worst = worst.max(comparison.max_error);
        assert!(
            comparison.mean_error < 0.5 && comparison.worst_tile_error < 5.0,
            "kept {kept:?}: the two readings of §11.7.4.3 part: {comparison:?}"
        );
    }
    eprintln!("worst single channel over the eight kept sets: {worst} of 255");
}

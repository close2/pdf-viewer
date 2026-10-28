//! How many dots a dotted rule paints, on **all three** backends, against the two sentences that
//! decide it.
//!
//! ISO 32000-2 §8.4.3.6:
//!
//! > A stroking operation shall take no measures to coordinate the dash pattern with features of
//! > the path; it simply shall dispense dashes and gaps along the path in the pattern defined by
//! > the dash array.
//!
//! ISO 32000-2 §8.5.3.2, of a zero-length dash in the pattern of a subpath that is not degenerate:
//!
//! > In the latter case, the line caps shall always be painted, since their orientation is
//! > determined by the direction of the underlying path except in the case of a degenerate
//! > subpath.
//!
//! So `[0 2] 0 d` under round caps on a rule `L` units long paints a dot at every even distance
//! from 0 to `L`, both ends included: 51 dots for 100 units, 52 for 102 and still 51 for 101, the
//! last of which ends a unit past its last dot. The rule is 0.6 units wide, which is the dotted
//! table rule a corpus page drew 18 levels apart between the processor and raster at its last
//! dot, because `tiny-skia`'s dasher never dispenses a dash that begins at the contour's end and
//! `kurbo`'s does. `pdf_render::extended_to_a_zero_dash_at_the_end` states the rule for the three.
//!
//! **The count needs no tolerance on area.** The dots are two units apart and the rule is
//! horizontal or vertical, so at scale 1 every dot of one rule falls on the same sub-pixel phase
//! and rasterises to the same ink, and none reaches another's pixels; the rule's ink divided by
//! the ink of a rule one unit long, which holds the first dot alone, is the number of dots, an
//! integer, whatever any backend's anti-aliasing or §10.7.4's widening does to a circle 0.6
//! pixels across. Eight placements put the rule at each eighth of a pixel in both axes.

#![expect(
    clippy::arithmetic_side_effects,
    clippy::cast_precision_loss,
    reason = "test code: the page's own coordinates are the small numbers below, and a page of \
              160 x 160 pixels holds less ink than an f64 counts exactly"
)]
#![expect(
    clippy::expect_used,
    reason = "test code: a backend that cannot draw one dotted rule onto a 160 x 160 page is the \
              failure this file exists to report, and it reports it by name"
)]
#![expect(
    clippy::print_stdout,
    reason = "test code: each backend's count is the measurement, and `--nocapture` is how it is \
              read"
)]

use std::sync::Arc;

use pdf_render::{
    BlendMode, Color, Command, DisplayList, LineCap, LineJoin, Paint, Path, PathCommand, Point,
    Raster, Rasterizer, Size, Stroke, TargetSpec, Transform,
};
use render_raster::QuorraRasterizer;

/// The page every rule is drawn on, large enough for a rule of 102 units in either axis.
const PAGE: Size = Size {
    width: 160.0,
    height: 160.0,
};

/// The line width: a dotted table rule's.
const LINE: f32 = 0.6;

/// Where a rule starts before its placement's eighth of a pixel is added.
const ORIGIN: f32 = 20.0;

/// The way a rule is laid along the page.
#[derive(Clone, Copy, Debug)]
enum Lay {
    /// Left to right.
    Across,
    /// Right to left, from the far end: the same dots, so the same count.
    Back,
    /// Bottom to top.
    Up,
}

/// One rule `length` units long at placement `eighth`, its first point and its last.
fn ends(lay: Lay, length: f32, eighth: u8) -> (Point, Point) {
    let shift = f32::from(eighth) / 8.0;
    let (x, y) = (ORIGIN + shift, ORIGIN + shift);
    match lay {
        Lay::Across => (Point::new(x, y), Point::new(x + length, y)),
        Lay::Back => (Point::new(x + length, y), Point::new(x, y)),
        Lay::Up => (Point::new(x, y), Point::new(x, y + length)),
    }
}

/// The scene: one `m … l S` under `[0 2] 0 d` and `1 J`.
fn rule(from: Point, to: Point) -> DisplayList {
    let mut path = Path::new();
    path.push(PathCommand::MoveTo(from));
    path.push(PathCommand::LineTo(to));
    let mut list = DisplayList::new(PAGE);
    list.push(Command::Stroke {
        path: Arc::new(path),
        transform: Transform::IDENTITY,
        stroke: Stroke {
            width: LINE,
            cap: LineCap::Round,
            join: LineJoin::Miter,
            dash_array: vec![0.0, 2.0],
            ..Stroke::default()
        },
        paint: Paint::Solid(Color {
            r: 0.0,
            g: 0.0,
            b: 0.0,
            a: 1.0,
        }),
        clip: None,
        mask: None,
        blend: BlendMode::Normal,
    });
    list
}

/// The rule's ink over the whole page, in levels of the first channel.
fn ink(raster: &Raster) -> u64 {
    raster
        .data
        .chunks_exact(4)
        .map(|pixel| u64::from(255 - pixel[0]))
        .sum()
}

/// The end of a rule one unit long from `from` along the same line, which holds the first dot
/// alone: the next is dispensed two units on.
fn one_dot(from: Point, to: Point, length: f32) -> Point {
    Point::new(
        from.x + (to.x - from.x) / length,
        from.y + (to.y - from.y) / length,
    )
}

/// One backend: its name and a closure that draws a scene with it.
type Backend = (
    &'static str,
    Box<dyn FnMut(&DisplayList, TargetSpec) -> Raster>,
);

/// The three backends.
fn backends() -> Vec<Backend> {
    let mut cpu = render_cpu::CpuRasterizer::new();
    let mut raster = QuorraRasterizer::new_headless().expect("a raster device");
    let mut gpu = render_gpu::GpuRasterizer::new_headless().expect("a vello device");
    vec![
        (
            "processor",
            Box::new(move |list, target| cpu.rasterize(list, target).expect("a cpu raster")),
        ),
        (
            "raster",
            Box::new(move |list, target| raster.rasterize(list, target).expect("a raster raster")),
        ),
        (
            "vello",
            Box::new(move |list, target| gpu.rasterize(list, target).expect("a vello raster")),
        ),
    ]
}

/// How far a count may sit from the integer it is.
///
/// The processor and raster count to three decimals at every placement. Vello's ink for one dot
/// moves with where on the page the dot falls, by up to 0.22 of a dot on these rules, so the count
/// is read to the nearest dot: a dot missing or added moves it by a whole one.
const HALF_A_DOT: f64 = 0.5;

/// Every backend paints the dots §8.4.3.6 dispenses — the one at the rule's end included — at
/// each of eight placements, for an odd count, an even count and a rule that ends between dots.
#[test]
fn every_backend_paints_the_dot_at_the_rules_end() {
    let mut drawn = backends();
    // (length, the dots at 0, 2, … up to it)
    for (length, stated) in [(100.0_f32, 51.0_f64), (102.0, 52.0), (101.0, 51.0)] {
        for lay in [Lay::Across, Lay::Back, Lay::Up] {
            for eighth in 0..8 {
                let (from, to) = ends(lay, length, eighth);
                let list = rule(from, to);
                let target = TargetSpec::for_page(&list, 1.0, 1 << 30).expect("a valid target");
                for (name, draw) in &mut drawn {
                    let counted = ink(&draw(&list, target)) as f64
                        / ink(&draw(&rule(from, one_dot(from, to, length)), target)) as f64;
                    if eighth == 0 {
                        println!("  {length} {lay:?} {name:>9}: {counted:.3} dots of {stated}");
                    }
                    assert!(
                        (counted - stated).abs() < HALF_A_DOT,
                        "{name} painted {counted:.3} dots on a {length}-unit rule laid {lay:?} at \
                         placement {eighth}/8, where §8.4.3.6 dispenses {stated}"
                    );
                }
            }
        }
    }
}

/// How far apart, in levels of one channel, the processor and raster may draw one pixel of the
/// rule: measured at 1, against the whole dot of 36 to 72 levels per column that the missing
/// last dot was.
const LANES_APART: u8 = 2;

/// The processor and raster draw the rule within [`LANES_APART`] of each other at every pixel and
/// every placement.
#[test]
fn the_processor_and_raster_agree_at_every_pixel_of_the_rule() {
    let mut cpu = render_cpu::CpuRasterizer::new();
    let mut raster = QuorraRasterizer::new_headless().expect("a raster device");
    let mut worst = 0_u8;
    for (length, lay) in [(100.0, Lay::Across), (102.0, Lay::Up), (100.0, Lay::Back)] {
        for eighth in 0..8 {
            let (from, to) = ends(lay, length, eighth);
            let list = rule(from, to);
            let target = TargetSpec::for_page(&list, 1.0, 1 << 30).expect("a valid target");
            let a = cpu.rasterize(&list, target).expect("a cpu raster");
            let b = raster.rasterize(&list, target).expect("a raster raster");
            assert_eq!(a.data.len(), b.data.len(), "one target, one raster size");
            let apart = a
                .data
                .iter()
                .zip(&b.data)
                .map(|(p, q)| p.abs_diff(*q))
                .max()
                .unwrap_or(0);
            worst = worst.max(apart);
            assert!(
                apart <= LANES_APART,
                "the processor and raster drew a {length}-unit rule laid {lay:?} at placement \
                 {eighth}/8 {apart} levels apart"
            );
        }
    }
    println!("  the lanes at most {worst} levels apart");
}

/// How many dotted subpaths the hostile probe draws, each of which ends on a dot.
const HOSTILE: usize = 200_000;

/// A path of [`HOSTILE`] rules two units long under `[0 2]`, every one of which ends on a dot and
/// is therefore extended: the most `pdf_render::extended_to_a_zero_dash_at_the_end` can add to one
/// path, one segment per subpath. Run alone, under `getrusage`, beside the same test at HEAD.
// not a gate: a probe of resident memory under a hostile path, read by its `getrusage` high-water
// mark beside HEAD's binary when the end-dash rule or its bound changes; a process shared with the
// other tests cannot measure it, and nothing in it asserts (ADR 2570).
#[test]
#[ignore = "a memory probe, read by its high-water mark rather than by an assertion"]
fn a_path_whose_every_subpath_ends_on_a_dot() {
    // `DOTTED_PROBE_LENGTH=2.5` draws the same two dots a subpath without reaching the rule, which
    // is the control: the high-water mark of the marks themselves.
    let length: f32 = std::env::var("DOTTED_PROBE_LENGTH")
        .ok()
        .and_then(|given| given.parse().ok())
        .unwrap_or(2.0);
    let mut path = Path::new();
    for index in 0..HOSTILE {
        let row = (index % 150) as f32 + 5.0;
        let column = ((index / 150) % 75) as f32 * 2.0 + 2.0;
        path.push(PathCommand::MoveTo(Point::new(column, row)));
        path.push(PathCommand::LineTo(Point::new(column + length, row)));
    }
    let mut list = DisplayList::new(PAGE);
    list.push(Command::Stroke {
        path: Arc::new(path),
        transform: Transform::IDENTITY,
        stroke: Stroke {
            width: LINE,
            cap: LineCap::Round,
            join: LineJoin::Miter,
            dash_array: vec![0.0, 2.0],
            ..Stroke::default()
        },
        paint: Paint::Solid(Color {
            r: 0.0,
            g: 0.0,
            b: 0.0,
            a: 1.0,
        }),
        clip: None,
        mask: None,
        blend: BlendMode::Normal,
    });
    let target = TargetSpec::for_page(&list, 1.0, 1 << 30).expect("a valid target");
    let mut cpu = render_cpu::CpuRasterizer::new();
    let drawn = cpu.rasterize(&list, target).expect("a cpu raster");
    println!("  {HOSTILE} dotted subpaths: {} levels of ink", ink(&drawn));
}

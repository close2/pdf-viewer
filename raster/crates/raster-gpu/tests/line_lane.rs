//! The line lane (ADR 0097): a one-segment stroke's exact area, computed in the shader.
//!
//! **The expected value is the clause's, by a route the shader does not take.** ISO 32000-2
//! §8.4.3.2 makes a stroke "all points whose perpendicular distance from the path in user
//! space is less than or equal to half the line width", and §8.4.3.3's Table 53 states the
//! three ends of an open segment. Each pixel's expected coverage below is that set's area in
//! the pixel's unit cell — ADR 0005's exact-area rule — taken in `f64` as the area of a convex
//! polygon clipped to the cell, the set's round ends inscribed with 2 048 vertices each, whose
//! missing area is under a ten-millionth of the end's (the inscribed polygon's deficit,
//! `1 − sin(2π/N)/(2π/N)` of a full disc). The shader takes the same area through circular
//! sectors in `f32`; the two meet only in the set they measure, which is what lets one check
//! the other.
//!
//! The ladder is the population the lane exists for and its neighbours: widths from a
//! twentieth of a pixel to two pixels, lengths from a fifteenth of a pixel to past one,
//! endpoints placed across pixel edges, all three ends, four directions — both device axes
//! and two diagonals — and a transform that scales the axes unequally, where the pen is an
//! ellipse, and one that turns them a quarter. Every line sits in a cell of its own, so a
//! pixel's coverage is one line's area.
//!
//! **And what the lane must not change**: a line under a transform that does not keep the
//! axes, or under a clip with a non-rectangular link, takes the stroke road and draws a
//! stroke's bytes exactly.

// Test-file lint policy as in m1.rs.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap,
    clippy::arithmetic_side_effects,
    clippy::too_many_lines
)]

use raster_gpu::{Coverage, Device, Options, RenderError, Target, Viewport};
use raster_scene::{
    Affine, BlendMode, Color, FillRule, LineCap, LineJoin, Paint, Point, Scene, SceneBuilder,
    Segment, Stroke,
};

mod common;

/// Pixels a side of the cell each line of the ladder is drawn in.
const CELL: u32 = 8;
/// Cells a side of the ladder's target.
const CELLS: u32 = 48;
/// The target's side, in pixels.
const SIDE: u32 = CELL * CELLS;
/// Vertices on each round end of the reference polygon.
const ARC: u32 = 2048;

/// One line of the ladder, in the scene's own space, with the transform it is placed by.
#[derive(Debug, Clone, Copy)]
struct Case {
    from: Point,
    to: Point,
    width: f32,
    cap: LineCap,
    transform: Affine,
}

/// The ladder: every combination below, one per cell, in a fixed order.
fn ladder() -> Vec<Case> {
    let widths = [0.05f32, 0.13, 0.3, 0.7, 1.0, 2.0];
    let lengths = [0.07f32, 0.5, 1.3];
    let starts = [0.1f32, 0.47, 0.93];
    let middles = [0.25f32, 0.5, 0.97];
    let caps = [LineCap::Butt, LineCap::Round, LineCap::Square];
    // The device direction: along x, along y, and two diagonals.
    let directions = [0.0f32, 90.0, 30.0, 135.0];
    // The scene's linear part: the identity, unequal scales, and a quarter turn.
    let linear = [
        (1.0f32, 0.0f32, 0.0f32, 1.0f32),
        (1.7, 0.0, 0.0, 0.6),
        (0.0, 1.0, -1.0, 0.0),
    ];
    let mut cases = Vec::new();
    for &(a, b, c, d) in &linear {
        for &width in &widths {
            for &length in &lengths {
                for (index, &start) in starts.iter().enumerate() {
                    let middle = middles[(index + cases.len()) % middles.len()];
                    for &cap in &caps {
                        for &direction in &directions {
                            cases.push((a, b, c, d, width, length, start, middle, cap, direction));
                        }
                    }
                }
            }
        }
    }
    assert!(
        cases.len() <= (CELLS * CELLS) as usize,
        "{} cases",
        cases.len()
    );
    cases
        .into_iter()
        .enumerate()
        .map(
            |(index, (a, b, c, d, width, length, start, middle, cap, direction))| {
                // The cell's corner in the device, and the transform that puts the line's own
                // origin a few pixels into it.
                let cell_x = (index as u32 % CELLS * CELL) as f32 + 3.0;
                let cell_y = (index as u32 / CELLS * CELL) as f32 + 3.0;
                let transform = Affine {
                    a,
                    b,
                    c,
                    d,
                    e: cell_x,
                    f: cell_y,
                };
                // In the line's own space, so that its device placement starts at `(start,
                // middle)` past the cell's pixel corner and runs `length` in `direction` whatever
                // the linear part is: the inverse of the linear part applied to a device offset.
                let det = a * d - b * c;
                let own =
                    |dx: f32, dy: f32| Point::new((d * dx - c * dy) / det, (a * dy - b * dx) / det);
                let (cos, sin) = (direction.to_radians().cos(), direction.to_radians().sin());
                Case {
                    from: own(start, middle),
                    to: own(start + length * cos, middle + length * sin),
                    width,
                    cap,
                    transform,
                }
            },
        )
        .collect()
}

/// The set §8.4.3.2 and Table 53 state, in the device, as a convex polygon.
///
/// The pen — the disc of half the width — under a linear part that keeps the axes is the
/// ellipse whose semi-axes along device x and y are half the width times each axis's stretch.
/// In the frame where that ellipse is the unit disc the set is the band of half-width one
/// around the segment with its ends; the polygon is built there and scaled back, which a
/// linear map does exactly.
fn reference_polygon(case: &Case) -> Vec<(f64, f64)> {
    let t = case.transform;
    let apply = |p: Point| {
        (
            f64::from(t.a) * f64::from(p.x) + f64::from(t.c) * f64::from(p.y) + f64::from(t.e),
            f64::from(t.b) * f64::from(p.x) + f64::from(t.d) * f64::from(p.y) + f64::from(t.f),
        )
    };
    let (from, to) = (apply(case.from), apply(case.to));
    let half = f64::from(case.width) / 2.0;
    let pen = (
        half * (f64::from(t.a).abs() + f64::from(t.c).abs()),
        half * (f64::from(t.b).abs() + f64::from(t.d).abs()),
    );
    let b = ((to.0 - from.0) / pen.0, (to.1 - from.1) / pen.1);
    let length = b.0.hypot(b.1);
    let d = (b.0 / length, b.1 / length);
    let n = (-d.1, d.0);
    let at = |u: f64, v: f64| {
        let unit = (u * d.0 + v * n.0, u * d.1 + v * n.1);
        (from.0 + unit.0 * pen.0, from.1 + unit.1 * pen.1)
    };
    match case.cap {
        LineCap::Butt => vec![
            at(0.0, -1.0),
            at(length, -1.0),
            at(length, 1.0),
            at(0.0, 1.0),
        ],
        LineCap::Square => vec![
            at(-1.0, -1.0),
            at(length + 1.0, -1.0),
            at(length + 1.0, 1.0),
            at(-1.0, 1.0),
        ],
        LineCap::Round => {
            let mut points = Vec::new();
            // Half a turn about `to`, from −n through d to +n, then half a turn about
            // `from`, from +n through −d to −n.
            for step in 0..=ARC {
                let theta = -std::f64::consts::FRAC_PI_2
                    + std::f64::consts::PI * f64::from(step) / f64::from(ARC);
                points.push(at(length + theta.cos(), theta.sin()));
            }
            for step in 0..=ARC {
                let theta = std::f64::consts::FRAC_PI_2
                    + std::f64::consts::PI * f64::from(step) / f64::from(ARC);
                points.push(at(theta.cos(), theta.sin()));
            }
            points
        }
    }
}

fn adapters() -> Vec<String> {
    // One name per driver: the same GPU appears through Vulkan and GL (compute_lane.rs).
    Device::adapter_names()
        .into_iter()
        .filter(|name| !name.contains("radeonsi"))
        .collect()
}

fn device_on(adapter: &str, coverage: Coverage) -> Device {
    Device::headless(&Options {
        adapter: Some(adapter.into()),
        coverage,
        ..Options::default()
    })
    .expect("an adapter that enumerated can be opened")
}

fn black() -> Color {
    Color::new(0.0, 0.0, 0.0, 1.0)
}

fn ladder_scene(cases: &[Case]) -> Scene {
    let mut builder = SceneBuilder::new();
    for case in cases {
        builder
            .line(
                (case.from, case.to),
                case.transform,
                (case.width, false),
                case.cap,
                black(),
                None,
                None,
            )
            .expect("a line the builder admits");
    }
    builder.finish()
}

fn draw(device: &mut Device, scene: &Scene) -> (Vec<u8>, raster_gpu::Counters) {
    draw_through(device, scene, Affine::IDENTITY)
}

/// The scene drawn through the view `view`.
fn draw_through(
    device: &mut Device,
    scene: &Scene,
    view: Affine,
) -> (Vec<u8>, raster_gpu::Counters) {
    let frame = device
        .render(scene, &Viewport::full(SIDE, SIDE, view), Target::Readback)
        .expect("the ladder is inside every budget");
    let counters = frame.counters();
    (common::headless::pixels(frame), counters)
}

/// **The lane's claim**: on every adapter and both lanes that make coverage on the device
/// side and the host's, every pixel of every line of the ladder carries the set's exact
/// area to one level of 255 — and the lane made no coverage tile for any of them.
#[test]
fn every_line_of_the_ladder_carries_its_exact_area() {
    ladder_carries_its_exact_area_through(Affine::IDENTITY, "the identity view");
}

/// **The view is the shader's** (viewer ADR 2022): a line's instance is stated in the page's
/// space and `line.wgsl` maps it by the view, so the ladder is held to the same set through
/// views that are not the identity — unequal scales with a translation, and a quarter turn,
/// under which the page's y stretch is the device's x. Each case's own transform is the
/// ladder's composed with the view's inverse, every coefficient a power of two times the
/// ladder's, so the device placement is the ladder's exactly and the expected areas are the
/// same polygons.
#[test]
fn every_line_of_the_ladder_carries_its_exact_area_through_a_view() {
    let stretched = Affine {
        a: 2.0,
        b: 0.0,
        c: 0.0,
        d: 0.5,
        e: -16.0,
        f: 8.0,
    };
    let turned = Affine {
        a: 0.0,
        b: 2.0,
        c: -2.0,
        d: 0.0,
        e: SIDE as f32,
        f: 0.0,
    };
    ladder_carries_its_exact_area_through(stretched, "unequal scales and a translation");
    ladder_carries_its_exact_area_through(turned, "a quarter turn");
}

/// The ladder drawn through `view`, its cases placed so that the device sees the ladder, held
/// to the set's exact area to one level on every adapter and both lanes.
fn ladder_carries_its_exact_area_through(view: Affine, what: &str) {
    let cases = ladder();
    let polygons: Vec<Vec<(f64, f64)>> = cases.iter().map(reference_polygon).collect();
    let back = view.invert().expect("the views here invert");
    let placed: Vec<Case> = cases
        .iter()
        .map(|case| Case {
            transform: case.transform.then(back),
            ..*case
        })
        .collect();
    for (case, placed) in cases.iter().zip(&placed) {
        assert_eq!(
            placed.transform.then(view),
            case.transform,
            "{what}: the placement is exact"
        );
    }
    let scene = ladder_scene(&placed);
    for adapter in adapters() {
        for coverage in [Coverage::Cpu, Coverage::Compute] {
            let mut device = device_on(&adapter, coverage);
            let (pixels, counters) = draw_through(&mut device, &scene, view);
            assert_eq!(
                counters.lanes.line as usize,
                cases.len(),
                "{adapter} {coverage:?}: every line took the line lane"
            );
            assert_eq!(
                counters.tiles, 0,
                "{adapter} {coverage:?}: the lane rasterises nothing"
            );
            let (mut worst, mut at) = (0.0f64, None);
            for (index, polygon) in polygons.iter().enumerate() {
                let (x0, y0) = (index as u32 % CELLS * CELL, index as u32 / CELLS * CELL);
                for y in y0..y0 + CELL {
                    for x in x0..x0 + CELL {
                        let expected = common::meet::in_pixel(polygon, x, y) * 255.0;
                        let alpha = f64::from(pixels[((y * SIDE + x) * 4 + 3) as usize]);
                        let off = (alpha - expected).abs();
                        if off > worst {
                            worst = off;
                            at = Some((index, x, y, alpha, expected));
                        }
                    }
                }
            }
            println!("{what}: {adapter} {coverage:?}: worst {worst:.3} levels at {at:?}");
            let detail = at.map(|(index, ..)| cases[index]);
            assert!(
                worst <= 1.0,
                "{what}: {adapter} {coverage:?}: a pixel is {worst:.3} levels from the set's \
                 area: {at:?} {detail:?}"
            );
        }
    }
}

/// A line is the stroke of its two-point outline: where both are exact — a butt or a
/// projecting end, whose set is a rectangle, under a transform that scales both axes alike —
/// the two lanes agree to one level; a round end is the stroker's chords on one side and the
/// arc on the other, and that difference is printed, not held, because it is the stroker's
/// flattening and not this lane's.
///
/// Unequal scales are left out on purpose: the stroke command carries one scalar device
/// width (ADR 0085), which is exact for a similarity and is not the set §8.4.3.2 states under
/// any other — the caller expands such a stroke itself — while the line's pen is the ellipse,
/// which the ladder above holds to the set.
#[test]
fn a_line_draws_what_the_stroke_of_its_outline_draws_where_both_are_exact() {
    let cases = ladder();
    let scene = ladder_scene(&cases);
    let mut device = device_on("llvmpipe", Coverage::Cpu);
    let mut builder = SceneBuilder::new();
    for case in &cases {
        let outline = device
            .upload_outline(&[Segment::MoveTo(case.from), Segment::LineTo(case.to)])
            .unwrap();
        builder
            .stroke(
                outline,
                case.transform,
                Stroke {
                    width: case.width,
                    adjust: false,
                    cap: case.cap,
                    join: LineJoin::Miter,
                    miter_limit: 10.0,
                },
                Paint::Solid(black()),
                None,
                BlendMode::Normal,
                None,
            )
            .unwrap();
    }
    let stroke_scene = builder.finish();
    let (lines, _) = draw(&mut device, &scene);
    let (stroked, _) = draw(&mut device, &stroke_scene);
    for cap in [LineCap::Butt, LineCap::Square, LineCap::Round] {
        let mut worst = 0u8;
        let similar = |c: &Case| {
            let t = c.transform;
            (t.a.abs() + t.c.abs() - (t.b.abs() + t.d.abs())).abs() < f32::EPSILON
        };
        for (index, case) in cases
            .iter()
            .enumerate()
            .filter(|(_, c)| c.cap == cap && similar(c))
        {
            let (x0, y0) = (index as u32 % CELLS * CELL, index as u32 / CELLS * CELL);
            for y in y0..y0 + CELL {
                for x in x0..x0 + CELL {
                    let at = ((y * SIDE + x) * 4 + 3) as usize;
                    let off = lines[at].abs_diff(stroked[at]);
                    assert!(
                        cap == LineCap::Round || off <= 1,
                        "{cap:?}: {off} levels at ({x}, {y}) for {case:?}"
                    );
                    worst = worst.max(off);
                }
            }
        }
        println!("{cap:?}: the line and the stroke differ by at most {worst} levels");
    }
}

/// The road the closed form does not reach is the stroke's, to the bit: under a transform
/// that turns the axes the pen is a turned ellipse, and the line draws the bytes of the
/// stroke of its outline, on the path lane.
#[test]
fn a_line_under_a_turn_is_the_stroke_of_its_outline() {
    let mut device = common::headless::device();
    let (from, to) = (Point::new(3.2, 4.1), Point::new(20.7, 13.3));
    let turned = Affine {
        a: 0.8,
        b: 0.6,
        c: -0.6,
        d: 0.8,
        e: 12.0,
        f: 2.0,
    };
    let stroke = Stroke {
        width: 1.3,
        adjust: false,
        cap: LineCap::Round,
        join: LineJoin::Miter,
        miter_limit: 1.0,
    };
    let mut builder = SceneBuilder::new();
    builder
        .line(
            (from, to),
            turned,
            (stroke.width, false),
            stroke.cap,
            black(),
            None,
            None,
        )
        .unwrap();
    let line = builder.finish();
    let outline = device
        .upload_outline(&[Segment::MoveTo(from), Segment::LineTo(to)])
        .unwrap();
    let mut builder = SceneBuilder::new();
    builder
        .stroke(
            outline,
            turned,
            stroke,
            Paint::Solid(black()),
            None,
            BlendMode::Normal,
            None,
        )
        .unwrap();
    let stroked = builder.finish();
    let (drawn, counters) = draw(&mut device, &line);
    assert_eq!((counters.lanes.line, counters.lanes.path), (0, 1));
    assert_eq!(drawn, draw(&mut device, &stroked).0);
}

/// Under a clip with a non-rectangular link the line meets the residue the way a stroke
/// does, because it is drawn as one: the same bytes, on the path lane.
#[test]
fn a_line_under_a_residue_is_the_stroke_of_its_outline() {
    let mut device = common::headless::device();
    let polygon = common::meet::polygon_path(&common::meet::clip_polygon());
    let (from, to) = (Point::new(2.0, 8.4), Point::new(14.5, 8.4));
    let mut builder = SceneBuilder::new();
    let clip = common::meet::residue_clip(&mut device, &mut builder, &polygon);
    builder
        .line(
            (from, to),
            Affine::IDENTITY,
            (2.5, false),
            LineCap::Square,
            black(),
            Some(clip),
            None,
        )
        .unwrap();
    let line = builder.finish();
    let outline = device
        .upload_outline(&[Segment::MoveTo(from), Segment::LineTo(to)])
        .unwrap();
    let mut builder = SceneBuilder::new();
    let clip = common::meet::residue_clip(&mut device, &mut builder, &polygon);
    builder
        .stroke(
            outline,
            Affine::IDENTITY,
            Stroke {
                width: 2.5,
                adjust: false,
                cap: LineCap::Square,
                join: LineJoin::Miter,
                miter_limit: 1.0,
            },
            Paint::Solid(black()),
            Some(clip),
            BlendMode::Normal,
            None,
        )
        .unwrap();
    let stroked = builder.finish();
    let (drawn, counters) = draw(&mut device, &line);
    assert_eq!(
        counters.lanes.line, 0,
        "a residue takes the line off its lane"
    );
    assert_eq!(drawn, draw(&mut device, &stroked).0);
}

/// §8.5.3.2's dot as a polygon of the device: "a filled circle centred at the single point",
/// which under a linear part that keeps the axes is the pen's ellipse about it, inscribed with
/// twice [`ARC`] vertices.
fn reference_dot(case: &Case) -> Vec<(f64, f64)> {
    let t = case.transform;
    let centre = (
        f64::from(t.a) * f64::from(case.from.x)
            + f64::from(t.c) * f64::from(case.from.y)
            + f64::from(t.e),
        f64::from(t.b) * f64::from(case.from.x)
            + f64::from(t.d) * f64::from(case.from.y)
            + f64::from(t.f),
    );
    let half = f64::from(case.width) / 2.0;
    let pen = (
        half * (f64::from(t.a).abs() + f64::from(t.c).abs()),
        half * (f64::from(t.b).abs() + f64::from(t.d).abs()),
    );
    (0..2 * ARC)
        .map(|step| {
            let theta = std::f64::consts::TAU * f64::from(step) / f64::from(2 * ARC);
            (
                centre.0 + pen.0 * theta.cos(),
                centre.1 + pen.1 * theta.sin(),
            )
        })
        .collect()
}

/// **The dot's claim** (viewer ADR 1857): a line of no length under a round cap is §8.5.3.2's
/// filled circle, the line's own set at length zero, and the lane carries its exact area to one
/// level on every adapter and both lanes — under the identity, unequal scales and a quarter
/// turn, at every placement across a pixel's corner. Under a butt or projecting cap the scene
/// refuses the line, the clause's "no output" being the caller's to give.
#[test]
fn a_round_line_of_no_length_carries_the_area_of_its_filled_circle() {
    let widths = [0.05f32, 0.13, 0.3, 0.7, 1.0, 2.0, 3.5];
    let places = [0.0f32, 0.1, 0.47, 0.5, 0.93];
    let linear = [
        (1.0f32, 0.0f32, 0.0f32, 1.0f32),
        (1.7, 0.0, 0.0, 0.6),
        (0.0, 1.0, -1.0, 0.0),
    ];
    let mut cases = Vec::new();
    for &(a, b, c, d) in &linear {
        for &width in &widths {
            for &x in &places {
                for &y in &places {
                    let index = cases.len() as u32;
                    let transform = Affine {
                        a,
                        b,
                        c,
                        d,
                        e: (index % CELLS * CELL) as f32 + 4.0,
                        f: (index / CELLS * CELL) as f32 + 4.0,
                    };
                    let det = a * d - b * c;
                    let own = Point::new((d * x - c * y) / det, (a * y - b * x) / det);
                    cases.push(Case {
                        from: own,
                        to: own,
                        width,
                        cap: LineCap::Round,
                        transform,
                    });
                }
            }
        }
    }
    assert!(cases.len() <= (CELLS * CELLS) as usize);
    for cap in [LineCap::Butt, LineCap::Square] {
        let refused = SceneBuilder::new().line(
            (Point::new(1.0, 1.0), Point::new(1.0, 1.0)),
            Affine::IDENTITY,
            (1.0, false),
            cap,
            black(),
            None,
            None,
        );
        assert!(refused.is_err(), "{cap:?}: a dot is the caller's no output");
    }
    let polygons: Vec<Vec<(f64, f64)>> = cases.iter().map(reference_dot).collect();
    let scene = ladder_scene(&cases);
    for adapter in adapters() {
        for coverage in [Coverage::Cpu, Coverage::Compute] {
            let mut device = device_on(&adapter, coverage);
            let (pixels, counters) = draw(&mut device, &scene);
            assert_eq!(
                counters.lanes.line as usize,
                cases.len(),
                "{adapter} {coverage:?}: every dot took the line lane"
            );
            let (mut worst, mut at) = (0.0f64, None);
            for (index, polygon) in polygons.iter().enumerate() {
                let (x0, y0) = (index as u32 % CELLS * CELL, index as u32 / CELLS * CELL);
                for y in y0..y0 + CELL {
                    for x in x0..x0 + CELL {
                        let expected = common::meet::in_pixel(polygon, x, y) * 255.0;
                        let alpha = f64::from(pixels[((y * SIDE + x) * 4 + 3) as usize]);
                        let off = (alpha - expected).abs();
                        if off > worst {
                            worst = off;
                            at = Some((index, x, y, alpha, expected));
                        }
                    }
                }
            }
            println!("{adapter} {coverage:?}: dots worst {worst:.3} levels at {at:?}");
            let detail = at.map(|(index, ..)| cases[index]);
            assert!(
                worst <= 1.0,
                "{adapter} {coverage:?}: a pixel is {worst:.3} levels from the circle's area: \
                 {at:?} {detail:?}"
            );
        }
    }
}

/// Where the closed form does not apply — a clip with a non-rectangular link — the dot takes
/// the stroke road, which expands nothing for a subpath of no length; it is drawn there as the
/// stroker's own two round caps about the point, and carries the area of the circle met with
/// the clip to within the caps' flattening.
#[test]
fn a_round_dot_under_a_residue_is_the_circle_met_with_the_clip() {
    let clip_points = common::meet::clip_polygon();
    let polygon = common::meet::polygon_path(&clip_points);
    // On the clip's right edge, so that the residue cuts the circle.
    let centre = Point::new(14.4, 8.6);
    let width = 3.0f32;
    for coverage in [Coverage::Cpu, Coverage::Gpu] {
        let mut device = common::meet::device_with(coverage, 1);
        let mut builder = SceneBuilder::new();
        let clip = common::meet::residue_clip(&mut device, &mut builder, &polygon);
        builder
            .line(
                (centre, centre),
                Affine::IDENTITY,
                (width, false),
                LineCap::Round,
                black(),
                Some(clip),
                None,
            )
            .unwrap();
        let scene = builder.finish();
        let frame = device
            .render(
                &scene,
                &Viewport::full(24, 24, Affine::IDENTITY),
                Target::Readback,
            )
            .expect("one dot is inside every budget");
        assert_eq!(
            frame.counters().lanes.line,
            0,
            "a residue takes the dot off its lane"
        );
        let pixels = common::headless::pixels(frame);
        let disc: Vec<(f64, f64)> = (0..2 * ARC)
            .map(|step| {
                let theta = std::f64::consts::TAU * f64::from(step) / f64::from(2 * ARC);
                (
                    f64::from(centre.x) + 1.5 * theta.cos(),
                    f64::from(centre.y) + 1.5 * theta.sin(),
                )
            })
            .collect();
        let met = common::meet::clip_convex(&disc, &clip_points);
        let (mut worst, mut ink) = (0.0f64, 0.0f64);
        for y in 0..24 {
            for x in 0..24 {
                let expected = common::meet::in_pixel(&met, x, y) * 255.0;
                let alpha = f64::from(pixels[((y * 24 + x) * 4 + 3) as usize]);
                worst = worst.max((alpha - expected).abs());
                ink += alpha;
            }
        }
        let area = common::meet::area(&met) * 255.0;
        println!("{coverage:?}: worst {worst:.3} levels, ink {ink:.1} against {area:.1}");
        assert!(
            worst <= 2.0,
            "{coverage:?}: {worst:.3} levels from the met circle"
        );
        assert!(
            (ink - area).abs() <= 0.01 * area + 4.0,
            "{coverage:?}: ink {ink} against {area}"
        );
    }
}

/// A rectangular clip is met per pixel inside the shader: a round-ended line cut by a clip
/// rectangle carries the area of the set inside it and nothing outside.
#[test]
fn a_rectangular_clip_cuts_the_line_inside_the_pixel() {
    let mut device = common::headless::device();
    let clip_outline = device
        .upload_outline(&common::scene::rect_outline(raster_scene::Rect::new(
            Point::new(4.25, 2.0),
            Point::new(9.6, 12.0),
        )))
        .unwrap();
    let mut builder = SceneBuilder::new();
    let clip = builder
        .clip(clip_outline, Affine::IDENTITY, FillRule::NonZero, None)
        .unwrap();
    let case = Case {
        from: Point::new(4.4, 6.3),
        to: Point::new(9.1, 8.9),
        width: 1.6,
        cap: LineCap::Round,
        transform: Affine::IDENTITY,
    };
    builder
        .line(
            (case.from, case.to),
            case.transform,
            (case.width, false),
            case.cap,
            black(),
            Some(clip),
            None,
        )
        .unwrap();
    let pixels = common::headless::render(&mut device, &builder.finish(), 16, 16);
    // The set inside the clip is the polygon clipped to the clip's rectangle, which is
    // counter-clockwise in the y-down device space as `clip_convex` wants its window.
    let inside = common::meet::clip_convex(
        &reference_polygon(&case),
        &[(4.25, 2.0), (9.6, 2.0), (9.6, 12.0), (4.25, 12.0)],
    );
    for y in 0..16u32 {
        for x in 0..16u32 {
            let expected = if inside.len() < 3 {
                0.0
            } else {
                common::meet::in_pixel(&inside, x, y)
            };
            let alpha = f64::from(pixels[((y * 16 + x) * 4 + 3) as usize]);
            assert!(
                (alpha - expected * 255.0).abs() <= 1.0,
                "({x}, {y}): {alpha} against {:.3}",
                expected * 255.0
            );
        }
    }
}

/// The pre-count a line is charged (ADR 0097): the larger of its two roads, a line
/// instance or a coverage quad — 64 bytes where a stroke is charged a rectangle and a quad,
/// 96. A budget between the two admits a frame of lines and refuses the same frame of
/// strokes, naming the stroke's 96 bytes a command.
#[test]
fn a_line_is_charged_less_than_a_stroke_before_the_walk() {
    const MARKS: u64 = 4_000;
    let budget = MARKS * 80;
    let mut device = Device::headless(&Options {
        adapter: Some("llvmpipe".into()),
        max_frame_bytes: budget,
        ..Options::default()
    })
    .unwrap();
    let outline = device
        .upload_outline(&[
            Segment::MoveTo(Point::new(1.0, 1.0)),
            Segment::LineTo(Point::new(1.0, 1.2)),
        ])
        .unwrap();
    let (mut lines, mut strokes) = (SceneBuilder::new(), SceneBuilder::new());
    for index in 0..MARKS {
        let at = Affine::translate((index % 60) as f32, (index / 60) as f32);
        lines
            .line(
                (Point::new(1.0, 1.0), Point::new(1.0, 1.2)),
                at,
                (0.1, false),
                LineCap::Round,
                black(),
                None,
                None,
            )
            .unwrap();
        strokes
            .stroke(
                outline,
                at,
                Stroke {
                    width: 0.1,
                    adjust: false,
                    cap: LineCap::Round,
                    join: LineJoin::Miter,
                    miter_limit: 1.0,
                },
                Paint::Solid(black()),
                None,
                BlendMode::Normal,
                None,
            )
            .unwrap();
    }
    let viewport = Viewport::full(64, 80, Affine::IDENTITY);
    device
        .render(&lines.finish(), &viewport, Target::Readback)
        .expect("a frame of lines fits a budget of 80 bytes a mark");
    let refused = device.render(&strokes.finish(), &viewport, Target::Readback);
    assert!(
        matches!(refused, Err(RenderError::FrameBudgetExceeded { needed, .. }) if needed == MARKS * 96),
        "{:?}",
        refused.map(|frame| frame.counters())
    );
}

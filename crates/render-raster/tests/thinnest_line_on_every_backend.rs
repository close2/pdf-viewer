//! ISO 32000-2 §8.4.3.2's thinnest line, §8.4.3.5's zero mitre length and a dash pattern past
//! `pdf_render::MAX_DASHES`, on **all three** backends.
//!
//! §8.4.3.2: "A line width of 0 shall denote the thinnest line that can be rendered at device
//! resolution: 1 device pixel wide." §8.4.3.5: "When the line width is zero, the miter length is
//! zero." And §8.5.3.2 paints a stroke "either solid or with a dash pattern", which no dash count
//! turns into no mark. Each is decided once in `pdf-render` (ADRs 3333 and 3334) and constructed
//! by each backend in its own way — `tiny-skia`'s hairline, raster's device-space expansion, and a
//! restated path under `vello` — so each is asserted here of every backend: the case audit 2155
//! found on the processor, and the same scene on the graphics device's two.
//!
//! Ink is summed over a box of device pixels rather than read at one pixel, because the marks are
//! one device pixel wide and a single sample would read anti-aliasing.

#![expect(
    clippy::arithmetic_side_effects,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "test code: the page's own coordinates are the small numbers below, and an index over \
              a raster this file sized itself cannot overflow"
)]
#![expect(
    clippy::expect_used,
    reason = "test code: a backend that cannot draw one stroked polyline onto a 100 x 100 page is \
              the failure this file exists to report, and it reports it by name"
)]

#[path = "../../../tools/conformance/test-support/gpu_lock.rs"]
mod gpu_lock;
#[path = "../../../tools/conformance/test-support/gpu_lock_hook.rs"]
mod gpu_lock_hook;

use std::sync::Arc;

use pdf_render::{
    BlendMode, Color, Command, DisplayList, LineCap, LineJoin, Paint, Path, PathCommand, Point,
    Raster, Rasterizer, Size, Stroke, TargetSpec, Transform,
};
use render_raster::QuorraRasterizer;

const PAGE: f32 = 100.0;

type Backend = (
    &'static str,
    Box<dyn FnMut(&DisplayList, TargetSpec) -> Raster>,
);

fn backends() -> Vec<Backend> {
    let mut cpu = render_cpu::CpuRasterizer::new();
    gpu_lock_hook::install();
    let mut raster = QuorraRasterizer::new_headless().expect("a raster device");
    let _vello_lock = gpu_lock::Hold::unless_software("vello-device");
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

/// One black stroke of the polyline `points` under `transform`.
fn polyline(points: &[(f32, f32)], transform: Transform, stroke: Stroke) -> DisplayList {
    let mut path = Path::new();
    for (index, &(x, y)) in points.iter().enumerate() {
        let at = Point::new(x, y);
        path.push(if index == 0 {
            PathCommand::MoveTo(at)
        } else {
            PathCommand::LineTo(at)
        });
    }
    let mut list = DisplayList::new(Size::new(PAGE, PAGE));
    list.push(Command::Stroke {
        path: Arc::new(path),
        transform,
        stroke,
        paint: Paint::Solid(Color::BLACK),
        clip: None,
        mask: None,
        blend: BlendMode::Normal,
    });
    list
}

/// Device pixels' worth of ink inside the page-space box `[x0, x1] × [y0, y1]`, at scale one.
fn ink_in(raster: &Raster, (x0, x1): (f32, f32), (y0, y1): (f32, f32)) -> f32 {
    let mut total = 0.0;
    for row in (PAGE - y1) as u32..(PAGE - y0) as u32 {
        for column in x0 as u32..x1 as u32 {
            let at = ((row * raster.width + column) * 4) as usize;
            total += f32::from(255 - raster.data[at]) / 255.0;
        }
    }
    total
}

fn line(width: f32, join: LineJoin) -> Stroke {
    Stroke {
        width,
        cap: LineCap::Butt,
        join,
        miter_limit: 10.0,
        ..Stroke::default()
    }
}

fn draw(list: &DisplayList) -> Vec<(&'static str, Raster)> {
    let target = TargetSpec::for_page(list, 1.0, 1 << 22).expect("a valid target");
    backends()
        .iter_mut()
        .map(|(name, draw)| (*name, draw(list, target)))
        .collect()
}

/// §8.4.3.2 under `4 0 0 1`: the zero-width line is one device pixel thick whether it runs along
/// the stretched axis or across it.
#[test]
fn a_zero_width_line_is_one_device_pixel_wide_in_every_direction() {
    let stretched = Transform::scale(4.0, 1.0);
    let zero = line(0.0, LineJoin::Miter);
    let horizontal = draw(&polyline(
        &[(2.5, 50.0), (22.5, 50.0)],
        stretched,
        zero.clone(),
    ));
    let vertical = draw(&polyline(&[(15.0, 10.0), (15.0, 40.0)], stretched, zero));
    let wrong: Vec<String> = horizontal
        .iter()
        .zip(&vertical)
        .filter_map(|((name, across), (_, along))| {
            let per_column = ink_in(across, (20.0, 80.0), (45.0, 55.0)) / 60.0;
            let per_row = ink_in(along, (55.0, 65.0), (15.0, 35.0)) / 20.0;
            (!(0.75..=1.5).contains(&per_column) || !(0.75..=1.5).contains(&per_row))
                .then(|| format!("{name}: {per_column} a column and {per_row} a row"))
        })
        .collect();
    assert_eq!(
        wrong,
        Vec::<String>::new(),
        "a zero-width line is one device pixel wide in both directions: {wrong:?}"
    );
}

/// §8.4.3.5: the zero-width line's mitre length is zero, so a 14° corner, whose ratio of 8.2 a
/// limit of 10 admits for any other width, puts no more ink beyond its vertex than a bevel.
#[test]
fn a_zero_width_line_has_no_mitre() {
    let corner = [(10.0, 50.0), (90.0, 50.0), (10.0, 70.0)];
    let beyond = ((90.75, 95.0), (46.0, 52.0));
    let mitred = draw(&polyline(
        &corner,
        Transform::IDENTITY,
        line(0.0, LineJoin::Miter),
    ));
    let bevelled = draw(&polyline(
        &corner,
        Transform::IDENTITY,
        line(0.0, LineJoin::Bevel),
    ));
    // Control: a line one pixel wide by its own width is mitred, and the box sees it.
    let control = draw(&polyline(
        &corner,
        Transform::IDENTITY,
        line(1.0, LineJoin::Miter),
    ));
    let mut wrong = Vec::new();
    for (((name, mitred), (_, bevelled)), (_, control)) in
        mitred.iter().zip(&bevelled).zip(&control)
    {
        let mitred = ink_in(mitred, beyond.0, beyond.1);
        let bevelled = ink_in(bevelled, beyond.0, beyond.1);
        let control = ink_in(control, beyond.0, beyond.1);
        assert!(
            control > bevelled + 1.0,
            "{name}: control, a one-pixel line's mitre reaches the box: {control}"
        );
        if mitred > bevelled + 0.3 {
            wrong.push(format!(
                "{name}: {mitred} mitred against {bevelled} bevelled"
            ));
        }
    }
    assert_eq!(
        wrong,
        Vec::<String>::new(),
        "a zero-width line has no mitre: {wrong:?}"
    );
}

/// §8.5.3.2's stroke under `[0.00001 0.00001] 0`, four million dashes along 80 units, is drawn at
/// the half of the line its butt-capped dashes cover (ADR 3334), on every backend.
#[test]
fn a_dash_pattern_past_the_bound_is_drawn_at_its_coverage() {
    let along = [(10.0, 50.0), (90.0, 50.0)];
    let dashed = |dash: f32| Stroke {
        width: 2.0,
        cap: LineCap::Butt,
        dash_array: if dash > 0.0 {
            vec![dash, dash]
        } else {
            Vec::new()
        },
        dash_phase: 0.0,
        ..Stroke::default()
    };
    let solid = draw(&polyline(&along, Transform::IDENTITY, dashed(0.0)));
    let finer = draw(&polyline(&along, Transform::IDENTITY, dashed(0.000_01)));
    let wrong: Vec<String> = solid
        .iter()
        .zip(&finer)
        .filter_map(|((name, solid), (_, finer))| {
            let share = ink_in(finer, (10.0, 90.0), (45.0, 55.0))
                / ink_in(solid, (10.0, 90.0), (45.0, 55.0));
            (!(0.45..=0.55).contains(&share)).then(|| format!("{name}: {share}"))
        })
        .collect();
    assert_eq!(
        wrong,
        Vec::<String>::new(),
        "a pattern past the bound is drawn at its coverage, half the line: {wrong:?}"
    );
}

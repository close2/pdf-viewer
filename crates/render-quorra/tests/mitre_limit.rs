//! The mitre ISO 32000-2 §8.4.3.5 admits, on **all three** backends, against the clause's own
//! arithmetic.
//!
//! §8.4.3.5 bounds one ratio and states it as a formula:
//!
//! > The miter limit shall impose a maximum on the ratio of the miter length to the line width
//! > (see "Figure 15 -Miter length"). When the limit is exceeded, the join is converted from a
//! > miter to a bevel.
//!
//! `miterLength / lineWidth = 1 / sin(φ / 2)` for the angle φ between the segments, so a join at or
//! under the limit is drawn to the length its own angle implies and one over it is a bevel. The
//! mitre length spans the whole join — inner crossing to outer crossing — so the **tip sits
//! `(w/2) / sin(φ/2)` from the vertex**, and that is the number every assertion here is written
//! against. Nothing in this file is derived from another renderer: `mutool` and `ghostscript`
//! putting the tip within a few pixels of the same place is evidence that the reading is right, and
//! `poppler` and this tree's own processor drawing a bevel there for its whole life is what
//! `doc/todo/11` §6 was about.
//!
//! # Why all three backends and why this file exists
//!
//! The three strokers answered differently, which makes this trap 2's shape rather than one
//! library's bug: `tiny-skia` bevels every ratio over 90.51 before it reads the limit, while
//! `kurbo` and quorra draw the spike. A fix in the backend that was wrong is only half of the
//! answer — the other half is a scene that holds each backend to the clause, so that the day a
//! library changes its mind in either direction, this fails and names which one.
//!
//! Two magnitudes, on purpose (trap 2's fifth instance). The **ratio** is the corpus witness's
//! 166.676, which is above what one stroker draws and inside the `333 M` the file states; and the
//! **coverage** is fractional the whole way, because a spike that long is under a pixel wide over
//! most of its length — so what is asserted is ink against an area rather than a shape against a
//! shape, and a construction that snapped the spike to whole pixels would fail as surely as one
//! that dropped it.
//!
//! `render-quorra/examples/mitre_ladder` is the instrument to run when this fails: it prints what
//! each backend draws at every rung from 45° down to 0.2°.

#![expect(
    clippy::arithmetic_side_effects,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "test code: the page's own coordinates are the small numbers below, and an index over \
              a raster this file sized itself cannot overflow"
)]
#![expect(
    clippy::expect_used,
    reason = "test code: a backend that cannot draw one stroked path onto a 200 x 1200 page is the \
              failure this file exists to report, and it reports it by name"
)]
#![expect(
    clippy::print_stdout,
    reason = "test code: the ink beside the closed form is the measurement, and `--nocapture` is \
              how it is read"
)]

use std::sync::Arc;

use pdf_render::{
    BlendMode, Color, Command, DisplayList, LineCap, LineJoin, Paint, Path, PathCommand, Point,
    Raster, Rasterizer, Size, Stroke, TargetSpec, Transform,
};
use render_quorra::QuorraRasterizer;

/// The page every scene is drawn on: tall enough to contain the tip the clause puts 833 units up.
const PAGE: Size = Size {
    width: 200.0,
    height: 1200.0,
};

/// Where the two segments meet, in page space.
const VERTEX: Point = Point { x: 100.0, y: 100.0 };

/// The line width, which is `LargeMitreLimit.pdf`'s own.
const WIDTH: f32 = 10.0;

/// The limit the corpus witness states, and the one this file uses where a mitre is admitted.
const LIMIT: f32 = 333.0;

/// `LargeMitreLimit.pdf`'s join, reduced to four points and translated so that the tip fits.
///
/// The outgoing segment leaves the vertex 0.9 units sideways over 75 down, so
/// φ = atan(0.9 / 75) = 0.687516°, the ratio is 166.676 — inside `333 M` — and both segments
/// descend from the vertex, which is what makes "ink above the vertex" the wedge's own area with
/// nothing of the stroke's body in it.
fn witness(limit: f32, alpha: f32) -> DisplayList {
    let mut path = Path::new();
    path.push(PathCommand::MoveTo(Point::new(75.0, VERTEX.y - 75.0)));
    path.push(PathCommand::LineTo(Point::new(100.0, VERTEX.y - 50.0)));
    path.push(PathCommand::LineTo(VERTEX));
    path.push(PathCommand::LineTo(Point::new(100.9, VERTEX.y - 75.0)));

    let mut list = DisplayList::new(PAGE);
    list.push(Command::Stroke {
        path: Arc::new(path),
        transform: Transform::IDENTITY,
        stroke: Stroke {
            width: WIDTH,
            cap: LineCap::Round,
            join: LineJoin::Miter,
            miter_limit: limit,
            ..Stroke::default()
        },
        paint: Paint::Solid(Color {
            r: 0.0,
            g: 0.0,
            b: 0.0,
            a: alpha,
        }),
        clip: None,
        mask: None,
        blend: BlendMode::Normal,
    });
    list
}

/// §8.4.3.5's own arithmetic for the scene above: the angle, the ratio, and the tip's reach.
fn closed_form() -> (f64, f64, f64) {
    let phi = (0.9_f64 / 75.0).atan();
    let ratio = 1.0 / (phi / 2.0).sin();
    (phi, ratio, f64::from(WIDTH) / 2.0 * ratio)
}

/// The ink, in device pixels, above the page-space height `y`.
///
/// Ink rather than coverage: the paint is black at the scene's own alpha over white, so one fully
/// painted pixel is `alpha` and the sum is comparable with an area times that alpha.
fn ink_above(raster: &Raster, y: f32, scale: f32) -> f64 {
    let rows = ((PAGE.height - y) * scale) as u32;
    let mut total = 0.0;
    for row in 0..rows.min(raster.height) {
        let start = (row * raster.width * 4) as usize;
        let end = start + (raster.width * 4) as usize;
        for pixel in raster.data[start..end].chunks_exact(4) {
            total += 1.0 - f64::from(pixel[0]) / 255.0;
        }
    }
    total / f64::from(scale * scale)
}

/// The highest page-space height carrying any ink at all.
fn reaches(raster: &Raster, scale: f32) -> f64 {
    for row in 0..raster.height {
        let start = (row * raster.width * 4) as usize;
        let end = start + (raster.width * 4) as usize;
        if raster.data[start..end].chunks_exact(4).any(|p| p[0] < 255) {
            return f64::from(PAGE.height) - f64::from(row) / f64::from(scale);
        }
    }
    0.0
}

/// How far a backend's ink may sit from the wedge's own area, as a fraction of it.
///
/// **Measured rather than chosen**, and the measurement is the surprise: over the whole wedge the
/// three backends land within **0.11%** of the closed form's 4166.97 device pixels at both scales
/// and both alphas, the worst single figure being quorra's −0.11% at scale 2.5. An eight-bit
/// raster's rounding cancels along a spike a thousand rows tall, which is what makes this the
/// number to assert rather than the per-row coverage. 2% is eighteen times the worst residual and
/// still fails by the whole quantity on the bevel this file was written for, which puts **nothing**
/// above the vertex.
const TOLERANCE: f64 = 0.02;

/// How short of the clause's tip a backend's last visible ink may fall, in user units.
///
/// The spike ends in a point, so the question is where its coverage drops under one level of 255:
/// at `1/255` of a pixel wide the remaining length is `1 / (2 × 255 × tan(φ/2))` = 0.33 units. The
/// two device backends stop 0.4 and 1.0 short of the tip and the processor 10.4, which is
/// `tiny-skia` quantising a nearly vertical sliver's run to quarter pixels rather than anything
/// this construction states. 20 units is slack of a factor of two on the worst of the three,
/// against a bevel that falls 833 short.
const TIP_SHORTFALL: f64 = 20.0;

/// One backend: its name and a closure that draws a scene with it.
///
/// A closure rather than a `dyn Rasterizer`, because the three rasterisers have three error types
/// and what this file wants from each is one raster or a loud failure.
type Backend = (
    &'static str,
    Box<dyn FnMut(&DisplayList, TargetSpec) -> Raster>,
);

/// The three backends, each built once per scene so that a device is not held across the run.
fn backends() -> Vec<Backend> {
    let mut cpu = render_cpu::CpuRasterizer::new();
    let mut quorra = QuorraRasterizer::new_headless().expect("a quorra device");
    let mut gpu = render_gpu::GpuRasterizer::new_headless().expect("a vello device");
    vec![
        (
            "processor",
            Box::new(move |list, target| cpu.rasterize(list, target).expect("a cpu raster")),
        ),
        (
            "quorra",
            Box::new(move |list, target| quorra.rasterize(list, target).expect("a quorra raster")),
        ),
        (
            "vello",
            Box::new(move |list, target| gpu.rasterize(list, target).expect("a vello raster")),
        ),
    ]
}

/// The mitre a `333 M` admits is drawn to the length its own angle implies, on every backend.
///
/// At two scales, one of them fractional so that the join and the tip land between device pixels:
/// a construction that snapped either would pass at scale 1 and fail here.
#[test]
fn every_backend_draws_the_mitre_the_limit_admits() {
    let (phi, ratio, reach) = closed_form();
    println!(
        "phi {:.6}° ratio {ratio:.4} tip {reach:.3} above the join",
        phi.to_degrees()
    );
    // The wedge is a triangle of half-angle φ/2 and height `reach`, so its area is
    // `reach² tan(φ/2)`; both segments descend from the vertex, so nothing else is up there.
    let area = reach * reach * (phi / 2.0).tan();

    for scale in [1.0_f32, 2.5] {
        for alpha in [1.0_f32, 0.6] {
            let list = witness(LIMIT, alpha);
            let target = TargetSpec::for_page(&list, scale, 1 << 30).expect("a valid target");
            for (name, draw) in &mut backends() {
                let raster = draw(&list, target);
                let ink = ink_above(&raster, VERTEX.y, scale);
                let expected = area * f64::from(alpha);
                let reached = reaches(&raster, scale) - f64::from(VERTEX.y);
                println!(
                    "  scale {scale} alpha {alpha} {name:>9}: ink {ink:9.2} of {expected:9.2} \
                     ({:+.2}%), reaches {reached:8.2} of {reach:.2}",
                    (ink / expected - 1.0) * 100.0
                );
                assert!(
                    (ink / expected - 1.0).abs() < TOLERANCE,
                    "{name} at scale {scale} alpha {alpha}: {ink:.2} of the wedge's own \
                     {expected:.2} device pixels"
                );
                assert!(
                    reached > reach - TIP_SHORTFALL,
                    "{name} at scale {scale} alpha {alpha}: the mitre reaches {reached:.2} where \
                     §8.4.3.5 puts its tip at {reach:.2}"
                );
            }
        }
    }
}

/// A ratio over the file's own limit is a bevel, which puts nothing above the join.
///
/// The discrimination in the other direction, and the reason this is not "draw a spike whenever a
/// join is sharp": with the same geometry and a limit of 100 the clause converts the join, and a
/// backend that drew the mitre anyway would be as wrong as one that never drew it.
#[test]
fn every_backend_bevels_a_ratio_over_the_limit() {
    let (_, ratio, reach) = closed_form();
    let list = witness(100.0, 1.0);
    assert!(
        ratio > 100.0,
        "the scene's own ratio has to exceed the limit"
    );
    let target = TargetSpec::for_page(&list, 1.0, 1 << 30).expect("a valid target");
    for (name, draw) in &mut backends() {
        let raster = draw(&list, target);
        // The bevel's own outer corner sits a hundredth of a unit above the vertex, so the honest
        // bound is a fraction of a pixel rather than nothing at all.
        let ink = ink_above(&raster, VERTEX.y + 1.0, 1.0);
        println!("  {name:>9}: {ink:.4} device pixels above the join, tip would be at {reach:.1}");
        assert!(
            ink < 1.0,
            "{name} draws {ink:.4} device pixels above a join §8.4.3.5 converts to a bevel"
        );
    }
}

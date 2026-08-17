//! What each backend draws for a mitre at each ratio ISO 32000-2 §8.4.3.5 admits.
//!
//! §8.4.3.5 bounds the ratio of mitre length to line width by the file's own `M` and states the
//! ratio as `1 / sin(φ/2)` for the angle φ between the segments — so the tip of the mitre sits
//! `(w/2) / sin(φ/2)` from the vertex, the mitre length itself spanning the whole join. This walks
//! one join from 45° down to 0.2° at a fixed limit and prints, for each of the three backends, how
//! much ink lands in the topmost tenth of the spike and how far up the page any ink reaches, beside
//! the arithmetic.
//!
//! It is the instrument `doc/todo/11` §6 was written without, and it found the boundary the fix is
//! conditioned on: the processor's `tiny-skia` stroker draws the mitre at a ratio of 90.23 and
//! **nothing at all** at 95.50, because `dot_to_angle_type` calls a join whose normals' dot product
//! is within `1/4096` of −1 `Nearly180` and bevels it before the limit is read. The two
//! graphics-device backends have no such cutoff. Run it when
//! `render-quorra/tests/mitre_limit.rs` fails, and read three things:
//!
//! - **the tip column** against the arithmetic: a bevel puts the highest ink at the vertex itself,
//!   which is the defect this ladder was built to see;
//! - **the ink column** against `L² tan(φ/2)`, the area of a wedge measured `L` from its own tip:
//!   2% to 3% under it is the eight-bit raster, since the last third of a unit of the spike is
//!   thinner than one level of 255;
//! - **the last two rungs**, where the ratio passes the stated limit and every backend must draw a
//!   bevel — the clause's own conversion, and the discrimination that stops "draw a spike whenever
//!   a join is sharp" from passing.
//!
//! One rung reads zero on all three for a reason that is this ladder's rather than a backend's: at
//! 0.5° the clause puts the tip 1345.92 above a join on a page 1200 tall, so the band measured lies
//! off the raster. The tip column says so — it reads the page's own height instead of the join's.
//!
//! ```sh
//! cargo run --release -p render-quorra --example mitre_ladder
//! ```

#![expect(
    clippy::print_stdout,
    clippy::arithmetic_side_effects,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::expect_used,
    reason = "a measurement example: its output is the point, and a backend that cannot draw one \
              stroked path onto a 400 x 1200 page should stop it loudly"
)]

use std::sync::Arc;

use pdf_render::{
    BlendMode, Color, Command, DisplayList, LineCap, LineJoin, Paint, Path, PathCommand, Point,
    Raster, Rasterizer, Size, Stroke, TargetSpec, Transform,
};

/// The page every rung is drawn on.
const PAGE: Size = Size {
    width: 400.0,
    height: 1200.0,
};

/// Where the join sits, in page space.
const VERTEX: Point = Point { x: 200.0, y: 200.0 };

/// The line width, which is `LargeMitreLimit.pdf`'s own.
const WIDTH: f32 = 10.0;

/// The limit every rung states, which is the corpus witness's own `333 M`.
const LIMIT: f32 = 333.0;

/// How far back along each segment the path extends from the vertex.
const ARMS: f64 = 150.0;

/// One join of interior angle `phi_deg`, opening upward: a segment arrives up the y axis and one
/// leaves back down, turned by that angle, so the mitre points at the top of the page.
fn rung(phi_deg: f64) -> DisplayList {
    let phi = phi_deg.to_radians();
    let (sin, cos) = phi.sin_cos();
    let mut path = Path::new();
    path.push(PathCommand::MoveTo(Point::new(
        VERTEX.x,
        VERTEX.y - ARMS as f32,
    )));
    path.push(PathCommand::LineTo(VERTEX));
    path.push(PathCommand::LineTo(Point::new(
        VERTEX.x + (ARMS * sin) as f32,
        VERTEX.y - (ARMS * cos) as f32,
    )));

    let mut list = DisplayList::new(PAGE);
    list.push(Command::Stroke {
        path: Arc::new(path),
        transform: Transform::IDENTITY,
        stroke: Stroke {
            width: WIDTH,
            cap: LineCap::Butt,
            join: LineJoin::Miter,
            miter_limit: LIMIT,
            ..Stroke::default()
        },
        paint: Paint::Solid(Color::BLACK),
        clip: None,
        mask: None,
        blend: BlendMode::Normal,
    });
    list
}

/// Ink in device pixels above the page-space height `y`.
fn ink_above(raster: &Raster, y: f32) -> f64 {
    let rows = ((PAGE.height - y) as u32).min(raster.height);
    let mut total = 0.0;
    for row in 0..rows {
        let start = (row * raster.width * 4) as usize;
        let end = start + (raster.width * 4) as usize;
        for pixel in raster.data[start..end].chunks_exact(4) {
            total += 1.0 - f64::from(pixel[0]) / 255.0;
        }
    }
    total
}

/// The page-space height of the highest row carrying any ink.
fn reaches(raster: &Raster) -> f64 {
    (0..raster.height)
        .find_map(|row| {
            let start = (row * raster.width * 4) as usize;
            let end = start + (raster.width * 4) as usize;
            raster.data[start..end]
                .chunks_exact(4)
                .any(|p| p[0] < 255)
                .then(|| f64::from(PAGE.height) - f64::from(row))
        })
        .unwrap_or(0.0)
}

fn main() {
    let mut cpu = render_cpu::CpuRasterizer::new();
    let mut quorra = render_quorra::QuorraRasterizer::new_headless().expect("a quorra device");
    let mut gpu = render_gpu::GpuRasterizer::new_headless().expect("a vello device");
    println!("quorra adapter: {}", quorra.adapter_description());
    println!("line width {WIDTH}, limit {LIMIT}, join at y {}", VERTEX.y);
    println!(
        "{:>8} {:>9} {:>9} {:>9} | {:>19} | {:>19} | {:>19}",
        "phi(°)",
        "ratio",
        "tip y",
        "band ink",
        "processor ink/tip",
        "vello ink/tip",
        "quorra ink/tip"
    );

    for phi_deg in [
        45.0_f64, 20.0, 11.0, 5.0, 2.0, 1.5, 1.3, 1.27, 1.2, 1.0, 0.687_516, 0.5, 0.343_75, 0.2,
    ] {
        let phi = phi_deg.to_radians();
        let ratio = 1.0 / (phi / 2.0).sin();
        let reach = f64::from(WIDTH) / 2.0 * ratio;
        let tip = f64::from(VERTEX.y) + reach;
        // The topmost tenth of the spike, whose area is `L² tan(φ/2)` for a wedge measured `L`
        // from its tip — the clause's arithmetic with nothing of the stroke's body in it.
        let band = reach / 10.0;
        let line = (tip - band) as f32;
        let expected = band * band * (phi / 2.0).tan();

        let list = rung(phi_deg);
        let target = TargetSpec::for_page(&list, 1.0, 1 << 30).expect("a valid target");
        let each = [
            cpu.rasterize(&list, target).expect("a cpu raster"),
            gpu.rasterize(&list, target).expect("a vello raster"),
            quorra.rasterize(&list, target).expect("a quorra raster"),
        ];
        print!("{phi_deg:8.4} {ratio:9.3} {tip:9.2} {expected:9.3} |");
        for raster in &each {
            print!(" {:9.3}/{:8.1} |", ink_above(raster, line), reaches(raster));
        }
        println!();
    }
    println!(
        "a rung whose ratio exceeds {LIMIT} is a bevel by the clause: its tip column reads the \
         join's own height"
    );
}

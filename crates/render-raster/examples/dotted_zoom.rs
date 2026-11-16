//! What a **zoom step** costs on a page of §8.5.3.2's marks, with the scene kept and with it
//! rebuilt (ADR 2884).
//!
//! The page is a form's dotted leaders: rules a unit wide dashed `[0 3] 0 d` under round caps, so
//! every mark is a zero-length dash's cap, a disc a device pixel across or more at every scale of
//! the sequence — where §10.7.4 restates nothing and a kept scene is the build of each view. The
//! second arm is the same page with one more dot, a fiftieth of a unit across, which §10.7.4
//! restates as the device pixel its centre lies in: that answer names this grid's pixel, so the
//! scene reads the view and every step rebuilds it, which is what every scene holding a mark did
//! before ADR 2884. The one dot costs nothing measurable beside the leaders, so the two arms differ
//! by whether the scene is kept and by nothing else.
//!
//! ```sh
//! cargo run --release -p render-raster --example dotted_zoom [rules] [rounds]
//! ```
//!
//! Each line is one step of the sequence, the minimum over the rounds of the frame's total and its
//! scene phase, and whether the step kept the scene.

#[path = "../../../tools/conformance/test-support/gpu_lock.rs"]
mod gpu_lock;
#[path = "../../../tools/conformance/test-support/gpu_lock_hook.rs"]
mod gpu_lock_hook;

use std::sync::Arc;
use std::time::Duration;

use pdf_render::{
    BlendMode, Color, Command, DisplayList, LineCap, LineJoin, Paint, Path, PathCommand, Point,
    Size, Stroke, TargetSpec, Transform,
};
use render_raster::{PresentFrame, QuorraRasterizer};

/// The window every frame is drawn into.
const WINDOW: (u32, u32) = (1200, 1000);

/// A US Letter page, in user units.
const PAGE: Size = Size {
    width: 612.0,
    height: 792.0,
};

/// The scales of the sequence: the first frame, then each step.
const SCALES: [f32; 6] = [1.0, 1.25, 1.5, 2.0, 1.25, 1.0];

/// One solid round-capped stroke of the segment `from`–`to` under `dash`.
fn rule(list: &mut DisplayList, (from, to): ((f32, f32), (f32, f32)), width: f32, dash: &[f32]) {
    let mut path = Path::new();
    path.push(PathCommand::MoveTo(Point::new(from.0, from.1)));
    path.push(PathCommand::LineTo(Point::new(to.0, to.1)));
    list.push(Command::Stroke {
        path: Arc::new(path),
        transform: Transform::IDENTITY,
        stroke: Stroke {
            width,
            adjust: false,
            cap: LineCap::Round,
            join: LineJoin::Miter,
            miter_limit: 10.0,
            dash_array: dash.to_vec(),
            dash_phase: 0.0,
        },
        paint: Paint::Solid(Color::rgb(0.0, 0.0, 0.0)),
        clip: None,
        mask: None,
        blend: BlendMode::Normal,
    });
}

/// The page of `rules` dotted leaders, and with `pixel_dot` the one dot §10.7.4 restates as its
/// pixel.
fn page(rules: u16, pixel_dot: bool) -> Arc<DisplayList> {
    let mut list = DisplayList::new(PAGE);
    let pitch = 740.0 / f32::from(rules);
    for index in 0..rules {
        let y = f32::from(index).mul_add(pitch, 30.0);
        rule(&mut list, ((30.0, y), (582.0, y)), 1.0, &[0.0, 3.0]);
    }
    if pixel_dot {
        rule(&mut list, ((300.5, 10.5), (300.5, 10.5)), 0.02, &[]);
    }
    Arc::new(list)
}

/// The page at `scale`, its top-left corner at the window's.
fn placed(scale: f32) -> TargetSpec {
    TargetSpec {
        width: WINDOW.0,
        height: WINDOW.1,
        transform: Transform::scale(scale, -scale)
            .then(Transform::translate(0.0, scale * PAGE.height)),
    }
}

/// Each step's least total and scene over `rounds` runs of the sequence, and whether it kept.
fn arm(list: &Arc<DisplayList>, rounds: u32) -> Vec<(Duration, Duration, bool)> {
    let mut best = vec![(Duration::MAX, Duration::MAX, false); SCALES.len()];
    for _ in 0..rounds {
        gpu_lock_hook::install();
        let Ok(mut gpu) = QuorraRasterizer::new_headless() else {
            eprintln!("no adapter available for raster");
            std::process::exit(1);
        };
        for (step, scale) in SCALES.into_iter().enumerate() {
            let frame = PresentFrame {
                width: WINDOW.0,
                height: WINDOW.1,
                pages: &[(list, placed(scale))],
                raster: None,
                overlays: &[],
            };
            if let Err(error) = gpu.rasterize_frame(&frame) {
                eprintln!("the device refused the page: {error}");
                std::process::exit(1);
            }
            let cost = gpu.last_frame();
            let slot = &mut best[step];
            *slot = (
                slot.0.min(cost.total),
                slot.1.min(cost.scene),
                cost.scene_kept,
            );
        }
    }
    best
}

fn main() {
    let mut args = std::env::args().skip(1);
    let rules: u16 = args.next().and_then(|a| a.parse().ok()).unwrap_or(150);
    let rounds: u32 = args.next().and_then(|a| a.parse().ok()).unwrap_or(5);
    println!(
        "{rules} dotted leaders, {} marks; {rounds} rounds",
        u32::from(rules) * 185
    );
    for (name, pixel_dot) in [("kept", false), ("rebuilt", true)] {
        let list = page(rules, pixel_dot);
        for (step, (total, scene, kept)) in arm(&list, rounds).into_iter().enumerate() {
            println!(
                "{name:8} step {step} scale {:4} total {:8.2} ms scene {:8.2} ms kept {kept}",
                SCALES[step],
                total.as_secs_f64() * 1e3,
                scene.as_secs_f64() * 1e3,
            );
        }
    }
}

//! What the *first* frame costs that the tenth does not.
//!
//! `CLAUDE.md` puts page one on the graphics device and forbids waiting for warmth: "no
//! `wait_until_warm` before the first present, no probe frame, no pipeline pre-compilation 'to be
//! safe'". That rule is only safe if a cold device draws a page in a time a person accepts, and
//! ADR 0179's launch timeline cannot say — its 54 to 68 ms of first present is `lavapipe` under
//! `Xvfb`, which is a CPU rasteriser wearing a Vulkan hat.
//!
//! This measures it on whatever adapter the machine really has, headless: a device brought up,
//! then ten renders of the same page and the same target, each timed, with nothing waited for.
//!
//! ```sh
//! cargo run --release -p render-raster --example first_frame -- [page] [scale] [settle-ms]
//! FIRST_FRAME_COVERAGE=gpu cargo run --release -p render-raster --example first_frame
//! ```
//!
//! Readback is included and is the same on every frame, so it cancels out of the *difference*,
//! which is what this example is about. `frame_race.rs` is the one that quotes absolute numbers.
//!
//! Each frame is printed stage by stage as `FrameCost` reports it, because a first frame's excess
//! is only actionable once it is known which stage holds it: the scene's walk and its resource
//! hand-over are the page's own work, which no warm-up thread can do before the page exists, and
//! the device's stages are what a warm-up could reach.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, missing_docs)]

use std::time::{Duration, Instant};

use pdf_render::{Rasterizer as _, TargetSpec};
use pdf_syntax::Document;

fn main() {
    let mut args = std::env::args().skip(1);
    let index: usize = args.next().map_or(6, |n| n.parse().expect("a page number"));
    let scale: f32 = args.next().map_or(1.0, |s| s.parse().expect("a scale"));

    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../doc/ISO_32000-2_sponsored_EC3.pdf");
    let bytes = std::fs::read(&path).expect("the specification is in doc/");
    let document = Document::open(bytes).expect("opens");
    let pages = pdf_model::Pages::new(&document);
    let page = pages.get(index).expect("that page exists");
    let list = pdf_model::content::interpret(&document, &page).display_list;
    let target = TargetSpec::for_page(&list, scale, 1 << 30).expect("a target");

    // The device is created *after* the page is interpreted, so that nothing it does on a
    // background thread has had the interpretation to hide behind — the launch path's order.
    // **Which coverage lane**, because the two are opposite curves and the first frame is where
    // they differ most: the CPU lane's atlas pays for a tile once per *page* and the GPU lane
    // has no atlas at all, so a measurement of "the first frame" taken on the default lane says
    // nothing about the one `quorra.rs` switches to past `GPU_COVERAGE_MAGNIFICATION`. A
    // value that is neither is a panic rather than a fallback, for `tests/corpus.rs`'s reason.
    let coverage = match std::env::var("FIRST_FRAME_COVERAGE")
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase()
        .as_str()
    {
        "" | "cpu" => raster_gpu::Coverage::Cpu,
        "gpu" => raster_gpu::Coverage::Gpu,
        other => panic!("FIRST_FRAME_COVERAGE={other}: expected `cpu` or `gpu`"),
    };

    let brought_up = Instant::now();
    let mut backend = render_raster::QuorraRasterizer::new_headless().expect("an adapter");
    backend.set_coverage(coverage);
    let bring_up = brought_up.elapsed().as_secs_f64() * 1e3;

    // A third argument in milliseconds waits before the first frame, which is the experiment
    // that says *what* the first frame is paying for: pipelines compile on a background thread,
    // so a wait long enough for them to finish separates "the first frame waited for a shader"
    // from "the first frame allocated something every frame after it reuses".
    let settle: u64 = args
        .next()
        .map_or(0, |ms| ms.parse().expect("milliseconds"));
    if settle > 0 {
        std::thread::sleep(Duration::from_millis(settle));
    }

    let mut frames = Vec::new();
    for nth in 1..=10_u32 {
        mark(nth);
        let faulted = minor_faults();
        let started = Instant::now();
        backend
            .rasterize(&list, target)
            .unwrap_or_else(|e| panic!("refused: {e}"));
        let wall = started.elapsed().as_secs_f64() * 1e3;
        frames.push(Measured {
            wall,
            cost: backend.last_frame(),
            faults: minor_faults()
                .zip(faulted)
                .map(|(after, before)| after.saturating_sub(before)),
            phases: backend.last_phases().to_vec(),
        });
    }
    mark(0);

    println!(
        "page {} at {scale} ({}x{}) on {}, {} coverage lane",
        index.saturating_add(1),
        target.width,
        target.height,
        backend.adapter_description(),
        match coverage {
            raster_gpu::Coverage::Cpu => "cpu",
            raster_gpu::Coverage::Gpu => "gpu",
            raster_gpu::Coverage::Compute => "compute",
        }
    );
    println!("  bring-up          {bring_up:8.2} ms");
    print_stages(&frames);
    print_phases(&frames);
}

/// One frame as the loop saw it: its wall clock in milliseconds, the stages `FrameCost` reports,
/// the minor page faults the process took during it, and raster's named spans.
struct Measured {
    wall: f64,
    cost: render_raster::FrameCost,
    faults: Option<u64>,
    phases: Vec<(&'static str, Duration)>,
}

fn ms(d: Duration) -> f64 {
    d.as_secs_f64() * 1e3
}

/// One line a frame, stage by stage: an excess is only actionable once it is known which stage
/// holds it.
fn print_stages(frames: &[Measured]) {
    println!(
        "  {:<10} {:>8} | {:>7} {:>7} {:>7} {:>7} {:>7} {:>7} {:>7} {:>7}  (ms) {:>7} {:>7}",
        "",
        "wall",
        "scene",
        "handovr",
        "device",
        "encode",
        "upload",
        "execute",
        "readbk",
        "settle",
        "uploads",
        "faults"
    );
    for (nth, frame) in frames.iter().enumerate() {
        let cost = &frame.cost;
        println!(
            "  frame {:<4} {:8.2} | {:7.2} {:7.2} {:7.2} {:7.2} {:7.2} {:7.2} {:7.2} {:7.2}       {:>7} {:>7}",
            nth.saturating_add(1),
            frame.wall,
            ms(cost.scene),
            ms(cost.handover),
            ms(cost.device),
            ms(cost.encode),
            ms(cost.upload),
            ms(cost.execute),
            ms(cost.readback),
            ms(cost.settle),
            cost.uploads,
            frame
                .faults
                .map_or_else(|| "-".to_owned(), |n| n.to_string()),
        );
    }
}

/// raster's own named spans for the two frames that differ and one that does not: the stage
/// columns say *which* stage holds an excess, and these say which step inside it.
fn print_phases(frames: &[Measured]) {
    println!("  phases (ms)                     frame 1  frame 2  frame 10");
    let mut names: Vec<&str> = Vec::new();
    for frame in frames {
        for (name, _) in &frame.phases {
            if !names.contains(name) {
                names.push(name);
            }
        }
    }
    let of = |nth: usize, name: &str| -> f64 {
        frames.get(nth).map_or(0.0, |frame| {
            frame
                .phases
                .iter()
                .filter(|(each, _)| *each == name)
                .map(|(_, d)| ms(*d))
                .sum()
        })
    };
    for name in names {
        println!(
            "    {name:<28} {:8.2} {:8.2} {:8.2}",
            of(0, name),
            of(1, name),
            of(9, name)
        );
    }
}

/// The process's minor page faults so far — field 10 of `/proc/self/stat`, read after the
/// command's closing parenthesis because the command itself may hold spaces. A fault is the
/// kernel giving a page of memory its first touch, which is what a fresh mapping costs when it
/// is first written; `None` where the file is not Linux's.
fn minor_faults() -> Option<u64> {
    let stat = std::fs::read_to_string("/proc/self/stat").ok()?;
    let (_, fields) = stat.rsplit_once(')')?;
    fields.split_whitespace().nth(7)?.parse().ok()
}

/// A boundary between frames that an instrument outside the process can see: `strace` reads it
/// as a `statx` of a path that does not exist, and `callgrind --dump-before=*first_frame*mark*`
/// splits its profile there, so the creations a frame makes are counted frame by frame rather
/// than inferred from a difference of totals. Outside every timed span; `0` is the end of the
/// last frame.
#[inline(never)]
fn mark(frame: u32) {
    drop(std::fs::metadata(format!("/first-frame-mark/{frame}")));
}

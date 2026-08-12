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
//! cargo run --release -p render-quorra --example first_frame -- [page] [scale] [settle-ms]
//! FIRST_FRAME_COVERAGE=gpu cargo run --release -p render-quorra --example first_frame
//! ```
//!
//! Readback is included and is the same on every frame, so it cancels out of the *difference*,
//! which is what this example is about. `frame_race.rs` is the one that quotes absolute numbers.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, missing_docs)]

use std::time::Instant;

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
    // nothing about the one `pdf-viewer.rs` switches to past `GPU_COVERAGE_MAGNIFICATION`. A
    // value that is neither is a panic rather than a fallback, for `tests/corpus.rs`'s reason.
    let coverage = match std::env::var("FIRST_FRAME_COVERAGE")
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase()
        .as_str()
    {
        "" | "cpu" => quorra_gpu::Coverage::Cpu,
        "gpu" => quorra_gpu::Coverage::Gpu,
        other => panic!("FIRST_FRAME_COVERAGE={other}: expected `cpu` or `gpu`"),
    };

    let brought_up = Instant::now();
    let mut backend = render_quorra::QuorraRasterizer::new_headless().expect("an adapter");
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
        std::thread::sleep(std::time::Duration::from_millis(settle));
    }

    let mut times = Vec::new();
    for _ in 0..10 {
        let started = Instant::now();
        backend
            .rasterize(&list, target)
            .unwrap_or_else(|e| panic!("refused: {e}"));
        times.push(started.elapsed().as_secs_f64() * 1e3);
    }

    println!(
        "page {} at {scale} ({}x{}) on {}, {} coverage lane",
        index.saturating_add(1),
        target.width,
        target.height,
        backend.adapter_description(),
        match coverage {
            quorra_gpu::Coverage::Cpu => "cpu",
            quorra_gpu::Coverage::Gpu => "gpu",
        }
    );
    println!("  bring-up          {bring_up:8.2} ms");
    for (nth, ms) in times.iter().enumerate() {
        println!("  frame {:<2}          {ms:8.2} ms", nth.saturating_add(1));
    }
}

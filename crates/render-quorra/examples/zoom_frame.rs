//! What a **zoom step** costs, phase by phase, on a device that has already drawn the page.
//!
//! The frame a person waits for is not the first one. The first uploads every outline the page
//! states and fills every cache; what a magnification costs is the *second* frame, drawn at a new
//! transform against caches that are already warm — which is the frame ADR 0368 attributed and
//! the one the project owner's own traces are full of. Nothing in this tree drew that frame in
//! isolation before: `examples/encode_threads.rs` brings a cold device up per sample deliberately,
//! and a cold device measures the population this one exists to exclude.
//!
//! ```sh
//! cargo run --release -p render-quorra --example zoom_frame -- <file.pdf> [page] [scale] [factor]
//! ZOOM_FRAME_ENCODE_PHASES=1 cargo run --release -p render-quorra --example zoom_frame -- …
//! ZOOM_FRAME_ROUNDS=5        cargo run --release -p render-quorra --example zoom_frame -- …
//! ```
//!
//! **Two frames, one device, one page.** Frame 1 places the page at `scale`; frame 2 places the
//! same display list — the same `Arc`, so every outline is a cache hit — at `scale × factor`.
//! Both are reported, because the pair is the measurement: the difference between them is what a
//! warm cache is worth, and frame 2 alone is what a zoom step costs.
//!
//! **`bytes_uploaded` is printed because nothing else in this tree prints it.** quorra counts the
//! bytes it schedules for transfer and `FrameCost` has carried the number across the boundary
//! since ADR 0227; no host had ever read it, so `transfer` was a duration with no denominator and
//! the frame line's `up` count — this crate's *resource* uploads, a different quantity entirely —
//! stood in for one. ADR 0387.
//!
//! **`ZOOM_FRAME_ENCODE_PHASES` costs what quorra says it costs.** `Options::instrument_encode`
//! inflates `encode` by a few per cent (ADR 0368 measured 512.8 ms against 475.9 on this page), so
//! the subdivision it prints is shares rather than absolutes, and it is off by default for that
//! reason.
//!
//! **The statistic is the minimum of `ZOOM_FRAME_ROUNDS` round-robin rounds**, for the reason
//! `examples/encode_threads.rs` states: this machine is shared, and a phase measured under load is
//! a measurement of the load. Each round builds its own device, because a device that has drawn
//! this pair already answers frame 1 from the atlas.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::cast_precision_loss,
    clippy::too_many_lines,
    missing_docs
)]

use std::sync::Arc;

use pdf_render::TargetSpec;
use pdf_syntax::Document;
use render_quorra::{FrameCost, PresentFrame};

/// One frame's numbers, in the units this prints them in.
struct Sample {
    total: f64,
    scene: f64,
    device: f64,
    encode: f64,
    upload: f64,
    execute: f64,
    readback: f64,
    elsewhere: f64,
    uploads: u32,
    bytes: u64,
    commands: u32,
    culled: u32,
    replayed: bool,
    phases: Vec<(String, f64)>,
}

/// Milliseconds, which is the unit every frame line in this project is in.
fn ms(span: std::time::Duration) -> f64 {
    span.as_secs_f64() * 1e3
}

impl Sample {
    fn of(cost: FrameCost, phases: &[(&'static str, std::time::Duration)]) -> Self {
        // `device` minus the phases quorra names — the same arithmetic the viewer's own summary
        // does, and carrying the same warning: `execute` is usually the adapter's clock and the
        // rest are the host's, so this is a bound rather than a duration. `readback` is in the
        // subtraction here and absent from the viewer's, which is not an inconsistency: this
        // example draws into a readback and a window never does.
        let elsewhere = cost
            .device
            .saturating_sub(cost.encode)
            .saturating_sub(cost.upload)
            .saturating_sub(cost.execute)
            .saturating_sub(cost.readback);
        Self {
            total: ms(cost.total),
            scene: ms(cost.scene),
            device: ms(cost.device),
            encode: ms(cost.encode),
            upload: ms(cost.upload),
            execute: ms(cost.execute),
            readback: ms(cost.readback),
            elsewhere: ms(elsewhere),
            uploads: cost.uploads,
            bytes: cost.bytes_uploaded,
            commands: cost.commands,
            culled: cost.commands_culled,
            replayed: matches!(cost.encode_source, Some(quorra_gpu::EncodeSource::Replayed)),
            phases: phases
                .iter()
                .map(|(name, spent)| ((*name).to_owned(), ms(*spent)))
                .collect(),
        }
    }
}

/// The one-minute load average, or `None` where this system does not publish one.
fn load_average() -> Option<f64> {
    let text = std::fs::read_to_string("/proc/loadavg").ok()?;
    text.split_whitespace().next()?.parse().ok()
}

fn variable(name: &str) -> Option<String> {
    std::env::var(name).ok()
}

fn main() {
    let mut arguments = std::env::args().skip(1);
    let path = arguments.next().expect("a document to draw");
    let index: usize = arguments
        .next()
        .map_or(1, |n| n.parse().expect("a page number"));
    let scale: f32 = arguments
        .next()
        .map_or(1.0, |s| s.parse().expect("a scale"));
    let factor: f32 = arguments
        .next()
        .map_or(1.25, |s| s.parse().expect("a zoom factor"));
    let rounds: usize = variable("ZOOM_FRAME_ROUNDS")
        .and_then(|n| n.parse().ok())
        .unwrap_or(3);
    let instrument = variable("ZOOM_FRAME_ENCODE_PHASES").is_some_and(|on| on != "0");
    // The one knob a *deterministic* run needs: callgrind counts instructions across every
    // thread quorra's geometry phase spawns, so an attribution of the walk this example is
    // about is read from a single-threaded arm. Named like the corpus gate's own two knobs
    // (ADR 0377) rather than given a flag, for the reason that ADR states.
    let threads: usize = variable("ZOOM_FRAME_ENCODE_THREADS")
        .and_then(|n| n.parse().ok())
        .unwrap_or_else(|| render_quorra::options().encode_threads);

    let document =
        Document::open(std::fs::read(&path).expect("the document is readable")).expect("it opens");
    let pages = pdf_model::Pages::new(&document);
    let page = pages
        .get(index.saturating_sub(1))
        .expect("that page exists");
    let list = Arc::new(pdf_model::content::interpret(&document, &page).display_list);
    let first = TargetSpec::for_page(&list, scale, 1 << 30).expect("a target");
    let second = TargetSpec::for_page(&list, scale * factor, 1 << 30).expect("a target");

    let before = load_average();
    let mut best: Vec<Option<Sample>> = vec![None, None];
    let mut adapter = String::new();
    for _ in 0..rounds {
        let mut backend = render_quorra::QuorraRasterizer::with_options(&quorra_gpu::Options {
            instrument_encode: instrument,
            encode_threads: threads,
            ..render_quorra::options()
        })
        .expect("an adapter");
        backend.adapter_description().clone_into(&mut adapter);
        for (slot, target) in best.iter_mut().zip([first, second]) {
            let frame = PresentFrame {
                width: target.width,
                height: target.height,
                page: Some((&list, target)),
                raster: None,
                overlays: &[],
            };
            backend
                .rasterize_frame(&frame)
                .unwrap_or_else(|error| panic!("refused: {error}"));
            let sample = Sample::of(backend.last_frame(), backend.last_phases());
            if slot.as_ref().is_none_or(|kept| sample.total < kept.total) {
                *slot = Some(sample);
            }
        }
    }

    println!("{path} page {index}, {adapter}");
    println!(
        "{threads} encode thread(s), load average {} → {}, minima of {rounds} round(s){}",
        before.map_or_else(|| "?".to_owned(), |load| format!("{load:.2}")),
        load_average().map_or_else(|| "?".to_owned(), |load| format!("{load:.2}")),
        if instrument {
            ", encode instrumented — read its shares, not its absolutes"
        } else {
            ""
        }
    );
    println!(
        "{:<8}{:>10}{:>9}{:>8}{:>8}{:>8}{:>9}{:>10}{:>8}{:>9}{:>11}{:>7}{:>8}",
        "frame",
        "px",
        "total",
        "scene",
        "device",
        "encode",
        "transfer",
        "elsewhere",
        "execute",
        "readback",
        "bytes",
        "up",
        "culled"
    );
    for (name, target, sample) in [
        ("before", first, best[0].as_ref().expect("a first frame")),
        ("zoom", second, best[1].as_ref().expect("a second frame")),
    ] {
        println!(
            "{name:<8}{:>10}{:>9.1}{:>8.1}{:>8.1}{:>8.1}{:>9.1}{:>10.1}{:>8.1}{:>9.1}{:>11}{:>7}{:>8}{}",
            format!("{}x{}", target.width, target.height),
            sample.total,
            sample.scene,
            sample.device,
            sample.encode,
            sample.upload,
            sample.elsewhere,
            sample.execute,
            sample.readback,
            sample.bytes,
            sample.uploads,
            sample.culled,
            if sample.replayed { " replayed" } else { "" }
        );
        println!("        {} command(s) encoded", sample.commands);
        for (phase, spent) in &sample.phases {
            println!("        {phase:<20}{spent:>9.3}");
        }
    }
}

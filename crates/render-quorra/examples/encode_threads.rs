//! How many threads this host should let quorra's geometry phase use, on *this* machine.
//!
//! quorra divides the coverage rasterisation of one frame across
//! `Options::encode_threads` (their ADR 0054, `doc/QUORRA_ENCODE_THREADS_ANSWER.md`), whose
//! default is 1 — a permission rather than a preference. What number a host should ask for
//! is not something upstream would publish and not something this tree may assume: their
//! own round read 24 threads as *worse* than 8 on a machine under load, and this project's
//! ADR 0260 found the same shape for `rayon` over pages. So it is measured here, by the
//! host, on the pages the host actually draws.
//!
//! ```sh
//! cargo run --release -p render-quorra --example encode_threads -- <file.pdf> [page] [scale] [1,2,4,…]
//! ENCODE_THREADS_ROUNDS=7 cargo run --release -p render-quorra --example encode_threads -- …
//! ```
//!
//! **A cold device per sample**, which is the one thing that makes this measurement honest:
//! quorra's tile cache answers a second frame at the same transform from the atlas, so a
//! ladder run over one device measures the cache on every rung after the first (ADR 0368
//! saw 640 ms become 140 for exactly that reason). Each sample therefore brings a device
//! up, draws once, and drops it.
//!
//! **The rounds are round-robin and the statistic is the minimum**, because the machine is
//! shared: a thread count measured under load is a measurement of the load, and a minimum
//! over interleaved rounds is the least contaminated number a shared machine yields. The
//! load average is printed at both ends so a reader can see what the run was competing with.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::cast_precision_loss,
    missing_docs
)]

use std::sync::Arc;

use pdf_render::TargetSpec;
use pdf_syntax::Document;
use render_quorra::PresentFrame;

/// What one sample cost, in the three parts `FrameCost` separates.
struct Sample {
    encode: f64,
    device: f64,
    total: f64,
}

/// The one-minute load average, or `None` where this system does not publish one.
fn load_average() -> Option<f64> {
    let text = std::fs::read_to_string("/proc/loadavg").ok()?;
    text.split_whitespace().next()?.parse().ok()
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
    let ladder: Vec<usize> = arguments.next().map_or_else(
        || vec![1, 2, 4, 8, 12, 16, 24],
        |list| {
            list.split(',')
                .map(|n| n.trim().parse().expect("a thread count"))
                .collect()
        },
    );
    let rounds: usize = std::env::var("ENCODE_THREADS_ROUNDS")
        .ok()
        .and_then(|n| n.parse().ok())
        .unwrap_or(5);

    let document =
        Document::open(std::fs::read(&path).expect("the document is readable")).expect("it opens");
    let pages = pdf_model::Pages::new(&document);
    let page = pages
        .get(index.saturating_sub(1))
        .expect("that page exists");
    let list = Arc::new(pdf_model::content::interpret(&document, &page).display_list);
    let target = TargetSpec::for_page(&list, scale, 1 << 30).expect("a target");

    let before = load_average();
    let mut samples: Vec<Vec<Sample>> = ladder.iter().map(|_| Vec::new()).collect();
    let mut commands = 0;
    let mut adapter = String::new();
    for _ in 0..rounds {
        for (slot, threads) in samples.iter_mut().zip(&ladder) {
            let mut backend = render_quorra::QuorraRasterizer::with_options(&quorra_gpu::Options {
                encode_threads: *threads,
                ..quorra_gpu::Options::default()
            })
            .expect("an adapter");
            backend.adapter_description().clone_into(&mut adapter);
            // The *window's* frame rather than `Rasterizer::rasterize`, because it is the path
            // the viewer takes and the only one that reports what each stage cost.
            let frame = PresentFrame {
                width: target.width,
                height: target.height,
                pages: &[(&list, target)],
                raster: None,
                overlays: &[],
            };
            backend
                .rasterize_frame(&frame)
                .unwrap_or_else(|error| panic!("refused at {threads} threads: {error}"));
            let cost = backend.last_frame();
            let total = cost.total.as_secs_f64() * 1e3;
            commands = cost.commands;
            slot.push(Sample {
                encode: cost.encode.as_secs_f64() * 1e3,
                device: cost.device.as_secs_f64() * 1e3,
                total,
            });
        }
    }
    let after = load_average();

    println!(
        "{path} page {index} at {scale}× ({} × {}) on {adapter}",
        target.width, target.height
    );
    println!(
        "{commands} commands encoded, {rounds} round-robin rounds, load {} → {}",
        before.map_or_else(|| "?".to_owned(), |l| format!("{l:.1}")),
        after.map_or_else(|| "?".to_owned(), |l| format!("{l:.1}")),
    );
    println!(
        "{:>8} {:>12} {:>12} {:>12} {:>9}",
        "threads", "encode min", "device min", "frame min", "speed-up"
    );
    let mut baseline = None;
    for (slot, threads) in samples.iter().zip(&ladder) {
        let least = |pick: fn(&Sample) -> f64| slot.iter().map(pick).fold(f64::INFINITY, f64::min);
        let encode = least(|s| s.encode);
        let base = *baseline.get_or_insert(encode);
        println!(
            "{threads:>8} {encode:>9.2} ms {:>9.2} ms {:>9.2} ms {:>8.2}×",
            least(|s| s.device),
            least(|s| s.total),
            base / encode,
        );
    }
}

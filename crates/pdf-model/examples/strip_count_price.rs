//! What a strip count costs a page on the pool it is drawn on, asked count by count.
//!
//! `render_cpu`'s planner asks for as many strips as it is told, or as the thread pool is wide
//! where it is told nothing (ADR 1859). Whether that is the right number is a clock's question:
//! a strip replays every command that reaches it (`pdf_render::replay_ratio`), so a strip more
//! than the pool can run at once costs its replay and buys only the balance work stealing gives
//! an uneven division. This draws one page at several stated counts on whatever pool
//! `RAYON_NUM_THREADS` makes, interleaved round by round so that a drift in the machine's clock
//! lands on every count alike, and prints for each count the strips the plan granted, the
//! quickest and slowest draw and the digest of the pixels.
//!
//! ```sh
//! RAYON_NUM_THREADS=4 cargo run --release -p pdf-model --example strip_count_price -- \
//!     <file.pdf> [page] [rounds] [counts, e.g. 4,8,16]
//! ```
//!
//! Five targets, each as a viewer asks for it: the page at 150 dpi (`quorra-transform render`'s
//! scale); a 1600 × 1000 window at its fit and at four times the fit, centred as `viewer_core`
//! centres a page larger than its window; and at four times the fit the whole page, which is what
//! a toolkit window is handed, beside the band of its rows that the window shows across the page's
//! whole width — the cheapest target a toolkit window could be handed without cutting columns
//! (ADR 1841 section 2 says why a window is not cut into columns).
#![expect(
    clippy::expect_used,
    clippy::print_stdout,
    clippy::cast_precision_loss,
    reason = "a measurement tool: it stops loudly on a missing input, its arithmetic is on pixel \
              extents far inside every type it uses, and printing is the whole point"
)]

use std::time::Instant;

use pdf_render::{DisplayList, Rasterizer as _, TargetSpec, Transform};
use render_cpu::CpuRasterizer;

/// The window the fit and the zoomed targets are measured against, in device pixels.
const WINDOW: (u32, u32) = (1600, 1000);

fn main() {
    let mut arguments = std::env::args().skip(1);
    let path = arguments.next().expect("a document to draw");
    let index: usize = arguments
        .next()
        .map_or(1, |n| n.parse().expect("a page number"));
    let rounds: usize = arguments
        .next()
        .map_or(3, |n| n.parse().expect("a count of rounds"));
    let counts: Vec<u32> = arguments.next().map_or_else(
        || vec![4, 8, 16],
        |list| {
            list.split(',')
                .map(|n| n.parse().expect("a strip count"))
                .collect()
        },
    );

    let bytes = std::fs::read(&path).expect("the document is readable");
    let document = pdf_syntax::Document::open(bytes).expect("it opens");
    let pages = pdf_model::Pages::new(&document);
    let page = pages
        .get(index.saturating_sub(1))
        .expect("that page exists");
    let list = pdf_model::content::interpret(&document, &page).display_list;
    println!(
        "{path} page {index}: {} top-level commands; pool {} threads; counts {counts:?}; {rounds} rounds",
        list.command_count(),
        rayon::current_num_threads()
    );

    let unit = TargetSpec::for_page(&list, 1.0, u64::MAX).expect("a target at one");
    let fit = (WINDOW.0 as f32 / unit.width as f32).min(WINDOW.1 as f32 / unit.height as f32);
    let zoomed_page = TargetSpec::for_page(&list, fit * 4.0, u64::MAX).expect("a zoomed page");
    let targets = [
        (
            "150 dpi page",
            TargetSpec::for_page(&list, 150.0 / 72.0, u64::MAX).expect("a 150 dpi target"),
        ),
        ("fit window", window_at(&list, fit)),
        ("4x window", window_at(&list, fit * 4.0)),
        ("4x whole page", zoomed_page),
        ("4x band", band_of(zoomed_page, WINDOW.1)),
    ];

    for (name, target) in targets {
        // One list per target, so that the bounds the cull keeps are measured once per
        // magnification as a viewer's are, and not in the first count's clock.
        let fresh = list.clone();
        let planned = fresh
            .reaching(target)
            .map_or_else(|| fresh.clone(), |window| window.copied());
        let mut clocks = vec![Vec::new(); counts.len()];
        let mut digests = vec![0_u64; counts.len()];
        for _ in 0..rounds {
            for (at, strips) in counts.iter().enumerate() {
                let started = Instant::now();
                let raster = CpuRasterizer::new()
                    .with_strips(*strips)
                    .rasterize(&fresh, target)
                    .expect("the processor draws this page");
                clocks[at].push(started.elapsed().as_secs_f64() * 1000.0);
                digests[at] = digest(&raster.data);
            }
        }
        for (at, strips) in counts.iter().enumerate() {
            let granted = CpuRasterizer::new()
                .with_strips(*strips)
                .strip_plan(&planned, target)
                .len()
                .saturating_sub(1);
            let quickest = clocks[at].iter().copied().fold(f64::INFINITY, f64::min);
            let slowest = clocks[at].iter().copied().fold(0.0, f64::max);
            println!(
                "{name:<14} {} x {}, {} commands reach it: {strips:>2} asked, {granted:>2} granted, quickest {quickest:>8.1} ms, slowest {slowest:>8.1} ms, digest {:016x}",
                target.width,
                target.height,
                planned.command_count(),
                digests[at]
            );
        }
    }
}

/// The window at `scale`, centred on the page, as `viewer_core` centres a page larger than it.
fn window_at(list: &DisplayList, scale: f32) -> TargetSpec {
    let page = TargetSpec::for_page(list, scale, u64::MAX).expect("a magnified target");
    let origin = (
        -((page.width.saturating_sub(WINDOW.0)) as f32 / 2.0).round(),
        -((page.height.saturating_sub(WINDOW.1)) as f32 / 2.0).round(),
    );
    TargetSpec {
        width: WINDOW.0.min(page.width),
        height: WINDOW.1.min(page.height),
        transform: page
            .transform
            .then(Transform::translate(origin.0, origin.1)),
    }
}

/// The `rows` rows of `page` a centred window shows, across the page's whole width.
fn band_of(page: TargetSpec, rows: u32) -> TargetSpec {
    let rows = rows.min(page.height);
    let first = page.height.saturating_sub(rows) / 2;
    TargetSpec {
        width: page.width,
        height: rows,
        transform: page
            .transform
            .then(Transform::translate(0.0, -(first as f32))),
    }
}

/// A 64-bit FNV-1a digest of a raster's bytes, so that two counts' pixels can be told apart.
fn digest(data: &[u8]) -> u64 {
    data.iter().fold(0xcbf2_9ce4_8422_2325, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(0x0000_0100_0000_01b3)
    })
}

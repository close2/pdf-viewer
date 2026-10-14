//! How one page's draw on the processor's pool scales with the pool, stage by stage.
//!
//! ```text
//! RAYON_NUM_THREADS=12 cargo run --release -p viewer-host --example pooled_draw -- document.pdf [width] [height] [rounds]
//! ```
//!
//! The page is read once, as a window reads it, and drawn `rounds` times at the fit of a
//! `width` × `height` window on the global rayon pool, whose width the environment states — one
//! process per pool, alternated by the caller so that a neighbour's build lands on each. Each draw
//! is of a fresh copy of the list, so the bounds every draw measures first are measured in the
//! figure as a window's first frame measures them (ADR 1841). Printed per draw, in milliseconds:
//! the bounds, the strip plan with the bounds kept, the draw whole as `CpuRasterizer::rasterize`
//! draws it, and how many strips the plan cut. `POOLED_EVEN=15,24` draws the page whole a second
//! time at each count of even cuts, as a page drawn ahead in strips is finished
//! (`render_cpu::Underway::in_strips`), so that a division's balance is priced apart from the
//! planner's. ADR 1896 is what it was written to answer.

#![expect(
    clippy::expect_used,
    clippy::print_stdout,
    clippy::arithmetic_side_effects,
    reason = "a measurement tool: it stops loudly on a missing input, prints its answer, and its \
              arithmetic is on a target's rows and a count of strips far inside a u64"
)]

use std::time::{Duration, Instant};

use pdf_render::{DisplayList, Rasterizer as _, TargetSpec};
use render_cpu::CpuRasterizer;

fn main() {
    let mut arguments = std::env::args().skip(1);
    let path = arguments.next().expect("a document to draw");
    let width: u32 = arguments
        .next()
        .map_or(1416, |w| w.parse().expect("a window width"));
    let height: u32 = arguments
        .next()
        .map_or(1000, |h| h.parse().expect("a window height"));
    let rounds: usize = arguments
        .next()
        .map_or(3, |n| n.parse().expect("a count of rounds"));

    let bytes = std::fs::read(&path).expect("the document is readable");
    let document = pdf_syntax::Document::open(bytes).expect("it opens");
    let pages = pdf_model::Pages::new(&document);
    let page = pages.get(0).expect("a first page");
    let read = Instant::now();
    let list = pdf_model::content::interpret(&document, &page).display_list;
    println!(
        "{path}: {} commands read in {:.1} ms",
        list.command_count(),
        ms(read.elapsed())
    );
    let unit = TargetSpec::for_page(&list, 1.0, u64::MAX).expect("a target at one");
    #[expect(
        clippy::cast_precision_loss,
        reason = "a window's and a page's extents, far below 2^24"
    )]
    let fit = (width as f32 / unit.width as f32).min(height as f32 / unit.height as f32);
    let target = TargetSpec::for_page(&list, fit, u64::MAX).expect("a fit target");
    println!("target {} × {} at {fit:.4}", target.width, target.height);

    let threads = std::env::var("RAYON_NUM_THREADS").unwrap_or_else(|_| "the machine's".into());
    let fresh = || list.shape_list(list.commands().to_vec());
    let even: Vec<u32> = std::env::var("POOLED_EVEN").map_or_else(
        |_| Vec::new(),
        |counts| {
            counts
                .split(',')
                .map(|n| n.parse().expect("a count of strips"))
                .collect()
        },
    );
    for round in 0..rounds {
        let measured = fresh();
        let began = Instant::now();
        drop(measured.mark_bounds(target.transform));
        let bounds = began.elapsed();
        let began = Instant::now();
        let strips = CpuRasterizer::new()
            .strip_plan(&measured, target)
            .len()
            .saturating_sub(1);
        let plan = began.elapsed();
        drop(measured);
        let drawn = fresh();
        let whole = draw(CpuRasterizer::new(), &drawn, target);
        for count in &even {
            let cuts: Vec<u32> = (0..=*count)
                .map(|strip| {
                    u32::try_from(u64::from(target.height) * u64::from(strip) / u64::from(*count))
                        .expect("a row of the target")
                })
                .collect();
            let drawn = fresh();
            let took = draw(CpuRasterizer::new().with_cuts(&cuts), &drawn, target);
            println!(
                "round {round} pool {threads}: {count:>2} even strips, draw whole {:>7.1} ms",
                ms(took)
            );
        }
        println!(
            "round {round} pool {threads}: bounds {:>6.1} ms, plan {:>6.1} ms, {strips:>2} strips, \
             draw whole {:>7.1} ms",
            ms(bounds),
            ms(plan),
            ms(whole)
        );
    }
}

/// One draw of a list nothing has measured yet, timed.
fn draw(mut rasterizer: CpuRasterizer, list: &DisplayList, target: TargetSpec) -> Duration {
    let began = Instant::now();
    let raster = rasterizer
        .rasterize(list, target)
        .expect("the processor draws the page");
    let took = began.elapsed();
    drop(raster);
    took
}

fn ms(duration: Duration) -> f64 {
    duration.as_secs_f64() * 1e3
}

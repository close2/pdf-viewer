//! The number the brief's §6.2 judges everything by: wall-clock `rasterize` time
//! on the dense page, all three backends, same display list, same target.
//!
//! Run with `--release`. The CPU baseline the brief quotes is 5.9 ms at 1191×1684;
//! §6.2 calls a third of that a success and a tenth a clear win.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, missing_docs)]

use std::time::Instant;

use pdf_render::{Rasterizer, TargetSpec};
use pdf_syntax::Document;

fn fastest<R: Rasterizer>(backend: &mut R, list: &pdf_render::DisplayList, target: TargetSpec) -> f64
where
    R::Error: std::fmt::Display,
{
    let mut best = f64::INFINITY;
    for _ in 0..12 {
        let started = Instant::now();
        backend
            .rasterize(list, target)
            .unwrap_or_else(|e| panic!("refused: {e}"));
        best = best.min(started.elapsed().as_secs_f64() * 1e3);
    }
    best
}

fn main() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../doc/ISO_32000-2_sponsored_EC3.pdf");
    let bytes = std::fs::read(&path).unwrap();
    let document = Document::open(bytes).expect("opens");
    let pages = pdf_model::Pages::new(&document);
    let page = pages.get(6).expect("exists");
    let list = pdf_model::content::interpret(&document, &page).display_list;

    for scale in [1.0_f32, 1.9008] {
        let target = TargetSpec::for_page(&list, scale, 1 << 30).unwrap();
        let mut cpu = render_cpu::CpuRasterizer::new();
        let cpu_ms = fastest(&mut cpu, &list, target);
        let mut vello = render_gpu::GpuRasterizer::new_headless().expect("adapter");
        let vello_ms = fastest(&mut vello, &list, target);
        let mut quorra = render_quorra::QuorraRasterizer::new_headless().expect("adapter");
        let quorra_ms = fastest(&mut quorra, &list, target);
        println!(
            "page 6 @{scale} ({}x{}): cpu {cpu_ms:.2} ms, vello {vello_ms:.2} ms, quorra {quorra_ms:.2} ms",
            target.width, target.height
        );
    }
}

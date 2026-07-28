//! Opens, interprets and rasterises one document, for isolating a pathological file.
//!
//! `cargo run -p pdf-model --example open_one -- <file.pdf> [scale] [out.png]`
//!
//! The corpus gate cannot bound a document that never returns — a thread cannot be
//! cancelled — so isolating one means running it in a process that can be killed.

#![expect(
    clippy::print_stdout,
    clippy::expect_used,
    reason = "a diagnostic binary whose output is its purpose"
)]

use std::time::Instant;

use pdf_render::{Rasterizer, TargetSpec};
use pdf_syntax::Document;
use render_cpu::CpuRasterizer;

fn main() {
    let path = std::env::args().nth(1).expect("a path to a PDF");
    let started = Instant::now();
    let bytes = std::fs::read(&path).expect("readable");
    let Ok(document) = Document::open(bytes) else {
        println!("{path}: does not open ({:?})", started.elapsed());
        return;
    };
    println!("  opened at {:?}", started.elapsed());
    let pages = pdf_model::Pages::new(&document);
    println!(
        "  catalog {:?}",
        document.catalog().map(|c| format!("{c:?}"))
    );
    println!("  page count {}", pages.len());
    let Some(page) = pages.get(0) else {
        println!("{path}: no first page ({:?})", started.elapsed());
        return;
    };
    println!("  page one at {:?}", started.elapsed());
    let interpretation = pdf_model::interpret(&document, &page);
    println!(
        "  interpreted at {:?}, {} commands, unsupported {:?}",
        started.elapsed(),
        interpretation.display_list.commands().len(),
        interpretation.unsupported
    );
    let list = &interpretation.display_list;
    let clips: std::collections::HashSet<_> = list
        .commands()
        .iter()
        .filter_map(pdf_render::Command::clip)
        .collect();
    println!("  {} distinct clips referenced", clips.len());

    let scale: f32 = std::env::args()
        .nth(2)
        .and_then(|a| a.parse().ok())
        .unwrap_or(1.0);
    if let Ok(target) = TargetSpec::for_page(&interpretation.display_list, scale, 64 << 20) {
        println!(
            "  target {}x{} at {:?}",
            target.width,
            target.height,
            started.elapsed()
        );
        let raster = CpuRasterizer::new().rasterize(&interpretation.display_list, target);
        // Trap 1 in the handover: no metric tells you a page is *right*, so the third
        // argument writes the page out to be looked at. It is the cheapest diagnostic here
        // and the one most often skipped.
        if let (Some(out), Ok(raster)) = (std::env::args().nth(3), raster) {
            let handle = std::fs::File::create(&out).expect("writable");
            let mut encoder =
                png::Encoder::new(std::io::BufWriter::new(handle), raster.width, raster.height);
            encoder.set_color(png::ColorType::Rgba);
            encoder.set_depth(png::BitDepth::Eight);
            encoder
                .write_header()
                .expect("valid header")
                .write_image_data(&raster.data)
                .expect("pixel data matches the dimensions");
            println!("  wrote {out}");
        }
    }
    println!("{path}: done in {:?}", started.elapsed());
}

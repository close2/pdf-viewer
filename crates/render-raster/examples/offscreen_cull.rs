//! One page through the device offscreen twice — its whole list and the list of the marks that
//! reach the target (`DisplayList::reaching`, ADR 1841) — on each of raster's three lanes, with the
//! two frames compared byte for byte and each held against the CPU oracle.
//!
//! ADR 1841 culled the offscreen path and took it back out when one corpus page,
//! `bug946506.pdf`, moved on the gpu lane with 254 of its 462 commands off its page. §10.7.4 paints
//! only a pixel a shape's area meets, so a mark that meets no pixel of the target marks none of
//! it, and a frame that changes when such a mark is left out is a frame some other input decided.
//! This prints where: the pixels that differ, the box they lie in, the largest difference, and
//! the frame's own counters for each list — the commands raster encoded and culled, the resources
//! handed over, the residue regions — so that a difference can be put down to a stage before
//! anything is built on it (ADR 1857).
//!
//! ```sh
//! cargo run --release -p render-raster --example offscreen_cull -- bug946506 [scale]
//! ```
//!
//! `OFFSCREEN_THREADS` states the encode threads (default: what the viewer ships), and
//! `OFFSCREEN_PNG` names a directory each lane's two frames are written to.

#![expect(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::print_stdout,
    clippy::cast_possible_truncation,
    clippy::arithmetic_side_effects,
    reason = "a measurement example: a missing corpus document or adapter stops the run, the \
              numbers on stdout are the point, and the arithmetic is over one raster's pixels"
)]

use std::path::Path;

use pdf_render::{Raster, Rasterizer, TargetSpec};
use pdf_syntax::Document;
use render_cpu::CpuRasterizer;
use render_raster::QuorraRasterizer;

fn main() {
    let mut args = std::env::args().skip(1);
    let stem = args.next().unwrap_or_else(|| "bug946506".to_owned());
    let scale: f32 = args.next().map_or(1.0, |s| s.parse().expect("a scale"));
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../doc/pdf.js/test/pdfs");
    let bytes = std::fs::read(root.join(format!("{stem}.pdf"))).expect("the corpus document");
    let document = Document::open(bytes).expect("opens");
    let pages = pdf_model::Pages::new(&document);
    let page = pages.get(0).expect("has a first page");
    let list = pdf_model::content::interpret(&document, &page).display_list;
    let target = TargetSpec::for_page(&list, scale, 64 << 20).expect("a target");
    // As a list of its own, so that each backend is handed it as it is handed a page.
    let culled = list.reaching(target).map(|window| window.copied());
    let kept = culled
        .as_ref()
        .map_or(list.commands().len(), |c| c.commands().len());
    println!(
        "{stem} at {scale}: {}x{}, {} commands, {} reach the target",
        target.width,
        target.height,
        list.commands().len(),
        kept
    );
    let Some(culled) = culled else {
        println!("  every command reaches the target; nothing to compare");
        return;
    };
    let oracle = CpuRasterizer::new()
        .with_strips(render_cpu::MAX_STRIPS)
        .rasterize(&list, target)
        .unwrap();
    let threads = std::env::var("OFFSCREEN_THREADS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or_else(|| render_raster::options().encode_threads);
    for (name, coverage) in [
        ("cpu", raster_gpu::Coverage::Cpu),
        ("gpu", raster_gpu::Coverage::Gpu),
        ("compute", raster_gpu::Coverage::Compute),
    ] {
        let mut frames = Vec::new();
        for (which, drawn) in [("whole", &list), ("culled", &culled)] {
            // A device of its own per frame, so that nothing the first frame cached decides the
            // second.
            let mut backend = QuorraRasterizer::with_options(&raster_gpu::Options {
                encode_threads: threads,
                ..render_raster::options()
            })
            .expect("adapter");
            backend.set_coverage(coverage);
            let raster = backend.rasterize(drawn, target).unwrap();
            let cost = backend.last_frame();
            let error = raster_compare::compare(&oracle, &raster).unwrap();
            println!(
                "  {name:7} {which:6}: commands {} culled {} uploads {} residue {:?} \
                 oracle mean {:.4} differing {:.4}",
                cost.commands,
                cost.commands_culled,
                cost.uploads,
                backend.last_clip_residue(),
                error.mean_error,
                error.differing_fraction
            );
            if let Ok(dir) = std::env::var("OFFSCREEN_PNG") {
                std::fs::create_dir_all(&dir).unwrap();
                write(
                    &Path::new(&dir).join(format!("{stem}-{name}-{which}.png")),
                    &raster,
                );
            }
            frames.push(raster);
        }
        difference(name, &frames[0], &frames[1]);
    }
}

/// The pixels where `whole` and `culled` differ: how many, the box they lie in and the largest
/// channel difference.
fn difference(name: &str, whole: &Raster, culled: &Raster) {
    let (mut count, mut worst) = (0_u64, 0_u8);
    let (mut x0, mut y0, mut x1, mut y1) = (u32::MAX, u32::MAX, 0, 0);
    for (index, (a, b)) in whole
        .data
        .chunks_exact(4)
        .zip(culled.data.chunks_exact(4))
        .enumerate()
    {
        let most = a
            .iter()
            .zip(b)
            .map(|(p, q)| p.abs_diff(*q))
            .max()
            .unwrap_or(0);
        if most > 0 {
            let (x, y) = (index as u32 % whole.width, index as u32 / whole.width);
            count += 1;
            worst = worst.max(most);
            (x0, y0, x1, y1) = (x0.min(x), y0.min(y), x1.max(x), y1.max(y));
        }
    }
    if count == 0 {
        println!("  {name:7}: the two frames are the same bytes");
    } else {
        println!(
            "  {name:7}: {count} pixels differ, by up to {worst} levels, within x {x0}..={x1} y \
             {y0}..={y1}"
        );
    }
}

fn write(at: &Path, raster: &Raster) {
    let file = std::fs::File::create(at).unwrap();
    let mut encoder = png::Encoder::new(std::io::BufWriter::new(file), raster.width, raster.height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder
        .write_header()
        .unwrap()
        .write_image_data(&raster.data)
        .unwrap();
}

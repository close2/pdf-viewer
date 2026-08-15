//! Where the two backends stop agreeing as a page is magnified.
//!
//! A viewer at 4000% does not rasterise the whole page — it rasterises the *window*, at a
//! transform that scales the page and translates the region of interest into view. This walks
//! that transform up a ladder of magnifications and compares the two backends on each rung, so
//! that "the characters go wrong at a certain zoom" becomes a number and a picture.
//!
//! ```sh
//! cargo run --release -p render-quorra --example zoom_ladder -- [file.pdf] [page] [out-dir]
//! ```
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    missing_docs
)]

use pdf_render::{Rasterizer, TargetSpec, Transform};
use pdf_syntax::Document;
use render_cpu::CpuRasterizer;
use render_quorra::QuorraRasterizer;

/// The window this pretends to be, which is where the comparison happens.
const WINDOW: (u32, u32) = (900, 1100);

fn main() {
    let mut arguments = std::env::args().skip(1);
    let path = arguments.next().unwrap_or_else(|| {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../doc/PDF20_AN001-BPC.pdf")
            .to_string_lossy()
            .into_owned()
    });
    let page: usize = arguments
        .next()
        .and_then(|index| index.parse().ok())
        .unwrap_or(3);
    let out = arguments.next();

    let document = Document::open(std::fs::read(&path).unwrap()).expect("opens");
    let pages = pdf_model::Pages::new(&document);
    let page = pages.get(page.saturating_sub(1)).expect("the page exists");
    let list = pdf_model::content::interpret(&document, &page).display_list;
    let size = list.page_size;

    let mut cpu = CpuRasterizer::new();
    // **The lane the viewer would use for this rung**, which is the whole point: `pdf-viewer.rs`
    // switches quorra to the GPU coverage lane above 10× magnification, so a ladder drawn with
    // the default lane never exercises what a person sees past 1000%.
    let lane = |zoom: f32| {
        if zoom >= 10.0 {
            quorra_gpu::Coverage::Gpu
        } else {
            quorra_gpu::Coverage::Cpu
        }
    };
    let mut gpu = QuorraRasterizer::with_options(&quorra_gpu::Options {
        coverage: quorra_gpu::Coverage::Cpu,
        ..render_quorra::options()
    })
    .expect("an adapter");

    println!("{path} — page size {:.1} × {:.1}", size.width, size.height);
    println!(
        "{:>4} {:>9}  {:>12}  {:>9} {:>9} {:>9}",
        "leg", "zoom", "target", "mean", "worst", "ssim"
    );
    // Up the ladder and back down it, through **one** rasteriser: the owner's report is that the
    // page is right on the way up, goes wrong, and stays wrong on the way down — which is a
    // statement about state that survives a frame, and the only way to see it is to keep the
    // device rather than make a new one per rung.
    // `ZOOM_LADDER_FROM` starts the ladder part-way up, which is how "is the first frame at this
    // magnification already wrong, or does it take a bigger one to break it" is asked.
    let from: f32 = std::env::var("ZOOM_LADDER_FROM")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(1.0);
    let mut rungs: Vec<f32> = Vec::new();
    let mut zoom = from;
    while zoom <= 64.0 {
        rungs.push(zoom);
        zoom *= 2.0;
    }
    let descent: Vec<f32> = rungs.iter().rev().skip(1).copied().collect();
    for (leg, zoom) in rungs
        .iter()
        .map(|zoom| ("up", *zoom))
        .chain(descent.iter().map(|zoom| ("down", *zoom)))
    {
        let zoom: f32 = zoom;
        // The middle of the page, held in the middle of the window — which is what the keyboard
        // zoom does: no anchor, so the core keeps the viewport's centre.
        let (w, h) = (WINDOW.0 as f32, WINDOW.1 as f32);
        // The page's own transform at this magnification — `for_page` composes §7.7.3.3's
        // rotation and the y flip, which a hand-written scale would get wrong — and then the
        // translation that holds the page's middle in the window's middle.
        let page = TargetSpec::for_page(&list, zoom, u64::MAX).expect("a page has an extent");
        let centre = Transform::translate(
            w.mul_add(0.5, -(size.width * zoom * 0.5)),
            h.mul_add(0.5, -(size.height * zoom * 0.5)),
        );
        let target = TargetSpec {
            width: WINDOW.0,
            height: WINDOW.1,
            transform: page.transform.then(centre),
        };
        gpu.set_coverage(lane(zoom));
        let ours = cpu.rasterize(&list, target).expect("the CPU draws it");
        match gpu.rasterize(&list, target) {
            Ok(theirs) => {
                let c = raster_compare::compare(&ours, &theirs).unwrap();
                println!(
                    "{leg:>4} {:>8.0}%  {:>5} × {:<5}  {:>9.4} {:>9.2} {:>9.5}",
                    zoom * 100.0,
                    (size.width * zoom) as u32,
                    (size.height * zoom) as u32,
                    c.mean_error,
                    c.worst_tile_error,
                    c.structural_similarity
                );
                if let Some(dir) = out.as_ref() {
                    let stem = format!("{dir}/{leg}-zoom{:05.0}", zoom * 100.0);
                    write(&format!("{stem}-cpu.png"), &ours);
                    write(&format!("{stem}-gpu.png"), &theirs);
                }
            }
            Err(error) => println!("{:>8.0}%  refused: {error}", zoom * 100.0),
        }
    }
}

fn write(path: &str, raster: &pdf_render::Raster) {
    let file = std::fs::File::create(path).unwrap();
    let mut encoder = png::Encoder::new(file, raster.width, raster.height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder
        .write_header()
        .unwrap()
        .write_image_data(&raster.data)
        .unwrap();
}

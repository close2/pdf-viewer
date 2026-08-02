//! Where a GPU frame's time goes: CPU scene encoding against device execution.
//!
//! `doc/gpu.txt` closes on a measurement nobody had taken — "how much of a frame is CPU
//! encoding versus GPU execution?" — and it is the number that decides where a document
//! renderer should spend its effort. Vello re-encodes the whole scene on the CPU every frame,
//! so if encoding dominates, a retained scene is the win; if execution dominates, the atlas
//! and the pipeline count are.
//!
//! ```sh
//! cargo run --release -p render-gpu --example frame_split -- [file.pdf] [page] [scale]
//! ```
#![expect(
    clippy::print_stdout,
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    reason = "a measurement tool: it should stop loudly if its input is missing, its \
              arithmetic is over one page's timings, and printing is the whole point"
)]

use pdf_render::Rasterizer as _;
use std::time::Instant;

fn main() {
    let mut args = std::env::args().skip(1);
    let path = args.next().unwrap_or_else(|| {
        format!(
            "{}/../../doc/ISO_32000-2_sponsored_EC3.pdf",
            env!("CARGO_MANIFEST_DIR")
        )
    });
    let index: usize = args.next().map_or(6, |n| n.parse().expect("a page number"));
    let scale: f32 = args.next().map_or(2.0, |n| n.parse().expect("a scale"));

    let bytes = std::fs::read(&path).expect("readable");
    let document = pdf_syntax::Document::open(bytes).expect("valid PDF");
    let page = pdf_model::Pages::new(&document)
        .get(index - 1)
        .expect("page exists");
    let list = pdf_model::interpret(&document, &page).display_list;
    let target = pdf_render::TargetSpec::for_page(&list, scale, 1 << 30).expect("a target");

    let mut gpu = render_gpu::GpuRasterizer::new_headless().expect("a GPU adapter");
    println!(
        "{path} page {index} at {scale}x: {}x{}, {} commands, adapter {}",
        target.width,
        target.height,
        list.command_count(),
        gpu.context().adapter_description()
    );

    // One warm render first: the first ever call compiles pipelines and allocates, which is a
    // startup number and not a frame number.
    let _ = gpu.rasterize(&list, target).expect("the page renders");

    // The fastest of ten rather than the mean, which is `strip_spans`'s habit and the same
    // argument: the least contended run is the one least polluted by whatever else the machine
    // is doing, and a mean over ten frames on a shared GPU is mostly noise.
    let rounds = 10;
    let mut encode = std::time::Duration::MAX;
    let mut whole = std::time::Duration::MAX;
    for _ in 0..rounds {
        let masks = render_gpu::SoftMaskRasters::default();
        let at = Instant::now();
        let scene = render_gpu::build_scene(&list, target, &masks).expect("a scene");
        encode = encode.min(at.elapsed());
        drop(scene);

        let at = Instant::now();
        let _ = gpu.rasterize(&list, target).expect("the page renders");
        whole = whole.min(at.elapsed());
    }
    // The floor: the same target, drawn from a list of one rectangle. What is left is texture
    // allocation, one submit, the readback and the demultiply — everything a frame pays that is
    // not this page's own drawing, and everything a *window* does not pay for the readback.
    let mut bare = pdf_render::DisplayList::new(list.page_size);
    let mut rect = pdf_render::Path::new();
    for point in [(0.0, 0.0), (8.0, 0.0), (8.0, 8.0), (0.0, 8.0)] {
        let point = pdf_render::Point::new(point.0, point.1);
        if rect.is_empty() {
            rect.push(pdf_render::PathCommand::MoveTo(point));
        } else {
            rect.push(pdf_render::PathCommand::LineTo(point));
        }
    }
    rect.push(pdf_render::PathCommand::Close);
    bare.push(pdf_render::Command::Fill {
        path: std::sync::Arc::new(rect),
        transform: pdf_render::Transform::IDENTITY,
        fill_rule: pdf_render::FillRule::NonZero,
        paint: pdf_render::Paint::Solid(pdf_render::Color::BLACK),
        clip: None,
        mask: None,
        blend: pdf_render::BlendMode::Normal,
    });
    let mut floor = std::time::Duration::MAX;
    for _ in 0..rounds {
        let at = Instant::now();
        let _ = gpu.rasterize(&bare, target).expect("one rectangle renders");
        floor = floor.min(at.elapsed());
    }

    let encode = encode.as_secs_f64() * 1000.0;
    let whole = whole.as_secs_f64() * 1000.0;
    let floor = floor.as_secs_f64() * 1000.0;
    println!(
        "  scene encoding {encode:>8.2} ms  ({:.0}% of the whole frame)",
        100.0 * encode / whole
    );
    println!("  whole frame    {whole:>8.2} ms  (encode, upload, execute, read back)");
    println!("  one rectangle  {floor:>8.2} ms  (the same target, from a list of one command)");
    // Stated as a difference and not as a component, because it is one: the two frames above
    // are measured and this is subtracted. On a small target it comes out at or below zero,
    // which is the finding rather than a bug — a page of thousands of glyphs costs about what
    // one rectangle costs at the same size, so what a frame pays for is pixels and not
    // content. Separating the readback from the execution needs timestamp queries, which this
    // harness does not have.
    println!(
        "  difference     {:>8.2} ms  (the page's own content, where it is above the noise)",
        whole - floor
    );
}

//! A page drawn in a confined process, with what each step of it cost.
//!
//! ```sh
//! cargo run --release -p viewer-confined --example confined_page -- file.pdf [page] [out.png]
//! ```
//!
//! What it prints is the thing `CLAUDE.md`'s startup rules would want to know before this path
//! ever went in front of a first frame: what a worker costs to start and confine, what the
//! document costs to hand across a pipe, and what a page costs to interpret and draw behind the
//! filter — each separately, because they are three different decisions.
//!
//! It also prints the confinement the worker reached, or [`Confinement::shortfall`]'s sentence
//! where it reached less than everything. A person running this on a kernel without seccomp-BPF
//! should be told, not left to assume.

#![expect(
    clippy::print_stdout,
    clippy::expect_used,
    clippy::panic,
    reason = "an example whose whole output is what it printed; a run that cannot do the thing \
              should stop loudly rather than print a number about something else"
)]

use std::time::Instant;

use viewer_confined::{Confined, Reply};
use viewer_core::{Command, DocumentId, Event, PageTarget, Query};

#[expect(
    clippy::too_many_lines,
    reason = "one straight line of measurement, and splitting it would put the numbers in one \
              function and what they are of in another"
)]
fn main() {
    let mut arguments = std::env::args().skip(1);
    let Some(path) = arguments.next() else {
        eprintln!("usage: confined_page <file.pdf> [page] [out.png]");
        std::process::exit(2);
    };
    let page: usize = arguments
        .next()
        .map_or(1, |text| text.parse().expect("a page number"));
    let out = arguments.next();

    let bytes = std::fs::read(&path).expect("the document is readable");
    println!("{path}: {} bytes", bytes.len());

    let started = Instant::now();
    let mut confined = Confined::start().expect("a confined viewer starts");
    let spawned = started.elapsed();
    let confinement = confined.confinement();
    println!(
        "worker started and confined in {:.3} ms — {}",
        spawned.as_secs_f64() * 1e3,
        confinement.shortfall().unwrap_or_else(|| {
            "seccomp, Landlock and an address-space ceiling, all enforced".to_owned()
        })
    );
    println!(
        "  landlock {:?}, address space {} MiB, system calls {:?}",
        confinement.landlock,
        confinement.address_space_limit / (1 << 20),
        confinement.system_calls
    );

    confined
        .handle(&Command::Resize {
            width: 900,
            height: 1200,
            scale: 1.0,
        })
        .expect("a resize crosses");

    let at = Instant::now();
    let events = confined
        .handle(&Command::Open {
            id: DocumentId(1),
            bytes,
            password: None,
            fragment: None,
        })
        .expect("an open crosses");
    println!(
        "opened, interpreted and drew page 1 in {:.3} ms",
        at.elapsed().as_secs_f64() * 1e3
    );
    for event in &events {
        match event {
            Event::Opened { pages, .. } => println!("  {pages} page(s)"),
            Event::Reported { notes, .. } => {
                for note in notes {
                    println!("  reported: {note}");
                }
            }
            other => println!("  {other:?}"),
        }
    }

    if page > 1 {
        let at = Instant::now();
        confined
            .handle(&Command::GoTo(PageTarget::Index(page.saturating_sub(1))))
            .expect("a page turn crosses");
        println!(
            "turned to page {page} in {:.3} ms",
            at.elapsed().as_secs_f64() * 1e3
        );
    }

    let at = Instant::now();
    let Reply::Frame { raster, .. } = confined.query(Query::Frame).expect("a frame crosses") else {
        panic!("the confined viewer holds the frame it drew");
    };
    println!(
        "{}x{} pixels crossed the pipe in {:.3} ms",
        raster.width,
        raster.height,
        at.elapsed().as_secs_f64() * 1e3
    );
    let ink = raster.data.chunks_exact(4).filter(|p| p[0] < 200).count();
    println!("  {ink} dark pixels");

    // The same work in this process, so that what the confinement costs is a difference rather
    // than a number on its own. Two things are folded into it and both are named in ADR 0218: the
    // pipe, and the one rasterising thread a confined process is held to.
    let bytes = std::fs::read(&path).expect("the document is readable");
    let at = Instant::now();
    let mut viewer = viewer_core::Viewer::new(900, 1200, 1.0);
    let mut rasterizer = render_cpu::CpuRasterizer::new();
    let mut pending = vec![Command::Open {
        id: DocumentId(1),
        bytes,
        password: None,
        fragment: None,
    }];
    if page > 1 {
        pending.push(Command::GoTo(PageTarget::Index(page.saturating_sub(1))));
    }
    while let Some(command) = pending.pop() {
        for event in viewer.handle(command) {
            if let Event::NeedsRender(request) = event {
                use pdf_render::Rasterizer as _;
                let raster = rasterizer
                    .rasterize(&request.list, request.target)
                    .expect("the CPU backend draws this page");
                pending.push(Command::RenderReady {
                    token: request.token,
                    rendered: viewer_core::Rendered::Raster(raster),
                });
            }
        }
    }
    println!(
        "unconfined, in this process, on every core: {:.3} ms",
        at.elapsed().as_secs_f64() * 1e3
    );

    if let Some(out) = out {
        let file = std::fs::File::create(&out).expect("the output is writable");
        let mut encoder =
            png::Encoder::new(std::io::BufWriter::new(file), raster.width, raster.height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        encoder
            .write_header()
            .expect("a header")
            .write_image_data(&raster.data)
            .expect("the pixels");
        println!("wrote {out}");
    }
}

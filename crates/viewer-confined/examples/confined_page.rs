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

use viewer_confined::{Confined, Payload, Reply};
use viewer_core::{Command, DocumentId, Event, PageTarget, Query};

/// A valid one-page document padded to about `ballast` bytes with a stream nothing refers to.
///
/// The point is a document whose *size* is the ISO specification's and whose *cost* is nothing,
/// so that what is timed is the pipe. Object 4 is never reached from the catalogue, so the reader
/// never inflates it or even looks at it: it reads the cross-reference table, the catalogue, the
/// page tree and a page with no contents.
fn ballasted(ballast: usize) -> Vec<u8> {
    let objects = [
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] >>".to_vec(),
        {
            let mut stream = format!("<< /Length {ballast} >>\nstream\n").into_bytes();
            stream.resize(stream.len().saturating_add(ballast), b'0');
            stream.extend_from_slice(b"\nendstream");
            stream
        },
    ];

    let mut bytes: Vec<u8> = b"%PDF-1.7\n".to_vec();
    let mut offsets: Vec<usize> = Vec::new();
    for (index, object) in objects.iter().enumerate() {
        offsets.push(bytes.len());
        bytes.extend_from_slice(format!("{} 0 obj\n", index.saturating_add(1)).as_bytes());
        bytes.extend_from_slice(object);
        bytes.extend_from_slice(b"\nendobj\n");
    }
    let table = bytes.len();
    let size = objects.len().saturating_add(1);
    bytes.extend_from_slice(format!("xref\n0 {size}\n0000000000 65535 f \n").as_bytes());
    for offset in &offsets {
        bytes.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    bytes.extend_from_slice(
        format!("trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{table}\n%%EOF\n").as_bytes(),
    );
    bytes
}

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
    let bytes_len = bytes.len();
    println!("{path}: {bytes_len} bytes");

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
    // **"Drew" is only half true since ADR 0640**, and the wording says so rather than flattering
    // the number: the confined worker rasterises a page only where the page's *pixels* are the
    // payload that crosses. A page whose marks are smaller is interpreted, measured and shipped
    // undrawn — so on that arm this line is an interpretation and a pipe, and the drawing appears
    // below, once, on the host's side where it always had to happen anyway.
    println!(
        "opened, interpreted and readied page 1 in {:.3} ms",
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
    let Reply::Frame(frames) = confined.query(Query::Frame).expect("a frame crosses") else {
        panic!("the confined viewer holds the frame it drew");
    };
    let crossed = at.elapsed();
    let Some(shown) = frames.first() else {
        panic!("a frame crossed with no pages in it");
    };

    // **Both arms of ADR 0607's choice, and which one this page took.** The list arm is the
    // measurement the seven-hundred-and-thirty-sixth session wired in: what crossed, and what a
    // raster of the same target would have been. The host draws the list itself, which is what
    // the second timing below is — a cost this side of the boundary did not use to pay and which
    // belongs in the same run as the saving it buys.
    //
    // **Since ADR 0640 that draw is the only one there is.** The worker used to do it too and
    // throw the pixels away; the line above is what fell when it stopped.
    let raster = match &shown.payload {
        Payload::Raster(raster) => {
            println!(
                "{}x{} pixels crossed the pipe in {:.3} ms",
                raster.width,
                raster.height,
                crossed.as_secs_f64() * 1e3
            );
            raster.clone()
        }
        Payload::List { list, target } => {
            let pixels = u64::from(target.width)
                .saturating_mul(u64::from(target.height))
                .saturating_mul(4);
            let bytes = viewer_confined::wire::encode_display_list(list)
                .expect("a list the worker encoded encodes here")
                .len();
            println!(
                "a display list for {}x{} crossed the pipe in {:.3} ms: {bytes} B against \
                 {pixels} B of pixels",
                target.width,
                target.height,
                crossed.as_secs_f64() * 1e3
            );
            let at = Instant::now();
            let raster = {
                use pdf_render::Rasterizer as _;
                render_cpu::CpuRasterizer::new()
                    .rasterize(list, *target)
                    .expect("the host draws the list it was handed")
            };
            println!(
                "  the host drew it on the processor in {:.3} ms",
                at.elapsed().as_secs_f64() * 1e3
            );
            raster
        }
    };
    let ink = raster.data.chunks_exact(4).filter(|p| p[0] < 200).count();
    println!("  {ink} dark pixels");

    // **The document crossing, with nothing on the other side to interpret.**
    //
    // `doc/todo/34`'s item 5 asks what the pipe costs, and the open above cannot say: it is the
    // pipe, the interpretation and the render together. So this sends the same number of bytes in
    // a document that is *valid and empty* — a one-page catalogue plus one stream nothing refers
    // to — which the confined side reads whole, parses in microseconds and draws blank. The
    // difference between the two lines below is the bytes; what is left in the second is the
    // frame, the process and the blank page.
    for (id, ballast) in [(2u64, bytes_len), (3, 0)] {
        let at = Instant::now();
        confined
            .handle(&Command::Open {
                id: DocumentId(id),
                bytes: ballasted(ballast),
                password: None,
                fragment: None,
            })
            .expect("an open crosses");
        println!(
            "{ballast} bytes of ballast crossed and drew blank in {:.3} ms",
            at.elapsed().as_secs_f64() * 1e3
        );
    }

    // The same work in this process, so that what the confinement costs is a difference rather
    // than a number on its own. **Twice**, because two things are folded into that difference and
    // ADR 0218 names both: the pipe, and the one rasterising thread a confined process is held
    // to. One strip isolates the second — it is what the worker does — so the gap between the two
    // lines is `doc/todo/34`'s item 4 and the gap to the confined figure above is its item 5.
    for strips in [1, 0] {
        let bytes = std::fs::read(&path).expect("the document is readable");
        let at = Instant::now();
        let mut viewer = viewer_core::Viewer::new(900, 1200, 1.0);
        let mut rasterizer = render_cpu::CpuRasterizer::new();
        if strips > 0 {
            rasterizer = rasterizer.with_strips(strips);
        }
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
        let how = if strips > 0 {
            "on one strip, as the worker does"
        } else {
            "on every core"
        };
        println!(
            "unconfined, in this process, {how}: {:.3} ms",
            at.elapsed().as_secs_f64() * 1e3
        );
    }

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

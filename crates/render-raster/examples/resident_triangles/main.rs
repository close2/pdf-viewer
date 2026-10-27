//! The game-like lane, priced: a page's marks resident on the device as triangles, drawn by
//! the hardware rasteriser under multisampling, with a zoom step changing one uniform.
//!
//! The project owner asked on 2026-10-26 why the page is not sent to the device "in a way,
//! that it's GPU elements", as a game's scene is, and whether "the PDF paint instructions
//! [are] completely incompatible to GPU painting instructions". The tree's compute lane keeps
//! a page resident and recomputes exact coverage per view in compute passes (ADRs 0698, 2315,
//! 2355); ADR 0368 refused a tessellation lane drawn into coverage tiles. This probe prices the
//! shape neither priced: stencil-then-cover fills and Loop–Blinn curve triangles uploaded once,
//! recorded once as a render bundle, drawn straight into an MSAA target at a view that is a
//! uniform. RFC 0013 carries the figures and what the lane gives up; nothing here is a lane of
//! the product.
//!
//! It lives in `render-raster` rather than under `raster/` because it reads real documents,
//! and raster never sees a PDF byte (raster's CLAUDE.md, "Who our caller is").
//!
//! ```sh
//! cargo run --release -p render-raster --example resident_triangles -- <file.pdf> <page[,page]> [WxH]
//! RESIDENT_SAMPLES=4|8          the MSAA sample count (default 8)
//! RESIDENT_LADDER=0.5,1,2,…     magnifications, multiples of the fit to the window's width
//! RESIDENT_ROUNDS=5             round-robin rounds; each figure is the minimum
//! RESIDENT_ADAPTER=llvmpipe     an adapter by name (default: the first hardware adapter)
//! RESIDENT_ORACLE=1             compare each rung with render-cpu drawing the carried marks
//! RESIDENT_DIGEST=1             print each rung's FNV-1a digest, for a cross-adapter check
//! RESIDENT_DUMP=<dir>           write each rung's PNG (and the oracle's, with RESIDENT_ORACLE)
//! RESIDENT_INTO_GROUPS=1        walk a group whose compositing is not carried, for its geometry
//! ```
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::print_stdout,
    clippy::cast_precision_loss,
    clippy::too_many_lines,
    // A probe's counters and page-space geometry, bounded by one document's marks.
    clippy::arithmetic_side_effects,
    missing_docs
)]

mod gpu;
mod mesh;
mod oracle;

use std::time::{Duration, Instant};

use pdf_render::{TargetSpec, Transform};
use pdf_syntax::Document;

/// The window ADR 2408 §2 measured the compute lane in, in device pixels.
const WINDOW: (u32, u32) = (800, 1600);

/// A column's gap between pages, in page units; its size moves no figure here.
const GAP: f32 = 8.0;

/// The highest magnification the resident quadratics are cut for: a sixty-fourth of a device
/// pixel there, so every rung of the default ladder draws the cut below what a sample can see.
const CUT_FOR: f64 = 64.0;

fn variable(name: &str) -> Option<String> {
    std::env::var(name).ok().filter(|value| !value.is_empty())
}

fn ms(span: Duration) -> f64 {
    span.as_secs_f64() * 1e3
}

fn main() {
    let mut arguments = std::env::args().skip(1);
    let path = arguments.next().expect("a document");
    let pages: Vec<usize> = arguments
        .next()
        .unwrap_or_else(|| "1".to_owned())
        .split(',')
        .map(|page| page.parse().expect("a page number"))
        .collect();
    let window = arguments.next().map_or(WINDOW, |spec| {
        let (w, h) = spec.split_once('x').expect("a window as WIDTHxHEIGHT");
        (w.parse().expect("a width"), h.parse().expect("a height"))
    });
    let samples: u32 = variable("RESIDENT_SAMPLES").map_or(8, |n| n.parse().expect("a count"));
    let rounds: usize = variable("RESIDENT_ROUNDS").map_or(5, |n| n.parse().expect("a count"));
    let ladder: Vec<f32> = variable("RESIDENT_LADDER")
        .unwrap_or_else(|| "0.5,1,1.25,2,4,8,16".to_owned())
        .split(',')
        .map(|f| f.trim().parse().expect("a magnification"))
        .collect();

    let document = Document::open(std::fs::read(&path).expect("readable")).expect("it opens");
    let all = pdf_model::Pages::new(&document);
    let started = Instant::now();
    let lists: Vec<pdf_render::DisplayList> = pages
        .iter()
        .map(|&n| {
            let page = all.get(n.saturating_sub(1)).expect("that page exists");
            pdf_model::content::interpret(&document, &page).display_list
        })
        .collect();
    let interpreted = started.elapsed();
    let first = &lists[0];
    let fit = window.0 as f32 / first.page_size.width;
    let top = f64::from(fit) * f64::from(ladder.iter().copied().fold(1.0, f32::max)).max(CUT_FOR);
    let tolerance = 1.0 / 64.0 / top;

    let started = Instant::now();
    let mut mesh = mesh::Mesh {
        into_groups: variable("RESIDENT_INTO_GROUPS").is_some(),
        ..mesh::Mesh::default()
    };
    let mut below = 0.0_f32;
    for list in &lists {
        mesh::append(
            &mut mesh,
            list,
            Transform::translate(0.0, -below),
            tolerance,
        );
        below += list.page_size.height + GAP;
    }
    let tessellated = started.elapsed();

    let wanted = variable("RESIDENT_ADAPTER");
    let lane = match gpu::Lane::new(wanted.as_deref(), samples, window, &mesh) {
        Ok(lane) => lane,
        Err(refusal) => {
            println!("refused: {refusal}");
            std::process::exit(2);
        }
    };

    println!(
        "{path} page(s) {pages:?}, {}x{} window, {samples}x MSAA, {} ({})",
        window.0, window.1, lane.adapter.name, lane.adapter.driver
    );
    println!("before: {}", machine());
    let c = mesh.census;
    println!(
        "carried: {} fill(s) ({} even-odd), {} stroke(s); {} translucent; {} group(s) flattened",
        c.fills, c.even_odd, c.strokes, c.translucent, c.flattened_groups
    );
    println!(
        "not carried: {} zero-width stroke(s), {} shading paint(s), {} image(s), {} group(s), \
         {} blend or mask, {} other; {} clipped command(s) drawn unclipped\n\
         {} image sample(s) decoded, {} image(s) at device scale\n\
         groups not runs, by reason (alpha, mask, blend, knockout, space): {:?}; {} walked anyway",
        c.zero_width_strokes,
        c.shaded,
        c.images,
        c.other_groups,
        c.other_blend_or_mask,
        c.other_commands,
        c.clipped,
        c.image_samples,
        c.images_at_device_scale,
        c.group_reasons,
        c.groups_as_runs
    );
    println!(
        "resident: {} batch(es), {} fan + {} curve + {} cover vertices, {} quadratic(s), {} bytes",
        mesh.batches.len(),
        mesh.fan.len() / mesh::STRIDE,
        mesh.curve.len() / mesh::STRIDE,
        mesh.cover.len() / mesh::STRIDE,
        mesh.quads,
        mesh.vertex_bytes()
    );
    println!(
        "once: interpret {:.1} ms, tessellate {:.1} ms, upload {:.1} ms, pipelines {:.1} ms, \
         bundle {:.1} ms",
        ms(interpreted),
        ms(tessellated),
        ms(lane.upload.buffers),
        ms(lane.upload.pipelines),
        ms(lane.upload.bundle)
    );

    let views: Vec<(f32, TargetSpec)> = ladder
        .iter()
        .map(|&factor| (factor, placed(first, fit * factor, window)))
        .collect();
    // The first step pays the driver's first use of every pipeline; it is drawn and not
    // counted, and its own wall time is printed (trap 271).
    let first_use = lane.step(view_of(views[0].1.transform));
    println!(
        "first step (not counted): host {:.3} ms, device {:.3} ms, wall {:.3} ms",
        ms(first_use.host),
        ms(first_use.device),
        ms(first_use.wall)
    );
    let mut best: Vec<Option<gpu::Step>> = vec![None; views.len()];
    for _ in 0..rounds {
        for (slot, (_, target)) in best.iter_mut().zip(&views) {
            let step = lane.step(view_of(target.transform));
            *slot = Some(slot.map_or(step, |kept| gpu::Step {
                host: kept.host.min(step.host),
                device: kept.device.min(step.device),
                wall: kept.wall.min(step.wall),
            }));
        }
    }
    println!("after: {}", machine());
    println!("minima of {rounds} round(s), milliseconds:");
    println!(
        "{:>8}{:>12}{:>10}{:>10}{:>10}",
        "zoom", "scale", "host", "device", "wall"
    );
    for ((factor, target), step) in views.iter().zip(&best) {
        let step = step.expect("every rung was drawn");
        println!(
            "{:>7}×{:>12.4}{:>10.3}{:>10.3}{:>10.3}",
            factor,
            target.transform.a.hypot(target.transform.b),
            ms(step.host),
            ms(step.device),
            ms(step.wall)
        );
    }

    let compare = variable("RESIDENT_ORACLE").is_some();
    let digest = variable("RESIDENT_DIGEST").is_some();
    let dump = variable("RESIDENT_DUMP").map(std::path::PathBuf::from);
    if !(compare || digest || dump.is_some()) {
        return;
    }
    let carried = compare.then(|| oracle::list(first, &mesh));
    if compare && lists.len() > 1 {
        println!("the oracle compares one page; a column is timed only");
    }
    for (factor, target) in &views {
        lane.step(view_of(target.transform));
        let drawn = lane.read();
        if digest {
            println!("digest {factor}× {:016x}", oracle::fnv(&drawn));
        }
        if let Some(dir) = &dump {
            oracle::png(&dir.join(format!("probe-{factor}.png")), window, &drawn);
        }
        if let (Some(list), true) = (&carried, lists.len() == 1) {
            let reference = oracle::draw(list, *target);
            if let Some(dir) = &dump {
                oracle::png(
                    &dir.join(format!("oracle-{factor}.png")),
                    window,
                    &reference.data,
                );
            }
            println!(
                "oracle {factor}×: {}",
                oracle::differ(&reference.data, &drawn)
            );
            println!("        {}", oracle::agreement(&reference, &drawn));
        }
    }
}

/// The page's middle in the window's middle, at `scale` — `examples/zoom_frame.rs`'s own
/// arithmetic, so the compute lane's figures from it are taken at the same views.
fn placed(list: &pdf_render::DisplayList, scale: f32, (width, height): (u32, u32)) -> TargetSpec {
    let page = TargetSpec::for_page(list, scale, 1 << 40).expect("a target");
    let size = list.page_size;
    let centre = Transform::translate(
        (width as f32).mul_add(0.5, -(size.width * scale * 0.5)),
        (height as f32).mul_add(0.5, -(size.height * scale * 0.5)),
    );
    TargetSpec {
        width,
        height,
        transform: page.transform.then(centre),
    }
}

fn view_of(t: Transform) -> [f32; 6] {
    [t.a, t.b, t.c, t.d, t.e, t.f]
}

/// The one-minute load, the graphics clock's levels with the current one starred (trap 275),
/// and the processes of this account holding a render node (trap 262).
fn machine() -> String {
    let load = std::fs::read_to_string("/proc/loadavg")
        .ok()
        .and_then(|l| l.split_whitespace().next().map(str::to_owned))
        .unwrap_or_else(|| "?".to_owned());
    let sclk = std::fs::read_dir("/sys/class/drm")
        .into_iter()
        .flatten()
        .flatten()
        .find_map(|card| std::fs::read_to_string(card.path().join("device/pp_dpm_sclk")).ok())
        .map_or_else(
            || "?".to_owned(),
            |table| table.lines().map(str::trim).collect::<Vec<_>>().join(" | "),
        );
    let me = std::process::id();
    let holders: Vec<String> = std::fs::read_dir("/proc")
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|entry| {
            let pid: u32 = entry.file_name().to_str()?.parse().ok()?;
            let holds = std::fs::read_dir(entry.path().join("fd"))
                .ok()?
                .flatten()
                .filter_map(|fd| std::fs::read_link(fd.path()).ok())
                .any(|target| target.to_string_lossy().starts_with("/dev/dri/renderD"));
            (holds && pid != me).then(|| {
                let name = std::fs::read_to_string(entry.path().join("comm")).unwrap_or_default();
                format!("{pid} {}", name.trim())
            })
        })
        .collect();
    format!(
        "load {load}, sclk [{sclk}], render node held by {}",
        if holders.is_empty() {
            "no other process of this account".to_owned()
        } else {
            holders.join(", ")
        }
    )
}

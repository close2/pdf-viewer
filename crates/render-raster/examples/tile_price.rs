//! What a page costs at window size, divided the two ways a viewer could stop paying for it.
//!
//! ADR 1817 asks which one change to the loading and display pipeline would move a page of the
//! owner's witness's shape — four million marks, every one of them smaller than a pixel at the
//! view a person opens it in — by an order of magnitude, and prices two candidates on the page
//! itself rather than in prose:
//!
//! - **the page is a set of tiles cut from one spatially culled list** — each tile a target of
//!   its own, drawn from only the commands whose extent meets it, on a thread of its own, so a
//!   view the window shows costs what is *in* the view and every tile is a picture that can be
//!   kept and moved;
//! - **a mark smaller than a pixel is drawn by a kernel of its own** — which is priced here by
//!   what the marks cost today against the page without them, since a kernel that cost nothing
//!   could remove no more than that.
//!
//! ```sh
//! cargo run --release -p render-raster --example tile_price -- <file.pdf> [page] [w] [h] [--gpu]
//! ```
//!
//! Four sections, all on one display list interpreted once:
//!
//! 1. **The census**: every leaf command's device extent at the window's fit, at 150 dpi and at
//!    four times the fit, binned by its longer side. A stroke's extent here is its path's hull
//!    grown by half its device width — the reach of a butt or round cap, the only caps a
//!    two-point path has to reach with — and not `Command::device_bounds`, which grows by the
//!    mitre limit too and is the bound a cull must use but would call a half-pixel dot a
//!    ten-pixel mark. A multi-segment stroke's join can reach further than that, so for those
//!    the figure is a lower bound, which is the honest direction for "how many are small".
//! 2. **The processor, one thread**: the fit page whole; the same page with every mark under a
//!    pixel left out (candidate two's price); a window at four times the fit drawn from the whole
//!    list, which is what `viewer_ui::software::compose_pages` does today; and the same window
//!    drawn from the commands `device_bounds` says can reach it (candidate one's cull).
//! 3. **The processor, four threads**: the fit page and the four-times window, each cut into a
//!    two-by-two grid of tiles drawn concurrently from their own culled lists, and the
//!    assembled tiles compared with the whole draw pixel for pixel — the seam is the risk.
//! 4. **The device** (`--gpu`, the real adapter headless): whether the whole fit frame, each fit
//!    tile and the four-times window are drawn or refused, and how long a drawn one took; and the
//!    four-times window handed over as the whole list, as a zoom step from the list's own fit
//!    frame and after another list's frame — the two cases a window's scene meets (ADR 1841).
//! 5. **The window as the shipped processor draws it** at the fit and at two, four and eight
//!    times it: the whole list handed to `CpuRasterizer::new()`, strips on this machine's pool as
//!    a window gets them, against the same window drawn as one strip on one thread — the time of
//!    each, the first draw apart from the quickest (the first is the one that measures the
//!    marks' bounds, ADR 1841), the strips the planner granted, the pixels the two differ by, and
//!    a digest of each so that two builds' windows can be compared byte for byte.
//!
//! Every duration is the minimum of three draws in this process; pin the run with `taskset` to
//! one class of core, as `doc/habits/measuring.md` says, because the arms are compared with one
//! another and not with any other program.
#![expect(
    clippy::expect_used,
    clippy::print_stdout,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::arithmetic_side_effects,
    clippy::too_many_lines,
    reason = "a measurement tool: it stops loudly on a missing input, prints its answer, and its \
              arithmetic is on pixel extents far inside every type it uses"
)]

use std::sync::Arc;
use std::time::{Duration, Instant};

use pdf_render::{
    Command, DisplayList, Point, Raster, Rasterizer as _, Rect, TargetSpec, Transform,
};
use render_cpu::CpuRasterizer;

/// How many draws each arm takes, of which the quickest is reported.
const DRAWS: usize = 3;

/// The upper edges of the census bins, in device pixels of a mark's longer side.
const BINS: [f32; 6] = [0.25, 0.5, 1.0, 2.0, 4.0, 16.0];

fn main() {
    let mut arguments = std::env::args().skip(1);
    let path = arguments.next().expect("a document to draw");
    let index: usize = arguments
        .next()
        .map_or(1, |n| n.parse().expect("a page number"));
    let window = (
        arguments
            .next()
            .map_or(1600, |w| w.parse::<u32>().expect("a window width")),
        arguments
            .next()
            .map_or(1000, |h| h.parse::<u32>().expect("a window height")),
    );
    let gpu = arguments.any(|flag| flag == "--gpu");

    let bytes = std::fs::read(&path).expect("the document is readable");
    let started = Instant::now();
    let document = pdf_syntax::Document::open(bytes).expect("it opens");
    let pages = pdf_model::Pages::new(&document);
    let page = pages
        .get(index.saturating_sub(1))
        .expect("that page exists");
    let list = pdf_model::content::interpret(&document, &page).display_list;
    let interpreted = started.elapsed();
    let leaves = leaf_count(list.commands());
    println!(
        "{path} page {index}: opened and interpreted in {:.0} ms, {} top-level commands, {leaves} leaves, {} clips, {} soft masks",
        ms(interpreted),
        list.command_count(),
        list.clip_count(),
        list.soft_mask_count()
    );

    // The window's fit: the whole page inside the window, as `viewer_core` opens a document.
    let unit = TargetSpec::for_page(&list, 1.0, u64::MAX).expect("a target at one");
    let fit = (window.0 as f32 / unit.width as f32).min(window.1 as f32 / unit.height as f32);
    let fit_page = TargetSpec::for_page(&list, fit, u64::MAX).expect("a fit target");
    println!(
        "window {} × {}; fit {fit:.4} px/unit, the page {} × {} px",
        window.0, window.1, fit_page.width, fit_page.height
    );

    // ── 1. the census ──
    for (name, scale) in [
        ("fit", fit),
        ("150 dpi", 150.0 / 72.0),
        ("4× fit", fit * 4.0),
    ] {
        census(&list, name, scale);
    }

    // ── 2. the processor, one thread ──
    let (whole, whole_raster) = quickest(|| draw(&list, fit_page));
    println!(
        "cpu  fit, whole page, one thread                  {:>9.1} ms",
        ms(whole)
    );

    let large: Vec<Command> = list
        .commands()
        .iter()
        .filter(|command| {
            tight_extent(command, fit_page.transform)
                .is_none_or(|extent| extent.width().max(extent.height()) >= 1.0)
        })
        .cloned()
        .collect();
    let small = list.command_count() - large.len();
    let without = list.shape_list(large);
    let (without_small, _) = quickest(|| draw(&without, fit_page));
    drop(without);
    let saved = whole.saturating_sub(without_small);
    println!(
        "cpu  fit, every mark under a pixel left out       {:>9.1} ms  ({small} marks left out, {:.0} ns each)",
        ms(without_small),
        saved.as_nanos() as f64 / small.max(1) as f64
    );

    let (floor, ink) = quickest(|| composite_floor(&list, fit_page));
    println!(
        "cpu  fit, the floor: each of those marks blended into the pixels it meets {:>9.1} ms  ({:.0} ns each; ink {ink})",
        ms(floor),
        floor.as_nanos() as f64 / small.max(1) as f64
    );

    let zoomed = window_at(&list, fit * 4.0, window);
    let (today, today_raster) = quickest(|| draw(&list, zoomed));
    println!(
        "cpu  4× fit window, the whole list, one thread    {:>9.1} ms",
        ms(today)
    );
    let cull_started = Instant::now();
    let culled = culled_to(&list, zoomed, full(zoomed));
    let cull = cull_started.elapsed();
    let kept = culled.command_count();
    let (culled_time, culled_raster) = quickest(|| draw(&culled, zoomed));
    drop(culled);
    println!(
        "cpu  4× fit window, the culled list, one thread   {:>9.1} ms  ({kept} commands kept; the linear cull {:.0} ms)",
        ms(culled_time),
        ms(cull)
    );
    println!(
        "     the culled window against the whole list's: {} pixels differ",
        differing(&today_raster, &culled_raster)
    );

    // ── 3. the processor, four threads ──
    for (name, target, reference) in [
        ("fit, whole page", fit_page, &whole_raster),
        ("4× fit window", zoomed, &today_raster),
    ] {
        let tiles = quarters(target);
        let lists: Vec<DisplayList> = tiles
            .iter()
            .map(|tile| culled_to(&list, *tile, full(*tile)))
            .collect();
        let (elapsed, drawn) = quickest(|| {
            std::thread::scope(|scope| {
                let handles: Vec<_> = lists
                    .iter()
                    .zip(&tiles)
                    .map(|(tile_list, tile)| scope.spawn(move || draw(tile_list, *tile)))
                    .collect();
                handles
                    .into_iter()
                    .map(|handle| handle.join().expect("a tile thread"))
                    .collect::<Vec<Raster>>()
            })
        });
        let assembled = assemble(target, &tiles, &drawn);
        println!(
            "cpu  {name:<16} four tiles on four threads  {:>9.1} ms  (largest tile {} commands; {} pixels differ from the whole draw)",
            ms(elapsed),
            lists
                .iter()
                .map(DisplayList::command_count)
                .max()
                .unwrap_or(0),
            differing(reference, &assembled)
        );
    }

    // ── 4. the device ──
    if gpu {
        let mut backend = render_raster::QuorraRasterizer::new_headless().expect("an adapter");
        backend.set_coverage(raster_gpu::Coverage::Cpu);
        println!("gpu  {}", backend.adapter_description());
        let whole_list = Arc::new(list.clone());
        println!(
            "gpu  fit, whole page                               {}",
            on_device(&mut backend, &whole_list, fit_page)
        );
        // The step a window takes next: the same list at four times the fit, its scene replacing
        // the fit's — which read the view, so the window's cull applies (ADR 1841) — against the
        // same window drawn from the list `culled_to` makes, pixel for pixel.
        let stepped = drawn_on_device(&mut backend, &whole_list, zoomed);
        let window_list = Arc::new(culled_to(&list, zoomed, full(zoomed)));
        let reference = drawn_on_device(&mut backend, &window_list, zoomed);
        println!(
            "gpu  4× fit window, the whole list, a zoom step from its fit frame  {}; {}",
            stepped.0,
            match (&stepped.1, &reference.1) {
                (Some(a), Some(b)) =>
                    format!("{} pixels differ from the culled list's", differing(a, b)),
                _ => "not compared".to_owned(),
            }
        );
        for (number, tile) in quarters(fit_page).into_iter().enumerate() {
            let tile_list = Arc::new(culled_to(&list, tile, full(tile)));
            println!(
                "gpu  fit, tile {} of four ({} commands)        {}",
                number + 1,
                tile_list.command_count(),
                on_device(&mut backend, &tile_list, tile)
            );
        }
        println!(
            "gpu  4× fit window, culled ({} commands)       {}",
            window_list.command_count(),
            on_device(&mut backend, &window_list, zoomed)
        );
        println!(
            "gpu  4× fit window, the whole list after another list's frame  {}",
            on_device(&mut backend, &whole_list, zoomed)
        );
    }

    // ── 5. the window as the shipped processor draws it ──
    for (name, factor) in [
        ("fit", 1.0),
        ("2× fit", 2.0),
        ("4× fit", 4.0),
        ("8× fit", 8.0),
    ] {
        let target = window_at(&list, fit * factor, window);
        // A copy per magnification, so that the first draw of each measures its bounds afresh
        // as a viewer's first frame at a new zoom does.
        let fresh = list.clone();
        let started = Instant::now();
        let pooled_first = CpuRasterizer::new()
            .rasterize(&fresh, target)
            .expect("the processor draws every command on this page");
        let first = started.elapsed();
        let (pooled, pooled_raster) = quickest(|| {
            CpuRasterizer::new()
                .rasterize(&fresh, target)
                .expect("the processor draws every command on this page")
        });
        let (single, single_raster) = quickest(|| {
            CpuRasterizer::new()
                .with_strips(1)
                .rasterize(&fresh, target)
                .expect("the processor draws every command on this page")
        });
        let planned = planned(&fresh, target);
        let strips = CpuRasterizer::new()
            .strip_plan(&planned, target)
            .len()
            .saturating_sub(1);
        // What a frame at this magnification pays for its cull once the bounds are kept: the
        // test of every command and the list of those that reach the target.
        let (cull, ()) = quickest(|| drop(planned_list(&fresh, target)));
        println!(
            "cpu  {name:<7} window, shipped: first {:>7.1} ms, quickest {:>7.1} ms on {strips} strip(s), {} commands, the cull {:.1} ms; one strip {:>7.1} ms; {} pixels differ, {} against the first; digests {:016x} {:016x}",
            ms(first),
            ms(pooled),
            planned.command_count(),
            ms(cull),
            ms(single),
            differing(&pooled_raster, &single_raster),
            differing(&pooled_raster, &pooled_first),
            digest(&pooled_raster),
            digest(&single_raster)
        );
    }
}

/// The list the shipped rasteriser plans `target`'s strips over: the commands that reach it.
fn planned(list: &DisplayList, target: TargetSpec) -> DisplayList {
    planned_list(list, target).unwrap_or_else(|| list.clone())
}

/// The cull the shipped rasteriser makes of `list` for `target`, as a list of its own, or `None`
/// where it keeps all.
fn planned_list(list: &DisplayList, target: TargetSpec) -> Option<DisplayList> {
    list.reaching(target).map(|window| window.copied())
}

/// A 64-bit FNV-1a digest of a raster's bytes, for comparing two builds' windows by eye.
fn digest(raster: &Raster) -> u64 {
    raster
        .data
        .iter()
        .fold(0xcbf2_9ce4_8422_2325, |hash, byte| {
            (hash ^ u64::from(*byte)).wrapping_mul(0x0000_0100_0000_01b3)
        })
}

/// Milliseconds, for printing.
fn ms(duration: Duration) -> f64 {
    duration.as_secs_f64() * 1000.0
}

/// The quickest of [`DRAWS`] runs of `work`, and the answer of the last.
fn quickest<T>(mut work: impl FnMut() -> T) -> (Duration, T) {
    let mut best = Duration::MAX;
    let mut answer = None;
    for _ in 0..DRAWS {
        let started = Instant::now();
        let value = work();
        best = best.min(started.elapsed());
        answer = Some(value);
    }
    (best, answer.expect("at least one draw"))
}

/// One draw on the processor, the oracle's own rasteriser on the calling thread.
fn draw(list: &DisplayList, target: TargetSpec) -> Raster {
    CpuRasterizer::new()
        .rasterize(list, target)
        .expect("the processor draws every command on this page")
}

/// How many leaves a command list holds, a group counting as its elements.
fn leaf_count(commands: &[Command]) -> usize {
    commands
        .iter()
        .map(|command| match command {
            Command::Group { commands, .. } => leaf_count(commands),
            _ => 1,
        })
        .sum()
}

/// A mark's device extent as a person would measure the ink: see the module comment.
fn tight_extent(command: &Command, to_device: Transform) -> Option<Rect> {
    match command {
        Command::Stroke {
            path,
            transform,
            stroke,
            ..
        } => {
            // The width is in the path's own space, so it grows the hull before the hull is
            // mapped, as `device_bounds` grows it.
            let placed = transform.then(to_device);
            let half = stroke.device_width(placed) / 2.0;
            Some(path.hull()?.grown(half).mapped(placed))
        }
        Command::Group { .. } => None,
        other => other.device_bounds(to_device),
    }
}

/// Prints how many leaves fall in each bin at `scale`, and how many lie inside one pixel.
fn census(list: &DisplayList, name: &str, scale: f32) {
    let target = TargetSpec::for_page(list, scale, u64::MAX).expect("a census target");
    let mut bins = [0usize; BINS.len() + 1];
    let mut unbounded = 0usize;
    let mut inside_one_pixel = 0usize;
    let mut strokes = 0usize;
    let mut walk = |command: &Command| match tight_extent(command, target.transform) {
        None => unbounded += 1,
        Some(extent) => {
            if matches!(command, Command::Stroke { .. }) {
                strokes += 1;
            }
            let side = extent.width().max(extent.height());
            let bin = BINS
                .iter()
                .position(|edge| side < *edge)
                .unwrap_or(BINS.len());
            bins[bin] += 1;
            // The pixel a corner lies in, as an integer, so that "the same pixel" is a
            // comparison of indices rather than of floats.
            let pixel = |value: f32| value.floor() as i64;
            if pixel(extent.min.x) == pixel(extent.max.x)
                && pixel(extent.min.y) == pixel(extent.max.y)
            {
                inside_one_pixel += 1;
            }
        }
    };
    for command in list.commands() {
        walk(command);
    }
    let labels = ["<¼", "<½", "<1", "<2", "<4", "<16", "≥16"];
    let row: Vec<String> = labels
        .iter()
        .zip(bins)
        .map(|(label, count)| format!("{label} {count}"))
        .collect();
    println!(
        "census {name:<8} longer side in px: {}; inside one pixel {inside_one_pixel}; strokes {strokes}; groups or unbounded {unbounded}",
        row.join(", ")
    );
}

/// What no kernel for a mark under a pixel can go below: one source-over per pixel it meets.
///
/// Every mark the census calls smaller than a pixel is blended, at its own share of each pixel
/// its extent meets, into an RGBA buffer the size of the fit page — the arithmetic a kernel that
/// computed §10.7.4's area for free would still owe. The coverage is the extent's area spread
/// evenly, which is not the exact area and is not meant to be: this is the price's floor, not a
/// rasteriser, and its pixels are never compared with anything.
fn composite_floor(list: &DisplayList, target: TargetSpec) -> u64 {
    let (width, height) = (target.width as usize, target.height as usize);
    let mut buffer = vec![1.0f32; width * height * 4];
    for command in list.commands() {
        let Some(extent) = tight_extent(command, target.transform) else {
            continue;
        };
        if extent.width().max(extent.height()) >= 1.0 {
            continue;
        }
        let colour = match command {
            Command::Stroke { .. } | Command::Fill { .. } => [0.4f32, 0.5, 0.6],
            _ => continue,
        };
        let (x0, x1) = (
            extent.min.x.floor().max(0.0) as usize,
            extent.max.x.ceil() as usize,
        );
        let (y0, y1) = (
            extent.min.y.floor().max(0.0) as usize,
            extent.max.y.ceil() as usize,
        );
        let pixels = ((x1 - x0).max(1) * (y1 - y0).max(1)) as f32;
        let alpha = (extent.width() * extent.height() / pixels).min(1.0);
        for y in y0..y1.min(height) {
            for x in x0..x1.min(width) {
                let at = (y * width + x) * 4;
                for (channel, value) in colour.iter().enumerate() {
                    let backdrop = buffer[at + channel];
                    buffer[at + channel] = backdrop + (value - backdrop) * alpha;
                }
            }
        }
    }
    buffer.iter().map(|value| (value * 255.0) as u64).sum()
}

/// The window at `scale`, centred on the page, as `viewer_core` centres a page larger than it.
fn window_at(list: &DisplayList, scale: f32, window: (u32, u32)) -> TargetSpec {
    let page = TargetSpec::for_page(list, scale, u64::MAX).expect("a magnified target");
    let origin = (
        -((page.width.saturating_sub(window.0)) as f32 / 2.0).round(),
        -((page.height.saturating_sub(window.1)) as f32 / 2.0).round(),
    );
    TargetSpec {
        width: window.0,
        height: window.1,
        transform: page
            .transform
            .then(Transform::translate(origin.0, origin.1)),
    }
}

/// The whole of a target, as a rectangle in its own pixels.
fn full(target: TargetSpec) -> Rect {
    Rect::from_corners(
        Point::new(0.0, 0.0),
        Point::new(target.width as f32, target.height as f32),
    )
}

/// The commands of `list` whose conservative extent meets `region` of `target`, in order.
///
/// `device_bounds` is the bound `render_cpu` itself culls rows by, and a group or a command it
/// cannot bound is kept whole — the safe direction, which is why the culled draw must come out
/// identical to the whole one and the run says whether it did.
fn culled_to(list: &DisplayList, target: TargetSpec, region: Rect) -> DisplayList {
    let reach = region.grown(1.0);
    let kept = list
        .commands()
        .iter()
        .filter(|command| {
            command
                .device_bounds(target.transform)
                .is_none_or(|bounds| bounds.intersection(reach).is_some())
        })
        .cloned()
        .collect();
    list.shape_list(kept)
}

/// A target cut into a two-by-two grid, each quarter a target of its own on the same grid.
fn quarters(target: TargetSpec) -> Vec<TargetSpec> {
    let (left, top) = (target.width / 2, target.height / 2);
    let mut tiles = Vec::new();
    for (y, height) in [(0, top), (top, target.height - top)] {
        for (x, width) in [(0, left), (left, target.width - left)] {
            tiles.push(TargetSpec {
                width,
                height,
                transform: target
                    .transform
                    .then(Transform::translate(-(x as f32), -(y as f32))),
            });
        }
    }
    tiles
}

/// The four quarters of [`quarters`] put back into one raster the size of `target`.
fn assemble(target: TargetSpec, tiles: &[TargetSpec], drawn: &[Raster]) -> Raster {
    let mut data = vec![0u8; target.width as usize * target.height as usize * 4];
    let (left, top) = (target.width / 2, target.height / 2);
    let origins = [(0, 0), (left, 0), (0, top), (left, top)];
    for ((tile, raster), (x, y)) in tiles.iter().zip(drawn).zip(origins) {
        for row in 0..tile.height {
            let from = (row * tile.width * 4) as usize;
            let to = (((y + row) * target.width + x) * 4) as usize;
            let length = (tile.width * 4) as usize;
            data[to..to + length].copy_from_slice(&raster.data[from..from + length]);
        }
    }
    Raster {
        width: target.width,
        height: target.height,
        format: pdf_render::RasterFormat::Rgba8,
        data,
    }
}

/// How many pixels of two rasters of one size differ in any channel.
fn differing(a: &Raster, b: &Raster) -> usize {
    a.data
        .chunks_exact(4)
        .zip(b.data.chunks_exact(4))
        .filter(|(x, y)| x != y)
        .count()
}

/// One frame on the device: how long it took, or what it said instead, and what it drew.
fn drawn_on_device(
    backend: &mut render_raster::QuorraRasterizer,
    list: &Arc<DisplayList>,
    target: TargetSpec,
) -> (String, Option<Raster>) {
    let frame = render_raster::PresentFrame {
        width: target.width,
        height: target.height,
        pages: &[(list, target)],
        raster: None,
        overlays: &[],
    };
    let started = Instant::now();
    match backend.rasterize_frame(&frame) {
        Ok(raster) => (
            format!(
                "drawn in {:.1} ms, readback included",
                ms(started.elapsed())
            ),
            Some(raster),
        ),
        Err(problem) => (
            format!("REFUSED after {:.1} ms — {problem}", ms(started.elapsed())),
            None,
        ),
    }
}

/// One frame on the device: how long it took, or what it said instead.
fn on_device(
    backend: &mut render_raster::QuorraRasterizer,
    list: &Arc<DisplayList>,
    target: TargetSpec,
) -> String {
    let frame = render_raster::PresentFrame {
        width: target.width,
        height: target.height,
        pages: &[(list, target)],
        raster: None,
        overlays: &[],
    };
    let started = Instant::now();
    match backend.rasterize_frame(&frame) {
        Ok(_) => format!(
            "drawn in {:.1} ms, readback included",
            ms(started.elapsed())
        ),
        Err(problem) => format!("REFUSED after {:.1} ms — {problem}", ms(started.elapsed())),
    }
}

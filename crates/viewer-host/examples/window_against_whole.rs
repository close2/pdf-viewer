//! What a toolkit window's zoomed page costs drawn whole and drawn as its window, and how the
//! window's pixels compare with the same part of the page drawn whole (ADR 1930).
//!
//! ```text
//! cargo run --release -p viewer-host --example window_against_whole -- document.pdf width height rounds zoom...
//! ```
//!
//! The page one request a window of `width` × `height` device pixels makes, its target magnified
//! by each `zoom` over the fit as notches would, the page centred in the window as a notch about
//! the window's middle leaves it. Whether [`viewer_host::oversize::oversized`] draws it as its
//! window, and, where it does, two arms through [`viewer_host::Drawing`] polled every
//! [`viewer_host::drawing::POLL`], alternated `rounds` times so a neighbour's load lands on both:
//! **whole**, `Drawing::ask`, as a page under the window's extent is drawn; **window**,
//! `Drawing::ask_window` with the window [`viewer_host::oversize::Oversized::due`] asks for.
//! Printed per run: milliseconds from the ask to the landing of each, the bytes each copies into
//! the toolkit at an update, how many pixels of the window differ from the whole page's same
//! rectangle, and by how much, and a digest of each drawing's pixels — which is what says that two
//! builds drew the same window and the same page (ADR 1941).

use std::time::{Duration, Instant};

use pdf_render::{Raster, TargetSpec, Transform};
use viewer_core::{Command, DocumentId, Event, PageGeometry, RenderRequest, Rendered, Viewer};
use viewer_host::drawing::{Drawing, POLL};
use viewer_host::oversize::{Oversized, Window, largest_window, oversized};

#[expect(
    clippy::expect_used,
    reason = "a measurement tool should stop loudly where its document is missing"
)]
fn first_request(path: &str, width: u32, height: u32) -> RenderRequest {
    let bytes = pdf_syntax::FileBytes::on_disk(std::path::Path::new(path)).expect("readable");
    let mut viewer = Viewer::new(0, 0, 1.0);
    viewer
        .handle(Command::Open {
            id: DocumentId(0),
            bytes,
            password: None,
            fragment: None,
        })
        .for_each(drop);
    viewer.anticipate().for_each(drop);
    viewer
        .handle(Command::Resize {
            width,
            height,
            scale: 1.0,
        })
        .find_map(|event| match event {
            Event::NeedsRender(request) => Some(request),
            _ => None,
        })
        .expect("a viewport asks for page one")
}

/// One draw through the drawing thread, of the whole target or of `window`: when it landed, and
/// its pixels.
#[expect(
    clippy::panic,
    reason = "a measurement tool should stop loudly where its frame did not land"
)]
fn drawn(request: &RenderRequest, window: Option<Window>) -> (Duration, Raster) {
    let mut drawing: Drawing = Drawing::new();
    let began = Instant::now();
    match window {
        Some(window) => drawing.ask_window(request.clone(), window),
        None => drawing.ask(request.clone()),
    }
    loop {
        let mut landed: Vec<_> = drawing.collect();
        landed.extend(drawing.windows().into_iter().map(|(finished, _)| finished));
        if let Some(done) = landed.into_iter().next() {
            let framed = began.elapsed();
            let Some(Rendered::Raster(raster)) = done.outcome else {
                panic!("the page did not land as a raster");
            };
            return (framed, raster);
        }
        std::thread::sleep(POLL);
    }
}

/// How many pixels of `window`'s raster differ from the same rectangle of `whole`, the largest
/// difference of one channel, and the rectangle of the window the differing pixels lie in.
fn compared(whole: &Raster, window: Window, drawn: &Raster) -> (usize, u8, Option<[usize; 4]>) {
    let (mut differing, mut worst) = (0_usize, 0_u8);
    let mut extent: Option<[usize; 4]> = None;
    let stride = usize::try_from(whole.width)
        .unwrap_or(usize::MAX)
        .saturating_mul(4);
    let row_bytes = usize::try_from(drawn.width)
        .unwrap_or(usize::MAX)
        .saturating_mul(4);
    for (row, drawn_row) in drawn.data.chunks_exact(row_bytes.max(1)).enumerate() {
        let from = usize::try_from(window.y)
            .unwrap_or(usize::MAX)
            .saturating_add(row)
            .saturating_mul(stride)
            .saturating_add(
                usize::try_from(window.x)
                    .unwrap_or(usize::MAX)
                    .saturating_mul(4),
            );
        let Some(whole_row) = whole.data.get(from..from.saturating_add(row_bytes)) else {
            continue;
        };
        for (column, (a, b)) in whole_row
            .chunks_exact(4)
            .zip(drawn_row.chunks_exact(4))
            .enumerate()
        {
            if a != b {
                differing = differing.saturating_add(1);
                let [x0, y0, x1, y1] = extent.unwrap_or([column, row, column, row]);
                extent = Some([x0.min(column), y0.min(row), x1.max(column), y1.max(row)]);
                worst = a
                    .iter()
                    .zip(b)
                    .map(|(a, b)| a.abs_diff(*b))
                    .fold(worst, u8::max);
            }
        }
    }
    (differing, worst, extent)
}

/// `raster`'s rectangle `[x0, y0, x1, y1]`, inclusive, as a binary PPM at `path` — for looking at
/// where two drawings differ (trap 1).
fn dump(raster: &Raster, [x0, y0, x1, y1]: [usize; 4], path: &std::path::Path) {
    let width = usize::try_from(raster.width).unwrap_or(usize::MAX);
    let mut bytes = format!(
        "P6\n{} {}\n255\n",
        x1.saturating_sub(x0).saturating_add(1),
        y1.saturating_sub(y0).saturating_add(1)
    )
    .into_bytes();
    for row in y0..=y1 {
        for column in x0..=x1 {
            let at = row
                .saturating_mul(width)
                .saturating_add(column)
                .saturating_mul(4);
            bytes.extend(
                raster
                    .data
                    .get(at..at.saturating_add(3))
                    .unwrap_or(&[0, 0, 0]),
            );
        }
    }
    if let Err(problem) = std::fs::write(path, bytes) {
        eprintln!("{}: {problem}", path.display());
    }
}

/// Where the page is not drawn as its window, the window it would be drawn as were the bound
/// lower — the largest window, centred and clamped to the page — so that the price of a window
/// that is most of its page is measured too (ADR 1941).
fn largest_centred(target: TargetSpec, viewport: (u32, u32)) -> Window {
    let (most_x, most_y) = largest_window(viewport);
    let (across, down) = (most_x.min(target.width), most_y.min(target.height));
    Window {
        x: target.width.saturating_sub(across) / 2,
        y: target.height.saturating_sub(down) / 2,
        width: across,
        height: down,
    }
}

/// A digest of a drawing's pixels, to compare two builds' drawings by.
fn digest(raster: &Raster) -> u64 {
    use std::hash::{Hash as _, Hasher as _};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    raster.data.hash(&mut hasher);
    hasher.finish()
}

/// Where the two drawings differ, both of them of that rectangle, for a person to look at — where
/// `WINDOW_AGAINST_WHOLE_DUMP` names a directory to write them into.
fn dump_both(whole: &Raster, window: Window, drawn: &Raster, extent: [usize; 4], zoom: f32) {
    let Some(directory) = std::env::var_os("WINDOW_AGAINST_WHOLE_DUMP") else {
        return;
    };
    let [x0, y0, x1, y1] = extent;
    let x = usize::try_from(window.x).unwrap_or(usize::MAX);
    let y = usize::try_from(window.y).unwrap_or(usize::MAX);
    let directory = std::path::Path::new(&directory);
    dump(
        whole,
        [
            x0.saturating_add(x),
            y0.saturating_add(y),
            x1.saturating_add(x),
            y1.saturating_add(y),
        ],
        &directory.join(format!("{zoom}-whole.ppm")),
    );
    dump(drawn, extent, &directory.join(format!("{zoom}-window.ppm")));
    println!("  differing within the window's {extent:?}");
}

fn millis(duration: Duration) -> String {
    format!("{:.0}", duration.as_secs_f64() * 1000.0)
}

#[expect(
    clippy::expect_used,
    reason = "a measurement tool should stop loudly where its arguments are wrong"
)]
fn main() {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let [path, width, height, rounds, zooms @ ..] = arguments.as_slice() else {
        eprintln!("usage: window_against_whole document.pdf width height rounds zoom...");
        std::process::exit(2);
    };
    let (width, height): (u32, u32) = (
        width.parse().expect("a width"),
        height.parse().expect("a height"),
    );
    let rounds: usize = rounds.parse().expect("a count of rounds");
    let fit = first_request(path, width, height);
    println!(
        "RAYON_NUM_THREADS {:?}; fit {}x{}; window {width}x{height}",
        std::env::var("RAYON_NUM_THREADS").ok(),
        fit.target.width,
        fit.target.height
    );
    for zoom in zooms {
        let zoom: f32 = zoom.parse().expect("a magnification");
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            clippy::cast_precision_loss,
            reason = "a page of a few thousand pixels, magnified a few times"
        )]
        let target = TargetSpec {
            width: (fit.target.width as f32 * zoom).ceil() as u32,
            height: (fit.target.height as f32 * zoom).ceil() as u32,
            transform: fit.target.transform.then(Transform::scale(zoom, zoom)),
        };
        let request = RenderRequest {
            target,
            ..fit.clone()
        };
        // The page centred in the window along both sides, as a notch at the window's centre
        // leaves a page that started centred.
        #[expect(
            clippy::cast_precision_loss,
            reason = "a page of a few thousand pixels"
        )]
        let geometry = PageGeometry {
            page: pdf_render::Size::new(target.width as f32, target.height as f32),
            scale: zoom,
            width: target.width,
            height: target.height,
            origin: (
                (width as f32 - target.width as f32) / 2.0,
                (height as f32 - target.height as f32) / 2.0,
            ),
        };
        let mut held = Oversized::default();
        let windowed = held.asked(&request, (width, height));
        let due = held.due(request.document, |_| Some(geometry), (width, height));
        let window = due.first().map_or_else(
            || largest_centred(target, (width, height)),
            |(_, window)| *window,
        );
        let share = (f64::from(window.width) * f64::from(window.height))
            / (f64::from(target.width) * f64::from(target.height));
        println!(
            "zoom {zoom}: {}x{}, oversized {}, window {window:?}, {:.0} % of the page",
            target.width,
            target.height,
            oversized(&request, (width, height)) && windowed,
            share * 100.0
        );
        for round in 0..rounds {
            let (whole, whole_page) = drawn(&request, None);
            let (windowed, window_page) = drawn(&request, Some(window));
            let (differing, worst, extent) = compared(&whole_page, window, &window_page);
            if let Some(extent) = extent {
                dump_both(&whole_page, window, &window_page, extent, zoom);
            }
            println!(
                "  round {round}: whole landed {} ms, {} bytes; window landed {} ms, {} bytes; \
                 {differing} of the window's pixels differ from the page's, by at most {worst}; \
                 digests {:016x} {:016x}",
                millis(whole),
                whole_page.data.len(),
                millis(windowed),
                window_page.data.len(),
                digest(&whole_page),
                digest(&window_page)
            );
        }
    }
}

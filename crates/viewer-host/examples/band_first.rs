//! What a toolkit window's zoomed page costs drawn whole and drawn band first (ADR 1868).
//!
//! ```text
//! cargo run --release -p viewer-host --example band_first -- document.pdf width height rounds zoom...
//! ```
//!
//! The page one request a window of `width` × `height` device pixels makes, its target magnified
//! by each `zoom` over the fit as a notch would, the window centred on the page. Two arms through
//! [`viewer_host::Drawing`] polled every [`viewer_host::drawing::POLL`], as a window polls it,
//! alternated `rounds` times so a neighbour's load lands on both: **whole**, `Drawing::ask`, as
//! HEAD's windows ask; **band**, `Drawing::ask_showing` with the rows the window shows. Printed per
//! run, milliseconds from the ask: when the rows the window shows were first on it finished
//! (`seen`; the whole arm's is its landing unless a glimpse finished them first), and when the
//! page landed. Beside them, what the planner costs alone — the band's edges are its cuts — and how
//! many pixels the two arms' pages differ in, and by how much.

use std::time::{Duration, Instant};

use pdf_render::{Raster, TargetSpec, Transform};
use viewer_core::{Command, DocumentId, Event, PageGeometry, RenderRequest, Rendered, Viewer};
use viewer_host::drawing::{Drawing, POLL};

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

/// One draw through the drawing thread: when the rows shown were finished, when it landed, and
/// the page.
#[expect(
    clippy::panic,
    reason = "a measurement tool should stop loudly where its frame did not land"
)]
fn drawn(
    request: &RenderRequest,
    seen: Option<std::ops::Range<u32>>,
) -> (Duration, Duration, Raster) {
    let mut drawing: Drawing = Drawing::new();
    let began = Instant::now();
    match seen {
        Some(seen) => drawing.ask_showing(request.clone(), Some(seen)),
        None => drawing.ask(request.clone()),
    }
    let mut finished = None;
    loop {
        let landed = drawing.collect();
        if finished.is_none()
            && let Some((_, after)) = drawing.seen_drawn()
        {
            finished = Some(after);
        }
        if let Some(done) = landed.into_iter().next() {
            let framed = began.elapsed();
            let Some(Rendered::Raster(raster)) = done.outcome else {
                panic!("the page did not land as a raster");
            };
            return (finished.unwrap_or(framed), framed, raster);
        }
        std::thread::sleep(POLL);
    }
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
        eprintln!("usage: band_first document.pdf width height rounds zoom...");
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
        // The window centred on the page, as a notch at the window's centre leaves it.
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
                0.0,
                -((target.height as f32 - height as f32) / 2.0).max(0.0),
            ),
        };
        let seen = viewer_host::drawing::seen(target, geometry, height);
        let planned = Instant::now();
        let plan = render_cpu::CpuRasterizer::new()
            .with_strips(16)
            .strip_plan(&request.list, target);
        let planning = planned.elapsed();
        println!(
            "zoom {zoom}: {}x{}, rows shown {seen:?}, plan {} ms over {} cuts",
            target.width,
            target.height,
            millis(planning),
            plan.len()
        );
        for round in 0..rounds {
            let (whole_seen, whole, whole_page) = drawn(&request, None);
            let (band_seen, band, band_page) = drawn(&request, seen.clone());
            let differing = whole_page
                .data
                .chunks_exact(4)
                .zip(band_page.data.chunks_exact(4))
                .filter(|(a, b)| a != b)
                .count();
            let worst = whole_page
                .data
                .iter()
                .zip(&band_page.data)
                .map(|(a, b)| a.abs_diff(*b))
                .max()
                .unwrap_or(0);
            println!(
                "  round {round}: whole seen {} landed {} ms; band seen {} landed {} ms; \
                 {differing} pixels differ, by at most {worst}",
                millis(whole_seen),
                millis(whole),
                millis(band_seen),
                millis(band)
            );
        }
    }
}

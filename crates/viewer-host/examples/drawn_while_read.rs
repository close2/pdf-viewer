//! When page one is first seen and when its first frame lands, read and then drawn against drawn
//! while it is read (ADR 1855).
//!
//! ```text
//! cargo run --release -p viewer-host --example drawn_while_read -- document.pdf [width] [height] [rounds]
//! ```
//!
//! A window's launch path without the window: the document opened and page one read on a
//! [`viewer_host::Loading`] thread, the window's size told to the core once the thread has ended,
//! the request that follows drawn through [`viewer_host::Drawing`] polled every
//! [`viewer_host::drawing::POLL`] as a window polls it. Two arms, alternated `rounds` times so a
//! neighbour's build lands on both: **before**, the opening thread calls `Viewer::anticipate` as
//! every window does today; **after**, it calls [`viewer_host::Ahead::anticipate`], which reads
//! the page on a thread of its own and draws it on the opening thread as it is read, at the target
//! the window's size foretells, and the drawing thread finishes it. Printed per run, in seconds
//! from the thread's start: when the read ended, when the page was first seen (the first look:
//! the drawing thread's first glimpse before, the first part drawn ahead after), and when the first
//! frame landed. The two arms' frames are compared byte for byte with the page drawn whole as one
//! strip, which is the oracle's own drawing of it.

use std::time::{Duration, Instant};

use pdf_render::Rasterizer as _;
use viewer_core::{Command, DocumentId, Event, Rendered, Viewer};
use viewer_host::drawing::{Drawing, POLL};
use viewer_host::loading::Window;
use viewer_host::{Ahead, Loading};

/// One launch, as a window would make it.
struct Launch {
    read: Duration,
    seen: Option<Duration>,
    framed: Duration,
    frame: pdf_render::Raster,
    list: std::sync::Arc<pdf_render::DisplayList>,
    target: pdf_render::TargetSpec,
}

#[expect(
    clippy::expect_used,
    reason = "a measurement tool should stop loudly where its document is missing"
)]
fn open(path: &str) -> (Viewer, Vec<Event>) {
    let bytes = pdf_syntax::FileBytes::on_disk(std::path::Path::new(path)).expect("readable");
    let mut viewer = Viewer::new(0, 0, 1.0);
    let events = viewer.handle(Command::Open {
        id: DocumentId(0),
        bytes,
        password: None,
        fragment: None,
    });
    (viewer, events.collect())
}

#[expect(
    clippy::expect_used,
    clippy::panic,
    reason = "a measurement tool should stop loudly where its document is missing or its frame \
              did not land"
)]
fn launch(path: &str, window: Window, ahead: bool) -> Launch {
    let began = Instant::now();
    let drawn_ahead = ahead.then(Ahead::page_sized);
    if let Some(drawn_ahead) = &drawn_ahead {
        drawn_ahead.window_is(window);
    }
    let loading = Loading::start({
        let path = path.to_owned();
        let drawn_ahead = drawn_ahead.clone();
        move || {
            let (mut viewer, mut events) = open(&path);
            match &drawn_ahead {
                Some(drawn_ahead) => events.extend(drawn_ahead.anticipate(&mut viewer)),
                None => events.extend(viewer.anticipate()),
            }
            (viewer, events)
        }
    });
    let mut seen = None;
    while !loading.ended() {
        if seen.is_none()
            && drawn_ahead
                .as_ref()
                .is_some_and(|drawn_ahead| drawn_ahead.look().is_some())
        {
            seen = Some(began.elapsed());
        }
        std::thread::sleep(viewer_host::loading::TICK);
    }
    let (mut viewer, _) = loading.join().expect("the opening thread");
    let read = began.elapsed();
    let (width, height) = window.viewport();
    let mut drawing = Drawing::new();
    if let Some(drawn_ahead) = drawn_ahead {
        drawing.draw_ahead(drawn_ahead);
    }
    for event in viewer.handle(Command::Resize {
        width,
        height,
        scale: window.scale,
    }) {
        if let Event::NeedsRender(request) = event {
            drawing.ask(request);
        }
    }
    loop {
        let finished = drawing.collect();
        if seen.is_none() && drawing.glimpsed() {
            seen = Some(began.elapsed());
        }
        if let Some(finished) = finished.into_iter().next() {
            let Some(Rendered::Raster(frame)) = finished.outcome else {
                panic!("the first frame did not land as a raster");
            };
            return Launch {
                read,
                seen,
                framed: began.elapsed(),
                frame,
                list: finished.request.list,
                target: finished.request.target,
            };
        }
        std::thread::sleep(POLL);
    }
}

#[expect(
    clippy::expect_used,
    clippy::print_stdout,
    reason = "a measurement tool: it stops loudly on a missing input and prints its answer"
)]
fn main() {
    let mut args = std::env::args().skip(1);
    let path = args
        .next()
        .expect("usage: drawn_while_read document.pdf [width] [height] [rounds]");
    let width = args.next().map_or(1416, |n| n.parse().expect("a width"));
    let height = args.next().map_or(1000, |n| n.parse().expect("a height"));
    let rounds = args.next().map_or(2, |n| n.parse().expect("a count"));
    let window = Window {
        width,
        height,
        inset: 0,
        scale: 1.0,
    };
    let seconds = |at: Duration| at.as_secs_f64();
    for round in 0..rounds {
        for ahead in [false, true] {
            let launched = launch(&path, window, ahead);
            let whole = render_cpu::CpuRasterizer::new()
                .with_strips(1)
                .rasterize(&launched.list, launched.target)
                .expect("the page draws whole");
            println!(
                "round {round} {:6}: read {:6.3} s, first seen {}, first frame {:6.3} s \
                 ({}x{}, {} commands; the frame {} the page drawn whole as one strip)",
                if ahead { "after" } else { "before" },
                seconds(launched.read),
                launched
                    .seen
                    .map_or("never".to_owned(), |at| format!("{:6.3} s", seconds(at))),
                seconds(launched.framed),
                launched.target.width,
                launched.target.height,
                launched.list.command_count(),
                if launched.frame.data == whole.data {
                    "is"
                } else {
                    "is NOT"
                },
            );
        }
    }
}

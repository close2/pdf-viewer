//! When a page that draws for seconds is first seen, and what being looked at costs its draw.
//!
//! ```text
//! cargo run --release -p viewer-host --example page_as_built -- document.pdf [scale] [rounds]
//! ```
//!
//! Interprets page one once, then draws it `rounds` times in alternation — once unobserved, as a
//! window drew before ADR 1825, and once through `viewer_host::Drawing` polled every
//! `viewer_host::drawing::POLL` the way a window polls it, so that it is glanced at after
//! `GLIMPSE_AFTER` and every `GLIMPSE_EVERY` — and prints, for the second, when the first picture
//! of the page arrived and how many did, and for both how long the whole page took. The two
//! rasters are compared byte for byte, because a look that changed the page would be a defect and
//! not a cost. Alternating rather than one arm after the other, so that a neighbour's build lands
//! on both.

use std::sync::Arc;
use std::time::{Duration, Instant};

use pdf_render::{DisplayList, Rasterizer as _, TargetSpec};
use viewer_host::drawing::{DrawRequest, Drawing, POLL};

/// The page, at the target the example draws it at.
struct Page {
    list: Arc<DisplayList>,
    target: TargetSpec,
}

impl DrawRequest for Page {
    fn page(&self) -> usize {
        0
    }

    fn list(&self) -> &Arc<DisplayList> {
        &self.list
    }

    fn target(&self) -> TargetSpec {
        self.target
    }
}

#[expect(
    clippy::expect_used,
    clippy::panic,
    reason = "a measurement tool should stop loudly where its document is missing, or where the \
              draw it measures did not land"
)]
fn main() {
    let mut args = std::env::args().skip(1);
    let path = args
        .next()
        .expect("usage: page_as_built document.pdf [scale] [rounds]");
    let scale = args
        .next()
        .map_or(1.0, |n| n.parse::<f32>().expect("a scale"));
    let rounds = args
        .next()
        .map_or(2, |n| n.parse::<usize>().expect("a count of rounds"));

    let bytes = pdf_syntax::FileBytes::on_disk(std::path::Path::new(&path)).expect("readable");
    let document = pdf_syntax::Document::open(bytes).expect("a PDF");
    let page = pdf_model::Pages::new(&document)
        .get(0)
        .expect("the page exists");
    let began = Instant::now();
    let list = Arc::new(pdf_model::interpret(&document, &page).display_list);
    let target = TargetSpec::for_page(&list, scale, 1 << 31).expect("a target");
    println!(
        "interpreted in {:.3} s: {} top-level commands, {}x{} at scale {scale}",
        began.elapsed().as_secs_f64(),
        list.commands().len(),
        target.width,
        target.height
    );

    let (mut bare, mut looked) = (Duration::ZERO, Duration::ZERO);
    for round in 0..rounds {
        let began = Instant::now();
        let unobserved = render_cpu::CpuRasterizer::new()
            .rasterize(&list, target)
            .expect("the page draws");
        let took = began.elapsed();
        bare = bare.saturating_add(took);

        let mut drawing = Drawing::new();
        let began = Instant::now();
        drawing.ask(Page {
            list: Arc::clone(&list),
            target,
        });
        let mut glimpses = Vec::new();
        let finished = loop {
            let finished = drawing.collect();
            if drawing.glimpsed() {
                let fraction = drawing
                    .partial()
                    .map_or(0.0, viewer_host::Partial::fraction);
                glimpses.push((began.elapsed(), fraction));
            }
            if let Some(finished) = finished.into_iter().next() {
                break finished;
            }
            std::thread::sleep(POLL);
        };
        let took_looked = began.elapsed();
        looked = looked.saturating_add(took_looked);
        let Some(viewer_core::Rendered::Raster(observed)) = finished.outcome else {
            panic!("the observed draw did not land as a raster");
        };
        assert!(
            observed.data == unobserved.data,
            "being looked at changed the page"
        );
        println!(
            "round {round}: unobserved {:.3} s; looked at {:.3} s, first at {}, {} pictures",
            took.as_secs_f64(),
            took_looked.as_secs_f64(),
            glimpses
                .first()
                .map_or("never".to_owned(), |(at, _)| format!(
                    "{:.3} s",
                    at.as_secs_f64()
                )),
            glimpses.len()
        );
        if round == 0 {
            let step = (glimpses.len() / 8).max(1);
            for (at, fraction) in glimpses.iter().step_by(step) {
                println!(
                    "  at {:6.3} s: {:5.1} % drawn",
                    at.as_secs_f64(),
                    fraction * 100.0
                );
            }
        }
    }
    let share = (looked.as_secs_f64() - bare.as_secs_f64()) / bare.as_secs_f64() * 100.0;
    println!(
        "total: unobserved {:.3} s, looked at {:.3} s, difference {share:+.2} %",
        bare.as_secs_f64(),
        looked.as_secs_f64()
    );
}

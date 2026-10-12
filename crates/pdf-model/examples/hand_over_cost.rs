//! What handing a page's display list over in parts costs the read, and what drawing the parts
//! beside it costs (ADR 1855).
//!
//! ```text
//! taskset -c 0-3,12-15 cargo run --release -p pdf-model --example hand_over_cost -- document.pdf [rounds]
//! ```
//!
//! Interprets page one `rounds` times in each of three arms, alternated so that a neighbour's build
//! lands on all of them: **bare**, as every caller but the window's launch reads; **handed**, with a
//! `pdf_model::progress::Parts` claimed by the read and a second thread taking every part and
//! dropping it — the copy and the hand-off and nothing else; and **drawn**, the second thread
//! painting every part into one page-sized target with `render_cpu::Underway`, as the window does.
//! Printed per arm: each read's wall time and the minimum. Pin the run to one class of core, as
//! `doc/habits/measuring.md` says: the read is worth half again on this machine's slower cores.

use std::time::{Duration, Instant};

use pdf_model::progress::{Handed, Parts};

#[expect(
    clippy::expect_used,
    clippy::print_stdout,
    reason = "a measurement tool: it stops loudly on a missing input and prints its answer"
)]
fn main() {
    let mut args = std::env::args().skip(1);
    let path = args
        .next()
        .expect("usage: hand_over_cost document.pdf [rounds]");
    let rounds = args
        .next()
        .map_or(3, |n| n.parse::<usize>().expect("a count"));
    let bytes = pdf_syntax::FileBytes::on_disk(std::path::Path::new(&path)).expect("readable");
    let document = pdf_syntax::Document::open(bytes).expect("a PDF");
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    let size = pdf_model::content::displayed_size(&page);
    let target =
        pdf_render::TargetSpec::for_page(&pdf_render::DisplayList::new(size), 1.0, 1 << 31)
            .expect("a target");
    let mut taken: [Vec<Duration>; 3] = [Vec::new(), Vec::new(), Vec::new()];
    for _ in 0..rounds {
        for (arm, times) in taken.iter_mut().enumerate() {
            let parts = Parts::new();
            let read = std::thread::scope(|scope| {
                let drawer = (arm > 0).then(|| {
                    let parts = parts.clone();
                    scope.spawn(move || {
                        let mut underway = (arm == 2)
                            .then(|| {
                                render_cpu::Underway::new(&render_cpu::CpuRasterizer::new(), target)
                            })
                            .flatten();
                        while let Some(Handed::Part(part)) = parts.take(Duration::from_mins(1)) {
                            if let Some(underway) = underway.as_mut() {
                                underway.take(part);
                            }
                        }
                    })
                });
                let began = Instant::now();
                let commands = if arm == 0 {
                    pdf_model::interpret(&document, &page)
                        .display_list
                        .command_count()
                } else {
                    parts
                        .during(|| pdf_model::interpret(&document, &page))
                        .display_list
                        .command_count()
                };
                let read = began.elapsed();
                if let Some(drawer) = drawer {
                    drawer.join().expect("the drawer");
                }
                assert!(commands > 0, "the page states commands");
                read
            });
            times.push(read);
        }
    }
    for (name, times) in ["bare", "handed", "drawn"].iter().zip(&taken) {
        let shown: Vec<String> = times
            .iter()
            .map(|time| format!("{:.3}", time.as_secs_f64()))
            .collect();
        let least = times.iter().min().copied().unwrap_or_default();
        println!(
            "{name:7} read {} s; minimum {:.3} s",
            shown.join(", "),
            least.as_secs_f64()
        );
    }
}

//! What `Query::AccessibilityTree` costs, on the document whose size makes it a question.
//!
//! A screen reader asks this when it attaches and again on **every page turn**, so its cost is a
//! latency a person waits through rather than a throughput. `doc/todo/31` records it at 67–91 ms
//! on ISO 32000-2's 1023 pages against 0.13–0.25 ms on a five-page document, because
//! `viewer_core::accessibility::nodes` walks the whole document's structure tree and prunes
//! afterwards — and that number was measured with a stopwatch nobody could rerun. This is the
//! stopwatch.
//!
//! It prints the best of `repeats` for the page it opens on and for one further page, because
//! the second is what a page *turn* costs and the first includes nothing else.
//!
//! ```sh
//! cargo run --release -p viewer-core --example accessibility_cost -- file.pdf [page] [repeats]
//! ```
//!
//! **Best of, not mean**: the measurement is of the work, and the slow runs of a warm loop are
//! this machine's other processes rather than this program's. `doc/habits.md`'s *Measuring* is
//! the rest of the rule, including that an A/B is worth more than an absolute number.

#![expect(
    clippy::print_stdout,
    reason = "an example whose entire output is the measurement"
)]

use std::time::{Duration, Instant};

use viewer_core::{Answer, Command, DocumentId, Event, PageTarget, Query, Viewer};

/// The document this run measures.
const DOCUMENT: DocumentId = DocumentId(1);

fn main() {
    let mut arguments = std::env::args().skip(1);
    let Some(path) = arguments.next() else {
        println!("usage: accessibility_cost <file.pdf> [page] [repeats]");
        return;
    };
    let page: usize = arguments
        .next()
        .and_then(|index| index.parse().ok())
        .unwrap_or(0);
    let repeats: usize = arguments
        .next()
        .and_then(|count| count.parse().ok())
        .unwrap_or(5);
    let Ok(bytes) = std::fs::read(&path) else {
        println!("cannot read {path}");
        return;
    };

    let mut viewer = Viewer::new(1100, 1200, 1.0);
    let events: Vec<Event> = viewer
        .handle(Command::Open {
            id: DOCUMENT,
            bytes,
            password: None,
            fragment: None,
        })
        .collect();
    if !events
        .iter()
        .any(|event| matches!(event, Event::Opened { .. }))
    {
        println!("{path} did not open: {events:?}");
        return;
    }

    let measured = ask(&viewer, repeats);
    println!("{path}: the page it opens on — {measured}");

    if page > 0 {
        viewer
            .handle(Command::GoTo(PageTarget::Index(page)))
            .for_each(drop);
        let measured = ask(&viewer, repeats);
        println!("  page {page} — {measured}");
    }
}

/// One run's best time, and what the answer it produced holds.
struct Measured {
    /// The best of the repeats.
    best: Duration,
    /// How many nodes came back.
    nodes: usize,
    /// How many carry Table 379's `/BBox`.
    bounded: usize,
    /// How many carry §14.8.4.8.3's header cells, and how many associations in all.
    headed: (usize, usize),
}

impl std::fmt::Display for Measured {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            out,
            "{:.3?}, {} node(s), {} with bounds, {} with headers ({} associations)",
            self.best, self.nodes, self.bounded, self.headed.0, self.headed.1
        )
    }
}

/// Asks the question `repeats` times, and answers with the best time and what came back.
///
/// The counts beside the time are what it bought: nodes carrying Table 379's `/BBox`, and cells
/// carrying §14.8.4.8.3's headers. Both cost work per *element* whatever the document states —
/// one `Tree::attribute` for the first, a walk of the table's grid for the second — so the count
/// of elements they answer for is the other half of the measurement.
fn ask(viewer: &Viewer, repeats: usize) -> Measured {
    let mut measured = Measured {
        best: Duration::MAX,
        nodes: 0,
        bounded: 0,
        headed: (0, 0),
    };
    for _ in 0..repeats.max(1) {
        let started = Instant::now();
        let answer = viewer.query(Query::AccessibilityTree);
        let elapsed = started.elapsed();
        measured.best = measured.best.min(elapsed);
        if let Answer::Accessibility(tree) = answer {
            measured.nodes = tree.len();
            measured.bounded = tree.iter().filter(|node| node.bounds.is_some()).count();
            measured.headed = (
                tree.iter().filter(|node| !node.headers.is_empty()).count(),
                tree.iter().map(|node| node.headers.len()).sum(),
            );
        }
    }
    measured
}

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

    let (best, nodes, bounded) = ask(&viewer, repeats);
    println!("{path}: the page it opens on — {best:.3?}, {nodes} node(s), {bounded} with bounds");

    if page > 0 {
        viewer
            .handle(Command::GoTo(PageTarget::Index(page)))
            .for_each(drop);
        let (best, nodes, bounded) = ask(&viewer, repeats);
        println!("  page {page} — {best:.3?}, {nodes} node(s), {bounded} with bounds");
    }
}

/// Asks the question `repeats` times, and answers with the best time and what came back.
///
/// The count of nodes carrying Table 379's `/BBox` is printed beside the time because the two
/// belong together: reading the attribute costs one `Tree::attribute` per element whatever the
/// document states, and how many elements it *answers* for is what that buys.
fn ask(viewer: &Viewer, repeats: usize) -> (Duration, usize, usize) {
    let mut best = Duration::MAX;
    let mut nodes = 0_usize;
    let mut bounded = 0_usize;
    for _ in 0..repeats.max(1) {
        let started = Instant::now();
        let answer = viewer.query(Query::AccessibilityTree);
        let elapsed = started.elapsed();
        best = best.min(elapsed);
        if let Answer::Accessibility(tree) = answer {
            nodes = tree.len();
            bounded = tree.iter().filter(|node| node.bounds.is_some()).count();
        }
    }
    (best, nodes, bounded)
}

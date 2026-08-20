//! What `Query::AccessibilityTree` costs, on the document whose size makes it a question.
//!
//! A screen reader asks this when it attaches and again on **every page turn**, so its cost is a
//! latency a person waits through rather than a throughput. ADR 0228 measured it by hand and left
//! nothing anybody could rerun; this is the stopwatch, and ADRs 0325 and 0394 are the two rounds
//! that used it to take the cost down on the document whose size makes it a question.
//!
//! **A stopwatch is the wrong instrument for a small change on a busy machine** (ADR 0312), so an
//! A/B belongs under `valgrind --tool=callgrind --collect-atstart=no
//! "--toggle-collect=*Viewer*::query*"`, which counts only the query and is load-independent. Run
//! it with one repeat and with three: the difference over two is what a *warm* page turn costs,
//! and the single run is what a cold one does.
//!
//! It prints the best of `repeats` for the page it opens on and for one further page, because
//! the second is what a page *turn* costs and the first includes nothing else.
//!
//! ```sh
//! cargo run --release -p viewer-core --example accessibility_cost -- file.pdf [page] [repeats] [column]
//! ```
//!
//! **The fourth argument is Table 29's arrangement**, and it is the question the six-hundred-and-
//! tenth session added: `column` puts `OneColumn` at half magnification, so several pages are on
//! the screen and the answer is several pages'. A screen reader asks this question of the screen
//! rather than of a page, so what a column costs is what a person waits through — and the run
//! prints how many pages it was answering for beside the time.
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
    let column = arguments.next().is_some_and(|word| word == "column");
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

    if column {
        // Half magnification first, because at `Zoom::FitPage` a column has one page on the
        // screen and would measure the arrangement this run is comparing against.
        viewer
            .handle(Command::Zoom {
                zoom: viewer_core::Zoom::Scale(0.5),
                at: None,
            })
            .for_each(drop);
        viewer
            .handle(Command::Layout(
                pdf_model::viewer_preferences::PageLayout::OneColumn,
            ))
            .for_each(drop);
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
    /// How many lines a caret could move through, and how many characters on them.
    ///
    /// Printed beside the time for the same reason the other two are: the lines are built per
    /// element out of the page's text layer, so what they cost is proportional to them.
    lined: (usize, usize),
    /// How many pages Table 29's arrangement was showing when the question was asked.
    ///
    /// The denominator of everything else here since the six-hundred-and-tenth session: the
    /// question answers for the screen rather than for a page, so a column's cost is several
    /// pages' and this is how many.
    pages: usize,
    /// The longest line the page answered with, which is what says the grouping is working.
    ///
    /// A page of prose whose longest line is three characters has grouped nothing, and the count
    /// of lines alone cannot tell that from a page of short captions. Quoted rather than counted
    /// for the same reason: a number cannot show a line broken in the middle of a word.
    longest: String,
}

impl std::fmt::Display for Measured {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            out,
            "{:.3?}, {} page(s) on screen, {} node(s), {} with bounds, {} with headers \
             ({} associations), {} line(s) of {} character(s), longest {:?}",
            self.best,
            self.pages,
            self.nodes,
            self.bounded,
            self.headed.0,
            self.headed.1,
            self.lined.0,
            self.lined.1,
            self.longest
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
        pages: 0,
        nodes: 0,
        bounded: 0,
        headed: (0, 0),
        lined: (0, 0),
        longest: String::new(),
    };
    for _ in 0..repeats.max(1) {
        let started = Instant::now();
        let answer = viewer.query(Query::AccessibilityTree);
        let elapsed = started.elapsed();
        measured.best = measured.best.min(elapsed);
        if let Answer::Accessibility(pages) = answer {
            // Every page the arrangement is showing, which under `SinglePage` is one and under a
            // column is what this example is for: the cost of the question is the cost of the
            // screen rather than of a page.
            measured.pages = pages.len();
            let tree: Vec<&viewer_core::AccessibilityNode> =
                pages.iter().flat_map(|page| page.nodes.iter()).collect();
            measured.nodes = tree.len();
            measured.bounded = tree.iter().filter(|node| node.bounds.is_some()).count();
            measured.headed = (
                tree.iter().filter(|node| !node.headers.is_empty()).count(),
                tree.iter().map(|node| node.headers.len()).sum(),
            );
            let lines = || tree.iter().flat_map(|node| node.lines.iter());
            measured.lined = (
                lines().count(),
                lines().map(|line| line.characters.len()).sum(),
            );
            measured.longest = lines()
                .max_by_key(|line| line.characters.len())
                .map(|line| line.text.clone())
                .unwrap_or_default();
        }
    }
    measured
}

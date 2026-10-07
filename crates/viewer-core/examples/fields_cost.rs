//! What `Query::Fields` costs, on a page whose form makes it a question.
//!
//! The two toolkit hosts ask it on every repaint, to move their controls with the page, so its cost
//! is paid on every scroll and every zoom step rather than once. Since ADR 1604 each text field's
//! answer carries what Table 199's `/F` displays beside its characters, and that is a format run
//! per field per question; this is the stopwatch that says what it costs.
//!
//! ```sh
//! cargo run --release -p viewer-core --example fields_cost -- file.pdf [repeats]
//! ```
//!
//! It prints the best of `repeats` and how many fields the answer held. **Best of, not mean**, for
//! `accessibility_cost`'s reason: the slow runs of a warm loop are this machine's other processes.
//! An A/B on a busy machine belongs under callgrind with `--toggle-collect=*Viewer*::query*`, which
//! counts the query alone (ADR 0312).

#![expect(
    clippy::print_stdout,
    reason = "an example whose entire output is the measurement"
)]

use std::time::{Duration, Instant};

use viewer_core::{Answer, Command, DocumentId, Query, Viewer};

/// The document this run measures.
const DOCUMENT: DocumentId = DocumentId(1);

fn main() {
    let mut arguments = std::env::args().skip(1);
    let Some(path) = arguments.next() else {
        println!("usage: fields_cost <file.pdf> [repeats]");
        return;
    };
    let repeats: usize = arguments
        .next()
        .and_then(|count| count.parse().ok())
        .unwrap_or(20);
    let Ok(bytes) = std::fs::read(&path) else {
        println!("cannot read {path}");
        return;
    };
    let mut viewer = Viewer::new(800, 1000, 1.0);
    viewer
        .handle(Command::Open {
            id: DOCUMENT,
            bytes: bytes.into(),
            password: None,
            fragment: None,
        })
        .for_each(drop);
    let mut best = Duration::MAX;
    let mut fields = 0;
    let mut formatted = 0;
    for _ in 0..repeats.max(1) {
        let began = Instant::now();
        let answer = viewer.query(Query::Fields);
        best = best.min(began.elapsed());
        if let Answer::Fields(answered) = answer {
            fields = answered.len();
            formatted = answered
                .iter()
                .filter(|field| {
                    field.displayed.as_deref()
                        != field.value.as_ref().map(|shown| shown.text.as_str())
                })
                .count();
        }
    }
    println!(
        "Query::Fields: best of {repeats} {:.3} ms, {fields} field(s), {formatted} displayed through /F",
        best.as_secs_f64() * 1e3
    );
}

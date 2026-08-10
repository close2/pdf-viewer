//! What a document-wide search costs, measured in `find_step` alone rather than in a host's loop.
//!
//! `doc/todo/47`'s standing warning is that the first measurement of this feature was a
//! measurement of something else: a full-document miss took 19.25 s in the window and 13 s of it
//! was `viewer-ui` presenting a whole frame per step. So this example drives the pump with no
//! host at all — nothing is rendered, nothing is presented — and the number it prints is the
//! search's.
//!
//! It prints three things, because the third is what decides where work should be removed from:
//!
//! - the **cold** sweep, every page interpreted for the first time;
//! - the **repeated** sweep, which is what [`viewer_core`]'s readback cache is for;
//! - the **split** of one cold sweep into the page-tree walk (`Pages::get`) and the
//!   interpretation of the page that walk found, timed directly rather than inferred.
//!
//! ```sh
//! cargo run --profile gates -p viewer-core --example find_cost -- file.pdf needle [repeats] [split]
//! ```
//!
//! The split is the expensive part of the run and is printed only when the fourth argument asks
//! for it: it opens the document a second time so that no cache above can pay for either half.
//!
//! The needle should be one the document does **not** contain: a miss reads the whole plan, which
//! is the worst case and the only one whose cost is a property of the document rather than of
//! where the word happens to sit.

#![expect(
    clippy::print_stdout,
    reason = "an example whose entire output is the measurement"
)]

use std::time::{Duration, Instant};

use viewer_core::{Command, DocumentId, Event, Find, FindDirection, Viewer};

/// The document this run measures.
const DOCUMENT: DocumentId = DocumentId(1);

fn main() {
    let mut arguments = std::env::args().skip(1);
    let (Some(path), Some(needle)) = (arguments.next(), arguments.next()) else {
        println!("usage: find_cost <file.pdf> <needle> [repeats]");
        return;
    };
    let repeats: usize = arguments
        .next()
        .and_then(|count| count.parse().ok())
        .unwrap_or(3);
    let Ok(bytes) = std::fs::read(&path) else {
        println!("cannot read {path}");
        return;
    };

    let mut viewer = Viewer::new(1100, 1200, 1.0);
    let opening = Instant::now();
    let events: Vec<Event> = viewer
        .handle(Command::Open {
            id: DOCUMENT,
            bytes: bytes.clone(),
            password: None,
            fragment: None,
        })
        .collect();
    let opened = opening.elapsed();
    if !events
        .iter()
        .any(|event| matches!(event, Event::Opened { .. }))
    {
        println!("{path} did not open: {events:?}");
        return;
    }
    let pages = match viewer.query(viewer_core::Query::PageCount) {
        viewer_core::Answer::Count(count) => count,
        answer => {
            println!("no page count: {answer:?}");
            return;
        }
    };

    println!("{path}: {pages} pages, opened in {opened:.2?}");
    println!("needle {needle:?}, {repeats} sweeps");
    for sweep in 1..=repeats {
        let (elapsed, steps, found) = sweep_once(&mut viewer, &needle);
        let per_page = elapsed
            .checked_div(u32::try_from(steps.max(1)).unwrap_or(u32::MAX))
            .unwrap_or_default();
        let outcome = if found { "found" } else { "no match" };
        println!(
            "  sweep {sweep}: {elapsed:.2?} over {steps} steps ({per_page:.3?} a page, {outcome})"
        );
    }
    if let Some(held) = viewer.readback_cache(DOCUMENT) {
        println!(
            "  cache: {} pages, {} bytes of {} budget, {} hits, {} misses, {} evicted",
            held.pages, held.bytes, held.budget, held.hits, held.misses, held.evicted
        );
    }

    if arguments.next().as_deref() == Some("split") {
        split(&bytes);
    }
}

/// One whole search, driven the way a host drives it, with nothing rendered.
///
/// Returns the wall clock, the number of steps taken and whether it landed on a match. A host's
/// [`Event::NeedsRender`] is deliberately ignored: this example is measuring the search, and a
/// rasteriser in the loop is what made the first measurement of it wrong.
fn sweep_once(viewer: &mut Viewer, needle: &str) -> (Duration, usize, bool) {
    let started = Instant::now();
    let mut events: Vec<Event> = viewer
        .handle(Command::Find(Find::Start {
            needle: needle.to_owned(),
            direction: FindDirection::Forward,
        }))
        .collect();
    let mut steps = 0_usize;
    loop {
        steps = steps.saturating_add(1);
        let mut remaining = 0_usize;
        let mut found = false;
        for event in &events {
            if let Event::Searched {
                found: answer,
                remaining: left,
                ..
            } = event
            {
                remaining = *left;
                found = answer.is_some();
            }
        }
        if found {
            return (started.elapsed(), steps, true);
        }
        if remaining == 0 {
            return (started.elapsed(), steps, false);
        }
        events = viewer.handle(Command::Find(Find::Continue)).collect();
    }
}

/// One cold sweep's cost split into the two halves of a step, timed rather than inferred.
///
/// A step is `Pages::get` — §7.7.3.2's tree walked from the root, once per page — and then
/// `interpret_with` on the leaf it found. Both are timed here against a document opened for this
/// purpose alone, so that the viewer's own caches above cannot pay for either.
fn split(bytes: &[u8]) {
    let Ok(document) = pdf_syntax::Document::open(bytes.to_vec()) else {
        println!("  split: the document would not open a second time");
        return;
    };
    let pages = pdf_model::Pages::new(&document);
    let state = pdf_model::view::ViewState::of(&document);
    let (mut walking, mut interpreting, mut readback) = (Duration::ZERO, Duration::ZERO, 0_usize);
    for index in 0..pages.len() {
        let started = Instant::now();
        let Some(page) = pages.get(index) else {
            continue;
        };
        walking = walking.saturating_add(started.elapsed());
        let started = Instant::now();
        let interpretation = pdf_model::content::interpret_with(&document, &page, &state);
        interpreting = interpreting.saturating_add(started.elapsed());
        readback = readback.saturating_add(interpretation.text.len());
    }
    // `Open::page` builds a fresh `Pages` on every call, so the index it costs to *have* is part
    // of a step as much as the leaf walk is. Timed apart because the two have different answers:
    // one is a catalogue lookup and a `/Count`, the other descends §7.7.3.2's tree.
    let started = Instant::now();
    let mut indexed = 0_usize;
    for _ in 0..pages.len() {
        indexed = indexed.saturating_add(pdf_model::Pages::new(&document).len());
    }
    let indexing = started.elapsed();
    println!(
        "  index: {indexing:.2?} to build {} page indices",
        pages.len()
    );
    let _ = indexed;

    let total = walking.saturating_add(interpreting);
    let share = |part: Duration| {
        if total.is_zero() {
            0.0
        } else {
            part.as_secs_f64() * 100.0 / total.as_secs_f64()
        }
    };
    println!(
        "  split: page tree {walking:.2?} ({:.1}%), interpretation {interpreting:.2?} ({:.1}%), \
         readback {} bytes",
        share(walking),
        share(interpreting),
        readback
    );
}

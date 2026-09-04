//! Every awkward class of document, opened and drawn through `pdf-view-worker`.
//!
//! # The question, and why the other sweep does not answer it
//!
//! `pdf-vfs`'s `tests/read_corpus.rs` walks the same ten classes through the *file system's*
//! confined worker and holds every file it produces against its generator. It is the wrong
//! instrument for this one thing: the program a person looks at pages with is a **different**
//! confined program. The two share `pdf-sandbox`'s `Profile::Interpreter` and
//! `confined-transport`'s supervision, so a system call one of them is killed for is one the
//! other is killed for — and session 914 found exactly that, `pdf_font::substitute` walking
//! `/usr/share/fonts` under a filter whose action is a kill, in a worker of each. What differs is
//! the *consequence*: a mount loses one generated file, and the viewer loses the whole page a
//! person was reading (ADR 0870).
//!
//! So this asks the narrow question of the other worker: **does `pdf-view-worker` survive**, over
//! a population drawn from every corpus on this disk rather than from the four committed
//! documents `tests/confined.rs` carries. ADR 0879.
//!
//! # What it does
//!
//! [`corpus_classes`] chooses the population — a stride sample of every root, classified, with
//! the first few of each class taken — so that this sweep and the read walk are one population
//! and one vocabulary rather than two that drift. For each document: a confined viewer is
//! started, the document is opened **as a descriptor** (ADR 0812, which is what a host does with
//! a file on disk), and the first [`PAGES_TURNED`] pages are turned, each one interpreted and
//! drawn behind the filter. The frame is asked for afterwards, because a page a host never asks
//! for is a page the worker may not have drawn.
//!
//! # What fails it
//!
//! A **death**: `confined-transport`'s supervision words one as `killed by signal N` and
//! [`corpus_classes::is_a_death`] is the predicate. Everything else is counted and printed — a
//! locked document, a page the budget refuses, an image whose codec is a program this process may
//! not start — because a refusal is a sentence a host can show and is not a failure (trap 11).
//!
//! # Running it
//!
//! ```text
//! cargo build -p viewer-confined --bins
//! tools/bounded.sh --data 12 --tree 12 -- \
//!     cargo test --profile gates -p viewer-confined --test awkward_classes -- --ignored --nocapture
//! ```

#![expect(
    clippy::expect_used,
    clippy::print_stdout,
    reason = "test code: a helper that cannot start a confined viewer must say so rather than \
              report every document as answered, and the matrix is the point of the run"
)]

use std::collections::BTreeMap;
use std::sync::Mutex;
use std::time::Instant;

use corpus_classes::{Choice, Chosen, Class, is_a_death};
use rayon::prelude::{IntoParallelRefIterator, ParallelIterator};
use viewer_confined::{Confined, Reply};
use viewer_core::{Command, DocumentId, Event, PageTarget, Query};

/// How many documents are classified from each corpus root.
const SAMPLE_PER_ROOT: usize = 1200;

/// How many documents of each class are swept, from each root.
const PER_CLASS: usize = 6;

/// How many pages of each document are turned to, beyond the one an open draws.
///
/// Three, and the reason is the shape of what these sweeps find rather than a coverage argument:
/// every confined-worker death this tree has met was on the *first* thing the worker did with a
/// page — a font looked up, an arena allocated, the machine asked how many cores it has — so the
/// pages after the first are there to reach a second kind of page in a document rather than to
/// reach page ten.
const PAGES_TURNED: usize = 3;

/// The viewport the sweep looks at every page through, in device pixels.
const VIEWPORT: (u32, u32) = (900, 1100);

/// What the host calls the one document each mount holds.
const DOCUMENT: DocumentId = DocumentId(1);

/// How many distinct refusal reasons the report prints.
const REASONS_PRINTED: usize = 20;

/// What one question of a confined viewer answered.
#[derive(Debug, Clone)]
enum Outcome {
    /// It answered.
    Answered,
    /// It refused, by name.
    Refused(String),
    /// The worker died: the failure this sweep exists to find.
    Killed(String),
}

/// One document's whole sweep.
#[derive(Debug)]
struct Swept {
    /// Which document, as `<root>/<file name>`.
    name: String,
    /// Every class it is swept under.
    classes: Vec<Class>,
    /// Every question asked of it and what it answered.
    cells: Vec<(String, Outcome)>,
    /// How many frames the viewer held afterwards.
    frames: usize,
    /// Every sentence §12.11's, §7.11.4's or a page's own report put to the person.
    ///
    /// Counted apart from the refusals, because a page that draws and *says* what it could not
    /// draw is `safedocs::survey::Outcome::Incomplete` rather than a refusal — and neither is a
    /// death, which is the only thing that fails this run.
    notes: Vec<String>,
}

/// One error, read as a death or as a refusal.
fn outcome_of(sentence: &str) -> Outcome {
    if is_a_death(sentence) {
        Outcome::Killed(sentence.to_owned())
    } else {
        Outcome::Refused(sentence.to_owned())
    }
}

/// Why an open did not open, in the viewer's own words, or nothing.
fn refusal_in(events: &[Event]) -> Option<String> {
    events.iter().find_map(|event| match event {
        Event::OpenFailed { reason, .. } => Some(reason.clone()),
        Event::PasswordRequired { .. } => {
            Some("§7.6.4.1: the document wants a password".to_owned())
        }
        _ => None,
    })
}

/// Every sentence a page reported, which is what a person would be shown beside it.
fn notes_in(events: &[Event]) -> Vec<String> {
    events
        .iter()
        .filter_map(|event| match event {
            Event::Reported { notes, .. } => Some(notes.clone()),
            _ => None,
        })
        .flatten()
        .collect()
}

/// Opens one document behind the filter and turns [`PAGES_TURNED`] pages of it.
fn sweep_one(chosen: &Chosen) -> Swept {
    let mut swept = Swept {
        name: chosen.display.clone(),
        classes: chosen.classes.clone(),
        cells: Vec::new(),
        frames: 0,
        notes: Vec::new(),
    };
    let bytes = match pdf_syntax::FileBytes::on_disk(&chosen.path) {
        Ok(bytes) => bytes,
        Err(error) => {
            swept
                .cells
                .push(("open the file".to_owned(), outcome_of(&error.to_string())));
            return swept;
        }
    };
    // A viewer that could not be started would report every document as answered, which is trap
    // 16's shape: the instrument measures a program the build did not produce.
    let mut confined = Confined::start().expect(
        "a confined viewer starts — `pdf-view-worker` is built beside the test binary by \
         `cargo build -p viewer-confined --bins`",
    );

    match confined.handle(&Command::Resize {
        width: VIEWPORT.0,
        height: VIEWPORT.1,
        scale: 1.0,
    }) {
        Ok(_) => swept.cells.push(("resize".to_owned(), Outcome::Answered)),
        Err(error) => {
            swept
                .cells
                .push(("resize".to_owned(), outcome_of(&error.to_string())));
            return swept;
        }
    }

    match confined.handle(&Command::Open {
        id: DOCUMENT,
        bytes,
        password: None,
        fragment: None,
    }) {
        Err(error) => {
            swept
                .cells
                .push(("open".to_owned(), outcome_of(&error.to_string())));
            return swept;
        }
        Ok(events) => {
            swept.notes.extend(notes_in(&events));
            if let Some(why) = refusal_in(&events) {
                swept.cells.push(("open".to_owned(), Outcome::Refused(why)));
                return swept;
            }
            swept.cells.push(("open".to_owned(), Outcome::Answered));
        }
    }

    for page in 1..=PAGES_TURNED {
        let what = format!("page {}", page.saturating_add(1));
        match confined.handle(&Command::GoTo(PageTarget::Next)) {
            Ok(events) => {
                swept.notes.extend(notes_in(&events));
                swept.cells.push((what, Outcome::Answered));
            }
            Err(error) => {
                let outcome = outcome_of(&error.to_string());
                let dead = matches!(outcome, Outcome::Killed(_));
                swept.cells.push((what, outcome));
                if dead {
                    return swept;
                }
            }
        }
    }

    // The frame is asked for rather than assumed: a page a host never asks for is a page the
    // worker may not have finished with, and what this sweep is about is what drawing does to it.
    match confined.query(Query::Frame) {
        Ok(Reply::Frame(held)) => {
            swept.frames = held.len();
            swept.cells.push(("frame".to_owned(), Outcome::Answered));
        }
        Ok(other) => swept.cells.push((
            "frame".to_owned(),
            Outcome::Refused(format!("the confined viewer answered {other:?}")),
        )),
        Err(error) => swept
            .cells
            .push(("frame".to_owned(), outcome_of(&error.to_string()))),
    }
    swept
}

/// A refusal's first line with the path taken off it, so that the report groups by *reason*
/// rather than by document.
fn first_sentence(sentence: &str) -> String {
    let head = sentence.lines().next().unwrap_or(sentence);
    match head.split_once(": ") {
        Some((_, reason)) if !reason.is_empty() => reason.to_owned(),
        _ => head.to_owned(),
    }
}

/// The class-by-class report, and the list that decides whether the run passes.
fn report(swept: &[Swept]) -> Vec<String> {
    let mut kills = Vec::new();
    let mut refusals: BTreeMap<String, usize> = BTreeMap::new();
    for class in Class::ALL {
        let mine: Vec<&Swept> = swept
            .iter()
            .filter(|one| one.classes.contains(class))
            .collect();
        if mine.is_empty() {
            println!(
                "view-awkward:   {:<24} no document on this disk",
                class.name()
            );
            continue;
        }
        let count = |of: fn(&Outcome) -> bool| -> usize {
            mine.iter()
                .map(|one| one.cells.iter().filter(|(_, out)| of(out)).count())
                .sum()
        };
        println!(
            "view-awkward:   {:<24} {} document(s), {} answered, {} frame(s), {} refused, \
             {} reported, {} killed",
            class.name(),
            mine.len(),
            count(|out| matches!(out, Outcome::Answered)),
            mine.iter().map(|one| one.frames).sum::<usize>(),
            count(|out| matches!(out, Outcome::Refused(_))),
            mine.iter().map(|one| one.notes.len()).sum::<usize>(),
            count(|out| matches!(out, Outcome::Killed(_))),
        );
        for one in mine {
            for note in &one.notes {
                let seen = refusals.entry(first_sentence(note)).or_default();
                *seen = seen.saturating_add(1);
            }
            for (question, outcome) in &one.cells {
                match outcome {
                    Outcome::Answered => {}
                    Outcome::Refused(sentence) => {
                        let seen = refusals.entry(first_sentence(sentence)).or_default();
                        *seen = seen.saturating_add(1);
                    }
                    Outcome::Killed(detail) => {
                        kills.push(format!("{} {question}: {detail}", one.name));
                    }
                }
            }
        }
    }
    println!("view-awkward: refused or reported, by reason:");
    let mut by_count: Vec<(&String, &usize)> = refusals.iter().collect();
    by_count.sort_by(|left, right| right.1.cmp(left.1).then(left.0.cmp(right.0)));
    for (reason, seen) in by_count.iter().take(REASONS_PRINTED) {
        println!("view-awkward:   {seen:>4}  {reason}");
    }
    kills
}

/// The sweep.
#[test]
#[ignore = "corpus-scale: a document of each class from every corpus on the disk, opened and drawn through the confined viewer; run it as doc/verify.md says"]
fn no_awkward_class_of_document_kills_the_confined_viewer() {
    let started = Instant::now();
    let roots = corpus_classes::roots();
    assert!(!roots.is_empty(), "no corpus root on this disk");
    let choice = Choice {
        whole: Vec::new(),
        sample_per_root: SAMPLE_PER_ROOT,
        per_class: PER_CLASS,
    };
    let (chosen, contributions) = corpus_classes::population(&roots, &choice, &|_| String::new());
    assert!(
        !chosen.is_empty(),
        "every corpus root on this disk is empty"
    );

    let swept = Mutex::new(Vec::new());
    chosen.par_iter().for_each(|document| {
        let one = sweep_one(document);
        if let Ok(mut swept) = swept.lock() {
            swept.push(one);
        }
    });
    let mut swept = swept.into_inner().expect("the sweeps");
    swept.sort_by(|left, right| left.name.cmp(&right.name));

    println!(
        "view-awkward: {} root(s), {} document(s) classified, {} swept, {} threads",
        roots.len(),
        contributions
            .iter()
            .map(|one| one.classified)
            .sum::<usize>(),
        chosen.len(),
        rayon::current_num_threads()
    );
    for contribution in &contributions {
        println!(
            "view-awkward:   {}: {} classified, {} swept",
            contribution.root, contribution.classified, contribution.chosen
        );
    }
    let kills = report(&swept);
    println!(
        "view-awkward: killed: {}, in {:.1}s",
        kills.len(),
        started.elapsed().as_secs_f64()
    );

    assert!(
        kills.is_empty(),
        "the confined viewer was killed by a signal on {} question(s), and a page a person was \
         reading is what that costs:\n{}",
        kills.len(),
        kills.join("\n")
    );
}

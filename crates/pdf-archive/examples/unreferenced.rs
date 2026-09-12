//! What ISO 19005's unreferenced-named-resource exemption reaches, and what nothing reaches.
//!
//! ```sh
//! cargo run --release -p pdf-archive --example unreferenced -- doc/veraPDF-corpus
//! ```
//!
//! # What it was, and what it is
//!
//! This example used to carry the whole computation: a hand walk of the object graph, a fixpoint
//! over indirect resources dictionaries, and two reachability passes, because
//! `pdf_archive::Examination` had no answer about a *relationship between objects* and
//! `doc/todo/62` was blocked on one. `pdf_archive::reach` is now that answer —
//! `Examination::reaches` and `Examination::exempt` — so what is left here is the printing.
//!
//! # The sentence being counted
//!
//! ISO 19005-2 section 6.2.2 and ISO 19005-4 section 6.2.2 both close with an exemption: a named
//! resource present in a resources dictionary whose name the associated content stream never
//! references is not used for rendering, and is therefore exempt from the part's requirements —
//! in part 4 with sections 6.1.6 to 6.1.9 carved back out, and in part 2 with sections 6.1.2 to
//! 6.1.13 carved back out by `TechNote 0010` A010. `pdf_archive::reach::exemption_narrows` reads
//! the two carve-outs and `pdf_archive::check` applies them.
//!
//! # What the fourth line means now, and why the number did not move
//!
//! A row that still fails on nothing but exempt objects is a row the applicable **carve-out
//! keeps**: the exemption withdrew nothing there because the clause is one of the ones the part
//! took back. Session 944 measured exactly that population before the exemption was implemented
//! and found every one of it inside a carve-out (`doc/todo/62` section 3); the same count printed
//! here after it is implemented is the check on that reading.
//!
//! Two approximations of the exempt set run one way — every name operand of a content stream
//! counts as a reference, and a resources dictionary two owners share carries the union of their
//! names — so a document this prints is a candidate and a document it does not print may still
//! hold a resource a stricter reading of *referenced* would free. `pdf_archive::Exempt` states
//! both.

#![expect(
    clippy::print_stdout,
    reason = "an example whose whole product is a count a person reads"
)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use pdf_archive::{Examination, Flavour, Level, Outcome, Target, check};
use pdf_syntax::{Document, FileBytes};

fn main() {
    let mut arguments = std::env::args().skip(1);
    let root = arguments
        .next()
        .map_or_else(|| PathBuf::from("doc/veraPDF-corpus"), PathBuf::from);
    if !root.is_dir() {
        println!("{}: not a directory", root.display());
        return;
    }
    for (folder, target) in [
        ("PDF_A-4", Target::Four(Flavour::Plain)),
        ("PDF_A-4f", Target::Four(Flavour::F)),
        ("PDF_A-4e", Target::Four(Flavour::E)),
        ("PDF_A-2b", Target::Two(Level::B)),
        ("PDF_A-2u", Target::Two(Level::U)),
        ("PDF_A-2a", Target::Two(Level::A)),
    ] {
        let corner = root.join(folder);
        if corner.is_dir() {
            sweep(folder, &corner, target);
        }
    }
}

/// One document's answer.
#[derive(Debug, Default)]
struct Measurement {
    /// Named resource entries whose name the associated content stream does not reference.
    entries: usize,
    /// How many objects are reachable only through such an entry.
    exempt: usize,
    /// How many objects a cross-reference section names that the trailer reaches not at all.
    unreferenced: usize,
    /// Whether a bound stopped one of the walks, so that nothing was exempted.
    bounded: bool,
    /// The rows that failed on nothing but exempt objects: citation and row identifier.
    only_exempt: BTreeMap<String, &'static str>,
    /// Whether some other row failed too, so that withdrawing these would not clear the verdict.
    other_failures: bool,
}

/// Runs one target's corner of the corpus and prints what the exemption reaches.
fn sweep(folder: &str, corner: &Path, target: Target) {
    let mut documents = 0_usize;
    let mut with_entries = 0_usize;
    let mut with_exempt = 0_usize;
    let mut with_unreferenced = 0_usize;
    let mut bounded = 0_usize;
    let mut would_pass = 0_usize;
    let mut candidates: Vec<(String, Vec<String>)> = Vec::new();
    for path in files(corner) {
        let Ok(bytes) = FileBytes::on_disk(&path) else {
            continue;
        };
        let Ok(document) = Document::open(bytes) else {
            continue;
        };
        documents = documents.saturating_add(1);
        let measured = measure(&document, target);
        if measured.entries > 0 {
            with_entries = with_entries.saturating_add(1);
        }
        if measured.exempt > 0 {
            with_exempt = with_exempt.saturating_add(1);
        }
        if measured.unreferenced > 0 {
            with_unreferenced = with_unreferenced.saturating_add(1);
        }
        if measured.bounded {
            bounded = bounded.saturating_add(1);
        }
        if measured.only_exempt.is_empty() {
            continue;
        }
        if !measured.other_failures {
            would_pass = would_pass.saturating_add(1);
        }
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("?")
            .to_owned();
        let rows = measured
            .only_exempt
            .iter()
            .map(|(citation, id)| format!("{citation}  {id}"))
            .collect();
        candidates.push((name, rows));
    }
    println!("\n== {folder} ==");
    println!("  {documents:>5} documents read");
    println!("  {with_entries:>5} state a named resource their content stream does not reference");
    println!("  {with_exempt:>5} hold an object reachable only through such an entry");
    println!(
        "  {:>5} fail a row whose every finding is on such an object, the carve-out keeping it",
        candidates.len()
    );
    println!("  {would_pass:>5} of those fail nothing else, so the verdict itself turns on one");
    println!("  {with_unreferenced:>5} hold an object the trailer reaches not at all");
    println!("  {bounded:>5} stopped a walk at a bound, so nothing in them is exempt");
    for (name, rows) in &candidates {
        println!("    {name}");
        for row in rows {
            println!("      {row}");
        }
    }
}

/// Every PDF under a directory, in a stable order.
fn files(root: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let mut pending = vec![root.to_path_buf()];
    while let Some(directory) = pending.pop() {
        let Ok(entries) = std::fs::read_dir(&directory) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                pending.push(path);
            } else if path.extension().is_some_and(|kind| kind == "pdf") {
                found.push(path);
            }
        }
    }
    found.sort();
    found
}

/// One document's exempt objects, and which of its failures rest on them alone.
fn measure(document: &Document, target: Target) -> Measurement {
    let exam = Examination::new(document, target);
    let exempt = exam.exempt();
    let mut measurement = Measurement {
        entries: exempt.entries(),
        exempt: exempt.objects().len(),
        unreferenced: exam.reaches().unreferenced().len(),
        bounded: exempt.stopped_at().is_some(),
        ..Measurement::default()
    };
    if measurement.exempt == 0 {
        return measurement;
    }
    for judgement in &check(document, target).judgements {
        let Outcome::Failed { places, total } = &judgement.outcome else {
            continue;
        };
        // A truncated list is a prefix, so "every finding is exempt" cannot be read off it, and
        // such a row counts as a failure the exemption leaves standing.
        if *total == places.len()
            && !places.is_empty()
            && places
                .iter()
                .all(|place| place.place.object.is_some_and(|id| exempt.holds(id)))
        {
            measurement
                .only_exempt
                .insert(judgement.citation.clone(), judgement.id);
        } else {
            measurement.other_failures = true;
        }
    }
    measurement
}

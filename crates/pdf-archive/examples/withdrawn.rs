//! What ISO 19005 section 6.2.2's exemption actually withdraws, per requirement, over a corpus.
//!
//! ```sh
//! cargo run --release -p pdf-archive --example withdrawn -- doc/veraPDF-corpus doc/pdf.js
//! cargo run --release -p pdf-archive --example withdrawn -- --threads 8 --targets 2a,4 corpus-cache
//! ```
//!
//! With no path it reads `doc/veraPDF-corpus`; `--targets` defaults to all six, `--threads` to one,
//! `--max-mb` to [`MAX_MB`], and `--row <identifier>` names every witness of one row on both sides
//! of the exemption rather than the first few.
//!
//! # The question, and why `examples/unreferenced.rs` does not answer it
//!
//! That example counts the *population*: how many documents state a named resource their content
//! stream never references, and how many objects only such an entry reaches. It says nothing
//! about which **requirements** the exemption then withdraws a finding from, and that is the
//! question `doc/questions/Q62` asks — a rule the exemption withdraws on every document that
//! would otherwise fail it is a rule no document is ever judged against, which is the shape Q62
//! found twice in the converter.
//!
//! So this walks the same corpora row by row rather than document by document: every requirement
//! the target binds is run twice where the narrowing could bite — once over the whole document,
//! once over the population section 6.2.2 leaves — and the difference is counted per row
//! identifier. `crate::check` does exactly this internally and keeps only the verdict; here the
//! two runs are kept apart so that the *reach* can be reported.
//!
//! # What each column of the row table means
//!
//! - **failed** — documents where the predicate found at least one place, before any narrowing.
//!   The denominator: a row with none of these is a rule no document in these corpora exercises,
//!   which is a fact about the corpora rather than about the exemption.
//! - **narrowed** — documents where the exemption withdrew at least one of those places.
//! - **cleared** — documents where it withdrew every one of them, so the row went from failed to
//!   met. These are the documents whose verdict the exemption decides.
//! - **stood** — documents where a place survived it, so the row still failed.
//!
//! A document counts once in each column it earns, across every target of the run: a row the exemption
//! clears under one part and not under the other is one `cleared` and one `stood`, which is the
//! honest shape — the two parts carve different ranges out of the same sentence, so a row can be
//! withdrawn under one and kept under the other.
//!
//! # The five bounds, each reported rather than applied in silence
//!
//! A corpus of real files is not the veraPDF corpus: it holds documents this reader cannot open,
//! documents large enough that judging them would cost more than the answer is worth, and — the
//! reason this example catches unwinds — documents that have found a panic in some reader
//! underneath. Each of those is counted and the names are printed, and so are the documents a
//! `--skip` pattern named and the ones that took longer than [`SLOW`] — because a run that quietly
//! passed over a thousand files would report a reach that is an artefact of what it passed over.

#![expect(
    clippy::print_stdout,
    clippy::print_stderr,
    reason = "an example whose whole product is a table a person reads, with progress on stderr"
)]

use std::collections::{BTreeMap, BTreeSet};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;

use pdf_archive::{
    Check, Examination, Findings, Flavour, Level, Part, Reaches, Target, reach, table, withdrawal,
};
use pdf_syntax::{Document, FileBytes, ObjectId};

/// How large a document may be before this example declines to judge it, in mebibytes.
///
/// Not a statement about the reader, which opens what it is given: a corpus of real files holds
/// a handful of documents of several gigabytes, and judging one of them costs more than every
/// other document of the corpus together. `--max-mb` overrides it and the skipped names are
/// printed, so the bound is visible in the answer rather than hidden in it.
const MAX_MB: u64 = 512;

/// How long one document may take to judge before this example names it, in seconds.
///
/// Not a bound — nothing is stopped — but a corpus walk whose wall clock is decided by three
/// documents out of ninety thousand should say which three, and a validator that takes minutes
/// over one file is a fact about this crate rather than about the exemption.
const SLOW: f64 = 10.0;

/// What this run was asked for.
#[derive(Debug)]
struct Options {
    /// The targets each document is judged against.
    targets: Vec<Target>,
    /// How many documents are judged at a time.
    threads: usize,
    /// The size above which a document is skipped and named.
    max_mb: u64,
    /// Requirement identifiers whose every witness is to be named, rather than the first few.
    rows: Vec<String>,
    /// File names holding one of these is skipped, and named in the report.
    ///
    /// One document of `doc/veraPDF-corpus` is the reason: the Isartor implementation-limits
    /// file states more indirect objects than the limit it is testing, and judging it against six
    /// targets takes longer than the whole corpus beside it. Skipping is a decision a person
    /// makes at the command line and the report prints, never a default.
    skip: Vec<String>,
}

fn main() {
    let mut roots: Vec<PathBuf> = Vec::new();
    let mut options = Options {
        targets: Target::ALL.to_vec(),
        threads: 1,
        max_mb: MAX_MB,
        rows: Vec::new(),
        skip: Vec::new(),
    };
    let mut arguments = std::env::args().skip(1);
    while let Some(argument) = arguments.next() {
        let mut value = || arguments.next().unwrap_or_default();
        match argument.as_str() {
            "--max-mb" => options.max_mb = value().parse().unwrap_or(MAX_MB),
            "--threads" => options.threads = value().parse().unwrap_or(1).max(1),
            "--row" => options.rows.push(value()),
            "--skip" => options.skip.push(value()),
            "--targets" => {
                let Some(targets) = targets_named(&value()) else {
                    eprintln!("--targets wants a comma-separated list of 2b 2u 2a 4 4f 4e");
                    return;
                };
                options.targets = targets;
            }
            _ => roots.push(PathBuf::from(argument)),
        }
    }
    if roots.is_empty() {
        roots.push(PathBuf::from("doc/veraPDF-corpus"));
    }
    println!("targets: {}", describe(&options.targets));
    let mut whole = Tally::default();
    for root in &roots {
        let mut corpus = Tally::default();
        sweep(root, &options, &mut corpus);
        report(&root.display().to_string(), &corpus, &options);
        whole.absorb(&corpus);
    }
    if roots.len() > 1 {
        report("every corpus named", &whole, &options);
    }
}

/// The targets a `--targets` list names, or `None` where one of the words is not a target.
fn targets_named(list: &str) -> Option<Vec<Target>> {
    list.split(',')
        .map(|word| match word.trim() {
            "2b" => Some(Target::Two(Level::B)),
            "2u" => Some(Target::Two(Level::U)),
            "2a" => Some(Target::Two(Level::A)),
            "4" => Some(Target::Four(Flavour::Plain)),
            "4f" => Some(Target::Four(Flavour::F)),
            "4e" => Some(Target::Four(Flavour::E)),
            _ => None,
        })
        .collect()
}

/// The targets of a run, as a person writes them.
fn describe(targets: &[Target]) -> String {
    targets
        .iter()
        .map(|target| format!("{target}"))
        .collect::<Vec<_>>()
        .join(", ")
}

/// What one corpus's documents did, per requirement and in total.
#[derive(Debug, Default)]
struct Tally {
    /// Per requirement identifier, what the exemption did to it.
    rows: BTreeMap<&'static str, Row>,
    /// Documents read to a verdict.
    documents: usize,
    /// Documents this reader could not open.
    unreadable: usize,
    /// Documents larger than the size bound, with their names.
    oversized: Vec<String>,
    /// Documents that panicked somewhere under a predicate, with their names.
    panicked: Vec<String>,
    /// Documents that took longer than [`SLOW`] to judge, with their names and what they took.
    slow: Vec<String>,
    /// Documents `--skip` named, with their names.
    skipped: Vec<String>,
    /// Documents that were asked for the exempt population and had one.
    ///
    /// Not *every* document holding an unreferenced named resource: the question is put only
    /// where a row the exemption narrows has already failed at a place naming an object, which is
    /// where `crate::check` puts it. `examples/unreferenced.rs` counts the population itself.
    with_exempt: usize,
    /// Documents where a bound stopped a walk, so nothing in them was exempt.
    bounded: usize,
}

impl Tally {
    /// Adds one corpus's answer to a running total.
    fn absorb(&mut self, other: &Self) {
        for (id, row) in &other.rows {
            self.rows.entry(id).or_default().absorb(row);
        }
        self.documents = self.documents.saturating_add(other.documents);
        self.unreadable = self.unreadable.saturating_add(other.unreadable);
        self.oversized.extend(other.oversized.iter().cloned());
        self.panicked.extend(other.panicked.iter().cloned());
        self.slow.extend(other.slow.iter().cloned());
        self.skipped.extend(other.skipped.iter().cloned());
        self.with_exempt = self.with_exempt.saturating_add(other.with_exempt);
        self.bounded = self.bounded.saturating_add(other.bounded);
    }
}

/// How many witnesses one row keeps, so that a reach can be looked at rather than only counted.
const WITNESSES: usize = 12;

/// What the exemption did to one requirement, counted in documents.
#[derive(Debug, Default)]
struct Row {
    /// Documents where the predicate found a place before the narrowing.
    failed: usize,
    /// Documents where the narrowing withdrew at least one place.
    narrowed: usize,
    /// Documents where it withdrew every place, so the row went from failed to met.
    cleared: usize,
    /// Documents where a place survived the narrowing.
    stood: usize,
    /// Whether any target this row binds has the exemption narrow its clause.
    narrowable: bool,
    /// The first few documents the narrowing withdrew a place from, by name.
    witnesses: Vec<String>,
    /// Every witness of a row `--row` named, with which side of the exemption it fell on.
    both: Vec<String>,
}

impl Row {
    /// Adds one corpus's answer for this row to a running total.
    fn absorb(&mut self, other: &Self) {
        self.failed = self.failed.saturating_add(other.failed);
        self.narrowed = self.narrowed.saturating_add(other.narrowed);
        self.cleared = self.cleared.saturating_add(other.cleared);
        self.stood = self.stood.saturating_add(other.stood);
        self.narrowable |= other.narrowable;
        for witness in &other.witnesses {
            if self.witnesses.len() < WITNESSES {
                self.witnesses.push(witness.clone());
            }
        }
        self.both.extend(other.both.iter().cloned());
    }
}

/// What one document did to one row, once, however many targets asked.
///
/// A document is one witness however many targets of the run bind the row: the question is how
/// many *documents* the exemption reaches, and counting a file once per target would answer a
/// question about the target list instead.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Did {
    /// The predicate found a place before the narrowing.
    Failed,
    /// The narrowing withdrew at least one place.
    Narrowed,
    /// The narrowing withdrew every place.
    Cleared,
    /// A place survived the narrowing.
    Stood,
    /// Some target's carve-out, or some target's clause, leaves this row narrowable.
    Narrowable,
}

/// Walks one corpus, judging every document against every target of the run.
///
/// The documents are independent — nothing is cached across them and each holds its own reader —
/// so the walk is shared over `--threads` workers taking the next path off one cursor. One
/// thread is the default, because `tools/bounded.sh`'s rule is that the machine runs one corpus
/// walk at a time and a worker costs a document in flight.
///
/// **A root is a checkpoint, and on a large corpus that is what to use it for.** Each root's
/// report is printed as that root finishes, so a walk stopped by a bound — eight workers over the
/// crawl reached `bounded.sh`'s twelve-gibibyte `RLIMIT_DATA` in session 1007 and aborted with
/// nothing printed — keeps what it had. Name the subdirectories rather than the corpus when the
/// walk is long enough to lose.
fn sweep(root: &Path, options: &Options, tally: &mut Tally) {
    let paths = files(root);
    let started = Instant::now();
    let cursor = AtomicUsize::new(0);
    let finished = AtomicUsize::new(0);
    std::thread::scope(|scope| {
        let mut workers = Vec::new();
        for _ in 0..options.threads {
            let (cursor, finished, paths, started) = (&cursor, &finished, &paths, started);
            workers.push(scope.spawn(move || {
                let mut mine = Tally::default();
                loop {
                    let at = cursor.fetch_add(1, Ordering::Relaxed);
                    let Some(path) = paths.get(at) else {
                        break;
                    };
                    one(path, options, &mut mine);
                    let done = finished.fetch_add(1, Ordering::Relaxed).saturating_add(1);
                    if done % 500 == 0 {
                        eprintln!(
                            "  {} {done}/{} in {:.0}s",
                            root.display(),
                            paths.len(),
                            started.elapsed().as_secs_f64()
                        );
                    }
                }
                mine
            }));
        }
        for worker in workers {
            match worker.join() {
                Ok(mine) => tally.absorb(&mine),
                // A worker that unwound past its own guard is a defect in this example rather
                // than a document's doing, and a total that silently lost a worker's documents
                // would be worse than a loud one.
                Err(_) => eprintln!("  a worker of {} died", root.display()),
            }
        }
    });
}

/// One document of the corpus, with the two bounds this example applies to it.
fn one(path: &Path, options: &Options, tally: &mut Tally) {
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("?")
        .to_owned();
    if options.skip.iter().any(|pattern| name.contains(pattern)) {
        tally.skipped.push(name);
        return;
    }
    let size = std::fs::metadata(path).map_or(0, |data| data.len());
    if size > options.max_mb.saturating_mul(1 << 20) {
        tally.oversized.push(name);
        return;
    }
    // A corpus of real files is where a reader's panic lives, and losing an hours-long
    // measurement to one is worse than reporting it: the name is kept and the walk goes on.
    // Nothing here is retried and no state crosses documents, so a caught unwind leaves nothing
    // half-done.
    let started = Instant::now();
    match catch_unwind(AssertUnwindSafe(|| document(path, &name, options))) {
        Ok(Some(answer)) => tally.absorb(&answer),
        Ok(None) => tally.unreadable = tally.unreadable.saturating_add(1),
        Err(_) => tally.panicked.push(name.clone()),
    }
    let took = started.elapsed().as_secs_f64();
    if took > SLOW {
        tally.slow.push(format!("{took:.0}s  {}", path.display()));
    }
}

/// One document, judged against every target of the run, as a tally of its own.
fn document(path: &Path, name: &str, options: &Options) -> Option<Tally> {
    let bytes = FileBytes::on_disk(path).ok()?;
    let document = Document::open(bytes).ok()?;
    let mut tally = Tally {
        documents: 1,
        ..Tally::default()
    };
    let mut did: BTreeMap<&'static str, BTreeSet<Did>> = BTreeMap::new();
    // The exempt population is a function of the document alone — `Exempt::of` reads the object
    // graph and the content streams, and no field of the target — so it is computed once here and
    // lent to every target. `crate::check` cannot do that, because it holds one examination and
    // one target at a time; an example judging the same file six times can, and it is the
    // difference between one walk of a large document and six.
    let mut population: Option<(Arc<BTreeSet<ObjectId>>, bool)> = None;
    for target in &options.targets {
        let exam = Examination::new(&document, *target);
        for requirement in table::binding(*target) {
            let Check::Implemented(predicate) = requirement.check else {
                continue;
            };
            let narrows = reach::exemption_narrows(requirement.clauses, *target);
            let entry = did.entry(requirement.id).or_default();
            if narrows {
                entry.insert(Did::Narrowable);
            }
            let mut whole = Findings::default();
            predicate(&exam, &mut whole);
            if whole.met() {
                continue;
            }
            entry.insert(Did::Failed);
            if !narrows || !whole.named_an_object() {
                entry.insert(Did::Stood);
                continue;
            }
            // Asking for the exempt population is what costs — two walks and a decode of every
            // associated content stream — so it is asked exactly where `crate::check` asks it:
            // only where a row the exemption narrows has already failed at a place naming an
            // object.
            let (exempt, stopped) = population.get_or_insert_with(|| {
                let computed = exam.exempt();
                (computed.shared(), computed.stopped_at().is_some())
            });
            if *stopped {
                tally.bounded = 1;
            }
            if exempt.is_empty() {
                entry.insert(Did::Stood);
                continue;
            }
            tally.with_exempt = 1;
            let mut narrowed = Findings::exempting(Arc::clone(exempt));
            predicate(&exam, &mut narrowed);
            if narrowed.seen() < whole.seen() {
                entry.insert(Did::Narrowed);
            }
            entry.insert(if narrowed.met() {
                Did::Cleared
            } else {
                Did::Stood
            });
        }
    }
    for (id, what) in did {
        let named = options.rows.iter().any(|wanted| wanted == id);
        let row = tally.rows.entry(id).or_default();
        row.narrowable = what.contains(&Did::Narrowable);
        row.failed = usize::from(what.contains(&Did::Failed));
        row.narrowed = usize::from(what.contains(&Did::Narrowed));
        row.cleared = usize::from(what.contains(&Did::Cleared));
        row.stood = usize::from(what.contains(&Did::Stood));
        if row.narrowed > 0 {
            row.witnesses.push(name.to_owned());
        }
        // A row named on the command line keeps every witness on both sides, which is what
        // prices a question about one rule; every other row keeps the first few, which is what
        // a table a person reads can hold.
        if named && row.failed > 0 {
            let side = if row.stood > 0 { "stood" } else { "cleared" };
            row.both.push(format!("{side}  {}", path.display()));
        }
    }
    Some(tally)
}

/// Prints one corpus's answer: what was read, what the exemption did, and what it passed over.
fn report(name: &str, tally: &Tally, options: &Options) {
    println!("\n== {name} ==");
    println!(
        "  {:>6} documents judged against {} target(s)",
        tally.documents,
        options.targets.len()
    );
    println!(
        "  {:>6} were asked for the exempt population and had one",
        tally.with_exempt
    );
    println!(
        "  {:>6} stopped a walk at a bound, so nothing in them is exempt",
        tally.bounded
    );
    println!("  {:>6} could not be opened", tally.unreadable);
    println!(
        "  {:>6} were larger than the size bound",
        tally.oversized.len()
    );
    println!("  {:>6} panicked under a predicate", tally.panicked.len());
    println!(
        "  {:>6} took longer than ten seconds to judge",
        tally.slow.len()
    );
    println!(
        "  {:>6} were skipped by name at the command line",
        tally.skipped.len()
    );
    withdrawn(tally);
    unjudged(tally);
    for id in &options.rows {
        named_row(tally, id);
    }
    for (label, names) in [
        ("oversized", &tally.oversized),
        ("panicked", &tally.panicked),
        ("slow", &tally.slow),
        ("skipped by name", &tally.skipped),
    ] {
        if !names.is_empty() {
            println!("\n  {label}:");
            for entry in names {
                println!("    {entry}");
            }
        }
    }
}

/// The rows the exemption withdrew a finding from, with what the audit says of each.
fn withdrawn(tally: &Tally) {
    let narrowed: Vec<(&&str, &Row)> = tally
        .rows
        .iter()
        .filter(|(_, row)| row.narrowed > 0)
        .collect();
    println!("\n  rows the exemption withdrew a finding from, in documents:");
    println!("  failed narrowed cleared  stood  requirement");
    for (id, row) in &narrowed {
        println!(
            "  {:>6} {:>8} {:>7} {:>6}  {id}  [{}]",
            row.failed,
            row.narrowed,
            row.cleared,
            row.stood,
            audited(id)
        );
        for witness in &row.witnesses {
            println!("           {witness}");
        }
    }
    if narrowed.is_empty() {
        println!("    none");
    }
}

/// The two ways a requirement goes unjudged over a corpus, which have nothing to do with each other.
///
/// The first is the corpus's doing — no document here states the fault at all — and the second is
/// the exemption's: every document that stated it had the finding withdrawn. They look identical
/// from inside a report and they are `CLAUDE.md`'s two denominators, so they are printed one after
/// the other and never added together.
fn unjudged(tally: &Tally) {
    let unexercised: Vec<&&str> = tally
        .rows
        .iter()
        .filter(|(_, row)| row.failed == 0)
        .map(|(id, _)| id)
        .collect();
    println!(
        "\n  rows no document in this corpus failed, before any narrowing ({} of {}):",
        unexercised.len(),
        tally.rows.len()
    );
    for id in &unexercised {
        println!("    {id}");
    }
    let never: Vec<&&str> = tally
        .rows
        .iter()
        .filter(|(_, row)| row.narrowable && row.failed > 0 && row.stood == 0)
        .map(|(id, _)| id)
        .collect();
    println!(
        "\n  rows every one of whose failures the exemption withdrew — judged by no document:"
    );
    for id in &never {
        println!("    {id}");
    }
    if never.is_empty() {
        println!("    none");
    }
}

/// One row `--row` named, with every witness and which side of the exemption it fell on.
fn named_row(tally: &Tally, id: &str) {
    let Some(row) = tally.rows.get(id) else {
        println!("\n  {id}: no document in this corpus failed it");
        return;
    };
    println!(
        "\n  {id}: failed on {} document(s), {} cleared by the exemption, {} standing",
        row.failed, row.cleared, row.stood
    );
    for witness in &row.both {
        println!("    {witness}");
    }
}

/// What `pdf_archive::withdrawal` says about the subclauses one row cites, for the report.
///
/// **The cross-check this example exists to make possible.** That module reads each subclause of
/// both parts and says whether the exemption can reach it at all; this walk says what it *did*
/// reach over a corpus. A row that was narrowed under a subclause the audit calls `NotAResource`
/// or `NoFileCanFail` is a contradiction — either the reading is wrong or a predicate is naming
/// an object its requirement is not about — so the verdict is printed beside every narrowed row
/// and `over-narrowing?` is printed where the two disagree.
fn audited(id: &str) -> String {
    let mut said = Vec::new();
    for requirement in table::requirements().filter(|requirement| requirement.id == id) {
        for (part, clause) in [
            (Part::Two, requirement.clauses.two),
            (Part::Four, requirement.clauses.four),
        ] {
            let Some(clause) = clause else {
                continue;
            };
            let verdict = withdrawal::reaches(part, clause);
            let word = match verdict {
                Some(Reaches::Resource(_)) => "a resource",
                Some(Reaches::NotAResource(_) | Reaches::NoFileCanFail(_)) => "over-narrowing?",
                Some(Reaches::Kept(_)) => "kept — and narrowed anyway?",
                None => "no entry in the audit",
            };
            said.push(format!("{part:?} {clause}: {word}"));
        }
    }
    said.join("; ")
}

/// Every PDF under a directory, in a stable order, following the symbolic links a corpus uses.
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
            } else if path
                .extension()
                .is_some_and(|kind| kind.eq_ignore_ascii_case("pdf"))
            {
                found.push(path);
            }
        }
    }
    found.sort();
    found
}

//! Every document in the pdf.js corpus, opened, interpreted and rasterised.
//!
//! # What this gate is for
//!
//! The other tests in this crate check that a feature works. This one checks that nothing
//! *breaks* — across 974 real documents produced by every generator anyone has pointed at
//! pdf.js over fifteen years, including a good number that are damaged, truncated or
//! deliberately hostile.
//!
//! Three things are asserted, and they are different in kind:
//!
//! 1. **Nothing panics.** A panic on untrusted input is a denial of service in a viewer
//!    and, in a crate that forbids unsafe code, the only way a malformed file can take the
//!    process down. Every failure must arrive as a typed error.
//! 2. **Nothing silently disappears.** A content stream that reaches an operator we do not
//!    implement must say so through [`pdf_model::Interpretation::unsupported`]. A viewer
//!    that draws nine tenths of a page and reports success is worse than one that admits
//!    what it left out, because nobody can tell from looking.
//! 3. **The numbers do not get worse.** The counts below are a ratchet. They are what the
//!    corpus produces today, and a change that raises any of them fails the build until
//!    the number is deliberately edited.
//!
//! # Why a ratchet rather than zero
//!
//! Some of these documents cannot be rendered by anything: they are fuzzer output and
//! truncation tests, present in pdf.js precisely to check that a reader refuses them
//! cleanly. Demanding zero failures would mean demanding that we render files with no
//! valid cross-reference table and no recoverable objects, which is not a coherent goal.
//! Demanding that the count never rises is coherent, and it catches the regression that
//! matters: a change that quietly stops handling a class of documents.
//!
//! # Running it
//!
//! The corpus is the `doc/pdf.js` submodule. When it is absent the test reports that and
//! passes, so a checkout without submodules is not a broken build — but CI has it, and the
//! ratchet only means anything where it runs.

#![expect(
    clippy::print_stdout,
    clippy::print_stderr,
    reason = "test code: an explanatory panic is the intended failure, and the survey \
              output is the point of the run"
)]

use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use pdf_render::{Rasterizer, TargetSpec};
use pdf_syntax::Document;
use rayon::prelude::{IntoParallelRefIterator, ParallelIterator};
use render_cpu::CpuRasterizer;

/// Pixel budget per page, generous enough that no real page reaches it.
const PIXEL_BUDGET: u64 = 64 << 20;

/// Documents that cannot be opened at all.
///
/// Zero, and it should stay zero: every file here yields *something*, even the fuzzed and
/// truncated ones, because recovery by scanning for `obj` headers works when no
/// cross-reference table does.
const MAX_UNOPENABLE: usize = 0;

/// Documents that open but whose first page cannot be reached.
///
/// Nineteen. Eleven are encrypted, which is unimplemented and listed as such in the
/// handover. The remaining eight are files whose page tree genuinely cannot be recovered.
///
/// This number is worth more than it looks. It was twenty until running this gate for the
/// first time: `outline_goto_action.pdf` declares twelve cross-reference entries and writes
/// eleven, so the twelfth read the `trailer` keyword, and resuming after that keyword
/// stepped over the only thing naming `/Root`. A document with every object intact produced
/// no catalogue and no pages. `pdf-syntax`'s robustness suite now pins it.
const MAX_PAGELESS: usize = 19;

/// Documents whose first page interprets with something reported as unsupported.
///
/// 281, and *not* a defect count — it is the honest-reporting requirement working. The
/// breakdown, which is the useful part:
///
/// | reported | count | why |
/// |---|---|---|
/// | `Image` | 152 | JBIG2 and JPX, deliberately blocked on the sandbox |
/// | `Text` | 73 | mostly CID encodings and embedded `CMap`s |
/// | `Shading` | 26 | soft masks in `/ExtGState`, which is transparency rather than shading |
/// | `Operator` | 19 | transparency groups |
/// | `Content` | 10 | a `/Contents` stream that did not decode |
/// | `LimitReached` | 1 | a bound reached and said so, which is the design |
///
/// This number went *up* by ten when content-stream decoding started reporting, and that
/// rise was the point. Nine of the ten are encrypted documents whose content stream is
/// unreadable without decryption, and they had been rendering as blank pages returning
/// `unsupported: []` — a wrong page indistinguishable from a sparse one. The tenth is
/// `bomb_giant.pdf`, refusing a decompression bomb, which is the design working.
///
/// Ratcheted downward otherwise: this falls as features land, and a rise that is not a new
/// *report* means something that used to draw no longer does.
const MAX_INCOMPLETE: usize = 281;

/// How long one document may take before it counts as a failure.
///
/// A viewer that takes half a minute to open a page has failed to open it. This bounds a
/// single document rather than the suite so that a failure names the file.
///
/// # This bound reports; it cannot enforce
///
/// The elapsed time is checked after the work finishes, because a Rust thread cannot be
/// cancelled from outside. A document that genuinely never returns hangs this test rather
/// than failing it. Bounding the work itself belongs inside the interpreter and the
/// rasteriser, which is where principle 3's "explicit time budgets" have to live; this is
/// the detector, not the guard. `cargo run --release -p pdf-model --example open_one` runs
/// one document in a process that *can* be killed, which is how a hang gets isolated.
const PER_DOCUMENT_BUDGET: Duration = Duration::from_secs(30);

/// Documents already known to exceed [`PER_DOCUMENT_BUDGET`], with the reason.
///
/// Named rather than counted, so that a new slow document fails the gate even though the
/// total has not risen — and so that fixing the cause deletes an entry rather than
/// decrementing a number nobody can interpret.
///
/// **Empty, and it earned that.** `bug1721218_reduced.pdf` was the only entry: a 612×792
/// page holding 3576 distinct clips, which rasterised in 39.6 s and held 1.7 GB. The CPU
/// backend now draws each command into the rows its clip admits rather than into the page
/// (ADR 0010), which takes it to 0.24 s and 25 MB of masks. Keeping the list empty is the
/// point: the next document to cross the budget fails the gate rather than joining a
/// list.
const KNOWN_SLOW: [&str; 0] = [];

/// What happened to one document.
#[derive(Debug, Default)]
struct Tally {
    unopenable: Vec<String>,
    pageless: Vec<String>,
    incomplete: Vec<(String, String)>,
    slow: Vec<(String, Duration)>,
}

/// Names a document on stderr when `PDFVIEWER_CORPUS_TRACE` is set.
///
/// Stderr rather than stdout because the test harness buffers stdout, and the whole value
/// of this is that it survives the run being killed.
fn trace(what: &str, name: &str) {
    if std::env::var_os("PDFVIEWER_CORPUS_TRACE").is_some() {
        eprintln!("{what} {name}");
    }
}

/// Adds to the shared tally, ignoring a poisoned lock.
///
/// A poisoned lock means another document's examination panicked, which the test as a
/// whole will report; losing one tally entry to it changes nothing.
fn record(tally: &Mutex<Tally>, update: impl FnOnce(&mut Tally)) {
    if let Ok(mut tally) = tally.lock() {
        update(&mut tally);
    }
}

/// The corpus files, or `None` when the submodule is not checked out.
fn corpus() -> Option<Vec<PathBuf>> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../doc/pdf.js/test/pdfs");
    let mut files: Vec<PathBuf> = std::fs::read_dir(&root)
        .ok()?
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| path.extension().is_some_and(|extension| extension == "pdf"))
        .collect();
    if files.is_empty() {
        return None;
    }
    files.sort();
    Some(files)
}

/// Opens, interprets and rasterises one document's first page.
///
/// Returns what went wrong, or nothing. Rasterisation is included because it is where a
/// display list with impossible geometry — an infinite coordinate, a degenerate transform —
/// would surface, and the interpreter is perfectly capable of producing one.
fn examine(path: &Path, tally: &Mutex<Tally>) {
    let name = path.file_name().map_or_else(
        || path.display().to_string(),
        |n| n.to_string_lossy().into(),
    );
    let started = Instant::now();
    // Named on stderr before and after, so that a document which never returns can be
    // identified from a killed run. There is no way to bound the work from outside: a
    // thread cannot be cancelled, so a genuinely unbounded loop hangs the suite and this
    // trace is the only thing that says which file caused it.
    trace("start", &name);

    let Ok(bytes) = std::fs::read(path) else {
        return;
    };
    let Ok(document) = Document::open(bytes) else {
        record(tally, |t| t.unopenable.push(name));
        return;
    };
    let Some(page) = pdf_model::Pages::new(&document).get(0) else {
        record(tally, |t| t.pageless.push(name));
        return;
    };

    let interpretation = pdf_model::interpret(&document, &page);
    if !interpretation.is_complete() {
        let reported = format!("{:?}", interpretation.unsupported);
        record(tally, |t| t.incomplete.push((name.clone(), reported)));
    }

    // A page whose extent cannot be targeted — empty, or larger than the budget — is a
    // reported outcome rather than a defect, so it is not counted.
    if let Ok(target) = TargetSpec::for_page(&interpretation.display_list, 1.0, PIXEL_BUDGET) {
        // The result is discarded deliberately: an unsupported command is a *reported*
        // outcome, already counted above. What this call is here to prove is that the
        // rasteriser returns rather than panicking or looping.
        drop(CpuRasterizer::new().rasterize(&interpretation.display_list, target));
    }

    let taken = started.elapsed();
    trace("done ", &name);
    if taken > PER_DOCUMENT_BUDGET {
        record(tally, |t| t.slow.push((name, taken)));
    }
}

/// The gate.
///
/// Ignored by default because it is a minute of work in release and fifteen in debug —
/// too slow to sit in the edit-test loop, and misleading there anyway, since the timing
/// bound is meaningless at debug speed. Run it deliberately:
///
/// ```text
/// cargo test --release -p pdf-model --test corpus -- --ignored --nocapture
/// ```
///
/// `PDFVIEWER_CORPUS_TRACE=1` additionally names each document on stderr as it starts and
/// finishes, which is how a document that never returns is identified from a killed run.
#[test]
#[ignore = "one minute over 974 documents; run explicitly, in release"]
fn the_corpus_opens_interprets_and_rasterises() {
    let Some(files) = corpus() else {
        println!("skipped: the doc/pdf.js submodule is not checked out");
        return;
    };

    let tally = Mutex::new(Tally::default());
    let started = Instant::now();
    files.par_iter().for_each(|path| examine(path, &tally));
    let elapsed = started.elapsed();

    let tally = tally.into_inner().expect("no examination panicked");

    println!(
        "{} documents in {:.1}s: {} unopenable, {} pageless, {} incomplete, {} slow",
        files.len(),
        elapsed.as_secs_f64(),
        tally.unopenable.len(),
        tally.pageless.len(),
        tally.incomplete.len(),
        tally.slow.len()
    );
    for (name, reported) in &tally.incomplete {
        println!("  incomplete: {name}: {reported}");
    }
    for name in tally.unopenable.iter().chain(&tally.pageless) {
        println!("  unusable: {name}");
    }
    for (name, taken) in &tally.slow {
        println!("  slow: {name}: {taken:?}");
    }

    let unexpected: Vec<&(String, Duration)> = tally
        .slow
        .iter()
        .filter(|(name, _)| !KNOWN_SLOW.contains(&name.as_str()))
        .collect();
    assert!(
        unexpected.is_empty(),
        "a document must not take longer than {PER_DOCUMENT_BUDGET:?} to open and draw: \
         {unexpected:?}"
    );
    assert!(
        tally.unopenable.len() == MAX_UNOPENABLE,
        "{} documents cannot be opened, was {MAX_UNOPENABLE}",
        tally.unopenable.len()
    );
    assert!(
        tally.pageless.len() <= MAX_PAGELESS,
        "{} documents have no reachable first page, was {MAX_PAGELESS}",
        tally.pageless.len()
    );
    assert!(
        tally.incomplete.len() <= MAX_INCOMPLETE,
        "{} documents draw incompletely, was {MAX_INCOMPLETE}",
        tally.incomplete.len()
    );
}

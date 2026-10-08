//! RFC 0008 section 6.7's `script_corpus` gate, its Tier 1 column: every document in the census
//! population that carries a field script Tier 0 does not run, opened through the view state's open
//! sequence and every such field committed, with the engine's realm supplied, and every outcome
//! counted by how it ended.
//!
//! # What this counts, and what it does not claim
//!
//! The population is Tier 0's gate's, read from the same list (`pdf-model`'s
//! `tests/support/script_population.rs`). For every document with a field whose `/K`, `/F`, `/V`
//! or `/C` is a script Tier 0 does not run, the walk runs the open sequence — Table 32's name tree,
//! the `/OpenAction`, page one's `/O` and its annotations' `/PO` — and then does what Tier 0's gate
//! does for each such field: its own value typed in, committed (so `/K`, `/V`, `/CO` and `/F` fire)
//! and its displayed value asked — and then Table 200's five, a save, a print and the close — with
//! one [`pdf_script::Engine`] for the document, so every event runs in the realm the open filled,
//! under [`Budget::FIELD_EVENT`]. Each run is counted in exactly
//! one of five columns: finished with no refusal, finished having been refused at least one call,
//! stopped by a budget, thrown, and unparsed; beside them, the runs by site, which members were
//! refused most, which names a `ReferenceError` found undefined, what the commonest uncaught throws
//! say, and the largest document-level library. A value is not held here — Tier 0's gate holds the
//! displayed values at the level `off` — because a Tier 1 value is this program's own output against
//! nothing, RFC 0008 section 8 item 5; what the columns say is how much of the world's scripts this
//! bridge can carry, and what to bridge next.
//!
//! # What it holds
//!
//! RFC 0008 section 6.7's three columns with a ratchet each, and a floor under the population: no
//! run over a budget ([`HELD_EXCEEDED`], held at zero, because a document that finishes under the
//! budgets of section 4.3 stays under them), and ceilings on the runs that threw, the runs refused a
//! call, and the scripts that do not parse; and at least [`HELD_RUNS`] runs, so that a walk which
//! reached fewer documents cannot pass the ceilings by running less. The three are ceilings rather
//! than equalities because the bridge grows every batch and each member bridged moves them; a
//! figure below its ceiling is printed so the ceiling is lowered with it. `tools/batch.sh gates`
//! runs this as `t2-script_corpus_engine`, building the `engine` feature for its own test binary
//! and for nothing a person runs (ADR 1625).
//!
//! # Running it
//!
//! ```text
//! RAYON_NUM_THREADS=4 tools/bounded.sh --lock --round <session> --data 8 --tree 12 -- \
//!     cargo test --profile gates -p pdf-script --features engine --test script_corpus -- \
//!     --ignored --nocapture
//! ```

// no sandbox worker: the walk reads field dictionaries, sets and commits field values and asks what each displays; no content stream is interpreted, so no image reaches `pdf-sandbox`.

#![cfg(feature = "engine")]
#![expect(
    clippy::print_stdout,
    reason = "test code: the report is the point of the run"
)]

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;

use pdf_model::aform::Trigger;
use pdf_model::aform::site::{self, Site};
use pdf_model::view::{
    DocumentTrigger, Entered, ScriptEvent, ScriptResult, ScriptRunner, ScriptSite, ViewState,
    widgets_by_field_name,
};
use pdf_script::surface::NOT_BRIDGED;
use pdf_script::{Budget, Ending, Engine, RefusalKind, Request};
use pdf_syntax::{Document, Limits};
use rayon::prelude::{IntoParallelRefIterator, ParallelIterator};

#[path = "../../pdf-model/tests/support/corpus_passwords.rs"]
#[expect(
    dead_code,
    reason = "the references' spelling of a password is the oracle's; this gate opens the \
              document itself and has no reference to hand one to"
)]
mod corpus_passwords;

#[path = "../../pdf-model/tests/support/script_population.rs"]
mod script_population;

use script_population::{MAX_FILE_BYTES, password_for, population, repository};

/// Runs a budget may stop: none (RFC 0008 section 6.7, ADR 1625).
const HELD_EXCEEDED: usize = 0;

/// Most runs that may end in an uncaught throw.
const HELD_THREW: usize = 9_317;

/// Most runs that may finish having been refused a call: none, since a write to `this.pageNum` —
/// the one refusal three scripts caught — is a page turn the host makes (ADR 1640).
const HELD_FINISHED_REFUSED: usize = 0;

/// Most runs whose script may not parse.
const HELD_UNPARSED: usize = 8;

/// Fewest runs the walk may hand the engine.
const HELD_RUNS: usize = 21_101;

/// One refused member's runs, the documents they were in, and whether the refusal was of an
/// admitted member the bridge does not carry.
type Refused = (usize, BTreeSet<String>, bool);

/// How the runs ended, counted across the walk.
#[derive(Debug, Default)]
struct Tally {
    /// Runs that finished and were refused nothing.
    finished: AtomicUsize,
    /// Runs that finished having been refused at least one call.
    finished_refused: AtomicUsize,
    /// Runs a budget stopped.
    exceeded: AtomicUsize,
    /// Runs that threw.
    threw: AtomicUsize,
    /// Runs whose script does not parse.
    unparsed: AtomicUsize,
    /// Every refusal, by member, with how many runs, which documents, and whether it was an
    /// admitted member the bridge does not carry.
    members: Mutex<BTreeMap<String, Refused>>,
    /// Every budget a run exceeded, by its sentence's kind.
    budgets: Mutex<BTreeMap<String, usize>>,
    /// Every uncaught throw, by its first words, with how many runs and which documents.
    throws: Mutex<BTreeMap<String, (usize, BTreeSet<String>)>>,
    /// Every uncaught throw, by the document it was thrown in, with the commonest first words
    /// there: what decides whether a class is one form's slip or the world's habit.
    throwing_documents: Mutex<BTreeMap<String, BTreeMap<String, usize>>>,
    /// Every uncaught throw, by the error's name.
    kinds: Mutex<BTreeMap<String, usize>>,
    /// Every name a `ReferenceError` found undefined, with how many runs and which documents.
    undefined: Mutex<BTreeMap<String, (usize, BTreeSet<String>)>>,
    /// Every uncaught throw, by the site it was thrown at.
    thrown_at: Mutex<BTreeMap<String, usize>>,
    /// Every run, by its site.
    sites: Mutex<BTreeMap<String, usize>>,
    /// The longest document-level script a run was handed, in bytes, and the document's name.
    library: Mutex<(usize, String)>,
    /// The deepest stack `pdf_script::depth` estimated for a script, in bytes, and the document's
    /// name (ADR 1626).
    deepest: Mutex<(u64, String)>,
    /// Runs that put a question, which no face here answers (ADR 1627).
    asked: AtomicUsize,
}

/// A runner that runs one document's realm and counts each outcome.
#[derive(Debug)]
struct Counting {
    /// The counts.
    tally: Arc<Tally>,
    /// The document's realm.
    engine: Engine,
    /// The document's file name, for the largest library.
    document: String,
    /// Whether each throw is printed with its site, field and script: one document's reading.
    print_throws: bool,
}

impl ScriptRunner for Counting {
    fn run(&self, event: &ScriptEvent<'_>) -> ScriptResult {
        let request = Request::of(event, 1_704_465_015_000, 0);
        let estimated = pdf_script::depth::estimate(event.script);
        if let Ok(mut deepest) = self.tally.deepest.lock()
            && estimated > deepest.0
        {
            *deepest = (estimated, self.document.clone());
        }
        let outcome = self.engine.run_request(&request);
        if outcome
            .notes
            .iter()
            .any(|note| note.starts_with("the script asked"))
        {
            self.tally.asked.fetch_add(1, Ordering::Relaxed);
        }
        if let Ok(mut sites) = self.tally.sites.lock() {
            let site = format!("{:?}", event.site);
            let held = sites.entry(site).or_default();
            *held = held.saturating_add(1);
        }
        if event.site == ScriptSite::Library
            && let Ok(mut largest) = self.tally.library.lock()
            && event.script.len() > largest.0
        {
            *largest = (event.script.len(), self.document.clone());
        }
        let column = match &outcome.ending {
            Ending::Finished if outcome.refusals.is_empty() => &self.tally.finished,
            Ending::Finished => &self.tally.finished_refused,
            Ending::Exceeded(exceeded) => {
                if let Ok(mut budgets) = self.tally.budgets.lock() {
                    let kind = format!("{exceeded:?}");
                    let kind = kind.split(['(', ' ']).next().unwrap_or_default().to_owned();
                    let held = budgets.entry(kind).or_default();
                    *held = held.saturating_add(1);
                }
                &self.tally.exceeded
            }
            Ending::Threw(thrown) | Ending::Declined(thrown) => {
                if self.print_throws {
                    let script: String = event.script.chars().take(400).collect();
                    println!(
                        "--- {:?} on {:?}: {}\n{script}",
                        event.site,
                        event.field,
                        thrown.lines().take(6).collect::<Vec<_>>().join(" / ")
                    );
                }
                let words: String = thrown.chars().take(80).collect();
                if let Ok(mut throws) = self.tally.throws.lock() {
                    let held = throws.entry(words.clone()).or_default();
                    held.0 = held.0.saturating_add(1);
                    held.1.insert(self.document.clone());
                }
                if let Ok(mut documents) = self.tally.throwing_documents.lock() {
                    let held = documents
                        .entry(self.document.clone())
                        .or_default()
                        .entry(words)
                        .or_default();
                    *held = held.saturating_add(1);
                }
                if let Ok(mut kinds) = self.tally.kinds.lock() {
                    let kind = thrown.split(':').next().unwrap_or_default().to_owned();
                    let held = kinds.entry(kind).or_default();
                    *held = held.saturating_add(1);
                }
                if let Ok(mut thrown_at) = self.tally.thrown_at.lock() {
                    let held = thrown_at.entry(format!("{:?}", event.site)).or_default();
                    *held = held.saturating_add(1);
                }
                if let Some((name, _)) = thrown
                    .strip_prefix("ReferenceError: ")
                    .and_then(|rest| rest.split_once(" is not defined"))
                    && let Ok(mut undefined) = self.tally.undefined.lock()
                {
                    let held = undefined.entry(name.to_owned()).or_default();
                    held.0 = held.0.saturating_add(1);
                    held.1.insert(self.document.clone());
                }
                &self.tally.threw
            }
            Ending::Unparsed(_) => &self.tally.unparsed,
        };
        column.fetch_add(1, Ordering::Relaxed);
        if let Ok(mut members) = self.tally.members.lock() {
            for refusal in &outcome.refusals {
                let held = members.entry(refusal.member.clone()).or_default();
                held.0 = held.0.saturating_add(1);
                held.1.insert(self.document.clone());
                held.2 = refusal.kind == RefusalKind::NotBridged;
            }
        }
        outcome.result()
    }
}

/// Runs the open sequence and every field script of one document that Tier 0 does not, answering
/// how many fields had one; `print_throws` prints each throw with its script.
fn examine(path: &Path, tally: &Arc<Tally>, print_throws: bool) -> usize {
    if std::fs::metadata(path).is_ok_and(|meta| meta.len() > MAX_FILE_BYTES) {
        return 0;
    }
    let Ok(bytes) = std::fs::read(path) else {
        return 0;
    };
    let Ok(document) = Document::open_with_password(bytes, Limits::DEFAULT, password_for(path))
    else {
        return 0;
    };
    let table = widgets_by_field_name(&document);
    let scripted: Vec<&String> = table
        .iter()
        .filter(|(_, widgets)| {
            widgets.first().is_some_and(|first| {
                document.get(*first).as_dict().is_some_and(|widget| {
                    [
                        Trigger::Keystroke,
                        Trigger::Format,
                        Trigger::Validate,
                        Trigger::Calculate,
                    ]
                    .into_iter()
                    .any(|trigger| {
                        matches!(site::of_widget(&document, widget, trigger), Site::NotRun(_))
                    })
                })
            })
        })
        .map(|(name, _)| name)
        .collect();
    if scripted.is_empty() {
        return 0;
    }
    let mut view = ViewState::of(&document);
    view.run_scripts_with(Some(Arc::new(Counting {
        tally: Arc::clone(tally),
        engine: Engine::new(Budget::FIELD_EVENT),
        document: path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default(),
        print_throws,
    })));
    view.run_open_scripts(&document, 0);
    for name in &scripted {
        let Some(shown) = view.field_value(&document, name) else {
            continue;
        };
        if shown.obscured {
            continue;
        }
        view.set_field(&document, name, &Entered::Text(shown.text));
        view.commit_field(&document, name);
        let _ = view.displayed_value(&document, name);
    }
    // Table 200's five, in the order a reader's session meets them: a save, a print, the close.
    for trigger in [
        DocumentTrigger::WillSave,
        DocumentTrigger::DidSave,
        DocumentTrigger::WillPrint,
        DocumentTrigger::DidPrint,
        DocumentTrigger::WillClose,
    ] {
        view.run_document_scripts(&document, trigger, 0);
    }
    if print_throws {
        // What a reader is told: the view state's report, one sentence each, repeats folded.
        let reports = view.script_reports();
        println!("{} report sentence(s); the first:", reports.len());
        for sentence in reports.iter().take(6) {
            let sentence: String = sentence.chars().take(240).collect();
            println!("  {sentence}");
        }
    }
    scripted.len()
}

#[test]
#[ignore = "walks the census population; run behind the heavy-walk lock"]
fn every_script_tier_0_does_not_run_is_run_in_its_document_s_realm_and_counted() {
    let started = Instant::now();
    let root = repository();
    let files = population(&root);
    let tally = Arc::new(Tally::default());
    let fields: usize = files
        .par_iter()
        .map(|path| examine(path, &tally, false))
        .sum();
    let count = |column: &AtomicUsize| column.load(Ordering::Relaxed);
    let runs = [
        &tally.finished,
        &tally.finished_refused,
        &tally.exceeded,
        &tally.threw,
        &tally.unparsed,
    ]
    .into_iter()
    .map(count)
    .fold(0_usize, usize::saturating_add);
    println!(
        "{} PDF(s) walked; {fields} field(s) with a /K, /F, /V or /C Tier 0 does not run; {runs} engine \
         run(s): {} finished, {} finished refused, {} over a budget, {} threw, {} unparsed; {} s",
        files.len(),
        count(&tally.finished),
        count(&tally.finished_refused),
        count(&tally.exceeded),
        count(&tally.threw),
        count(&tally.unparsed),
        started.elapsed().as_secs()
    );
    if let Ok(budgets) = tally.budgets.lock() {
        println!("budgets exceeded: {budgets:?}");
    }
    if let Ok(sites) = tally.sites.lock() {
        println!("runs by site: {sites:?}");
    }
    if let Ok(largest) = tally.library.lock() {
        println!(
            "largest document-level script: {} bytes, in {}",
            largest.0, largest.1
        );
    }
    depth_and_questions(&tally);
    if let Ok(thrown_at) = tally.thrown_at.lock() {
        println!("throws by site: {thrown_at:?}");
    }
    if let Ok(undefined) = tally.undefined.lock() {
        let mut ranked: Vec<(&String, &(usize, BTreeSet<String>))> = undefined.iter().collect();
        ranked.sort_by(|left, right| {
            right
                .1
                .1
                .len()
                .cmp(&left.1.1.len())
                .then(right.1.0.cmp(&left.1.0))
                .then(left.0.cmp(right.0))
        });
        let total: usize = undefined.values().map(|(runs, _)| runs).sum();
        let documents: BTreeSet<&String> = undefined
            .values()
            .flat_map(|(_, documents)| documents)
            .collect();
        println!(
            "ReferenceError: {total} run(s) in {} document(s), {} distinct name(s)",
            documents.len(),
            undefined.len()
        );
        for (name, (runs, documents)) in ranked.iter().take(25) {
            let first = documents.iter().next().map_or("", String::as_str);
            println!(
                "undefined {:>4} document(s) {runs:>6} run(s)  {name}  (first: {first})",
                documents.len()
            );
        }
    }
    if let Ok(kinds) = tally.kinds.lock() {
        println!("throws by name: {kinds:?}");
    }
    if let Ok(members) = tally.members.lock() {
        let mut ranked: Vec<_> = members.iter().collect();
        ranked.sort_by(|left, right| right.1.0.cmp(&left.1.0).then(left.0.cmp(right.0)));
        for (member, (refused, documents, _)) in ranked.iter().take(25) {
            println!(
                "refused {refused:>6} run(s) {:>4} document(s)  {member}",
                documents.len()
            );
        }
        not_bridged(&members);
    }
    throws_by_cause(&tally);
    assert!(
        fields == 0 || runs > 0,
        "the hook handed none of {fields} field script(s) to the engine"
    );
    hold(&tally, runs);
}

/// The census of what RFC 0008 section 4.2 admits and the bridge does not carry: every member of
/// [`NOT_BRIDGED`], with the runs and documents that reached it — a zero is printed, so that a
/// member no document reaches is seen not to be reached — and any `NotBridged` refusal of a member
/// the list does not hold, which a refusal raised outside the list would be (ADR 1689).
fn not_bridged(members: &BTreeMap<String, Refused>) {
    let listed: BTreeSet<String> = NOT_BRIDGED
        .iter()
        .flat_map(|(holder, members)| {
            members
                .iter()
                .map(move |member| format!("{}{member}", holder.prefix()))
        })
        .collect();
    let mut reached = 0_usize;
    for member in &listed {
        let (runs, documents) = members
            .get(member)
            .map_or((0, 0), |(runs, documents, _)| (*runs, documents.len()));
        if runs > 0 {
            reached = reached.saturating_add(1);
        }
        let first = members
            .get(member)
            .and_then(|(_, documents, _)| documents.iter().next())
            .map_or("", String::as_str);
        println!("not bridged {runs:>6} run(s) {documents:>4} document(s)  {member}  {first}");
    }
    println!(
        "not bridged: {reached} of {} listed member(s) reached",
        listed.len()
    );
    for (member, (runs, documents, kind)) in members {
        if *kind && !listed.contains(member) {
            println!(
                "not bridged, outside the list {runs:>6} run(s) {:>4} document(s)  {member}",
                documents.len()
            );
        }
    }
}

/// Prints the commonest uncaught throws with the documents they were thrown in, and the documents
/// that threw most with what each threw most: a throw counted per run says how often a reader meets
/// it, and counted per document whether it is one form's or many forms'.
fn throws_by_cause(tally: &Tally) {
    if let Ok(throws) = tally.throws.lock() {
        let mut ranked: Vec<(&String, &(usize, BTreeSet<String>))> = throws.iter().collect();
        ranked.sort_by(|left, right| right.1.0.cmp(&left.1.0).then(left.0.cmp(right.0)));
        for (thrown, (count, documents)) in ranked.iter().take(40) {
            let first = documents.iter().next().map_or("", String::as_str);
            println!(
                "threw {count:>6} in {:>4} document(s)  {thrown}  (first: {first})",
                documents.len()
            );
        }
    }
    if let Ok(documents) = tally.throwing_documents.lock() {
        let mut ranked: Vec<(usize, &str, &str, usize)> = documents
            .iter()
            .map(|(document, throws)| {
                let total = throws
                    .values()
                    .copied()
                    .fold(0_usize, usize::saturating_add);
                let (words, count) = throws
                    .iter()
                    .max_by(|left, right| left.1.cmp(right.1).then(right.0.cmp(left.0)))
                    .map_or(("", 0), |(words, count)| (words.as_str(), *count));
                (total, document.as_str(), words, count)
            })
            .collect();
        ranked.sort_by(|left, right| right.0.cmp(&left.0).then(left.1.cmp(right.1)));
        println!("documents that threw: {}", ranked.len());
        for (total, document, words, count) in ranked.iter().take(25) {
            let words = words.lines().next().unwrap_or_default();
            println!("document {total:>6}  {document}  ({count} of them: {words})");
        }
    }
}

/// Prints the deepest script's estimate and the runs that put a question (ADRs 1626, 1627).
fn depth_and_questions(tally: &Tally) {
    if let Ok(deepest) = tally.deepest.lock() {
        println!(
            "deepest script: {} KiB of stack estimated against a budget of {} KiB, in {}",
            deepest.0 >> 10,
            Budget::FIELD_EVENT.depth >> 10,
            deepest.1
        );
    }
    println!(
        "runs that put a question, answered as a closed dialogue answers: {}",
        tally.asked.load(Ordering::Relaxed)
    );
}

/// Holds the walk's endings to the column's ceilings and its runs to the floor (ADR 1625), printing
/// the held figures and each one the walk has moved below them.
fn hold(tally: &Tally, runs: usize) {
    let count = |column: &AtomicUsize| column.load(Ordering::Relaxed);
    let held = [
        ("over a budget", count(&tally.exceeded), HELD_EXCEEDED),
        ("threw", count(&tally.threw), HELD_THREW),
        (
            "finished refused",
            count(&tally.finished_refused),
            HELD_FINISHED_REFUSED,
        ),
        ("unparsed", count(&tally.unparsed), HELD_UNPARSED),
    ];
    println!(
        "held: {HELD_EXCEEDED} over a budget, {HELD_THREW} threw, {HELD_FINISHED_REFUSED} finished \
         refused, {HELD_UNPARSED} unparsed at most; {HELD_RUNS} run(s) at least (ADR 1625)"
    );
    for (column, now, ceiling) in held {
        if now < ceiling {
            println!("ratchet: {column} fell from {ceiling} to {now}; lower its ceiling to it");
        }
    }
    if runs > HELD_RUNS {
        println!("ratchet: the runs rose from {HELD_RUNS} to {runs}; raise the floor to it");
    }
    let over: Vec<String> = held
        .iter()
        .filter(|(_, now, ceiling)| now > ceiling)
        .map(|(column, now, ceiling)| format!("{column}: {now} against {ceiling}"))
        .collect();
    assert!(
        over.is_empty() && runs >= HELD_RUNS,
        "the Tier 1 column moved past what it holds — a run a budget stopped, a script that now \
         throws or is refused, or fewer runs ({runs} against {HELD_RUNS}): {over:?}. Read the \
         throws and refusals above before moving a figure (ADR 1625)"
    );
}

/// One document's throws, each printed with its site, its field and its script, as the column runs
/// them — what a class in the column's ranking is read from (ADR 1641):
///
/// ```text
/// PDF_SCRIPT_THROWS_DOCUMENT=<path> cargo test --profile gates -p pdf-script --features engine \
///     --test script_corpus one_document -- --ignored --nocapture
/// ```
#[test]
#[ignore = "reads one document named by PDF_SCRIPT_THROWS_DOCUMENT"]
fn one_document_s_throws_are_printed_with_their_scripts() {
    let Some(path) = std::env::var_os("PDF_SCRIPT_THROWS_DOCUMENT") else {
        println!("no document named: set PDF_SCRIPT_THROWS_DOCUMENT to a path");
        return;
    };
    let tally = Arc::new(Tally::default());
    let fields = examine(Path::new(&path), &tally, true);
    println!(
        "{fields} scripted field(s); {} run(s) threw",
        tally.threw.load(Ordering::Relaxed)
    );
}

/// A runner that keeps every request it is handed and runs none, so that a document's requests can
/// be replayed into a realm measured on its own.
#[derive(Debug, Default)]
struct Capturing {
    /// Every request, in order.
    requests: Mutex<Vec<Request>>,
}

impl ScriptRunner for Capturing {
    fn run(&self, event: &ScriptEvent<'_>) -> ScriptResult {
        if let Ok(mut requests) = self.requests.lock() {
            requests.push(Request::of(event, 1_704_465_015_000, 0));
        }
        ScriptResult {
            rc: true,
            value: None,
            change: None,
            edits: Vec::new(),
            report: Vec::new(),
        }
    }
}

/// This process's resident set and its high-water mark, in kibibytes, from `/proc/self/status`.
fn resident() -> (u64, u64) {
    let status = std::fs::read_to_string("/proc/self/status").unwrap_or_default();
    let read = |key: &str| {
        status
            .lines()
            .find_map(|line| line.strip_prefix(key))
            .and_then(|rest| rest.split_whitespace().next())
            .and_then(|kib| kib.parse::<u64>().ok())
            .unwrap_or_default()
    };
    (read("VmRSS:"), read("VmHWM:"))
}

/// The ninth budget's measurement (ADR 1602): one document's requests — the open sequence and one
/// commit of each scripted field — captured with no engine, the document dropped, and then
/// replayed into one realm, the process's high-water mark read before and after. One document per
/// process, so that no earlier realm's freed heap is what this one reuses:
///
/// ```text
/// PDF_SCRIPT_REALM_DOCUMENT=<path> cargo test --profile gates -p pdf-script --features engine \
///     --test script_corpus a_realm -- --ignored --nocapture
/// ```
#[test]
#[ignore = "measures one document named by PDF_SCRIPT_REALM_DOCUMENT; run one process per document"]
fn a_realm_s_heap_is_measured_for_one_document() {
    let Some(path) = std::env::var_os("PDF_SCRIPT_REALM_DOCUMENT") else {
        println!("no document named: set PDF_SCRIPT_REALM_DOCUMENT to a path");
        return;
    };
    let path = Path::new(&path);
    let requests = {
        let Ok(bytes) = std::fs::read(path) else {
            println!("{} does not read", path.display());
            return;
        };
        let Ok(document) = Document::open_with_password(bytes, Limits::DEFAULT, password_for(path))
        else {
            println!("{} does not open", path.display());
            return;
        };
        let capturing = Arc::new(Capturing::default());
        let mut view = ViewState::of(&document);
        view.run_scripts_with(Some(capturing.clone()));
        view.run_open_scripts(&document, 0);
        for name in widgets_by_field_name(&document).keys() {
            if let Some(shown) = view.field_value(&document, name)
                && !shown.obscured
            {
                view.set_field(&document, name, &Entered::Text(shown.text));
                view.commit_field(&document, name);
            }
        }
        capturing
            .requests
            .lock()
            .map(|requests| requests.clone())
            .unwrap_or_default()
    };
    let script_bytes: usize = requests.iter().map(|request| request.script.len()).sum();
    let (_, before) = resident();
    let started = Instant::now();
    let mut realm = match pdf_script::Realm::new(Budget::FIELD_EVENT) {
        Ok(realm) => realm,
        Err(why) => {
            println!("no realm: {why}");
            return;
        }
    };
    let mut ended: BTreeMap<String, usize> = BTreeMap::new();
    for request in &requests {
        let outcome = realm.run(request);
        let kind = format!("{:?}", outcome.ending);
        let kind = kind.split(['(', ' ']).next().unwrap_or_default().to_owned();
        let held = ended.entry(kind).or_default();
        *held = held.saturating_add(1);
    }
    let (_, after) = resident();
    println!(
        "{}: {} request(s), {script_bytes} byte(s) of script, {ended:?}; high-water mark {before} \
         -> {after} KiB, {} KiB for the realm, {} ms",
        path.display(),
        requests.len(),
        after.saturating_sub(before),
        started.elapsed().as_millis()
    );
}

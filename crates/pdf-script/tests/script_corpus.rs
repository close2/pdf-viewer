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
//! and its displayed value asked, with one [`pdf_script::Engine`] for the document, so every event
//! runs in the realm the open filled, under [`Budget::FIELD_EVENT`]. Each run is counted in exactly
//! one of five columns: finished with no refusal, finished having been refused at least one call,
//! stopped by a budget, thrown, and unparsed; beside them, the runs by site, which members were
//! refused most, which names a `ReferenceError` found undefined, what the commonest uncaught throws
//! say, and the largest document-level library. A value is not held here — Tier 0's gate holds the
//! displayed values at the level `off` — because a Tier 1 value is this program's own output against
//! nothing, RFC 0008 section 8 item 5; what the columns say is how much of the world's scripts this
//! bridge can carry, and what to bridge next.
//!
//! # Running it
//!
//! ```text
//! RAYON_NUM_THREADS=4 flock /home/AI/heavy-walk.lock tools/bounded.sh --data 8 --tree 12 -- \
//!     cargo test --profile gates -p pdf-script --features engine --test script_corpus -- \
//!     --ignored --nocapture
//! ```

// no sandbox worker: the walk reads field dictionaries, sets and commits field values and asks what each displays; no content stream is interpreted, so no image reaches `pdf-sandbox`.
// not a gate: a census of what the engine behind `pdf-script`'s default-off `engine` feature makes of the world's scripts, ranking what to bridge next; no build `tools/batch.sh gates` makes turns the feature on, and nothing in it is held to a number (RFC 0008 section 6.7, ADR 1602).

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
    Entered, ScriptEvent, ScriptResult, ScriptRunner, ScriptSite, ViewState, widgets_by_field_name,
};
use pdf_script::{Budget, Ending, Engine, Request};
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
    /// Every refusal, by member.
    members: Mutex<BTreeMap<String, usize>>,
    /// Every budget a run exceeded, by its sentence's kind.
    budgets: Mutex<BTreeMap<String, usize>>,
    /// Every uncaught throw, by its first words.
    throws: Mutex<BTreeMap<String, usize>>,
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
}

impl ScriptRunner for Counting {
    fn run(&self, event: &ScriptEvent<'_>) -> ScriptResult {
        let request = Request::of(event, 1_704_465_015_000, 0);
        let outcome = self.engine.run_request(&request);
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
                if let Ok(mut throws) = self.tally.throws.lock() {
                    let words: String = thrown.chars().take(60).collect();
                    let held = throws.entry(words).or_default();
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
                *held = held.saturating_add(1);
            }
        }
        outcome.result()
    }
}

/// Runs the open sequence and every field script of one document that Tier 0 does not, answering
/// how many fields had one.
fn examine(path: &Path, tally: &Arc<Tally>) -> usize {
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
    scripted.len()
}

#[test]
#[ignore = "walks the census population; run behind the heavy-walk lock"]
fn every_script_tier_0_does_not_run_is_run_in_its_document_s_realm_and_counted() {
    let started = Instant::now();
    let root = repository();
    let files = population(&root);
    let tally = Arc::new(Tally::default());
    let fields: usize = files.par_iter().map(|path| examine(path, &tally)).sum();
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
        let mut ranked: Vec<(&String, &usize)> = members.iter().collect();
        ranked.sort_by(|left, right| right.1.cmp(left.1).then(left.0.cmp(right.0)));
        for (member, refused) in ranked.iter().take(25) {
            println!("refused {refused:>6}  {member}");
        }
    }
    if let Ok(throws) = tally.throws.lock() {
        let mut ranked: Vec<(&String, &usize)> = throws.iter().collect();
        ranked.sort_by(|left, right| right.1.cmp(left.1).then(left.0.cmp(right.0)));
        for (thrown, count) in ranked.iter().take(25) {
            println!("threw {count:>6}  {thrown}");
        }
    }
    assert!(
        fields == 0 || runs > 0,
        "the hook handed none of {fields} field script(s) to the engine"
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

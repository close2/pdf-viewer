//! RFC 0008 section 6.7's `script_corpus` gate, its Tier 1 column: every field `/K` and `/F` in the
//! census population that Tier 0 reports as not run, handed to the engine through the view state's
//! hook, and every outcome counted by how it ended.
//!
//! # What this counts, and what it does not claim
//!
//! The population is Tier 0's gate's, read from the same list (`pdf-model`'s
//! `tests/support/script_population.rs`). For every field whose `/K` or `/F` is a script Tier 0
//! does not run, the walk does what Tier 0's gate does — the field's own value typed in, committed,
//! and its displayed value asked — with [`pdf_script::Engine`] supplied, so every event the hook
//! hands over is one the engine runs under [`Budget::FIELD_EVENT`]. Each run is counted in exactly
//! one of five columns: finished with no refusal, finished having been refused at least one call,
//! stopped by a budget, thrown, and unparsed; beside them, which members were refused most and
//! what the commonest uncaught throws say. A value is not held here — Tier 0's gate holds the displayed values at the
//! level `off` — because a Tier 1 value is this program's own output against nothing, RFC 0008
//! section 8 item 5; what the columns say is how much of the world's field scripts this bridge can
//! carry, and what to bridge next.
//!
//! # Running it
//!
//! ```text
//! RAYON_NUM_THREADS=4 flock /home/AI/heavy-walk.lock tools/bounded.sh --data 8 --tree 12 -- \
//!     cargo test --profile gates -p pdf-script --features engine --test script_corpus -- \
//!     --ignored --nocapture
//! ```

// no sandbox worker: the walk reads field dictionaries, sets and commits field values and asks what each displays; no content stream is interpreted, so no image reaches `pdf-sandbox`.

#![cfg(feature = "engine")]
#![expect(
    clippy::print_stdout,
    reason = "test code: the report is the point of the run"
)]

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;

use pdf_model::aform::Trigger;
use pdf_model::aform::site::{self, Site};
use pdf_model::view::{
    Entered, FieldEvent, FieldResult, ScriptRunner, ViewState, widgets_by_field_name,
};
use pdf_script::{Budget, Ending, Event, Request, utf16_offset};
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
}

/// A runner that runs the engine and counts each outcome.
#[derive(Debug)]
struct Counting {
    /// The counts.
    tally: Arc<Tally>,
}

impl ScriptRunner for Counting {
    fn run(&self, event: &FieldEvent<'_>) -> FieldResult {
        let request = Request {
            trigger: event.trigger,
            field: event.field.to_owned(),
            script: event.script.to_owned(),
            event: Event {
                value: event.value.to_owned(),
                change: event.change.to_owned(),
                selection_start: utf16_offset(event.value, event.selection.0),
                selection_end: utf16_offset(event.value, event.selection.1),
                will_commit: event.will_commit,
            },
            moment: 1_704_465_015_000,
            utc_offset_seconds: 0,
        };
        let outcome = pdf_script::run(&request, &Budget::FIELD_EVENT);
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
        FieldResult {
            rc: outcome.rc,
            value: outcome.value.clone(),
            change: outcome.change.clone(),
            report: outcome.sentences(),
        }
    }
}

/// Runs every field script of one document that Tier 0 does not, answering how many fields had one.
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
                    [Trigger::Keystroke, Trigger::Format]
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
    })));
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
fn every_field_script_tier_0_does_not_run_is_run_and_counted() {
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
        "{} PDF(s) walked; {fields} field(s) with a /K or /F Tier 0 does not run; {runs} engine \
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

//! RFC 0008 section 6.7's Tier 1 column run once through the confined worker rather than in this
//! process: the walk `pdf-script`'s `tests/script_corpus.rs` makes — every census document with a
//! field script Tier 0 does not run, its open sequence and then each such field typed, committed
//! and asked what it displays — with a [`ScriptWorker`] as the runner, one per document, so every
//! script runs in `pdf-script-worker` under `pdf_sandbox::lockdown::Profile::Script` (ADRs 1608,
//! 1609). Then the events a window raises (ADR 1750): on each of the first [`MAX_TURNS`] pages,
//! every annotation's Table 197 pointer and focus events in the order a click and a departure
//! raise them, and the page turned to the next, so Table 198's `/C` and `/O` and Table 197's four
//! page events run through the worker as they run in a window.
//!
//! # What this counts
//!
//! The worker deaths, by cause — the deadline, a death, a garbled answer — and among the deaths
//! the ones by `SIGSYS`, which is the filter's kill: a system call the profile does not admit and
//! the bridged object model reached. A `SIGSYS` death is a defect of the profile or of the model,
//! never of the document, so the column says how many there were and the first few by document and
//! trigger; the profile then gains the call with its reason, or the model loses the member (ADR
//! 1615). Beside the deaths, the runs are counted in the in-process column's five columns, read off
//! each run's sentences, so the two columns can be read against each other.
//!
//! # Running it
//!
//! ```text
//! RAYON_NUM_THREADS=4 tools/bounded.sh --lock --round <session> --data 8 --tree 12 -- \
//!     cargo test --profile gates -p pdf-script-worker --features engine --test script_column -- \
//!     --ignored --nocapture
//! ```

// no sandbox worker: the walk reads field dictionaries, sets and commits field values and asks what each displays; no content stream is interpreted, so no image reaches `pdf-sandbox`.
// not a gate: a census of what the confined worker makes of the world's scripts, counting the profile's kills; no build `tools/batch.sh gates` makes turns the feature on, and nothing in it is held to a number (RFC 0008 section 6.7, ADR 1615).

#![cfg(feature = "engine")]
#![expect(
    clippy::print_stdout,
    reason = "test code: the report is the point of the run"
)]

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use pdf_model::action::{PageTrigger, Trigger as AnnotationTrigger};
use pdf_model::aform::Trigger;
use pdf_model::aform::site::{self, Site};
use pdf_model::view::{
    DocumentTrigger, Entered, ScriptEvent, ScriptResult, ScriptRunner, ScriptSite, ViewState,
    widgets_by_field_name,
};
use pdf_script_worker::{Cause, ScriptWorker};
use pdf_syntax::{Document, Limits};
use rayon::prelude::{IntoParallelRefIterator, ParallelIterator};

#[path = "../../pdf-model/tests/support/corpus_passwords.rs"]
#[expect(
    dead_code,
    reason = "the references' spelling of a password is the oracle's; this walk opens the \
              document itself and has no reference to hand one to"
)]
mod corpus_passwords;

#[path = "../../pdf-model/tests/support/script_population.rs"]
mod script_population;

use script_population::{MAX_FILE_BYTES, password_for, population, repository};

/// The worker program Cargo built beside this test.
const WORKER: &str = env!("CARGO_BIN_EXE_pdf-script-worker");

/// How many `SIGSYS` deaths the report names one by one.
const NAMED_KILLS: usize = 20;

/// Most pages a document's walk turns through, raising every annotation's events on each: the
/// census's scripted forms are a few pages long, and a long document's pages past these repeat
/// the same kind of event.
const MAX_TURNS: usize = 64;

/// Table 197's six events a person's pointer and focus raise, in the order a click raises them
/// and the cursor's departure ends them (RFC 0008 section 6.5 step 3).
const POINTER: [AnnotationTrigger; 6] = [
    AnnotationTrigger::Enter,
    AnnotationTrigger::Down,
    AnnotationTrigger::Focus,
    AnnotationTrigger::Up,
    AnnotationTrigger::Blur,
    AnnotationTrigger::Exit,
];

/// What the walk counts.
#[derive(Debug, Default)]
struct Tally {
    /// Runs handed to a worker.
    runs: AtomicUsize,
    /// Runs whose sentences say they finished refused nothing.
    finished: AtomicUsize,
    /// Runs that finished having been refused a call.
    finished_refused: AtomicUsize,
    /// Runs a budget stopped.
    exceeded: AtomicUsize,
    /// Runs that threw.
    threw: AtomicUsize,
    /// Runs whose script does not parse.
    unparsed: AtomicUsize,
    /// Runs that did not finish because their worker was lost, or was not started.
    lost: AtomicUsize,
    /// Workers started.
    spawns: AtomicUsize,
    /// Documents whose scripts stopped after their losses.
    stopped: AtomicUsize,
    /// Every loss, by cause.
    causes: Mutex<BTreeMap<String, usize>>,
    /// Every `SIGSYS` death: document, trigger and the death's detail.
    kills: Mutex<Vec<(String, String, String)>>,
    /// Every refused member, by its spelling.
    members: Mutex<BTreeMap<String, usize>>,
    /// Runs whose script put a question, answered here as a closed dialogue answers.
    asked: AtomicUsize,
    /// Runs at a site a window raises past the open: Table 197's ten events and Table 198's two.
    raised: AtomicUsize,
}

/// A runner that hands each event to one document's worker and counts what came back.
#[derive(Debug)]
struct Counting {
    /// The counts.
    tally: Arc<Tally>,
    /// The document's worker.
    worker: Arc<ScriptWorker>,
}

impl ScriptRunner for Counting {
    fn run(&self, event: &ScriptEvent<'_>) -> ScriptResult {
        let mut result = self.worker.run(event);
        // No person reads this walk: a question is answered at once as a closed dialogue answers,
        // and the held run's outcome, which the answer releases, is the run's (ADR 1627).
        if let Some(question) = self.worker.take_question() {
            self.tally.asked.fetch_add(1, Ordering::Relaxed);
            self.worker.answer(question.dismissed());
            if let Some(resumed) = self.worker.take_resumed() {
                result = resumed.result;
            }
        }
        self.tally.runs.fetch_add(1, Ordering::Relaxed);
        if matches!(event.site, ScriptSite::Annotation(_) | ScriptSite::Page(_)) {
            self.tally.raised.fetch_add(1, Ordering::Relaxed);
        }
        let says = |prefix: &str| {
            result
                .report
                .iter()
                .any(|sentence| sentence.starts_with(prefix))
        };
        let lost = result.report.iter().any(|sentence| {
            sentence.contains("did not finish and changed nothing")
                || sentence.starts_with("scripts stopped running for this document")
                || sentence.contains(" is not run: ")
        });
        let mut refused = false;
        if let Ok(mut members) = self.tally.members.lock() {
            // A sentence saying how the run ended repeats the refusal that ended it.
            for sentence in result
                .report
                .iter()
                .filter(|sentence| !sentence.starts_with("the script "))
            {
                let member = sentence
                    .split_once(" is not allowed: ")
                    .or_else(|| sentence.split_once(" did not run: "))
                    .map(|(member, _)| member);
                if let Some(member) = member {
                    refused = true;
                    let held = members.entry(member.to_owned()).or_default();
                    *held = held.saturating_add(1);
                }
            }
        }
        let column = if lost {
            &self.tally.lost
        } else if says("the script was stopped") {
            &self.tally.exceeded
        } else if says("the script threw") {
            &self.tally.threw
        } else if says("the script does not parse") {
            &self.tally.unparsed
        } else if refused {
            &self.tally.finished_refused
        } else {
            &self.tally.finished
        };
        column.fetch_add(1, Ordering::Relaxed);
        result
    }
}

/// Runs one document's open sequence and its field scripts through a worker of its own, answering
/// how many fields had a script Tier 0 does not run.
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
    let worker = Arc::new(ScriptWorker::with_program(WORKER));
    let mut view = ViewState::of(&document);
    view.run_scripts_with(Some(Arc::new(Counting {
        tally: Arc::clone(tally),
        worker: Arc::clone(&worker),
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
    raise_events(&document, &mut view);
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
    let name = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    tally.spawns.fetch_add(worker.spawns(), Ordering::Relaxed);
    let deaths = worker.deaths();
    if deaths.len() >= pdf_script_worker::MAX_DEATHS {
        tally.stopped.fetch_add(1, Ordering::Relaxed);
    }
    for death in deaths {
        let cause = match &death.cause {
            Cause::Deadline(_) => "deadline".to_owned(),
            Cause::Died(detail) if detail.contains("SIGSYS") => "SIGSYS".to_owned(),
            Cause::Died(detail) => {
                format!("died: {}", detail.split(':').next().unwrap_or_default())
            }
            Cause::Garbled(_) => "garbled".to_owned(),
        };
        if cause == "SIGSYS"
            && let (Ok(mut kills), Cause::Died(detail)) = (tally.kills.lock(), &death.cause)
        {
            kills.push((name.clone(), death.subject.clone(), detail.clone()));
        }
        if let Ok(mut causes) = tally.causes.lock() {
            let held = causes.entry(cause).or_default();
            *held = held.saturating_add(1);
        }
    }
    scripted.len()
}

/// Raises what a window raises past the open, through the requests the view state exposes (ADR
/// 1750): each annotation's pointer and focus events on the page shown, then a turn to the next
/// page — the page left closed, the page reached opened — for the first [`MAX_TURNS`] pages.
fn raise_events(document: &Document, view: &mut ViewState) {
    let pages = pdf_model::Pages::new(document);
    let count = pages.len().min(MAX_TURNS);
    for index in 0..count {
        let Some(page) = pages.get(index) else {
            break;
        };
        let annotations = document
            .get_key(&page.dict, "Annots")
            .as_array()
            .unwrap_or_default()
            .iter()
            .filter_map(pdf_syntax::Object::as_reference)
            .collect::<Vec<_>>();
        for annotation in annotations {
            // `/Fo` and `/Bl` are "widget annotations only", as a window gives only a widget the
            // focus.
            let widget = document
                .get(annotation)
                .as_dict()
                .is_some_and(|dictionary| {
                    document
                        .get_key(dictionary, "Subtype")
                        .as_name()
                        .is_some_and(|name| name.as_bytes() == b"Widget")
                });
            for trigger in POINTER {
                if widget || !matches!(trigger, AnnotationTrigger::Focus | AnnotationTrigger::Blur)
                {
                    view.run_annotation_scripts(document, annotation, trigger);
                }
            }
        }
        if index.saturating_add(1) < count {
            view.run_page_scripts(document, index, PageTrigger::Close);
            view.run_page_scripts(document, index.saturating_add(1), PageTrigger::Open);
        }
    }
}

#[test]
#[ignore = "walks the census population through the confined worker; run behind the heavy-walk lock"]
fn every_script_tier_0_does_not_run_is_run_in_the_confined_worker_and_its_deaths_counted() {
    let started = Instant::now();
    let files = population(&repository());
    let tally = Arc::new(Tally::default());
    let fields: usize = files.par_iter().map(|path| examine(path, &tally)).sum();
    let count = |column: &AtomicUsize| column.load(Ordering::Relaxed);
    println!(
        "{} PDF(s) walked; {fields} field(s) with a /K, /F, /V or /C Tier 0 does not run; {} \
         run(s) through {} worker(s): {} finished, {} finished refused, {} over a budget, {} \
         threw, {} unparsed, {} lost; {} document(s) stopped; {} s",
        files.len(),
        count(&tally.runs),
        count(&tally.spawns),
        count(&tally.finished),
        count(&tally.finished_refused),
        count(&tally.exceeded),
        count(&tally.threw),
        count(&tally.unparsed),
        count(&tally.lost),
        count(&tally.stopped),
        started.elapsed().as_secs()
    );
    let kills = tally
        .kills
        .lock()
        .map(|kills| kills.clone())
        .unwrap_or_default();
    if let Ok(causes) = tally.causes.lock() {
        println!("workers lost, by cause: {causes:?}");
    }
    println!("SIGSYS deaths: {}", kills.len());
    println!(
        "runs that put a question, answered as a closed dialogue answers: {}",
        count(&tally.asked)
    );
    println!(
        "runs at a widget's or a page's event a window raises (Tables 197 and 198): {}",
        count(&tally.raised)
    );
    for (document, subject, detail) in kills.iter().take(NAMED_KILLS) {
        println!("SIGSYS  {document}  {subject}  ({detail})");
    }
    if let Ok(members) = tally.members.lock() {
        let mut ranked: Vec<(&String, &usize)> = members.iter().collect();
        ranked.sort_by(|left, right| right.1.cmp(left.1).then(left.0.cmp(right.0)));
        for (member, refused) in ranked.iter().take(25) {
            println!("refused {refused:>6}  {member}");
        }
    }
    assert!(
        fields == 0 || count(&tally.runs) > 0,
        "the hook handed none of {fields} field script(s) to a worker"
    );
    assert!(
        kills.is_empty(),
        "the profile killed a worker for a call the bridged model made: {kills:?}"
    );
}

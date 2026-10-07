//! The host's half: a `ScriptRunner` whose runs happen in a confined worker it starts when first
//! asked (ADR 1609).
//!
//! # Spawned on the first trigger, never at open
//!
//! RFC 0008 section 6.6: a document with no script, or a reader whose level runs none, must not pay
//! for a process. So [`ScriptWorker::new`] finds nothing and starts nothing; the first
//! [`ScriptRunner::run`] a view state makes starts the worker, and a host whose level is `off`
//! supplies no runner at all, so the first run never comes.
//!
//! # A deadline per trigger, enforced from outside
//!
//! The engine's own budgets — wall time between execution slices, steps, loop iterations, the
//! sizes a built-in is asked for — stop every script they can see. What they cannot see is named in
//! ADR 1590: work inside one native call that calls back into script, and growth through an
//! operator. Those are this process's to bound, and the bound is the kernel's: a run that has not
//! answered by [`DEADLINE`] has its worker killed, as `confined_transport::Canceller` kills, and
//! the address-space ceiling aborts one that grows past it. Either way the trigger that was running
//! is named, the field keeps its value, and the next trigger starts another worker — holding
//! nothing, so the scripts it needs cross again, once each.
//!
//! # A new worker is brought back to where the last one was
//!
//! A realm holds what a document's scripts left in it (ADR 1602): the functions the name tree's
//! scripts defined at the open, and the fields as it was last told of them. A worker started after
//! a loss holds neither, so before it runs the trigger it was started for it is told every field
//! this runner has heard of, and every document-level script it has run is run in it again, in the
//! order first run, its edits set aside because they were made the first time.
//!
//! # And a bound on how often that happens
//!
//! A script that kills its worker kills it every time it runs, and a keystroke script runs on every
//! key. So after [`MAX_DEATHS`] deaths in one document no worker is started again, and every later
//! trigger is answered with the sentence saying so — RFC 0008 section 6.8's row for a crashed
//! engine: scripts stop running for the document, and every field is as it was.

use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::sync::{Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use confined_transport::{Canceller, Host, TransportError};
use pdf_model::view::{FieldState, ScriptEvent, ScriptResult, ScriptRunner, ScriptSite};
use pdf_script::{Ending, Outcome, Request};

use crate::wire::{
    FRAME_RUN, MAGIC, MAX_RUN_BYTES, MAX_SCRIPT_BYTES, Reply, Run, decode_reply, encode_run,
};

/// How long one trigger's run may take before its worker is killed.
///
/// Measured (ADR 1609): a run the engine's own budgets stop — `pdf_script::Budget::FIELD_EVENT`'s
/// 100 ms of wall time or its step count, whichever comes first — answered at worst 103 ms after
/// the exchange began in a debug build and 76 ms in release, slowest of ten each at a load of four;
/// a run that finishes answers in a millisecond or two. A quarter of a second is more than twice
/// the slowest: past every run the engine stops by itself, with room for a loaded machine, and
/// still short of what a person typing reads as the field having hung.
pub const DEADLINE: Duration = Duration::from_millis(250);

/// How many workers one document may lose before no other is started.
///
/// Each loss is a trigger that did not finish, reported by name; a document that loses this many
/// is one whose scripts kill their worker as a matter of course, and starting a fifth costs the
/// reader a process per keystroke for nothing.
pub const MAX_DEATHS: usize = 4;

/// Most document-level scripts a runner keeps to run again in a worker started after a loss.
///
/// Table 32's name tree holds a document's library, a handful of entries in every census document
/// that has one; past this many the rest are not kept, and a later worker is told so.
const MAX_LIBRARY: usize = 1024;

/// Most bytes of script text the scripts a host has sent one worker may come to.
///
/// The worker's own bound, mirrored, so that a run the worker would refuse to hold is declined here
/// with the same reason before it crosses.
const MAX_HELD_BYTES: usize = 8 << 20;

/// What starting the worker and its first run cost: RFC 0008 section 6.6's `script_open`.
///
/// Measured where it happens, after the first present and never before it, because the worker is
/// started by the first trigger and a trigger cannot fire before there is a page to type into.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OpenCost {
    /// From asking for the worker to reading its greeting: the program's start and its
    /// confinement.
    pub spawn: Duration,
    /// The first run's exchange: the engine constructed, the bridge installed and the script run,
    /// with the frames both ways.
    pub first_run: Duration,
}

impl OpenCost {
    /// The line a timeline prints, one `key=value` pair per figure in milliseconds.
    #[must_use]
    pub fn line(&self) -> String {
        format!(
            "script_open spawn_ms={:.3} first_run_ms={:.3}",
            self.spawn.as_secs_f64() * 1000.0,
            self.first_run.as_secs_f64() * 1000.0
        )
    }
}

/// Why a worker was lost.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Cause {
    /// It had not answered when the deadline came, so it was killed.
    Deadline(Duration),
    /// It ended without answering — the address-space ceiling's abort, the filter's kill, a
    /// panic — as far as the host can tell.
    Died(String),
    /// It answered with bytes that are not a reply, so it was killed rather than believed.
    Garbled(String),
}

/// One worker lost, and the trigger it was running.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Death {
    /// Where the script that was running runs.
    pub site: ScriptSite,
    /// The script, as a report names it: `the format script of Total`.
    pub subject: String,
    /// Why the worker was lost.
    pub cause: Cause,
}

impl Death {
    /// The sentence a report carries.
    #[must_use]
    pub fn sentence(&self) -> String {
        let what = format!("{} did not finish and changed nothing", self.subject);
        match &self.cause {
            Cause::Deadline(deadline) => format!(
                "{what}: its worker had not answered after {} ms and was stopped",
                deadline.as_millis()
            ),
            Cause::Died(detail) => format!("{what}: its worker stopped ({detail})"),
            Cause::Garbled(why) => {
                format!("{what}: its worker answered with something that is not a reply ({why})")
            }
        }
    }
}

/// A runner whose scripts run in `pdf-script-worker`, started on the first trigger.
#[derive(Debug)]
pub struct ScriptWorker {
    /// The worker program, where the host named one; found beside this executable otherwise.
    program: Option<PathBuf>,
    /// How long a run may take.
    deadline: Duration,
    /// The worker and what it holds.
    state: Mutex<State>,
}

/// What a [`ScriptWorker`] holds between runs.
#[derive(Debug, Default)]
struct State {
    /// The running worker, from the first trigger until it is lost.
    host: Option<Host>,
    /// The index each script text was given, for the worker now running.
    held: HashMap<String, u32>,
    /// Their bytes, together.
    held_bytes: usize,
    /// The index the next new script is given.
    next_index: u32,
    /// Every field this runner has been told of, as last told, by name: what a new worker's realm
    /// is told before its first run.
    fields: BTreeMap<String, FieldState>,
    /// Whether the running worker has been told every field in [`Self::fields`].
    told: bool,
    /// The document-level scripts run so far, label and text, in the order first run.
    library: Vec<(String, String)>,
    /// How many workers have been started.
    spawns: usize,
    /// Each worker lost, in order.
    deaths: Vec<Death>,
    /// The first worker's start and first run.
    open_cost: Option<OpenCost>,
    /// The first start's duration, until the first run completes the open cost.
    pending_spawn: Option<Duration>,
    /// Set once no worker will be started again, with the sentence saying why.
    stopped: Option<String>,
}

impl ScriptWorker {
    /// A runner that will start the worker found beside this executable — or named by
    /// [`crate::WORKER_PATH_VARIABLE`] — at its first trigger. Starts nothing now.
    #[must_use]
    pub fn new() -> Self {
        Self {
            program: None,
            deadline: DEADLINE,
            state: Mutex::new(State::default()),
        }
    }

    /// A runner that will start `program` at its first trigger. Starts nothing now.
    #[must_use]
    pub fn with_program(program: impl Into<PathBuf>) -> Self {
        Self {
            program: Some(program.into()),
            ..Self::new()
        }
    }

    /// This runner with another deadline per trigger.
    #[must_use]
    pub fn with_deadline(mut self, deadline: Duration) -> Self {
        self.deadline = deadline;
        self
    }

    /// How many workers this runner has started.
    #[must_use]
    pub fn spawns(&self) -> usize {
        self.state().spawns
    }

    /// Each worker lost, with the trigger it was running.
    #[must_use]
    pub fn deaths(&self) -> Vec<Death> {
        self.state().deaths.clone()
    }

    /// What the running worker reported its kernel granted, where a worker is running.
    #[must_use]
    pub fn confinement(&self) -> Option<pdf_sandbox::lockdown::Confinement> {
        self.state().host.as_ref().map(Host::confinement)
    }

    /// What the first worker's start and first run cost, once both have happened.
    #[must_use]
    pub fn open_cost(&self) -> Option<OpenCost> {
        self.state().open_cost
    }

    /// The state, recovered from a poisoned lock: nothing under it panics, so a poisoned lock is a
    /// panic elsewhere, and the state is still whole.
    fn state(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// The worker program.
    fn program(&self) -> Result<PathBuf, String> {
        if let Some(program) = &self.program {
            return Ok(program.clone());
        }
        confined_transport::program_beside_executable(
            crate::WORKER_PROGRAM,
            crate::WORKER_PATH_VARIABLE,
        )
        .map_err(|missing| {
            format!(
                "the script worker `{}` was {missing}; build it with `cargo build -p \
                 pdf-script-worker --features engine --bins`, or name it in {}",
                crate::WORKER_PROGRAM,
                crate::WORKER_PATH_VARIABLE
            )
        })
    }

    /// Runs one event in the worker, starting one where none is running.
    fn run_event(&self, event: &ScriptEvent<'_>) -> Result<Outcome, Vec<String>> {
        let mut state = self.state();
        let subject = subject(event);
        for field in event.fields {
            state.fields.insert(field.name.clone(), field.clone());
        }
        if event.site == ScriptSite::Library
            && state.library.len() < MAX_LIBRARY
            && !state
                .library
                .iter()
                .any(|(label, text)| label == event.label && text == event.script)
        {
            state
                .library
                .push((event.label.to_owned(), event.script.to_owned()));
        }
        if let Some(stopped) = &state.stopped {
            return Err(vec![stopped.clone()]);
        }
        if event.script.len() > MAX_SCRIPT_BYTES {
            return Err(vec![format!(
                "{subject} is not run: it is {} bytes, past the {MAX_SCRIPT_BYTES} a script \
                 worker is handed",
                event.script.len()
            )]);
        }

        let mut report = Vec::new();
        if state.host.is_none() {
            let program = self.program().map_err(|sentence| {
                let stopped = format!("scripts stopped running for this document: {sentence}");
                state.stopped = Some(stopped.clone());
                vec![stopped]
            })?;
            let started = Instant::now();
            match Host::start(&program, MAGIC, &Canceller::new()) {
                Ok(host) => {
                    if let Some(shortfall) = host.confinement().shortfall() {
                        report.push(format!("the script worker started: {shortfall}"));
                    }
                    if state.open_cost.is_none() && state.pending_spawn.is_none() {
                        state.pending_spawn = Some(started.elapsed());
                    }
                    state.host = Some(host);
                    state.held.clear();
                    state.held_bytes = 0;
                    state.told = false;
                    state.spawns = state.spawns.saturating_add(1);
                    if state.spawns > 1 {
                        self.restore(&mut state, event, &mut report);
                        if state.host.is_none() {
                            return Err(report);
                        }
                    }
                }
                Err(error) => {
                    let stopped = format!("scripts stopped running for this document: {error}");
                    state.stopped = Some(stopped.clone());
                    return Err(vec![stopped]);
                }
            }
        }

        let mut request = Request::of(event, now(), 0);
        if !state.told {
            request.fields = state.fields.values().cloned().collect();
        }
        match self.exchange(&mut state, event.site, &subject, request, &mut report) {
            Ok(mut outcome) => {
                if !report.is_empty() {
                    report.append(&mut outcome.log);
                    outcome.log = report;
                }
                Ok(outcome)
            }
            Err(()) => Err(report),
        }
    }

    /// Tells a worker started after a loss what its realm held: every field, and every
    /// document-level script run so far, each run again with its edits set aside.
    fn restore(&self, state: &mut State, event: &ScriptEvent<'_>, report: &mut Vec<String>) {
        let library = state.library.clone();
        for (label, text) in library {
            if event.site == ScriptSite::Library && event.label == label && event.script == text {
                continue;
            }
            let mut request = Request::of(event, now(), 0);
            request.site = ScriptSite::Library;
            request.field = String::new();
            request.label.clone_from(&label);
            request.script = text;
            request.event = pdf_script::Event {
                value: String::new(),
                change: String::new(),
                selection_start: 0,
                selection_end: 0,
                will_commit: false,
                source: String::new(),
            };
            request.fields = if state.told {
                Vec::new()
            } else {
                state.fields.values().cloned().collect()
            };
            let subject = format!("the document-level script {label}");
            match self.exchange(state, ScriptSite::Library, &subject, request, report) {
                Ok(outcome) if outcome.ending == Ending::Finished => {}
                Ok(outcome) => report.push(format!(
                    "{subject}, run again in a new script worker, {}",
                    outcome
                        .ending
                        .sentence()
                        .unwrap_or_else(|| "did not finish".to_owned())
                )),
                Err(()) => return,
            }
        }
    }

    /// Sends one request to the running worker under the deadline, the script crossing first where
    /// the worker does not hold it, and reads the outcome back; or records the loss and answers
    /// `Err` with the sentences in `report`.
    fn exchange(
        &self,
        state: &mut State,
        site: ScriptSite,
        subject: &str,
        mut request: Request,
        report: &mut Vec<String>,
    ) -> Result<Outcome, ()> {
        let text = std::mem::take(&mut request.script);
        let (new_script, index) = if let Some(index) = state.held.get(&text) {
            (None, *index)
        } else {
            if state.held_bytes.saturating_add(text.len()) > MAX_HELD_BYTES {
                report.push(format!(
                    "{subject} is not run: the script worker holds {} bytes of this document's \
                     scripts and it would take them past {MAX_HELD_BYTES}",
                    state.held_bytes
                ));
                return Err(());
            }
            (Some((state.next_index, text)), state.next_index)
        };
        let told = !request.fields.is_empty() || state.fields.is_empty();
        let run = Run {
            new_script,
            script: index,
            request,
        };
        let bytes = encode_run(&run);
        if bytes.len() > MAX_RUN_BYTES {
            report.push(format!(
                "{subject} is not run: its event is {} bytes, past the {MAX_RUN_BYTES} a script \
                 worker reads",
                bytes.len()
            ));
            return Err(());
        }

        let Some(host) = state.host.as_mut() else {
            return Err(());
        };
        let canceller = host.canceller();
        let started = Instant::now();
        let exchanged = within(self.deadline, &canceller, || {
            host.exchange(FRAME_RUN, &bytes, None)
        });
        let spent = started.elapsed();

        let (answer, fired) = match exchanged {
            Ok(result) => result,
            Err(sentence) => {
                report.push(format!("{subject} is not run: {sentence}"));
                return Err(());
            }
        };
        let reply = match answer {
            Ok((kind, payload)) => decode_reply(kind, &payload),
            Err(TransportError::Cancelled) if fired => {
                lose(state, site, subject, Cause::Deadline(self.deadline), report);
                return Err(());
            }
            Err(error) => {
                let detail = match error {
                    TransportError::WorkerDied { detail } => detail,
                    other => other.to_string(),
                };
                lose(state, site, subject, Cause::Died(detail), report);
                return Err(());
            }
        };
        if fired {
            // The deadline came as the answer did: the worker is killed either way, and the answer
            // that arrived is still the run's.
            state.host = None;
        }
        let reply = match reply {
            Ok(reply) => reply,
            Err(error) => {
                if let Some(host) = state.host.take() {
                    host.canceller().cancel();
                }
                lose(
                    state,
                    site,
                    subject,
                    Cause::Garbled(error.to_string()),
                    report,
                );
                return Err(());
            }
        };
        if let Some((index, text)) = run.new_script {
            state.held_bytes = state.held_bytes.saturating_add(text.len());
            state.held.insert(text, index);
            state.next_index = state.next_index.saturating_add(1);
        }
        if told {
            state.told = true;
        }
        if let Some(spawn) = state.pending_spawn.take() {
            state.open_cost = Some(OpenCost {
                spawn,
                first_run: spent,
            });
        }
        match reply {
            Reply::Outcome(outcome) => Ok(outcome),
            Reply::Refused(sentence) => {
                report.push(format!("{subject} is not run: {sentence}"));
                Err(())
            }
        }
    }
}

/// Records a lost worker, and stops starting new ones once a document has lost enough.
fn lose(
    state: &mut State,
    site: ScriptSite,
    subject: &str,
    cause: Cause,
    report: &mut Vec<String>,
) {
    state.host = None;
    let death = Death {
        site,
        subject: subject.to_owned(),
        cause,
    };
    report.push(death.sentence());
    state.deaths.push(death);
    if state.deaths.len() >= MAX_DEATHS {
        state.stopped = Some(format!(
            "scripts stopped running for this document: {} script workers were lost, the last \
             running {subject}",
            state.deaths.len()
        ));
    } else {
        report.push("the next trigger starts another script worker".to_owned());
    }
}

/// The script an event runs, as a report names it.
fn subject(event: &ScriptEvent<'_>) -> String {
    let on = |what: &str| {
        if event.field.is_empty() {
            what.to_owned()
        } else {
            event.field.to_owned()
        }
    };
    match event.site {
        ScriptSite::Field(trigger) => format!("the {} script of {}", trigger.noun(), event.field),
        ScriptSite::Annotation(trigger) => {
            format!("the /{} script of {}", trigger.key(), on("an annotation"))
        }
        ScriptSite::Page(pdf_model::action::PageTrigger::Open) => {
            format!("the open script of page {}", event.page.saturating_add(1))
        }
        ScriptSite::Page(pdf_model::action::PageTrigger::Close) => {
            format!("the close script of page {}", event.page.saturating_add(1))
        }
        ScriptSite::OpenAction => "the document's open action".to_owned(),
        ScriptSite::Library => format!("the document-level script {}", event.label),
        ScriptSite::Document(trigger) => format!("the document's /{} script", trigger.key()),
    }
}

/// Now, in milliseconds since 1970: the moment `Date` answers for one run.
fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| {
            u64::try_from(since.as_millis()).unwrap_or(u64::MAX)
        })
}

impl Default for ScriptWorker {
    fn default() -> Self {
        Self::new()
    }
}

impl ScriptRunner for ScriptWorker {
    fn run(&self, event: &ScriptEvent<'_>) -> ScriptResult {
        match self.run_event(event) {
            Ok(outcome) => outcome.result(),
            Err(report) => ScriptResult {
                rc: true,
                value: None,
                change: None,
                edits: Vec::new(),
                report,
            },
        }
    }
}

/// Runs `work` with a watchdog that cancels the worker if `work` has not returned by `deadline`;
/// answers what `work` returned and whether the watchdog fired, or the sentence saying no watchdog
/// could be armed — in which case `work` is not run, because a run without its deadline is a run
/// the engine's budgets alone bound, and ADR 1590 names what they cannot.
fn within<T>(
    deadline: Duration,
    canceller: &Canceller,
    work: impl FnOnce() -> T,
) -> Result<(T, bool), String> {
    let fired = AtomicBool::new(false);
    let (finished, waiting) = mpsc::channel::<()>();
    let flag = &fired;
    std::thread::scope(|scope| {
        let watchdog = std::thread::Builder::new()
            .name("script deadline".to_owned())
            .spawn_scoped(scope, move || {
                if matches!(
                    waiting.recv_timeout(deadline),
                    Err(RecvTimeoutError::Timeout)
                ) {
                    flag.store(true, Ordering::SeqCst);
                    canceller.cancel();
                }
            });
        if let Err(error) = watchdog {
            return Err(format!("no deadline could be armed for it: {error}"));
        }
        let out = work();
        drop(finished);
        Ok(out)
    })
    .map(|out| (out, fired.load(Ordering::SeqCst)))
}

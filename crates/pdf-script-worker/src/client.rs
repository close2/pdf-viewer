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
//!
//! # A question is held in the worker, never on a host's thread
//!
//! A script's `app.alert` or `app.response` comes back from the worker as a question in place of
//! the run's outcome, the script held mid-call. The run answers at once, having changed nothing,
//! and later triggers of the document queue behind it; [`ScriptWorker::take_question`] hands the
//! question to the window, [`ScriptWorker::answer`] sends the person's answer and runs the queue,
//! and each finished run waits for `ScriptRunner::take_resumed` (ADRs 1627, 1628).

use std::collections::{BTreeMap, HashMap, VecDeque};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::sync::{Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use confined_transport::{Canceller, Host, TransportError};
use pdf_model::view::{FieldState, Resumed, ScriptEvent, ScriptResult, ScriptRunner, ScriptSite};
use pdf_script::wire::encode_answer;
use pdf_script::{Answer, Ending, Outcome, Question, Request};

use crate::wire::{
    FRAME_ANSWER, FRAME_RUN, MAGIC, MAX_RUN_BYTES, MAX_SCRIPT_BYTES, Reply, Run, decode_reply,
    encode_run,
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

/// How long a script's question waits on the person before the runner answers it itself, with the
/// answer a closed dialogue gives (ADR 1627).
///
/// A question is a person's to answer and has no deadline a script could meet; this bounds the case
/// where nobody does — a window that lost its card, a person who walked away — so that a document's
/// scripts are not held for ever behind one dialogue. Two minutes is longer than any person takes
/// over a dialogue of a sentence or two, and the bound is applied at the runner's next call after
/// it passes, since a runner keeps no thread of its own.
pub const ANSWER_WAIT: Duration = Duration::from_mins(2);

/// Most triggers a runner queues behind a script that waits on a person.
///
/// A person with a question in front of them is not typing into the form, so what queues is the
/// rest of a sequence already under way — the open's library, a commit's calculations — and a form
/// with more than this in one sequence is one whose further triggers are named as not run.
const MAX_QUEUED: usize = 256;

/// Longest the triggers queued behind an answered question run in one call, after which the rest
/// wait for the runner's next call.
///
/// They run on the thread that handed the answer over, which is a window's; `pdf_model::view`'s
/// one second for a sequence of scripts is the same bound for the same reason.
const DRAIN_TIME: Duration = Duration::from_secs(1);

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
    /// How long a question waits on the person before the runner answers it.
    answer_wait: Duration,
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
    /// The script held on a person's answer, where one is.
    waiting: Option<Waiting>,
    /// The triggers handed over while a script waits, in the order handed over.
    queued: VecDeque<Queued>,
    /// The runs that were held and have since finished, in the order they finished.
    resumed: VecDeque<Resumed>,
    /// Whether the last run handed back was held rather than finished.
    holding: bool,
    /// Set when a question's wait ran out and the runner answered it; taken by
    /// [`ScriptWorker::question_withdrawn`].
    withdrawn: bool,
}

/// A script held in its worker on a person's answer.
#[derive(Debug)]
struct Waiting {
    /// What it asked.
    question: Question,
    /// Whether the host has taken the question to put it.
    taken: bool,
    /// When it asked.
    since: Instant,
    /// Where the script runs.
    site: ScriptSite,
    /// The event's field, or empty.
    field: String,
    /// The script, as a report names it.
    subject: String,
}

/// A trigger handed over while a script waits, to run once it has finished.
#[derive(Debug)]
struct Queued {
    /// The request, its script included.
    request: Request,
    /// The script, as a report names it.
    subject: String,
}

/// What one exchange with a worker came back with.
#[derive(Debug)]
enum Exchanged {
    /// The run finished.
    Finished(Outcome),
    /// The run's script asked a question and is held.
    Asked(Question),
}

impl ScriptWorker {
    /// A runner that will start the worker found beside this executable — or named by
    /// [`crate::WORKER_PATH_VARIABLE`] — at its first trigger. Starts nothing now.
    #[must_use]
    pub fn new() -> Self {
        Self {
            program: None,
            deadline: DEADLINE,
            answer_wait: ANSWER_WAIT,
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

    /// This runner with another wait for a question's answer than [`ANSWER_WAIT`].
    #[must_use]
    pub fn with_answer_wait(mut self, wait: Duration) -> Self {
        self.answer_wait = wait;
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
        state.holding = false;
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
        self.expire(&mut state);
        self.drain(&mut state);
        if state.waiting.is_some() || !state.queued.is_empty() {
            return Self::queue(&mut state, event, subject);
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
            Ok(Exchanged::Finished(mut outcome)) => {
                if !report.is_empty() {
                    report.append(&mut outcome.log);
                    outcome.log = report;
                }
                Ok(outcome)
            }
            Ok(Exchanged::Asked(question)) => {
                state.holding = true;
                let mut outcome = Outcome::unchanged(Ending::Finished);
                outcome.notes.push(format!(
                    "{subject} asked {} and is held in its worker until the person answers; \
                     what it does is applied then (ADR 1627)",
                    question.summary()
                ));
                outcome.log = report;
                state.waiting = Some(Waiting {
                    question,
                    taken: false,
                    since: Instant::now(),
                    site: event.site,
                    field: event.field.to_owned(),
                    subject,
                });
                Ok(outcome)
            }
            Err(()) => Err(report),
        }
    }

    /// Queues a trigger behind the script that waits, and answers that it is held.
    fn queue(
        state: &mut State,
        event: &ScriptEvent<'_>,
        subject: String,
    ) -> Result<Outcome, Vec<String>> {
        if state.queued.len() >= MAX_QUEUED {
            return Err(vec![format!(
                "{subject} is not run: a script of this document waits on the person's answer \
                 and {MAX_QUEUED} triggers already wait behind it"
            )]);
        }
        let mut request = Request::of(event, now(), 0);
        request.fields = state.fields.values().cloned().collect();
        state.holding = true;
        let mut outcome = Outcome::unchanged(Ending::Finished);
        outcome.notes.push(format!(
            "{subject} waits behind a script of this document that asked the person a question, \
             and runs in order once that is answered (ADR 1627)"
        ));
        state.queued.push_back(Queued { request, subject });
        Ok(outcome)
    }

    /// The question a script of this document is waiting on, handed over once: `None` where none
    /// waits, and where this one has been taken already (ADR 1628).
    ///
    /// What a host polls after every command, to put the question to the person on its own
    /// thread; [`Self::answer`] is how the answer comes back. Triggers still queued behind an
    /// answered question run here first, for at most [`DRAIN_TIME`].
    pub fn take_question(&self) -> Option<Question> {
        let mut state = self.state();
        self.expire(&mut state);
        // A queue a drain left part of runs on here, a host's poll after each command being the
        // runner's next chance.
        self.drain(&mut state);
        let waiting = state.waiting.as_mut()?;
        if waiting.taken {
            return None;
        }
        waiting.taken = true;
        Some(waiting.question.clone())
    }

    /// The person's answer to the question last taken: the held script resumes in its worker under
    /// a fresh deadline per trigger, finishes, and the triggers queued behind it run in order; each
    /// finished run waits for [`ScriptRunner::take_resumed`] (ADR 1627).
    ///
    /// Nothing waits where the question's wait has run out, or the worker was lost, and then the
    /// answer is kept nowhere.
    #[expect(
        clippy::needless_pass_by_value,
        reason = "the answer is the host's to hand over, and a host hands over what the person gave"
    )]
    pub fn answer(&self, answer: Answer) {
        let mut state = self.state();
        if state.waiting.is_none() {
            return;
        }
        self.resume(&mut state, &answer, None);
    }

    /// Whether a question this runner put has been withdrawn since last asked: its wait ran out
    /// and the runner answered it as a closed dialogue answers, so a host showing it drops it.
    /// True once per withdrawal.
    pub fn question_withdrawn(&self) -> bool {
        let mut state = self.state();
        self.expire(&mut state);
        std::mem::take(&mut state.withdrawn)
    }

    /// Answers a question that has waited past [`ANSWER_WAIT`] with its closed-dialogue answer.
    fn expire(&self, state: &mut State) {
        let expired = state
            .waiting
            .as_ref()
            .is_some_and(|waiting| waiting.since.elapsed() > self.answer_wait);
        if !expired {
            return;
        }
        let Some(question) = state
            .waiting
            .as_ref()
            .map(|waiting| waiting.question.clone())
        else {
            return;
        };
        state.withdrawn = true;
        let note = format!(
            "its question was not answered within {} ms, so the runner answered it as a closed \
             dialogue answers (ADR 1627)",
            self.answer_wait.as_millis()
        );
        self.resume(state, &question.dismissed(), Some(note));
    }

    /// Hands the held script its answer, keeps what it then did, and runs the queue behind it
    /// until a run finishes asking another question or the queue is empty.
    fn resume(&self, state: &mut State, answer: &Answer, note: Option<String>) {
        let Some(waiting) = state.waiting.take() else {
            return;
        };
        let mut report = Vec::new();
        let finished = match self.send(
            state,
            waiting.site,
            &waiting.subject,
            FRAME_ANSWER,
            &encode_answer(answer),
            &mut report,
        ) {
            Ok(Reply::Outcome(outcome)) => Some(outcome),
            Ok(Reply::Refused(sentence)) => {
                report.push(format!("{} did not resume: {sentence}", waiting.subject));
                None
            }
            Ok(Reply::Asked(_)) => {
                report.push(format!(
                    "{} asked a second question where its outcome was due, and was stopped",
                    waiting.subject
                ));
                if let Some(host) = state.host.take() {
                    host.canceller().cancel();
                }
                None
            }
            Err(()) => None,
        };
        let mut result = finished.map_or_else(
            || ScriptResult {
                rc: true,
                value: None,
                change: None,
                edits: Vec::new(),
                report: Vec::new(),
            },
            |outcome| outcome.result(),
        );
        if let Some(note) = note {
            result.report.insert(0, note);
        }
        result.report.extend(report);
        state.resumed.push_back(Resumed {
            site: waiting.site,
            field: waiting.field,
            result,
        });
        self.drain(state);
    }

    /// Runs the queue in order until a run asks a question, the queue is empty, or
    /// [`DRAIN_TIME`] has passed.
    fn drain(&self, state: &mut State) {
        let started = Instant::now();
        while state.waiting.is_none() && started.elapsed() < DRAIN_TIME {
            let Some(queued) = state.queued.pop_front() else {
                break;
            };
            self.run_queued(state, queued);
        }
    }

    /// Runs one trigger that waited in the queue, keeping what it did.
    fn run_queued(&self, state: &mut State, queued: Queued) {
        let site = queued.request.site;
        let field = queued.request.field.clone();
        let mut report = Vec::new();
        let outcome = if state.host.is_some() {
            self.exchange(state, site, &queued.subject, queued.request, &mut report)
        } else {
            report.push(format!(
                "{} is not run: the script worker it waited for was lost",
                queued.subject
            ));
            Err(())
        };
        match outcome {
            Ok(Exchanged::Asked(question)) => {
                state.waiting = Some(Waiting {
                    question,
                    taken: false,
                    since: Instant::now(),
                    site,
                    field,
                    subject: queued.subject,
                });
            }
            Ok(Exchanged::Finished(outcome)) => {
                let mut result = outcome.result();
                result.report.extend(report);
                state.resumed.push_back(Resumed {
                    site,
                    field,
                    result,
                });
            }
            Err(()) => state.resumed.push_back(Resumed {
                site,
                field,
                result: ScriptResult {
                    rc: true,
                    value: None,
                    change: None,
                    edits: Vec::new(),
                    report,
                },
            }),
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
                commit_key: 0,
                field_full: false,
                change_ex: String::new(),
                source: String::new(),
            };
            request.fields = if state.told {
                Vec::new()
            } else {
                state.fields.values().cloned().collect()
            };
            let subject = format!("the document-level script {label}");
            let exchanged =
                match self.exchange(state, ScriptSite::Library, &subject, request, report) {
                    // A library run again asked the person once already; its question is answered as a
                    // closed dialogue answers rather than put twice.
                    Ok(Exchanged::Asked(question)) => self
                        .send(
                            state,
                            ScriptSite::Library,
                            &subject,
                            FRAME_ANSWER,
                            &encode_answer(&question.dismissed()),
                            report,
                        )
                        .map(|reply| match reply {
                            Reply::Outcome(outcome) => outcome,
                            _ => Outcome::unchanged(Ending::Declined(
                                "it did not finish after its question".to_owned(),
                            )),
                        }),
                    Ok(Exchanged::Finished(outcome)) => Ok(outcome),
                    Err(()) => Err(()),
                };
            match exchanged {
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
    ) -> Result<Exchanged, ()> {
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

        let started = Instant::now();
        let reply = self.send(state, site, subject, FRAME_RUN, &bytes, report)?;
        let spent = started.elapsed();
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
            Reply::Outcome(outcome) => Ok(Exchanged::Finished(outcome)),
            Reply::Asked(question) => Ok(Exchanged::Asked(question)),
            Reply::Refused(sentence) => {
                report.push(format!("{subject} is not run: {sentence}"));
                Err(())
            }
        }
    }

    /// Sends one frame to the running worker under the deadline and reads its reply; or records
    /// the loss and answers `Err` with the sentences in `report`.
    ///
    /// **The deadline is the exchange's**: a run's ends when the worker answers it, with its
    /// outcome or with a question, and the exchange that carries the answer to a question has a
    /// deadline of its own, as long, from the moment the answer is sent. The time between, which is
    /// a person reading, is no exchange's; the engine's own wall budget leaves it out the same way
    /// (ADR 1627).
    fn send(
        &self,
        state: &mut State,
        site: ScriptSite,
        subject: &str,
        kind: u8,
        bytes: &[u8],
        report: &mut Vec<String>,
    ) -> Result<Reply, ()> {
        let Some(host) = state.host.as_mut() else {
            return Err(());
        };
        let canceller = host.canceller();
        let exchanged = within(self.deadline, &canceller, || {
            host.exchange(kind, bytes, None)
        });
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
        match reply {
            Ok(reply) => Ok(reply),
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

    fn waiting(&self) -> bool {
        self.state().holding
    }

    fn take_resumed(&self) -> Option<Resumed> {
        let mut state = self.state();
        self.expire(&mut state);
        state.resumed.pop_front()
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

//! Boa, as one realm a document's scripts share, each run held to a [`Budget`] and read back into
//! an [`Outcome`].
//!
//! # One realm per document
//!
//! A [`Realm`] is one Boa context that lives as long as its document: §12.6.4.17 has the
//! name tree's scripts executed "[w]hen the document is opened, … defining ECMAScript functions for
//! use by other scripts in the document", so what the open defines is what a field's script calls
//! later, and the realm is where both run (ADR 1602). Each run installs a fresh `event` and clears
//! its record; every global a script leaves — a function, a variable, a prototype it changed —
//! stays, as it does in the reference's viewer. The realm also holds the document's fields as it
//! was last told of them ([`crate::Request::fields`]), so `this.getField` reads any field without
//! asking across the process boundary.
//!
//! # How the budgets are enforced
//!
//! Boa offers three limits of its own — loop iterations per call frame, recursion depth, value
//! stack — and those are set from the budget. It has no wall-clock interrupt and no memory
//! ceiling, so the rest are this module's: the script is evaluated with Boa's
//! `evaluate_async_with_budget`, which yields after a fixed number of its cost units, and the poll
//! loop below checks the clock and the step count at every yield and abandons the evaluation
//! where either is spent. The sizes an argument can ask a built-in to allocate are checked before
//! the built-in runs (`guard`), and how deep a script's text nests before the parser is handed it
//! (`crate::depth`, ADR 1626) — its own text, and any string `eval` or `Function` compiles while it
//! runs. Boa's AST optimizer is off, for ADR 1626's figures. What none of these bounds — work inside one native call that
//! calls back into script, growth through an operator, and a realm's heap across its lifetime — is
//! the process's to bound, and ADRs 1590 and 1602 name each.

mod bridge;
mod guard;
mod members;

use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::future::Future;
use std::pin::pin;
use std::rc::Rc;
use std::sync::mpsc;
use std::sync::{Arc, Mutex, PoisonError};
use std::task::{Context as TaskContext, Poll, Waker};
use std::thread::JoinHandle;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use boa_engine::context::HostHooks;
use boa_engine::context::time::{Clock, JsInstant};
use boa_engine::error::{EngineError, RuntimeLimitError};
use boa_engine::module::IdleModuleLoader;
use boa_engine::optimizer::OptimizerOptions;
use boa_engine::realm::Realm as BoaRealm;
use boa_engine::{Context, JsError, JsNativeError, JsResult, JsString, JsValue, Script, Source};
use pdf_model::view::{
    DocumentState, FieldState, ScriptEdit, ScriptEvent, ScriptResult, ScriptRunner, ScriptSite,
};

use crate::{Asker, Budget, Ending, Exceeded, Nobody, Outcome, Refusal, RefusalKind, Request};

/// The engine cost units one execution slice spends before the poll loop looks at the clock.
///
/// Boa's own default for its asynchronous evaluation is 256. Each yield costs a poll, a clock read
/// and a comparison, so a slice a sixteenth the size of the step budget's resolution would be
/// overhead for nothing; 4 096 units is a few tens of microseconds of a script, which is the
/// granularity the wall-time budget is enforced at (ADR 1590).
const SLICE: u32 = 4096;

/// Most refusals one run records; a script refused more often than this is refused the same few
/// calls in a loop.
const MAX_REFUSALS: usize = 64;

/// Most lines one run's log keeps.
const MAX_LOG_LINES: usize = 64;

/// Most notes one run keeps.
const MAX_NOTES: usize = 64;

/// Longest logged line kept, in characters.
const MAX_LOG_CHARACTERS: usize = 1024;

/// Most edits one run hands back: a script setting every field of the census's largest form once
/// is under it, and one setting a field in a loop is the same edit many times.
const MAX_EDITS: usize = 4096;

/// The stack a realm's thread runs on.
///
/// Boa's interpreter keeps a script's own frames on its heap, and its recursion limit of 512
/// bounds them; what reaches the native stack is a native's call back into script, a sort's
/// comparator or a getter, each a few kilobytes. Sixteen mebibytes is the main thread's default
/// twice over, so a realm on its own thread meets no bound a run on the main thread would not.
const REALM_STACK: usize = 16 << 20;

/// Runs one request's script in a realm constructed for it alone.
///
/// The realm is dropped afterwards, so nothing the script defines reaches another run: this is
/// the one-shot form, for a caller with no document to keep a realm for. Never panics on any input
/// and never leaves the engine running: a run ends finished, over a budget, thrown, unparsed or
/// declined, and [`Outcome::ending`] says which.
#[must_use]
pub fn run(request: &Request, budget: &Budget) -> Outcome {
    match Realm::new(*budget) {
        Ok(mut realm) => realm.run(request),
        Err(why) => Outcome::unchanged(Ending::Declined(why)),
    }
}

/// One document's scripts' shared engine context: what the name tree defines at the open, and the
/// fields as the realm was last told of them (ADR 1602).
///
/// Not `Send`: Boa's values are reference-counted per thread. A realm lives on the thread that
/// built it, which is the confined worker's main thread or [`Engine`]'s own.
#[derive(Debug)]
pub struct Realm {
    /// The context every run evaluates in.
    context: Context,
    /// What `Date` answers, set per run.
    clock: Rc<Moment>,
    /// The offset local time is read with, set per run.
    utc_offset_seconds: Rc<Cell<i32>>,
    /// The budget every run is held to.
    budget: Budget,
    /// How long the current run's question waited on its answer.
    waited: Rc<Cell<Duration>>,
}

impl Realm {
    /// A realm with the host object model installed and no field known, whose scripts' questions
    /// nobody is asked ([`Nobody`]).
    ///
    /// # Errors
    ///
    /// The sentence saying what could not be installed, where the engine refuses a property on a
    /// context just constructed — which it does not.
    pub fn new(budget: Budget) -> Result<Self, String> {
        Self::with_asker(budget, Rc::new(Nobody))
    }

    /// A realm whose scripts' questions `asker` answers (ADR 1627).
    ///
    /// # Errors
    ///
    /// As [`Self::new`].
    pub fn with_asker(budget: Budget, asker: Rc<dyn Asker>) -> Result<Self, String> {
        let clock = Rc::new(Moment::default());
        let utc_offset_seconds = Rc::new(Cell::new(0));
        let hooks = Hooks {
            utc_offset_seconds: Rc::clone(&utc_offset_seconds),
            buffer_bytes: budget.buffer_bytes,
        };
        // Boa's default module loader resolves `.` against the file system when a context is
        // built — a `realpath`, which the script worker's confinement kills for (trap 31, ADR
        // 1608). A document's scripts import nothing, so the loader is the one that refuses every
        // module and reads nothing.
        let mut context = Context::builder()
            .host_hooks(Rc::new(hooks))
            .clock(Rc::clone(&clock))
            .module_loader(Rc::new(IdleModuleLoader))
            .build()
            .map_err(|error| format!("the engine could not be constructed: {error}"))?;
        let limits = context.runtime_limits_mut();
        limits.set_loop_iteration_limit(budget.loop_iterations);
        limits.set_recursion_limit(usize::try_from(budget.recursion).unwrap_or(usize::MAX));
        limits.set_stack_size_limit(usize::try_from(budget.stack).unwrap_or(usize::MAX));
        // Boa's AST optimizer is off (ADR 1626): it was 70.8% of the largest library's
        // instructions, it is a second recursive walk over what a hostile script nests, and what it
        // buys — folding a chain of constants — is a case no form's script is.
        context.set_optimizer_options(OptimizerOptions::empty());
        let waited = Rc::new(Cell::new(Duration::ZERO));
        context.insert_data(State::new(budget, asker, Rc::clone(&waited)));
        bridge::install(&mut context)
            .map_err(|error| format!("the host object model could not be installed: {error}"))?;
        Ok(Self {
            context,
            clock,
            utc_offset_seconds,
            budget,
            waited,
        })
    }

    /// Runs one request's script in this realm.
    ///
    /// The request's fields replace the realm's record of each by name first; then a fresh `event`
    /// is installed and the script evaluated under the budget. Never panics on any input.
    pub fn run(&mut self, request: &Request) -> Outcome {
        self.clock.set(request.moment);
        self.utc_offset_seconds.set(request.utc_offset_seconds);
        State::begin(&self.context, request);
        let event = match bridge::begin(&mut self.context, request) {
            Ok(event) => event,
            Err(error) => {
                return Outcome::unchanged(Ending::Declined(format!(
                    "the event could not be installed: {error}"
                )));
            }
        };
        if let Some(exceeded) = too_deep(&request.script, &self.budget) {
            return Outcome::unchanged(Ending::Exceeded(exceeded));
        }
        let script = match Script::parse(
            Source::from_bytes(request.script.as_bytes()),
            None,
            &mut self.context,
        ) {
            Ok(script) => script,
            Err(error) => return Outcome::unchanged(Ending::Unparsed(error.to_string())),
        };
        self.waited.set(Duration::ZERO);
        let ending = evaluate(&script, &mut self.context, &self.budget, &self.waited);
        let recorded = State::take(&self.context);
        let ending = match (ending, recorded.exceeded) {
            (_, Some(exceeded)) => Ending::Exceeded(exceeded),
            (ending, None) => ending,
        };
        let finished = ending == Ending::Finished;
        let mut notes = recorded.notes;
        if recorded.unasked > 0 {
            notes.push(format!(
                "{} further question(s) in the same run were not put: one question is put per \
                 trigger, and each of the rest was answered as a closed dialogue answers (RFC \
                 0008 section 6.8)",
                recorded.unasked
            ));
        }
        let mut outcome = Outcome {
            ending,
            refusals: recorded.refusals,
            log: recorded.log,
            notes,
            // A run that did not finish changes nothing, its edits included: a calculation
            // stopped half way through has set some fields and not others.
            edits: if finished { recorded.edits } else { Vec::new() },
            ..Outcome::unchanged(Ending::Finished)
        };
        if finished {
            bridge::read_event(&event, request, &mut self.context, &mut outcome);
        } else {
            State::forget_edits(&self.context);
        }
        outcome
    }
}

/// The budget a script's text exceeds before it is parsed: its brackets' nesting, or the stack its
/// parse and compilation would need ([`crate::depth`]).
fn too_deep(script: &str, budget: &Budget) -> Option<Exceeded> {
    if nesting(script) > usize::try_from(budget.nesting).unwrap_or(usize::MAX) {
        return Some(Exceeded::Nesting(budget.nesting));
    }
    let estimated = crate::depth::estimate(script);
    (estimated > budget.depth).then_some(Exceeded::Depth {
        estimated,
        ceiling: budget.depth,
    })
}

/// The deepest a script's brackets nest, counted over every byte.
///
/// Boa 0.22's parser recurses once per nested expression and states no depth limit: five hundred
/// nested parentheses overflow an 8 MiB stack, which in this process aborts it rather than one
/// realm (ADR 1602). A bracket inside a string or a comment is counted too, which errs towards not
/// running a script rather than towards overflowing; the fuzz target `script` skips the same
/// depth.
fn nesting(script: &str) -> usize {
    let (mut depth, mut deepest) = (0_usize, 0_usize);
    for byte in script.bytes() {
        match byte {
            b'(' | b'[' | b'{' => {
                depth = depth.saturating_add(1);
                deepest = deepest.max(depth);
            }
            b')' | b']' | b'}' => depth = depth.saturating_sub(1),
            _ => {}
        }
    }
    deepest
}

/// Evaluates the script a slice at a time, abandoning it where the clock or the steps are spent.
fn evaluate(
    script: &Script,
    context: &mut Context,
    budget: &Budget,
    waited: &Cell<Duration>,
) -> Ending {
    let started = Instant::now();
    let mut spent = 0_u64;
    let mut waker = TaskContext::from_waker(Waker::noop());
    let mut evaluation = pin!(script.evaluate_async_with_budget(context, SLICE));
    loop {
        match evaluation.as_mut().poll(&mut waker) {
            Poll::Ready(Ok(_)) => return Ending::Finished,
            Poll::Ready(Err(error)) => return ending_of(&error, budget),
            Poll::Pending => {
                spent = spent.saturating_add(u64::from(SLICE));
                if spent > budget.steps {
                    return Ending::Exceeded(Exceeded::Steps(budget.steps));
                }
                // The time a question waited on a person is not the script's (ADR 1627).
                if started.elapsed().saturating_sub(waited.get()) > budget.wall {
                    return Ending::Exceeded(Exceeded::Wall(budget.wall));
                }
            }
        }
    }
}

/// How a run that ended in an error ended.
///
/// A budget a guard enforced is read from the record by the caller; this reads Boa's own limits,
/// which arrive as a bare kind and are given the budget's number, and the thrown value.
fn ending_of(error: &JsError, budget: &Budget) -> Ending {
    if let Some(engine) = error.as_engine() {
        return match engine {
            EngineError::RuntimeLimit(RuntimeLimitError::LoopIteration) => {
                Ending::Exceeded(Exceeded::LoopIterations(budget.loop_iterations))
            }
            EngineError::RuntimeLimit(RuntimeLimitError::Recursion) => {
                Ending::Exceeded(Exceeded::Recursion(budget.recursion))
            }
            EngineError::RuntimeLimit(RuntimeLimitError::StackSize) => {
                Ending::Exceeded(Exceeded::Stack(budget.stack))
            }
            other => Ending::Threw(other.to_string()),
        };
    }
    Ending::Threw(error.to_string())
}

/// What a realm keeps in its context's host data: the fields as it was last told of them, and
/// what the current run records.
#[derive(Debug)]
pub(crate) struct State {
    /// The fields, by name.
    table: RefCell<Table>,
    /// The current run's record, borrowed for the length of one statement each time a native
    /// function writes to it.
    record: RefCell<Record>,
    /// Where the realm's questions are put.
    asker: Rc<dyn Asker>,
    /// How long the current run's question waited on its answer, read by the poll loop that holds
    /// the context while the script runs.
    waited: Rc<Cell<Duration>>,
}

/// The document as a realm knows it.
#[derive(Debug, Default)]
pub(crate) struct Table {
    /// Every field the realm was told of, by §12.7.4.2's name.
    pub(crate) fields: BTreeMap<String, FieldState>,
    /// The page the current event happens on.
    pub(crate) page: u32,
    /// How many pages the document has.
    pub(crate) pages: u32,
    /// Whether the view state holds work no save has written.
    pub(crate) dirty: bool,
    /// The document as a whole, as last told.
    pub(crate) document: DocumentState,
}

/// One run's record.
#[derive(Debug, Default)]
pub(crate) struct Record {
    /// The budget, for the guards.
    pub(crate) budget: Budget,
    /// Where the script runs.
    pub(crate) site: Option<ScriptSite>,
    /// The event's field.
    pub(crate) field: String,
    /// Every refusal, first time each member is met.
    pub(crate) refusals: Vec<Refusal>,
    /// `console.println`'s lines, and the library's messages.
    pub(crate) log: Vec<String>,
    /// Every change the script made to a field, in order.
    pub(crate) edits: Vec<ScriptEdit>,
    /// The fields' states before the run's first edit of each, so that a run that does not finish
    /// leaves the realm's table as it found it.
    touched: BTreeMap<String, FieldState>,
    /// The groups as they were before the run's first switch of one, for the same reason.
    pub(crate) layers_before: Option<Vec<pdf_model::view::Layer>>,
    /// The budget a guard stopped the script on.
    pub(crate) exceeded: Option<Exceeded>,
    /// The notes the run owes a report.
    pub(crate) notes: Vec<String>,
    /// Whether the run has put its one question.
    pub(crate) asked_once: bool,
    /// How many questions after the first the run was not let put.
    pub(crate) unasked: u32,
    /// What a script set `this.dirty` to in this run, which it reads back until the run ends.
    pub(crate) dirty_written: Option<bool>,
}

impl State {
    /// An empty table and record, and the realm's asker.
    fn new(budget: Budget, asker: Rc<dyn Asker>, waited: Rc<Cell<Duration>>) -> Self {
        Self {
            table: RefCell::new(Table::default()),
            record: RefCell::new(Record {
                budget,
                ..Record::default()
            }),
            asker,
            waited,
        }
    }

    /// Adds to the time the current run's question waited.
    pub(crate) fn waited(context: &Context, waited: Duration) {
        if let Some(state) = context.get_data::<Self>() {
            state.waited.set(state.waited.get().saturating_add(waited));
        }
    }

    /// The realm's asker.
    pub(crate) fn asker(context: &Context) -> Option<Rc<dyn Asker>> {
        context
            .get_data::<Self>()
            .map(|state| Rc::clone(&state.asker))
    }

    /// Takes in a request's fields and starts a fresh record for it.
    fn begin(context: &Context, request: &Request) {
        let Some(state) = context.get_data::<Self>() else {
            return;
        };
        if let Ok(mut table) = state.table.try_borrow_mut() {
            for field in &request.fields {
                table.fields.insert(field.name.clone(), field.clone());
            }
            table.page = request.page;
            table.pages = request.pages;
            table.dirty = request.dirty;
            if let Some(document) = &request.document {
                table.document.clone_from(document);
            }
        }
        if let Ok(mut record) = state.record.try_borrow_mut() {
            let budget = record.budget;
            *record = Record {
                budget,
                site: Some(request.site),
                field: request.field.clone(),
                ..Record::default()
            };
        }
    }

    /// The record, taken out of the context at the end of a run, its rollback kept.
    fn take(context: &Context) -> Record {
        let Some(state) = context.get_data::<Self>() else {
            return Record::default();
        };
        let Ok(mut record) = state.record.try_borrow_mut() else {
            return Record::default();
        };
        let budget = record.budget;
        let touched = std::mem::take(&mut record.touched);
        let layers_before = record.layers_before.take();
        std::mem::replace(
            &mut *record,
            Record {
                budget,
                touched,
                layers_before,
                ..Record::default()
            },
        )
    }

    /// Puts back every field the stopped run changed.
    fn forget_edits(context: &Context) {
        let Some(state) = context.get_data::<Self>() else {
            return;
        };
        let (Ok(mut record), Ok(mut table)) =
            (state.record.try_borrow_mut(), state.table.try_borrow_mut())
        else {
            return;
        };
        for (name, was) in std::mem::take(&mut record.touched) {
            table.fields.insert(name, was);
        }
        if let Some(layers) = record.layers_before.take() {
            table.document.layers = layers;
        }
    }

    /// Runs `with` on the record, if the context holds one.
    pub(crate) fn with<T>(context: &Context, with: impl FnOnce(&mut Record) -> T) -> Option<T> {
        let state = context.get_data::<Self>()?;
        let mut record = state.record.try_borrow_mut().ok()?;
        Some(with(&mut record))
    }

    /// Runs `with` on the table, if the context holds one.
    pub(crate) fn table<T>(context: &Context, with: impl FnOnce(&mut Table) -> T) -> Option<T> {
        let state = context.get_data::<Self>()?;
        let mut table = state.table.try_borrow_mut().ok()?;
        Some(with(&mut table))
    }

    /// Changes one field in the table through `change` and records the edit — the state before the
    /// run's first change of it kept, so that a run that does not finish can be undone.
    ///
    /// `false` where the realm knows no such field.
    pub(crate) fn edit(
        context: &Context,
        name: &str,
        edit: ScriptEdit,
        change: impl FnOnce(&mut FieldState),
    ) -> bool {
        let Some(state) = context.get_data::<Self>() else {
            return false;
        };
        let (Ok(mut record), Ok(mut table)) =
            (state.record.try_borrow_mut(), state.table.try_borrow_mut())
        else {
            return false;
        };
        let Some(field) = table.fields.get_mut(name) else {
            return false;
        };
        record
            .touched
            .entry(name.to_owned())
            .or_insert_with(|| field.clone());
        change(field);
        if record.edits.len() < MAX_EDITS {
            record.edits.push(edit);
        }
        true
    }

    /// Records an edit that changes no field the table holds — a reset, a recalculation.
    pub(crate) fn note(context: &Context, edit: ScriptEdit) {
        State::with(context, |record| {
            if record.edits.len() < MAX_EDITS {
                record.edits.push(edit);
            }
        });
    }
}

impl Record {
    /// Records a refusal, once per member.
    pub(crate) fn refuse(&mut self, refusal: Refusal) {
        if self.refusals.len() < MAX_REFUSALS
            && !self
                .refusals
                .iter()
                .any(|held| held.member == refusal.member)
        {
            self.refusals.push(refusal);
        }
    }

    /// Records a note, cut to [`MAX_LOG_CHARACTERS`].
    pub(crate) fn note(&mut self, sentence: &str) {
        if self.notes.len() < MAX_NOTES {
            self.notes
                .push(sentence.chars().take(MAX_LOG_CHARACTERS).collect());
        }
    }

    /// Records a logged line, cut to [`MAX_LOG_CHARACTERS`].
    pub(crate) fn log(&mut self, line: &str) {
        if self.log.len() < MAX_LOG_LINES {
            self.log
                .push(line.chars().take(MAX_LOG_CHARACTERS).collect());
        }
    }
}

/// The moment `Date` answers: the request's, the same value throughout a run, so that a script
/// cannot time the host (RFC 0008 section 4.2), and never earlier than a moment it answered before.
#[derive(Debug, Default)]
struct Moment(Cell<u64>);

impl Moment {
    /// Moves the clock to `millis`, or leaves it where a later moment already put it.
    fn set(&self, millis: u64) {
        self.0.set(self.0.get().max(millis));
    }
}

impl Clock for Moment {
    fn now(&self) -> JsInstant {
        let millis = self.0.get();
        JsInstant::new(
            millis / 1000,
            u32::try_from((millis % 1000).saturating_mul(1_000_000)).unwrap_or(0),
        )
    }

    fn system_time_millis(&self) -> i64 {
        i64::try_from(self.0.get()).unwrap_or(i64::MAX)
    }
}

/// The host hooks a realm constructs its context with: no time zone of the engine's own, and the
/// buffer ceiling.
///
/// Boa asks `max_buffer_size` before every `ArrayBuffer` allocation and throws ECMA-262's own
/// `RangeError` for one larger, which a script may catch: the allocation has not happened, so the
/// ceiling holds whether or not it does.
#[derive(Debug, Clone)]
struct Hooks {
    /// The offset local time is read with.
    utc_offset_seconds: Rc<Cell<i32>>,
    /// The largest `ArrayBuffer`.
    buffer_bytes: u64,
}

impl HostHooks for Hooks {
    fn local_timezone_offset_seconds(&self, _unix_time_seconds: i64) -> i32 {
        self.utc_offset_seconds.get()
    }

    fn max_buffer_size(&self, _context: &mut Context) -> u64 {
        self.buffer_bytes
    }

    /// A string compiled while a script runs — `eval`, `Function` — is held to the budgets a
    /// script's own text is held to before it is parsed, since it reaches the same parser and the
    /// same compiler (ADR 1626). Over either, the compile is refused and the run stopped on it.
    fn ensure_can_compile_strings(
        &self,
        _realm: BoaRealm,
        parameters: &[JsString],
        body: &JsString,
        _direct: bool,
        context: &mut Context,
    ) -> JsResult<()> {
        let mut text = String::new();
        for parameter in parameters {
            text.push_str(&parameter.to_std_string_lossy());
            text.push(',');
        }
        text.push_str(&body.to_std_string_lossy());
        let budget = State::with(context, |record| record.budget).unwrap_or_default();
        let Some(exceeded) = too_deep(&text, &budget) else {
            return Ok(());
        };
        let sentence = exceeded.sentence();
        State::with(context, |record| record.exceeded = Some(exceeded));
        Err(JsNativeError::range()
            .with_message(format!("a string compiled at run time: {sentence}"))
            .into())
    }
}

/// A runner a view state can be handed: one document's realm, on a thread of its own, every event
/// run through it under one budget.
///
/// The thread is started at the first event and ends when the engine is dropped; nothing is
/// constructed for a document whose view state never hands one over. It is this process's stand-in
/// for the confined worker of RFC 0008 section 6.2, which holds a [`Realm`] the same way: requests
/// in, outcomes out, one at a time. A realm whose thread has died — a panic reachable in Boa —
/// answers every later event with the sentence saying scripts stopped, and the document stays open
/// (RFC 0008 section 6.8). Keeps a log of every sentence its runs produced, because a view state
/// answering [`pdf_model::view::ViewState::displayed_value`] records nothing of its own (ADR 1591).
#[derive(Debug)]
pub struct Engine {
    /// The budget every run is held to.
    budget: Budget,
    /// Where the realm's questions are put, where the caller supplied somewhere.
    asker: Option<Arc<dyn Asker + Send + Sync>>,
    /// The realm's thread, once started.
    realm: Mutex<Thread>,
    /// Every sentence a run produced, in order, bounded.
    log: Mutex<Vec<String>>,
}

/// The state of an engine's realm thread.
#[derive(Debug, Default)]
enum Thread {
    /// Not started: no event has been handed over.
    #[default]
    Idle,
    /// Running, taking requests, each with the channel its outcome comes back on. Dropping the
    /// sender ends the thread's loop, and so the thread.
    Running {
        /// Where requests go.
        requests: mpsc::Sender<(Request, mpsc::Sender<Outcome>)>,
        /// The thread, held so that it is not detached before its sender is dropped.
        _thread: JoinHandle<()>,
    },
    /// Stopped: the sentence saying why.
    Stopped(String),
}

/// Most sentences an [`Engine`]'s log keeps.
const MAX_ENGINE_LOG: usize = 1024;

impl Engine {
    /// An engine that runs every event under `budget`.
    #[must_use]
    pub fn new(budget: Budget) -> Self {
        Self {
            budget,
            asker: None,
            realm: Mutex::new(Thread::Idle),
            log: Mutex::new(Vec::new()),
        }
    }

    /// This engine with its scripts' questions put to `asker`, which answers on the realm's thread
    /// while the script waits (ADR 1627). The in-process engine's asker answers in the call; the
    /// confined worker's question waits on a person without holding a host's thread, and that is
    /// `pdf_script_worker`'s.
    #[must_use]
    pub fn with_asker(mut self, asker: Arc<dyn Asker + Send + Sync>) -> Self {
        self.asker = Some(asker);
        self
    }

    /// A runner for a view state, under `budget`.
    #[must_use]
    pub fn runner(budget: Budget) -> Arc<dyn ScriptRunner> {
        Arc::new(Self::new(budget))
    }

    /// Every sentence this engine's runs produced, each with its field, in order.
    #[must_use]
    pub fn log(&self) -> Vec<String> {
        self.log
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    /// Runs one request in this engine's realm, starting the realm's thread if it has not started.
    #[must_use]
    pub fn run_request(&self, request: &Request) -> Outcome {
        let mut thread = self.realm.lock().unwrap_or_else(PoisonError::into_inner);
        if matches!(*thread, Thread::Idle) {
            *thread = start(self.budget, self.asker.clone());
        }
        let stopped = match &*thread {
            Thread::Running { requests, .. } => {
                let (reply, outcome) = mpsc::channel();
                match requests.send((request.clone(), reply)) {
                    Ok(()) => match outcome.recv() {
                        Ok(outcome) => return outcome,
                        Err(_) => "the script engine stopped while it ran a script".to_owned(),
                    },
                    Err(_) => "the script engine had stopped".to_owned(),
                }
            }
            Thread::Stopped(why) => why.clone(),
            Thread::Idle => "the script engine did not start".to_owned(),
        };
        let sentence = format!("{stopped}, so scripts no longer run for this document");
        *thread = Thread::Stopped(stopped);
        Outcome::unchanged(Ending::Declined(sentence))
    }
}

/// An asker shared with the thread that built it, as a realm holds one.
#[derive(Debug)]
struct Shared(Arc<dyn Asker + Send + Sync>);

impl Asker for Shared {
    fn ask(&self, question: &crate::Question) -> crate::Answer {
        self.0.ask(question)
    }
}

/// Starts a realm's thread.
fn start(budget: Budget, asker: Option<Arc<dyn Asker + Send + Sync>>) -> Thread {
    let (requests, received) = mpsc::channel::<(Request, mpsc::Sender<Outcome>)>();
    let spawned = std::thread::Builder::new()
        .name("pdf-script realm".to_owned())
        .stack_size(REALM_STACK)
        .spawn(move || {
            let asker: Rc<dyn Asker> = match asker {
                Some(asker) => Rc::new(Shared(asker)),
                None => Rc::new(Nobody),
            };
            let mut realm = Realm::with_asker(budget, asker);
            for (request, reply) in received {
                let outcome = match &mut realm {
                    Ok(realm) => realm.run(&request),
                    Err(why) => Outcome::unchanged(Ending::Declined(why.clone())),
                };
                // A requester that stopped waiting has nothing to be told.
                let _ = reply.send(outcome);
            }
        });
    match spawned {
        Ok(thread) => Thread::Running {
            requests,
            _thread: thread,
        },
        Err(error) => Thread::Stopped(format!("the script engine's thread did not start: {error}")),
    }
}

impl ScriptRunner for Engine {
    fn run(&self, event: &ScriptEvent<'_>) -> ScriptResult {
        let moment = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |since| {
                u64::try_from(since.as_millis()).unwrap_or(u64::MAX)
            });
        let outcome = self.run_request(&Request::of(event, moment, 0));
        let result = outcome.result();
        {
            let mut log = self.log.lock().unwrap_or_else(PoisonError::into_inner);
            let subject = if event.field.is_empty() {
                event.label
            } else {
                event.field
            };
            for sentence in &result.report {
                if log.len() < MAX_ENGINE_LOG {
                    log.push(format!("{subject}: {sentence}"));
                }
            }
        }
        result
    }
}

/// A `NotAllowedError`: RFC 0008 section 4.3's form for a refused call, an exception the script
/// can see and catch, whose message is the refusal's sentence. The refusal is recorded first.
pub(crate) fn refuse(member: String, kind: RefusalKind, context: &mut Context) -> JsError {
    let refusal = Refusal { member, kind };
    let sentence = refusal.sentence();
    State::with(context, |record| record.refuse(refusal));
    let error = JsNativeError::error()
        .with_message(sentence)
        .into_opaque(context);
    // A property the error object was just given cannot refuse to be set; were it to, the error
    // would still be an `Error` carrying the sentence, which is the part a reader needs.
    let _ = error.set(
        JsString::from("name"),
        JsValue::from(JsString::from("NotAllowedError")),
        false,
        context,
    );
    JsError::from_opaque(error.into())
}

//! Boa, constructed for one run, held to a [`Budget`], and read back into an [`Outcome`].
//!
//! # One context per run
//!
//! A run constructs its own context and drops it at the end, so nothing a script leaves behind —
//! a global, a prototype it changed, a closure — reaches the next trigger. That is RFC 0008 section
//! 6.2's isolation inside one process; the realm that persists for a document's lifetime, which
//! `global` and the document-level scripts of §12.6.4.17's name tree need, is the confined
//! worker's to hold (ADR 1590).
//!
//! # How the budgets are enforced
//!
//! Boa offers three limits of its own — loop iterations per call frame, recursion depth, value
//! stack — and those are set from the budget. It has no wall-clock interrupt and no memory
//! ceiling, so the rest are this module's: the script is evaluated with Boa's
//! `evaluate_async_with_budget`, which yields after a fixed number of its cost units, and the poll
//! loop below checks the clock and the step count at every yield and abandons the evaluation
//! where either is spent. The sizes an argument can ask a built-in to allocate are checked before
//! the built-in runs (`guard`). What none of these bounds — work inside one native call that
//! calls back into script, and growth through an operator — is the process's to bound, and ADR
//! 1590 names both.

mod bridge;
mod guard;

use std::cell::RefCell;
use std::future::Future;
use std::pin::pin;
use std::rc::Rc;
use std::sync::{Arc, Mutex, PoisonError};
use std::task::{Context as TaskContext, Poll, Waker};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use boa_engine::context::HostHooks;
use boa_engine::context::time::FixedClock;
use boa_engine::error::{EngineError, RuntimeLimitError};
use boa_engine::{Context, JsError, JsString, JsValue, Script, Source};
use pdf_model::aform::Trigger;
use pdf_model::view::{FieldEvent, FieldResult, ScriptRunner};

use crate::{
    Budget, Ending, Event, Exceeded, Outcome, Refusal, RefusalKind, Request, utf16_offset,
};

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

/// Longest logged line kept, in characters.
const MAX_LOG_CHARACTERS: usize = 1024;

/// Runs one request's script under `budget`.
///
/// Never panics on any input and never leaves the engine running: a run ends finished, over a
/// budget, thrown, unparsed or declined, and [`Outcome::ending`] says which.
#[must_use]
pub fn run(request: &Request, budget: &Budget) -> Outcome {
    if !matches!(request.trigger, Trigger::Keystroke | Trigger::Format) {
        return Outcome::unchanged(Ending::Declined(format!(
            "the field's {} script is not run: this bridge carries a field's keystroke and format \
             scripts (ADR 1591)",
            request.trigger.noun()
        )));
    }
    let mut context = match construct(request, budget) {
        Ok(context) => context,
        Err(error) => {
            return Outcome::unchanged(Ending::Declined(format!(
                "the engine could not be constructed: {error}"
            )));
        }
    };
    let event = match bridge::install(&mut context, request) {
        Ok(event) => event,
        Err(error) => {
            return Outcome::unchanged(Ending::Declined(format!(
                "the host object model could not be installed: {error}"
            )));
        }
    };
    let script = match Script::parse(
        Source::from_bytes(request.script.as_bytes()),
        None,
        &mut context,
    ) {
        Ok(script) => script,
        Err(error) => return Outcome::unchanged(Ending::Unparsed(error.to_string())),
    };
    let ending = evaluate(&script, &mut context, budget);
    let recorded = State::take(&context);
    let ending = match (ending, recorded.exceeded) {
        (_, Some(exceeded)) => Ending::Exceeded(exceeded),
        (ending, None) => ending,
    };
    let mut outcome = Outcome {
        ending,
        refusals: recorded.refusals,
        log: recorded.log,
        ..Outcome::unchanged(Ending::Finished)
    };
    if outcome.ending == Ending::Finished {
        bridge::read_event(&event, request, &mut context, &mut outcome);
    }
    outcome
}

/// A context with this run's clock, hooks, limits and empty record.
fn construct(request: &Request, budget: &Budget) -> Result<Context, JsError> {
    let hooks = Hooks {
        utc_offset_seconds: request.utc_offset_seconds,
        buffer_bytes: budget.buffer_bytes,
    };
    let mut context = Context::builder()
        .host_hooks(Rc::new(hooks))
        .clock(Rc::new(FixedClock::from_millis(request.moment)))
        .build()?;
    let limits = context.runtime_limits_mut();
    limits.set_loop_iteration_limit(budget.loop_iterations);
    limits.set_recursion_limit(usize::try_from(budget.recursion).unwrap_or(usize::MAX));
    limits.set_stack_size_limit(usize::try_from(budget.stack).unwrap_or(usize::MAX));
    context.insert_data(State::new(request, *budget));
    Ok(context)
}

/// Evaluates the script a slice at a time, abandoning it where the clock or the steps are spent.
fn evaluate(script: &Script, context: &mut Context, budget: &Budget) -> Ending {
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
                if started.elapsed() > budget.wall {
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

/// What a run records while it runs, kept in the context's host data.
#[derive(Debug)]
pub(crate) struct State {
    /// The record, borrowed for the length of one statement each time a native function writes
    /// to it.
    record: RefCell<Record>,
}

/// The record itself.
#[derive(Debug, Default)]
pub(crate) struct Record {
    /// The budget, for the guards.
    pub(crate) budget: Budget,
    /// The event's field.
    pub(crate) field: String,
    /// Every refusal, first time each member is met.
    pub(crate) refusals: Vec<Refusal>,
    /// `console.println`'s lines, and the library's messages.
    pub(crate) log: Vec<String>,
    /// The budget a guard stopped the script on.
    pub(crate) exceeded: Option<Exceeded>,
}

impl State {
    /// An empty record for one request.
    fn new(request: &Request, budget: Budget) -> Self {
        Self {
            record: RefCell::new(Record {
                budget,
                field: request.field.clone(),
                ..Record::default()
            }),
        }
    }

    /// The record, taken out of the context at the end of a run.
    fn take(context: &Context) -> Record {
        context
            .get_data::<Self>()
            .map(|state| state.record.take())
            .unwrap_or_default()
    }

    /// Runs `with` on the record, if the context holds one.
    pub(crate) fn with<T>(context: &Context, with: impl FnOnce(&mut Record) -> T) -> Option<T> {
        let state = context.get_data::<Self>()?;
        let mut record = state.record.try_borrow_mut().ok()?;
        Some(with(&mut record))
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

    /// Records a logged line, cut to [`MAX_LOG_CHARACTERS`].
    pub(crate) fn log(&mut self, line: &str) {
        if self.log.len() < MAX_LOG_LINES {
            self.log
                .push(line.chars().take(MAX_LOG_CHARACTERS).collect());
        }
    }
}

/// The host hooks a run constructs its context with: no time zone of the engine's own, and the
/// buffer ceiling.
///
/// Boa asks `max_buffer_size` before every `ArrayBuffer` allocation and throws ECMA-262's own
/// `RangeError` for one larger, which a script may catch: the allocation has not happened, so the
/// ceiling holds whether or not it does.
#[derive(Debug, Clone, Copy)]
struct Hooks {
    /// The offset local time is read with.
    utc_offset_seconds: i32,
    /// The largest `ArrayBuffer`.
    buffer_bytes: u64,
}

impl HostHooks for Hooks {
    fn local_timezone_offset_seconds(&self, _unix_time_seconds: i64) -> i32 {
        self.utc_offset_seconds
    }

    fn max_buffer_size(&self, _context: &mut Context) -> u64 {
        self.buffer_bytes
    }
}

/// A runner a view state can be handed: every `/K` and `/F` Tier 0 does not run goes through
/// [`run`] under one budget.
///
/// Constructs a context per run and holds none, so it is `Send` and `Sync` as `ScriptRunner`
/// requires. Keeps a log of every sentence its runs produced, because a view state answering
/// [`pdf_model::view::ViewState::displayed_value`] records nothing of its own (ADR 1591).
#[derive(Debug)]
pub struct Engine {
    /// The budget every run is held to.
    budget: Budget,
    /// Every sentence a run produced, in order, bounded.
    log: Mutex<Vec<String>>,
}

/// Most sentences an [`Engine`]'s log keeps.
const MAX_ENGINE_LOG: usize = 1024;

impl Engine {
    /// An engine that runs every event under `budget`.
    #[must_use]
    pub fn new(budget: Budget) -> Self {
        Self {
            budget,
            log: Mutex::new(Vec::new()),
        }
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
}

impl ScriptRunner for Engine {
    fn run(&self, event: &FieldEvent<'_>) -> FieldResult {
        let moment = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |since| {
                u64::try_from(since.as_millis()).unwrap_or(u64::MAX)
            });
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
            moment,
            utc_offset_seconds: 0,
        };
        let outcome = run(&request, &self.budget);
        let report = outcome.sentences();
        {
            let mut log = self.log.lock().unwrap_or_else(PoisonError::into_inner);
            for sentence in &report {
                if log.len() < MAX_ENGINE_LOG {
                    log.push(format!("{}: {sentence}", event.field));
                }
            }
        }
        FieldResult {
            rc: outcome.rc,
            value: outcome.value,
            change: outcome.change,
            report,
        }
    }
}

/// A `NotAllowedError`: RFC 0008 section 4.3's form for a refused call, an exception the script
/// can see and catch, whose message is the refusal's sentence. The refusal is recorded first.
pub(crate) fn refuse(member: String, kind: RefusalKind, context: &mut Context) -> JsError {
    let refusal = Refusal { member, kind };
    let sentence = refusal.sentence();
    State::with(context, |record| record.refuse(refusal));
    let error = boa_engine::JsNativeError::error()
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

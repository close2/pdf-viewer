//! The realm's job queue: a promise's reactions and an `async` function's continuations, run before
//! a run's outcome is assembled.
//!
//! ECMA-262, 11th edition (ECMAScript 2020) — the language ISO 21757-1's erratum #185 names —
//! enqueues a promise's reaction as a job through `HostEnqueuePromiseJob` (section 25.6.1.8), and
//! section 8.4 requires every job to be performed at some future point, only when no execution
//! context is running and the execution context stack is empty. Boa enqueues its jobs on the
//! context's executor and runs none of them until the embedder asks, so a realm that never asks
//! performs none of them, ever: every `then` reaction and every continuation after an `await` is
//! silently not run. Which moment a host chooses is the host's; this realm chooses the first one,
//! the moment its script returns, so that what a job does belongs to the run that enqueued it.
//!
//! Promise jobs run first in, first out, as section 8.4.1 requires of `HostEnqueuePromiseJob`'s
//! jobs; the note under section 8.4 leaves the scheduling of any other kind to the host, and Boa's
//! own kinds wait until no promise job is pending. A job is run by Boa's synchronous interpreter, not the sliced one a
//! script is evaluated with, so the step budget cannot see inside one: a job is bounded as a
//! native's call back into script is — by the loop-iteration, recursion and stack limits and by the
//! process — and the wall-time budget is checked between jobs, which is what stops a chain of
//! reactions each of which enqueues the next (ADR 2865).

use std::cell::{Cell, RefCell};
use std::collections::VecDeque;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context as TaskContext, Poll, Waker};
use std::time::{Duration, Instant};

use boa_engine::builtins::promise::PromiseState;
use boa_engine::job::{GenericJob, Job, JobExecutor, NativeAsyncJob, PromiseJob};
use boa_engine::object::builtins::JsPromise;
use boa_engine::{Context, JsError, JsNativeError, JsResult, JsValue};

use crate::{Budget, Ending, Exceeded};

use super::{State, ending_of};

/// The jobs a realm's context has enqueued and not yet run.
#[derive(Debug, Default)]
pub(super) struct Jobs {
    /// Promise reaction and resolve-thenable jobs, section 8.4.1's `HostEnqueuePromiseJob`.
    promise: RefCell<VecDeque<PromiseJob>>,
    /// Boa's generic jobs, run in order after the promise jobs enqueued before them.
    generic: RefCell<VecDeque<GenericJob>>,
    /// Boa's jobs that are futures of its own: a dynamic `import`'s `FinishDynamicImport` (section
    /// 15.2.1.19), and two kinds from editions after the eleventh that Boa also implements, an
    /// `Atomics.waitAsync` and a finalization registry's cleanup.
    native: RefCell<VecDeque<NativeAsyncJob>>,
    /// Timer jobs Boa was handed and this realm does not run: nothing in `boa_engine` itself
    /// enqueues one — only an embedder's own `setTimeout` does, and this realm's timers are the
    /// host's (ADR 1702) — so the count stays zero, and is reported if it ever does not.
    timers: Cell<usize>,
}

impl Jobs {
    /// Forgets every pending job: a run that stopped leaves nothing behind for the next run to
    /// carry out on its behalf.
    pub(super) fn clear(&self) {
        self.promise.borrow_mut().clear();
        self.generic.borrow_mut().clear();
        self.native.borrow_mut().clear();
        self.timers.set(0);
    }

    /// The next synchronous job, promise jobs first.
    fn next(&self) -> Option<Synchronous> {
        if let Some(job) = self.promise.borrow_mut().pop_front() {
            return Some(Synchronous::Promise(job));
        }
        self.generic
            .borrow_mut()
            .pop_front()
            .map(Synchronous::Generic)
    }
}

/// A job Boa runs to completion in one call.
enum Synchronous {
    /// A promise job.
    Promise(PromiseJob),
    /// A generic job.
    Generic(GenericJob),
}

impl JobExecutor for Jobs {
    fn enqueue_job(self: Rc<Self>, job: Job, _: &mut Context) {
        match job {
            Job::PromiseJob(job) => self.promise.borrow_mut().push_back(job),
            Job::GenericJob(job) => self.generic.borrow_mut().push_back(job),
            Job::AsyncJob(job) | Job::FinalizationRegistryCleanupJob(job) => {
                self.native.borrow_mut().push_back(job);
            }
            Job::TimeoutJob(_) | Job::IntervalJob(_) => {
                self.timers.set(self.timers.get().saturating_add(1));
            }
            // `Job` is `#[non_exhaustive]`; a kind a later Boa adds is counted with the timers, so
            // that it is reported rather than lost.
            _ => self.timers.set(self.timers.get().saturating_add(1)),
        }
    }

    /// Refused: this realm runs its jobs itself, under its budget ([`settle`]), and nothing in
    /// `boa_engine` calls this on a context whose module loader loads nothing.
    fn run_jobs(self: Rc<Self>, _: &mut Context) -> JsResult<()> {
        Err(JsNativeError::error()
            .with_message("a realm's jobs are run by the realm, under its budget")
            .into())
    }
}

/// A native job's future, pinned so that it can be polled across rounds of the queue.
type Pending<'a> = Pin<Box<dyn Future<Output = JsResult<JsValue>> + 'a>>;

/// Runs every pending job until none is left, or the wall-time budget is spent, or a job ends in
/// an engine error.
///
/// `started` is when the run began, so that the script and its jobs share one wall-time budget;
/// `waited` is the time a question spent waiting on a person, which is not the script's (ADR
/// 1627). Answers [`Ending::Finished`] where every job ran; a job that throws an ordinary error
/// has already rejected the promise it belongs to, as section 25.6.2.1's `NewPromiseReactionJob`
/// has it — section 8.4 requires a job to complete normally — so only an engine
/// error — a runtime limit — reaches here, and it ends the run as the script's own would.
pub(super) fn settle(
    jobs: &Jobs,
    context: &mut Context,
    budget: &Budget,
    started: Instant,
    waited: &Cell<Duration>,
) -> (Ending, Vec<String>) {
    let mut notes = Vec::new();
    let cell = RefCell::new(context);
    let mut pending: Vec<Pending<'_>> = Vec::new();
    let mut waker = TaskContext::from_waker(Waker::noop());
    let ending = loop {
        if started.elapsed().saturating_sub(waited.get()) > budget.wall {
            break Ending::Exceeded(Exceeded::Wall(budget.wall));
        }
        let ran = match jobs.next() {
            Some(Synchronous::Promise(job)) => Some(job.call(&mut cell.borrow_mut())),
            Some(Synchronous::Generic(job)) => Some(job.call(&mut cell.borrow_mut())),
            None => None,
        };
        if let Some(result) = ran {
            if let Err(error) = result {
                break ending_of(&error, budget);
            }
            continue;
        }
        let started_native: Vec<NativeAsyncJob> = jobs.native.borrow_mut().drain(..).collect();
        for job in started_native {
            pending.push(Box::pin(job.call(&cell)));
        }
        let mut progressed = false;
        let mut failed = None;
        pending.retain_mut(|future| match future.as_mut().poll(&mut waker) {
            Poll::Ready(result) => {
                progressed = true;
                if let Err(error) = result {
                    failed.get_or_insert(error);
                }
                false
            }
            Poll::Pending => true,
        });
        if let Some(error) = failed {
            break ending_of(&error, budget);
        }
        let idle = jobs.promise.borrow().is_empty()
            && jobs.generic.borrow().is_empty()
            && jobs.native.borrow().is_empty();
        if idle && !progressed {
            break Ending::Finished;
        }
    };
    let unsettled = pending.len();
    drop(pending);
    // Every job has run, so a rejection still held is one nothing handled and nothing now can
    // within this run. A run stopped by a budget is reported as stopped and no more.
    if ending == Ending::Finished {
        notes.extend(unhandled(&cell.borrow()));
    }
    if unsettled > 0 {
        notes.push(format!(
            "{unsettled} of the engine's own asynchronous job(s) had not settled when the run \
             ended, and their promises stay pending"
        ));
    }
    if jobs.timers.get() > 0 {
        notes.push(format!(
            "{} timer job(s) the engine enqueued were not run: a document's timers are the \
             host's, through app.setTimeOut and app.setInterval",
            jobs.timers.get()
        ));
    }
    jobs.clear();
    (ending, notes)
}

/// A sentence for each promise the run rejected that no handler took up by the end of its jobs,
/// with the reason it was rejected with, and one for the rejections past what a run follows.
///
/// ECMA-262 section 25.6.1.9 leaves the tracking of rejections to the host; this host says each
/// one, because an error thrown after an `await` that nothing catches is otherwise the one error a
/// run loses without a word (principle 1, ADR 2866). The run is not ended by it: an `async`
/// function turns the throw into its promise's rejection (section 25.7), and the statement that
/// called it completed normally.
fn unhandled(context: &Context) -> Vec<String> {
    let Some((rejected, unfollowed)) = State::with(context, |record| {
        (
            std::mem::take(&mut record.rejected),
            std::mem::take(&mut record.rejections_unfollowed),
        )
    }) else {
        return Vec::new();
    };
    let mut sentences: Vec<String> = rejected
        .into_iter()
        .filter_map(
            |promise| match JsPromise::from_object(promise).ok()?.state() {
                PromiseState::Rejected(reason) => Some(format!(
                    "a promise was rejected and nothing handled it: {}",
                    JsError::from_opaque(reason)
                )),
                PromiseState::Pending | PromiseState::Fulfilled(_) => None,
            },
        )
        .collect();
    if unfollowed > 0 {
        sentences.push(format!(
            "{unfollowed} further promise(s) were rejected with no handler at the time, past the \
             {} a run follows, and whether one was added later is not known",
            super::MAX_REJECTIONS
        ));
    }
    sentences
}

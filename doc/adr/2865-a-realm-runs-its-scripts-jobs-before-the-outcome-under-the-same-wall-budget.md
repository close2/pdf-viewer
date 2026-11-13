# 2865 — A realm runs its script's jobs before the outcome, under the same wall budget

Status: **accepted and built** in `pdf-script` (new: `engine/jobs.rs`; `engine/mod.rs`'s
`Realm::run`; `engine/xfa.rs`'s `FormRealm`). Tests: `crates/pdf-script/tests/promise_jobs.rs`, and
the last two of `crates/pdf-script/tests/xfa_javascript.rs`.
Clauses: ISO 32000-2 §12.6.4.17, through ISO 21757-1's language reference as erratum #185 pins it
(ECMA-262, 11th edition), sections 8.4, 8.4.1, 15.2.1.19 and 25.6. Principle 3 for the budget.

## Context

A realm evaluated each script with Boa's `evaluate_async_with_budget` and assembled the outcome
when it returned. Boa enqueues a promise's reactions on the context's job executor and runs none
of them until the embedder calls `run_jobs`, which nothing here did. So every `then` reaction and
every continuation after an `await` was dropped without a word: `Promise.resolve().then(f)` never
called `f`, and an `async` function stopped at its first `await`. The test that shows it was
written first and failed with `log: ["sync"]` where `["sync", "then"]` was expected.

ECMA-262 section 8.4 requires every job to be performed at some future point, and only when no
execution context is running and the execution context stack is empty; section 8.4.1 requires
`HostEnqueuePromiseJob`'s jobs to run first in, first out. A realm that never drains its queue
performs no job ever, which breaks the first requirement outright. It breaks a second as well:
section 15.2.1.19 requires a dynamic `import()` the host cannot satisfy to reach
`FinishDynamicImport` with an abrupt completion "at some future time", and Boa does that in a job,
so a refused import's promise never settled either. The language the PDF standard names is ES2020
(`doc/todo/56` section 2), which has `Promise`, `async`/`await` and `import()`. Whether Acrobat runs
them is evidence about a reading and not the reading (principle 5).

Which future point is the host's choice. This realm chooses the first one, the moment a run's
script returns, so that what a job does — a log line, a refusal, an edit — belongs to the run that
enqueued it and is carried in that run's outcome.

## Decision

1. **The realm's context gets its own job executor** (`engine::jobs::Jobs`): promise jobs and
   generic jobs first in, first out, and Boa's own future jobs (a refused dynamic `import`'s
   rejection, `Atomics.waitAsync`, a finalization registry's cleanup) polled beside them.
2. **`Realm::run` drains the queue after the script returns, whether it finished or threw**, and
   before the record is taken, so a continuation's log lines, refusals and edits are the run's. A
   run stopped by a budget runs none of its jobs. The queue is cleared at the start and the end of
   every run, so nothing a stopped run enqueued is carried out by the next.
3. **The script and its jobs share one wall-time budget**, checked between jobs, less the time a
   question waited on a person (ADR 1627). That is what stops a chain of reactions each of which
   enqueues the next.
4. **`run_jobs` on this executor refuses**, with a sentence: the realm drains its own queue under
   its budget, and an unbudgeted drain from elsewhere is the thing this decision exists to prevent.
5. **What is not run is said.** A Boa future job still pending when the queue is otherwise empty
   leaves its promise pending and the outcome carries a note with the count; a timer job Boa was
   handed is counted and noted rather than run, since a document's timers are the host's (ADR
   1702) and nothing in `boa_engine` itself enqueues one.

## Cost

- **The step budget cannot see inside a job.** Boa runs a job with its synchronous interpreter, not
  the sliced one, so a job is bounded as a native's call back into script already is: by the
  loop-iteration, recursion and stack limits, by the wall budget between jobs, and by the worker's
  deadline and kill (ADR 1609). A single job that calls a looping function from a loop is not
  stopped in-process until it returns. This widens an existing gap (`engine/mod.rs`'s module comment
  names it) to code after an `await`; it does not open a new kind.
- **The infinite chain is stopped by the clock, not by a count.** Trap 36 prefers the step budget
  because it is deterministic; here the ending is always `Exceeded::Wall`, so which budget fires is
  deterministic even though how many jobs ran first is not.

## What the standard leaves to the host, and this decision leaves as it was

- **An unhandled rejection is not reported.** Section 25.6.1.9's `HostPromiseRejectionTracker` is
  implementation-defined and its default does nothing, so the standard is met; but an error thrown
  after an `await` and caught by nobody leaves no trace in the outcome, where the same error thrown
  before it ends the run as `Ending::Threw`. That is principle 1's question rather than the
  standard's, and it is its own change.

## The XFA realm

`engine/xfa.rs`'s `FormRealm` gets the same executor and drains it in the same place: after the
script returns, before its value is read and before the run's host data is removed, so a
continuation's `rawValue` assignment reaches the host as the script's own would. Two differences
follow from what an XFA run hands back, a value or one sentence:

- **The calculation's value is still the script's completion value** (XFA 3.3 page 379), read
  after the jobs have run; a job does not change it.
- **Item 5's note becomes the run's refusal sentence** (`Run::refused`), where no refusal was
  already recorded, since that is the one place the run reports what its value rests on. A job that
  ends in an engine error ends the run with a sentence saying so.

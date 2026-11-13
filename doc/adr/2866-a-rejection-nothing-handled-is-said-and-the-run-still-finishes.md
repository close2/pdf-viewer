# 2866 — A rejection nothing handled is said, and the run still finishes

Status: **accepted and built** in `pdf-script` (`engine/mod.rs`: `Hooks::promise_rejection_tracker`,
`Record::track_rejection`; `engine/jobs.rs`: `unhandled`). Tests: the last three of
`crates/pdf-script/tests/promise_jobs.rs`, and the last of `crates/pdf-script/tests/xfa_javascript.rs`.
Takes up the item ADR 2865 left under *What the standard leaves to the host*.
Clauses: ISO 32000-2 §12.6.4.17, through ECMA-262 11th edition sections 25.6.1.9 and 25.7.
Principle 1, not the standard.

## Context

Once a realm performs its jobs (ADR 2865), an error thrown after an `await` is thrown inside a job.
The `async` function's body turns it into its promise's rejection (section 25.7), and if no handler
is attached the rejection is the end of it: the job completes normally, the run finishes, and the
outcome says nothing. The same error thrown before the `await` ends the run as `Ending::Threw` and
is reported. So the one error a run could lose without a word was the asynchronous one.

The standard is met either way. Section 25.6.1.9's `HostPromiseRejectionTracker` is
implementation-defined, is called with `"reject"` when a promise is rejected with no handler and
with `"handle"` when a handler is first added to a rejected one, must complete normally, and by
default does nothing. Principle 1 is what is not met: no silent error swallowing.

## Decision

1. **The realm's host hooks implement the tracker.** A `"reject"` holds the promise in the run's
   record; a `"handle"` lets it go. The record is the run's, so a rejection belongs to the run that
   made it and is never said by the next.
2. **What is still held once every job has run is said**, one note per promise with the reason it
   was rejected with: `a promise was rejected and nothing handled it: <reason>`. Only then, because
   a handler added by a later job of the same run handles it, as section 25.6.1.9's `"handle"`
   operation exists to say. A run stopped by a budget is reported as stopped and nothing more.
3. **The run is not ended by it, and its edits stand.** The statement that called the `async`
   function completed normally, and what the script did before and after the rejection happened.
   Turning it into `Ending::Threw` would throw away edits the language says were made. The XFA
   realm has no notes, and there a note becomes the run's refusal sentence (ADR 2865), which ends
   the activation, as an XFA script's own throw does.
4. **A run follows at most as many rejections as it keeps notes** (`MAX_REJECTIONS`, 64). A
   rejection past that is counted, and the count is said with the sentence that whether a handler
   was added later is not known, since a promise not held cannot be checked.

## Cost

- Each followed rejection keeps its promise alive until the run ends, at most 64 of them a run.
- A rejection a page-level `try`/`catch` would have caught in another host's event loop, but that
  is only handled in a *later* run of the same realm, is said by its own run. A realm's runs are
  separate events, and a reader is told about the run that left the rejection unhandled.

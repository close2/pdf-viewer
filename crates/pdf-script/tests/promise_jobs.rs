//! A promise's reaction and an `async` function's continuation run before a run's outcome is
//! handed back.
//!
//! The language is ECMA-262, 11th edition (ECMAScript 2020), the version ISO 21757-1's ISO-approved
//! erratum #185 pins (`doc/todo/56` section 2). Its section 8.4, *Jobs and Host Operations to
//! Enqueue Jobs*, requires every job — a promise's reaction among them (section 25.6.1.8) — to be
//! performed at some future point, and only when no execution context is running and the execution
//! context stack is empty. The realm performs them at the first such point, once the script has
//! returned (ADR 2865): before that a realm performed none of them, ever.
//!
//! Every expected value is the order the standard fixes: no job before the script has returned
//! (section 8.4), and promise jobs in the order their `HostEnqueuePromiseJob` scheduled them
//! (section 8.4.1's first in, first out).

#![cfg(feature = "engine")]
#![expect(
    clippy::expect_used,
    reason = "test code: an explanatory panic is the intended failure"
)]

use pdf_model::view::{ScriptEdit, ScriptSite};
use pdf_script::{Budget, Ending, Event, Exceeded, Outcome, Realm, Request};

/// A request for `script` at the document's open, telling the realm of no field.
fn request(script: &str) -> Request {
    Request {
        site: ScriptSite::OpenAction,
        field: String::new(),
        label: String::new(),
        script: script.to_owned(),
        event: Event {
            value: String::new(),
            change: String::new(),
            selection_start: 0,
            selection_end: 0,
            will_commit: false,
            commit_key: 0,
            field_full: false,
            change_ex: String::new(),
            source: String::new(),
            shift: false,
            modifier: false,
            key_down: false,
            rich_value: String::new(),
        },
        fields: Vec::new(),
        page: 0,
        pages: 3,
        dirty: false,
        document: None,
        view: pdf_model::view::WindowView::default(),
        moment: 1_704_465_015_000,
        utc_offset_seconds: 0,
    }
}

/// Runs `script` as the document's open action in a fresh realm.
fn opened(script: &str) -> Outcome {
    let mut realm = Realm::new(Budget::FIELD_EVENT).expect("a realm");
    realm.run(&request(script))
}

/// A `then` reaction is a job: it runs after the synchronous statements, and before the run ends.
#[test]
fn a_promise_reaction_runs_before_the_run_ends() {
    let outcome = opened(
        "Promise.resolve('then').then(function (v) { console.println(v); }); \
         console.println('sync');",
    );
    assert_eq!(outcome.ending, Ending::Finished, "{outcome:?}");
    assert_eq!(outcome.log, vec!["sync", "then"], "{outcome:?}");
}

/// An `await` suspends its function and its continuation is a job; a chain of them runs to the
/// end before the run's outcome is assembled.
#[test]
fn an_async_function_s_continuation_runs_before_the_run_ends() {
    let outcome = opened(
        "(async function () { \
             console.println('before await'); \
             await 0; \
             console.println('after one await'); \
             await Promise.resolve(); \
             console.println('after two awaits'); \
         })(); \
         console.println('sync');",
    );
    assert_eq!(outcome.ending, Ending::Finished, "{outcome:?}");
    assert_eq!(
        outcome.log,
        vec![
            "before await",
            "sync",
            "after one await",
            "after two awaits"
        ],
        "{outcome:?}"
    );
}

/// What a continuation writes is the run's, as what the script's own statements write is: a page
/// turn made after an `await` is an edit of the run that awaited.
#[test]
fn a_continuation_s_write_is_an_edit_of_the_run() {
    let outcome = opened("(async function () { await 0; this.pageNum = 2; }).call(this);");
    assert_eq!(outcome.ending, Ending::Finished, "{outcome:?}");
    assert_eq!(
        outcome.edits,
        vec![ScriptEdit::GoTo { page: 2 }],
        "{outcome:?}"
    );
}

/// A reaction that throws rejects the promise its `then` returned (section 25.6.2.1's
/// `NewPromiseReactionJob`), and that promise's own reaction sees the error: the run itself
/// finishes.
#[test]
fn a_reaction_that_throws_rejects_its_promise_and_the_run_finishes() {
    let outcome = opened(
        "Promise.resolve() \
             .then(function () { throw new Error('inside'); }) \
             .catch(function (e) { console.println('caught ' + e.message); });",
    );
    assert_eq!(outcome.ending, Ending::Finished, "{outcome:?}");
    assert_eq!(outcome.log, vec!["caught inside"], "{outcome:?}");
}

/// A chain of reactions each of which enqueues the next never empties the queue; the wall-time
/// budget the script runs under is the one its jobs run under, and it stops the run.
#[test]
fn a_chain_of_reactions_that_never_ends_is_stopped_by_the_wall_budget() {
    let outcome = opened("function again() { Promise.resolve().then(again); } again();");
    assert_eq!(
        outcome.ending,
        Ending::Exceeded(Exceeded::Wall(Budget::FIELD_EVENT.wall)),
        "{outcome:?}"
    );
    assert!(outcome.edits.is_empty(), "{outcome:?}");
}

/// A run that was stopped leaves no job behind: the next run in the same realm runs its own
/// script's jobs and nothing the stopped one enqueued.
#[test]
fn a_stopped_run_leaves_no_job_for_the_next_run() {
    let mut realm = Realm::new(Budget::FIELD_EVENT).expect("a realm");
    let stopped = realm.run(&request(
        "Promise.resolve().then(function () { console.println('left over'); }); \
         while (true) {}",
    ));
    assert!(matches!(stopped.ending, Ending::Exceeded(_)), "{stopped:?}");
    let next = realm.run(&request(
        "Promise.resolve().then(function () { console.println('own'); });",
    ));
    assert_eq!(next.ending, Ending::Finished, "{next:?}");
    assert_eq!(next.log, vec!["own"], "{next:?}");
}

/// ECMA-262 section 15.2.1.19: a dynamic `import()` the host cannot satisfy takes the failure path,
/// whose `FinishDynamicImport` rejects the promise the expression returned. A document's realm
/// loads no module, so the rejection is what a script sees, and it sees it within the run.
#[test]
fn a_dynamic_import_is_rejected_within_the_run() {
    let outcome = opened(
        "import('./anything.js').then( \
             function () { console.println('loaded'); }, \
             function () { console.println('refused'); });",
    );
    assert_eq!(outcome.ending, Ending::Finished, "{outcome:?}");
    assert_eq!(outcome.log, vec!["refused"], "{outcome:?}");
}

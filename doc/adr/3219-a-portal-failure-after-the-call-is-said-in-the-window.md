# 3219 — A portal failure after the call is said in the window

Session 2129. Status: accepted. Context: ADR 3202, ADR 1291 (`Submitter`'s shape), ADR 2886
(`Refresher`).

## Decision

`viewer_host::policy::PortalCalls` is one window's set of calls to the desktop portal that are
still out. It has the shape of `Submitter` and `Refresher`: a channel, and a count that the
window's timer reads through `interval`. `PortalCalls::portal` gives a `Portal` whose waiting
thread sends what became of the call: `None` when `gdbus` exited with success, and the sentence
from `not_opened` when it did not. `WindowState::portal_calls` is counted in `looking` and
`answers_interval`, and `take_the_answers` says each failure. The GTK, Qt and C ABI windows
therefore say it in each one's existing way: a status line or a `QUORRA_OWED_SAID`. A failure that
the WinUI window's `Launcher` reports through `quorra_window_open_failed` uses the same sentence
(ADR 3217).

The count is an `Arc<AtomicUsize>`, because an `Opener` is called through `&self`. It goes up
before the thread starts and comes down when the answer is taken in, so it never lags behind the
answers.

## Not changed

`quorra` and `quorra-confined` say everything on the terminal, so the terminal is their window's
way of saying a failure. They keep `Portal::default()`, whose thread prints there, as before.

# 3098 — The GTK window is driven on a registered application that never runs

Session 2099. Status: **implemented.** Context: ADR 3063 ("GTK is left: it has no constructor a test can call
without a running `gtk4::Application`"), ADR 3062, ADR 2936. Clauses: ISO 32000-2 §12.8.3.4.4; ETSI TS 119 172-2
clause 4.22.2.1 b), TS 119 172-3's `TAsInTrustedList`.

## Decision

1. **No second constructor.** `Host::open(app, opening, trace)` needs an application only for
   `gtk4::ApplicationWindow::new`, and GTK asks of that application that it be *registered* — GIO's `startup`, after
   which GTK takes an application's windows — not that it run. A test makes
   `gtk4::Application::new(None, NON_UNIQUE)`, the flags `quorra-gtk` passes and no identifier, so `register` asks no
   session bus for a name, and calls `Host::open` on it. The window is the one the program builds; the test's own
   `MainContext::iteration` calls stand where `Application::run`'s loop would, under `#[gtk4::test]` and `xvfb-run`.
2. **The one seam is visibility.** `Host::key`, which the window's key controller calls, is `pub(super)` so a test in
   the host module presses a key as the controller does. Nothing else in the window changed.
3. **The question is read off the window.** `host/trusted_lists_shift_t.rs` sizes the window (the first allocation
   joins the open, ADR 1539), turns the loop until the document is taken, sets `ask`, and presses Shift + T. The
   question is the modal dialogue the window presents over itself, found among GTK's toplevels by its
   `transient-for`, its text read from its labels and answered by its own buttons: closed unanswered before the
   policy's copy is bound (the calibration: it names no reference), and "Go ahead" after, when it names
   `signed_under_a_policy::REFERENCE` and the refresh starts. Two plants fail it: the references emptied in
   `viewer_host`'s shared `requests::trusted_lists`, and GTK's `WindowAct::TrustedLists` arm dropped.

So all four windows that bind Shift + T drive it on the one generated document, and ADR 3063's GTK remainder is
closed.

## Cost

The test reads the dialogue by walking its widgets, so a reworded or restructured question dialogue is a change to
this test as well. `viewer-gtk` gains `pdf-signature` as a dev-dependency, as `viewer-qt` did, because the generator
states the policy's digest with it.

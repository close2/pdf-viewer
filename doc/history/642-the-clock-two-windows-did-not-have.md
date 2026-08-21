# 642 — The clock two windows did not have

**Finding.** Session 638 gave all three hosts §12.4.4's window and named what it left: "the two
native hosts have the mode and not the clock". So a presentation in GTK or Qt was full screen and
**static** — Table 160's `/Dur` advanced nothing and Table 164's effects did not animate. All three
drive the clock now, on one shared decision, and the boundary needed nothing new for the third
round running.

Date: 2026-08-21. ADR: [0473](../adr/0473-the-clock-two-windows-did-not-have.md).

Touched: `crates/viewer-host/src/clock.rs` (new), `crates/viewer-host/src/lib.rs`,
`crates/viewer-host/Cargo.toml`, `crates/viewer-gtk/src/host.rs`,
`crates/viewer-qt/{src/host.rs,src/bridge.rs,cpp/window.h,cpp/window.cpp}`,
`crates/viewer-ui/src/bin/pdf-viewer/{presentation.rs,window.rs}`,
`crates/viewer-core/src/{presentation.rs,viewer.rs}`,
`crates/viewer-core/tests/sub_page_navigation.rs`, `doc/conformance/ledger.toml`
(§12.4.4, §12.4.4.1, §12.4.4.2), `doc/todo/32`, `doc/todo/30`, `doc/ui-boundary.md`,
`doc/running-the-viewer.md`.

## What the round did

`viewer_host::Clock` is §12.4.4.1's clock: how often to tick, what a tick carries, that the clock
is **held** while a transition is drawn — the subclause's own EXAMPLE puts the effect "[b]efore the
page is displayed" — and when Table 164's `/D` has elapsed. Four questions the clause asks and no
toolkit answers, so they sit where `arrangement` and `presentation` already do. The three event
loops supply only the wall clock: a re-armed `glib` one-shot, one `QTimer` whose interval the host
sets, and winit's `ControlFlow::WaitUntil`.

**`viewer-ui` adopted it and is shorter**, which is the test for taking anything into that crate: a
third host would otherwise have been a third copy. Its `Playing` struct and its `/D` arithmetic are
gone.

**Nothing was added to the boundary and no variant changed shape** — the third round running, and
this one had the shape that usually does ask for a message. `Command::Tick { millis }` has carried
exactly what two hosts needed since ADR 0135.

## Idle means idle

Three answers, because the round was asked for them:

- **Nothing presenting: no timer at all.** `Clock` is an `Option` with no paused state; leaving
  full screen removes GTK's source and stops Qt's `QTimer`.
- **A page stating no `/Dur`: no repaint.** "[T]he page shall not advance automatically", so the
  core produces no events, and both native hosts return without refreshing. A tier-1 refresh copies
  a page of samples into a fresh texture; doing that ten times a second for an unchanged picture
  would have been the defect.
- **Occluded: the timer runs, the drawing does not.** Stated as a choice — `/Dur` is a property of
  the page rather than of anyone watching — and the toolkits do not paint an unmapped window.

## The erratum, and it is not in the clause the round was reading

`spec-errata emit` over clause 12 first, which is the habit six of the last ten rounds have been
repaid for. Nothing new against §12.4.4.1. But it prints two `Text` annotations under the heading
**§12.5.1** whose content is plainly §12.4.4.2's, and the standard's own page settles it: their
icons sit in the right margin of items (b) and (d).

**Issue #304** inserts *If there is no node specified by Next then navigate to the next page. If the
current page is the last page, then the current navigation node remains unchanged*, and the same of
`/Prev`. `viewer_core::presentation` had a paragraph headed *Where the chain ends, there is no
current node, and that is a decision* — and the erratum decides it the other way in both halves. So
a person stepping through a three-bullet slide pressed the key four times for three states and a
page turn, and on the last page the node was cleared where the clause says it stays.

Two callers decline the erratum's page turn and each names the clause that says so: Table 165's
`/Dur` names "the next navigation node" as its destination, and an arrival ends at its own step (c),
"shall make the new page the current page and shall display it".

## What the screen said, and what it could not

Under `Xvfb :79` at 1200×900. Both native hosts photograph a `Wipe` **mid-flight** — blue sweeping
left to right over red, `/Di 0`, with the boundary at about 47% of the width — and both walk the
fixture red → blue → green on `/Dur 2` alone. Escape stops it: two captures three seconds apart are
the same green. `viewer-ui` does the same after the refactor.

What could not be captured is what 638 could not: there is no window manager here, so a window's
*extent* cannot be shown, and `xwd` returns what a window last painted, so a mid-transition frame is
a photograph of whichever frame the toolkit had drawn and not proof of a rate. The rate is asserted
by `viewer-host`'s own tests over an injected `Instant`, which is the instrument that can see it.

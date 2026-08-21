# ADR 0473 — The clock two windows did not have

Status: accepted, 2026-08-21. Session 642.

## Context

Session 638 gave all three hosts §12.4.4's *window* (ADR 0470) and named what it left in one line:

> **the two native hosts have the mode and not the clock** — neither drives `Command::Tick`, so
> `/Dur` and §12.4.4.1's transition frames are still `viewer-ui`'s alone.

So a presentation in `viewer-gtk` or `viewer-qt` was full screen and **static**: Table 160's `/Dur`
never advanced the page and Table 164's effects never animated. Two of three hosts obeyed the
window and one obeyed the clock, which is exactly the shape `doc/todo/30`'s "all three hosts stay
level" exists to prevent — the claim that six consumers have never asked for a new message is
evidence only while every consumer carries what is added.

## What §12.4.4.1 asks of a clock, verbatim

Read against `doc/md/ISO_32000-2_sponsored_EC3.md`, with `cargo run --release -p spec-errata --
emit doc/*.pdf` run over clause 12 first.

The page's duration, Table 160's `/Dur` stated in the subclause's own prose:

> The Dur entry in the page object specifies the page's display duration (also called its advance
> timing): the maximum length of time, in seconds, that the page shall be displayed before the
> presentation automatically advances to the next page.

and its silence, stated rather than left to a reader:

> If no Dur entry is specified in the page object, the page shall not advance automatically.

NOTE 1 gives the other half — "[t]he user can advance the page manually before the specified time
has expired" — which is a key press and not a clock. Table 164 states the effect's own duration,
`/D`, as

> (Optional) The duration of the transition effect, in seconds. Default value: 1.

and NOTE 2 says which page's `/D` is in force: "the transition duration specified for a page (page
2 in the figure) governs the transition to that page from another page". **The one thing that
composes the two is the EXAMPLE**, and it is the whole of the arithmetic in this decision:

> The following example shows the presentation parameters for a page to be displayed for 5
> seconds. Before the page is displayed, there is a 3.5-second transition in which two vertical
> lines sweep outward from the centre to the edges of the page.

*Before the page is displayed.* A transition is therefore not part of the display duration it
introduces, and a clock that ran through one would spend the arriving page's `/Dur` on the effect
that brings it in.

## The decision

**`viewer_host::clock` is §12.4.4.1's clock, and all three hosts drive it.**

`Clock` answers four questions and none of them is a toolkit's:

| the question | the answer | the clause |
|---|---|---|
| how often to look at a wall clock | `Clock::interval` — `RESTING` a tenth of a second, `ANIMATING` a sixtieth | `/Dur` is "in seconds" |
| what a tick carries | `Clock::tick` → the milliseconds that really passed, or `None` | `Command::Tick { millis }` |
| whether the clock runs during a transition | it does not; it restarts where the effect ends | §12.4.4.1's EXAMPLE |
| when the effect is over | `Clock::frame` → `None` at Table 164's `/D` | `/D`, linear, no curve stated |

**It is in `viewer-host` for the reason [`presentation`] and [`arrangement`] are** (ADR 0246, ADR
0442, ADR 0470): `glib::timeout_add_local_once`, `QTimer` and winit's `ControlFlow::WaitUntil`
differ in every letter and agree about every one of the four, and the third copy of a decision is
where two hosts stop agreeing. `viewer-ui` adopted it too and is **shorter** for it — its own
`Playing` struct and its own `/D` arithmetic are gone — so this is one decision in three windows
rather than one decision plus two new ones.

**It needed nothing new on the boundary**, which is `doc/ui-boundary.md`'s claim tested rather than
repeated, and the third round running to answer that way. `Command::Tick` has carried milliseconds
since ADR 0135 and `Event::Transition` has named a transition since ADR 0230; what was missing was
never a channel but two hosts driving the one that existed. No variant changed shape and no
consumer failed to compile.

`viewer-host` gains one dependency, `pdf-render`, and that is the line this decision does **not**
cross: a `Clock` holds two `Image`s and hands back a `DisplayList`, which are values, and the
rasteriser that turns a page into pixels stays in each host, where it already was. The crate has no
widget, no window and no event loop, exactly as [`crate`]'s own header claims.

## What each host does now

| | the timer | the frame | at rest |
|---|---|---|---|
| `viewer-ui` | `ControlFlow::WaitUntil(wake)`, and the *surface's* cadence while animating | the transition's list is presented at the identity transform | one redraw per `Clock::RESTING` |
| `viewer-gtk` | one `glib::timeout_add_local_once`, removed and re-armed at `Clock::interval` | one `gdk::MemoryTexture` at the viewport's origin, in place of `Query::Frame`'s pages | **nothing at all** — see below |
| `viewer-qt` | one `QTimer`, whose interval is `Host::presentation_wait` | `frame_count`/`frame`/`frame_pixels` answer the frame's raster, so the C++ draws it with the code it draws a page with | **nothing at all** |

Three host-side notes worth keeping:

- **GTK re-arms a one-shot rather than repeating.** The interval changes when a transition starts
  and stops, and a `glib` repeating source cannot change its mind; `pump_search` already had this
  shape for a different reason.
- **Qt keeps one `QTimer` and adjusts it**, guarded on `interval() != wait`. A chain of
  `singleShot`s would multiply, because `applyUpdates` runs after every key press as well as after
  every tick — that is the defect this shape was chosen to avoid rather than one it found.
- **A transition frame is answered as *the* frame.** Neither native host grew a second drawing
  path: GTK substitutes one placement for the page's, and Qt's three frame accessors answer from
  the raster in flight. The C++ side of the bridge learned two functions and no new picture.

## A clock that runs when nothing is presenting is a defect

`CLAUDE.md` principle 2 owns this, and the answer is in three parts.

- **When nothing is presenting there is no timer.** `Clock` is an `Option` in every host and there
  is no paused state: leaving full screen drops it, GTK removes its source, and Qt's
  `presentation_wait` answers `-1`, which stops the `QTimer`. A reader who never presses `p` never
  arms anything.
- **When a page states no `/Dur`, a tick repaints nothing.** The clause says "the page shall not
  advance automatically", so the core produces no events at all; both native hosts check that and
  return without a refresh. This is not a micro-optimisation — a tier-1 refresh copies a page's
  worth of samples into a fresh texture, and doing that ten times a second for a picture that has
  not changed would have been a processor spent on a still page.
- **When the window is occluded, the timer still runs and the repaint does not.** That is a
  choice and it is stated as one: `/Dur` is a property of the *page* rather than of anyone
  watching, so a presentation that stopped advancing behind another window would be a clock the
  clause does not describe. What is skipped is the drawing, by the toolkit — GTK and Qt do not
  paint an unmapped window — and what costs anything while occluded is at most one whole-viewport
  rasterisation per transition frame, for the length of one transition.

## What is chosen rather than derived

- **`Clock::RESTING` is a tenth of a second.** `Command::Tick` carries the milliseconds that really
  passed rather than an assumed step, so this decides only when an advance is *noticed* — a tenth
  of a second against a duration a document states in whole ones.
- **`Clock::ANIMATING` is a sixtieth, and it is a floor.** A host with a better clock uses it:
  `viewer-ui` animates on the surface's own cadence (`doc/todo/36`) and never asks.
- **The face of a transition is the page drawn into the whole viewport**, which
  `viewer_core::transition::Frame::draw` states as its own precondition; `viewer_host::face_target`
  is that composition, shared, so that three windows cannot disagree about where the last frame of
  an effect leaves the page.

## The erratum this reading found, and it is not in §12.4.4.1

`spec-errata emit` over clause 12 reports nothing new against §12.4.4.1 — issue #36's *number* →
*integer* on `/Di` was taken in ADR 0470 and issue #75's is a spelling. It reports two `Text`
annotations under the heading **§12.5.1**, and they are not §12.5.1's: that heading opens on the
page they fall on, and their icons sit in the right margin of §12.4.4.2's own items (b) and (d).

**Issue #304**, state Review/Completed, inserts into item (b):

*If there is no node specified by Next then navigate to the next page. If the current page is the
last page, then the current navigation node remains unchanged.*

and the same of `/Prev`, the previous page and the first page into item (d).

`viewer_core::presentation` had a paragraph headed *Where the chain ends, there is no current node,
and that is a decision*, and the erratum decides it — differently, in both halves:

- **The request that runs the last node's `/NA` is the request that turns the page.** This reader
  swallowed it and turned the page on the *next* one, so a person stepping through a three-bullet
  slide pressed the key four times for three states and a page turn.
- **Where there is no page to turn to, the node stays current.** The objection this tree recorded
  against leaving a node current — that its `/NA` would re-execute for ever and no page could be
  turned — does not apply where the clause puts the rule, because on the last page there is no page
  to turn to.

`step` now answers a `Step { actions, onward }`, and **two callers decline `onward`, each with the
clause that says so**:

- **Table 165's own `/Dur`** names "the next navigation node" as its destination, so a node with no
  successor has nothing to advance to and that clock stops rather than turning a page. §12.4.4.1's
  `/Dur` is the one that turns pages, and the two are separate maxima by the standard's own
  construction.
- **An arrival** performs one request against the primary node "as described previously" but its
  paragraph ends at step (c), "[t]he interactive PDF processor shall make the new page the current
  page and shall display it". A page whose only node states no `/Next` would otherwise be arrived
  at and left again in the same breath, and no reader would ever see it.

And §12.4.4.2 states the one other case explicitly, so it is honoured rather than guessed: "[i]f NA
specifies an action that navigates to another page, the following actions for navigating to another
page take place, and Next should not be present" — a node whose actions have already moved the page
is not owed the erratum's turn as well.

## Verified on screen

Under `Xvfb :79` at 1200×900 with `lavapipe`, on `pdf-model/examples/presentation_fixture`.

What could be photographed: `viewer-gtk` and `viewer-qt` both advance the page by themselves while
presenting and both stop advancing the moment Escape leaves. What could **not** be photographed is
what session 638 could not either, and for the same two reasons recorded in `doc/environment.md`:
there is no window manager here, so full screen is a request nobody answers and a window's *extent*
cannot be shown; and `xwd` returns what a window last painted, so a capture taken mid-transition is
a photograph of whichever frame the toolkit had drawn, not proof of a rate. The rate is asserted by
`viewer-host`'s own tests over an injected `Instant`, which is the instrument that can see it.

## Consequences

- `doc/todo/32`'s "all three hosts stay level" item is closed. What that file still owes is the
  five Table 164 styles ADR 0230 refused, unchanged by this round; 638 looked for a clause making
  them reachable and found none, and so did this one.
- `viewer-ui` has less code than before and one fewer private type; the three hosts now differ in
  their event loop and in nothing else about §12.4.4.
- §12.4.4.2's state machine is one key press shorter per page, and correct on the last page, for
  the first time.

[`arrangement`]: ../../crates/viewer-host/src/arrangement.rs
[`presentation`]: ../../crates/viewer-host/src/presentation.rs
[`crate`]: ../../crates/viewer-host/src/lib.rs

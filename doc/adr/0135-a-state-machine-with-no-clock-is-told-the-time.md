# ADR 0135 — A state machine with no clock is told the time

Status: accepted, 2026-08-02. Session 150. The last item of `viewer-core`'s vocabulary.

## Context

ISO 32000-2 §12.4.4.1's `/Dur` was read in the seventieth session and acted on by nobody. The
ledger row said why, and the reason was architectural rather than a shortfall:

> `partial` because nothing plays any of it: there is no presentation mode, no clock and no
> animation.

`viewer-core`'s rule 3 is "no clock", and it is not a limitation to be worked around: a state
machine whose output depends on when it is asked cannot be replayed, and the headless consumer
that proves this crate is toolkit-free would need a thread and a sleep to test anything.

## Decision

**`Command::Tick { millis }`.** A host with a clock tells the viewer that time passed; a host
reading a document sends none, and nothing advances.

That is also the answer to a question this ADR deliberately does not add state for: **there is no
presentation mode in this crate.** Full screen is chrome, chrome is the host's (rule 5), and
"is a presentation running" is answered by whether anything is driving the clock. A host that
stops presenting stops ticking.

Three rules, each from the clause rather than from taste:

- **`>=`, and every page turn restarts the clock.** `/Dur` is "the maximum length of time, in
  seconds, that the page shall be displayed before the presentation automatically advances to the
  next page" — a maximum, not a schedule — and NOTE 1's "[t]he user can advance the page manually
  before the specified time has expired" is then satisfied by the same rule rather than by a
  second one. A page with no `/Dur` swallows every tick, which is the clause's "[b]y default, the
  viewer shall not advance automatically" and not an advance of zero.
- **The page arrived at names the transition.** §12.4.4.1 makes `/Trans` "the transition style
  that shall be used when moving to *this* page from another during a presentation", so it is the
  new page's. It is read from the page tree rather than from `Open::current`, which is filled
  during interpretation — and interpretation happens in `settle`, after this. Reading it there
  would have named the transition of the page just *left*, which is the off-by-one the clause's
  own wording rules out. One page-tree walk per automatic advance, at most one a second, and not
  the per-item loop ADR 0124 was about.
- **A transition is emitted only on a tick-driven advance.** A page turn is part of a presentation
  only when something is driving the clock, and that is the one thing a `Tick` tells this crate
  that an arrow key does not.

## What is still not done, and it is not architecture

The transition is **named, not played**. `Event::Transition` carries Table 164's style, duration
and direction, and a host with a clock draws the frames; a host without one draws the page, which
is the transition's own end state. Nothing here animates, and nothing should: a sequence of frames
over 0.7 seconds is exactly the thing a crate with no clock cannot own.

## What it is worth today, honestly

**Not one page of the corpus's 964 openable documents states a `/Trans` or a `/Dur`** — measured
in the seventieth session and unchanged. This is a clause implemented on the specification's
demand and not the corpus's, which is what `CLAUDE.md`'s two-track rule asks for and what §6.3.2.2
ranks: a rendering processor's obligations are not a function of what a test corpus happens to
contain. The fixture is therefore hand-built, three pages with `/Dur 1` and two different
`/Trans`, and the test asserts the off-by-one directly: page two's `Wipe` is what is named, not
page one's `Split`.

## Consequence

All five of the items `doc/HANDOVER.md` section 0 listed as blocked on the `viewer-core` boundary
are now built, and the vocabulary is complete: `Command` has nothing owed, `Query` has nothing
owed. What is left of every one of those features is a **host** — a presentation player, an
AccessKit bridge, a layer panel — which is what the boundary was built to make possible and is
not itself a boundary question.

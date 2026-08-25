# 749 — The frame nobody will look at

ADR 0650's policy debt, taken: a draw is interrupted exactly where finishing it would produce a
picture the program has already decided it will never show — no clock, no deadline, and no message
owed to `viewer-core`. ADR 0657. Date 2026-08-25.

## The briefing's premise was wrong in one place, and finding out where is the round

ADR 0650 ended *"nothing in this tree is yet a host on this boundary"*, and the round was set up on
that. The boundary the mechanism was built for — `viewer-confined`'s marks arm — has no host, true.
But the interrupt's *question* is asked wherever a display list this program did not write is drawn
by `render-cpu`, and four hosts do that today. `viewer-ui` under `--cpu` has drawn on a thread of
its own since ADR 0461, which is exactly the arrangement a raised flag needs, and nothing raised
one. So the policy was written for that window and measured in it.

The other three are the round's second finding and they are in `doc/todo/30`: `viewer-gtk` and
`viewer-qt` both call `rasterize` **inside** their `Event::NeedsRender` arm, on the toolkit's main
thread, so a page written to draw for 27.6 s takes the window with it and there is no second thread
to raise a flag from. The answer is not a watchdog — that is precisely the automatic deadline §1
refused — it is the drawing thread `viewer-ui` already has.

## Why a deadline was refused, which is a different measurement from ADR 0650's

That one says the tree cannot *predict* a draw. This one says the cost is not the question.
`host_draw --scale 2` over `doc/pdf.js`'s **957** first pages, load 8.4 falling to 7.7: median
**2.2 ms**, p90 **10.2**, p95 **19.6**, p99 **73.9**, slowest **252.1 ms** (`issue1905.pdf`).
**113** of them (11.8%) take longer than one 120 Hz period and **58** (6.1%) longer than one 60 Hz
period. The amplification fixture takes **27 600 ms** out of 1567 bytes.

So the gap a deadline would have to sit in spans two orders of magnitude, and a document's author
picks where in it to sit: a page written for the deadline minus one millisecond passes every time
and is redrawn at every view change. A deadline low enough to catch that refuses one legitimate
first page in sixteen at 60 Hz, and every one of those could have finished — which is the answer
the owner has already ruled out (`CLAUDE.md` §2, `doc/todo/36`, `doc/todo/37`).

At scale 1 the same population is median **0.8 ms**, p99 **28.6**, max **305.2**; 5.9% over 8.3 ms
and 2.3% over 16.7. Both arms are in `tmp/host-draw-749*.tsv`.

## What was built

- `viewer_ui::software::compose_pages_interruptibly` and `was_interrupted`, with `compose_pages`
  delegating to the first with `None` so nothing else in the tree pays the 0.18%.
- `crate::composer`'s `InFlight` — the pages being drawn and the interrupt for them — `Drawn`'s
  three outcomes, and `Composer::superseded`, which is the policy.
- `crate::stale::could_stand_in`, which is `doc/todo/37`'s own question asked of a frame not yet
  drawn, over a `carrying` that `Stale::carries` and `Stale::reproject` now share.
- `crate::surface`: the raise before the ask, and the abandoned frame answered to nobody.

## The measurement, and the unit it had to be taken in

A resize is the gesture: `xdotool windowsize` is a direct request on the window, so it works on a
headless display with no window manager, and a resize is one of the view changes rule 1 interrupts.
One cycle is two resizes 200 ms apart — the first starts a frame, the second lands while it is
being drawn — and the measure is from the tick that first noticed the pair to the rendering the
window comes to rest on. `tmp/amplified2.pdf`, the amplification fixture at two levels (1100 bytes,
a hundred page-covering fills), in an 1150-pixel-tall window, so one frame is most of a second.

| run | s to rest, per cycle | one frame | in frames of work |
|---|---|---|---|
| before 1 | 3.52 3.43 4.60 4.14 | 1.48 s | 2.19 2.50 2.53 3.62 |
| after 1 | 1.10 0.63 0.79 1.06 | 0.59 s | 1.45 1.55 1.56 1.68 |
| before 2 | 0.81 0.97 1.10 0.70 | 0.39 s | 2.18 2.30 2.41 2.61 |
| after 2 | 1.53 2.10 | 1.33 s | 1.34 1.38 |

**Read the last column and not the second.** Three other rounds were running; the load average went
from 3.7 to **72** across the session and one frame of this page cost between 0.39 and 1.48 s. In
seconds, `before 2` beats `after 1` and the change reads as a regression. In frames of work the two
populations do not touch — every before cycle above 2.18, every after cycle below 1.68 — which is
what the argument predicts: two frames of waiting against one frame plus the 200 ms the pair was
spread over. **A duration measured on a shared machine is a measurement of the machine; a ratio
between two durations of the same run is not.**

Every cycle of the after arm raised exactly one interrupt and dropped one frame: 1.56 s of drawing
thrown away over six cycles, against the frames of waiting it bought.

**The shipped binary was run once more after the last edit** (`tmp/smoke-749.log`), because the
`Stages::composed` change landed after the table above: three cycles, three interrupts, three
`drawn and dropped` lines, and the frame line after each of them now carries no `composed` column
where the measured runs' did. Trap 1's sentence one directory over — the instrument that says a
change happened is not the change.

An earlier attempt on a real corpus page (`issue1905.pdf`, ~50 ms a frame) is in
`tmp/pair-*.log` and could not answer: the stand-in refusal line the instrument keys on repeats at
**every tick** until the frame lands, so the interval it measures is one tick whatever the host is
doing. That is why the fixture is a second a frame and the reference is the *first* refusal of a
cycle rather than the last.

## What could not be checked here

- **That the policy is right for the confinement's host**, because there is still no host on that
  boundary. What is established is that it is right for a host that draws marks it did not write,
  which is the same question.
- **The owner's abort.** Not built, and ADR 0657 §5 says why rather than pleading time: it needs an
  input, an input is `viewer_host::keys`' shared table, and rule 1 already keeps the window
  responsive while a hostile page draws — so it is a convenience rather than the bound.
- **Anything on a real display.** `Xvfb` and a software surface throughout; `--cpu` is the arm this
  policy is about, so no adapter was involved.

## Instruments left behind

`tmp/` holds the scripts, named for the round rather than committed because they are about a
machine: `amplify-749.py` writes the *file* — the construction is
`crates/viewer-confined/tests/support/amplification.rs`'s, which builds the same bytes in memory
and says why the document itself is deliberately not committed — `amplified-749.sh` drives one arm
and `read-amplified-749.py` reads a trace. What is committed is the four tests ADR 0657 §9 names.

## Gates

Whole, as a change that can reach a pixel through `viewer-ui`'s own compositing.

## Ledger

Untouched. `CLAUDE.md` principle 3 against principle 2, citing no clause.

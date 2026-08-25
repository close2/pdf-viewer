# ADR 0657 — The frame nobody will look at

Status: accepted, 2026-08-25. Session 749. Takes the policy half of the debt ADR 0650 left, and
the finding about `viewer_core::Rendered::Failed` that goes with it.
Cites no clause: this is `CLAUDE.md` principle 3 against principle 2, and the ledger is untouched.

## What was owed

ADR 0650 built `pdf_render::Interrupt` and `render_cpu::CpuRasterizer::interruptible`, measured
what a raise costs and what it buys, and stopped: *"[n]othing in this tree decides when to raise
an interrupt, because nothing in this tree is yet a host on this boundary."*

**The second half of that sentence is wrong, and finding out why is most of this round.** The
boundary the mechanism was built for is `viewer-confined`'s, where a page crosses as marks and an
unconfined host draws them — and no host takes that road yet. But the *question* an interrupt
answers is not about the confinement at all. It is asked wherever a display list this program did
not write is drawn by `render-cpu`, and this tree has **four** such hosts today:

| host | draws on | can another thread raise a flag it would honour? |
|---|---|---|
| `viewer-ui` under `--cpu` | `crate::composer`'s thread (ADR 0461) | **yes**, and nothing did |
| `viewer-gtk` | the toolkit's main thread, inside `Event::NeedsRender` | no — there is no other thread |
| `viewer-qt` | the same | no |
| `viewer-ffi` | the C caller's thread, by design | not through this ABI |

So the policy has a host, it is `viewer-ui`'s `--cpu` window, and what the other three need is
recorded below rather than guessed at.

## 1. What a deadline would have to separate, and why nothing can

ADR 0650 section 2 established that nothing in this tree *predicts* what a display list will cost:
the only pre-draw estimate correlates with the measured draw at 0.115 by Pearson. This round asks
the weaker question that a deadline actually needs — not *how long will this page take* but *is
there a duration above every legitimate page and below every hostile one* — and the answer is no,
for a reason that is about the adversary rather than about the estimate.

`examples/host_draw` over `doc/pdf.js`'s first pages, at twice device scale, load average 8.4
falling to 7.7 over the run:

| | |
|---|---|
| pages priced | **957** |
| median | **2.2 ms** |
| 90th / 95th / 99th percentile | **10.2 / 19.6 / 73.9 ms** |
| slowest of the 957 | **252.1 ms** (`issue1905.pdf`) |
| over one 120 Hz period (8.3 ms) | **113**, 11.8% |
| over one 60 Hz period (16.7 ms) | **58**, 6.1% |
| over 100 ms | **6**, 0.6% |
| the amplification fixture, 1567 bytes | **27 600 ms** (ADR 0650) |

**A deadline set anywhere between 252 ms and 27.6 s refuses no corpus page and stops nothing.** A
document's author picks its cost: a page written to draw for the deadline minus one millisecond
passes every time, and is redrawn at every view change, which is a sustained refusal of the frame
rate rather than a hang and is worse for being invisible. A deadline set low enough to catch that
one — say a frame period — refuses **one legitimate first page in sixteen** at 60 Hz, and every one
of those could have finished. The project owner's rule is that a correct frame is wanted wherever
it is achievable, with reprojection for the ones that are missed (`doc/todo/36`, `doc/todo/37`), so
a page interrupted that could have finished is the worse of the two answers.

So the two populations are not separated by *duration*, and a policy that reads a clock is picking
a number that does not decide anything. **This is not the same finding as ADR 0650 section 2** and
it is stronger: that one says the tree cannot predict a cost, this one says the cost is not the
question.

## 2. The policy: one rule, and it reads no clock

> **A draw is interrupted exactly where finishing it would produce a picture this program has
> already decided it will never show.**

That is a question about *want* rather than about cost, and the host can answer it exactly. It is
also not a new question: `doc/todo/37`'s stand-in policy has been deciding which pictures are worth
anything for eight rounds, and its answer is `crate::stale`'s. A frame that has landed is worth
something if it is the view being asked for (`depicts`) or if one affine carries it onto that view
(`carrying`, over `AGREEMENT` — half a device pixel, derived in ADR 0444). `crate::stale::could_stand_in`
asks that of a frame **still being drawn**, and `Composer::superseded` raises the interrupt where it
answers `false`.

What that comes to, gesture by gesture:

| the person does | the frame in flight | so |
|---|---|---|
| nothing | is of the view being asked for | never interrupted |
| scrolls | carries onto the new view; it is the next stand-in's base | **not** interrupted |
| zooms a single page | carries | **not** interrupted |
| turns a page | is of another page — `Refusal::AnotherPage` | interrupted |
| resizes the window | is of another window — `Refusal::Resized` | interrupted |
| zooms a column | `Refusal::Rearranged`: the gap does not scale, so no one affine carries it | interrupted |
| toggles a §8.11 layer | is of superseded ink, and `Picture` says a re-interpretation is another picture | interrupted |

**The scroll row is the one that makes this a policy rather than a reflex.** *Interrupt whenever
the view moved* is the simpler rule and it is wrong: a superseded scroll frame is thrown away as a
*present* and kept as a *base*, and every stand-in until the next real frame is drawn from it. Rule
1 as written spends the thread only on frames that are worthless in both roles.

## 3. What the viewer is told, which is nothing — and why that had to be established

ADR 0650 section 6 predicted this would need a message: *"a host that raises an interrupt has to
tell the viewer that the render failed, or the stand-in becomes permanent."* Reading
`viewer_core::Viewer::rendered` says the opposite, and the reasoning is worth keeping because the
prediction was reasonable:

- `Rendered::Failed` sets `on_screen.shown` to the pending target and revision. That is
  **deliberate** and its own comment says why — a rasteriser that refused this page at this size
  will refuse it again, so recording the refusal as an answer is what stops the scheduler and the
  host spinning. What changes the answer is the *question* changing.
- So a host that reported an abandoned draw as `Failed` would mark the page shown for good. The
  person would be left with a stand-in and a status line blaming the document for a decision the
  host made about its own thread.
- And a token that is answered **not at all** is not re-asked either: `settle` skips a page whose
  `pending` matches the target and revision being asked for.

Both are now asserted by `viewer-core/tests/headless.rs`'s
`a_refusal_is_final_for_this_view_and_a_token_never_answered_is_not_re_asked`, because they are
what the policy rests on and neither was written down anywhere a host could read it — and the
lesson is `doc/traps/the-interactive-loop.md`'s **trap 20**, because it binds any host on this
boundary and not only this one.

**So an abandoned draw produces no `Rendered` at all, and no message is owed.** What a tick tells
the core is decided by what that tick puts on the window, exactly as it was before this change:
this host is tier 2 and acknowledges its outstanding tokens with `Presented` when a frame reaches
the window. What it must never say is `Failed`, and that is the whole of the rule. `Stale` is not
told either: no pixels of the abandoned view exist, and recording it as settled would have the
next stand-in reproject from a picture that was never drawn, and would feed rule 5's prediction a
frame that did not finish.

## 4. Does an interrupted page ever get finished?

Yes, and the reason there is no livelock is structural rather than a bound:

- **The only thing that raises an interrupt is a view change that has already been made.** A job
  cannot supersede itself, and the job that replaces the abandoned one is the one the person is
  looking at.
- **Progress is guaranteed while the view comes to rest.** The tick that raises is the tick that
  asks; the interrupted thread returns within one command — 1.3 to 2.1 ms against a 2.76 ms
  command, ADR 0650 — and the tick that collects it sends the new job. So the cost of a raise is
  one period plus one command, and after the last gesture the frame being drawn is the right one.
- **A view that never comes to rest never lands a frame, and never would have shown one.** Every
  tick of a gesture is a stand-in by `doc/todo/37`'s own rules whatever the composer is doing; what
  changes is that the thread is no longer spending the gesture on pictures that were going to be
  discarded.

What is **not** claimed is that an interrupted draw is resumed. `CpuRasterizer::rasterize` starts
at the first command every time, so an abandoned frame's work is lost. That is what rules out the
*growing allowance* a briefing might reach for — a deadline that doubles on each retry — as well:
with no resumption a page needing `T` is redrawn from scratch at `p, 2p, 4p …`, so it lands at up to
`2T` and costs up to `2T` of thread, to buy a responsiveness rule 1 gives for nothing. It would be
worth revisiting only against a resumable rasteriser, which is a different item.

And the *first frame* is not the exception it looks like. It is the frame with no stand-in at all —
no base, no retained page — so interrupting it shows the person nothing whatever. That is an
argument for a **longer** leash on the first frame, not a shorter one; rule 1 gives it one for free,
because before a first frame lands there is no earlier view to have moved away from.

## 5. What the owner's callback is, and what it still needs

`doc/todo/10`'s brief — *"the UI could provide a callback warning the user and allowing the user to
abort — however don't block and wait for the user"* — is the second raiser and it is **not built
here**, deliberately rather than for want of time:

- Rule 1 already leaves the program responsive while a hostile page draws: the window keeps its
  cadence, the person can scroll, turn the page, close the document or quit, and each of those
  takes the drawing thread back within a command. So an abort is a convenience rather than the
  thing standing between a document and the machine, which is what it would have been before rule
  1.
- What it needs is an input, and an input is `viewer_host::keys`' table — which all three windowed
  hosts share by `doc/todo/30`'s levelness rule. Adding a key to one of them is the third-copy
  failure that crate exists to prevent.
- And the threshold at which the offer is *made* is a human-factors number this tree does not
  measure. What section 1 gives it is a floor rather than a value: below 252 ms it would fire on
  ordinary documents. The asymmetry that makes a choice acceptable there and not for a budget is
  worth stating — **a wrong budget refuses a page a person is entitled to see, and a wrong offer
  shows a sentence early.**

`doc/todo/15` carries what is left.

## 6. Where the policy lives, and where it does not

In `crate::composer` and `crate::stale`, both private modules of the `pdf-viewer` binary. **Not in
`viewer-host`**, although it is a host's decision, and the reason is that crate's own stated test:
what belongs in it is what two hosts would otherwise write twice, and there is not yet a second
host that *can* raise one. `viewer-gtk` and `viewer-qt` rasterise inside their event handler, so
there is no thread to raise the flag from and nothing for it to interrupt but the loop that would
raise it; giving them a drawing thread is `doc/todo/30`'s work and the policy moves when it lands.

`crate::stale::could_stand_in` is in `stale.rs` rather than in the composer for the sharper version
of the same rule: what makes a superseded frame worth finishing is exactly what makes its pixels a
stand-in, and that is decided by the five rules in one file (`doc/todo/37`'s rule 2 — nothing
outside the binary may name a stand-in).

## 7. What it buys, measured in the window rather than argued

The gesture is a **resize**, because it is the one an instrument can drive on a headless display
without a window manager (`xdotool windowsize` is a direct request on the window rather than a hint
to one) and because it is one of the view changes rule 1 interrupts. One cycle is two resizes 200
ms apart: the first starts a frame, the second arrives while that frame is being drawn, and what is
measured is from the tick that first noticed the pair to the rendering the window comes to rest on.
Both arms alternate in one sitting, on `tmp/amplified2.pdf` — the amplification fixture at two
levels, 1100 bytes and a hundred page-covering fills — in an 1150-pixel-tall window, so that one
frame is most of a second rather than tens of milliseconds.

**The number is *frames of work* rather than seconds, and that is not a dodge.** Three other rounds
were running: the load average went from 3.7 to 72 over the session and one frame of this page
cost between 0.39 and 1.48 s across the four runs. What the policy changes is not how long a frame
takes — it costs 0.18% — it is **how many frames the window waits through before it shows the shape
the person asked for.** Dividing by the resting frame's own cost, which the trace prints, is what
takes the machine out of the comparison. The seconds are in the round's history file.

| | frames of work, one per cycle | median | interrupts | abandoned |
|---|---|---|---|---|
| **before**, two runs | 2.18 2.19 2.30 2.41 2.50 2.53 2.61 3.62 | **2.46** | 0 | none |
| **after**, two runs | 1.34 1.38 1.45 1.55 1.56 1.68 | **1.50** | one per cycle, always | 6, 1.56 s of drawing dropped |

**The two populations do not overlap** — every *before* cycle is above 2.18 and every *after* cycle
below 1.68 — and the numbers are the ones the argument predicts rather than a surprise: without the
interrupt the window finishes the first resize's frame and *then* draws the second's, so it waits
two frames; with it the first is abandoned and the wait is one frame plus the 200 ms the pair was
spread over. The trace says the same thing in words — `the frame being drawn is of a view this one
could not stand in with, so the composing thread was interrupted`, then `the interrupted frame came
back after 266.2 ms, drawn and dropped`.

**And the seconds would have said the opposite**, which is why the unit is worth the paragraph
above: the second *before* run came to rest in a median of 0.89 s and the first *after* run in
0.92, so a table of durations would have reported this change as a small regression. Its frames
cost 0.39 s against the other's 0.59, because the machine was carrying four rounds and the load
average moved between them. **A duration measured on a shared machine is a measurement of the
machine**; a ratio between two durations from the same run is not.

**What a person sees is unchanged for the whole of that time and that is the point**: both arms
show `doc/todo/37`'s stand-in — a retained low-resolution page under the placement this view
gives it, at a resample every refresh — for as long as the frame takes. What differs is when the
sharp picture replaces it.

## 8. What it costs

Handing an interrupt over costs a draw **0.18%** and the path with none costs 867 instructions a
page (ADR 0650 section 5, callgrind, three arms in one sitting). Every `--cpu` frame now hands one
over, so that is what this window pays; the gates pay nothing, because `compose_pages` is
`viewer-ui`'s and every gate in this tree rasterises through `CpuRasterizer::new()` directly.
`compose_pages` is unchanged as a signature and delegates to `compose_pages_interruptibly(_, None)`,
so the `Option` is `None` on every other caller.

## 9. The instruments

- `cargo run --release -p viewer-confined --example host_draw -- --scale 2 doc/pdf.js/test/pdfs/*.pdf`
  — section 1's distribution.
- `crates/viewer-ui/src/bin/pdf-viewer/stale.rs`'s
  `a_frame_is_worth_finishing_exactly_where_its_pixels_could_still_stand_in` and
  `a_scroll_of_a_column_is_worth_finishing_and_a_zoom_of_one_is_not` — rule 1's four cases and the
  pair that separates it from *interrupt whenever the view moved*.
- `crates/viewer-core/tests/headless.rs`'s
  `a_refusal_is_final_for_this_view_and_a_token_never_answered_is_not_re_asked` — section 3.
- `crates/viewer-ui/src/software.rs`'s
  `a_raised_interrupt_refuses_the_arrangement_and_says_which_refusal_it_is` — the mechanism through
  the arrangement, and that the refusal is legible as itself.
- The window itself: `--cpu --trace=frames` prints a line when the composing thread is interrupted
  and a line when the abandoned frame comes back. Section 7's run is `Xvfb`, `xdotool windowsize`
  and the two-level amplification fixture; the round's history file has the scripts and the
  seconds.

**One reporting change went with it**, and it is a measurement decision rather than a tidy-up:
`Stages::composed` is no longer set for an abandoned draw. That field is a row of the trace
summary's percentiles and what those describe is what a *frame* costs the composing thread; a draw
stopped part way through is a sample of nothing, and letting it in would pull `doc/todo/36`'s
number down by however far through the page the interrupt landed. What it cost is said in its own
line instead. Section 7's runs were taken before that change and it moves no timing — the "two
renderings a cycle" the scripts report in the *after* arm is the abandoned frame being counted, and
after it there is one.

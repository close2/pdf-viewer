# ADR 0650 — The draw a kill does not reach

Status: accepted, 2026-08-25. Session 745. Takes the host-side debt ADR 0640 wrote down.
Cites no clause: this is `CLAUDE.md` principle 3 against principle 2, and the ledger is untouched.

## What was owed

ADR 0633 made a page cross the confinement as **marks** where the marks are smaller, and ADR 0640
stopped the confined worker drawing what it does not send. Both were plainly right and the second
is worth 8.73 → 2.82 ms on a sparse page and 26.5 s → 18 ms on a fixture. What ADR 0640 exposed
rather than caused is one sentence:

> **A cancel stops the work the worker does, and on the marks arm that is the interpretation, not
> the drawing.**

`doc/todo/34` §3 settled that the worker's cancel **is a kill**, and the argument is sound: the
process being stopped is interpreting a hostile document, so a cancel it has to *agree* to is one
the document can decline. That argument does not transfer one inch, because on the marks arm the
expensive thing does not happen in that process at all. It happens in the **host**, unconfined, on
the host's own thread — and `Canceller::cancel` there would end a worker that has already answered
and gone quiet.

So the question this round was given: *what bounds the host's own draw, and can it be stopped?*

## 1. What bounds it — the enumeration, because this tree has guessed wrong about a bound before

Every layer between a hostile document and the host's rasteriser, read rather than assumed:

| layer | bounds | value |
|---|---|---|
| the frame header | the whole **message** | `protocol::MAX_MESSAGE`, 2 GiB |
| `display_list::count` | each table's count against the bytes behind it | `least::COMMAND` is **1 byte** |
| `protocol::decode`'s target check | the **target**, before it is an allocation | `viewer_core::MAX_PIXELS`, 2²⁸ |
| `TargetSpec::for_page`, `CpuRasterizer::rasterize` | either **dimension** | `MAX_EXTENT`, 2²⁴ |
| both decoder and backend | **nesting** | `MAX_GROUP_DEPTH`, 16 |
| any of them | **the work** | nothing at all |

The last row is the finding. Before this round no rasteriser in this tree — `render-cpu`,
`render-gpu`, `render-quorra` — contained a cancel, an interrupt, a deadline or an abort, and
`Rasterizer::rasterize` has no way in and no early way out: `&mut self`, a list, a target, and a
`Result` at the end of it.

**`render-quorra`'s budget refusals are not the exception they look like.** That budget is the
*device's resources* — atlas bytes and retained tiles — so a refusal there says a frame's
resources do not fit. It says nothing whatever about how long a frame that does fit will take.

**What the first two rows multiply out to** is the reason this is a principle 3 question rather
than a tidiness one. The fixture's marks are 990 453 bytes for 10 000 page-covering fills — **99
bytes a fill** — and they draw in **27.6 s** at a 900×1165 window, which is **2.76 ms a fill**. A
message the wire already permits is 2 GiB, so it is 21.7 million such fills, and drawing it is
**about seventeen hours**. Every term of that is measured; none is picked.

## 2. There is no budget to derive, and that is a measurement rather than a preference

The briefing's instruction was to derive a budget rather than pick one, on ADR 0597's model. The
attempt was made and it failed, which is worth more than the budget would have been.

A budget needs a *predictor*: something a host can compute from a display list and a target,
before drawing, that says roughly what the draw will cost. This tree contains exactly one
candidate and it is already computed on **every** CPU draw — `pdf_render::row_costs` over
`pdf_render::command_extents`, summed, which is the estimate `plan_strips` cuts the strips by and
then throws away. It is the number a host would reach for.

`viewer-confined`'s `examples/host_draw` prices it against the measured rasterisation over
`doc/pdf.js`'s 958 first pages:

| | |
|---|---|
| Pearson correlation of the estimate with the time | **0.115** |
| Spearman | **0.649** |
| the same two over the 952 pages whose target is under 3 Mpixels, so that a giant `/MediaBox` cannot be the confound | **0.161**, **0.650** |
| of the 40 slowest pages, how many are among the 40 with the most estimated work | **8** |
| `personwithdog.pdf` — 484 704 pixels, the page painted **0.2** times over | **162.4 ms** |
| `pattern_text_embedded_font.pdf` — 501 832 pixels, the page painted **593.5** times over | **15.9 ms** |

**The last two rows are the argument and the rest is arithmetic around them.** Two pages of
essentially the same size: the estimate puts them three thousand times apart, the clock puts them
ten times apart *the other way round*, and no amount of load on the machine reaches three orders
of magnitude. Nor is it a curiosity of one pair — `issue12295.pdf` is a page-sized target painted
3.5 times over that draws for **493.9 ms**, and `bug1721218_reduced.pdf` is 0.3 times over and
**147.1 ms**.

**That is not a defect in the estimate.** `pdf_render::strips` says so in its own module comment:
it "ignores edge building, which is proportional to a path's complexity rather than to its
bounding box", and it exists to choose *where* to cut, where an error that applies to every row
alike cancels. Read as a predictor of a time it is close to useless, and the pairs above are the
demonstration.

So a budget over it would refuse legitimate pages and admit hostile ones, which is `doc/todo/10`
§6's first rule — *nothing arbitrary may be replaced by something equally arbitrary* — arriving
from the direction nobody expects: the arbitrary thing would have had a measurement behind it and
would still have been arbitrary, because the measurement does not measure the thing.

**The general form is worth keeping.** A number a hot path already computes is not thereby a
number that answers a different question, and this one carries its own disclaimer in the crate
that produces it.

## 3. The decision: `pdf_render::Interrupt`, and it is not a `Canceller`

A flag one thread raises and a drawing loop honours. `render_cpu::CpuRasterizer::interruptible`
takes one; the check is a `Relaxed` load at the top of `encode`'s command loop, and every
recursion — a group, a shaped pair, a soft mask's own list — comes back through that loop, so one
check covers all of them. `BackendError::Interrupted` is what a refused draw says.

**The name is the argument.** A `Canceller` *ends a process*: it is what a hostile document cannot
decline, and it costs the document, the worker and every answer the worker was holding. An
`Interrupt` is *raised* and *honoured*: it costs one draw, the worker is never told, and the
document stays open. Calling both of them a cancel would erase the distinction `doc/todo/34` §3
spent a round establishing — and would invite the reasonable-sounding question *why is a
cooperative cancel good enough here and not there?*, whose answer is that **the loop is ours**. A
hostile document reaches `render-cpu` as a `DisplayList`, which is data: it can make the loop long
and it cannot make an iteration of it decline to check.

**Where it is not offered is as deliberate as where it is.** The device backends take no
interrupt, rather than taking one and ignoring it. A submitted frame is the driver's, and a flag
in this process cannot recall it; offering the method would be a promise this tree cannot keep, so
a host handing an interrupt to a device backend fails to compile instead of failing to stop.
That is `doc/traps/parsers-and-streams.md` trap 5 applied to an API.

## 4. What it reaches, and the three things it does not

- **It reaches the command loop**, which is the unbounded quantity. That is the whole point.
- **It does not interrupt one command.** The check is between commands, so a raise waits for the
  page-covering fill in progress — measured at **1.3, 1.9 and 2.1 ms** over three runs against a
  2.76 ms command, which is exactly the granularity claimed and no better.
- **It does not interrupt the planning pass.** `command_extents`, `row_costs` and
  `unsplittable_rows` run before the first check; `examples/host_draw` prints what they cost, and
  on the fixture it is **2.4 ms against a 14.5 s draw**. It is `O(commands × rows)` and therefore
  unbounded in principle, which is worth knowing rather than hiding — it is three orders of
  magnitude below the drawing it plans because it touches a row rather than blending it.
- **It does not interrupt the per-pixel pass** after the drawing — the medium composite and the
  crop. Both are `O(target area)`, which `MAX_PIXELS` bounds, and that is the point: what an
  interrupt is for is the quantity nothing bounds.

## 5. What it costs

Instructions rather than wall clock, because the difference being asked about is one relaxed
atomic load per command and the machine's load average was above twenty. `callgrind_rasterise`,
ISO 32000-2 page 101 — the densest page in the tree at 3007 commands — drawn 20 times, three arms
in one sitting:

| arm | instruction refs | against the first |
|---|---|---|
| the tree **before** this change, built from an export of `HEAD` | 5 441 579 467 | — |
| after it, **no** interrupt handed over | 5 441 596 808 | **+0.0003%** |
| after it, an interrupt handed over and never raised | 5 451 627 652 | **+0.18%** |

The middle row is the one the gates care about: **867 instructions a draw**, against a page of
3007 commands replayed into as many as sixteen strips. That is far short of one check per command
visit, so the check is plainly not being paid per command where the field is `None` — which is
what a `&self` `Option` is in a position to buy and what a flag behind a pointer would not be. The
mechanism is an inference; the number is not. **The path every gate in this tree runs is
unchanged**, and the 0.18% is paid only by a caller that asked to be able to stop.

And byte equality is asserted rather than assumed: `render-cpu`'s
`a_draw_with_an_unraised_interrupt_is_the_draw_without_one` draws seven scenes both ways and
compares every byte. This backend is the oracle every corpus and oracle verdict is taken from, so
a check in its command loop that moved a pixel would move the project's whole reference.

## 6. What a person sees meanwhile — established as far as it can be, and no further

`doc/todo/37`'s stale frame is the machinery, and its shape fits this case exactly: `Stale::plan`
already takes `drawing: bool` and stands in for a view whose real frame has not landed, at three
quads a refresh, with `approximated` in the trace and a count in the summary. A draw that refuses
produces no raster, so the person sees the pixels they had, moved to where the new view puts them.

**What cannot be established is that it fires**, and the reason is not a defect: `stale.rs` is
`viewer-ui`'s own composing loop and `viewer-ui` is not on this boundary — `doc/todo/34` §2's last
line is that the window does not use the confinement at all. So this is the answer the machinery
is *placed* to give rather than one anything in the tree gives today, and saying otherwise would
be the shape of claim `doc/habits.md` has a whole section about.

**And reading it that far turned up what the policy will owe.** `Stale` stands in until a
rendering lands, and a refused draw never lands — so a host that raises an interrupt has to tell
the viewer that the render *failed*, or the stand-in becomes permanent and the person sees a frozen
approximation with no explanation. That is a message rather than a mechanism, it belongs with the
policy, and `doc/todo/15` now carries it.

## 7. What is still not built, and deliberately

Nothing in this tree decides *when* to raise an interrupt, because nothing in this tree is yet a
host on this boundary. The owner's brief in `doc/todo/10` states the shape the policy takes when
it is built — *"the UI could provide a callback warning the user and allowing the user to abort —
however don't block and wait for the user"* — and what section 2 above establishes is that such a
callback has to be driven by a clock the host reads **while** drawing, because there is nothing
worth reading before it.

## 8. The instruments

- `cargo run --release -p viewer-confined --example host_draw -- [--scale N] [--levels K]
  <file.pdf>…` — §2's estimate priced against the measured draw, page by page, with the planning
  pass beside it. The finding is in its module comment rather than only here, so that the next
  round to reach for that estimate meets it first.
- `cargo run --release -p viewer-confined --example confined_cancel -- --marks [--finish]` — the
  two arms beside each other: a worker that answers in 14 ms with 990 kB of marks it never drew,
  a host that draws them for 27.6 s, and three interrupts.
- `crates/render-cpu/tests/interrupted_draw.rs`, and
  `viewer-confined`'s `a_host_drawing_marks_that_will_not_finish_interrupts_its_own_draw`.

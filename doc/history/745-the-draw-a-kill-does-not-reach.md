# 745 — The draw a kill does not reach

ADR 0640's host-side debt, taken: on the marks arm the expensive drawing happens in the
**unconfined** host, where a cancel is a kill of something that has already answered — so
`pdf_render::Interrupt` is raised and honoured between commands, and there is no budget to derive
because nothing in this tree predicts what a display list will cost to draw. ADR 0650. Date
2026-08-25.

## What bounds the host's draw — the enumeration, which was the first half of the round

Read rather than assumed, because this block has been wrong about a bound repeatedly. The wire
caps the **message** at 2 GiB and every table's count against the bytes behind it, at
`least::COMMAND` = **one byte**; the decoder refuses a **target** past `viewer_core::MAX_PIXELS`;
`MAX_EXTENT` caps either **dimension** and `MAX_GROUP_DEPTH` the **nesting**. Nothing anywhere
bounds the **work**, and before this round no rasteriser in the tree — `render-cpu`, `render-gpu`,
`render-quorra` — contained a cancel, an interrupt, a deadline or an abort.
`render-quorra`'s budget refusals are about the *device's resources* and say nothing about a
frame's cost.

Multiplied out with terms that are all measured: the fixture's marks are 99 bytes a
page-covering fill and 2.76 ms a fill, so a message the wire already permits is 21.7 million fills
and **about seventeen hours** of drawing.

## The finding: the budget could not be derived, and the reason is a number

The instruction was ADR 0597's model — derive it, do not pick it. A budget needs a predictor, and
the tree has exactly one candidate: the sum of `row_costs` over `command_extents`, which every CPU
draw already computes to place its strips and then discards. Over `doc/pdf.js`'s 958 first pages it
correlates with the measured draw at **0.115** (Pearson), **0.649** (Spearman) — **0.161** and
**0.650** over the 952 whose target is under 3 Mpixels, so a giant `/MediaBox` is not the confound
— and **8** of the 40 slowest pages are among the 40 with the most estimated work.

One pair carries it: `personwithdog.pdf` at 484 704 pixels and a cover of **0.2** draws for
**162.4 ms**, `pattern_text_embedded_font.pdf` at 501 832 pixels and a cover of **593.5** draws in
**15.9 ms**. Same size of page, the estimate three thousand times apart, the clock ten times apart
the other way round — which no amount of load on the machine reaches.

`pdf_render::strips` says why in its own module comment — it "ignores edge building, which is
proportional to a path's complexity rather than to its bounding box" — and edge building is what
real pages spend their time on. `examples/host_draw` is the instrument and carries the finding in
its module comment, so the next round to reach for that number meets it first.

## What was built

`pdf_render::Interrupt` and `BackendError::Interrupted`;
`render_cpu::CpuRasterizer::interruptible`, checking a `Relaxed` flag at the top of `encode`'s
command loop — the one loop every recursion comes back through.

The name is the argument: a `Canceller` **ends a process**, an `Interrupt` is **raised and
honoured**, and the reason a cooperative flag suffices on the host's side and not inside the
confinement is that **the loop is ours** — a hostile document arrives in `render-cpu` as data.
The device backends are given no such method rather than one they would ignore.

## Numbers

**Instructions**, because the question is one atomic load per command and the machine's load
average was above twenty. `callgrind_rasterise`, ISO 32000-2 page 101, ×20, three arms in one
sitting: **5 441 579 467** before (built from an export of `HEAD`), **5 441 596 808** after with
no interrupt handed over (**+0.0003%**), **5 451 627 652** with one handed over and never raised
(**+0.18%**). The gates' path is unchanged, and byte equality over seven scenes is asserted rather
than assumed.

**Wall clock**, load 9.5 to 13. `confined_cancel --marks --finish`: the worker answers in **14 ms**
with **990 453 B** of marks it never drew; the host draws them for **27.6 s** at 900×1165; three
interrupts return the drawing thread in **1.3, 1.9 and 2.1 ms** — one page-covering fill, which is
the granularity claimed.

## The second finding, from reading rather than measuring

`doc/todo/37`'s stale-frame machinery is what a person sees while a slow draw is going, and it
fits — `Stale::plan` already takes `drawing: bool`. But it **stands in until a rendering lands**,
and an interrupted draw never lands, so a host that raises one owes the viewer a message saying
the render *failed* or the stand-in becomes permanent. That is the policy's, and `doc/todo/15`
now carries it. What could not be established is that any of it fires: `stale.rs` is
`viewer-ui`'s, and `viewer-ui` is not on this boundary.

## Gates

Whole, as a fifth round and a change to `pdf-render`. One failure on the way and it was the
citation gate: `ADR 0650 §4` in a doc comment, where `§` is checked against ISO 32000-2's clauses
and would have passed by landing on one.

## Ledger

Untouched. `CLAUDE.md` principle 3 against principle 2, citing no clause.

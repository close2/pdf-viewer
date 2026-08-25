# 740 — The outcome that is one page's, and the render it removes

The one `viewer-core` change 736 stopped at, taken: `Rendered::Listed` says *the host took this
request's own list* about a **page**, so the confined worker stops drawing what it does not send
and the raster budget stays on. ADR 0640. Date 2026-08-25.

## What needed a variant, and what was checked first

`doc/ui-boundary.md` records that no host round has needed a new message for a long run, which is a
reason to be sure rather than a rule against adding one. Three alternatives were tried on paper and
each fails on the budget rather than on taste:

- **Make `Rendered::Presented` per page.** A page that once presented would then have an unbounded
  budget for ever, and a page that flips arms later — a layer switch turning a chart into a scan —
  is rasterised at an unbounded target. The hole, one indirection further away.
- **Re-tighten the bound inside `viewer-confined`.** A second copy of `MAX_PIXELS`, in the crate
  that cannot see how the first is applied. That constant is public precisely so there is one.
- **Give `Answer::Frame` a page without pixels.** The design that looked cleanest and costs the
  most for the least: an unreachable arm in four hosts that never answer `Listed`, and a C frame
  with a size and no bytes. What the worker actually needed was an *origin*, and
  `Query::PageGeometry` already answers that — and answers `Answer::None` for a page the
  arrangement is not showing, which is the marks store's eviction rule in the same question.

So the merge stayed on the transport's side, which is where ADR 0633 put the payload choice for the
same reason: `Marks` holds each page's origin and `encode_answer` joins the pixels the viewer holds
to the marks the store holds. **`MAGIC` did not move** — the format and the population it carries
are byte for byte what `PDFVCF04` already was — and `viewer-ui` was the one consumer that failed to
compile, being the only one that matches `Rendered` exhaustively.

## What it saves

Both workers built from this tree, run interleaved in one sitting, five runs apiece, **load 4.1 to
4.5**. `Command::Open` to the events coming back: `PDF20_AN001-BPC.pdf` p1 **8.73 → 2.82 ms**, ISO
32000-2 p1 **48.86 → 40.38 ms**, and the two pixel-arm documents — `scan-bad.pdf` and
`issue12841_reduced.pdf` — flat, which is the control that makes the first two mean anything.
The extreme is the fixture: 1.5 kB amplifying to ten thousand page-covering fills, **26.5 s of
rasterisation to 18 ms**. ADR 0640 §3 has the table.

## The finding: a fixture whose premise the change removed

`a_document_that_will_not_finish_is_cancelled_and_the_host_gets_its_thread_back` failed in 31 ms.
Its hostile document's 44 seconds were all in the *rasterisation*, and its marks are smaller than a
window's raster — so the worker shipped it undrawn and there was nothing left to cancel. The
fixture is five levels deep now, which puts its list past any raster this boundary permits, and the
test **checks that premise** with `wire::crossing` before it blocks on it.

What that exposed is worth more than the fix: **a cancel stops the work the worker does**, so on
the marks arm it covers the interpretation and not the drawing. The drawing was always the host's —
since ADR 0633 the marks are what crosses — and the worker had merely been doing it a second time
and discarding the result. `doc/todo/15` now carries what a host taking marks owes.

## Gates

Whole, on the briefing's instruction and because it is a fifth round. Both workers built first
(trap 10) — which cost one wrong diagnosis on the way, `the_two_deferred_producers_reach_the_raster_arm_by_name`
failing on a page whose JPEG 2000 image no worker was there to decode. Fuzzers before the sequence:
`confined_wire` clean at 1 000 000 runs on a freshly seeded corpus, `display_list` clean at
3 437 105.

## Ledger

Untouched. `CLAUDE.md` principle 3 against principle 2, citing no clause.

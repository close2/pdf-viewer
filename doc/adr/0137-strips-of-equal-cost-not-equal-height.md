# ADR 0137 — Strips of equal cost, not of equal height

Status: accepted, 2026-08-02. Session 154. **A measurement, and a design decided by it.** ADR 0136
ended by saying the question that decides parallel CPU rasterisation is how many strips a command
touches, and that the answer is a counter rather than a patch. This is the counter and the answer.

## The question

`render-cpu` draws single-threaded. The obvious way to change that is to cut the target into
horizontal strips and replay the display list into each on its own thread: the geometry `Band`
already has (ADR 0010) for a different reason, rayon is already in the stack, and every `Command`
carries its own absolute transform and clip precisely so that "a backend may reorder or
parallelise them" — `display_list.rs` has said so since it was written.

Two things could make it not worth building, and only measurement separates them.

**Duplicated fixed work.** 19% of page 101's rasterisation is per-command work that does not shrink
with the band — `render_cpu::convert::path` 405 M, `Rect::from_points` 218 M,
`RasterPipelineBlitter::new` 164 M, of 4 104 M in `CpuRasterizer::draw` (session 154, callgrind).
A strip replay pays all of it once per strip a command touches. If the mean command touched every
strip, four strips would cost more in duplication than they saved.

**Imbalance.** Threads finish when the last one does. A split whose worst strip holds 70% of the
work is a 1.4× speedup on any number of threads.

## The counter

`crates/pdf-model/examples/strip_spans.rs`, committed so this can be re-priced in one command. It
walks the display list, computes each command's device extent — a path's control points through
its transform, a stroke's outset by half its device width, an image's unit square, a group's
elements individually because those are what the rasteriser walks — meets it with the clip chain's
extent, and reports two splits: boundaries at equal *heights*, and boundaries at equal *estimated
cost*, where a row's cost is the summed width of every command that can mark it.

## The answer, at 1× on four pages

`touches` is (command, strip) pairs per command: 1.00 means a strip replay duplicates nothing.
`slowest` is the worst strip's share of the estimated cost; the ideal is the last column.

| page | strips | equal rows: touches / slowest | equal cost: touches / slowest | ideal |
|---|---|---|---|---|
| ISO 32000-2 p. 101 | 8 | 1.05 / **15.9%** | 1.13 / **12.9%** | 12.5% |
| ISO 32000-2 p. 6 | 8 | 1.01 / 15.8% | 1.01 / **13.0%** | 12.5% |
| `tracemonkey.pdf` | 8 | 1.04 / 22.3% | 1.09 / **12.6%** | 12.5% |
| `bug1721218_reduced.pdf` | 8 | 1.01 / **72.0%** | 1.06 / **12.8%** | 12.5% |

**Duplication is not the problem.** At eight strips no page tested exceeds 1.13 touches per
command, so the 19% of fixed work is multiplied by 1.13 and not by 8: the whole penalty is
0.19 × 0.13 ≈ **2.5% of the render**. Glyphs are small and a page is tall, which is a fact about
what documents contain rather than about this implementation, so it should be re-checked on a page
of full-width fills before it is believed generally.

**Imbalance is the problem, and it is fixable with a prefix sum.** Equal heights are adequate on a
uniform text page and useless on the project's worst page: `bug1721218_reduced.pdf` is one wide
gradient over part of its height, and equal heights give one strip 72% of the work — a 1.4×
ceiling on eight threads. Cutting where the *cost* is even takes it to 12.8%, which is 97.6% of
perfect. **Every page tested lands within 4% of ideal once the split is by cost.**

So the decision: **if this is built, the strips are chosen by cost, and choosing them by height is
not a simpler first version — it is the version that does not work on the page that most needs
it.**

## What the counter also settled

`MaskCache` cannot be shared across threads, so each strip needs its own, and a clip chain that
spans *k* strips is built *k* times. On the page where that could hurt most — 3608 chains, and
`MaskCache::get` a quarter of the whole render (ADR 0103) — a chain touches **1.06 strips of
eight**. The chains that span many strips are the page-wide ones, and a page-wide chain's mask is
band-tall, so building it eight times over an eighth of the rows each is the same total area. Page
6 is the extreme: one deduplicated chain (ADR 0132) touching all eight, and eight eighth-height
masks instead of one. `tracemonkey.pdf` has no clip at all.

## The honest limits of this number

- **The cost model is extent area, not work.** It ignores edge building, which is proportional to a
  path's complexity rather than to its bounding box, and it treats a diagonal hairline's bounding
  box as if it were inked. Text pages have tight extents; the gradient page's extent is its ink.
  Where it is wrong it will be wrong in the same direction for every strip, which is the direction
  that matters for a *split*.
- **The split has to be computed at render time and that is not free.** It needs every command's
  device extent before the first strip starts, and transforming a page's path points is work
  `Rect::from_points` already does inside the rasteriser at 4.4% of the render. Computed once in a
  pre-pass and handed to the strips, it should *replace* that rather than add to it — but "should"
  is not a measurement, and this is the first thing to check when it is built.
- **This is a ceiling, not a speedup.** Memory bandwidth, rayon's own dispatch and the pixmap
  allocation per strip are all outside it. The only honest headline is "the decomposition does not
  forbid it", which the equal-height version very nearly did.

## Why it matters more than the ratio it improves

`CLAUDE.md` makes time-to-first-page a first-class requirement and makes the CPU backend the
startup path as well as the correctness oracle, so this is not throughput for a batch converter —
it is the number a person judges the program by, on the path they take before the GPU is ready.
ADR 0136's comparison found our page-one time growing with the pixels where another renderer's did
not; this is the axis that answers it.

Not built in this session, deliberately: the measurement is the deliverable, and the tree's own
habit is that a ledger row is an entry and an entry gets measured before it gets believed.

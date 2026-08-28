# ADR 0731 — A strip is a replay, and the planner cannot see what a reduction costs

Status: accepted, 2026-08-28. Session 797. Takes the `render-cpu` half of
[`doc/todo/45`](../todo/45-where-a-frame-goes.md) §2a, which ADR 0297 closed for
`render-quorra` and left named for the other two backends. Keeps ADR 0025's departure
exactly and changes nothing about it; amends no ledger status.

## The question, and it is not the one §2a asked

§2a asked for the *redraw*: `render-cpu` recomputes a reduced raster on every draw, so a
confined host redrawing a scanned page pays what the window no longer does. It said the
lines were not worth writing before that host had been measured.

Measuring found a second mechanism in front of it, larger and on every draw rather than on
the second one. **A strip is a replay of the display list**, and a full-page image reaches
every strip.

`render-cpu` cuts a target into horizontal strips and draws the whole list into each on its
own thread (ADR 0139). What bounds that replay is `pdf_render::replay_ratio`, which counts
the **rows** a command covers — exact for a fill, whose cost is its pixels, and blind to
`Image::area_averaged`, whose cost is per *source* sample and does not shrink with the band.
So the planner reads a page of one scan and 1201 other commands as a replay of **1.00**,
grants thirteen strips, and the page reduces the same 8.7 million samples thirteen times.

`issue12963.pdf` page 1 is that page: a 2480×3506 `JBIG2Decode` scan placed on 596×842.

## What it costs, measured

**Instructions**, `examples/callgrind_rasterise`, two draws of page 1, both arms built in one
sitting from one worktree — wall clock is not the instrument here and that example's own
header says why. The machine grants thirteen strips, so the arms below are pinned to four
cores with `taskset -c 0-3`, which makes `available_parallelism` say four and the ratio
legible:

| `issue12963.pdf` page 1, 4 strips, 2 draws | before | after |
|---|---|---|
| whole program | 3 884 597 422 | **2 248 310 893** (−42.1%) |
| `Image::area_averaged` | 2 178 913 072 (56.1%) | **544 728 268** (24.2%) |

544 728 268 × 4 = 2 178 913 072 to the instruction: the reduction now runs once per draw
where it ran once per strip. Unpinned, on this machine's thirteen strips, the before arm is
7 911 126 419 with **75.74%** of the whole program inside that one closure.

**Wall clock**, `examples/strip_spans`, which draws the page at each strip count with a
rasteriser of its own and prints the fastest of five renders. Medians of five interleaved
before/after runs, on a quiet machine (load average under one at the start):

| strips | before | after |
|---|---|---|
| 1 | 17.0 ms | 14.0 ms |
| 2 | 14.0 | **10.8** |
| 4 | 13.3 | **7.8** |
| 8 | 20.2 | **6.7** |
| 16 | 30.9 | **6.7** |

The shape is the finding. Before, this page got **slower** the more strips it was granted —
20.2 and 30.9 against 14.0 at two — which is `CLAUDE.md` principle 2's own sentence about a
parallel path that worsens latency, on a corpus document, at the count the shipped planner
picks. After, the curve is flat from four strips on and the page costs a fifth of what it did.

The one-strip row is the control — the warm pass does not run there, because one strip is not a replay — and it moved by 3.0 ms, which the memo cannot have produced
by sharing — at one strip there is one reduction either way. Two things account for it and
neither is the subject: the reduced buffer is now freed when the rasteriser is dropped rather
than inside the draw, and `best_of`'s timer stops before that; and the row's own run-to-run
spread is 15.7–17.5 before and 13.6–14.5 after.

A page with nothing to share is unmoved: `scan-bad.pdf` is one command, so it is drawn in one
strip, and its two draws go 988 725 351 → 977 867 793 instructions with
`Image::area_averaged` at 527 379 600 in **both** arms.

## The decision

`render_cpu::images::ReducedImages` — a memo on the rasteriser, holding the reduced samples
so that everything which would produce them again is answered instead.

**The key is ADR 0297's, unchanged**: the source samples' address and the reduction's two
factors. `Image::reduction` exists precisely to answer the factor half *before* the reduction
is paid for, and `Reduction`'s own doc comment already called that "the cache key a backend
needs and cannot otherwise have" — one backend had it, and now two do. An address is an
identity only while its allocation lives, so an entry **pins** the `Arc<[u8]>` it was keyed
on; that is the same ABA argument `render-quorra`'s `cache` module states, and it is why the
pin is a field nothing reads.

**Nothing is held across the reduction, and that is a safety property this round paid for.** The
first construction put an `Arc<OnceLock<_>>` in the map so that the first strip to ask ran the
reduction and the others blocked on the value. It measured beautifully and it **deadlocks**:
`Image::area_averaged` divides its rows across rayon above `PARALLEL_FLOOR`, and a worker waiting
inside `par_chunks` may have *another strip's job* stolen onto its own stack — a job that comes
straight back to the same key. A `Mutex` held there is re-entered by the thread that holds it; a
`OnceLock` is re-entered inside its own initialiser, which its documentation says may "panic or
deadlock". It did the second. The corpus gate hung with all twenty-six threads in `futex_do_wait`
and one per cent of a core, and the same page had already hung `raster_digest` and
`callgrind_rasterise` earlier in the round, both dismissed as a loaded machine.

**The rule that came out of it is more general than this module**: in a crate whose rasterisation
runs on rayon, a lock may be taken to read or write a map and may never be held across work that
itself uses the pool. So the memo takes its lock twice, briefly, with the reduction between them,
and a race costs the losing caller its own copy of identical bytes.

**Which leaves the sharing to be arranged rather than synchronised, and that is the warm pass.**
Thirteen strips of a page whose ink is one scan reach that scan within microseconds of each other,
so a memo that never blocks would miss thirteen times and buy nothing on the first draw of exactly
the page it was built for. `ReducedImages::warm` walks the list on the thread that plans the strips
— **before any strip is queued** — and reduces each image once; the strips then only ever hit. It is
also the one place the reduction may use rayon safely, because there is no other job in the pool to
be re-entered by.

**One placement answers for every strip, and the reason is in the arithmetic.** A strip's transform
differs from the warm pass's only by the row offset `ToDevice` composes last, and
`Image::reduction`'s factors are the lengths of the placement's two column vectors — which a
translation does not change. `Reduction`'s own documentation already states the consequence: two
placements with the same factors over the same source ask for the same bytes. A `Group` carries no
transform of its own in this display list, so the walk needs no matrix stack; it applies the same
"does this command mark the target at all" test the strip loop applies, so it warms nothing the
strips would have skipped.

**Where it lives is the liveness rule.** The memo is a field of `CpuRasterizer`, so a
rasteriser kept across frames keeps its reductions — `viewer-confined`'s worker holds one, and
that is §2a's original ask answered by construction — and a rasteriser made per job keeps
nothing, which is what `viewer_host::drawing` does deliberately and says so. Nothing had to be
decided at either call site.

**The bound is derived rather than measured, and that is the stronger statement.** A
reduction's factor is the floor of source samples per device pixel, so the grid it produces is
between one and two samples per device pixel per axis — under four samples, so under sixteen
bytes, per device pixel the image covers. A full-page image on a 612×792 page therefore holds
under 7.8 MB, and the 32 MB budget holds four of them. What eviction costs is exactly what
this module buys and no more: an evicted entry is reduced again, which is what every draw did
before.

## Why this is not a departure, and how that was checked

Nothing here computes anything. `Image::area_averaged` is called with the same arguments and
its answer is handed to the same code; a memoised draw and a fresh one produce the same bytes
by construction, and §10.7.4's departure is carried out the same number of *distinct* times
it always was.

Checked rather than argued: `examples/raster_digest` over the 974-document pdf.js corpus,
both arms built in one sitting with the crates `touch`ed as that example's header requires —
**957 first pages rasterised, not one digest moved**.

## What the tests discriminate, and what they were calibrated against

Four planted defects, each reverted (trap 13), above the commit that introduced the module:

| planted | fails |
|---|---|
| the factors leave the key | `two_reductions_of_one_image_are_two_answers` — it is served the 8× reduction for the 4× request, which is the silent failure a key missing a field has |
| the memo stores nothing and reduces on every ask | `a_memoised_reduction_is_the_reduction_and_is_the_same_buffer` — right bytes for ever, so only buffer identity sees it |
| eviction disabled | `eviction_bounds_the_memo_keeps_the_newest_and_costs_only_the_work` |
| eviction drops the newest too | the same test, from the other side |

The second is the one worth keeping: a memo that never hits is correct, and correct for ever,
and would leave this whole module visible only in a profiler.

## What is left

- **The deferred source is not memoised.** `ImageSource::AtDeviceScale` — §11.6.5.2's soft-mask
  image, whose samples do not exist until the device scale does — produces a fresh buffer per
  call, so it has no address that outlives one draw and nothing here can key on it. Each strip
  therefore still produces its own. Keying it on the `Arc<dyn ImageAtDeviceScale>` and the grid
  is possible and needs an accessor `pdf-render` does not offer; no corpus document reports an
  `/SMask` at all, so this has no witness and is priced in `doc/todo/45`.
- **`render-gpu` still recomputes**, which is the remaining third of §2a. It has no per-frame
  resource cache to hang an entry on and it is not the backend a host redraws with.
- **The planner still cannot see a reduction.** This makes the replay cheap rather than making
  `replay_ratio` honest, and a command whose cost is not in the rows it covers is a category
  the planner has no term for. Nothing else in the display list is in that category today.

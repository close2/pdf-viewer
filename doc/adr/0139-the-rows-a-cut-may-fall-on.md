# ADR 0139 — The rows a cut may fall on

Status: accepted, 2026-08-02. Session 155. **Built and shipped**, which is what ADR 0138 was
not. That one cut the page wherever the cost was even, four oracle pages stopped agreeing with
the reference consensus, and everything but the planner was reverted. It closed by naming the
shape of the next attempt and one probe that would decide it. This is that probe, and what
followed from its answer.

## The probe, and it answered more than it was asked

ADR 0138's question was whether an axis-aligned rectangular clip cut by a horizontal edge is
exact. Filling one shape into a 600×1200 pixmap and then into a top piece and a bottom piece,
and comparing the joined result with the whole:

| shape crossing the cut | bytes differing of 2.9 M | worst byte |
|---|---|---|
| axis-aligned rectangle at fractional coordinates | **0** | 0 |
| quadrilateral with no horizontal or vertical edge | 292–528 | 32 |
| closed cubic | 2480–2744 | 64 |

So the answer to the question asked is **yes** — and the answer to the question that matters is
that ADR 0138's proposed rule was both too weak and too strong. Too weak, because an *oblique
straight edge* is not exact either: a clipped line keeps its geometry and loses its endpoints,
the new one being computed by interpolation and the edge's slope taken from it. Too strong,
because a rule stated over whole command extents would forbid a cut under
`bug1721218_reduced.pdf`'s page-wide clip and under page 6 of ISO 32000-2's, and both of those
are rectangles.

A second probe fixed the boundary exactly. The cubic's control hull spans device rows 100 to
970; cuts at 96 through 100 and at 970 through 974 are bit-identical and 101 through 969 are
not. A boundary at row `r` cuts along `y = r`, so a segment is chopped exactly when
`top < r < bottom` — which is rows `floor(top) + 1` to `ceil(bottom) - 1` and no more. Both
probes are `render-cpu/tests/strip_cut_exactness.rs` now, because the rule is a claim about a
dependency's internals and those change without asking.

## What was built

`pdf_render::unsplittable_rows` marks every row a re-stated segment spans:

- a fill's curves, always, over their control hull;
- a fill's straight segments **unless** they are axis-aligned *and* the transform preserves the
  axes — judged in the path's own space, because judging it on the transformed coordinates
  would rest on this crate and `tiny-skia` rounding one multiplication identically;
- a **stroke's whole extent, whatever its geometry.** Two reasons and the second is why the
  first is not enough: a stroker puts curves on a straight path at a round cap or join, and a
  stroke thin enough to be a hairline is not turned into an outline at all but scan-converted
  by `tiny-skia`'s hairline path, which clips the *line* and — measured — draws nothing
  whatever into a target under three rows tall;
- a clip chain's segments wherever the chain is used, and a soft mask's group's, recursively.

`pdf_render::strip_boundaries_avoiding` then chooses cuts among the rows left. ADR 0137's
prefix sum was adequate unconstrained and is not here — page 101 of ISO 32000-2 grants 46% of
its rows, and snapping each prefix-sum boundary to the nearest legal one gives a worst strip of
24.5% against a 12.5% ideal. So the split minimises the maximum strip instead: a binary search
on that maximum with a greedy feasibility test, `O(strips · log rows)` per probe and exact for
the estimate it is given. It returns **fewer strips than asked for** where the page's geometry
forbids the cuts, and `[0, height]` where it forbids all of them.

`CpuRasterizer::rasterize` then borrows disjoint runs of the pixmap's bytes, gives each a
`TargetSpec` of its own height whose transform carries its first row, and runs them on rayon
with a `MaskCache` each and the mask budget divided by the strip count.

## Two things that were not part of the plan and were most of the work

**A serial per-pixel pass bounds a parallel render, and `impose_on_medium` was 7.8 ms of a
17 ms page.** §11.4.7's page group is isolated, so an unmarked pixel is transparent, and the
general path spent eight integer divisions on each of them. A pixel that is `[0, 0, 0, 0]` is
exactly the medium — with `clear` at 255 the quotient is `(below × 255 + 127) / 255`, which is
`below` for every byte — so that case is now a copy: 7.8 ms → 1.7. What is left is split across
threads like the drawing. Neither changes a byte.

**A planner on the drawing path is not a planner in an example.** `command_extents` rebuilt
every command's clip chain from the leaf, which is the chain's depth *per command*: on
`bug1721218_reduced.pdf` — 7050 commands over 3608 chains — it took **606 ms**, six times that
page's whole rasterisation. Memoising the chain per identifier takes it to 5 ms. The function
had been correct and unmeasured for two sessions because only an example called it.

## The measurement that changed the design

The oracle's verdicts were unchanged from the first run — 836 agreeing, 70 contradicted, over
1794 pages — which is the evidence the strips are exact. Its **clock** was not: 37.0 s serial
against 59.1 s split, and 88 s of processor time against 159.

Five pages held most of it, and `issue12841_reduced.pdf` is the shape of all five: **two
commands, each covering the page**, so sixteen strips replay both sixteen times. 105 ms serial,
166 ms split. ADR 0137 measured the replay ratio at 1.01 to 1.13 over eight strips on four
pages and concluded duplication was not the problem; it is not, *on those pages*, and this is
the same number computed per page instead of trusted per corpus.

So `plan_strips` asks from the most strips downwards and takes the first division whose
`replay_ratio` is at most **1.25**. A page of small marks sits far inside it; a page of a few
page-wide commands cannot reach it above one strip. With that bound the oracle is **37.0 s,
exactly the serial figure**, at +17% processor time — which is the honest price of parallelism
and is paid only where it buys something.

## What it buys

Fastest of five renders, `examples/strip_spans`, at the scale a laptop window asks for:

| page | serial | split | strips |
|---|---|---|---|
| ISO 32000-2 p. 6 at 2× (1192×1684) | 20.8 ms | **7.9 ms** | 16 |
| ISO 32000-2 p. 101 at 2× | 27.0 ms | **10.9 ms** | 16 |
| `tracemonkey.pdf` at 2× | 33.5 ms | **15.8 ms** | 4 |
| `bug1721218_reduced.pdf` | 105 ms | 105 ms | 1 — no legal cut |

`tracemonkey.pdf` is two columns whose line boxes do not share their gaps, so it grants 37% of
its rows and only four strips; `bug1721218_reduced.pdf` is one gradient under one curved clip
and grants none where its ink is. **Both are correct answers rather than failures**, and the
planner saying so is the whole difference between this and ADR 0138.

## Why bit-identity and not a tolerance

`render-cpu` is the oracle the GPU backend is judged against and the reference every corpus and
oracle verdict is taken from. A backend whose output depended on how it divided the page would
put the machine's core count into every comparison this project makes — and the cross-backend
tolerances are tuned to a tenth of a level in places (trap 12). It also makes the strip count a
free variable: `available_parallelism` may decide it, and `with_strips` may override it for a
test, because neither can change what the page looks like.

## The habits

- **A suite of shapes is a suite of shapes, and so is a suite of probes.** ADR 0138's probe
  used one cubic and concluded "a clipped line is the same line", which is false. The
  quadrilateral took ten minutes and moved the rule.
- **A dependency's refusal can be silent and size-dependent.** `tiny-skia` insets the clip by a
  pixel before hairline stroking and returns early when the inset is empty, so a hairline in a
  two-row target draws nothing and says nothing. Found by a test that had passed for a hundred
  and fifty sessions failing at one scale out of three — trap 2's rule, that a case at one
  scale cannot tell a reciprocal from a constant, paying again in a place nobody aimed it.
- **A ratio measured on four pages is a fact about four pages.** ADR 0137's 1.01–1.13 was
  correct and was read as a property of pages. Computing it per page is one function and it is
  what makes the split safe to ship.
- **Parallelism is a latency decision, and its price is processor time.** `CLAUDE.md` ranks
  those for an interactive viewer; this ADR is the first change in the tree where the two point
  in opposite directions, and the numbers for both are above.

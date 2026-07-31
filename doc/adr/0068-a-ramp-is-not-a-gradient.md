# ADR 0068 — A ramp is not a gradient

Status: accepted, 2026-07-31.

## Context

The handover has carried the same profile since the forty-sixth session and the same conclusion
with it: on `bug1721218_reduced.pdf`, "**the gradient stage** is the largest single item because
a `Ramp` carries 256 samples, so a shading becomes a 256-stop gradient and `tiny-skia` scans its
stops per pixel batch; handing the *rasteriser* fewer stops would fix it, while coarsening the
`Ramp` in the display list would lose fidelity and is not the same thing."

Re-measured in the sixty-fifth session, that page is **144.05 G instructions** and
`tiny_skia::pipeline::lowp::gradient` is **68% of it**. The rest of the old profile has moved
on — `Function::parse` was 23.2% and is 2.5% — so the diagnosis that survived is the one nobody
had acted on.

The handover also asked a second question, and the answer turned out not to matter: whether the
page's shadings are 3576 distinct functions or one re-parsed. Instrumenting the pattern path
counts **eight** shadings on page one. Parsing is not the cost; painting is.

## Decision

**A ramp keeps the stops a rasteriser cannot compute for itself, and drops the rest.**

`Ramp::sample_across` samples a colour function at `RESOLUTION` = 256 positions, because that is
the resolution at which a *function* has to be believed — a PDF function may do anything between
two samples. A *gradient* is a different object: both backends interpolate linearly between
consecutive stops, so a stop that lies on the line its neighbours draw contributes nothing.

`simplify` drops those. It is run once when the ramp is built, in `pdf-render`, so both backends
get the same stops — the rule trap 2 states, that a device decision either backend could make
alone is a decision neither has made.

### The rule is exact, not approximate

A stop is dropped only where **every** dropped stop lies within `COLLINEAR` = 1/512 of the line
the surviving neighbours draw, on every channel. That is half a level in eight bits, so the byte
a rasteriser computes at each position is the same whether the stop is there or not. Checking
all of the dropped stops rather than only the one being removed is what keeps the error from
accumulating across a long run.

Two things it deliberately cannot do. It cannot collapse the two stops `sample_across` places at
one position to express a discontinuity (ADR 0059): a zero-width span has no line to lie on and
fails the test immediately. And it cannot smooth a curve — a `/FunctionType 0` sampled function
or a type 4 PostScript calculator keeps every sample it needs.

What it does do is the common case, and the common case is nearly all of them: a
`/FunctionType 2` exponential interpolation with `/N 1` — the two-colour gradient every producer
writes — is one straight line and comes out as **two stops instead of 256**.

## Consequences

**`bug1721218_reduced.pdf` page 1: 144.05 G → 54.05 G instructions, a 62% fall.** The gradient
stage went 97.9 G → 15.8 G, from 68% of the page to 29% of a much smaller page, and
`render_cpu::shading::stops` left the top ten entirely. The oracle's own timing for that page
went 2.3 s → 1.8 s.

**Over the whole corpus, our total against `hayro` is 6.20 s**, against 7.13 s recorded in the
fifty-eighth session — and the two sessions between them added features rather than removing
work, so the fall is this. The median ratio is 2.15× against 2.29×; as the fifty-eighth session
recorded, the number to trust across sessions is our own total and not the ratio, because their
total moved 39.03 s → 34.93 s for reasons nothing here can see.

**No pixel changes.** Both gates were run: 858 documents draw with nothing reported, 831 pages
agree with the reference consensus, 65 contradicted, every verdict identical. The fifteen
cross-backend scenes pass, which is what says the GPU backend sees the same simplification.

**One test changed, and it was asserting the wrong thing.**
`an_unbroken_ramp_spans_the_whole_interval` required `stops.len() == RESOLUTION`, which is a
statement about how a ramp is *built* rather than about what it says. It now requires a straight
line to be two stops, which is this decision at the smallest possible scale.

**What is left on that page**, in order: `Function::parse` at 6.7%, `Mask::intersect_path` and
`build_soft_mask` at 6.4% each, `fill_path_impl` at 5.1%. The next item is the one the handover
already names — the CPU backend gives every transparency group a page-sized pixmap — and this
page is exactly the one that would show it.

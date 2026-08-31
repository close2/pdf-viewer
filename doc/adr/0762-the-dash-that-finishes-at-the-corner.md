# ADR 0762 — The dash that finishes at the corner, and the vertex no dasher was asked about

Status: accepted, 2026-08-31. Session 835. Cites ISO 32000-2 §8.4.3.4 and §8.4.3.6, both of which
stay `implemented` and both of whose notes were wrong about this tree. It sits beside ADR 0398
(§8.4.3.5's long mitre, the previous defect this corpus produced and the previous time a stroke
rule had to be stated in `pdf-render` because three libraries answered it three ways) and beside
`pdf_render::degenerate`, whose argument this copies.

## What the round was looking for, and what it found instead

`doc/todo/03` asks for a chunk of an unwalked corpus a round. There is none: the SafeDocs crawl is
65 944 of 65 944 ranked, the four submodule corpora are ranked, and what is left in that file is
either a 31 GB download or the item §14 names — *the per-case gates this corpus makes possible, one
clause and one hand-built witness apiece, each with its expected value derived rather than voted*.
The oracle gate is green and every one of its seven verdicts is held by name, so the fallback the
round was given (the worst standing page) had no unexplained head either.

So the chunk was `doc/corpora/pdf-differences` read *as clauses*: its eighteen cases each quote a
normative sentence and publish the picture that sentence requires. Every one of them was rendered
and read against the clause it quotes rather than against the picture. Fourteen agree — ColorBurn
and ColorDodge on ISO 32000-2's corrected edge cases, §8.6.6.3's out-of-range indices, §8.4.3.6's
negative dash phase (measured: the leading edge moves one unit right per unit of negative phase,
which is `−p mod 2Σ` and nothing else), §8.5.3.2's degenerate line caps, §8.9.7's inline-image
abbreviations, §11.7.4.4's atomic fill-and-stroke, §9.4.4's negative font size, the default colour
spaces both ways — and the four the corpus was already known to hold (§9.6.4's two colours,
§8.4.3.5's mitre, §9.3.6's winding, §7.5.7's rebuild) are as ADR 0393 and its successors left them.

**One case disagreed, and it is a defect of this tree on all three of its rasterisers.**

## The two sentences

§8.4.3.4, below Table 54:

> In a closed subpath that is dashed, if the first segment starts with an on-dash and the last
> segment ends within an on-dash, then they shall be joined.

§8.4.3.6:

> If the end of a dashed segment coincides exactly with a join point, then the end cap is painted
> before the corner.

*Within* is load-bearing in the first and *exactly* in the second, and together they split one
vertex into two answers. Every other corner of a dashed path needs neither sentence, because the
dasher settles it by construction: a dash that stops short of a corner becomes its own open contour
and the stroker caps it, and a dash that spans a corner keeps the corner inside its contour and the
stroker joins it. The vertex where a subpath *closes* is the exception — there the dasher has to
decide whether the last dash and the first are one mark wrapping round — and a dasher that merges
them whenever both are on has read §8.4.3.4 without its adverb.

## The witness, which states both cases in one file

`DegenerateDashing.pdf` draws eight rectangles under `[ 10 10 ] 0 d`, `5 w` and a round join, two
hundred units wide and 44, 45, 49 or 50 high, so that only the perimeter differs:

| rectangle | perimeter | pattern position at the close | the clause |
|---|---|---|---|
| 200 × 45 | 490 | 10 — the end of an on-dash | §8.4.3.6: two end caps |
| 200 × 44 | 488 | 8 — inside an on-dash | §8.4.3.4: joined |

This tree drew the round join on **both**. Under the file's projecting square caps that is a
rounded outer corner where the clause asks for a square one; under its butt caps it is a filled
corner where the clause leaves a notch, which is the picture the corpus publishes as correct and
which this tree now draws.

## Three rasterisers, one wrong answer

Measured with the rule below turned off, on a scene of the same shape at width 4 so that the
quadrant outside the corner is whole device pixels: the quadrant §8.4.3.6 leaves empty carried
**3.133** square units of ink from the processor, **3.086** from quorra and **2.753** from vello,
against a quarter disc of π = 3.142. Three libraries, one answer, and it is the wrong one — which
is why this is stated in `pdf-render` rather than fixed in the backend that happened to be looked
at first. `pdf_render::degenerate`'s comment has the same argument one clause over.

## The rule

`pdf_render::opened_where_a_dash_ends_at_the_close` replaces such a subpath's `Close` with the
straight segment Table 58 defines it as — "a straight line segment from the current point to the
starting point of the subpath" — so the geometry is unchanged and the subpath now has two ends for
the stroker to cap. All three rasterisers call it immediately after `split_degenerate` and before
their own dasher; a subpath the rule does not reach is not copied at all, and a solid stroke or a
path with no closed subpath allocates nothing.

Two decisions inside it are worth their reasons.

**The comparison is exact.** §8.4.3.6's condition is that the dash's end and the join point
"coincide exactly", so the test is whether `(phase + length) mod period` equals one of the array's
cumulative sums, with no margin. A tolerance here would be one nobody derived, and it would take
the *neighbouring* case with it: a close that lands a hair inside its last on-dash is precisely
what §8.4.3.4 joins. The witness is exact in `f32` — 200 + 45 + 200 + 45 is 490 and 490 mod 20 is
10, both without rounding — which is what a producer writing small integers is relying on, and the
corpus's own README says as much.

**A subpath holding a cubic is left as the file wrote it.** A Bézier's arc length has no closed
form, so no position along the pattern can be established exactly for such a subpath and no
coincidence is claimed. That is the standard's own word rather than a shortcut: a coincidence
nobody can establish is not one.

**And most of what the rule opens changes no pixel, which is what makes it safe.** Opening a
subpath is only visible where the boundary it lands on *ends an on-dash*; a rectangle whose
perimeter is an exact multiple of the whole period lands at position 0, where the thing before the
close is a gap and nothing is painted at the corner either way.

## What moved

Nothing in either gate population. The oracle's 1945 pages, quorra's corpus gate and the fixed
documents are byte-identical either side of the change, which is the expected result and not a
weak one: the construction needs a closed dashed subpath whose perimeter lands exactly on a dash
boundary, and `pdf-differences` is the only corpus on this disk that states one.

Both ledger notes were corrected, and both were wrong in the way `doc/habits.md`'s ledger section
describes. §8.4.3.4's said the joining sentence was one "which both dashers do" — two claims in
four words, and both false: they did not do it as stated, and there are three of them, not two,
which is `--bin parts`'s own subject (ADR 0709). §8.4.3.6's said "[t]he per-subpath restart and the
cap and join treatment are the rasteriser's, and both backends do it", which is true of every
vertex but the one this ADR is about.

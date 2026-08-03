# ADR 0155 — A pattern cell clipped where it cuts nothing

Status: accepted, 2026-08-03. Session 188. The fix `AMBIGUOUS_TILING_CELL_CLIP` priced in the
session before it.

## What was measured

`issue16038.pdf` fills two squares with uncoloured tiling patterns whose cells are rules about
three device pixels apart. Its two patterns state the same rules at two phases, so the ink the
document asks for is computable — 316.29 square points — and every renderer can be held to that
number rather than to each other. We were the only one *below* it, at 85.3% of it, where
§10.7.4 says the painted area "shall always be at least as large as the area of the original
shape".

Removing the suspect found the cause in one probe: with the per-cell `/BBox` clip deleted, the
left square's coverage went **0.1114 → 0.1323** against a geometric 0.1333, while the right
square's doubled. So the clip is load-bearing on one pattern and removes nothing on the other —
and on the other it was removing 16% of the ink.

## Why a clip that removes nothing is not free

Table 74: "These boundaries shall be used to clip the pattern cell." That sentence is about
*geometry*. A clip mask is anti-aliased, so a mark lying **on** the boundary keeps a fraction
of the boundary pixel and the neighbouring cell keeps the rest, and two fractions composite as
`1 − (1−a)(1−b)` rather than adding. `/pgfpat21`'s rule spans exactly its own cell, which is
how a producer draws a continuous line out of a repeating figure, and it is exactly the shape
that pays.

This is not the anti-aliasing departure §10.7.4 already records. That one draws a thin shape at
the coverage its area implies; this one drew *less* than the area, and two independent
renderers — `hayro` at 0.1353 and `mupdf`'s left square at 0.1329 — land on the closed form,
which is what says the arithmetic rather than the rasteriser was the difference.

## The rule

A tiling pattern's cell is drawn under its box clip as before, and the clip is taken back off
where the cell drew nothing outside the box. The first cell decides for all of them: the cells
are one figure at translations of each other.

Three decisions:

- **It is decided after the cell is drawn, not before.** A cell's extent is not known until its
  content stream has run, and running it twice is not free of consequence — the readback, the
  text layer, §14.8.2's artifact spans and §9.3.8's overlap bookkeeping all accumulate as it
  goes, and rolling them back would be a list of fields to keep in step with every future one.
  Drawing the cell *with* the clip and editing the clip off the commands afterwards needs no
  rollback at all: a command carries its geometry and *names* its clip, so only the name
  changes. `Command::set_clip` exists for this and says so.
- **It is conservative in three places**, each keeping a picture rather than a saving: a command
  whose extent cannot be bounded, a command whose clip is a *chain* the cell's own content built
  on top of the box, and a box that does not contain what the cell drew.
- **The containment test needed a bound this tree did not have.** `Command::device_bounds` is
  the strip planner's answer — the memoised hull grown by `width × miter_limit` in every
  direction — and it is the right shape for *may this command mark this strip*, asked once per
  command per strip. It cannot answer *does this command mark outside this rectangle*: 3.99
  units of reach here against a box 2.99 across. `pdf_render::outline::stroked_bounds` is the
  slow tight answer, asked once per pattern: a straight segment's stroke grows its own box by
  `(w/2)·(|dy|, |dx|)/L`, so a horizontal rule reaches half a width across and **nothing along**,
  which is the whole case.

## What the second opinion found in the first

Writing the tight bound and asserting that the loose one contains it — `the_loose_bound_
contains_the_tight_one` — failed immediately, and the loose one was wrong. It grew the
*device*-space box by a reach computed from the **path**-space width, so every stroke drawn at
a scale above the factor-of-two slack in that formula was under-bounded: a page at 3× with a
mitre near its limit reaches 15 device units where the bound said 10. `misses_target` skips a
command whose bound misses a strip, so the failure mode was a sliver of a mitre missing at a
strip boundary, on a path no gate walks. The reach is now grown into the path's hull *before*
the hull is mapped, which is dimensionally right, tighter, and costs nothing — `Path::hull` is
the memo that was already there.

## What it cost and what it bought

- Interpretation **2 167.0 M**, against session 185's 2 175.5 M: below it, because the pass over
  the first cell's commands is cheaper than the clips it stops creating. `issue16038.pdf` builds
  133 clips where it built 276.
- Every oracle verdict, corpus count and text percentage is unchanged — 847 agreeing, 70
  contradicted, 751 ambiguous, 79 incomplete — which is what says the change fires only where it
  is exact.
- `issue16038.pdf page 1` goes from 61.72 bounds from the furthest reference to 41.12, and its
  left square from 85% of the ink its geometry states to 99.2%.

## What is left

The right square, at 0.1159 against 0.1333. There the clip *is* load-bearing — the rule sits on
the cell's edge and is meant to be halved — so the two halves are drawn by different cells and
composite rather than add. Removing that clip would draw the rule twice at full width, which is
what `mupdf` does and what makes its two squares differ by 1.63. Fixing it means rasterising a
tiling's coverage once rather than cell by cell, which is a different construction and is not
this ADR's.

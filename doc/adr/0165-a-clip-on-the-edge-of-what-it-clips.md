# ADR 0165 — A clip on the edge of what it clips

Status: accepted, 2026-08-03. Sessions 205 and 207. Found on the fourth page of §3a's ranking,
and it moved the oracle's headline numbers; the next page down found a second bound in the same
function.

## The page

`bug1863910.pdf` is 353×59 points and its whole content is **two empty text fields**. Each
widget's appearance stream is five operators:

```
q 0 G 0.5 0.5 149 21 re s Q
```

— a one-point stroke (Table 52's default `/LW`) around a rectangle inside a `/BBox [0 0 150 22]`.
So the stroke spans 0.0 to 1.0 and 149.0 to 150.0 in x, and its **outer edge lies exactly on the
box that §8.10.2 clips it with**.

Ink over the page: ours 6.47, `hayro` 6.46, `poppler` 8.16, `mupdf` 8.20, `ghostscript` 10.60.

## The closed form, and it says outright that we are wrong

Two rectangles' perimeters at one point wide is `2 × 2 × (149 + 21) − 4 corners ≈ 680` square
points over a 353 × 59 page: **8.3 of 255**. `poppler` at 72, 576 and 2304 dpi gives 8.16, 8.29,
8.30 — the same number from the other direction.

We were producing 6.47, **22% under the geometry**. A per-stroke profile said the same thing more
sharply: a horizontal border deposited **0.718** of a pixel of coverage where a one-point stroke
at scale 1 covers exactly 1.0.

## Why

ADR 0155 found this once already, for tiling cells: **a clip mask is anti-aliased, so a mark
lying on the boundary keeps only a fraction of the boundary pixel.** The annotation's `/Rect`
starts at page-x 25.104, so the clip's edge falls 10% into pixel 25; the stroke's coverage of that
pixel is 0.896 and the clip's is 0.896, and the two *multiply* to 0.803 where the geometry says
0.896.

ADR 0155's answer was to take the clip back off where it removes no geometry —
`unclip_redundant_cell`, which no path but tiling used.

## Two changes, and the second is the one that took the work

**`unclip_redundant`** is `unclip_redundant_cell`'s body with the box and its space as
parameters, and `draw_appearance` now calls it after running an appearance stream. Same three
conservative refusals: a command whose extent cannot be bounded, a command whose clip is a chain
built on the box, a box that does not contain what was drawn.

**And it did not fire**, because the containment test could not pass. `pdf_render::hull` bounded
a miter join by §8.4.3.5's *limit* — `half × miter_limit`, a square of side ten line widths around
every vertex with the default limit. That is a sound bound and useless as a containment test: a
rectangle's corner came out five widths outside the rectangle, so no stroked border has ever been
inside its own `/BBox` as far as this code was concerned.

§8.4.3.5 says what the limit is:

> The miter limit shall impose a maximum on the ratio of the miter length to the line width (see
> "Figure 15 -Miter length"). When the limit is exceeded, the join is converted from a miter to a
> bevel.

**A maximum, not a length.** `join_extent` computes where the miter actually goes: the tip is
where §8.4.3.4's two outer offset lines cross, which for a right angle is the stroke's own outer
corner — `(52, 8)` for a 4-wide stroke turning at `(50, 10)`, not twenty in every direction. Both
candidate tips are returned rather than working out which side is outer; their union is still
tight. A join that nearly doubles back is dealt with below.

Two joins needed direction information the walk did not keep: `joins_from`, the point the
incoming segment's tangent comes from — its own start for a line, its second control point for a
curve — and **`leaves_start`, where the subpath's first segment goes**, which is the outgoing
direction of the join `Close` creates at the start point. Without the second, exactly one join per
closed subpath still fell back to the limit, and one join at the limit is enough to put a
rectangle's bound five widths out. That cost an hour and is the whole reason this ADR mentions it.

## What it moved

| | before | after |
|---|---|---|
| `bug1863910.pdf` ink | 6.47 | **8.299**, against a geometry of 8.30 |
| its distance from the nearest reference | 3.03 | **0.79** |
| oracle **agrees** | 849 | **851** |
| oracle **contradicted** | 70 | **68** |

`bug1669097.pdf` and `issue19505.pdf` left `CONTRADICTED_PAGE_ROUNDING` — both are widget borders,
and the clip was eating a fifth of every stroke. **Six of that group's eight members have now left
it for a reason other than its name.** Everything else is unchanged: corpus 80 incomplete, text
98.2%, quorra 912/44/1 at the time, dates 97.99%.

## What is left on the page

`ambiguous` at 0.79, and honestly. Ours 8.299, `poppler` 8.16, `mupdf` 8.20 — 1–2% under the
geometry — `ghostscript` 10.60, which is §10.7.4's "any pixel whose half-open square region
intersects the shape" applied to a one-pixel stroke, and `hayro` 6.46, which is where we were.
Four one-pixel borders in four slightly different places on a page 59 rows tall.

## A second bound, found by the next page down

`issue21068.pdf` is four rows of comb text fields, and the fix above did not reach it. Each
separator is a two-point subpath **closed on itself**, so both of its joins double back, and
`join_extent` fell back to `square(limit)` for a reversal — putting every separator 4.5 units
outside the `/BBox` that contains it, and the clip went on eating a fraction of each.

§8.4.3.5's own next sentence is the answer: "When the limit is exceeded, the join is converted
from a miter to a bevel." **A miter over the limit is not a long miter; it is a bevel**, and a
bevel reaches half a width. Bounding it by the limit is the same mistake this ADR is about, one
case in — the limit is what the join is *not*.

Ink 18.54 → **20.35** against a high-resolution limit of 20.12, distance 2.82 → 1.46, and the page
also left `render-quorra`'s differing list: both backends draw the same display list, so once the
redundant clip came off there was nothing left to differ about. **A page on a cross-backend
differing list can be there because of something upstream of both backends**, which is not what
that list's name suggests.

The fallback to `square(limit)` survives in one place and it is the honest one: where the incoming
or outgoing direction is unknown, nothing is known about the angle either.

## Alternatives rejected

- **Drop the `/BBox` clip for appearances.** §8.10.2 makes it a `shall`, and an appearance that
  draws outside its box would spill across the page. The rule is *redundant clip*, never *no
  clip*.
- **Special-case a clip whose edge coincides with a mark.** That is a rule about pixels; ADR
  0155's is a rule about geometry, and it is the same rule in both places.
- **Leave `hull` alone and loosen the containment test.** A containment test that accepts marks
  outside the box is not a containment test. The bound was the thing that was wrong.

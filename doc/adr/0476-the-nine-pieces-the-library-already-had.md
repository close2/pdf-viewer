# ADR 0476 — The nine pieces the library already had

Status: accepted, 2026-08-22. Session 646. Draws an axis-aligned rectangle's edge at the coverage
§10.7.4's own definition of a pixel implies, for a **fill** and for a **clip region** alike, where
`render-cpu` rounded it to a quarter of a pixel and to nothing below an eighth. Closes the half of
`doc/todo/11` item 7 the closed form covers; amends §10.7.4's ledger row and
`doc/todo/_scan-conversion.md`'s departure (1); empties
`CONTRADICTED_COINCIDENT_CLIP_EDGES` and takes `issue18823.pdf` off `render-quorra`'s differing
list. **It is cheaper than what it replaces**, which is the opposite of the price item 7 was
carrying.

## What was owed, and what the price was said to be

Session 643 established by closed form that `render-cpu`'s raster of `colors.pdf` *is* the page's
own arithmetic with every edge's coverage rounded to a quarter — mean 0.0023 of 255 against the
quantised form, max 1 over half a million pixels — and that the defect is a **quantum** rather than
a softness (ADR 0474). `tiny-skia`'s anti-aliased *path* scan converter supersamples four times per
axis at 0.125, 0.375, 0.625 and 0.875, and an axis-aligned edge looks the same to all four sub-rows,
so the sixteenth of a pixel that is its quantum for a general shape becomes a **quarter** for the
commonest shape in every PDF — and **nothing at all below an eighth**, which is the wrong side of
§10.7.4's

> The area covered by painted pixels shall always be at least as large as the area of the original
> shape.

643 priced the cure and declined to start it: *the interior plus eight exact edge and corner pieces
— nine `scan::fill` calls where there is one — and it moves every axis-aligned edge on every gated
page.* Principle 2 makes that a measurement rather than a preference, so this round's brief was to
build it and then decide with a number.

**The price was wrong, and the reason it was wrong is the finding.** It was priced as nine calls
into the converter that has the defect. The question worth asking was *which converter*, and
`tiny-skia` ships a second one: `PixmapMut::fill_rect` with anti-aliasing on walks a rectangle as
one interior run, four edges and four corners in 8.8 fixed point — the nine pieces, in one call, with
no supersampled accumulation and no alpha-run buffer, and a `blit_rect` memset for the interior.

## The geometry, and which sentence it comes from

§10.7.4 defines the device pixel as a **product of two half-open intervals**:

> for any point whose real-number coordinates are ( x , y ), let i = floor( x ) and j = floor( y ).
> The pixel that contains this point is the one identified as ( i, j ).

with the region "defined to be the set of points ( x′ , y′ )" bounded by `i ≤ x′` below `i + 1` and
`j ≤ y′` below `j + 1`. Two paragraphs earlier it gives a filled shape the same form — shapes "are
also treated as half-open regions that include the boundaries along their 'floor' sides, but not
along their 'ceiling' sides" — which changes no area, since a boundary has none.

An axis-aligned rectangle is a product of two intervals too. The intersection of two products of
intervals is the product of the two intersections, and the area of a product of intervals is the
product of their lengths. So

```text
  coverage(i, j) = overlap_x(i) · overlap_y(j)
```

exactly, for every pixel of every axis-aligned rectangle, at every placement. That is arithmetic out
of the clause's own definition — **a product of two overlaps, not a tuned constant** — which is what
`CLAUDE.md`'s principle 5 requires of an expected value, and it is why nothing here was fitted to a
renderer. Under this tree's anti-aliasing departure (§10.7.1's NOTE) that area *is* the coverage the
pixel is painted at; the sentence quoted above fixes the direction of anything left over.

`pdf_render::edge` states both halves — `device_rectangle`, which decides whether a fill is that
shape under an axis-preserving transform and where it lands, and `rectangle_coverage`, which is the
area. It is in the shared crate for trap 2's reason: *where two backends are the oracle, a decision
either can make alone is a decision neither has made.*

## The two sites, and why measuring only one breaks the clause

`render-cpu` uses the closed form twice:

- **a fill**, handed to `tiny-skia`'s rectangle scan converter (`scan::fill_rectangle`);
- **a rectangular clip region's mask**, written out by `scan::mask_rectangle`, because a `Mask` has
  no rectangle entry point of its own.

The second was not in the plan and is not optional. §10.7.4 says the clipping region "consists of
the set of pixels that would be included by a fill operation" — one rule, not two — and the first
version of this change measured only the fill. `render-cpu/tests/clip_intersection.rs` failed inside
one round of writing, by **26 levels of 255** at a boundary pixel: it asserts §10.7.4's own set
identity `S ∩ C = S` where `S ⊆ C` by rendering a mark under a clip that is its own rectangle and
comparing with the mark unclipped, and a mark painted at its exact area under a region still
measured to a quarter is drawn at the quarter. **A gate written for one clause caught the violation
of a different sentence of the same clause**, which is the cheapest kind of evidence there is.

The two arithmetics agree to **one level of 255** — measured, by tightening
`edge_coverage.rs`'s tolerance until it failed: `tiny-skia`'s 8.8 fixed point and this crate's
`round(255 · area)` are never further apart than that anywhere in the sweep. That is what lets the
fill keep the library's fast converter while the mask uses ours.

## What it costs

`callgrind_rasterise`, twenty repeats, `RAYON_NUM_THREADS=1` — instruction counts rather than a
wall clock, and pinned to one thread because rayon's spin under valgrind is what made two
unpinned runs of the *same binary* differ by 0.3%. Two passes agreed to 0.001%. Both arms built
from this worktree in one sitting, the baseline by disabling the two entry points. **Machine load
44.72 / 34.17 / 40.23 at the start and 29.40 / 30.47 / 37.03 at the end** — three neighbouring
rounds were building — and the counts are load-immune, which this round can assert rather than
assume: the same six figures came back within 0.01% from a pass taken at load 103.

| page | before | after | |
|---|---|---|---|
| ISO 32000-2 p101, text | 5,420,405,148 | 5,396,982,320 | **−0.43%** |
| ISO 32000-2 p6, 303 runs in one page-wide clip | 3,993,290,492 | 3,608,520,927 | **−9.64%** |
| `colors.pdf` p1, sixteen rectangles | 1,758,375,215 | 1,617,776,872 | **−7.99%** |

Cheaper on all three. The clip-heavy page gains most, because a page-wide rectangular clip stops
being a supersampled path fill; the text page is the case where the construction almost always
*declines*, and −0.43% is the cost of asking, which is below nothing.

**The launch clock cannot see this change and that is structural rather than lucky.** `CLAUDE.md`'s
principle 2 states that page one goes to the GPU, so `render-cpu` is not on the launch path at all;
what would be measured is the parse and the device bring-up, neither of which this touches. A
wall-clock launch A/B on a machine carrying three other rounds would have been a measurement of the
neighbours, which is sessions 627's and 633's lesson, so it was not taken.

**One de-optimisation was found and paid inside the round**, and it is worth recording because it is
principle 2 working as designed. The first `mask_rectangle` asked `rectangle_coverage` per pixel over
the whole region. On `colors.pdf`, whose page-wide `W n` clip is 595 × 841, that cost **+33%** of the
page's whole rasterisation. Writing the interior as a run — the same nine-piece decomposition, this
time by hand — is what turns +33% into −7.99%.

## What moved

- **The ladder.** `edge_coverage_ladder` had `render-cpu` answering 0, 0.2510, 0.5020, 0.7529 and
  1.0000 at twenty-one rungs; it now tracks the fraction to a level of 255 on both backends and both
  axes, and is within one level of the geometry at every rung.
- **`issue21346.pdf` page 1 left the oracle's contradicted list**, emptying
  `CONTRADICTED_COINCIDENT_CLIP_EDGES`. Every one of that page's seven statements is the *same*
  device rectangle whose edge falls at 14.173, so each was worth 0.75 where its own coverage is
  0.827. Its edge went **0.306 → 0.469** of the mark on both axes, measured through
  `examples/render_at` either side of the change. It does **not** pay `doc/todo/11` item 4: the two
  edges stand in the ratio `(0.75/0.827)^4.4`, so four to five of the seven are still *products*.
  What changed is what each factor is worth.
- **`issue18823.pdf` left `render-quorra`'s differing list** — the processor moving to the device,
  which has never had the quantum. It is one of the four widget pages whose border sits on its own
  `/BBox`; the other three stay, because their disagreement is the clip *product* rather than the
  coverage, and one of four moving is the evidence that the two are different things.
- **The oracle: 907 agrees / 66 contradicted → 908 / 65**, measured both ways in this worktree.
  `render-quorra`: 933 agree / 23 differ → 933 / 22.
- **`colors.pdf` is still contradicted, and it is the confirmation rather than the residue.**
  Session 643 computed — from the file's own arithmetic, with no code written and no renderer in
  the loop — that a rasteriser painting precisely the covered area would read ssim **0.98772** on
  page 1 and **0.98001** on page 2, against bounds of 0.98862 and 0.98402. The gate now measures
  **0.9879** and **0.9802**. A prediction made three rounds earlier by an independent route landing
  to the fourth decimal is the strongest evidence in this tree that the construction is the
  clause's and not a fit — and both pages remain `CONTRADICTED_TIGHT_CONSENSUS`, because the pair
  that votes is the pair furthest from the geometry.

## The ink sweep, which is where a change like this hides

`doc/todo/00` step 7, run before and after over all 786 ambiguous pages from the gate's own output
(`doc/todo/02` §7 requires it of any round that changes drawing): **645 pages identical to a
thousandth, 141 moved, median move 0.0040 of 255**. Of those 141, **98 moved toward the lightest
reference and 43 away**, and the summed distance fell 139.109 → 137.985. The negative head is
byte-identical and **19 pages at or past −1 before and after**, all diagnosed; the positive tail is
unchanged but for `bug920426.pdf` at +0.003.

The two largest movers are `bug1844583.pdf` −0.372 and `bug1844576.pdf` −0.298, both widget
appearances — the same `/BBox`-clip population `issue18823.pdf` came from — and both moving *toward*
the references. Opened and looked at (trap 1): a text field with a border reading "Hello World",
drawn correctly, our ink sitting between `hayro`'s and `mupdf`'s before and after. **A rectangular
clip's edge whose true coverage is below three quarters now admits less than the quantum did**,
which is the construction subtracting ink rather than a mark going missing, and is why the sweep's
moves go both ways.

## What was declined, and why

- **A path stating more than one rectangle.** Two rectangles drawn as two marks composite by
  §11.3.7.3's union where one scan conversion accumulates them, so taking them one at a time would
  trade this defect for `doc/todo/11` item 5's along every shared seam. `device_rectangle` returns
  `None` for anything but a single subpath and says so.
- **Everything that is not an axis-aligned rectangle** — a glyph, a curve, a diagonal, a stroke's
  outline — keeps the quantum, where it is a sixteenth rather than a quarter and averages along the
  edge. The closed form is claimed for the shape it is exact for and no further.
- **Matching `tiny-skia`'s 8.8 rounding in `mask_rectangle`.** It would be curve-fitting to a
  library's internals; `round(255 · area)` is the clause's number and the one level between them is
  measured and stated instead.

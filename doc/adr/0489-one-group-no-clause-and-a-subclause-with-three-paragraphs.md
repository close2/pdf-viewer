# ADR 0489 — One group with no clause, and a subclause that had three paragraphs for it

Status: accepted, 2026-08-22. Session 662. Rewrites `CONTRADICTED_TIGHT_CONSENSUS`'s note around
measurements; adds `crates/pdf-model/examples/coincident_edge_probe`; amends §10.7.4's ledger row,
`doc/todo/11` item 4 and traps 1, 9 and 12. **No pixel moves.**

## How the group was chosen, and the criterion is new

Three rounds have worked the oracle's contradicted pool and each left a better way of choosing than
it found: 643 wrote out a page's closed form and ranked five renderers against the geometry; 651
found a group's *name* holding while everything under it was wrong; 656 asked how many of a group's
own members its note actually measures. That last one is spent — thirteen of fourteen measure all of
theirs — so this round asked the question one level further out, and it is a principle-5 question
rather than a bookkeeping one:

> **A contradicted verdict is the claim that the standard rather than the consensus decides the
> page. How many clauses of ISO 32000-2 does the group's note cite?**

Counted over every non-empty `CONTRADICTED_*` list in `oracle.rs`, taking every `§n.n.n` in the doc
comment above the constant and looking each one up in `doc/conformance/ledger.toml`:

```text
  CONTRADICTED_DEVICE_CMYK_CONVERSION            5 pages   17 clauses
  CONTRADICTED_SUBSTITUTED_FONT                  8 pages   18
  CONTRADICTED_NEGATIVE_LINE_WIDTH               1 page     3
  CONTRADICTED_IMAGE_SAMPLE_AT_THE_PIXEL_CENTRE  1 page     3
  CONTRADICTED_SUBPIXEL_IMAGE                    1 page     3
  CONTRADICTED_CALRGB_TO_SCREEN                  5 pages    4
  CONTRADICTED_REFERENCES_DREW_NOTHING           2 pages    4
  CONTRADICTED_LINK_BORDER                       3 pages    2
  CONTRADICTED_ON_A_PAGE_WE_REPORT               1 page     2
  CONTRADICTED_VISIBILITY_EXPRESSION             1 page     2
  CONTRADICTED_REFERENCE_GLYPH_WIDTHS            1 page     1
  CONTRADICTED_GLYPH_EDGES                      26 pages    1
  CONTRADICTED_SHARED_JBIG2_DECODER              7 pages    1  (§2, the normative references)
  CONTRADICTED_TIGHT_CONSENSUS                   3 pages    0
```

Every cited row the ledger has is non-`unreviewed`. Two citations have no row at all and neither is
a technical clause — `CONTRADICTED_SHARED_JBIG2_DECODER`'s §2 is the normative references, which is
where ISO/IEC 14492 is named, and `CONTRADICTED_SUBSTITUTED_FONT`'s §6.3.2.2 is the conformance
clause. So the check has one answer and it is unambiguous: **one group of the fourteen argues its
verdict without a clause under it.** That is not a small omission for
this group in particular, because its name is a statement about *the references* — "two agree with
each other more closely than anybody is right" — and a statement about the references with no
clause beside it is exactly the shape principle 5 exists to catch.

The criterion is cheap, mechanical and now spent as well; the next round owes a new one. What it
does not do is rank by size or by age, and that is deliberate: a group of twenty-six pages citing
one clause may be perfectly argued, and a group of three citing none cannot be.

## What the three pages are, measured

All three turn out to be **§10.7.4**, in three different paragraphs of it, and none of the three was
named. `tools/spec-errata emit doc/ISO_32000-2_sponsored_EC3.pdf` over the family prints one
annotation set for §10.7, Issue #371 on §10.7.2's flatness, and both of its annotations quote
flatness text; nothing is filed on pages 398–406 at all, so §10.7.4's paragraphs stand as printed.
The annotations were read rather than only the checker's verdict, which is 656's lesson.

### `issue7891_bc1.pdf` page 1 — the note blamed the word, and the word is where we are right

The note said the difference is "one word inside a luminosity soft mask whose group draws a
676 × 436 greyscale image reduced 2.8-fold", and argued from an ink ladder agreeing to 0.0014 of
255. The ink ladder is a metric that **passes**; the metric that fails is the worst tile, 6.73
against a bound of 6.04, and nothing in the note accounted for it.

The page admits a closed form. Object 12 strokes `211.76 421.544 243.36 156.960 re` in red, sets
`/GS1` — `/SMask` with `/S /Luminosity`, `/BC [1 1 1]`, `/G` the form drawing the image — and fills
the same rectangle black through it, so away from the stroke every pixel is `255 × (1 − L)` with `L`
the image's own sample inside the mask group's `/BBox` and `/BC`'s white outside it. Two forms were
written out pixel by pixel: **point**, §10.7.4's image paragraph carried out, and **area**, the exact
box average over each device pixel's source footprint, which is what ADR 0025's departure
approximates. Compared with `raster_compare` through `examples/compare_rasters`, on the tile the
gate fails at (device x 224–255 by y 320–351, the middle of the word):

| | vs the area form | vs the point form |
|---|---|---|
| ours | **0.166**, max 1 | 0.947, max 9 |
| `hayro` | 2.814, max 18 | 3.012, max 22 |
| `poppler` | 4.255, max 27 | 4.362, max 34 |
| `ghostscript` | 4.596, max 30 | 4.677, max 29 |
| `mupdf` | 6.723, max 40 | 6.802, max 43 |

Our raster is the page's own arithmetic to **one level of 255** on the tile that decides the verdict,
and our 6.725 against `mupdf` is `mupdf`'s own 6.723 from the form. Two consequences: the reduction
the note pointed at is settled rather than in dispute, and **all five renderers are nearer the
average than the point sample**, so "[t]here shall not be averaging over the pixel area" is departed
from by every one of them — ours in writing and theirs not.

### What the page does differ by is seven lines of pixels

Splitting our distance from each voting reference by row and column, in the gate's own arithmetic so
that the parts add to the mean it prints:

| | ours vs `mupdf` | ours vs `ghostscript` | `mupdf` vs `ghostscript` |
|---|---|---|---|
| fill rectangle's rows 213, 370 | 0.0197 | 0.0259 | 0.0153 |
| fill rectangle's columns | 0.0128 | 0.0266 | 0.0175 |
| mask `/BBox` rows 290, 357 | 0.0625 | 0.0625 | **0.0003** |
| mask `/BBox` column 362 | 0.0227 | 0.0228 | **0.0001** |
| everything else | 0.0560 | 0.0565 | 0.0474 |
| total | 0.1721 | 0.1926 | 0.0799 |

**68.3% and 71.5% of it is seven lines**, and the pair that votes agrees about the mask's `/BBox` to
0.0003 — because both take a clipping region as §10.7.4's set of pixels:

> For clipping, the clipping region consists of the set of pixels that would be included by a fill
> operation. Subsequent painting operations shall affect a region that is the intersection of the set
> of pixels defined by the clipping region with the set of pixels for the region to be painted.

Device row 290 is `[290, 291)` and the `/BBox` reaches device 290.536, so a fill would include it and
the clause admits the whole row: all three C references paint 255 there and we paint 118, this
tree's departure (1). **And the pair is not uniformly the clause**: at column 362, where the `/BBox`
reaches 362.16 and the clause admits the column entire, `poppler` keeps it and `mupdf` and
`ghostscript` drop it to black. Same pair, same construction, same 0.0001 of agreement, opposite
verdicts against the sentence — which is trap 9's seventh entry and the reason the unit to take back
to the specification here is the edge rather than the page.

### And one of the seven is ours, with four digits behind it

Object 16 is a form XObject whose `/BBox` is exactly the rectangle its content fills and which
carries `/Group`. Rows 213 and 370 are covered 0.504 and 0.456 by that rectangle; ours paints them
**0.2549** and **0.2079**, which are those two numbers squared. `doc/todo/11` item 4 is the debt, and
this round turned its remaining prose into a ladder with no document in the way —
`crates/pdf-model/examples/coincident_edge_probe`, one 40 × 40 page holding one fill whose edge covers
0.504 of a device row, restated a second time four ways and each of those with and without a
luminosity soft mask whose `/BC` is white and whose value is therefore 1.0 everywhere:

```text
  restated as     no soft mask     soft mask
  fill alone            0.5059        0.5059
  W n clip              0.5059        0.5059
  form /BBox            0.5059        0.5059
  group /BBox           0.5059        0.2549
```

**Seven rungs give the edge its own coverage and the eighth squares it.** So of item 4's three open
bullets, the stroke and the image edge have no witness on this construction and the group blit has a
two-factor one — and the reason nobody had a small witness is §11.4.4's NOTE 5: a group is flattened
away unless a soft mask is in force, so `draw_group`'s blit is only reached when there is a mask
beside it, and every probe written without one goes down the flattened path ADR 0355 already pays.

**Not fixed here**, and the arithmetic says why rather than a hope: those two rows are 0.0197 of a
distance of 0.1721, so paying it moves the page toward the bound and past neither, and the
composition it needs is the one item 4 prices — a group's buffer carries alpha, which is shape times
opacity, and only a shape channel beside it makes `min` the right answer.

### `colors.pdf` pages 1 and 2 — a sentence that was true when it was written

The note said ours is the closed form with every edge's coverage rounded to `tiny-skia`'s quarter and
`hayro`'s is the exact one. **ADR 0476 made ours the exact one, three sessions after the sentence was
written.** Both forms were re-derived here from the content stream in a script owing nothing to 643's,
and compared by the oracle's own arithmetic:

| | page 1 vs exact | vs quarter | page 2 vs exact | vs quarter |
|---|---|---|---|---|
| ours | **0.0026**, max 1 | 0.0428, max 34 | **0.0026**, max 2 | 0.0463, max 36 |
| `hayro` | 0.0015, max 2 | 0.0409, max 33 | 0.0017, max 2 | 0.0462, max 35 |
| `mupdf` | 0.0173, max 13 | 0.0395, max 37 | 0.0201, max 14 | 0.0383, max 44 |
| `ghostscript` | 0.1026, max 54 | 0.1342, max 75 | 0.0890, max 63 | 0.1046, max 64 |
| `poppler` | 0.2117, max 124 | 0.2483, max 127 | 0.1883, max 112 | 0.2078, max 128 |

Ours differs from the exact form on **0.0000% of either page**. The verdict is unchanged and its
reason is the group's name: the gate's bound is 0.98862 and 0.98402 and ours now reads 0.98786 and
0.98024, so a rasteriser painting precisely the area each rectangle covers is still contradicted,
because the pair that votes is the two renderers furthest from the geometry.

The correction had reached `doc/traps/pixels-and-rasterisers.md`, §10.7.4's ledger row and
`doc/todo/11` item 7 — everywhere except the group whose members it is about. That is a **third** way
a group's note can be wrong, after its name and its reading: a sentence true when written that
nothing pointed at when the tree moved under it. Trap 1 now says so, with the tell that finds one —
a note quoting a number the gate also prints, where the two disagree.

## Was the deciding clause the one the group cited?

The group cited none, so trivially no; the sharper answer is that **§10.7.4 decides all three pages
and does it in three different paragraphs** — the shape paragraph for `colors.pdf`, the image
paragraph for the reduced greyscale, the clipping paragraph for the `/BBox` edges that carry most of
the disagreement. A note naming "§10.7.4" and stopping would have been nearly as unhelpful as naming
nothing, which is the general form of 651's finding: the citation has to be to the sentence.

## Consequences

- `CONTRADICTED_TIGHT_CONSENSUS`'s note is rewritten around the measurements above and cites the
  subclause three times, by paragraph.
- `crates/pdf-model/examples/coincident_edge_probe` is the eight-rung ladder, and it is the
  instrument worth keeping: it needs no corpus document and it answers "which composition
  multiplies" in one run.
- §10.7.4's ledger row records the ladder and the independent confirmation of ADR 0476; it stays
  `partial`.
- `doc/todo/11` item 4 gains the ladder, the NOTE 5 explanation of why the residual hid, and
  `issue7891_bc1.pdf` page 1 as its corpus witness.
- Trap 1 gains the stale-sentence shape; trap 9 gains a seventh entry — a pair whose agreement is
  the clause on one line of pixels and a shared departure on the next; trap 12's standing witness
  gains the closed form on the tile that fails, beside the ink ladder for the metric that passes.
- No ratchet moves and no pixel moves. The oracle's lists are unchanged.

## Owed

- Item 4's group blit, which now has both a ladder and a corpus witness and still needs a shape
  channel beside a group's raster.
- A criterion for the next round; this one's is spent.
- Nothing links a group's note to the code it describes, so the next rasteriser change can stale
  another one exactly as ADR 0476 did. Trap 1 states the habit; no gate enforces it.

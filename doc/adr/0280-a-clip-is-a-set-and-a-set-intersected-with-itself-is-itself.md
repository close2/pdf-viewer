# ADR 0280 — A clip is a set, and a set intersected with itself is itself

Date: 2026-08-11 (session 444)
Status: accepted

## Context

ADR 0279 measured a fourth departure from §10.7.4 and left it standing with its price written
down: this tree composes a clip chain by **multiplying** coverages, so a rectangle a document
states as a clip *n* times over is drawn with an edge at the *n*th power of one boundary's
coverage. `issue21346.pdf` states one device rectangle six times and its edge was painted at
**0.041** of the mark where the geometry is 0.827 of it and the clause is 1.000.

That ADR gave three reasons for not moving. This round read the clause again, and one of the
three turns out to be a reason to move rather than a reason to wait.

## The clause, read with its two neighbours

§10.7.4's own clipping paragraph:

> For clipping, the clipping region consists of the set of pixels that would be included by a
> fill operation. Subsequent painting operations shall affect a region that is the intersection
> of the set of pixels defined by the clipping region with the set of pixels for the region to be
> painted.

§8.5.4 says the same thing from the transparent imaging model's side, and it is the sharper of
the two because it says what a clip does to a *value*:

> The effective shape is the intersection of the object's intrinsic shape with the clipping path;
> the source shape value shall be 0.0 outside this intersection.

**A clip zeroes what is outside it and is silent about what is inside it.** Nothing in either
clause lowers a value that the clip admits, and nothing anywhere makes a clip's own boundary a
quantity that multiplies. §11.6.5 is where a genuine product lives — a soft mask supplies §11.2's
*alpha*, and Table 142's mask value multiplies the object's — and it is a different mechanism with
a different clause.

**The clause alone does not choose between `min` and a product**, and saying so is the honest
half of this reading: a clipping region taken by the fill rule is 0 or 1 at every pixel, and on
such a pair `min(a, b)` and `a × b` are the same function. What makes the question exist at all is
that this tree **anti-aliases** — departure (1) of §10.7.4's ledger row — so a boundary pixel
carries a fraction and two boundaries falling in one pixel meet. There the two compositions part,
and three things decide:

1. **A product moves away from the clause with every restatement.** `c → c² → c³` walks the
   boundary towards zero, in the direction the same subclause's "[t]he area covered by painted
   pixels shall always be at least as large as the area of the original shape" forbids. It is not
   an approximation of anything: no reading of the clause makes a rectangle stated twice fainter
   than one stated once.
2. **`min` is exact where the two boundaries coincide or nest**, which is the case the corpus
   witness is made of, and it is what a set intersection does: restating a clip changes nothing.
3. **`min` is never below the product**, so on two unrelated boundaries sharing a pixel — the case
   where neither composition is exact — it is never *further* from the clause's whole pixel than
   the product is.

What is exact for two unrelated boundaries is the area of the intersection of the **paths**,
rasterised once. That is a conflation-free rasteriser and remains a project rather than an item.

## Decision — `render-cpu` composes a clip chain with `min`

`scan::mask_intersect` no longer calls `tiny_skia::Mask::intersect_path`, which multiplies. It
fills a scratch mask with the new clip's own coverage and takes the smaller of the two values per
pixel. `MaskCache::build` allocates that scratch once per chain, from the same width and height as
the chain's own mask, so the two are the same size by construction; `tiny-skia` allocates one per
`intersect_path` call.

### The ladder, before and after

The instrument is the one ADR 0279 wrote, re-derived here so that the *fill's* own edge does not
enter the product: one 178.34645-point page, the whole page filled black, under **n** `W n` clips
of `issue21346.pdf`'s own rectangle, rendered at 8× through `examples/render_at`. Coverage of the
boundary column, device 113, where the clip's left edge lands at 113.386:

```text
  coincident boundaries      1       2       3       4       5       6
  before                 0.5020  0.2510  0.1255  0.0627  0.0314  0.0157
  after                  0.5020  0.5020  0.5020  0.5020  0.5020  0.5020
```

Each rung was the one above it halved and now none of them moves, which is the clause's sentence
carried out: intersecting a set with itself is that set. (0.5020 rather than the geometry's 0.614
is `tiny-skia` quantising a scanline's run to quarter pixels, which is departure (1) and not this
decision's business.)

### The witness

`issue21346.pdf` at its own scale, device column 14 of row 89, against an interior of
`(206, 223, 235)` — the closed form `0.25 c + 0.75` per channel, which four renderers agree on byte
for byte:

| | pixel | coverage of the mark |
|---|---|---|
| before | `(253, 253, 254)` | **0.041** |
| after | `(247, 250, 252)` | **0.163** |
| this tree's departure (1) would give | | 0.827 |
| the clause, `poppler`, `ghostscript` | `(206, 223, 235)` | 1.000 |

**Four times the ink and the page is still contradicted**, which ADR 0279 predicted: the failing
bound is structural similarity, 0.9734 → **0.9781** against a bound of 0.9900. That is the honest
result and it is why this decision is written as a clause carried out rather than as a page fixed.

### Where the rest of the page's product comes from, since only part of it was ours

ADR 0279 counted six statements of one rectangle. Sorted by what composes them:

| factor | composed by | clause | folded? |
|---|---|---|---|
| the page's `W n` | `MaskCache::build`, the mark's chain | §10.7.4 clipping ¶ | **yes** |
| form 15's `/BBox` | the same chain | §8.10.1 step c) | **yes** |
| form 13's `/BBox` | the same chain | the same | **yes** |
| form 13's own fill | `tiny-skia`'s blitter, mark × clip mask | §10.7.4 clipping ¶ | no |
| the mask group's `/BBox` | the same, inside the mask's own render | §10.7.4 clipping ¶ | no |
| the mask group's own fill | the mark on the other side of that product | §11.6.5 (the *value*) | no |

Three of the six are one chain, so folding them removes **two** factors and leaves four: exactly
the fourfold the ladder's 0.5020 predicts, and exactly what the page did.

**Two of the four that remain are the same sentence as the three that moved.** A mark's coverage
meeting the clip mask *is* "the intersection of the set of pixels defined by the clipping region
with the set of pixels for the region to be painted", and `tiny-skia` multiplies it inside
`fill_path`. Changing that means not handing the mask to the library at all: rasterising the mark's
coverage into a buffer of our own, composing, and blitting — this backend's own blitter, which is
the same project the conflation-free rasteriser is. It is stated in `doc/todo/11` with the two
factors it is worth on this page, rather than left as an unnamed remainder. The fourth, the soft
mask's *value* multiplying the mark, stays: that one is alpha and the standard says multiply.

## The gates, before and after

The whole of `doc/todo/02` §2 was run twice. **Every verdict count and every summary line is
identical** — the corpus's 974 with 65 incomplete, the oracle's 1794 pages at 1693/101 with
905 agree / 68 contradicted / 786 ambiguous, both text gates at 99.8% (14257/14281) and 99.2%
(24007/24191), the dates, the XMP and the JPEG 2000 lines. Tests 1616 → **1619**, citations
6554 → **6559**, quotations 630 → **631**.

**22 of the oracle's 1794 per-page lines moved and none changed verdict**: `issue21346.pdf`,
`22060_A1_01_Plans.pdf`, `bug1721218_reduced.pdf`, `bug1885505.pdf`, `issue13520.pdf`,
`issue269_2.pdf`, `issue7014.pdf`, `issue840.pdf`, `stamps.pdf`,
`ContentStreamCycleType3insideType3.pdf`, `highlights.pdf` page 1, and the eleven copies of
`tracemonkey`'s page 13. Every movement is in the third or fourth decimal place of a mean or a
similarity, which is what a change confined to pixels where *two* clip boundaries fall looks like.

`doc/todo/00`'s step 7 was run before and after over all **786** ambiguous pages, on this file's
own recipe. **The negative tail is byte-identical**: twenty names at or past −1, sixteen of them
documents this tree calls incomplete, and on the complete documents `issue16038.pdf` −5.734,
`issue12295.pdf` −2.956, `issue14297.pdf` −1.150, `issue7821.pdf` −1.000, `jpx_smaskindata.pdf`
−0.839, `issue16473.pdf` −0.717 and nothing past −0.536. Twenty-one rows moved and twenty of them
*up*, by 0.001 to 0.025; the one that moves down is `22060_A1_01_Plans.pdf`, −0.265 → −0.280, which
is the right direction rather than a surprise — a clip that admits more of a *pale* mark subtracts
ink rather than adding it, and that page is 72 sampled images.

**Two of the sweep's own names have drifted since it was last run whole.** Session 405 and 406
recorded `issue16038.pdf` −5.507 and −5.758 and `issue12295.pdf` −1.709 and −1.712; the other four
names past −0.5 reproduce here to the thousandth. Both drifters are pages ADR 0268's and ADR 0213's
work is about, and nothing between session 415 and this one re-ran the sweep. That is `doc/todo/00`'s
own lesson holding — a round that changes drawing and skips step 7 leaves the number *unwatched*
rather than unchanged — and it is recorded rather than chased.

## What it costs, measured

`examples/callgrind_rasterise`, instructions for one page:

| page | before | after | |
|---|---|---|---|
| `bug1721218_reduced.pdf`, 3554 clips | 15 160 262 499 | **13 833 362 133** | **−8.75%** |
| ISO 32000-2 page 101 | 5 543 400 670 | **5 553 668 923** | **+0.19%** |

The heaviest clip page in the corpus got *faster*, and the reason is the scratch mask rather than
the arithmetic: `intersect_path` allocates and zeroes a whole band-sized mask per call, and a chain
now allocates one for the whole chain — 3554 allocations where there were 7108. An ordinary page of
text pays 0.19% for the clear-and-min it did not previously perform on its two-link chains. This is
a correctness change and the number is here because §2 of `CLAUDE.md` asks what one costs, not
because it was taken for speed.

## The two backends, and the ask

`render-cpu` is the correctness oracle, so a change here moves every comparison in the tree, and
`render-quorra` composes its chain inside the graphics library — this side hands over a chain of
paths and the device intersects them. **The gate did not part company**:
`render-quorra/tests/corpus.rs` reports `957 pages compared: 917 agree, 35 differ, 5 refused, 17 not
comparable` before and after, and **not one of its per-page lines changed by a digit**. None of the
22 pages this round moved is among the 35 the two backends already differ on, and on the other 935
the movement stays under the agreement bound.

That is a measurement rather than a licence. The two backends now compose clips by two different
rules, and the one that is not derived from the clause is the one this project cannot change, so it
is written down as **§18 of `doc/QUORRA_FEEDBACK.md`** with this ADR's ladder as the reproduction.
Until it is answered the divergence is bounded by a gate that watches it every round, which is the
best a viewer can do about a rule inside somebody else's device.

## What is not done

- **The mark's own coverage still multiplies into the clip mask**, twice on the witness page. It is
  the same sentence and it needs this backend's own blitter. `doc/todo/11` item 4 carries it with
  its price.
- **`min` is not exact for two unrelated boundaries in one pixel**, only for coincident or nested
  ones. It is never worse than the product, which is the whole of the argument for taking it, and
  the exact answer is a conflation-free rasteriser.
- **The graphics device is unchanged**, and §18 of the feedback document is the ask rather than a
  plan.

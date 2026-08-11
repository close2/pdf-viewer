# Shapes that still disappear

Status: **items 1 and 3 closed in the three-hundred-and-eighty-ninth session (ADR 0226) and their
one named residual — a sub-pixel rule that is *diagonal* — closed in the four-hundred-and-thirty-second
(ADR 0268); item 2 is fixed as far as any corpus document exercises it (ADR 0213) and its general
case is unwitnessed; item 4, the same subclause's clipping paragraph, is **half paid** — the clip
chain composes as a set intersection since the four-hundred-and-forty-fourth (ADR 0280) and the
mark's own coverage still multiplies into the mask.** What is left is that half, and one *new*
measurement at the boundary rather than under it.
Priority: 11
Corpus: 4 known witnesses; the general shape of the residual is stated
Clauses: §10.7.4, and §8.5.4 for item 4 — see `_scan-conversion.md`
Code: `crates/pdf-render/src/sub_pixel.rs`, `crates/render-cpu/src/lib.rs`,
`crates/render-cpu/src/scan.rs` (item 4's composition),
`crates/pdf-model/src/content.rs`'s `tile`, `crates/pdf-render/src/repeat.rs`,
`crates/render-quorra/examples/sub_pixel_marks.rs` (the instrument),
`crates/render-quorra/tests/sub_pixel_coverage.rs` (the gate, on **both** backends since 389)

Leftovers from the hundred-and-eighty-sixth to -eighth sessions, which closed §10.7.4's
"no shape ever disappears" for a fill with *no* area (ADR 0154) and for a redundant pattern-cell
clip (ADR 0155). All three are the same sentence one step along, and none of them is the
anti-aliasing departure.

## 1 and 3. A stroke or a fill thinner than the rasteriser's coverage quantum — **closed**

Both were `render-cpu`'s alone, which the three-hundred-and-forty-fourth session measured, and both
were paid in the three-hundred-and-eighty-ninth. `tiny-skia`'s scan converter supersamples four
times per pixel row and takes each sub-row's sample at its centre, so a fill under an eighth of a
pixel crossed no sample line and vanished; and its painter drew a stroke under a pixel wide as a
hairline smeared symmetrically about the path, so one within half a pixel of the raster's edge lost
the half of its smear that fell outside.

`pdf_render::sub_pixel_bands` draws an axis-aligned rectangle thinner than a device pixel as the
whole pixel line it lies in, at the coverage its own area there implies, and a sub-pixel stroke on a
straight axis-aligned rule is converted to the fill of its own outline first. Both backends now
answer within one level of 255 of the shape's own area at every thickness measured, and
`tests/sub_pixel_coverage.rs` gates **both** against the area rather than against each other. The
before/after ladders, the cost, and every declined case are in ADR 0226.

Two consequences worth keeping here:

- **The oracle was the backend being accused.** `render-quorra/tests/corpus.rs` calls a difference
  between the two backends quorra's by construction, so on a page of sub-pixel line work the render
  carrying the right ink was the one on trial. That gate went 914 agree / 42 differ to **920 / 36**,
  and `issue16038.pdf` — a page whose whole subject is a 0.53-pixel rule — moved **6.5359 → 1.8563**.
- **The rule takes a promotion away as often as it gives a mark back.** At 0.9 of a pixel
  `tiny-skia` rounded a rectangle up to a whole row, 11% heavy; `issue8125.pdf` page 1 left the
  oracle's contradicted list because of that half rather than the disappearing half.

### The residual they left — a rule that is not axis-aligned — **closed, and it was a different defect**

ADR 0226 named `22060_A1_01_Plans.pdf` as the witness and priced the answer as "a coverage span per
scanline rather than one rectangle per pixel line, which is a scan converter of our own". The
four-hundred-and-thirty-second session measured the case instead of inheriting that price and
**both halves of the sentence were wrong** (ADR 0268):

- **A diagonal does not disappear, and cannot.** A band lying between two of `tiny-skia`'s sample
  lines vanishes; a band that is *not parallel* to them crosses one every `1/(4 tan θ)` pixels of
  its length. A filled sliver 0.05 of a pixel thick reads 9.47 to 10.23 of its own 10 at every angle
  from 5° to 60°, where the axis-aligned one read **0** before ADR 0226. So a diagonal **fill** is
  owed nothing and is left alone.
- **What was failing was the other guarantee**, and only for a stroke: "[t]he area covered by
  painted pixels shall always be at least as large as the area of the original shape".
  `tiny-skia`'s hairline lays one pixel down per step along the line's **longer device axis**, so it
  carried `cos θ` of the rule's area — 3.4% short at 15°, 13.4% at 30° and **29.3% at 45°**, at
  every thickness under a pixel rather than only near the quantum.
- **And it needed no scan converter.** §10.7.4's own construction for a mark too thin to measure is
  a run of *whole pixels*, so the substitute is the same rule stroked one device pixel wide with the
  width it gave up carried in the paint's alpha — `pdf_render::substitute_width`, and §11.3.7.1's
  licence is the one ADR 0226 already used. Ink is then the shape's area at every angle.
- **`22060_A1_01_Plans.pdf` was never the witness.** Its page one is **72 sampled images** with a
  combined device footprint six times the raster, 24 fills and 40 strokes, of which 26 are
  sub-pixel and 98% of their length lies within 5° of a device axis — the hairline dropped **0.3%**
  of it. The page moved +0.06% and that is the correct answer. Its line work is §10.7.4's *image*
  paragraph and ADR 0025's area averaging; `oracle.rs` and `_scan-conversion.md` said "all strokes
  under a pixel wide" and have been corrected.
- **The real witness is `issue11473.pdf`**, whose three diagonal hatch swatches are `0.3985 w`
  strokes inside a §8.7.3 tiling cell: ink **0.6768 → 0.7566** where the two-ladder limit for the
  page is 0.752 to 0.760. Ten per cent under the geometry to on it.

## What is left: the rule that is **exactly** one device pixel wide

Found by the same instrument in the same session and **not paid**, because it is a different claim
from this file's: a mark that is thinner than the document said is not a mark that disappeared, and
§10.7.4's quantum has nothing to do with it.

`tiny-skia` chooses the hairline for `width <= 1.0` device pixels, so at *exactly* one pixel a
turned rule still gets it:

```text
  a 200-unit rule, one device pixel wide, total ink against its own 200
                    hairline (today)   the fill of the same outline
    30 degrees            173.20                  199.73
    45 degrees            141.42                  177.44
```

**−29.3% at 45°, on every `1 w` stroke at the page's own scale**, which is a large share of every
technical drawing in the corpus. ADR 0268 stops strictly under one pixel and therefore leaves a
one-point discontinuity at the boundary — 0.999 of a pixel is filled, 1.000 is a hairline, 1.001 is
filled again — which is `tiny-skia`'s `<=` rather than anything derived.

Taking it is a round of its own and the reason is blast radius rather than difficulty: the change is
one comparison, and it moves what **every** page with an ordinary hairline draws, so it owes its own
before/after over the oracle's 1794 pages and its own instruction count. Two things to settle first:

- **The `0 w` stroke must not follow it.** `Stroke::device_width` promotes a zero width to exactly
  one device pixel, so the two arrive indistinguishable at the rasteriser, and §10.7.4 exempts one of
  them by name — "Zero-width strokes may be done in an implementation-defined manner that may
  include fewer pixels than the rule implies". Telling them apart means reading the document's own
  width, which `draw_stroke` has and `draw_sub_pixel_rule` is not passed.
- **The 45° knife edge is `tiny-skia`'s and survives either way.** The plain fill of a
  one-device-pixel band at exactly 45° reads 177.44 of its own 200, because that converter quantises
  the band's per-row run to quarter pixels. 177.44 is much better than 141.42 and it is not the
  geometry.

## And one loss ADR 0268 takes deliberately: the cap it does not draw

The substitute is the stroke's **swept body**, butt-capped, because a cap's area goes as the square
of the width: widening multiplies it by `width / style.width` more than the alpha divides it back,
and on `issue12295.pdf` — 65 859 sub-pixel strokes, every one round-capped, 91.8% of them shorter
than one device pixel, median length **0.145** — keeping the cap put the page 66% over its own 8×
limit where the body alone puts it 8.9% over.

What is given up is the cap's own area, which is `O(w²)` with `w` under one device pixel. It is not
nothing in one case: a round-capped subpath *shorter than its own width* is a dot of area `πw²/4`
and the body it is replaced by is thinner still. §8.5.3.2's exactly-degenerate subpath is already
taken out and filled as a dot by `pdf_render::split_degenerate`; the nearly-degenerate one is not,
and that is where this would be answered rather than in `render-cpu`. **No corpus page reports it**:
`doc/todo/00`'s step-7 sweep over all 786 ambiguous pages moves every entry *up* except
`issue12295.pdf`'s own, which moves down because the page's geometry is below every reference.

A thin shape that is not a rectangle at all — a sliver of a triangle, a glyph stem — is declined
deliberately and permanently for the *exact* substitution: its cross-section is not constant along
its length, so a single coverage across a pixel line would be worse than what the rasteriser already
does. ADR 0226 argues it, and small text is the case that makes it a rule rather than a caution.
ADR 0268's substitute does not touch a fill at all, so it does not reopen the question.

## 4. A clip boundary that falls where another clip boundary already fell — **the chain is paid; the mark's own coverage is not**

**Found in the four-hundred-and-forty-third session (ADR 0279) and half taken in the
four-hundred-and-forty-fourth (ADR 0280).** It is the same clause's *other* paragraph — the one
about clipping, which neither this file nor §10.7.4's ledger row had cited before 443:

> For clipping, the clipping region consists of the set of pixels that would be included by a fill
> operation. Subsequent painting operations shall affect a region that is the intersection of the set
> of pixels defined by the clipping region with the set of pixels for the region to be painted.

A clipping region is a **set of pixels**, and §8.5.4 says what that does to a value: "[t]he effective
shape is the intersection of the object's intrinsic shape with the clipping path; the source shape
value shall be 0.0 outside this intersection." A clip zeroes what is outside it and is silent about
what is inside it. This tree multiplied instead, in *two* places — `MaskCache::build` composing a
chain, and `tiny-skia`'s `fill_path` composing the finished mask with the mark's own coverage.

### What was paid: the chain

`scan::mask_intersect` takes the smaller of the two coverages. The ladder — one page, the whole page
filled, under **n** `W n` clips of the same rectangle whose left edge lands at device 113.386 at 8× —
went from each rung being the one above it halved to flat:

```text
  coincident boundaries      1       2       3       4       5       6
  before                 0.5020  0.2510  0.1255  0.0627  0.0314  0.0157
  after                  0.5020  0.5020  0.5020  0.5020  0.5020  0.5020
```

`min` is exact where two boundaries coincide or nest, and where two unrelated ones share a pixel it
is never *below* the product, so it never moves further from the clause. It cost +0.19% of the
rasteriser on a page of text and **bought 8.75%** on the corpus's heaviest clip page, because the
scratch mask is allocated once per chain rather than once per link. Every oracle verdict, the
corpus's counts, both text gates and quorra's 917/35/5/17 are unmoved; 22 of 1794 per-page lines
moved in the third decimal place and none changed verdict.

### What is left: the mark's own coverage

**The witness is `issue21346.pdf`**, which states the same device rectangle six times over — a
`W n`, three `/BBox` clips under §8.10.1 step c), the mark's own path and the mask group's — and its
edge went **0.041 → 0.163** of the mark where departure (1) would give 0.827 and the clause gives
1.000. `poppler` and `ghostscript` give 1.000, `mupdf` 0.755, `hayro` 0.327; the page stays
`CONTRADICTED_COINCIDENT_CLIP_EDGES` in `oracle.rs`, its failing similarity 0.9734 → 0.9781 against
a bound of 0.9900.

Three of the six statements were the chain, so two factors came out and four remain. **Two of the
four are the same sentence**: `tiny_skia::PixmapMut::fill_path` multiplies the clip mask into the
mark's coverage, once for the mark and once for the fill inside the soft mask's group. Reaching them
means not handing the mask to the library at all — rasterising coverage into a buffer of this
backend's own, composing with `min`, and blitting — which is this backend's own blitter and is the
same project a conflation-free rasteriser is. The fourth is §11.6.5's alpha and is a product the
standard states; it is not owed.

Two things bound any attempt:

- **`min` is not exact for boundaries that merely share a pixel**, only for ones that coincide or
  nest. What is exact is intersecting the *paths* and rasterising once.
- **`render-quorra` still multiplies its chain**, inside the graphics library, so the two backends
  now compose clips by two different rules. No gate can see it — 957 pages, not one per-page line
  moved — and it is `doc/QUORRA_FEEDBACK.md` §18 rather than a silence.

## 2. Two marks that abut across a cell's box edge without repeating

**The witness closed in the three-hundred-and-seventy-fourth session and this is what is left of
the item.** `issue16038.pdf`'s second square drew a rule its cell states on *both* box edges, so
Table 74's clip halved it and the two halves composited as `1 − (1−a)(1−b)` rather than adding —
0.1159 against the geometry's 0.1333. The two statements are one mark of the tiling, a whole
`/YStep` apart, and §11.6.2 forbids compositing portions of one object; folding them to the one
mark they describe puts the right square within 1% of the left at every scale (ADR 0213).

Three things that were written here and are worth keeping:

- **Removing the clip alone is not the answer.** It draws the rule twice at full width, which is
  what `mupdf` does and what makes its two squares differ by a factor of 1.63.
- **The NOTE that looks like the answer is not one.** §11.6.7's NOTE 2 recommends treating all
  tiles as a single transparency group against "artifacts due to multiple marking of pixels along
  the boundaries between adjacent tiles", and `tile` has built that group since the
  hundred-and-seventeenth session. Compositing inside a group is still compositing; the loss was
  *inside* it. (This file said §8.7.3.1's NOTE 2 for four sessions. The note is §11.6.7's.)
- **The general case is still open, and no page in the corpus names it.** Two *different* marks
  hanging out of opposite edges of the box and meeting at the boundary: the clipped pair is then
  the right set of points, there is no repeat to fold, and joining them would mean either a
  coverage buffer per tiling or a boolean intersection of path against box — the second of which
  would bake a flattening resolution into a display list that has none. `repeated_subpaths`
  refuses the case by name: the fold's condition is that the mark's lattice copy reaching into the
  box is itself stated. What is left over is a seam one boundary pixel wide, which is what this
  item was about all along; `issue16038.pdf` was the family's only witness and its figure repeats,
  so the residual is *unwitnessed* rather than measured-and-left.

There is also a residual on the *witness*, which is not this and is smaller: both squares sit 1.5%
to 3% under the geometry at 2× and 4×, and that is the rules' **ends** abutting column by column —
the same seam one axis over, over one pixel column per three rather than the whole length of every
rule. It is `AMBIGUOUS_TILING_CELL_CLIP`'s own last paragraph.

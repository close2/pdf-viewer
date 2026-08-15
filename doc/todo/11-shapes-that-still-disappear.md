# Shapes that still disappear

Status: **items 1 and 3 closed (ADR 0226), their diagonal residual closed (ADR 0268), the boundary
itself — a rule *exactly* one device pixel wide — closed with ADR 0285, and the cap a substitute
does not draw closed with ADR 0290 along with §8.5.3.2's dot, which nobody had measured; item 2 is
fixed as far as any corpus document exercises it (ADR 0213) and its general case is unwitnessed;
item 4, the same subclause's clipping paragraph, is **paid for a fill** — the clip chain composes as
a set intersection on **both** backends since ADR 0280 and quorra's own ADR 0030, and since ADR 0355
a *clipping region* meets a filled mark's own coverage by `min` on this backend rather than by a
product, and since ADR 0363 a clip standing *beside* a soft mask does too.** What is left of it is a
stroke, an image, **a group's raster — which ADR 0355 called not owed and §8.5.4's third sentence
says is owed** — and the two
backends that still multiply; and what is left of the file is what an eight-bit
raster does to a mark whose ink is under one of its levels, and two marks abutting — which item 2
had only across a cell's box edge and which the four-hundred-and-seventy-third session measured in
its general form on a document the project owner reported (ADR 0308). **It is not a defect of this
program**, and item 5 says on what evidence.
Priority: 11
Corpus: 4 known witnesses; the general shape of the residual is stated
Clauses: §10.7.4, §8.4.3.3 and §8.5.3.2 for the marks that are `O(w²)`, §8.5.4 for item 4 and
§11.3.7.3 for item 5 — see `_scan-conversion.md`
Code: `crates/pdf-render/src/sub_pixel.rs`, `crates/render-cpu/src/lib.rs`,
`crates/render-cpu/src/scan.rs` (item 4's composition),
`crates/pdf-model/src/content.rs`'s `tile`, `crates/pdf-render/src/repeat.rs`,
`crates/render-quorra/examples/sub_pixel_marks.rs` and
`crates/pdf-model/examples/sub_pixel_width_census.rs` (the two instruments: what a backend does
with a mark, and what a page's own marks are),
`crates/render-quorra/tests/sub_pixel_coverage.rs` (the gate, on **both** backends since 389),
`crates/render-quorra/tests/abutting_marks.rs` and `crates/pdf-model/examples/uncovered_share.rs`
(item 5's gate and its instrument)

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

## The rule that is **exactly** one device pixel wide — **closed (ADR 0285)**

Found by the same instrument in the session that closed the residual above, and paid in the round
after this file was last written. `at_or_under_the_quantum` is the comparison, `<` became `<=`, and
three things had to be settled with it: the *exact* construction stays strictly under the quantum
because snapping a one-pixel rule onto a row would be §10.7.5's stroke adjustment without `/SA`
(ADR 0208); the **cap comes back** at the quantum, because ADR 0268's `width / style.width`
overstatement is exactly 1 there; and the `0 w` stroke follows the rule rather than keeping the
hairline — §10.7.4's exemption is a `may`, §8.4.3.2's "1 device pixel wide" is a `shall`, and what
decides between the two readings is that `pdf_render::Stroke::device_width` resolves a zero width
in the shared crate so that both backends draw one mark.

**The whole corpus gate is byte-identical with the exemption in and with it out**, so the choice is
recorded as a choice. The reference oracle did not move a single verdict; step 7's ink sweep moved
31 rows up and none into the negative tail; the cross-backend gate cost two pages, at bounds this
project chose. What it fixed:

```text
  a 200-unit rule, one device pixel wide, total ink against its own 200
                    hairline (today)   the fill of the same outline
    30 degrees            173.20                  199.73
    45 degrees            141.42                  177.44
```

**−29.3% at 45°, on every `1 w` stroke at the page's own scale**, which is a large share of every
technical drawing in the corpus, and it is a `shall` rather than a preference: §10.7.4's rule about
painted area "applies both to fill operations and to strokes with non-zero width". It broke under
the clause's binary model too — a 45° band of width 1 and length `L` has area `L` and the hairline
paints `L/√2` pixels — so the finding does not rest on this tree's anti-aliasing departure.

**The 45° knife edge is `tiny-skia`'s and survives the fix.** The plain fill of a one-device-pixel
band at exactly 45° reads 177.44 of its own 200, because that converter quantises the band's
per-row run to quarter pixels. 177.44 is much better than 141.42 and it is not the geometry; the
gate's `TURNED_TOLERANCE` of 14% is set by it.

**And one thing this cost that no gate would have shown**: `zero_area_fill.rs`'s placement of
`50.3` put the stroke's band 0.1 of a pixel off its row at scale 2, which is *below* `tiny-skia`'s
quarter-row sample quantum, so the snapped fill and the unsnapped band came back byte-identical and
the test failed with both constructions correct. A test must be placed off the rasteriser's own
sample grid and not merely off the pixel boundary. ADR 0285 §"the test that had to move".

## The cap ADR 0268 did not draw — **closed (ADR 0290), and it brought §8.5.3.2's dot with it**

ADR 0268's substitute was the stroke's **swept body**, butt-capped, because a cap's area goes as
the square of the width where the body's goes with it: widening by `k` multiplies the cap by `k²`
where the body's alpha divides by `k` once. The answer is the missing factor rather than the
missing mark — `pdf_render::enlarged_mark` states any such mark at the substitute's width with an
alpha of `(w / W)²` — and Table 53's projecting caps lie *outside* the butt-capped body, so
`pdf_render::sub_pixel_caps` is a second, disjoint mark whose ink adds rather than a wider stroke
whose ink would be overstated by `W / w`.

**The same measurement found a mark nobody had looked at**: §8.5.3.2's dot, "a filled circle centred
at the single point", is `π w² / 4` and vanished outright at 0.1 and 0.2 of a device pixel — silent,
ungated and unwitnessed. It takes the same substitution, as does a zero-length dash's mark.

### What is left of it: an eight-bit raster's own floor

`issue12295.pdf` is the witness and the residual is the *raster's*. All 65 859 of its sub-pixel
strokes are 0.1366 of a device pixel wide (`pdf-model/examples/sub_pixel_width_census`), so their
caps are 2170.93 device pixels of geometry — 1.14 levels of 255 over the page — and the page's ink
rose by **0.133**. A cap at that width is 0.0073 of a pixel of ink spread over the few pixels its
substitute covers, about **half a level of 255 each**, and half a level is what an eight-bit raster
rounds away. The mark no longer disappears, which is the clause's requirement; what lands is what
the raster can hold.

**What would recover the rest is one draw rather than two**: the cap's coverage *added* into the
body's own mark instead of composited beside it at its own alpha, so that one deposit of 40 levels
is made where two of 35 and half a level are made now. It needs the subpath's arc length — a
flattening for a curve — and a second construction for a path with joins, where one alpha cannot be
right for every subpath. It would also take back most of what this cost: **+146.8% of the
rasteriser's instructions on that page** (21.30e9 → 52.57e9 over 20 rasterisations), against +0.19%
on an ordinary page of text, and the profile says it is the second `scan::fill` per stroke rather
than the geometry.

A thin shape that is not a rectangle at all — a sliver of a triangle, a glyph stem — is declined
deliberately and permanently for the *exact* substitution: its cross-section is not constant along
its length, so a single coverage across a pixel line would be worse than what the rasteriser already
does. ADR 0226 argues it, and small text is the case that makes it a rule rather than a caution.
ADR 0268's substitute does not touch a fill at all, so it does not reopen the question.

## 4. A clip boundary that falls where another clip boundary already fell — **the chain is paid, and so is a fill's own coverage**

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

### What was paid next: a fill's own coverage — **ADR 0355**

**The witness is `issue21346.pdf`**, which states the same device rectangle six times over — a
`W n`, three `/BBox` clips under §8.10.1 step c), the mark's own path and the mask group's — and its
edge has gone **0.041 → 0.163 → 0.306** of the mark, where departure (1) would give 0.827 and the
clause gives 1.000. `poppler` and `ghostscript` give 1.000, `mupdf` 0.755, `hayro` 0.327; the page
stays `CONTRADICTED_COINCIDENT_CLIP_EDGES` in `oracle.rs`, its failing similarity 0.9734 → 0.9781 →
**0.9846** against a bound of 0.9900.

**The price ADR 0280 wrote for this was wrong, in ADR 0268's direction.** It said reaching the mark
meant "not handing the mask to the library at all — rasterising coverage into a buffer of this
backend's own, composing with `min`, and blitting — which is this backend's own blitter". All three
steps are already public API on `tiny-skia`: `Mask::fill_path` is the buffer, `PixmapMut::fill_rect`
through that mask over the whole device pixels the mark reaches is the blit, and the composition is
a `min` between them. No scan converter and no blitter of ours. It costs +1.21% of the rasteriser on
a page of text and +5.54% on the corpus's heaviest clip page — and **+54% before one reused coverage
buffer replaced one allocation per mark**, which is the part worth remembering: `tiny-skia` takes a
mask only at the pixmap's own size, so a per-mark allocation is a band's worth of zeroing per mark.

**The corpus population is not the witness's.** It is a §12.5.5 widget appearance whose border rule
sits on the `/BBox` §8.10.1 step c) clips it by — `bug1844576.pdf`, `bug1844583.pdf`,
`issue16473.pdf`, `issue18823.pdf`, `multiline.pdf`, `textfields.pdf` — which is the same finding
`render-quorra`'s differing list wrote about `issue21068.pdf` in the two-hundred-and-seventh
session, when a *redundant* clip came off its comb separators.

### What was paid after that: a clip *beside* a soft mask — **ADR 0363**

The fold `MaskCache::combine` performs is one buffer, and the standard states the two in an order
the fold destroyed. §8.5.4 intersects the clipping path with the object's **own** shape — "[t]he
effective shape is the intersection of the object's intrinsic shape with the clipping path" — and
§11.3.7.2 then multiplies the mask shape into what comes out: "[t]he three shape inputs shall be
multiplied together, producing an intermediate value called the source shape". So
`fₛ = (fⱼ ∩ C) · fₘ` and not `fⱼ · (C · fₘ)`.

The cache keeps the soft mask's rows beside the product, and the composition needs nothing else,
because multiplication by a non-negative value distributes over a minimum: `min(M, C)·S =
min(M·S, C·S) = min(M·S, P)`. Rounding is monotone too, so the eight-bit form is *exact* rather
than bounded. The ladder, a half-plane at device 2.25 under a coincident clip and a mask of 128:
the mark's own 192, the mask alone 96, the product taken as a value **72**, the composition **96**.

**No corpus page can tell the two apart, and the population is why.** Over the 974 first pages:
120 commands take a clip and a soft mask together, 27 of them are fills reaching the composition,
14 decline because the clip is already a set under the mark, and **13 compose** — over
`bug1703683_page2_reduced.pdf`, `bug1721218_reduced.pdf`, `issue16287.pdf`, `issue17069.pdf` and
`issue18032.pdf`, whose rasters are byte-identical before and after at scale 1 and at 4×. The two
compositions part only where `M` *and* `C` are both fractional in one pixel, and at all thirteen
the mark is whole where the clip is not.

### What is left of item 4, each with what it needs

- **A stroke's coverage.** `tiny-skia` fills a wide stroke's outline but draws one under a device
  pixel wide as a hairline that is *not* that outline (ADR 0268), so composing here means choosing
  between duplicating the library's stroker and contradicting its hairline. The substitutions
  §10.7.4 already asks for on a sub-pixel rule are *fills* and are composed today.
- **An image's edge**, which is `draw_pixmap`'s and is the library's own path.
- **A group's raster, which ADR 0355 recorded as *not owed* and which is owed.** That ADR argued
  "what a group's buffer carries at a pixel is §11.4.5's group alpha rather than one mark's
  coverage, so there is no second shape here"; §8.5.4's own third sentence answers it — the shape
  of a transparency group is "defined as the union of the shapes of its constituent objects" and
  "shall be influenced both by the clipping path in effect when each of the objects is painted and
  by the one in effect at the time the group's results are painted onto its backdrop". A group has
  a shape and the clip at the blit intersects it. What is true is narrower: **this backend's group
  buffer carries alpha, which is shape times opacity** (§11.3.7.1), and the two coincide only where
  every element's opacity is 1 — §11.6.4.2's default, which `ca`, `CA` and a nested soft mask make
  false. So what it needs is **a shape channel beside a group's raster**, which nothing in this
  tree carries today and which would cost a band's bytes per live group.

  **And it is where `issue21346.pdf`'s next factor actually is**, which is the correction below.

### The witness's residual is not where ADR 0355 said it was

ADR 0355 wrote that the witness's mark "carries a soft mask *and* a clip … so what arrives at
`scan::fill` is a `Clip::Value`". **Nothing on that page arrives at `scan::fill` that way**
(ADR 0363). Instrumented, page one takes the clip-and-mask pair exactly twice and both consumers
are a *group's* plain blit; the only two marks reaching the composition are clipping regions with
no mask at all. Its similarity is 0.9846 before this round and after it.

With the group's raster meeting the clip as a set instead of by the product — the identity's own
answer, since that page's group lies inside its clip — device column 14 of row 89 goes
`(240, 245, 249)` to `(227, 237, 244)` against an interior of `(206, 223, 235)`: **0.306 → 0.571**
of the mark, where departure (1) gives 0.827 and the clause 1.000. The construction measured is
`min(group alpha, clip)` with the blit then carrying no mask — alpha standing in for the shape it
is not, which is the same approximation the item above says has to go away. So the group blit is one of the
remaining factors and not the last of them, and a round taking it owes the ladder again.

Two things bound any attempt at the rest:

- **`min` is not exact for boundaries that merely share a pixel**, only for ones that coincide or
  nest. What is exact is intersecting the *paths* and rasterising once.
- ~~**`render-quorra` still multiplies its chain**, inside the graphics library, so the two backends
  now compose clips by two different rules.~~ **Answered: quorra takes `min` there too** (its ADR
  0030, `doc/QUORRA_FEEDBACK.md` §18), reached from §8.5.4's own sentence rather than from this
  tree's reading of it — the graphics state holds *one* clipping path, so rasterising each link
  separately is a convenience and nothing in the standard composes two fractional coverages. **No
  gate could see it before and none can see it now**: product chain, `min` chain and `min` chain
  with a `min` mark all give 915 / 37 / 5 over the 957 pages, with no per-page line moving. Both
  sides still multiply where the clip meets the **mark**, and quorra records that as a choice with
  the same reason this file gives — two unrelated boundaries in one pixel are the common case, and
  only a conflation-free rasteriser answers the clause. **This tree stopped multiplying there for a
  *fill* in ADR 0355 and quorra has not**, so the two part at the mark: the cross-backend gate went
  934 agree / 20 differ to 930 / 24, every arrival a widget border sitting on its own `/BBox`, and
  `doc/QUORRA_FEEDBACK.md`'s twenty-fourth section is the ask. The magnified lane does not see it.

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

## 5. Two marks that abut anywhere — **witnessed, measured, and not this program's** (ADR 0308)

Item 2 above is this one inside a tiling cell, and the *unwitnessed general case* it hands on is
narrower than what a document actually does. **A witness arrived from the project owner in the
four-hundred-and-seventy-third session**: a 50 MB Inkscape geological cross-section whose page is
one 148 MB content stream of **58 003 `f` operators, 2 868 970 curve segments, 57 413 colour
changes and no clipping path at all**. A dark green frame rule runs under it, and the owner's
screenshot shows the rule shining through the polygons drawn over it — and disappearing when the
page is magnified.

It reproduces exactly, and the mark either side of it is stated: the rule is two stroked
rectangles sharing a coincident edge (user x 1329.8279 and 1329.914, display-list commands 1 and
2), and the polygons over it are ordinary opaque fills three to seven device pixels across at page
scale. **The seam is what §11.3.7.3 states.** Result shape is the *union* of the backdrop's and
the source's, "an 'inverted multiplication' -a multiplication with the inputs and outputs
complemented", so two marks each covering half a boundary pixel unite to three quarters of it and
a quarter of whatever lies beneath survives. Four marks covering a quarter each leave `0.75⁴`, and
*n* of them rise towards `1/e`: **the seam gets worse the more pieces a drawing states its region
in**, which is why a cross-section of 58 003 polygons is where it is seen and a page of text is
not.

The numbers, all from `crates/render-quorra/tests/abutting_marks.rs` and
`crates/pdf-model/examples/uncovered_share.rs`:

```text
  two opaque rectangles abutting mid-pixel, share of the backdrop still showing
    render-cpu (tiny-skia)   0.2510      Union(0.5, 0.5) leaves   0.2500
    render-quorra            0.2471      four quarter-covers      0.3164
    render-gpu (vello)       0.2510        cpu 0.3137, quorra 0.3137, vello 0.3176

  the same fixture, the references
    mutool draw              0.2510
    gs -dGraphicsAlphaBits=4 0.2200
    gs, anti-aliasing off    0.0000      — §10.7.4's own aliased rule, which has no seam
    pdftoppm                 0.0000      — and this one is **not** conflation handling

  the owner's page, share of a layer under the fills still showing (interior pixels)
    1x 0.1937    2x 0.1282    4x 0.0673    8x 0.0156
```

**`poppler`'s zero is trap 9 and the control says so.** A single rectangle whose right edge lands
half way across a device pixel reads 0.498 here, 0.533 on `mupdf` and 0.467 on `gs` — and **0.000**
on `pdftoppm`, which snaps an axis-aligned rectangle to whole pixels and is therefore aliased for
exactly the shape the fixture is made of. Restate the same pair of marks with a seam that is *not*
axis-aligned and `poppler` leaks too: 1.765 device pixels over the fixture's square against ours
2.698, `mupdf`'s 2.871 and `gs`'s 1.004. Three renderers with true analytic coverage — this tree's
two, and `mupdf` — agree to a level of 255; the two that do not are the two that are not
anti-aliasing the shape.

So **nothing here is `render-quorra`'s** and no feedback section is owed: it answers 0.2471 where
the processor answers 0.2510 and vello 0.2510, all three inside one level of 255 of the arithmetic.

### What it would take, and what that costs

Only one thing removes it: **the marks are resolved against one another before either is
composited.** Two constructions, both priced rather than taken:

- **Draw at *N*× and box-filter down.** This is what `-dGraphicsAlphaBits` is, and its 0.2200
  above is that filter's quantisation rather than conflation — it is *not* exact either. Cost is
  `N²` of the rasteriser and `N²` of the raster's memory, on the one path where this project has
  said latency is the feature. It is the cheap answer to write and the expensive one to run, and
  it would be paid on every page including the ones that do not need it.
- **A conflation-free rasteriser**, keeping each boundary pixel's marks as sub-pixel geometry
  until the pixel is finished rather than as one accumulated coverage. That is the same project
  item 4 above names for the clip that meets a mark — "this backend's own blitter" — and it has to
  answer what a blend mode and a transparency group do to a fragment list, which item 4's version
  does not.

Neither is started, and the reason is written down rather than left as a silence: **the artefact
shrinks by roughly half per doubling of the magnification and is gone by 8×**, so it is worst
exactly where a reader is looking at a whole page rather than at its detail, and both cures cost
their most on the frame where time-to-first-page is measured. A round that takes this owes the
measurement above at both ends.

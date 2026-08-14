# ADR 0355 — The clip that contains a mark takes nothing from it

Date: 2026-08-14 (session 520)
Status: accepted

## Context

ADR 0280 read §10.7.4's clipping paragraph and stopped one step short of the mark. It composed a
clip *chain* with `min` — a set intersected with itself is that set — and wrote down what it did
not take:

> **The mark's own coverage still multiplies into the clip mask**, twice on the witness page. It
> is the same sentence and it needs this backend's own blitter.

`doc/todo/11`'s item 4 has carried that half since, priced as "rasterising the mark's coverage
into a buffer of this backend's own, composing, and blitting — which is this backend's own
blitter, and is the same project the conflation-free rasteriser is". **The price was wrong**, in
the direction ADR 0268 found the last one wrong: `tiny-skia` already exposes both halves of that
construction as public API, so the composition needed no scan converter and no blitter of ours.

## The clause, and the closed form

§10.7.4's clipping paragraph:

> For clipping, the clipping region consists of the set of pixels that would be included by a fill
> operation. Subsequent painting operations shall affect a region that is the intersection of the
> set of pixels defined by the clipping region with the set of pixels for the region to be
> painted.

§8.5.4 states the same thing about a *value*:

> The effective shape is the intersection of the object's intrinsic shape with the clipping path;
> the source shape value shall be 0.0 outside this intersection.

Both are set intersections, and **the closed form this round tests against is the set identity
rather than any renderer's arithmetic**: `S ∩ C = S` where `S ⊆ C`. A mark drawn under a clip that
contains it is the mark drawn under no clip at all, at every pixel including the boundary's. A
product does not have that property — two coverages of `c` in one pixel give `c²` — and the whole
of ADR 0280's argument for `min` over a product carries over unchanged, because it was an argument
about two coverages meeting and not about which two:

1. a product moves away from the clause with every restatement, in the direction the same
   subclause's "[t]he area covered by painted pixels shall always be at least as large as the area
   of the original shape" forbids;
2. `min` is exact where the two boundaries coincide or nest;
3. `min` is never below the product elsewhere, so it is never further from the clause's whole
   pixel than the product is.

**Where the clause is not silent about a product, nothing changes**: §11.6.5's soft mask is a
*value* and Table 142 multiplies it into the object's alpha. `scan::Clip` therefore has three
variants rather than two — `Unclipped`, `Region` and `Value` — and a clip already multiplied into
a soft mask by `MaskCache::combine` is a `Value`, because once the two are one buffer there is no
set left to intersect with.

## Decision — `render-cpu` composes a clipping region with the mark by `min`

`scan::intersected` builds the mark's own coverage with `tiny_skia::Mask::fill_path` — the same
scan converter, the same transform handling and the same anti-aliasing test as the ordinary draw —
takes the smaller of that and the region per pixel, and blits the paint through the composed mask
over the whole device pixels the mark reaches. `tiny_skia::PixmapMut::fill_rect` with an integer
rectangle and no anti-aliasing puts coverage 255 into every pixel of the run, so the mask *is* the
composition and the mark's coverage cannot enter the product a second time.

Three declines, each because the substitution would otherwise say something else:

- **A clip that is already a set** — every value under the mark 0 or 255 — takes the ordinary
  call, because there the product *is* the intersection pixel for pixel. This is a correctness
  statement first and the cost control second, and it is what keeps the change off the pages that
  do not need it.
- **A mark that is not anti-aliased**, whose own coverage is 0 or 255 for the same reason.
- **`BlendMode::Source`**, which is `carries_coverage_as_alpha`'s existing exclusion (ADR 0226).
  The construction delivers coverage as the *mask* of a fully covered run, and `tiny-skia` applies
  a mask by scaling the source where it applies a path's coverage by interpolating towards the
  destination. The two are the same function for every mode whose result has Porter-Duff's form —
  which is what `BlendMode::should_pre_scale_coverage` says of the seven modes it names, and what
  the algebra says of the rest, since scaling a premultiplied source by `c` scales `αs` and leaves
  the unpremultiplied colours the blend function reads — and they part for Source, where the
  destination does not enter the result at all. §11.4.6's knockout is where this backend states
  that mode.

**The paint carries the transform the library would have applied to it.** `fill_path` with a
non-identity transform transforms the path, calls `paint.shader.transform(ts)` and recurses with
an identity transform; drawing a device rectangle instead means performing that same step one call
earlier. That is trap 2's exact shape — a paint is positioned in the path's space — so the scene
that guards it is a **gradient** under a coincident clip, at three scales and once rotated, against
the unclipped render of the same geometry: a shader transform dropped, doubled or mirrored moves
the gradient inside the rectangle while every coverage stays where it was.

### What it is worth, on the geometry

`scan.rs`'s own unit scene, a half-plane whose edge falls at device 2.25 under a clip with the
same edge, alpha of the boundary column:

```text
  the mark's own coverage, unclipped     192 of 255   (0.75 of a pixel, 0.7529 as tiny-skia quantises it)
  the product, which is what was drawn   144          (0.7529² = 0.5669)
  min, which is what is drawn now        192          — the clause's own S ∩ C = S
```

`clip_intersection.rs` walks the same identity through the display list: with the composition
removed the three scenes fail by 15, 39 and 64 levels of 255, and with it they agree with the
unclipped render **to the byte at scale 2 and 4 and within one level at scale 1**. That one level
is the composition's own and is stated as a bound rather than measured away: the clip's mask and
the mark's coverage are two eight-bit rasterisations of one edge, and `min` takes the smaller, so
where the two quantisations disagree by a level the composed value is a level under the mark's own.

## The witness, one rung further, and what still holds it down

`issue21346.pdf` states one device rectangle six times over. ADR 0279 measured its edge at 0.041
of the mark, ADR 0280's chain took it to 0.163, and this round takes it to **0.306** — device
column 14 of row 89, `(240, 245, 249)` against an interior of `(206, 223, 235)` — where departure
(1) would give 0.827 and the clause gives 1.000. The page is still `CONTRADICTED`, and closer:
mean 0.22 → 0.19, worst tile 0.77 → 0.64, similarity **0.9781 → 0.9846** against a bound of
0.9900.

**Only one of that page's two remaining products came out, and the reason is worth more than the
number.** The mark carries a soft mask *and* a clip, and `MaskCache::combine` multiplies the two
into one buffer before either reaches a draw — so what arrives at `scan::fill` is a `Clip::Value`,
the clip's own boundary is inside it, and the composition declines exactly as it is written to.
What did come out is the fill inside the mask group's own render, where the clip is alone. Taking
the other one means keeping the clip and the soft mask apart as far as the blit and composing
`min(mark, clip) × soft` in the buffer this round already builds — one more pass over the reach,
and a cache that retains the soft mask's rows beside the product rather than only the product.
`doc/todo/11` item 4 carries it with this number.

## The population it moves, which is not the witness's

The corpus feels this somewhere else: **a widget appearance whose border sits on its own `/BBox`**.
§12.5.5's appearance stream is clipped by §8.10.1 step c), the box rule the producer drew lands on
that boundary, and the clip that contains it was halving it.

The oracle's 1794 per-page lines: **not one verdict moved** (906 agree, 67 contradicted, 786
ambiguous, 13 not comparable, 19 no render, before and after), and **88 lines moved numerically**,
38 with the similarity rising, 20 falling and 30 unchanged in that column. The two largest are the
form pages `bug1844583.pdf` (ssim 0.8738 → 0.5956) and `bug1844576.pdf` (0.8050 → 0.6506), and
both are 181×54-pixel pages where a similarity window is a handful of pixels: at 4× the two renders
are indistinguishable, and at the page's own scale 300 and 488 pixels differ, all of them on the
field's border rows, with the page's ink rising 17.6% and 10.6%. **They move away from the
references and towards the clause**, which is the same result ADR 0280 recorded on its own witness
and is what principle 5 asks be reported rather than chased: poppler, mupdf and ghostscript all
multiply the clip into the mark.

`doc/todo/00` step 7's ink sweep, before and after over all 786 ambiguous pages: **241 rows moved
and 240 of them up**. The negative tail — the alarm — is intact and less negative: twenty at or
past −1 with sixteen of them documents this tree calls incomplete, and the four complete ones the
same four names in the same order, `issue16038.pdf` −5.734 → −5.655, `issue12295.pdf` −2.823 →
−2.821, `issue14297.pdf` −1.145 → −1.129, `issue7821.pdf` −1.000 unmoved. The one row that moves
*down* is `issue1905.pdf` −0.213 → −0.242, and it is the direction ADR 0280 explained on a
different page: a clip that admits more of a *pale* mark subtracts ink rather than adding it.
The largest movers are the same form pages plus `issue16473.pdf` −0.683 → **+2.432**.

## What it costs, measured

`examples/callgrind_rasterise`, instructions for one page:

| page | before | after | |
|---|---|---|---|
| ISO 32000-2 page 101 | 5 493 757 950 | **5 560 102 612** | **+1.21%** |
| `bug1721218_reduced.pdf`, 3554 clips | 19 644 191 539 | **20 733 209 623** | **+5.54%** |

**The first version of this cost +54% on the second page, and the difference is one buffer.**
`tiny-skia` will only take a mask of the pixmap's own size, so a coverage buffer is a band's worth
of bytes; allocating and zeroing one per mark on a page that states 3554 clipped fills is 1.7 GB of
zeroing. `scan::Scratch` lives in the `MaskCache` — one per strip of a parallel render, which is
what keeps it out of any shared state — and only the pixels a mark can reach are cleared. The
oracle's 1794 lines are byte-identical across that change, which is what says it was a performance
change and nothing else.

## The two backends, and the ask

`render-quorra` composes a clip with a mark inside the graphics library and multiplies, as ADR
0280 recorded it doing and as its own ADR 0030 chose deliberately. The cross-backend gate
therefore parts on four pages: 934 agree / 20 differ before, **930 / 24** after, the arrivals
being `bug1844576.pdf`, `bug1978317.pdf`, `issue16473.pdf` and `issue18823.pdf` — every one of
them a widget appearance with a border on its `/BBox`, which is the population above. One page
moved the other way inside the list: `issue16038.pdf`, mean 1.3235 → 1.2808, because a tiling
cell's clip is coincident with the rule it admits. `doc/QUORRA_FEEDBACK.md`'s twenty-fourth section
is the ask, with this ADR's unit ladder as the reproduction.

**The magnified lane does not see it at all**: `PDFVIEWER_QUORRA_COVERAGE=gpu
PDFVIEWER_QUORRA_SCALE=4` gives 937 agree / 9 differ / 5 refused / 23 not comparable before and
after. A boundary pixel is a smaller share of a mark at four times the scale, which is the same
shape ADR 0308 measured on the abutting-marks seam and is worth knowing about this defect too: it
is worst where a reader looks at a whole page.

`render-gpu` is unchanged and multiplies as well. It is not the oracle and no gate compares it
over the corpus, so what it costs is `test-scenes`' fixtures — which are unmoved, none of them
stating a clip coincident with a mark.

## What is not done, and what each would need

- **A stroke's coverage still multiplies.** `tiny-skia` converts a wide stroke to its outline and
  fills it, but draws one under a device pixel wide as a hairline that is *not* that outline (ADR
  0268), so composing a stroke's coverage here would mean choosing between duplicating the
  library's stroker and contradicting its hairline. The substitutions §10.7.4 already asks for on
  a sub-pixel rule are *fills* and go through the composed path today.
- **An image's edge still multiplies.** `draw_pixmap` is the library's own path and an image's
  alpha is not a coverage this side holds.
- **A group's raster still multiplies**, and that one is not owed: what a group's buffer carries
  at a pixel is §11.4.5's group alpha rather than one mark's coverage.
- **A clip folded into a soft mask keeps multiplying**, which is the witness's remaining factor
  above and is the cheapest of these to take next.
- **`min` is still not exact for two unrelated boundaries in one pixel**, only for coincident or
  nested ones. It is never worse than the product, which is the whole argument for taking it, and
  the exact answer is the conflation-free rasteriser `doc/todo/11` item 5 prices.

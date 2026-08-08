# ADR 0236 — A gradient drawn across a page, to be kept in a few dozen pixels

Status: accepted, 2026-08-08 (session 399).

## Context

`doc/todo/40` was the corpus's worst page and had been for a hundred sessions. It stated its item
as *a clip chain as one crop and one intersect*, on a profile taken in the hundred-and-thirteenth
session: `tiny_skia::pipeline::lowp::gradient` 36.6%, `Mask::intersect_path` 8.1%,
`build_soft_mask` 8.0%, and the note that the two mask lines are one item worth 24.34% of the page.
The file said the item was **priced and blocked on a measurement**. This round took it.

**The measurement said the item was not the largest thing on the page and the profile had said so
all along.** The gradient line was already first at 36.6%, and it was first because of what the
page is made of, which nobody had asked. Two instruments, five minutes apart:

- `pdfimages -list` names **no image at all**, so session 391's warning about a cost paid per
  source sample does not apply here.
- The content stream, decompressed, holds **3490 `sh` operators** against 479 `f`, each in the
  shape `q · tiny path · W n · q · 0 0 612 792 re · W n · q · cm · BX /Sh0 sh EX · Q · Q · Q`.

That is what a gradient mesh looks like when a producer exports it as thousands of small patches,
and it meets the rule ISO 32000-2 §8.7.4.2's Table 76 states for the operator:

> Paint the shape and colour shading described by a shading dictionary, subject to the current
> clipping path.

The clipping path is the *only* bound, so `pdf-model` states such a command as a fill of the whole
page whose paint is the shading. A rasteriser then evaluates its shader over the spans of the
**path** and multiplies the clip mask in afterwards — so every column the mask rejects is shaded at
full price and thrown away. The same table says what that costs, and it is the clause's own
sentence rather than an inference: an unbounded shading "paints the shading's gradient fill across
the entire clipping region, which may be time-consuming".

`crates/pdf-model/examples/clip_chain_census.rs`, written for this round, counts it. On
`bug1721218_reduced.pdf` page one:

| | |
|---|---|
| commands, nested groups included | 7022 fills, 29 groups, 28 strokes |
| of those, fills painted by a shading | **3511** |
| device pixels inside those fills' own paths | **1 701 795 744** — 484 704 each, which is the page |
| device pixels inside their clips as well | **85 608** — 24 each |

A ratio of **19 878** on the paths and, once the band has already narrowed the rows, **122** on
what the rasteriser actually shades: 10.4 M pixels a render to keep 85 608.

It is not one document's shape. Over all 974 corpus first pages — 958 censused, 16 refused by the
census's own `expect` — **49 carry a shading fill, and 99.7% of their shaded pixels lie outside
the clip**; 21 documents keep under half. `issue840.pdf` keeps 0.30%, `bug1703683_page2_reduced.pdf`
0.09%, `personwithdog.pdf` 2.44%.

## Decision

### A rectangular fill is drawn as the part of it its mask can mark

`pdf_render::cropped_rectangle` replaces an axis-aligned rectangle with its intersection with a
rectangle the caller says the mask is zero outside of. `render-cpu`'s `MaskCache` already computed
that rectangle in `MaskCache::build` and kept only its rows; it keeps the columns now, in
`Built::admits`, and `MaskCache::effective` hands them to the fill through a new `Admitted` struct
beside the band and the mask.

**Why the pixels cannot move**, which is the whole of why this is allowed in the correctness
oracle's own rasteriser:

- Replacing an axis-aligned rectangle by a smaller one changes coverage in exactly two places —
  between the two rectangles, and in pixels straddling a *new* edge. Both lie outside the retained
  rectangle's interior, and the mask is zero there, so every changed coverage is multiplied by zero.
- The crop is computed in the **path's own space**, by mapping the device rectangle back rather
  than mapping the path forward. A side that is not narrowed is therefore `f32::max` of a
  coordinate against a smaller one, which is that coordinate's own bits — not a restatement of it —
  so the rasteriser's arithmetic on an unnarrowed edge is the arithmetic it already did.
  `a_side_the_mask_does_not_narrow_keeps_its_own_bits` asserts it by bits.
- The rectangle is outset by a whole pixel and rounded outward before it is used, exactly as
  `Band::covering` treats the rows and for the same reason: a clip's bounds compose the clip's
  transform with the page's, while the mask composes the band's in as well, and the two agree to
  within rounding rather than exactly. The outset also guarantees the crop is at least two device
  pixels across, which keeps it clear of §10.7.4's sub-pixel rule — a rectangle cropped *thinner*
  than a pixel would be drawn down a different path through the backend and would not be the same
  picture.
- Non-rectangles are declined outright. A general path clipped to a rectangle is
  re-parameterised, which is ADR 0139's measurement and is not exact.
- A transform that does not preserve the axes is declined: under a shear a rectangle is a
  parallelogram, and cropping a parallelogram's *bound* cuts the shape.

### A clip that covers the whole surface records no rectangle at all

`MaskCache::build` stores `admits: None` where the rectangle reaches every pixel of the surface.
That is not tidiness — it is what keeps the crop off a page with nothing to gain. The commonest
clip in the corpus is a page-wide rectangle (ADR 0132's page 6 wraps 303 text runs in one), and
asking once per chain *built* costs a containment test where asking once per fill *drawn* costs
5933 of them. Measured both ways below.

## Measurement

`examples/callgrind_rasterise`, twenty renders of one page, A/B in one sitting on the same machine
with the same corpus, `--release`. Callgrind counts instructions, which is the instrument this page
needs: ADR 0139 records that it grants no legal strip boundary, so it is drawn serially and
`doc/performance.md`'s rule — the clock for a parallel change, the counter for a serial one —
selects the counter.

| 20 renders | before | after | |
|---|---|---|---|
| `bug1721218_reduced.pdf` page 1 | **38 453.3 M** | **20 030.7 M** | **−47.9%** |
| ISO 32000-2 page 6 | 4 004.7 M | 4 007.7 M | +0.08% |
| ISO 32000-2 page 101 | 5 531.7 M | 5 523.1 M | −0.16% |

Where it went on the worst page, self cost:

| | before | after |
|---|---|---|
| `tiny_skia::pipeline::lowp::gradient` | 15 783.8 M (41.05%) | 578.9 M (2.89%) |
| `tiny_skia::pipeline::lowp::mask_u8` | 1 810.3 M (4.71%) | under the threshold |
| `<CpuRasterizer>::build_soft_mask` | 3 430.6 M (8.92%) | 3 430.6 M (17.13%) |
| `<MaskCache>::get`, inclusive | 8 335.3 M (21.68%) | 8 302.6 M (41.46%) |

The mask lines do not move by a single instruction of their own; what changes is that they are now
the page. Wall clock over seven samples of the same twenty renders, quoted only because the
difference is an order of magnitude past the spread: **2.45–2.63 s before, 1.43–1.52 s after**.

The +0.08% on page 6 is what is left of the containment test after the paragraph above; asked per
fill instead of per chain it was **+0.42%** (4 021.7 M), which is how that decision was made rather
than assumed.

## Proof the pixels held

- The page's own PNG through `examples/open_one` is **byte-identical** before and after, checked
  with `cmp` after each of the four builds this round produced.
- `callgrind_rasterise` prints the sum of every byte of twenty rasters, and it is **9666314140**
  both ways.
- Every gate in `doc/todo/02` §2 run and read: corpus `974 documents, 0 unopenable, 8 locked,
  2 encrypted, 5 pageless, 67 incomplete, 0 slow`; oracle `904 agree / 69 contradicted /
  786 ambiguous`, undiagnosed ambiguous list empty; text `99.2% (24043/24243 words), 25 below 90%`;
  quorra `916 agree, 36 differ, 5 refused, 17 not comparable`. Every one of those reproduces
  `doc/HANDOVER.md`'s recorded line character for character.

## What the round found about the item it was sent to take

`doc/todo/40` stays open, and all three of the things it said about itself moved. The census and
one temporary counter in `MaskCache::admit` re-derived them:

| the file said | it is |
|---|---|
| worth "most of `MaskCache::get`'s 24.3%" | worth **42% of that function**, because intermediates are barely shared: 3551 leaf clips reach through **7066 distinct nodes**, so building each once replaces 3551 fills and 10 702 intersects with 7065 intersects and as many crops |
| blocked because the page "already peaks at 27.9 MB against `MASK_BUDGET`'s 32 MB" | **not blocked**: the peak is **12.31 MB** (10.85 clip, 1.45 soft, three products), and the intermediates cost **+9.4 MB** |
| justified by "a mask value at a given device row does not depend on which band holds that row (`band.offset()` is a translation)" | **the sentence ADR 0219 refuted.** `ToDevice` composes the band's first row into the translation *last*, and shifting `ty` by a whole number of rows moves `y·sy + ty` into another binade — fewer than one pixel in ten thousand, none by more than one supersample, but this backend is the oracle |

The 27.9 MB was true in the hundred-and-thirteenth session and stopped being true in the
hundred-and-forty-seventh, when ADR 0132 made `DisplayList::add_clip` return an existing identifier
for an identical region. **A margin recorded as thin is a claim that decays like any other**, and
the standing example in `CLAUDE.md` is about a claim that a clause says nothing; this is the same
failure wearing a number.

## Consequences

- The corpus's worst page is half of what it was, and its largest remaining cost is the clip chain
  the round was sent to look at — now 41.5% of it rather than 21.7%.
- **The graphics backend has the same defect and this does not fix it.** `render-quorra` is handed
  the same display list and encodes the same 3490 page-sized rectangles into its scene; the crop
  lives in `pdf-render` rather than in `render-cpu` precisely so that it can call it, and
  `doc/QUORRA_FEEDBACK.md` is where that ask belongs. Nothing about the display list changed, which
  is why the quorra gate cannot have moved and did not.
- `pdf-model` could crop at the display list instead, which would serve both backends at once and
  is the tidier place. It was **not** done there, and the reason is the pixel of margin: the margin
  is a device-pixel notion and `pdf-model` does not know the device scale, so cropping there would
  either need a fudge in page units or would rest on an exactness this round could not prove.
- `crates/pdf-model/examples/clip_chain_census.rs` is kept. It answers a question no profile can —
  whether a page's clip chains *share* their intermediates — and it is what says `doc/todo/40` is
  worth 42% of a function rather than most of it.

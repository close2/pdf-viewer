# ADR 0279 — Two pages that were the harness's crop, and a clip that multiplies its own edge

Date: 2026-08-11 (session 443)
Status: accepted

## Context

`doc/todo/00`'s method, applied to the contradicted list rather than the ambiguous bucket. The list
stands at 68 of 1794 pages, 66 of them on documents this tree calls complete, and it has been worked
three times — sessions 405, 406 and 431 — each of which found the *diagnosis* to be the defect
rather than the page. This round took three pages off session 406's ranking and off the two-member
group that had never been re-derived.

Twelve rounds have moved pixels since 431, so the round's first question was whether any group was
describing a page that had changed underneath it. **None had.** Every one of the 1794 per-page lines
the gate prints is byte-identical before and after this round's diff, and the step-7 sweep over all
68 contradicted pages reproduces session 431's head to the thousandth. What was wrong was written
down rather than drawn.

## Decision 1 — `CONTRADICTED_PAGE_ROUNDING` is emptied, because this tree's raster was never the smaller one

The group has held since the sixth session on one sentence: a page box whose size is fractional, and
at 72 dpi "we and `ghostscript` produce a raster of one size while `poppler` and `mupdf` produce one
a pixel wider". Rendered straight through `pdf-model/examples/render_at` at scale 1:

| | this comment said | our own render | `poppler`, `mupdf` | `ghostscript` |
|---|---|---|---|---|
| `colorkeymask.pdf` | 595 × 842 | **596 × 842** | 596 × 842 | 595 × 842 |
| `issue21346.pdf` | 178 × 178 | **179 × 179** | 179 × 179 | 178 × 178 |

`TargetSpec::for_page` rounds a fractional page **up** so that the raster contains it, and
§10.7.4's ledger row has said so since the sixty-first session — the same rounding that makes ADR
0064's y-flip anchor a question at all. So on both pages this tree is with `poppler` and `mupdf`, and
`ghostscript` is the renderer that truncates.

### What was misread is an artefact, and it is doing what it is documented to do

`<stem>-p<n>-ours.png` under `<target>/tmp/oracle/` is our raster **after
`normalise::to_common_size` cropped it to the smallest size any voting reference produced**, which on
both pages is `ghostscript`'s. The reference PNGs beside it come from the render *cache* and are not
cropped. A directory listing therefore shows `ours` at 595 or 178 next to a `poppler` at 596 or 179,
and reads exactly like this tree rounding down. Both were checked rather than argued: our own render
cropped to the reference's size is byte-identical to the artefact, `magick compare -metric AE` = 0 on
both.

**The rule this earns is trap 1's, arriving in an instrument rather than in a count**: the only place
our page size can be read is a render of our own. It is written where the file is produced
(`pdfref::report::write_artefacts`) as well as where it was misread (`oracle.rs`).

Both pages stay contradicted and both are now grouped by what they differ by. That is the tenth and
eleventh time a group's name in `oracle.rs` has named a hypothesis rather than a diagnosis, and the
first time the hypothesis was contradicted by the harness's own output.

## Decision 2 — `colorkeymask.pdf` is §10.7.4's image paragraph carried out, and the consensus is wrong

Nine commands: a §8.7.3 tiling pattern whose cell draws one 200 × 267 `/DeviceRGB` image with
§8.9.6.4's `/Mask [255 255 0 255 0 255]`, under `200 0 0 267 0 0 cm` and the pattern's
`/Matrix [1 0 0 1 18 557]`. Device region x `[18, 218)`, y `[17.9998, 284.9998)` — **one device pixel
per source sample exactly**, no reduction and no `/Interpolate`.

Ours is **byte-identical to `ghostscript` over the whole 595 × 842 raster**. `poppler`, which votes
with `mupdf`, differs from both on 942 pixels of 500 990: 268 apiece in device columns 78, 138 and
218, and 141 in device row 17. §10.7.4 decides all four groups and decides for us:

> However, only those pixels whose centres lie within the region shall be painted. The position of
> the centre of such a pixel -in other words, the point whose coordinate values have fractional parts
> of one-half -shall be mapped back into source space to determine how to colour the pixel. There
> shall not be averaging over the pixel area.

- Device row 17's centre is at y 17.5, outside the region. Nothing is painted there and we paint
  nothing there; `poppler` paints it. Rows 18 to 284 are 267 rows for 267 samples.
- Device column 78's centre is at x 78.5 → source x 60.5 → sample **60**. The image's row 110 is
  `(255, 0, 0)` at samples 58 and 59 and `(0, 255, 0)` at 60 and 61, read out of the file's own
  uncompressed bytes. Ours paints `(0, 255, 0)`; `poppler` paints `(130, 201, 77)`, which is neither
  sample. Columns 138 and 218 are the image's other two colour boundaries.

This is **not** ADR 0025's departure seen from the other side. That departure averages the samples
sharing a device pixel when an image is *reduced*; at one sample per pixel it has nothing to average,
and the clause's own sentence is what runs. The page fails one bound and it is the worst tile, 5.03
against 5.00 — 942 pixels at up to 255 levels gathered into three one-pixel columns is what a tile
maximum is for — while the ink agrees to **0.03 of 255** across all five renderers, which is why
nothing else notices.

`CONTRADICTED_IMAGE_SAMPLE_AT_THE_PIXEL_CENTRE`, and it joins
`CONTRADICTED_VISIBILITY_EXPRESSION` as the second entry on that list where the specification answers
against the two renderers that agree.

## Decision 3 — a clip chain composes by multiplication, and it is left that way with the price written down

`issue21346.pdf` is 178.34645 points square and holds one mark: a 150 × 150 square of
`(0.227, 0.498, 0.690)` at a mask value of 0.25 over white. The interior is not in dispute — the
closed form is `0.25 c + 0.75` per channel, **(206, 223, 235)**, and ours, `poppler`'s,
`ghostscript`'s and `hayro`'s centre pixel is that byte for byte. What differs is the one-pixel
border, and this round is the first to look at it.

**Every construction on the page states the same device rectangle**, `[14.173, 164.173]` on both
axes: the page's `W n`, form 15's `/BBox` and form 13's `/BBox` under §8.10.1 step c), form 13's own
fill path, the §11.6.5.2 mask group's `/BBox` and the mask group's own fill.
`examples/clip_chain_census` says the first three outright — *clip references 3, distinct leaves 2,
distinct clip nodes 3, chain depth histogram {2: 1, 3: 1}*.

### The ladder

One 178.34645-point page, one fill of a rectangle whose left edge lands at device 113.386 at 8×,
under **n** `W n` clips of the same rectangle. Coverage of the boundary column:

```text
  boundaries      1       2       3       4       5       6
  coverage    0.5025  0.2487  0.1218  0.0609  0.0305  0.0152
```

Each rung is the one above it halved. The coverages are multiplied; the small deficit against an
exact `0.5025ⁿ` is the byte each mask is stored in. On the corpus page the six boundaries put the
edge at **0.041 of the mark where the geometry is 0.827 of it**, level 253 against an interior of
206.

### The clause, which neither `oracle.rs` nor §10.7.4's ledger row had cited

> For clipping, the clipping region consists of the set of pixels that would be included by a fill
> operation. Subsequent painting operations shall affect a region that is the intersection of the set
> of pixels defined by the clipping region with the set of pixels for the region to be painted.

A clipping region is a *set of pixels*, taken by the fill rule that includes any pixel the path meets
however little of it is covered. So the clause paints this edge at **1.000**, which is what `poppler`
and `ghostscript` do. `mupdf` gives 0.755 and `hayro` 0.327 — both anti-alias their clips and conflate
fewer times than we do. This tree's documented departure (1) from the subclause would give **0.827**.
It gives 0.041, and a product of six coverages is neither the clause nor the departure: it moves
further from the clause with every nesting level, in the direction the same paragraph's "[t]he area
covered by painted pixels shall always be at least as large as the area of the original shape"
forbids.

### Why it is not changed

`min` is the composition a set intersection asks for: exact where two boundaries coincide, never
below the product, therefore never further from the clause than the product is. Three things stop it
being this round's fix, and none of them is difficulty:

1. **Only the clip chain is this tree's to compose.** A mark's own coverage meets the clip mask
   inside `tiny_skia`'s fill, and a soft mask is §11.6.5's alpha and a genuine product. Three of this
   page's six factors would survive, taking the edge 0.041 → 0.064 against a bound that wants 0.827.
   **The verdict would not move.**
2. **`render-quorra` composes its clips the same way and is not this project's to change**
   (`doc/RENDER_LIBRARY.md`), while `render-cpu` is the correctness oracle. Changing one side alone
   makes `render-quorra/tests/corpus.rs` report a difference that is a deliberate divergence, which
   is the one thing that gate cannot say.
3. **`min` is exact only for boundaries that coincide or nest**, not for two that merely share a
   pixel. What is exact is intersecting the *paths* and rasterising once — a conflation-free
   rasteriser, which is a project rather than an item.

So the departure is named in §10.7.4's ledger row as its fourth, with the ladder, and
`doc/todo/11` gets item 4. **A departure that is measured and written down is not the same as one
nobody had noticed**, which is the whole of what this decision buys: before this round the row
described three departures and the fourth was drawing a corpus page at a twentieth of its geometry.

## Decision 4 — a substituted symbolic face costs a quarter of its ink, and the closed form is the two programs

`issue15716.pdf` was the head of the contradicted ranking that is not a JBIG2 page or a link border:
**3.10 from the nearest reference against 3.92 from the furthest**, which step 1 of `doc/todo/00`
reads as *we are alone*. It has sat in `CONTRADICTED_SUBSTITUTED_FONT` since the
hundred-and-forty-eighth session under one sentence and no number.

The page is 200 × 200 points, a §8.7.3 tiling pattern of a 100 × 100 cell, sixteen marks that are
four glyphs drawn four times: `/BaseFont /ZapfDingbats`, `/Differences [1 /a109 /a110 /a111 /a112]`,
`64 0 0 64 … Tm` with `/F1 1 Tf`, two black under one `/OC` layer and two red under another. Each
glyph is placed by its own `TD`, so **no advance affects any position** and the only unknown is the
outlines. `pdf_font::standard` answers from `FoxitDingbats.pfb` (ADR 0133); the three C references
resolve `D050000L` through this machine's fontconfig.

Glyph areas from the two font programs' own charstrings, scaled by `(64/1000)²` and taken four times,
against the areas the rasters carry — black from `(1 − mean R) × 200²`, since a red glyph leaves R at
255, and the total from the same over G:

| | black px² | red px² | total |
|---|---|---|---|
| **`FoxitDingbats`, from its charstrings** | **6147.1** | **6373.6** | **12 520.7** |
| ours | 6129.2 | 6382.7 | 12 511.9 |
| `hayro` | 6102.3 | 6406.7 | 12 509.0 |
| **`D050000L`, from its charstrings** | **8200.5** | **7081.5** | **15 282.0** |
| `poppler` | 8212.8 | 7081.8 | 15 294.5 |
| `mupdf` | 8189.5 | 7079.4 | 15 269.0 |
| `ghostscript` | 8188.5 | 7078.4 | 15 266.9 |

**Every renderer paints the area its own font program states, to a fifth of a percent**, and the
whole 18.1% difference is the two programs' outlines. No reference is trusted anywhere in that table:
both closed forms come out of the two files.

What the two faces share is exactly what the standard states. The advances are **626, 694, 595 and
776** in *both* — Adobe's published ZapfDingbats metrics, §9.6.2.1's Table 109 half honoured on both
sides — and `a110`, the heart, has the same outline in both to 0.2% of its area. `a109` is 20.0%
smaller in area in Foxit's face, `a111` 24.5% and `a112` 28.7%, which is why the red pair (holding
the shared glyph) is 10.0% apart and the black pair 25.0%.

So the group's three mechanisms are now measured and they are three sizes: a substituted **serif**
costs nothing measurable (session 431), a substituted **sans** costs one number — `/CapHeight`, 5.7%
(ADR 0267) — and a substituted **symbolic** face costs a quarter of its ink while costing nothing at
all in placement. All three are §9.5 NOTE 5, and the third is the plainest instance of it in the
file, because a dingbat *is* its outline. The face is not changed, for ADR 0267's reason one clause
along.

## The gates

Nothing under `crates/` that produces a pixel was touched: the diff is `oracle.rs`'s group comments
and arrays, one doc comment in `tools/pdfref`, the ledger, `doc/todo/11` and the documents. So the
rasters are byte-identical by construction, and they were checked rather than assumed — **every one
of the oracle's 1794 per-page lines is identical before and after**, 905 agree / 68 contradicted /
786 ambiguous, 66 of the contradicted on complete documents.

`doc/todo/00`'s step 7 was run over the contradicted list (§3b's population) and reproduces session
431's head to the thousandth: `issue5751.pdf` −5.115 [incomplete], `issue4436r.pdf` −2.203,
`issue9243.pdf` −1.549, `smask_luminosity_oob_transfer.pdf` −0.779, `issue7580.pdf` −0.482 and
nothing else past −0.4; positive tail `issue14802.pdf` +9.982 and `issue11740_reduced.pdf` +13.704.
**Twelve rounds of change and the list's ink has not moved anywhere**, which is what that alarm is
for. A before/after pair over the *ambiguous* bucket is not owed, on the reason session 406 recorded:
with no pixel moved the two halves would compare a file with itself.

`tools/spec-errata` was run and its §9.6.2.2 hits were read rather than counted: all four — two in
`pdf-font/src/lib.rs` and two in `standard.rs` — quote a sentence Errata Collection 3 struck
*because* it was struck, each saying so in the same paragraph. That is the tool's looser section
working as designed and there is nothing to correct there.

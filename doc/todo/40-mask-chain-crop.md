# A clip chain as one crop and one intersect

Status: **priced, and unblocked** — the measurement the file was waiting for was taken in the
three-hundred-and-ninety-ninth session (ADR 0236), and it moved every number here.
Priority: 40
Corpus: 1 document (the worst page in the corpus, by a wide margin) — **and two web documents
since the four-hundred-and-thirty-fifth**, which want the *same* crop for a different buffer
Code: `crates/render-cpu/src/lib.rs`, `MaskCache::build`, `CpuRasterizer::build_soft_mask`,
`initial_backdrop`

## A second buffer that is target-sized, and two documents that pay for it

**Added in the four-hundred-and-thirty-fifth** (ADR 0271). A clip mask is built over its own
band — `MaskCache::build` computes the chain's device bounds and `Band::covering` narrows to
them — and **a soft mask and a transparency group's buffer are not**: both are allocated at
`surface.width() × surface.rows.height` on the stated ground that clips are resolved against the
surface, so one coordinate system beats two that have to agree. That is a real argument and it
is why this is an item rather than a defect. What it costs, on the two slowest documents of
65 944 crawled ones:

| | page | groups | buffer allocated | band actually used |
|---|---|---|---|---|
| `0423548.pdf` | 1843 × 5103 | 136 (132 non-isolated) | **4.3 GB** | 82 MB — **52×** |
| `6081357.pdf` | 2552 × 1693 | 1831 (1 non-isolated) | **31.6 GB** | 487 MB — **65×** |

The *arithmetic* over those buffers is gone — `SoftMask::outside` took `6081357.pdf` from 52.6 s
to 3.9 s without touching the allocation — so what is left is the copying and the clearing:
`initial_backdrop` is **2.85 s of `0423548.pdf`'s remaining 6.6**, one whole-surface `Source`
draw per non-isolated group.

**The soft mask's band is not the clip's band, and that is the part to settle first.** A clip
outside its band admits nothing, so a band-sized mask and "zero elsewhere" are the same thing. A
soft mask outside its group's marks takes `SoftMask::outside`, which is zero for `/Alpha` and for
a black `/BC` and is **255 for a white one** — so a band means "constant outside" rather than
"nothing outside", and `Built` would have to carry the constant beside the band. That is a
change to what a `MaskCache` entry *is*, which is why it is here and not in ADR 0271.


`bug1721218_reduced.pdf` is the corpus's worst page: 144.05 G instructions → 54.05 G when a ramp
stopped carrying 256 stops for a linear function (ADR 0068) → 43.13 G when the built shading was
cached per object (ADR 0069) → **20.03 G** when a rectangular fill stopped being drawn wider than
its mask can mark (ADR 0236). Those are twenty renders through `examples/callgrind_rasterise`.

**The item this file was written for was never the largest thing on the page**, and the profile it
quoted said so: `gradient` was 36.6% and the two mask lines 24.34%, and the file added the two mask
lines together and left the first one alone. What that line was is `sh` — 3490 of them, each a
page-sized rectangle under a clip that admits about 24 pixels — and cropping the rectangle halved
the page. What is left, in order, after the crop:

```text
build_soft_mask                      17.1%
Mask::intersect_path                  8.3%
fill_path_impl                        7.7%
calloc                                4.5%
gradient                              2.9%
```

`MaskCache::get` is now **41.5%** of the page, inclusive, and is its largest cost. The item is
still this: **a child's band is inside its parent's**, so a chain could be one crop and one
intersect instead of a fill and three.

## What the measurement said, against what this file used to say

`crates/pdf-model/examples/clip_chain_census.rs` counts the clip tree; one temporary counter in
`MaskCache::admit` measured the peak.

| this file said | it is |
|---|---|
| worth "most of `MaskCache::get`'s 24.34%" | **42% of that function**, so ~3.5 G of the page's 20.03 G, **17%** |
| blocked: "the page is already at 87% of `MASK_BUDGET`" (27.9 MB of 32) | **not blocked**: the peak is **12.31 MB**, and the intermediates cost **+9.4 MB** |

The sharing is what decides the first, and it is poor: **3551 leaf clips reach through 7066
distinct nodes**, 1.99 nodes per leaf against chains 4.01 deep. The depth histogram says why — one
node at depth 1, one at depth 2, then **3494 at depth 3 and 3518 at depth 4**, so the chains share
their first two ancestors and nothing below. Building each node once replaces 3551 fills and 10 702
intersects with 7065 intersects and as many band-sized copies; at the profile's 29 472 instructions
an `intersect_path` and 13 102 a `Mask::fill_path`, that is 361.9 M per render against 208.2 M plus
the copies.

The memory figure was true when it was taken (session 113) and stopped being true in session 147,
when ADR 0132 made `DisplayList::add_clip` return one identifier for an identical region.

## What is *not* settled, and is the reason this is still a file

**The justification in `MaskCache::build`'s comment is the sentence ADR 0219 refuted.** It says a
mask value at a given device row does not depend on which band holds that row, "`band.offset()` is
a translation". `ToDevice` composes the band's first row into the translation **last**, and ADR 0219
measured what shifting `ty` by a whole number of rows does to `y·sy + ty`: fewer than one pixel in
ten thousand, none by more than one supersample. A parent's mask rows are therefore *nearly* the
prefix's contribution for the child's band, and this backend is the correctness oracle.

So taking the item means one of:

- building each intermediate in the **child's** band, which is not a cache — each chain would want
  its own copy of every ancestor — unless the crop is followed by a re-fill, which is the cost back;
- proving the difference away, which ADR 0219 says no arrangement of this crate's arithmetic
  closes;
- or taking it, measuring it against the oracle, and recording whatever it moves as a departure.

The third is the honest one and the round that takes it should say so before it starts.

## The other half, which is not this file's

`render-quorra` is handed the same display list and encodes the same 3490 page-sized rectangles.
`pdf_render::cropped_rectangle` is in the shared crate so that it can call it; nobody has.
`doc/QUORRA_FEEDBACK.md` is where that belongs.

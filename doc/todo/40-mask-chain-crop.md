# A clip chain as one crop and one intersect

Status: **open — the chain-sharing half.** The copying half was taken in the
four-hundred-and-ninety-third session (ADR 0328), byte-identically, and is no longer owed.
Priority: 40
Corpus: 1 document (the worst page in the corpus, by a wide margin)
Code: `crates/render-cpu/src/lib.rs`, `MaskCache::build`

## The half that was taken, and why it could be exact

This file used to price a second buffer beside the clip chain: a soft mask and a
transparency group's buffer were allocated, filled and stored at
`surface.width() × surface.rows.height` while the work lived in a band — 4.3 GB of
backdrop copies for 82 MB of band on `0423548.pdf`'s 132 non-isolated groups, and 912
surface-sized mask conversions and stores on `6081357.pdf`. ADR 0328 banded **the copy,
the conversion and the storage** and left **the drawing** surface-sized, which is the
line that made it exact: the departure this file warned about — ADR 0219's supersample,
`fl(p + ty) − k ≠ fl(p + ty − k)` — lives in the drawing transform, and none of the three
banded steps touches one. `Built` carries `SoftMask::outside` beside the band, exactly
the change to what a `MaskCache` entry *is* that this file said would be needed, and the
one reader that needs a whole-surface raster (a soft mask with no clip to band the draw)
is served by a memoised expansion. The numbers, the exactness argument and the 42
byte-identical renders are ADR 0328's; `open_one` on the two documents above went
**−48.8% and −79.2%** in instructions.

## The half that is still owed: the chain itself

`bug1721218_reduced.pdf` is the corpus's worst page: 144.05 G instructions → 54.05 G
(ADR 0068) → 43.13 G (ADR 0069) → 20.03 G (ADR 0236) — and session 493's A/B measured
today's base at **13.83 G**, so the sessions between kept taking pieces off it (ADR 0271's
transparent-pixel shortcut the largest); ADR 0328's banding is worth **−1.1%** on it,
because its three soft masks were already cheap. Those are twenty renders through
`examples/callgrind_rasterise`. What remains largest on it is `MaskCache::get`: every
chain is built from the root — a fill and depth-minus-one intersects — although **a
child's band is inside its parent's**, so a chain could be one crop of the parent's rows
and one intersect.

What the three-hundred-and-ninety-ninth session's census (`clip_chain_census`, ADR 0236)
established still stands:

- **Worth about 42% of `MaskCache::get`** — 3551 leaf clips through 7066 distinct nodes,
  1.99 nodes per leaf against chains 4.01 deep; building each node once replaces 3551
  fills and 10 702 intersects with 7065 intersects and as many band-sized crops.
- **Not blocked on memory**: the peak is 12.31 MB against `MASK_BUDGET`'s 32, and the
  intermediates cost +9.4 MB.
- **Not obviously pixel-exact, and that is the open question.** A parent's mask rows are
  only *nearly* the prefix's contribution for the child's band: `ToDevice` composes the
  band's first row into the translation last, and ADR 0219 measured what shifting `ty` by
  a whole number of rows does to `y·sy + ty` — fewer than one pixel in ten thousand, none
  by more than one supersample, and this backend is the oracle.

So taking it still means one of:

- building each intermediate in the **child's** band, which is not a cache — each chain
  would want its own copy of every ancestor — unless the crop is followed by a re-fill,
  which is the cost back;
- proving the difference away, which ADR 0219 says no arrangement of this crate's
  arithmetic closes;
- or taking it, measuring it against the oracle, and recording whatever it moves as a
  departure.

The third is the honest one and the round that takes it should say so before it starts.
ADR 0328 is the precedent for the *shape* of the split — band what is outside the drawing
arithmetic, decline what is inside it — not for taking the departure.

## The other half, which is not this file's

`render-quorra` is handed the same display list and encodes the same 3490 page-sized
rectangles. `pdf_render::cropped_rectangle` is in the shared crate so that it can call
it; nobody has. `doc/QUORRA_FEEDBACK.md` is where that belongs.

## What upstream's parallel result still says

quorra's ADRs 0036–0039 sized a layer, a soft mask and the root to what the plan marks,
measured `issue16287.pdf` from 291 199 104 frame bytes to 6 158 496 with no verdict moved
— the same argument as ADR 0328's, on the other backend, with the departure they may take
and the oracle may not. Their census lesson stands for whoever takes the chain item:
most pages mark most of their area, the gain is in the tail, and the census comes before
the code.

# A clip chain as one crop and one intersect

Status: **open — the chain-sharing half, and it is now a priced choice rather than an open
question.** The copying half was taken in the four-hundred-and-ninety-third session (ADR 0328),
byte-identically. The *covering-step* half — which this file did not know existed — was taken in
the seven-hundred-and-forty-seventh (ADR 0656), also byte-identically, and it is worth more on the
page this item is about than the sharing half's departure would be.
Priority: 40
Corpus: 1 document (the worst page in the corpus, by a wide margin); 50 of 958 first pages carry
any covering step at all and the rest of them carry 147 between them
Code: `crates/render-cpu/src/lib.rs`, `MaskCache::build`; `crates/render-cpu/src/scan.rs`,
`admits_every_pixel`
Instrument: `cargo run --release -p pdf-model --example clip_chain_census -- <file.pdf> [page]`,
which prints all three arms and the covering-step count. **Every number below comes off that
command or off `examples/callgrind_rasterise`; none of them is written down here.**

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

## The half this file never named, and which was taken instead

**A chain step whose path is a device rectangle containing the mask admits every pixel of it.**
§10.7.4 states the region as "the set of pixels that would be included by a fill operation", so
such a step contributes the whole set and the `min` that composes it is the identity — and
declining it is not a form of reuse at all. Nothing is carried between bands, so ADR 0219's
arithmetic never enters, which is what separates it from everything below.

The census counts them, and on the corpus's worst page it is three chain steps in four. ADR 0656
has what it moved and what it cost; the short version is a tenth off that page's whole run, a third
off `MaskCache::get`, and `raster_digest` byte-identical on every corpus first page.

**One line of it is worth keeping here because it generalises**: the predicate answers from the
*rectangle* rather than from what the scan converter would have done, and two thirds of what it
finds is only reachable that way — the page wraps every chain in a rectangle thousands of pages
across, which `tiny-skia`'s fixed point cannot express, so `mask_fill` sends it to the library with
anti-aliasing off. Containment is exact whatever the converter would have made of the coordinate.

## The half that is still owed: the chain itself

`bug1721218_reduced.pdf` is the corpus's worst page: 144.05 G instructions → 54.05 G
(ADR 0068) → 43.13 G (ADR 0069) → 20.03 G (ADR 0236) — and the sessions between kept taking pieces
off it (ADR 0271's transparent-pixel shortcut the largest, then ADR 0328's banding, then ADR 0656's
covering steps), while every correctness feature that landed in this backend put some back. **Do not
quote a figure from that ladder as today's base**: the four-hundred-and-ninety-third session's was
26% low when the seven-hundred-and-forty-seventh re-ran it, and re-running it is one command.

What remains largest on it is still `MaskCache::get`: every chain is built from the root,
although **a child's band is inside its parent's**, so a chain could be one crop of the parent's
rows and one intersect.

What the three-hundred-and-ninety-ninth session's census (`clip_chain_census`, ADR 0236)
established still stands, re-run in the seven-hundred-and-forty-seventh with the counts unmoved —
they are a property of the display list, and the interpreter has not changed it:

- **Intermediates are barely shared** — 3551 leaf clips through 7066 distinct nodes, 1.99 nodes per
  leaf against chains 4.01 deep; building each node once replaces 3551 fills and 10 702 intersects
  with 7065 intersects and as many band-sized crops.
- **Not blocked on memory**: the peak is 12.31 MB against `MASK_BUDGET`'s 32, and the
  intermediates cost +9.4 MB.
- **Not obviously pixel-exact, and that is the open question.** A parent's mask rows are
  only *nearly* the prefix's contribution for the child's band: `ToDevice` composes the
  band's first row into the translation last, and ADR 0219 measured what shifting `ty` by
  a whole number of rows does to `y·sy + ty` — fewer than one pixel in ten thousand, none
  by more than one supersample, and this backend is the oracle.

**And the third bullet now has a price, which is the thing that was missing for three hundred
rounds** (ADR 0656). The census prints an `exact` arm — reuse restricted to the prefixes a parent
shares a band with, which are reusable byte for byte — beside the `full` one. The exact arm is a
small single-digit percentage of the page's scanned mask rows where the full arm is about half of
them, and corpus-wide the gap is the same shape. Half the page's non-root nodes *do* share their
parent's band, so the exact arm is not short of candidates: it is worth almost nothing because the
sharing is one-to-one, and building an intermediate that serves a single leaf moves work rather
than removing it.

So the three roads this file used to offer collapse to one honest sentence: **everything worth
taking on this road costs ADR 0219's departure, in the backend that is the oracle.** The roads,
restated with that known:

- building each intermediate in the **child's** band is not a cache — each chain would want its own
  copy of every ancestor — unless the crop is followed by a re-fill, which is the cost back;
- proving the difference away buys the `exact` arm, which the census now prices and which is not
  worth a round on its own;
- or taking it, measuring it against the oracle, and recording whatever it moves as a departure.

The third is still the honest one and the round that takes it should say so before it starts.
ADR 0328 is the precedent for the *shape* of the split — band what is outside the drawing
arithmetic, decline what is inside it — not for taking the departure.

**And there is now a witness outside the 974 that is worse than anything this file names**, found
by the eight-hundred-and-fifty-seventh session's walk of the Tika issue-tracker corpus's Mozilla
directory (`doc/todo/03` §30): `corpus-cache/tika-issue-tracker/batch3/MOZILLA/MOZILLA-831621-14.pdf`
opens in 2.1 ms and interprets in 414 ms into **3166 commands referencing 3149 distinct clips** —
very nearly one clip apiece, where `bug1721218_reduced.pdf`'s census counts 3551 leaves through
7066 shared nodes — and then spends **41 seconds** rasterising them onto a 1280 × 800 target, with
nothing reported. It is not diagnosed further here and it is not a substitute for the census: what
it is, is a page on which this file's subject is the *whole* cost rather than the largest term, and
a round taking the third road above should measure on it as well as on the corpus's worst page.

## The cheapest thing left, and it is not the chain

**`convert::path` runs for every chain node**, including the three in four ADR 0656 now drops, and
it is a few per cent of what is left of that page. Deciding droppability against the **surface**
rather than against the band would let the conversion be skipped entirely for those nodes — a
rectangle containing the surface contains every band of it, so the test is strictly stronger and
band-independent. What it costs is computing the chain's device bounds from the source
`pdf_render::Path` rather than from the converted `tiny_skia::Path`, and `admits` is derived from
those bounds, so the round that takes it owes a byte-identity run rather than an argument.

## The other half, which is not this file's

`render-quorra` is handed the same display list and encodes the same 3490 page-sized
rectangles. `pdf_render::cropped_rectangle` is in the shared crate so that it can call
it; nobody has. `doc/QUORRA_FEEDBACK.md` is where that belongs.

**And ADR 0656's saving is this backend's alone so far.** The covering steps are a property of the
*document*, not of the rasteriser, so quorra encodes the same redundant rectangles into the same
scenes; whether its encoder already collapses them is a question for that feedback file.

## What upstream's parallel result still says

quorra's ADRs 0036–0039 sized a layer, a soft mask and the root to what the plan marks,
measured `issue16287.pdf` from 291 199 104 frame bytes to 6 158 496 with no verdict moved
— the same argument as ADR 0328's, on the other backend, with the departure they may take
and the oracle may not. Their census lesson stands for whoever takes the chain item:
most pages mark most of their area, the gain is in the tail, and the census comes before
the code.

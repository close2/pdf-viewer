# ADR 0656 — The clip step that states nothing, and the price of `doc/todo/40`'s exactness

Status: accepted, 2026-08-25. Session 747. Re-derives `doc/todo/40`'s price, finds that the item's
value is almost entirely in the departure it declines, and takes a fourth road it did not name.
Clause: ISO 32000-2 §10.7.4, whose ledger row moves with it.

## What this round was asked, and what it found first

`doc/todo/40` has stood open since ADR 0236 on a price taken in the three-hundred-and-ninety-ninth
session, and the standing rule is that a number this project wrote down and has not re-run is the
highest-yield thing a general round can check. Three of that item's figures were re-taken with
`pdf-model/examples/clip_chain_census` and `examples/callgrind_rasterise`, on the same page:

- **The census's own counts have not moved at all.** `bug1721218_reduced.pdf`'s first page still
  reaches 3551 distinct leaf clips through 7066 distinct nodes, still at 4.01 steps a chain and
  1.99 nodes a leaf. They are a property of the display list and the interpreter has not changed
  it.
- **The base has.** The file says 13.83 G instructions for twenty rasterisations. It is 17.47 G,
  and the sessions between are the reason — every correctness feature that landed in this backend
  since is in that number.
- **So has the share.** `MaskCache::get` was 24.3% of that run and is 29.96%. Separating the
  one-off from the slope (`repeats` 1 against 2: 2.33 G fixed, 0.740 G a rasterisation) makes it
  **35.4% of a rasterisation**, which is the honest denominator for a per-render change.

The item's price therefore stands and is larger than written. What changed the decision is a
number nobody had.

## 1. The exactness question had never been priced, and it is nearly the whole item

`doc/todo/40`'s open half is *reuse*: build a chain from its parent's cached mask instead of from
its root. Its third bullet says the reuse is "not obviously pixel-exact", because `Surface::to_device`
composes a band's first row into the translation and ADR 0219 measured what shifting a whole number
of rows does to `y·sy + ty`. The file offered three roads — build intermediates in the child's band,
prove the difference away, or take the departure and record it — and called the third the honest one.

**None of them had a number.** The census now prints two arms so that they cannot be argued in the
abstract again:

| arm | what it reuses | scanned mask rows on that page |
|---|---|---|
| today | nothing; every chain from its root | 66 394 |
| `exact` | only a prefix whose band *equals* the child's, which is byte-for-byte reusable | 62 688, **−5.6%** |
| `full` | every node once, cropped from its parent | 32 439, **−51.1%** |

Half the page's non-root nodes (3546 of 7065) do share their parent's band, so the exact arm is not
blocked for want of candidates — it is worth almost nothing because the sharing is one-to-one: a
depth-3 node on that page serves a single depth-4 leaf, so building it separately moves the work
rather than removing it. Over 958 corpus first pages the two arms are **−4.0%** and **−14.0%**.

**The consequence is the decision, and it is negative**: there is no cheap exact version of the
reuse. Anything worth taking on that road costs ADR 0219's departure, in the backend that is the
oracle, and that is now a priced choice rather than an open question.

## 2. The fourth road: a step that admits everything is declined, not reused

The same census asked one further thing, and it is where the round's code went. A chain step whose
path is a device rectangle containing the mask **admits every pixel of it**, so it contributes the
whole set and the `min(kept, 255)` that composes it is the identity. On `bug1721218_reduced.pdf`
**10 596 of 14 253 chain steps are one** — three in four.

Declining such a step is not a form of reuse and shares none of its problem: nothing is carried
between bands, so the arithmetic ADR 0219 prices never enters. It is §10.7.4 read forwards —

> For clipping, the clipping region consists of the set of pixels that would be included by a fill
> operation.

— and for a rectangle containing the mask that set is every pixel, by containment.

`scan::admits_every_pixel` is the predicate and it lives beside `mask_fill` rather than beside its
caller, because it is a claim about what `mask_fill` writes and the two would otherwise drift.
`MaskCache::build` now computes each shape's `rectangular_mark` once, in a `retain_mut` that drops
the shapes stating nothing, and fills the mask with `u8::MAX` in the case where all of them do.

### Why it does not ask `Exact::usable`, which is where two thirds of what it finds are

The first version asked the condition `mask_fill` itself branches on — anti-aliasing kept and the
rectangle inside `SUPERSAMPLED_LIMIT` — and dropped **7045** steps rather than 10 596. A probe
attributed the gap exactly: **3553 declines with `usable = false` and `one = true`**, one per chain.
That page wraps every chain in a page-covering clip stated as a rectangle thousands of pages across,
which `tiny-skia`'s fixed point cannot hold, so `mask_fill` sends it to the library with
anti-aliasing off.

Every pixel is still inside the rectangle, which is the only thing the predicate claims. Answering
from the rectangle rather than from the converter is also the more defensible of the two: §10.7.4
defines the region by the set a fill *would* include, and containment settles that whatever a
converter which cannot express the coordinate would have produced.
`a_rectangle_beyond_the_expressible_range_still_fills_every_pixel` pins that the two agree today,
under both settings of `anti_alias`, so this is a saving rather than a correction.

`Exact::Several` and `Exact::Shared` are declined rather than unioned: whether several portions'
union covers a pixel is a question about *area* that `mask_shared_rectangles` answers by summing,
and a containment test cannot stand in for it.

## 3. What it moved, and what it cost

Every figure is `examples/callgrind_rasterise`, twenty rasterisations, on this round's own build.

| | before | after | |
|---|---|---|---|
| `bug1721218_reduced.pdf` page 1, whole run | 17 471 143 843 | 15 807 895 387 | **−9.52%** |
| the same, per rasterisation (slope of `repeats` 1 → 2) | 739 915 947 | 657 936 980 | **−11.1%** |
| `MaskCache::get`, inclusive | 5 233 622 983 | 3 570 743 625 | **−31.8%** |
| ISO 32000-2 page 101, whole run | 5 425 499 707 | 5 438 025 378 | +0.23% |
| ISO 32000-2 page 6, whole run | 3 650 806 002 | 3 655 160 337 | +0.12% |

**The two regressions are not this change**, and the measurement says so rather than the argument:
`MaskCache::get` on page 101 goes from 48 920 542 to 48 968 064 — **47 522 instructions**, four
ten-thousandths of the page's delta. Those pages build one chain a rasterisation, so there is
nothing here for the predicate to cost them. What moved is the binary's layout, which a change in
this crate moves whatever it is. It is recorded because `CLAUDE.md` asks for an optimisation's cost
in writing, and the honest cost is "not measurable in the code that changed, and 0.2% of a text page
in the link".

**The pixels are identical.** `examples/raster_digest` over the pdf.js corpus is byte for byte the
same on all 957 first pages it rasterises, and `callgrind_rasterise`'s own ink sum is unchanged on
every page measured above.

## 4. What the tests are for, since no page can discriminate this

The saving is byte-identical by arithmetic, so trap 13's "run the sweep against the defect first"
had to be answered with a planted defect rather than a document. A one-axis containment test — the
shape a check written for one axis and copied takes — fails **both**
`a_rectangle_short_of_the_mask_admits_less_than_every_pixel` and
`a_chain_step_admitting_the_whole_band_changes_no_byte_of_the_mask`, which is what the two are for:
the first pins the predicate against the closed form, the second pins that a chain wrapped in
covering rectangles builds the mask the bare chain builds, bytes, band and rectangle alike.

`a_rectangle_containing_the_mask_is_filled_to_the_last_level` is the load-bearing one and the only
instrument in this tree that could see the thing which would break this: `mask_rectangle` writing
254 somewhere. No corpus page can, because today the two routes agree.

## 5. What is left, priced

- **`convert::path` still runs for every chain node**, including the three in four now dropped —
  751 M of the remaining run, 4.8%. Deciding droppability against the *surface* rather than the band
  would let the conversion be skipped, because a rectangle containing the surface contains every
  band of it; what it costs is computing the chain's device bounds from the source path rather than
  from the converted one, and `admits` depends on those bounds.
- **The reuse item itself**, now with its exactness priced at 5.6% against 51.1%. `doc/todo/40`
  carries the two arms and the command that prints them.

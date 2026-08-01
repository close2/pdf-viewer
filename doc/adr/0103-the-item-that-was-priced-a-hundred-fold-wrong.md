# ADR 0103 — The item that was priced a hundred-fold wrong

Status: accepted, 2026-08-01.

## Context

`doc/HANDOVER.md` has carried this item on its short list of small things for several sessions:

> **Bound a group's buffer to the band its clip admits.** The CPU backend gives every
> transparency group a page-sized pixmap … No corpus page pays for it; a page with hundreds of
> groups would. Measure first — `callgrind_rasterise` over a group-heavy page.

and named the page that would show it:

> The next item is the one already listed above — the CPU backend gives every transparency group
> a page-sized pixmap — and this is the page that would show it: `calloc` and
> `Mask::intersect_path` between them are an eighth of it.

The instruction to measure first was in the item. This session did.

## What the measurement says

`bug1721218_reduced.pdf` page 1 under `callgrind_rasterise`, 43.15 G instructions — the same
page and the same number ADR 0069 left behind, so nothing has drifted.

| | share | |
|---|---|---|
| `tiny_skia::pipeline::lowp::gradient` | 36.58% | unchanged, and still the page |
| `Mask::intersect_path` | 8.08% | plus 3.08% of the `calloc` below |
| `CpuRasterizer::build_soft_mask` | 7.95% | four masks per render, 88 instructions per pixel |
| `fill_path_impl` | 6.40% | |
| `calloc` | 4.48% | **3.08% from `Mask::intersect_path`, 1.31% from `Mask::new`, 0.14% from `Pixmap::new`** |

**The group buffer is `Pixmap::new`, and it is 0.14% of the page.** 580 calls over the example's
twenty iterations — 29 group pixmaps per render — costing 61.7 M of 43.15 G. The 4.48% of
`calloc` the handover attributed to it is *clip* masks: `Mask::intersect_path` allocating a
temporary per call and `Mask::new` allocating the band. Bounding the group buffer to its band
would save about a thousandth of the page named as the one that would show it.

**The real second item is the clip cache: `MaskCache::get` is 24.34%** — 10.5 G, the largest
thing on the page after the gradient. And it is *intrinsic work*, not a cache defect, which took
three measurements to establish:

- **No eviction.** 3608 chains built per render, 0 evictions, peak 27.9 MB against a 32 MB
  budget. The cache is not thrashing.
- **Almost no duplication.** The display list holds 7207 clips for that page, of which 7116 are
  distinct by content. Deduplicating at `DisplayList::add_clip` would remove 1.3% of them.
- **Rectangles are not the common case here.** 3681 of the 7207 clips are axis-aligned
  rectangles, but only **1840 of 72 160 chains** — 2.5% — are rectangles *all the way up*, so a
  fast path that writes a rectangular mask directly instead of scan-converting it would reach
  one chain in forty.

Each of those three was a plausible optimisation before it was counted, and each is now priced
and declined with its number.

## The correction that is worth more than the numbers

`MaskCache::build`'s own comment explained why a chain is drawn in one piece:

> a parent covers a different band from its child, so a parent's mask cannot be reused as a
> starting point

**That is backwards.** The band comes from the running *intersection* of the chain's bounds, so
a child's bounds are contained in its parent's and its band is contained in the parent's band.
And a mask value at a given device row does not depend on which band holds that row, because
`band.offset()` is a translation. So a parent's rows *are* exactly the prefix's contribution to
the child, and a chain could be one crop plus one `intersect_path` rather than a fill plus
depth-minus-one intersects — on a page whose chains average four deep, most of 24.34%.

What stops it is memory, not correctness. It needs every *intermediate* clip cached, and an
intermediate's band is larger than the leaf's, on a page already peaking at 87% of the budget.
Taking it means measuring the intermediates and sizing `MASK_BUDGET` from that first.

## Decision

- **Remove the group-buffer item from the to-do list**, with 0.14% written down beside it. It
  is not that it is small; it is that the page chosen to demonstrate it demonstrates its
  absence.
- **Record the clip cache's real number and its three declined shortcuts** in the code rather
  than only in the handover, since the handover is where the last mis-pricing lived.
- **Correct `MASK_BUDGET`'s justification.** It said "3576 distinct clips" and "25.5 MB … with
  headroom"; the measurement is 3608 chains and 27.9 MB, which is **13% of margin**. The
  constant is not raised — nothing has been measured to be paying for it — but the next session
  that finds a document evicting should know the margin was already thin rather than that
  something regressed.
- **Correct `build`'s reason**, and leave the optimisation it unlocks written down with the
  condition that would let it be taken.

No code path changed and no gate moved; this session's output is four numbers and two corrected
comments.

## Consequences

The habit this pays into is already in `doc/HANDOVER.md` — "Measure an entry before believing
its label, including a label written here" — and it has now been paid three times: the
subdivision lattice's "needs a Gouraud rasteriser in both backends", the profile whose largest
item was four times the share recorded, and this. **The pattern in all three is the same: the
sentence beside the number was doing the reasoning, and the number was never taken.**

The new instance sharpens it. The previous two were labels that *understated* what a change
would buy. This one overstated it by two orders of magnitude, and would have cost a session's
work on a coordinate-system change to save a thousandth of one page.

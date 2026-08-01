# ADR 0132 — One region, stated three hundred times

Status: accepted, 2026-08-01. Session 147. Found by the profile ADR 0131 took for a different
reason.

## What the profile said

`callgrind_rasterise` on page 6 of ISO 32000-2, twenty renders, 16 771 M instructions:

| | |
|---|---|
| `tiny_skia::scan::path::fill_path_impl` | 24.5% |
| **`calloc`** | **18.3%** |
| `SuperBlitter::blit_h` | 17.0% |

`callgrind_annotate --tree=both` puts **every instruction** of that `calloc` under
`tiny_skia::Mask::new`, called 6060 times — 303 per render. `examples/glyph_reuse` says why: the
page uses **303 distinct `ClipId`s of one distinct region.** The producer wraps each of its 303
text runs in `q … W n … Q` with the same clipping rectangle, and this tree gave each one an
identifier of its own.

`render-cpu` caches the effective mask of a clip chain, keyed by the leaf's `ClipId`. That key is a
**name**. Three hundred and three names for one region is three hundred and three page-wide masks
allocated, zeroed, filled and thrown away — and the cache reports a 100% hit rate the whole time,
because every lookup it was asked to answer was answered.

This is ADR 0115's lesson with the sign reversed. There, a cache key was too *weak* — a resource
name, which §7.8.3 scopes to the dictionary defining it — and two different fonts collided. Here a
key is too *strong*, and one region misses itself 302 times. **Both are the same question: is the
key what the claim is about?**

## Decision

`DisplayList::add_clip` returns the identifier of an identical region if the table already holds
one. The comparison is exact — same path, same transform, same fill rule, same parent — so nothing
decides that two regions are "close enough"; a producer whose rectangles differ in the last decimal
place gets two identifiers and gets what it asked for.

It is keyed by a hash of the clip's content with the identifiers that hashed to it, rather than by
the clip itself, so no clipping path is stored twice. A collision costs one `PartialEq`, which is
what decides the answer.

**In `pdf-render` rather than in either backend**, for trap 2's reason: a decision either backend
could take alone is a decision neither has made. Both consume this list, and Vello pushes a layer
per clip exactly as `tiny-skia` builds a mask per clip.

## What it is worth

`callgrind_rasterise`, twenty renders, A/B in one sitting:

| page | before | after | |
|---|---|---|---|
| ISO 32000-2 p. 5 | 15 244.6 M | **3 168.1 M** | 4.81× |
| ISO 32000-2 p. 6 | 16 771.5 M | **3 601.3 M** | 4.66× |
| ISO 32000-2 p. 101 | 14 405.7 M | **4 990.6 M** | 2.89× |

More than the 18.3% the `calloc` line predicted, because building a mask is a fill and *n* − 1
intersects on top of the zeroing, and all of it went.

The cost is on the interpretation side, where every clip is now hashed:
**2 150.77 M → 2 177.01 M, +1.22%**, measured by `callgrind_interpret` against this commit's
parent in the same sitting. Per page of the benchmark that is +0.5 M against −470 M.

**The gates are unmoved and that is the point**: the oracle's 1794 verdicts are identical, the
corpus's counts are identical, the text gate is 97.9%. Nothing about this changes a pixel, which
is exactly what ADR 0131 could not say about the optimisation it refused.

## The thing it broke, which is the more interesting half

`render-gpu/tests/real_pages.rs` failed — not on a pixel, on its own guard:

> no render needed banding — the scene that overflows the device's buffers is no longer reaching
> it, so this test is no longer testing what it was written for

**ADR 0127's cliff on page 6 was substantially our own duplicate clips.** Page 6 at 1132×1600 was
the page a person reported as black, the page that motivated `render_checked`, the banding, and a
whole ADR. With one clip instead of 303 it no longer overflows Vello's buffers at 1.9008 — or at
5.0, which is 2980×4210 pixels, measured.

Three things follow, and none of them is "remove the banding":

1. **Vello's constants are still fixed and a scene can still exceed them.** What is gone is a real
   page in this tree that does, and a test asserting a fact about a dependency needs a witness. The
   replacement is `a_scene_too_large_for_one_pass_is_banded`: page 6's own fills with a clip each,
   nudged so that no two are equal — which is what a producer whose per-run rectangle differs in
   the last decimal place actually emits, and which deduplication cannot collapse. It bands in
   eight passes and draws within 1% of the processor's ink.
2. **The guard is why this was noticed at all.** A test that stops testing what it was written for
   is a silent pass in every project that does not assert the mechanism as well as the result. This
   one asserted it, and failed loudly on an improvement.
3. **ADR 0127's diagnosis stands and its scale does not.** "A page of small text at a laptop's
   resolution can exceed Vello's buffers" was true of *this tree's* page 6, and part of why it was
   true is that this tree handed the device 303 layers for one region.

## The lesson

**A cache that reports a perfect hit rate can still be missing.** Every one of the 303 lookups was
answered, so the cache's own instrument said it was working; what nobody had counted was how many
*questions* were being asked, and the answer was 303 for a page with one clipping region.
**Instrument the count of distinct keys, not the hit rate** — a hit rate is a statement about the
lookups you made, not about the ones you should have made.

# A clip chain as one crop and one intersect

Status: priced, blocked on a measurement.
Priority: 40
Corpus: 1 document (the worst page in the corpus, by a wide margin)
Code: `crates/render-cpu/src/lib.rs`, `MaskCache::build`

`bug1721218_reduced.pdf` is the corpus's worst page: 144.05 G instructions → 54.05 G when a ramp
stopped carrying 256 stops for a linear function (ADR 0068) → **43.13 G** when the built shading
was cached per object (ADR 0069), re-measured unchanged in session 113. What is left, in order:

```text
tiny_skia::pipeline::lowp::gradient  36.6%
Mask::intersect_path                  8.1%
build_soft_mask                       8.0%
fill_path_impl                        6.4%
calloc                                4.5%
```

**The two mask lines are one item**: `MaskCache::get` is 24.34% of the page over 3608 chains, with
no eviction and no duplication worth removing (ADR 0103 — the obvious savings were counted and
refused).

The shortcut nobody has taken is written in `MaskCache::build`'s own comment: **a child's band is
inside its parent's**, so a chain could be one crop and one intersect instead of a fill and three.

**It starts with a measurement, not with code.** It needs the intermediate clips cached, and the
page is already at 87% of `MASK_BUDGET` — so the first question is what those intermediates cost
in memory, and whether the budget has room for them. A change that saves 20% of one page and
trips the budget on another is not a win.

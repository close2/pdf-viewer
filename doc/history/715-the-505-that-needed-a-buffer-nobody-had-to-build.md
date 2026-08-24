# 715 — The 505 that needed a buffer nobody had to build

`doc/todo/11` item 7's remainder — the fills whose rectangles share a device pixel — is paid. 711
priced it at "one coverage buffer per mark with the portions' areas summed into it and the paint
blitted once, which is `scan::intersected`'s shape already", and **the price was right down to the
buffer**: nothing new was allocated, nothing of item 5's conflation-free rasteriser was needed, and
what had to change was one sentence inside `intersected` — it declined wherever there was no
clipping region to intersect, and §11.6.2 asks for the same buffer with no clip at all.

The one thing 711 had not priced is the **rounding**. Adding each portion's rounded level is a level
or two out per shared pixel, and a coverage rounded away is the whole subject of item 7, so the
summing is two passes: each portion at its own area with ADR 0476's interior run kept, then the
pixels two footprints have in common revisited and the total written there in one addition.

Date: 2026-08-24.
ADR: [0590](../adr/0590-portions-in-one-pixel-are-one-blit.md).
ADR numbers 0591 and 0592 were allocated to this round and not used.

Touched: `crates/render-cpu/src/scan.rs` (`Exact::Shared`, `Clip`'s scratch in every variant,
`Clip::factors`, `intersected`, `mask_fill`, `mask_shared_rectangles`, `mask_summed_pixels`,
`area_of`, three unit tests), `crates/render-cpu/src/lib.rs` (`rectangular_mark`,
`MaskCache::effective`), `crates/render-cpu/tests/edge_coverage.rs` (two new scenes, one comment
corrected), `crates/pdf-model/tests/oracle.rs` (`AMBIGUOUS_DENSE_TEXT_AT_PAPER_SIZE`'s figures for
`issue14297.pdf`), `doc/conformance/ledger.toml` (§10.7.4, §11.6.2), `doc/todo/11`,
`doc/todo/_scan-conversion.md`, the ADR and this file.

## The order of the round

**The census first, before the code** (trap 14), and it reproduces 711's exactly: 223 545 fills,
12 987 one rectangle, 3419 several with no shared pixel, **505 several sharing one**, 3084 declined,
over the pdf.js corpus's 958 readable first pages at scale 1. It also gave the thing 711's line did
not: the 505 lie on **23** pages, and the widest such path states 2 rectangles on nineteen of them.

**Then 711's claim was checked rather than inherited.** It said the construction is
`scan::intersected`'s shape and explicitly not item 5's rasteriser. Both halves hold, and the reason
is one line of `pdf_render::edge`: `device_rectangles` already declines an *overlap*, so the
portions' interiors are pairwise disjoint and the area of their union in a pixel is the plain sum of
their areas. A sum needs no fragment list and no sub-pixel geometry — which is exactly what
separates this from item 5, where the two marks are different objects and their overlap is real.

**Then the plant** (trap 13). Two new scenes in `edge_coverage.rs` were run against the unfixed tree
and failed by 0.050 and 0.142 of a pixel's coverage — 13 and 36 levels of 255 — against a tolerance
of 2. The third scene, `two_portions_sharing_a_pixel_are_not_composited_with_one_another`, passes
both ways on purpose and is the discriminator between the two correct constructions.

**Then the A/B, built from the sources** — 711's own lesson about `if true { return }` — using a
scratch copy of the three edited files rather than `git stash`, three neighbours being on the same
repository.

## Measurement

Machine load was 1.0 over 24 cores for the pixel arms and the first callgrind arm, and rose to 71
under a neighbour's gates for the second, so **no timing figure was taken**. Every number is a
raster value or an instruction count, neither of which load moves.

**The confinement is the strongest number here.** `raster_digest` over the pdf.js corpus moves **22
of 958 first pages**, and every one of them is a page the census had named before a line was
written. No other page moves at all. The single named page that does not move is `issue12963.pdf`,
whose three such fills land where the quarter and the exact area agree.

Looked at rather than counted (trap 1): `issue8187.pdf` is a **barcode**, and its bars come out at
the levels their own sub-pixel widths imply instead of quantised to a quarter. `issue840.pdf` and
`issue11913.pdf` are ordinary pages of text and artwork and are indistinguishable at a glance —
their differences are edge levels, and a share of the moving marks is text, because a glyph whose
outline is two axis-aligned rectangles can perfectly well have two of them in one pixel.

`callgrind_rasterise`, `RAYON_NUM_THREADS=1`, twenty rasterisations, before → after:

```text
  ISO 32000-2 p101 (text, none of the population)   5,414,365,181 -> 5,417,771,847   +0.063%
  issue8187.pdf  p1  (6 of 14 fills)                   45,230,325 ->    30,640,987  -32.256%
  issue11913.pdf p1  (96)                           5,387,032,288 -> 5,372,623,208   -0.267%
  160F-2019.pdf  p1  (45)                           3,290,771,464 -> 3,303,278,333   +0.380%
  issue840.pdf   p1  (97)                           5,405,390,201 -> 5,456,728,448   +0.950%
```

The +0.95% is the buffer's clear, compose and blit over each of 97 fills' own reach — ADR 0355's
cost on the pages that ask for it — and it is recorded rather than optimised, because the page
carrying it is one of twenty-two and the alternative is a coverage that is wrong.

## What the gates said, measured both ways

- **The reference oracle: not one verdict moved.** 983 agrees, 65 contradicted, 832 ambiguous, 3 our
  geometry, 2 reference geometry, 42 not comparable, 18 no render — before *and* after, both arms a
  full run rather than a number quoted from a report. **21 of its 966 per-page lines moved**, every
  one of them `ambiguous` and none of them across a bound; the largest is `issue8187.pdf`, mean
  18.89 → **18.73**, worst tile 40.90 → 40.37, ssim 0.7901 → **0.7921**, toward the references. A
  100% reference-cache hit rate, so no reference renderer ran during either arm.
- **The cross-backend gate did not move**: 933 agree, 22 differ, 2 refused, 17 not comparable both
  ways, with the same names. Two of the differing lines moved toward the device —
  `issue12295.pdf` ssim 0.88272 → 0.88273 and `pr12564.pdf` mean 1.4403 → **1.4392** — which is the
  only direction available to a change that makes the processor's rectangle exact.
- **`doc/todo/00` step 7's ink sweep, both ways over all 768 ambiguous pages.** 25 rows moved, 20 up
  and 5 down, by at most **0.062** of 255. The negative tail is the same 19 names in the same order:
  head `issue12418_reduced.pdf` −19.447, `issue4722.pdf` −13.810, `issue15977_reduced.pdf` −12.927,
  `bug1050040.pdf` −11.272, `issue5801.pdf` −8.991 — ADR 0433's five, reproduced to the thousandth,
  which is what says the recipe was re-implemented rather than approximated. The only two tail
  entries that moved went *up*: `issue12295.pdf` −2.363 → −2.362 and `issue14297.pdf` −1.136 →
  −1.135, both of them pages of this round's own population.
- The rest of §2 green on a machine at load 6 to 10 of 24 cores: `fmt` and `clippy` silent under
  `RUSTFLAGS="-D warnings"`, 2553 workspace tests, the doctests, `fuzz/`'s `check`, the corpus gate,
  both censuses, `text_extraction`, `dates`, `xmp`, `jpeg2000`, `fixed_documents` (40 checked, 0
  absent) and `conformance`.

**And one sweep earned its keep.** `cargo run --release -p conformance --bin quoted` compared the
oracle's own printed figures against what its page-list notes quote, run against *both* arms, and
found exactly one figure this round made stale: `AMBIGUOUS_DENSE_TEXT_AT_PAPER_SIZE` quoting a mean
of 4.34 for `issue14297.pdf`, which now reads 4.33. Opening it found the rest — its ink ladder had
drifted 0.01 to 0.03 across the rounds since the three-hundred-and-eighty-third with nothing
pointing at it, which is trap 1's third shape in its purest form. Our column was re-measured at
three resolutions and the paragraph's whole argument survives: ours still rises where both
references fall, and at eight times it still sits between them. With the note corrected the sweep is
byte-identical to the before arm, and `overtaken` went 45 to 44 because the note now cites the
decision that moved it.

## Two things worth keeping that are not in the ADR's argument

**An optimisation that looks free was measured and removed.** `scan::fill` is the call every glyph
on a page of text makes, and it now reaches `intersected` unconditionally where it used to guard on
`clip.composable().is_some()`. That guard is *exactly* the callee's own first two declines for an
`Exact::Unknown` mark, so restoring it is free and looks like an obvious saving of a call frame per
fill. Measured, it **cost** 13 000 instructions of 5.4 × 10⁹ rather than saving any. The +0.063% on
that page is codegen, not the branch. The general shape: a guard that duplicates a callee's first
decline is a claim about a cost, and a claim about a cost is measured.

**A variant said what a flag could not.** `Exact::Several` and `Exact::Shared` hold the same `Vec`
and differ only in whether two rectangles fall in one pixel — but they admit *different
constructions*, and the difference is not a tuning knob: the first may be drawn one rectangle at a
time through `fill_rect` and the second may not be drawn that way at all. As a boolean on one
variant, the loop is one forgotten condition away; as a variant, the fallback where `intersected`
declines is written once and is the single supersampled conversion rather than the loop. The same
argument put the scratch buffer into `Clip::Unclipped`: a buffer the *mark* needs cannot belong to
the clip's presence, and the type is where that is said.

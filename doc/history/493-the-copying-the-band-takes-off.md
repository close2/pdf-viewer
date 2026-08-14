# 493 — The copying the band takes off

**Finding.** `doc/todo/40`'s remaining measured cost — the whole-surface copying in
`render-cpu`'s mask and group path — came off byte-identically, because the item splits
along a line the file had already drawn without naming it: the backdrop *copy*, the soft
mask's *conversion* and its *storage* are not drawing arithmetic, so ADR 0219's binade
departure cannot reach them, while the drawing buffers stay surface-sized and the chain
item stays open. `Built` now carries §11.6.5.1's outside constant beside the band, which
is exactly the change to what a `MaskCache` entry is that the todo file predicted.

**Date.** 2026-08-14.
**ADR.** [0328](../adr/0328-a-backdrop-copied-to-its-band-and-a-soft-mask-stored-to-its-marks.md).
**Touched.** `crates/render-cpu/src/lib.rs` (the whole change: `initial_backdrop`,
`build_soft_mask`, `marked_rows`/`vertical_extent`, `Built::outside`,
`MaskCache::{admit_soft_mask, expand_soft_mask, combine}`, five new unit tests),
`doc/adr/0328-*` (new), `doc/todo/40-mask-chain-crop.md` (rewritten: the copying half
recorded taken, the chain half kept with its ADR 0219 question),
`doc/todo/README.md` (item 40's line), `doc/performance.md` (two claims that stopped
being true), this file.

## The measurement

Callgrind instruction counts, two arms built `--release` in one sitting — the base arm
from the round's base commit `ffc587ef`, the new arm from this change — because the
machine ran ten parallel rounds and a wall clock is not evidence. Invocations:

```sh
valgrind --tool=callgrind --callgrind-out-file=/dev/null \
  target/release/examples/callgrind_rasterise doc/pdf.js/test/pdfs/bug1721218_reduced.pdf 1
valgrind --tool=callgrind --callgrind-out-file=/dev/null \
  target/release/examples/callgrind_rasterise            # ISO 32000-2 p101
valgrind --tool=callgrind --callgrind-out-file=/dev/null \
  target/release/examples/callgrind_rasterise doc/ISO_32000-2_sponsored_EC3.pdf 6
valgrind --tool=callgrind --callgrind-out-file=/dev/null \
  target/release/examples/open_one corpus-cache/safedocs/cc-main-2021-31/0423/0423548.pdf
valgrind --tool=callgrind --callgrind-out-file=/dev/null \
  target/release/examples/open_one corpus-cache/safedocs/cc-main-2021-31/6081/6081357.pdf
/usr/bin/time -v target/release/examples/open_one <each of the two>   # peak RSS
```

The numbers are in the ADR's table. The headline pair: `open_one` on `0423548.pdf`
**149 475 924 050 → 76 481 113 199** instructions (−48.8%), on `6081357.pdf`
**81 900 867 207 → 17 023 010 209** (−79.2%); the corpus's worst page
(`bug1721218_reduced.pdf`, 20 renders) 13 826 922 716 → 13 670 680 697 (−1.1%); the
specification's pages 101 and 6 at −0.013% and +0.070%, the level of repeat noise, which
is the regression check. Peak RSS moved inside its spread on both documents (medians
1 143 068 → 1 130 540 kB and 208 788 → 203 460 kB over three samples an arm) — the mask
cache is bounded by `MASK_BUDGET` either way, so the claim is instructions, not bytes.

## Byte-identity, directly

Twenty mask/group corpus documents rendered through `open_one` with both arms at scale
1.0 and 2.0, plus the two web-crawl documents at 1.0 — 42 `cmp` comparisons, every one
identical. The list is in the ADR.

## The gates

All of `doc/todo/02` §2, run on this tree after the final edit; every ratcheted list is
held to equality by its own test, so a pass is identity, and every one passed:

```
fmt         clean (exit 0)
clippy      silent (the only warnings are the documented cold-build gcc lines from
            cxx-qt's generated bridge, and cargo's proc-macro-error2 future-incompat note)
nextest     Summary [ 234.065s] 1767 tests run: 1767 passed (3 slow), 11 skipped
doctests    ok
corpus      974 documents in 9.5s: 0 unopenable, 8 locked, 2 encrypted beyond us,
            6 pageless, 65 incomplete, 0 slow
oracle      agrees 906 / contradicted 67 / ambiguous 786, our geometry 1,
            reference geometry 2, not comparable 13, no render 19;
            undiagnosed-ambiguous list empty; 6163 cached reference renders,
            15 produced (99.8% hit rate)
text        overall 99.3% (24010/24189 words), 22 below 90% — both tests ok
dates       ok        xmp  ok        jpeg2000  ok
quorra      956 pages compared in 59.8s: 918 agree, 37 differ, 1 refused,
            18 not comparable
conformance 5 passed (citations, table references, quotations)
```

## What the next round should know

- **The chain item in `doc/todo/40` is narrower now and its blocker is unchanged**: ADR
  0219's supersample. This round's split — band what is outside the drawing arithmetic,
  decline what is inside — is the shape to reuse, not a licence for the departure.
- **`marked_rows` is a superset by the same contract `misses_surface` trusts**
  (`Command::device_bounds` + one row of margin). If `device_bounds` is ever tightened
  below "everything a command can mark", both break together.
- The ledger was not touched: the clauses this change cites (§11.4.4, §11.4.5,
  §11.6.5.1, §8.5.4) were all non-`unreviewed` already and no behaviour moved.

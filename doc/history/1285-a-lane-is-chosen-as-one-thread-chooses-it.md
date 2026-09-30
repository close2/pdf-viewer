# 1285 — A lane is chosen as one thread chooses it, and a tight bend keeps its tiling

Batch forty-four, raster slot. No ledger row moved (the contract named none; §8.4.3.2 and §10.7.4
were `implemented` and stay so). ADR 1407; feedback section 57 appended. `crates/render-cpu/` was
not opened.

## Thread count (ADR 1407 section 1)
- Premise corrected: the 10 levels are ADR 0009's quantised phase against a tile's own transform,
  not the hybrid. The room probe read shelves that queued inserts had not filled, so many threads
  admitted what one thread refused. Hybrid vs scratch lane: one level, 22 bytes, on that page.
- `atlas::PendingInserts` and `probe_settled`: the probe is asked where no queued insert can change
  it, else `prospect_for` drains. 551 drains over the corpus; 12 atlas-pressure pages 366 vs 333 ms.
  Carrying the fill to its commit (`Job::follows`) measured and dropped: 123 536 unsettled, 22×.
- Instrument: the corpus gate draws each page again at one encode thread (`OneThread`), held empty.
  Before the fix 53 pages differed on this tree (1286's repeat-key change exposed 40 more); after, 0.
- `compute_lane.rs`: glyphs off the identity put a pixel on a tie; 6 (RADV) / 2 (llvmpipe) pixels,
  one level; held to one level. The mosaic cannot see it.

## Tiling (section 2) and weight (section 3)
- Tiling off: fixtures pass but the pieces test; hook and tight L to the byte. Corpus both ways: 19
  pages past 1/16, all darker; `bug1743245` 205/205 tight strokes spend the sweep, up to 184 levels.
  Kept; comment in `stroke.rs`.
- `STROKE_SEGMENT_WEIGHT` 24 → 21: glyph 0.1375/0.1378, stroke 3.011/3.014 µs a segment, pinned.

## Files
`raster/crates/raster-gpu/src/atlas.rs`, `encode.rs` (field), `encode/fill.rs` (glyph-site insert,
hybrid comment), `encode/parallel/commit.rs` (`prospect_for`, `probe_unsettled`,
`note_atlas_insert`, drain reset), `encode/parallel.rs` (the constant), `raster/stroke.rs` (comment),
`tests/compute_lane.rs`, `crates/render-raster/tests/corpus.rs`, `doc/QUORRA_FEEDBACK.md`, ADR 1407.

## Gates
Corpus 1× 959/2, thread count 0, exit 0; 4× cpu 958/0 (176 thread-dependent, a survey), gpu 957/0,
compute 958/0, each 0 differing by thread count. Left: the 4× CPU residue, carried page to page by
the retained atlas (ADR 1407 section 4); `winds_two_values_as`'s shared `OnceLock` (`resources.rs`).

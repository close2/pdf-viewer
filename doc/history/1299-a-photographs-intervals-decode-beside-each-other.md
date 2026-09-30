# 1299 — A photograph's restart intervals decode beside each other, and its reduction by rows

Date: 2026-09-30. Branch `batch-1297-1302`, shared worktree. ADR:
[1433](../adr/1433-a-photographs-restart-intervals-are-decoded-beside-each-other-and-its-reduction-is-divided-by-rows.md).

## What was asked, and what the measurement said instead

The performance slot: a page of one photograph turned in 126 ms. The brief read its `transfer`
as 37 MB crossing at 0.7 GB/s. It is 20 MB crossing in 2 ms; the other 51 ms were raster's
area-averaging reduction on one host thread. The photograph is 5 280 × 3 792 with a `DRI` of one
MCU row, and 11% of its interpretation was `Image::is_opaque` asking twenty million alphas one at
a time. `images.pdf`'s 132 ms was never a codec: this tree's per-sample unpacking and soft-mask
combination were 94% of its instructions.

## What was built

- Restart bands (`pdf-model/src/image/restart.rs`): a baseline frame whose intervals begin on MCU
  rows is cut into codestreams of its own and decoded on the pool, each band overlapping one row
  either side so the decoder's upsampling sees the neighbours it sees whole.
- `area_averaged` on the device's threads; `is_opaque` and `premultiplied` test 64 alphas at once.
- Eight-bit grey and RGB unpacked by table; the soft-mask combination asks each column's source
  once and walks a same-grid pair in step.

Photograph turn 126.7 → 49.3–52.2 ms pinned (unpinned 129–205 → 40–46); `images.pdf` 156 → 51;
the plan 128–132 → 93–94, its zoom step 49 → 23. Every pixel the same, by construction and by
the fixtures, the display-list digests and the gates.

## What was tried and not kept

A staging buffer filled by several threads instead of `write_texture`: `wgpu` zero-fills a
mapped buffer (or stages it twice without `MAP_WRITE`), and the zoom step got slower.

## Worth keeping

A column the instrument names is a phase's clock, not the thing its name says: "transfer" was a
host reduction. Time each step inside a phase before choosing a lever for it.

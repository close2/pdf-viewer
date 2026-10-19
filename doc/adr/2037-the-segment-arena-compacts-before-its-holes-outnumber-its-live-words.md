# 2037 — The segment arena compacts before its holes outnumber its live words

Session 1600, slot 2 of batch ninety-five, the residue round 1597 found and did not own
(`doc/history/1597`, "Found"). Status: **accepted** and **built** in `raster-gpu` (`compute.rs`'s
`SegmentArena::forget` and `compact`, the public `ArenaBytes`, `Device::segment_arena_bytes`) and
`render-raster` (`QuorraRasterizer::segment_arena_bytes`, `examples/zoom_frame.rs`'s
`ZOOM_FRAME_ARENA`). Context: ADRs 0075, 0081, 2030. Test: `raster-gpu`'s
`tests/arena_compaction.rs`. Cites no clause and moves no ledger row.

## 1. What was unpaid

The compute lane keeps every outline it has drawn in one device buffer, uploaded once (ADR 0081).
A released outline's range leaves the arena's map — the id may be reissued, and a stale range would
be another outline's geometry — but its words stayed until the device went. The live words are paid
for already: each is an outline's segments, which the resource budget charged at upload and gives
back at release. **The holes were paid for by nothing**, and ADR 2030's residency budget is the wrong
place for them: it charges what a *retained encode* keeps, and a refused charge there has an answer
(send the streams every frame) where a refused hole has none but the one taken here.

## 2. The bound, and why compaction rather than a charge

**The release that would leave more hole words than live words compacts the arena**: the live
ranges are copied down on the device in their offset order — one copy command for each run of
ranges that were adjacent before — into a buffer sized for them alone (the next power of two of the
live bytes, at least the starting 1 MiB), and every entry is pointed at its new offset. Stated as
the type's comment says it: after every release the holes are at most the live bytes, so the buffer
is at most four times its live bytes, or its starting size; and since a compaction empties the holes
and the next one waits until they outnumber what is live, the words a compaction copies are never
more than the words released since the one before it.

It runs inside `Device::release`, where the device and its queue are, and is submitted before any
later frame writes its records, so every record names an offset in the buffer it binds; a frame
already submitted keeps the old buffer until it ends, by `wgpu`'s own rule for a buffer dropped in
use. A charge against a budget was the alternative the brief offered, and it was not taken: a charge
can only refuse, and what it would refuse is a release.

## 3. What it measured

**The witness across twenty notches** (a 900 × 1100 window, the compute lane, the fit 0.594 times
1, 1.25, 1.5, 1.75, 2, 2.5, 3, 3.5, 4, 3, 2.5, 2, 1.5, 1.25, 1, 0.8, 0.6, 0.8, 1, 1.25, the 890M):
the arena holds 53 348 live bytes through the first fifteen and 53 864 by the twentieth, **no hole
and no compaction**, in its starting 1 MiB. So the brief's hypothesis held in the weaker form: a
zoom releases nothing, because the page's display list pins every outline and stroke expansion it
names (`render-raster`'s cache keeps an entry whose `Arc` another holder has), and the holes come from
what a session does besides zooming — a page turned away from, or the cache's eviction past half the
resource budget. `tests/arena_compaction.rs` is that case built: sixteen outlines kept and sixteen of
8 000 corners released between them, holes at most the live bytes after each release, four
compactions, the live bytes from 1 537 268 to 1 204 and the buffer from 2 MiB to 1 MiB, and the kept outlines drawing a fresh device's bytes from
their new offsets; with the entries left at their old offsets, or the bound read against all the
words used, it fails.

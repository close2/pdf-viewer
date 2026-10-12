# 1884 — A line's closed form is made ahead of the walk, and its instance is written where the walk meets it

Session 1524. Status: **accepted** and **built**. The encode half ADR 1878 §3 named for a zoom step
on the witness, for the project owner's performance target of 2026-10-10. Context: raster's ADRs
0023, 0054, 0097; this tree's ADRs 1409, 1877, 1878. Code: `raster/crates/raster-gpu/src/encode/
lines_ahead.rs` (new: `walk_lines_ahead`, the ring of blocks, `place_made`), `encode/line.rs`
(`LineMark::of`, `Unclipped`, `place_line`), `encode/instance.rs` (`LineInstance` as bytes,
`line_head`, `note_run`), `encode/parallel.rs` and `parallel/commit.rs` (a run of lines is a range
of the line stream), `encode.rs` (both walks through `walk_lines_ahead`). Held by raster's
`tests/encode_threads.rs`, two new tests. No ledger row moves; no pixel moves.

## 1. What the instruction count said

Callgrind over the encodes of `zoom_frame` on the witness (`Device::encode_scene`, `Cpu`, the fit and
1.25×, six encodes at HEAD `56e42ea5`): 735 instructions a command, of which a line's pure part —
`compose`, `pen`, `Pen::ink`, the clip's intersection — is about 450 and the rest is ordered work:
the replay record, `use_mask`, the queue, the commit that wrote each queued line's sixty bytes. The
witness's 4.4 million commands are lines in runs of 512 to 4 095 between single fills (3.06 million
lines) and in one run past 262 144 (1.33 million): a parallel unit cut at runs would miss two thirds
of them.

## 2. Decision

**What the closed form makes of a line before its clip is known is made on other threads, a block of
the walk ahead of it.** `Unclipped` — the instance's first eleven floats and the ink's device box, or
the stroke road — is a pure function of the command and the view, so up to three threads claim
blocks of 4 096 positions in order and the walk takes them in order; every other position, and every
line the closed form does not reach, is walked as before. The clip chain stays the walk's: it is
resolved in encounter order and remembered for the chain asked last, and the instance's last four
floats are the chain's rectangle met with the target's. Both the scene walk and the record replay
go through it. The ring is bounded by the encoder's `in_flight_limit`; a block no thread claimed is
made by the walk, and a block a thread stopped inside is walked as before.

**A line's instance is written where the walk meets it, whatever is queued.** Nothing but a line
writes the line stream, so its order is the walk's order; what a queued mark ahead of it decides is
where its draw falls among the plan's ops. So a run of lines behind a queued mark is a range of the
stream under one style and mask, and its commit notes the run as one batch (`note_run`), the batch
its lines noted one at a time would have made. The plan's box is a union and a plan is never left
with marks queued, so growing it at the walk grows it to the box the commit grew.

Floors and counts are measured, not chosen: below 16 384 line commands no thread starts; with makers
able to keep up (an exploring probe: the walk waited 0.1–0.4 ms over 1 077 blocks with two or more),
the third is the margin for a shared core.

## 3. What it moved

One clock hold, three exports each in its own target directory — HEAD, HEAD plus this ADR, and the
same plus ADR 1885's hint — `zoom_frame` on the witness at the fit (scale 0.594) then 1.25×, 1.5×, 2×
and the fit, three rounds' minima, two alternations, RADV on the Radeon 890M, load 1.3–1.6:

| frame | lane | HEAD | this ADR |
|---|---|---|---|
| a step's encode, 1.25× | `Cpu` | 224.6, 231.3 ms | **127.3, 130.0 ms** |
| a step's encode, 1.25× | `Compute` | 227.0, 227.8 ms | **118.3, 121.5 ms** |
| the walk (`encode: recording`), every frame | both | 186.1–192.4 ms | **81.7–95.6 ms** |
| a step, 1.25×, whole | `Cpu` | 381.3, 388.7 ms | **290.6, 295.9 ms** |
| the fit, first | `Cpu` | 610.7, 611.4 ms | **501.4, 512.8 ms** |

The encode's instructions on the walk's thread went from 3.2 G an encode to 1.23 G (callgrind, the
makers made to do every block). What is left on it is about 280 instructions a line: the bookkeeping
in order, the chain, the cull, the instance's sixty bytes and the plan.

## 4. Agreement

Byte for byte. `tests/encode_threads.rs`: a page of thirty thousand lines under no clip, a rectangle
clip and a curve clip, dots, culled and rotated lines and soft-masked ones, between queued fills drawn
over them and a group, is the same bytes and counters at 1, 2, 3, 7 and 64 threads on `Cpu` and
`Compute`; the same page without the replay's refusals is the same bytes record-replayed at a second
view. Planted back, each fails: a line's draw noted at the walk behind a queued fill, and the chain's
answer kept for the wrong chain. The witness's frame digests at all five views on both lanes are
HEAD's; the six corpus arms against `/home/AI/arms-1522/` moved no page and left none one-sided
(walked with ADR 1885's hint in the tree too, which moved no digest of its own).

## 5. What is left

ADR 1877 §5: the instances in page space and the view a uniform `line.wgsl` applies, so that a step
stops writing 264 MB of instances it wrote the view before — the encode is now under half of a
step, and the transfer, the execute and the readback are the rest. What this costs: a module of
pipeline (a ring, a lock and two guards) beside the walk, and an encoder whose line stream runs ahead
of its plan.

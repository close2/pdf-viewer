# 1992 — A zoom step is measured by the stream each byte crosses in, and the page kept on the device is priced by its marks

Session 1578, a "pixels" evaluation round on the project owner's ask of 2026-10-19: "evaluate: keep
the page's intermediate representation resident on the GPU and have a compute stage re-rasterise it
at the new transform, so a zoom sends only a matrix." Status: **accepted** — the evaluation and its
instrument; the first step is ADR 1993, the ordered list `doc/todo/75`, the bar the owner is asked
to move `doc/questions/Q473`. Context: ADRs 0368, 0387, 1493, 1845, 1857, 1877; raster's ADRs 0079–0082,
0087, 0095, 0097. Instrument: `raster_gpu::UploadSplit` in `Counters::uploads`, read by
`render_raster::QuorraRasterizer::last_counters` and printed by `examples/zoom_frame.rs` as its
`crossed:` and `lanes:` lines. Cites no clause and moves no ledger row.

## 1. The premise, and the hypothesis

ADR 0368 priced the *tessellation* lane and ADR 1877 §5 priced lines in page space without building
it; nothing had divided `bytes_uploaded`, so a step's `transfer` had a total and no stream. **The
premise held**: ADR 0368 measured the processor's geometry, not a compute rasteriser. **The brief's
"eight rows" are eleven** in `doc/checks/turn-path.toml`; all eleven are read here. **The hypothesis
— host encode and transfer are a third to a half of a step, fills resident, lines, glyphs and images
deciding the rest — held for the witness at its upper end and missed the rows**: on the witness the
two are half the step and the line lane is almost all of both; on the rows the step is the host's
stroke tiling, a mesh's rows, an image's finer level, or a scene the step rebuilds, and lines are
nowhere.

## 2. What crosses on one zoom step today

`zoom_frame`, `/home/AI/witness/demo_heavy_engineering_A1.pdf` at the fit (0.594, 1417 × 1001) then
1.25×, one `--clock` hold (load 6.3 → 2.5), minimum of three rounds, two runs per lane, HEAD's export.
**Every device figure here is the owner's adapter**, the Radeon 890M through RADV, drawn headless —
`zoom_frame` and the turn gate open the real device without a window (ADR 0387), so none is Xvfb's
software adapter, and the owner's measurement loop, which is for what needs the display, was not
used (`doc/environment.md`).
The witness is 4 408 406 commands; `lanes:` puts **4 398 932 on the line lane** (99.8 %), 1 339
rectangles, 6 920 glyphs and 1 215 sheet marks under `Cpu`, the glyphs counted with the path lane's
under `Compute`, which fills them.

| the 1.25× step | `Cpu` coverage | `Compute` coverage |
|---|---|---|
| frame | 286.7, 301.3 ms | 286.6, 293.7 ms |
| host encode | 120.1, 136.7 ms | 114.0, 120.9 ms |
| transfer | 29.8, 28.4 ms | 30.0, 30.3 ms |
| device passes (`execute`) | 99.3, 99.6 ms | 106.9, 107.3 ms, of which the compute lane's count 0.3 and emit+deposit 1.0 |
| bytes | 277 523 378 | 304 700 013 |
| — line instances | 263 935 920 (95.1 %), 60 B a line | the same (86.6 %) |
| — coverage sheet | 12 903 138 (host-rasterised strokes and fills) | 15 091 345 (host strokes) |
| — quads, rects, atlas | 520 640, 42 848, 120 832 | 520 640, 42 848, 0 |
| — compute records | 0 | 553 600 (the 6 920 glyphs' fills × 80 B) |
| — compute arena | 0 | 0: every outline resident since the first frame |
| — compute device-only | 0 | 24 555 660 allocated on the device, never written by the host |

**Host encode and transfer are 50–55 % of the step** on both lanes. The step at 4× (an exploring run,
load 13–14) sends the same 264 MB of lines and a sheet of 131 MB (`Cpu`) or 152 MB (`Compute`): the
sheet grows with the area, the lines do not.

The eleven rows (`zoom_frame` at scale 1 and 1.25, both lanes, exploring; and the turn gate's own
`step` stages in the same hold, `Cpu`): EC3 p101 (glyphs) and issue19802 step in 1.3–1.8 ms, 16–21 %
host; issue12841, issue13931, images.pdf and 22060 cross their images' finer level — `paint` 7.9,
2.0, 1.7 and 4.2 MB at the step, transfer 1.4–6.7 ms, 41–89 %; issue14415, the Type 3 page and
bug1743245 spend 6.8–36 ms in host encode on strokes the line lane does not take, 83–97 %;
personwithdog's patch mesh is 4.7–5.1 ms of encode, 53 %; bug1721218's 84 ms step is a scene the
step rebuilds (125–136 ms of `scene` in `zoom_frame`), with 2 % of it encode and transfer.

## 3. What is resident, and what is not

- **Fills under `Compute`**: resident. Every outline is in `SegmentArena` from its first frame and the
  four passes re-flatten it under the step's transform; what crosses is one 80-byte `TileRecord` per
  fill, and the accumulators and sheet image are device allocations (the `device-only` row).
- **Glyphs**: under `Compute` they take that lane; under `Cpu` the atlas rasterises each new scale on
  the host and sends its rows.
- **Lines**: not resident. 60 bytes a line a step, computed on the host from device endpoints, and on
  the witness that is the step.
- **Strokes the line lane does not take, residue clips, soft masks**: expanded or rasterised on the
  host per view into the sheet; a residue clip or a soft mask also abandons raster's record replay
  (ADR 0087), so the walk is paid whole.
- **Images**: resident per reduction level, written where sampled (ADR 1493): a step into a finer
  level sends the texels it newly samples, and a step back sends none.
- **Shadings**: a ramp once; a mesh's rows per view in the encode.
- **A replay of the same view sent every instance again**, which is what ADR 1993 changes.

## 4. The design, priced per mark kind

Records in page space, the view a uniform every stage reads, the op list built once. Per kind, by
its share of the witness's step and of the rows (the saving is the stream it stops and the encode
that made it):

| kind | witness share | rows | what the design needs |
|---|---|---|---|
| lines | 95 % of bytes, the encode's walk | none | page-space instances, the view a uniform, §8.4.3.2/§10.7.5's width and the cull in the vertex stage (`doc/todo/75` item 2) |
| the op list | the 4.4 M-command walk | small | built once, the view-dependent remainder re-encoded (item 3) |
| §10.7.4's dots | 28 684 asked again, 13–40 ms (ADR 1877) | none | the substitution decided per view on the device (item 4) |
| fills (`Compute`) | records 0.55 MB | ≤ 0.23 MB | nothing: resident |
| host strokes | sheet 12.9–15.1 MB, grows with area | issue14415, Type 3, bug1743245: the step | a device stroker (item 6), not priced |
| images | none | four rows: the finer level | nothing owed: a level's texels are its own |
| meshes, rebuilt scenes | none | personwithdog, bug1721218 | the scene side's, not the device's |

**The floor is the replay**, and the hold measured it: the same view drawn again, with ADR 1993's
streams kept, is 127–141 ms on the witness (one run 171), against the step's 287–301 — what a step
that sends only a matrix would cost there, less what it still re-sends (the sheet and the records).

## 5. The bars

- **Byte-determinism across adapters (the oracle)**: the processor stays the oracle. A line mapped by
  a uniform in WGSL can differ from the host's mapping by an ulp, since WGSL section 15.7.5 lets a
  driver fuse; a kept frame equals a fresh one by construction, and the line lane is held to the
  processor by its bound already (raster's ADR 0082). `Q473` asks the owner to accept the line lane's
  endpoints moving into that class.
- **§10.7.4's no-disappearance**: kept by item 4, or by the host's ask ADR 1877 already makes.
- **A memory refusal that speaks**: item 5; ADR 1993 prices the residency it introduces.
- **Nothing compiled on the launch path**: item 2 changes `line.wgsl`'s vertex stage in place, so no
  pipeline is added; a variant would join the warm set behind the first frame (ADR 1831).

## 6. The ordered list

`doc/todo/75`: (1) a replayed encode sends no instance stream — built, ADR 1993; (2) line instances in
page space, the view a uniform; (3) the op list built once, the remainder re-encoded; (4) §10.7.4's
dot decided on the device; (5) a residency budget that refuses by name; (6) host strokes expanded on
the device; (7) fills, glyphs, images and ramps owe nothing.

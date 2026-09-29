# 1279 — A clipped mark and a stroke leave the walk's thread

Batch forty-three, performance slot. No ledger row moved: the contract named none. ADR 1395.

## Measured first (one build per commit, own target directories, `md5sum`-distinct)
- `frame_budget`, turn row budget/encode ms, before ADR 1341 → after ADR 1389: ISO p101 9.06/6.27
  → 10.31/7.47 (ADR 1389); `issue14415` 10.28/5.07 → 40.49/35.56 (ADR 1375 ×2.3, ADR 1389 ×1.7);
  `issue19802` 3.00/1.37 → 7.23/5.65 (ADR 1389); the Type 3 page 4.64/3.33 → 47.17/45.59
  (ADR 1375 ×8.7). Plans and `images.pdf` unmoved. ADRs 1341/1348/1359: no row (render-cpu's walk).
- Raster corpus, base vs this change, alternated: 1× 4.14/3.92 s vs 3.90/3.93 s; 4× 19.29/19.58 s
  vs 19.06/19.34 s — inside the runs' spread. 959/2/6 at 1×, 958/0/8 at 4×, both arms.
- Launch gate, clocks, calibration 0.702: 42 of 42 banded figures judged, none outside.

## The two levers (both in raster's encoder, both byte-neutral)
- A residue-clipped fill or stroke is made by the fan-out; the commit multiplies the clip in, and
  `residue_intersection` drains first. The Type 3 page: 72 of 85 strokes were clipped.
- A stroke's segment weighs 24 fill segments (corpus: 3.456 vs 0.139 µs), so a run of strokes
  reaches the floor. `issue14415`: every drain was under it.
- Turn, pinned: Type 3 46.73 → 16.92 ms; `issue14415` 39.92 → 18.93; the rest unmoved.
- Exactness: 957 corpus first pages hashed, identical at 1× (24 and 1 threads), 2×, 4×. New test
  fails with the commit's multiply planted away.

## Found, not fixed
- 13 corpus pages differ between 1 and 24 encode threads before this change: the atlas room
  probe routes a tile to ADR 0090's hybrid on one thread and to the scratch lane on many, and the
  two lanes differ by up to 10 levels on `issue1905.pdf`. With the probe off, all 957 agree.
- `doc/verify.md` names `PDFVIEWER_QUORRA_SCALE`; the corpus gate reads `PDFVIEWER_RASTER_SCALE`.
- A text page's turn is serial: repeated-key and rectangle drains, none past the floor.

## Gates (worktree, siblings' edits included)
launch_path (clocks) 0, 42/42 in band; raster_golden 0, moved 0; render-raster corpus 0;
headless_gpu 0 (39); conformance 0. raster-gpu nextest 432/433: `archetypes` moves with the
stroke edits beside it and passes on HEAD plus this change; clippy clean there, blocked in the
worktree by lints in `raster/fill/topology.rs`, `stroke/centre.rs`, `tests/curve_join.rs`.

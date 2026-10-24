# 1739 — The sample pattern is a derivation

Session 1739, slot 4 of batch 113, a pixels round; 2026-10-24. ADRs 2312, 2313; Q602 unused.

**Premise.** Held: ADR 2296 withheld its searched table for want of a derivation and of a band width
outside a lattice's closed form. **Hypothesis, held in part.** The two widths of any pattern with a
row and a column to each sample are a finite computation (primitive directions no longer than `n`;
`n²` corners), and ADR 0070's argument needs nothing of the lattice (ADR 2312). The searched pattern
is reproducible and ships (ADR 2313). But the glyph pages do not fall on every page: 29 of the 40
fall, 3 rise by at most 0.0022, and ADR 2296's table beat this one on the 4x arm's sum.

**Built.** `raster-gpu/src/winding/pattern.rs`: `SamplePattern` (`from_rows`, both widths, the
straight-edge error), the derived table `SEARCHED` at every admitted count, the lattice the fallback
and the bound. `examples/sample_pattern.rs`: the seeded annealing search, `--check` re-deriving the
table (28 s), added to CI's example list. The lane's offsets and ADR 0070's threshold are the
pattern's; at sixteen the threshold falls `1/√10` → `5/16`. Two defects planted in the widths
(the band's wrap gap dropped; a same-row sample not a pixel up) turn four and three tests red.

**Pixels.** Against HEAD's six arms: `gpu` 4x 315 digests moved, 166 pages down, 23 up (largest
+0.0022, `endchar`), sum 18.97 → 18.72; `gpu` 1x 96 moved, 57 down, 4 up (+0.0012), 51.16 → 51.12;
`cpu` and `compute` at both scales 0 moved. The 40 pages where HEAD's device lane leads its processor
lane most at 4x: 29 fall, 8 hold, 3 rise, sum 2.7786 → 2.6334; `issue20062` and `issue5501` fall.
Looked at: `endchar`, `issue20062` at 4x beside the oracle, right.

**Files.** New: `raster-gpu/src/winding/pattern.rs`, `examples/sample_pattern.rs`, ADRs 2312 and
2313, this record. Edited: `raster-gpu/src/{winding.rs, winding/lattice.rs, encode/thin.rs, lib.rs,
startup.rs, shaders/winding.wgsl}`, `tests/{thin_marks, coverage_lanes, mul_add_hazard}.rs`,
`examples/lane_placement/{main, sweeps}.rs`, `render-raster/examples/sampled_lane_column.rs`,
`.github/workflows/ci.yml` (one example name), `raster/doc/PLAN.md`, raster ADRs 0070 and 0076 and
ADR 2296 (status lines), §10.7.4's ledger note (two sentences).

**Gates.** Worktree: `rustfmt --check` per file 0; `cargo fmt --check -p raster-gpu`, `-p
render-raster` 0; `-D warnings clippy --all-targets -p raster-gpu` 0, `-p render-raster` 0; rustdoc
`-D warnings -p raster-gpu` 0; nextest `raster-gpu` 772 of 772, `render-raster` 110 of 110; the
search's `--check` 0; `cargo test -p conformance` 0, and `--test evidence --test adr_status
--test records` 0; `tools/state.sh docs` 0. My export (worked in `scratchpad/r1739/`; it took `startup.rs` whole and so carried a sibling's device-selection
hunks, which with no selection variable set choose as HEAD does — the adapter line is HEAD's): six
arms exit 0; `raster_golden` cannot move (`cargo tree`: no `raster-gpu` in `pdf-model`'s graph);
`t2-turn_path` under `--clock` 33 banded, 3 judged, 0 outside, load 11 to 35. Duration 10800 s,
about 150 tool uses.

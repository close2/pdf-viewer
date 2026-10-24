# 1731 — The device lane samples a rank-1 lattice

Session 1731, slot 4 of batch 112, a pixels round; 2026-10-23 to 24. ADRs 2296, 2297; Q594 unused.

**Premise.** Held: `winding::sample_offsets` was an ordered 4×4 grid, and §10.7.1's NOTE leaves the
pattern ours. **Hypothesis, not held twice.** ADR 0070's guarantee does not survive at `1/√n`: the
lattice is not a product, so its widest empty band (`1/√10`, along `(1, 3)`) and widest empty square
(`5/16`) replace one column spacing, and the threshold rises 0.25 → 0.3162. And pages do get worse:
the lattice's worst straight edge is 0.187 of a pixel against the ordered grid's 0.125.

**Built.** `raster-gpu/src/winding/lattice.rs`: `SampleLattice`, generator derived (3 at sixteen),
both widths in closed form and re-derived by brute force at every admitted count, two planted
defects red. `thin.rs`'s threshold is `vanishing_width`. Tests and the `lane_placement` check moved
from `1/√n` to `1/n`; `coverage_lanes`' straight-edge bound is derived per edge from the lattice.

**Pixels.** Against HEAD's six arms: `gpu` 4x 329 digests moved, 255 pages down, 52 up (largest
+0.0214), sum 23.09 → 18.97; `gpu` 1x 110 moved, 78 down, 24 up (+0.0186), 53.31 → 51.16; `cpu` and
`compute` at both scales 0 moved. Glyph pages at 4x: 39 of HEAD's 40 worst fall (`issue21068` 0.4903
→ 0.0795, `endchar` 0.5775 → 0.2194); knockout page-arms: 13 of 19 fall, 2 hold, 4 rise —
`issue20062` (one slanted triangle) at both scales, `issue17069` and `bug852992_reduced` at 1x by ≤
0.0003. Looked at: `knockout_groups_test`, `issue20062`. A searched 16-rooks table did better on
`gpu` 4x (30 up, sum 18.67) and is ADR 2296's first revisit.

**Knockout residual** (ADR 2297, deferred): a probe in my export over the `cpu` 1x arm — 340 twin
paths, 8 shaped, 2 opaque images, 0 twinless, 0 bare groups. Nothing built.

**Files.** New: `raster-gpu/src/winding/lattice.rs`, ADRs 2296 and 2297, this record. Edited:
`raster-gpu/src/{winding.rs, winding/buffers.rs, encode.rs, encode/coverage.rs, encode/thin.rs,
shaders/winding.wgsl}`, `lib.rs` and `startup.rs` (my hunks only), `tests/{thin_marks, coverage_lanes,
mul_add_hazard}.rs`, `examples/lane_placement/{main, sweeps, fixture, witness}.rs`,
`render-raster/examples/sampled_lane_column.rs`, `raster/doc/PLAN.md`, raster ADRs 0070 and 0076
(status line), §10.7.4's ledger note (two sentences).

**Gates.** Worktree: `rustfmt --check` per file 0; `cargo fmt --check -p raster-gpu`, `-p
render-raster` 0; `-D warnings clippy --all-targets` both 0; rustdoc `-D warnings -p raster-gpu` 0;
nextest `raster-gpu` 763 of 763, `render-raster` 109 of 109, `viewer-ui --test
chrome_over_a_magnified_page` 1 of 1; `lane_placement --check` 0; `cargo test -p conformance` 0
after ADR 2296 landed; `tools/state.sh docs` 0. My export (worked in `scratchpad/r1731/`): six arms
exit 0; `raster_golden` held 974, moved 0; `t2-turn_path` under `--clock` 33 judged, 0 outside, load
2.4 to 2.7. Duration 12900 s, about 190 tool uses.

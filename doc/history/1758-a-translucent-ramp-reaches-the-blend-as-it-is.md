# 1758 — A translucent ramp reaches the blend as it is

Session 1758, slot 7 of batch 115, a pixels round; 2026-10-26. ADRs 2350, 2351; Q621 unused.

**Ranking.** Against HEAD's six arms at the quantum off, as ADR 2286 ranks (`cpu` 1x sum 8.7481
against 49.8050 shipped): past three named pages, `knockout_groups_test` (0.1883 `gpu`, 0.1292
`cpu`) and `issue7821` (0.1296 on every lane). **Premise held** for (a): no arm page reaches
destination-over with an opaque source. **Hypothesis not held as stated**: one device fix, one
oracle finding, and the second device lever priced and deferred.

**Built.** `issue7821`'s command 0, an axial ramp at alpha 0.85, is 0.1305 alone: the shading lane
rounded the ramp's colour before its alpha and the coverage multiplied it (trap 257). `fs_main` now
takes the level of an opaque paint only (ADR 2350). Destination-over: a census at the scene builder
over the `cpu` arm found two fills, white at alpha 0.3; withholding the level, built and measured,
put 381 of 765 greys a level off on RADV over an empty backdrop, so the level is kept and its cost
bounded. New tests `raster-gpu/tests/a_translucent_ramp_is_rounded_once.rs` (red on HEAD, 408 of
1 020 off on RADV) and `an_opaque_paint_under_the_backdrop.rs` (red with the level withheld).

**For slot 2.** `knockout_groups_test`'s group 395 (isolated, not knockout) differs at 69 696
pixels, its opaque `(0.91, 0.85, 0.7)` background's blue: oracle 179, device 178, ADR 2311's rule
178, the oracle's own top-level 0.7 (`close-path-bug`) 178; 0.0446 of the page's 0.1292. **Deferred**
(ADR 2351): the half float nearest a non-opaque source on every solid lane, 26 rises of 306 moved.

**Pixels.** Against HEAD's six arms by page: 4 pages moved per arm, 24 page-lanes, 18 fell, 6 rose;
the device lane's sum of page means fell on every arm (`cpu` 1x 49.8050 to 49.7206, 4x 16.1914 to
16.1017). Falls: `issue7821` 0.3062 to 0.2456, `issue18956` 0.0241 to 0.0007, `issue17069`. The 6
rises are `alphatrans`, 0.0005 to 0.0009, not ties: its ramp alone falls 0.0131 to 0.0015 and the
page rises where the ramp lies over two groups and an image (ADR 2350 §5). Looked at: `issue7821`,
`alphatrans`. Device-lane goldens: none moved.

**Files.** New: ADRs 2350, 2351, this record, the two tests. Edited: `raster-gpu/src/shaders/
shading.wgsl`; comments in `shaders/function_lane.wgsl`, `src/device/ramp.rs`; `doc/state-of-play.md`.

**Gates.** Worktree: `rustfmt --check` per file 0; `cargo fmt --check -p raster-gpu` 0; `-D warnings
clippy -p raster-gpu --all-targets` 0; rustdoc `-D warnings -p raster-gpu` 0; nextest `raster-gpu`
777 of 777, `render-raster` 109 of 109; `cargo test -p conformance` 464 of 464; `tools/state.sh
docs` queued 1 265 s behind a sibling's arms and withdrawn before its grant. My export, as I worked in
`scratchpad/r1758/`: six arms, every arm exit 0; `t2-turn_path` under `--clock`, mine then HEAD,
load 1.99 to 1.71: 33 banded, 33 judged, 0 outside on each, mine over HEAD 0.964 to 1.040, median
0.994. Duration 10268 s.

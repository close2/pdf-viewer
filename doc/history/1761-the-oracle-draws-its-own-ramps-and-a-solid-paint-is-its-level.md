# 1761 — The oracle draws its own ramps, and an opaque solid paint is its level

Session 1761, slot 2 of batch 116, an oracle round; 2026-10-26. ADRs 2356, 2357; Q624 unused.

**Premise.** Held for the ramps: `bug852992_reduced` and `issue840` are axial ramps the oracle handed
`tiny-skia` as gradients. **Not held for the knockout page**: the group is not the cause. Hand-built
pages of one `0.91 0.85 0.70 rg … re f` drew blue 179 at top level and inside isolated and
non-isolated groups; only a knockout group drew 178, and `close-path-bug`'s `0.7 g` is 178 because it is
a curve. The split is the shape: a rectangle's whole pixels are the library's `memset` of
`to_color_u8` (`(c·255+½)` truncated), a path's go through its store (nearest-even) (ADR 2357 §1).
**Hypothesis.** Held: the five rises are falls, the knockout background is 178, nothing rose.

**Built.** `render_cpu::shading::ramp_shader`: an opaque axial or non-cone radial fill is a pattern of
cells on the page's grid, each the level nearest `colour_at(t)` at the pixel centre's `t`, drawn through
every converter `draw_fill` already has; `RampCells` halves to the stop, bit for bit the walk. A
translucent ramp keeps the gradient — the first arms drew it from cells and rose `issue18956`,
`issue7821`, `alphatrans`. `convert::solid` moves an opaque source-over paint's component by an ulp
where either conversion misses its level. New test file `a_level_where_the_paint_is_the_pixel.rs`:
its fill and ramp tests red against HEAD's tree, green after; unit tests for both.

**Pixels.** Against HEAD's six arms by page, every arm exit 0, no device digest moved: 0 rises, 104
falls. `cpu` sum of page means 49.7155 → 49.5936 at 1x, 16.1005 → 15.9886 at 4x. `bug852992_reduced`
0.0196 → 0.0176 on the three 1x arms; `issue840` 0.1144 → 0.1134 (`cpu`), 0.1150 → 0.1140 (`gpu`);
`knockout_groups_test` 0.1825 → 0.1233. Looked at: `bug852992_reduced`. `raster_golden` re-taken: 35
pages, raster digest only, listed in ADR 2356 §4. `alphatrans`'s tie is ADR 2367's, named in ADR 2356 §2.

**Files.** New: ADRs 2356, 2357, this record, `crates/render-cpu/tests/a_level_where_the_paint_is_the_pixel.rs`.
Edited: `crates/render-cpu/src/{shading,fill,lib,convert}.rs`, `crates/pdf-model/tests/raster_golden.tsv`,
`doc/conformance/ledger.toml` (§8.7.4.5.3 stays `implemented`, its lists name `render-cpu`'s ramp),
`doc/state-of-play.md` (one sentence). I worked in `scratchpad/r1761/`.

**Gates.** Worktree: `rustfmt --check` on my five Rust files 0; `cargo fmt --check -p render-cpu` 0;
`-D warnings clippy -p render-cpu --all-targets` 0; nextest `render-cpu` 258 of 258; rustdoc
`-D warnings -p render-cpu --document-private-items` 0; `cargo test -p conformance --no-fail-fast`
exit 101, 462 passed, 2 red, `records` on 1764's and 1765's records, not mine. Export: six arms exit 0
each; `raster_golden` (update) exit 0; `t2-turn_path` under one `--clock` hold, this then HEAD, load
3.29 and 3.99 at the starts, the render node's other holders unreadable to this user: 33 banded, 3
and 0 judged (device busy above 20%), 0 outside each, this over HEAD 0.942 to 1.108, median 1.012.
`tools/state.sh docs` queued 1 652 s behind siblings' walks and withdrawn before its grant. Duration 9100 s.

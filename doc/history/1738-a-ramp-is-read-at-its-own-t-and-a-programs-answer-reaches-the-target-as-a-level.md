# 1738 — A ramp is read at its own `t`, and a program's answer reaches the target as a level

Session 1738, slot 3 of batch 113, a pixels round; 2026-10-24. ADRs 2310, 2311; Q601 unused.

**Ranking.** HEAD's arms at 1x, quantum off (habit 179): four named pages lead, then the brief's
`issue14165`, 0.1320 on all three lanes, one digest per scale. Next unnamed: `issue7821`.

**Mark and clause.** One page-tall axial shading; per row against the ramp at the centre's own `t`
(§8.7.4.5.3, "the colour with which to paint the point"), the oracle is right on 841 of 842 rows and
raster wrong on 149, exactly a 4 096-texel table read at its nearest texel: **the lane's converter**.
The table is now the ramp's stops, interpolated at `t` (ADR 2310). RADV still drew 51 rows a level
low, a fragment's colour exported as a half float rounded toward zero, so the colour leaves as its
nearest level, exact through `fma`; both adapters then draw the page's ramp on every row.

**Second page.** `issue17679` (opaque fills, 0.0550) is that export on a solid paint, and
`function_based_shading` on the device program, whose answer is now rounded as the ramp's is. A
solid paint rounded was built, walked twice and taken back: it moves every antialiased edge, and 30
`cpu` page-lanes rose at 1x (ADR 2311 §3); the remedy left is the lanes' write, raster's.
**Premise** "the oracle's rounding or the lane's": the lane's. **Hypothesis** "two pages, one in the
oracle and one in a lane": not held — one page, in the lane; the second was taken back.

**Pixels.** Final tree against HEAD's six arms: 38 pages moved on each arm, 228 page-lanes; mean
error fell on 171, equal to four decimals on 52, rose on 5 — `issue18529` on three lanes at 1x,
0.0142 to 0.0172, pixels whose colour is exactly a half level (246.5), a tie the oracle's `f32`
breaks down and this rounding up, and `alphatrans` +0.0001 on two. **The bar is not met on those 5.**
The 1x equals by the probe's exact sums: 7 fell, 1 held, 2 rose (by 95 and 1 units of 255).

**Files.** New: ADRs 2310, 2311, this record, `raster-gpu/tests/a_steep_ramp_is_read_at_its_own_t.rs`.
Edited in `raster-gpu/src/`: `device/ramp.rs` and its `tests.rs`, `device/resident.rs`, `device.rs`,
`resources.rs`, `error/resource.rs`, `shaders/shading.wgsl` and `function_lane.wgsl`; §8.7.4.5.3's
ledger row; `doc/state-of-play.md`, `doc/crate-map.md`.

**Gates.** In my export (habit 138), as I worked in `scratchpad/r1738/`: `rustfmt --check` per file
0; `cargo fmt --check -p raster-gpu` 0; `-D warnings clippy -p raster-gpu --all-targets` 0; nextest
`raster-gpu` 761 of 761, `render-raster` 110 of 110; rustdoc `-D warnings -p raster-gpu` 0; new test
red on HEAD (128 of 256 rows). `raster_golden`: `pdf-model`'s graph holds no raster crate (`cargo
tree -i`, habit 119). Arms: four small walks, every arm exit 0. `t2-turn_path` under `--clock`, the
final tree then HEAD in one hold, load 5 to 2: 33 judged, 0 outside each, mine over HEAD 0.93 to 1.06.
Worktree: `cargo test -p conformance` 463 of 464, `records` failing on siblings' records mid-write
and passing on this one. Duration 10117 s.

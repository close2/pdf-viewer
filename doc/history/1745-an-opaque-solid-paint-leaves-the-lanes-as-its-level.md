# 1745 — An opaque solid paint leaves the lanes as its level

Session 1745, slot 1 of batch 114, a pixels round; 2026-10-25. ADRs 2324, 2325; Q608 unused.

**Premise.** Held: `issue17679` is three opaque rectangles, every differing pixel inside one, and
RADV's half-float export takes `0.3` below its level. **Hypothesis, not held as stated.** ADR 2311's
named remedy — round each solid lane's write after coverage — fixes the page and still moves edges:
`canvas` 0.0133 to 0.0135, `close-path-bug` 0.0060 to 0.0062, because over a backdrop rounding the
source and its alpha before the blend rounds twice (ADR 2324 §2). Quantising coverage first fitted
one oracle path and broke another (`issue6127` 0.0125 to 0.0153) and was not built.

**Built.** The rectangle, line and coverage-quad lanes write the level nearest an *opaque* source,
where the write is the pixel, and leave every other source to the blend (ADR 2324). A level probe
of 765 near-half greys then showed ADR 2311's `fma` residual lost on RADV (127 a level high), so
`nearest_level` is taken in integers in all five shaders that use it, the copies held by `copies.rs`
(ADR 2325). New test `raster-gpu/tests/an_opaque_paint_is_written_as_its_level.rs`: red on HEAD
(381 of 765 off on RADV), green on llvmpipe, RADV and radeonsi.

**Pixels.** Against HEAD's six arms by page: 129 page-lanes moved, mean fell on 72, equal to four
decimals on 46, rose on 11; the device lane's sum of page means fell on every arm (`cpu` 1x 50.0442
to 49.8095). The 11 rises are `function_based_shading` (six arms), `bug852992_reduced` (three),
`issue840` (two): each draws the same bytes before and after on llvmpipe, and on RADV the moved
pixels are, but for 138, 10 and 1, llvmpipe's — half-level `f32` ties the oracle breaks upward,
named in ADR 2324 §4. `issue18529`, record 1738's unmet rise, falls 0.0172 to 0.0117. The edge pages
ADR 2311 named move no pixel. Device-lane goldens: none moved, none re-taken.

**Files.** New: ADRs 2324, 2325, this record, the test above. Edited in `raster/crates/raster-gpu/
src/shaders/`: `rect.wgsl`, `coverage.wgsl`, `line.wgsl`, `shading.wgsl`, `function_lane.wgsl`,
`copies.rs`; `doc/state-of-play.md`, `doc/crate-map.md`.

**Gates.** In my export, as I worked in `scratchpad/r1745/`: `rustfmt --check` per file 0; `cargo fmt
--check -p raster-gpu` 0; `-D warnings clippy -p raster-gpu --all-targets` 0; nextest `raster-gpu`
774 of 774, `render-raster` with `raster-pages` 109 of 109. `raster_golden`: no raster crate in
`pdf-model`'s graph (`cargo tree -i`, habit 119). Arms: four small walks, every arm exit 0.
`t2-turn_path` under `--clock`, mine then HEAD in one hold, load 4.7 to 4.3: 33 banded, 24 and 21
judged, 0 outside; mine over HEAD 0.949 to 1.104 on the 18 judged in both. rustdoc `-D warnings -p
raster-gpu` 0; `tools/state.sh docs` queued 1 016 s behind a sibling's clock hold and withdrawn
before its grant. Worktree: `cargo test -p conformance` 463 of 464, `records` failing on a
sibling's record mid-write and passing on this one; `--test evidence --test adr_status` 0.
Duration 7633 s.

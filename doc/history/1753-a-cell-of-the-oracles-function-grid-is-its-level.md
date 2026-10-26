# 1753 — A cell of the oracle's function grid is the level nearest its value

Session 1753, slot 2 of batch 115, an oracle round; 2026-10-26. ADRs 2340, 2341; Q616 unused.

**Premise.** Not held: `scan::level_of` turns a coverage into a mask byte and is not on the path of
ADR 2324 §4's eleven rises. Taken exactly, it moved two bytes of `issue840` at 4x and one text pixel
of `issue5501` at 1x away from all three lanes (a rise of 0.0001 on two arms), because raster's own
accumulators round the same `f32` product (ADR 2340 §3). The oracle's colour bytes are taken in three
places: `tiny-skia`'s high-precision store for a solid paint and a ramp, and `to_color_u8` for a cell
of a type 1 shading's grid. **Hypothesis.** Held for one page of three: the grid is
`function_based_shading`'s six rises; `bug852992_reduced` and `issue840` are axial ramps, whose level
is the library's store (ADR 2341, deferred). Rounding a solid paint first was tried on the three
pages and moved 4 804 bytes of `bug852992_reduced` away from the device (trap 257), so not built.
The §10.7.4 residue is the quantisation itself, which an exact product does not lift.

**Built.** `render_cpu::shading::nearest_level`, ADR 2325 §2's `(⌊510 c⌋ + 1) / 2` taken in `f64`,
writes each cell of `sampled_shader`'s grid. Tests: a unit test over ADR 2325's 765 greys, and
`sampled_shading.rs`'s flat-producer test, red on HEAD (129 for 128.499 999 94 levels), green after.

**Pixels.** Against HEAD's six arms by page, every arm exit 0: no device digest moved; one page-lane
per arm moved, `function_based_shading` 0.0314 → 0.0263 at 1x and 0.0087 → 0.0075 at 4x, below the
pre-2325 0.0285 and 0.0080; nothing else rose or fell to four decimals. `cpu` arm's sum of page
means: HEAD 49.8050, this 49.7999 at 1x; 16.1914 and 16.1902 at 4x. The page rendered before and
after: one level at most, on two of its nine squares' tie rows. Rises left: five, ADR 2341's.

**Row.** §10.7.4 stays `departed`; its note's `scan::level_of` residue is stated as the half level a
partly covered pixel may fall below its area (ADR 2340). `doc/state-of-play.md`: one sentence.

**Files.** New: ADRs 2340, 2341, this record. Edited: `crates/render-cpu/src/shading.rs`,
`crates/render-cpu/tests/sampled_shading.rs`, `crates/pdf-model/tests/raster_golden.tsv` (2 lines),
`doc/conformance/ledger.toml` (the §10.7.4 note), `doc/state-of-play.md`. I worked in `scratchpad/r1753/`.

**Gates.** Worktree: `rustfmt --check` on my three Rust files 0; `cargo fmt --check -p render-cpu`
0; `-D warnings clippy -p render-cpu --all-targets` 0; nextest `render-cpu` 253 of 253; rustdoc
`-D warnings -p render-cpu` 0. Export: `raster_golden` (update) exit 0, 2 moved, raster only,
re-taken; six arms exit 0 each, 0 rises and 6 falls against HEAD's six arms; `t2-turn_path` under
one `--clock` hold, this then HEAD, load 1.34 and 2.13 at the starts: 33 banded, 33 judged, 0
outside each, this over HEAD 0.941 to 1.095, median 1.005. `cargo test -p conformance
--no-fail-fast` exit 101 on one red, `records` on 1758's 41-line record, not mine. Duration 9150 s.

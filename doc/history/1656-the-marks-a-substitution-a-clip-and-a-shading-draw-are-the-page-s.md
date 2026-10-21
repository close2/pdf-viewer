# 1656 — The marks a substitution, a clip and a shading draw are the page's

Slot 2 of batch one hundred and two, 2026-10-21, a pixels round on ADR 2133's last section. ADRs
2148, 2149; Q551 not used. No row moves: §10.7.4 stays `departed`, its note's last sentence rewritten.

**Premise held; the residual was the route the brief did not name.** On HEAD the owner's witness at
1.25⁸ parted from its page at 99 pixels (worst 1). Isolated by the smallest mark on the full-size
target (trap 213): a zero-length round-capped stroke 1.44 px wide — a dot past `thin_segment`'s two
by two pixels, ADR 2133 §5's sixth route. Hypothesis: four of five moved; the fifth (the library's
stroker) stays — held, and a sixth (dots) and a seventh (long mitres) moved with them.

**Built.** `split_collapsed_fill`'s marks and `sub_pixel_bands` (fill and stroke), the one-pixel rule
and its caps, the dots, the long mitres: decided under the page's matrix, filled by `fill_stated`.
Clip masks: `clip_region` and `rectangular_mark` on the page's grid, `scan::mask_fill` takes a
`Place`. Cones and washed shadings: evaluated at the page's pixel centres (`shading::OnSurface`),
the edge `fill_positioned`. A mesh's raster and a sampled wash keep the surface (ADR 2148 §3). Five
tests, a window against the page at non-dyadic edges, failed on HEAD and pass.

**The bound (ADR 2149).** The library's stroker keeps the surface's matrix. A probe found the larger
bound behind it: the library's 8191 range is asked of the surface, so a knockout element on a page
9000 px wide is aliased on the page and anti-aliased in its window — 620 px apart. Not fixed.

**Measured.** `window_against_whole`, one `--clock` hold: 1.25⁸ 99 → 0 px, window 191, 223 → 208,
203 ms; 1.25¹⁰ 0 → 0, digest unchanged, 271, 272 → 263, 262 ms. Golden census 60 → 29 pages, worst
513 → 383 px, 3 levels; `PAST_THE_BOUND` 11 → 1 (`blendmode.pdf`). 6 golden pages moved one level,
all looked at; the TSV is regenerated on them.

**Gates.** `rustfmt --check` on the 9 files, `cargo fmt --check` and `-D warnings clippy
--all-targets` for `render-cpu` and `pdf-model` 0; nextest `render-cpu` 232 passed; `cargo test -p
pdf-model --test raster_golden` 2 passed; `cargo test -p conformance` 452 passed, 0 failed;
`state.sh --round 1656 docs` 0. In the export: `raster_golden` 974 held after regeneration, census
passed, 1 named; `hostile_geometry` 4, `fixed_documents --ignored` 1 passed; six arms against
`/home/AI/arms-1655/`: 0 frame digests differ, the oracle's error moved on 5, 5, 4 pages at 1× (≤
0.0017), 0 at 4×, the six pages drawn in strips by both builds and looked at.

**Files.** `render-cpu/src/{scan,fill,stroke,shading,mask_cache}.rs`, `stroke/sub_pixel_rule.rs`,
`render-cpu/tests/window_on_the_page_grid.rs`, `pdf-model/tests/raster_golden.{rs,tsv}`, the
§10.7.4 note, `state-of-play.md`, ADRs 2148, 2149, this. **Unfinished**: ADR 2149 §3's range; a
mesh's raster wants a region from `pdf-render`'s `MeshRaster::build`.

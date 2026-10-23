# 1723 — A union decline is said, and the degenerate pieces are measured

Session 1723, slot 4 of batch 111, a pixels round; 2026-10-23. ADRs 2280, 2281; no `Q`.

**Premise.** Not held, twice. (1) A new `pdf_model::Unsupported` variant cannot carry the decline:
that report is the interpreter's, worded before any draw, in a crate above `render-cpu`; so the
decline is typed in `render-cpu` and returned where the draw returns, `CpuRasterizer::declined`
(ADR 2280), and no host says it yet. (2) The push pin does not state every piece twice — 11 of 38
contours restate one before them at 1x, 25 of 70 at 4x. Held: a retraced path is measurable, as
weights rather than by dropping twins, which an outline's counter-wound lobe would make wrong
(ADR 2281). Not held: the `inks.pdf` piece is no named departure — a thin rectangle whose upright
sides lie along its neighbours' and whose horizontal edges along nothing, entered from a vertical
ray (ADR 2281). Both built; neither is a departure.

**Drawn.** Each mark in a window beside the page drawn sixteen times finer by the library: the pin
at 1x mean 10.8 against the library's 15.4 levels, at 4x 1.04 against 7.46, the worst value the
needle's thinnest line in both; the `inks.pdf` mark 0.59 against 4.05, worst 21.9 against 29.9.

**Census.** Page one of the corpus's 974 documents at 1x and 4x through `declined`: no mark declined;
with both changes taken back it names exactly ADR 2268's three.

**Files.** New: ADRs 2280, 2281, this record. Edited: `render-cpu/src/{area.rs,area/union.rs,scan.rs,
mask_cache.rs,strips.rs,soft_mask.rs,lib.rs}`, `pdf-model/tests/raster_golden.tsv` (2 lines), the
§10.7.4 row's note, `doc/state-of-play.md`. I worked in `scratchpad/r1723/`.

**Gates.** Worktree: `rustfmt --check` per file 0; `cargo fmt --check -p render-cpu` 0; `-D warnings
clippy -p render-cpu --all-targets` 0; nextest 250 of 250; `tools/state.sh --round 1723 docs` 0;
`cargo test -p conformance` one failure, `records` on a sibling's record being written, and
`--test evidence --test adr_status --test records` 0 for this record. Export: `raster_golden` 972
held, 2 moved (raster only), re-taken 0; the six arms against HEAD's six arms, 0 of 5 799 digests
moved, oracle mean error down on all 9 page-lanes it moved in;
`fixed_documents` 5 and `hostile_geometry` 4 with `--include-ignored`, gates and debug profiles, 0;
`t2-turn_path` under `--clock` at load 4.5 to 6.9, exit 101, 21 of 33 inside, 9 not judged, 3 step
rows outside — the device's, the test names no `render-cpu` item. Each new test red on its fault.
Duration 9 400 s, about 175 tool uses.

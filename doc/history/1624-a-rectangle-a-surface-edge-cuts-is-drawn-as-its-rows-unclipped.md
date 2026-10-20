# 1624 — A rectangle a surface's edge cuts is drawn as its rows unclipped

Slot 2 of batch ninety-eight, 2026-10-20, a pixels round on ADR 2068 §5 and trap 216. ADR 2084;
ADR 2085 and Q519 not used. No ledger row moves: §10.7.4's row stays `departed`.

**Premise held; the hypothesis did not.** One rectangle cut at row 20: the page states row 19 at
192 (corners 120, 96), the cut 191 (119, 95), §10.7.4's area 191.25 — so the closed form at a cut is
neither value. The mechanism is the library's one-row branch for a rectangle clipped to one row
(`bottom − top − 1`, corners `>> 8`), not a rounding of the clipped edge; a cut leaving two rows a
side is exact.

**Built** (`render_cpu::scan::surface_edge`): where a surface's edge leaves a rectangle two rows or
fewer and it reaches past, those rows go to a scratch surface with a row beyond each crossed edge,
the converter draws there, and the rows come back — the converter's own arithmetic, unclipped. The
planner option (`pdf_render` forbidding each rectangle's edge rows, fills and rectangular strokes)
was built first and taken back: exact, but `Underway::in_strips` gave up 54 first pages for a mark
across a cut against 19 (ADR 2084 §2). Six tests in `rectangle_across_a_cut.rs` (five red on HEAD,
one holding the library's half); `page_past_the_tile.rs`'s tall page and spikes held to the byte.

**Looked at** `bug1743245.pdf` p1, the one golden that moved: row 0's rule a level darker, the grid
as it was.

**Found, not fixed**: `bug1799927.pdf` at 4× differs from one strip at 212–309 pixels by up to 255
levels at 2–16 strips, on HEAD too. A rule inside one row is stated under its area by the library
(75 for 76.5) on every division (ADR 2084 §5). `pdfjs_wikipedia.pdf` can leave `PAST_THE_BOUND`
on HEAD already.

**Gates.** `rustfmt --check`, `cargo fmt --check` and `-D warnings clippy --all-targets` for
`render-cpu` and `pdf-model` 0; nextest `render-cpu` 218 passed; `cargo test -p conformance` 443
passed; `state.sh docs` 0. Six arms against `/home/AI/arms-1623/` (export): 0 digests differ in each
(968 ×3, 965 ×3), mean error moves on 4–6 oracle pages an arm, each named in ADR 2084 §4.
`raster_golden` held 973 moved 1 (re-held), census 176 → 173 pages, 1 308 → 943 pixels, 3 levels;
`hostile_geometry` 4 passed; `fixed_documents --release` 1 passed; ahead-strips 610 / 19, HEAD's;
`pdf-model` `strip_parallelism` 2 passed; `window_against_whole` one `--clock` hold, whole digests
HEAD's, times within noise.

**Files.** `render-cpu/src/scan.rs`, `render-cpu/src/scan/surface_edge.rs` (new),
`render-cpu/tests/{rectangle_across_a_cut,page_past_the_tile}.rs`, `pdf-model/tests/raster_golden.{rs,tsv}`,
ADR 2084, `doc/state-of-play.md`, this.

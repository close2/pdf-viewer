# 1640 — A rectangle's converter is the page's, and a one-row rectangle its closed form

Slot 2 of batch one hundred, 2026-10-20, a pixels round on ADR 2100 §4's find. ADRs 2116, 2117;
Q535 not used. No ledger row moves: §10.7.4 stays `departed`, its note gains two sentences.

**Premise held in shape, not in figure; the hypothesis did not hold.** `issue12810.pdf` at 4×
(6 912 × 10 368) is cut once, at row 1 219, at every count: 7 150 pixels move, worst 12 — but the
first strip moves 172 pixels by one level, and the twelve are below the cut. Isolated at full size
(trap 213), surfaces printed (trap 222): the 172 are one bar the page drawn whole (10 368 rows,
past 8191) wrote by the closed form and the first strip (1 219 rows) gave the library's 8.8
converter — a converter chosen by the surface (trap 146). The twelve are two folding contour
strokes whose pieces go to the supersampled converter on both drawings, `ty` 4 165.2 against
2 946.2: ADR 0219's shifted origin on slot 5's pieces, no decision from the surface.

**Built.** `Scratch::for_page` carries the page's extent into every band and
`rectangle_converter_reaches` asks it beside the surface's (ADR 2116); a mark holding a rectangle
inside one device row goes to the closed form, which the library's one-row branch is under (ADR
2117) — not every sub-pixel rule, as the brief expected: those are `sub_pixel_bands`' first. Two
tests, each red against what it guards.

**Gates.** `rustfmt --check` on the four files 0; `cargo fmt --check` and `-D warnings clippy
--all-targets` for `render-cpu` 0; `nextest -p render-cpu` 223 passed; `cargo test -p conformance`
450 passed; `state.sh docs` 0. In the export of HEAD plus this change: `raster_golden` held 949,
moved 25 (ADR 2117 names them, 2 to 377 pixels, ≤ 2 levels), census at 1× 183 pages, 943 → 984
pixels, 3 levels, 11 named past the bound, passed; census at 4×, 957 drawn, 310 moving and 11 past a
level on both, only the witness changed (7 150 → 6 978, first strip 172 → 0); `fixed_documents
--ignored` 1, `hostile_geometry` 4, `strip_parallelism` 2 passed; `window_against_whole` one
`--clock` hold, every digest HEAD's (load 18–20, so its times decide nothing); six arms against
`/home/AI/arms-1639/`, 0 frame digests differ (968 ×3, 965 ×3), the oracle's mean error moves on 7,
7 and 14 pages at 1×, down on all but `bug1865341`, and on none at 4×. Instructions, one draw: under
0.02% on `signed_verified`, `prefilled_f1040` and the witness. On HEAD plus both pixel slots'
`render-cpu` as the worktree held it at 18:55: `raster_golden` moved 49 (these 25 and slot 5's 24),
census 183 pages, ≤ 984 pixels, 3 levels, passed; the witness's worst 12 → 3. Slot 5 since took its
hunks out, so `raster_golden.tsv` is re-held on this change alone (25 lines).

**Files.** `render-cpu/src/scan.rs`, `render-cpu/src/mask_cache.rs` (one line, slot 5's file),
`render-cpu/tests/page_past_the_tile.rs`, `render-cpu/tests/edge_coverage.rs`,
`pdf-model/tests/raster_golden.tsv` (25 lines), `doc/conformance/ledger.toml` (§10.7.4's note),
`doc/state-of-play.md`, ADRs 2116, 2117, this.

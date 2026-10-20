# 1648 — A window of a page is the page's pixels, to the byte

Slot 2 of batch one hundred and one, 2026-10-20, a pixels round on ADR 2116's last section. ADRs
2132, 2133; Q543 not used. No ledger row moves: §10.7.4 stays `departed`, its note gains a sentence.

**Premise held; the hypothesis held in mechanism, not in remedy.** On HEAD a window parted from the
page at 5 478 pixels (worst 2) on the owner's witness at 1.25¹⁰ and 811 (worst 1) on
`issue12810.pdf` at 4×. Isolated by the smallest mark, matrices printed (traps 213, 222): the
window, a target under 8191, chose its rectangles' converter by its own extent (521 and 187 px),
and every other mark — measured by `crate::area`, not the library — was the same real numbers
summed at the window's magnitude. §10.7.4 fixes a pixel by its device corner, which a window shares
with the page: a departure, not a permission. The brief's remedy, vertices stated in page space,
left 21 px in the round's own test, since `area`'s crossings are sums at the vertices' magnitude.

**Built.** `CpuRasterizer::rasterize_window` draws a window on the page's grid, the corner composed
last and the page's extent choosing converters; the three window callers use it (ADR 2132).
`scan::Place` hands `area` the page's matrix and its `Region` the corner, the library the page's
vertices less the corner, and a rectangle's closed form is stated on the page's grid and moved (ADR
2133). Strips and clip bands are the same construction, so they move too, toward the page.

**Measured.** Windows 5 478 → 0 and 811 → 0. Division census 1×: pages moving 181 → 58, worst 3
→ 3; 4×: 310 → 25, worst 12 → 3; no page worse than HEAD anywhere. `raster_golden`: 949 held, 18
moved one level at 1–273 px (clip bands), two looked at, TSV regenerated. Instructions: +0.11 %
witness, +0.60 % `issue12810` 4×, +0.24 % ISO page 101.

**Gates.** `rustfmt --check` on the 19 files, `cargo fmt --check` and `-D warnings clippy` for
`render-cpu`, `viewer-host`, `viewer-confined`, `viewer-ffi` 0; nextest 227, 285, 172 passed;
`cargo test -p conformance` 450 passed, 1 failed: `worker_features`, slot 5's `mozjpeg-rs`;
`state.sh docs` 0. In the export: `raster_golden` as above, its census passed (60 pages, ≤ 513 px,
3 levels, 8 of 11 held names may leave); `hostile_geometry` 4, `strip_parallelism` 2,
`fixed_documents --ignored` 1 passed; six arms against `/home/AI/arms-1647/`: 0 frame digests
differ, the oracle's error moved on 13–22 pages an arm by ≤ 0.0014 — the three largest are HEAD's
16 strips parting from the page (538, 417, 99 px) where this build's do not; `window_against_whole`
one `--clock` hold at 4 and 24 threads, every window time within its spread.

**Files.** `render-cpu/src/{lib,surface,strips,glimpse,soft_mask,thin_segment,stroke,fill,scan,
area,mask_cache,image/centres,stroke/folding}.rs`, `render-cpu/tests/window_on_the_page_grid.rs`,
`viewer-host/src/{oversize,drawing}.rs`, `viewer-host/examples/window_against_whole.rs`,
`viewer-confined/src/worker.rs`, `viewer-ffi/src/session.rs`, `raster_golden.tsv`, the §10.7.4
note, `state-of-play.md`, ADRs 2132, 2133, this. **Unfinished**: ADR 2133 §5's routes.

# 1453 — A pixel gate states its strip count

Slot 4 of batch seventy-one, 2026-10-08, a pixels round. ADR 1742; no row moved, no question.

**Premise.** Half held. `raster_golden.rs` drew with `CpuRasterizer::new()`, whose strip count
`plan_strips` asks of `available_parallelism` (ADR 1734 §4, trap 134). `turn_path` holds no
digest — its witness is a command count — and the arms' `PDFVIEWER_RASTER_TIMES` digest is raster's
frame, which the corpus gate already holds equal at one encode thread and many; only the arms' mean
column, through the gate's CPU oracle, followed the machine.

**Built.** `raster_golden` draws each page in one strip (`STRIPS`), ADR 0219's undivided page, and
`raster_golden.tsv` is regenerated once under it: 150 of 967 first pages moved, all raster only.
The oracle's render and `examples/raster_digest` and `own_ink` state one strip for the same reason.
`render-raster`'s corpus oracle states `render_cpu::MAX_STRIPS` (now public, 16): its CPU clock is
half the survey, and sixteen is what the backend asks for on any machine of sixteen CPUs or more.

**Measured.** The 149 moved pages that open without a password, drawn at one CPU and unpinned:
7 042 pixels in all, worst 2 levels (5 pages), at most 0.36% of a page; on `issue1350.pdf` every
moved pixel is on a box's horizontal edge (looked at, trap 1). Three pages exceed
`strip_parallelism.rs`'s one pixel in a thousand and five its one level (ADR 1742 §2).

**Unfinished.** `strip_parallelism.rs`'s bounds against those eight pages are left to a later
pixels round. The fixture tests that call `CpuRasterizer::new()` were not read one by one.

**Gates.** `rustfmt --check` on the six Rust files: exit 0. `cargo clippy -p render-cpu -p
render-raster -p pdf-model --all-targets` with `-D warnings`: exit 0. `cargo nextest run -p
render-cpu`: 176 passed; `-p render-raster -p pdf-model`: 2 104 passed, 21 skipped.
`raster_golden` at one strip: against HEAD's file exit 101 (150 moved), `update` exit 0, then held
974, moved 0, exit 0 unpinned (18.61 s), pinned to eight CPUs at eight and four threads (23.50,
22.50 s) and to two CPUs (33.61 s). From an export of HEAD plus this patch: the six arms exit 0,
0 of 5 795 page lines moved in digest or mean against `/home/AI/arms-1450/`; `turn_path` twice,
exit 0, 33 of 33 figures judged and inside; the oracle exit 0, 1 968 pages, every list held.
`cargo test -p conformance`: exit 0, 421 passed in 55 binaries.

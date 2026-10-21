# 1665 — The library's range is asked of the page, and a mesh is drawn on the page's grid

Slot 3 of batch one hundred and three, 2026-10-21, a pixels round on ADR 2149 §3's bound and ADR 2148
§3's mesh. ADRs 2166, 2167; Q560 not used. No row moves: §10.7.4 stays `departed`, its note's last
sentence rewritten.

**Premise held; hypothesis did not.** Census first (`pdf-model/examples/library_range_census`, new):
of 963 pdf.js first pages, 1 is past 8191 at 1× (past every pixel budget) and 9 at 4×, one with a
knockout group (`issue12810.pdf`, rows 8 154 to 8 944). Its 13 elements are clipped, so they take ADR
1320's `DestinationOut`/`Plus` stages that `crate::area` measures: no corpus page at the gates' scales
holds a mark the range decides, and none moves. The census counts groups, which over-counts; the
probe printing each fill's blend mode is what said so (proposed trap below).

**Built.** `scan::library_reaches`: the library anti-aliases a mark only inside the range on the
page's grid and on the surface, asked by `fill_as`, `stroke` (handed `Placed`), `mask_fill`, and for
rectangles `Exact::usable_on` in `fill_rectangles` (now a `Place`) and `intersected`.
`MeshRaster::build_within` takes a region (`build` is it over the target); `fill_mesh` maps by the
page's matrix within the surface's part of the page. Four tests, red on HEAD: knockout window 644
px, tall page at two strips 864 px, mesh window 8 px (rebuilt once: at a zero translation across it
passed on HEAD, trap 13), and `pdf-render`'s region test.

**Measured.** Six arms against `/home/AI/arms-1663/`: 0 digests differ, the oracle's error moves on
0 pages in every arm. Witness, one `--clock` hold: 1.25⁸ 0 → 0 px, digests equal; 1.25¹⁰ window 0 →
0 px, window digest equal, **page digest moves**. Isolated (trap 213): one strip identical in both
builds; at 4 and 16 strips HEAD parts from it by 4 px, one level, x 8 760 to 8 948, this round by 0 —
a build with only `fill_rectangles`' old surface grid reproduces the 4 (trap 231's seventh route).
The pixel at (8 786, 5 315) is the top of the bar of "1" in "12. FIRE R…", looked at.

**Gates.** `rustfmt --check` on the 7 files 0; `-D warnings clippy --all-targets` for `render-cpu`
and `pdf-render` 0, for the census example (in the export) 0; nextest `render-cpu` + `pdf-render` 525
passed; `cargo test -p conformance` 420 passed, 1 failed — `records`, on records 1663 and 1667 (not
this round's); `state.sh --round 1665 docs` exit 0. In the export: `raster_golden` 974 held, 0 moved,
census 29 pages, 383 px, 3 levels, 1 named (unchanged, TSV not regenerated); `hostile_geometry` 4
passed; `fixed_documents --ignored` 1 passed, 77 checked; six arms each exit 0.

**Files.** `render-cpu/src/{scan,fill,stroke,shading}.rs`, `render-cpu/tests/window_on_the_page_grid
.rs`, `pdf-render/src/shading.rs`, `pdf-model/examples/library_range_census.rs` (new), the §10.7.4
note, `state-of-play.md`, ADRs 2166, 2167, this. Shared with slot 2: `scan.rs`, by `-U0` hunks
(`scratchpad/r1665/scan-1665.U0.patch`). **Unfinished**: ADR 2166 §3's `Source` coverage route; a
smoothed image's edge still asks the surface's range.

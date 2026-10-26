# 1785 — A stroke painted with a ramp is drawn from the oracle's own cells

Session 1785, slot 7 of batch 118, an oracle round; 2026-10-26. ADR 2404; ADR 2405 and Q651 unused.

**Premise.** Held: `draw_stroke` handed an opaque axial or radial ramp to `tiny-skia` as a gradient
(ADR 2356 §1's named residue). **Not held as the brief put it**: "the stroked outline's bounds" is no
one outline the library builds — the long mitres' wedges, the folding pieces and a dash's caps lie
outside the stroker's solid outline, and a mark too thin to measure is drawn a device pixel wide. The
extent is `pdf_render::stroked_bounds` (§8.4.3 term by term, ADR 0165) and a reach past it of the
clause's half-widths of one pixel. **Hypothesis.** Not held in its count: 89 strokes on 8 page ones
reach a ramp (88 drawn from cells, one left to the gradient by the alpha rule). Held in its direction.

**Built.** `stroke.rs`: `mark_brush` (a ramp's cells or `paint`'s brush, for the stroke and for
§8.5.3.2's filled marks) and `ink_extent`; `shading.rs`: `Extent` and `OnSurface::bounds_of_ink`;
`fill.rs` passes `Extent::Filled`. Two tests in `a_level_where_the_paint_is_the_pixel.rs`: a stroke's
level, red against HEAD's tree; a mitre tip's own `t`, red with the cells bounded by the path's box
grown by half a width (150 against 161) — the extent's calibration (trap 13).

**Pixels.** Census by what the backend draws (trap 232): a print in my export over the cpu arm at 1x.
Before and after at 1x every moved pixel is one level, 3 to 1 481 a page; looked at `issue19360` and
`issue6769`. Against HEAD's six arms by page, every arm exit 0, no device digest moved: 0 rises, 27
falls, all census pages. Sum of page means `cpu` 49.5936 → 49.5886 at 1x, 15.9886 → 15.9869 at 4x;
`gpu` 50.4659 → 50.4609, 18.0714 → 18.0697; `compute` 8.7762 → 8.7712, 4.9720 → 4.9703. `raster_golden`
re-taken: the census's 8 pages, raster digest only, listed in ADR 2404 §3. Callgrind at 1x, one thread,
HEAD then this: `issue20513` 245.1 M → 245.9 M, `issue19360` 130.3 M → 133.0 M instructions.

**Files.** New: ADR 2404, this record. Edited: `crates/render-cpu/src/{stroke,shading,fill}.rs`,
`crates/render-cpu/tests/a_level_where_the_paint_is_the_pixel.rs`,
`crates/pdf-model/tests/raster_golden.tsv`, `doc/state-of-play.md` (one clause). No ledger row moves:
§8.7.4.5.3 and §8.7.4.5.4 stay `implemented`, neither note named the residue. I worked in
`scratchpad/r1785/`.

**Gates.** Worktree: `rustfmt --check` on my four Rust files 0; `cargo fmt --check -p render-cpu` 0;
`-D warnings clippy -p render-cpu --all-targets` 0; nextest `render-cpu` 260 of 260; rustdoc `-D
warnings -p render-cpu --document-private-items` 0; `cargo test -p conformance --no-fail-fast` exit 101, 465 passed, 1 red, `records` on 1780's record, not mine. Export: six arms exit 0 each;
`raster_golden` check exit 101 (8 moved), update exit 0; `turn_path` in one `--clock --nice 0` hold,
this then HEAD twice, load 2.83–2.93 at the starts, no render-node holder of this account, sclk level
1 at each start: exit 101, 101, 0, 0 — `issue19802` step 1.84 and 1.83 ms against 1.80, HEAD's as much
as this; this over HEAD best of two 0.962 to 1.069, median 1.012, 33 figures. Duration 7131 s.

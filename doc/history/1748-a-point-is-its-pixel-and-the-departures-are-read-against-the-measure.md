# 1748 — A point is its pixel, and the departures are read against the measure

Session 1748, slot 4 of batch 114, a clause round; 2026-10-25. ADRs 2330, 2331; no `Q`. Every
`partial` row is blocked outside this machine, so this round worked `departed` rows.

**Premise.** Held in part: the §10.7.4 note's opening still named the one pixel as departed after the
exact measure was written into its body. **Hypothesis.** Not held: the exact measure lifts none of
(1)–(3) — it is (1)'s own construction made exact, and (3) is off its path (ADR 2331). The zero-extent
pixel is not a departure argued from "intersects": NOTE 1 and §8.5.3.3.1's `shall` answer yes, the
hedge after it has no modal verb, and nothing had priced it, so it is built (ADR 2330).

**Built.** A degenerate subpath paints the device pixel under it — fill, clip, a matrix of rank zero
and a path rank one carries onto a point; a subpath on a line across the axes, which the module had
named "not covered" and the ledger had not, is the band of one device pixel along it, by an exact
collinearity test. `raster-gpu` mirrors both. The first walk moved 195 of 5 799 digests, and a census
of what drew them found 31 of 33 pages were glyph contours at one point: a font's contour is the
font's (§10.7.4's last paragraph), so `pdf-font` drops it (ADR 2330 §3). Decided against: a subpath
retracing itself off any one line (§4). Looked at: a fixture, `issue6961`, `bug1721218_reduced`, `issue2537r`.

**Rows.** §8.5.3.3.1 `departed` → `implemented` ("the subpath shall be considered to enclose the
single device pixel lying under that point"). §10.7.4 stays `departed`, its note's opening rewritten
to (1)–(3) and the retraced subpath; §8.7.4.5.7 and §8.7.4.5.8 stay `departed`, re-read against the
chords (ADR 2331), one stale flatness sentence corrected. `doc/todo/65`, `11` and `00` kept true.

**Files.** New: ADRs 2330, 2331, this record, `pdf-model/tests/scan_conversion_shapes_without_area.rs`.
Edited: `pdf-render/src/{collapsed.rs,collapsed/stroke_image.rs,display_list.rs,geom.rs,paint.rs}`,
`pdf-font/src/loading/drawing.rs`, `pdf-model/src/content/report.rs`, `pdf-model/tests/{scan_
conversion_collapsed_by_transform.rs,variable_text.rs,raster_golden.tsv}` (32 lines),
`render-cpu/tests/{window_on_the_page_grid.rs,zero_area_clip.rs}`, `raster-gpu/src/{resources.rs,
encode/fill.rs}`, `raster-gpu/tests/{collapsed_fills.rs,no_ink.rs,mul_add_hazard.rs}`, the ledger,
`doc/todo/{00,11,65}`. I worked in `scratchpad/r1748/`.

**Gates.** Worktree: `rustfmt --check` per file 0; `cargo fmt --check` on `pdf-render`, `pdf-font`,
`pdf-model`, `render-cpu`, `raster-gpu` 0; `-D warnings clippy --all-targets` on the first four 0,
`raster-gpu` on its library and the three tests touched 0 (a sibling's new test fails `float_cmp`);
scoped rustdoc `-D warnings` on four crates 0; nextest 2 609 of 2 609 and 1 362 of 1 362; `cargo test
-p conformance --no-fail-fast` 0. Export: `raster_golden` exit 101 on 32 moved (25 list only),
re-taken; the six arms against HEAD's six arms, 37 of 5 799 digests moved on seven pages, oracle
mean error down on 16 page-lanes, up on 3 (`issue9972` on the compute lane, 0.0001), level on 18;
three `raster-gpu` suites 0. Duration 11319 s, about 215 tool uses.

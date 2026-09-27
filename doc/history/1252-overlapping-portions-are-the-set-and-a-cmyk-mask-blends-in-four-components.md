# 1252 — Overlapping portions are the set, and a CMYK mask blends in four components

Batch thirty-eight. §10.7.4 stays `partial`, narrowed; §10.7 follows. §11.5.3 `partial` → `departed`.

## §10.7.4 (ADR 1341)

§10.7.4 names no shared pixel; it scan-converts a shape after "all "insideness" computations have
been performed", so a pixel is covered by the area of the set §8.5.3.3's rule declares inside.
`render_cpu::area` summed the winding integral and declined a cell past one whole winding to
`tiny-skia`'s sixteenth. It now cuts such a row wherever edges begin, end or cross, walks each piece
with the running winding and deposits only the set's own boundary at unit weight — exact under
both rules. A stroker's outline with a join or a second subpath is walked whole: its thin overlaps
show no sign. Fixtures: two squares overlapping at a corner, both rules, fill and clip, every
pixel against inclusion–exclusion; planted away, pixel (3, 2) reads 1.0 for 0.75.

`raster_golden` moved 267 of 974 pages, all `raster only`, median 0.03% of ink; looked at: each
lands on its own 2×–8× ink at 1× (`issue6081` 68.85 → 53.98 vs 53.73, `issue20232` 19 324 →
17 932 vs 17 866), and the crops sit within a third of a level of the 8× raster averaged down.
`issue21068.pdf` left `render-raster`'s differing list. Cost: page 101 −2.7%, `issue19802` +50%.
The row keeps `doc/todo/11` items 4 and 8, which are not decisions, so it is not `departed`.

## §11.5.3 (ADR 1342)

§11.3.4 complements a subtractive component around the blend; one weighted channel cannot. A
`DeviceCMYK` mask group whose resources state a non-`Normal` `/BM` is now §11.4.7's pair, with
§10.4.2.4's nominal conversion in (weighted ink = the one-channel grey, so a `Normal` mark masks the
same) and EXAMPLE 2's `Y` as `Luminance::device_ink`. Multiply 0.16 (planted away 0.082). Hue,
Saturation, Color and Luminosity were exact on one channel already: EXAMPLE 2's weights are `Lum`'s.
The one departure left is ADR 1254's bound, as for §11.6.6 and §11.7.2.

## Files outside the brief's list

`render-raster/tests/corpus.rs`, `pdf-colour/src/colour.rs`, `pdf-render/src/soft_mask.rs`,
the confined protocol's `display_list.rs`, `pdf-model/src/content.rs` and `content/ext_gstate.rs`.

## Gates

fmt, clippy `-D warnings` and tests on seven crates; conformance passes but for three siblings'
`Vec::` names. Tier 2 under the lock: `pdf-model --test corpus` 0, `render-raster --test corpus` 0,
`raster_golden` 101 (267 moves, above), oracle 0 (1015 agree / 47 contradicted), `launch_path` 0.

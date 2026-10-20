# 1632 — An unsmoothed image's pixel takes the sample its centre maps to on the page

Slot 2 of batch ninety-nine, 2026-10-20, a pixels round on ADR 2084's "Found, not fixed". ADR 2100;
ADR 2101 and Q527 not used. No ledger row moves: §10.7.4 stays `departed`, its note, code and tests
gain the construction.

**Premise held; the hypothesis did not.** `bug1799927.pdf` at 4× differed from one strip at 212
pixels of 255 levels at 2, 4 and 16 strips, all in row 2361, none near a cut. Isolated at full size
(trap 213): stencils whose sample-row edge lies on that row's centre, drawn as a `tiny-skia`
pattern that maps a centre from the surface's first row in `f32` — ADR 0219's binade, on a paint. No
group, mask or planner row; images are not planned marks.

**Built** (`render_cpu::image::centres`): an unfiltered image's samples are chosen at each pixel's
centre stated on the page's own grid, the two matrices composed in `f64`, the division last so an
exactly placed centre is exact; the library gets one sample a pixel at whole pixels. A centre on a
shared edge takes the sample whose corner it is — a choice by analogy with §10.7.4's pixel (§8.9.4
names no side). A first formulation (invert, then multiply back) moved `issue269_2.pdf` 3 200 pixels
at 1×; the final one leaves it HEAD's. Two tests, both red against what they guard.

**Looked at**: the witness (379 pixels moved whole, a stencil bar a row longer or shorter) and
`issue6621.pdf` at 4× (1 461 pixels in five columns where a centre is exactly a sample edge).

**Found, not fixed**: `issue12810.pdf` at 4× (10 368 rows) moves up to 12 levels at every division,
in its first strip too — a decision taken from the surface's extent. The one-row rectangle is under
its area on all 2 079 dyadic probes (1 519 by over half a level); routing it moves every sub-pixel
rule at 1× beside slot 5's arithmetic, left (ADR 2100 §4).

**Gates.** `rustfmt --check` on the five files 0; `cargo fmt --check` and `-D warnings clippy
--all-targets` for `render-cpu` and `pdf-model` 0; `cargo test -p conformance` 444 passed;
`state.sh docs` 0. In the export: `render-cpu` 220 passed; six arms against `/home/AI/arms-1631/`,
0 digests differ (968 ×3, 965 ×3), oracle mean error moves on 0 pages at 1× and 4–5 at 4× (ADR 2100
§3); `raster_golden` held 974 moved 0, census 173 pages, 943 pixels, 3 levels, `pdfjs_wikipedia`
out of `PAST_THE_BOUND`; drawn ahead 610 / 19; `fixed_documents` 1, `hostile_geometry` 4,
`strip_parallelism` 2 passed; 4× census 974 pages, only the witness changed (212/255 → 0).
Instructions, one draw: witness +2.6%, page-sized image +1.6% axis-aligned, +24.9% sheared.

**Files.** `render-cpu/src/image.rs`, `render-cpu/src/image/centres.rs` (new),
`render-cpu/src/strips.rs` (a comment), `render-cpu/tests/image_centre_across_a_cut.rs` (new),
`pdf-model/tests/raster_golden.rs`, `doc/conformance/ledger.toml` (§10.7.4's row),
`doc/state-of-play.md`, ADR 2100, this.

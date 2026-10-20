# 2117 — A rectangle inside one device row is written by its closed form

Session 1640, slot 2 of batch one hundred, a pixels round on ADR 2084 §5 and ADR 2100 §4. Status:
**accepted** and **built** in `render-cpu`. Clause: ISO 32000-2 §10.7.4 (the row stays `departed`,
ADRs 1060, 1360, 2058; its note gains a sentence). Context: ADRs 0226, 0476, 2058, 2084, 2100,
2116; traps 1, 13, 214, 216. Code: `render_cpu::scan::Exact::holds_a_rectangle_inside_one_row`,
`scan::intersected`. Test: `render-cpu/tests/edge_coverage.rs`'s
`a_rectangle_inside_one_row_is_its_area_to_half_a_level` (new, red with the routing taken out).

## 1. What the clause asks, and what the library gave

§10.7.4: "The area covered by painted pixels shall always be at least as large as the area of the
original shape." This tree reads anti-aliasing as giving the shape's area and coming out under it
as a defect (`doc/todo/_scan-conversion.md`). `tiny-skia`'s rectangle converter states a rectangle
whose top and bottom fall in one device row by a branch of its own — the row's coverage
`bottom − top − 1` in 8.8 fixed point, each corner its overlap times that, shifted down eight bits
— and that is under the area at every placement: ADR 2100 §4 measured it on the library alone over
2 079 dyadic one-row rectangles, every one under, 1 519 by more than half a level, and ADR 2084 §5
gave 75 for an area of 76.5 at `19.6–19.9`. `scan::mask_rectangle`'s closed form is within half a
level of each, its own rounding.

## 2. Decision

A mark any of whose rectangles lies inside one device row and short of filling it is written by
the closed form into `intersected`'s coverage buffer, as a rectangle past the library's range
already is (ADRs 2058, 2116). The question is asked of the rectangle's own extent, which a strip's
whole-row origin does not change, so every division takes the same converter for it. Where
`intersected` declines — no anti-aliasing, or §11.4.6's `Source` — the library draws it as before.

## 3. Which marks these are — the brief's population did not hold

The brief expected every sub-pixel horizontal rule of every page at 1× to move. It does not: a
fill every subpath of which is thinner than a pixel is `pdf_render::sub_pixel_bands`' (ADR 0226)
and a stroke thinner than a pixel is `draw_rule_as_bands`', and both draw the whole pixel line at
the shape's coverage before any rectangle converter is asked. What reaches the one-row branch is
a rectangle beside a portion those decline — a path that also states a thicker rectangle, two
rectangles meeting in one pixel line, a glyph drawn as rectangles whose dot is one row tall. The
test is built on the first of those for that reason; a one-row rectangle alone never reaches it.

## 4. Measured

On the round's export of HEAD plus this round's change (ADR 1890), against an export of HEAD:

- **`raster_golden`**: held 949, **moved 25**, every one "raster only" — `160F-2019`,
  `XiaoBiaoSong`, `blendmode`, `bug1865341`, `issue11913`, `issue12295`, `issue13211`,
  `issue13447`, `issue1350`, `issue14297`, `issue15262`, `issue16091`, `issue17492`, `issue1905`,
  `issue6410`, `issue6931_reduced`, `issue7454`, `issue840`, `issue9713`, `opt_demo`,
  `poppler-395-0-fuzzed`, `prefilled_f1040`, `radial_gradients`, `signed_verified`,
  `tracemonkey_a11y` — from 2 to 377 pixels a page, at most 2 levels; re-held in
  `raster_golden.tsv`. Looked at (trap 1):
  `signed_verified.pdf` and `issue11913.pdf` at 1×, HEAD, this and the difference side by side —
  the same lines of text, the moved pixels on the dots and bars of glyphs drawn as rectangles.
- **The division census at 1×** (`raster_golden`'s second test): 183 pages move at some division
  on both; the most pixels 943 → 984 (`prefilled_f1040.pdf` at 16 strips, every one a level — the
  closed form at a shifted origin rounds a half-level boundary where the 8.8 converter truncated
  at 1/256), at most 3 levels on both; 11 named past the bound on both; the test passes.
- **The division census at 4×** (`stroke_cut_census --draw` on every tracked first page, 957
  drawn): 310 pages move at some division on both, 11 past one level on both, worst 12; the one page
  whose figures changed is ADR 2116's witness.
- **The six arms** against `/home/AI/arms-1639/`: 0 frame digests differ in any arm (968, 968, 968,
  965, 965, 965 pages) — the device and compute lanes draw no rectangle through this converter. The
  mean error against the oracle, which is this backend at sixteen strips, moves on 7, 7 and 14 pages
  at 1×, by at most 0.0007 and down on every page but `bug1865341` (0.0270 → 0.0272, and 0.0059 →
  0.0057 on the compute lane), and on none at 4×.
- **What it costs**, counted by `callgrind_rasterise`, one draw on one thread, HEAD → this:
  `signed_verified` 65.36 → 65.28 million instructions at 1×, 415.51 → 415.54 at 4×;
  `prefilled_f1040` 302.34 → 302.35 at 1×, 885.08 → 885.21 at 4× — under 0.02% either way.

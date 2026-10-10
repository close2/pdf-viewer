# 1819 — A stroke drawn a row at a time forbids no cut, and a tall mark is measured in runs of rows

Session 1491. Status: **accepted** and **built**. The project owner's target for batch seventy-eight
is performance — "loading and display time … the CPU renderer AND the GPU path" — and round 1488's
record named the cause this takes up: `/tmp/demo_heavy_engineering_A1.pdf` rasterises on one thread
because `pdf_render::strips` counts a stroke's whole extent as rows no strip may begin at, and the
sheet's frame is `re S`. Amends ADR 0139's stroke rule; ADR 0219's property and ADR 1758's bound
stand unchanged. Context: ADRs 0137, 0138, 0139, 0219, 1082, 1102, 1348, 1742, 1758, 1813; traps 1,
13, 66, 134. Code: `crates/pdf-render/src/strips.rs` (`unsplittable_rows_where`, `PlannedStroke`),
`crates/render-cpu/src/lib.rs` (`drawn_row_by_row`, `plan_strips`, `CpuRasterizer::strip_plan`),
`crates/render-cpu/src/area.rs` (`Region::runs`, `fill`). Tests: `crates/render-cpu/tests/stroke_cuts.rs`,
`strips.rs`'s three new unit tests. Instrument: `crates/pdf-model/examples/stroke_cut_census.rs`.

## 1. The premise, and what the brief's hypothesis did not see

The premise held. The witness at 150 dpi (4967 × 3509) states 4 400 060 strokes and **every one of
them has round caps**: 3 704 964 straight and axis-aligned at or under a device pixel, 667 897
oblique at or under one, 27 131 axis-aligned wider, 61 curved (`stroke_cut_census`). With every
stroke's extent forbidden it grants **0 of 3509 rows**, so sixteen strips asked are one granted.

That census decided the rule before anything was built. The brief's three candidates are all
geometric — an axis-aligned rectangle's stroke, a stroke of only horizontal and vertical segments,
any straight segment under ADR 0139's rule — and each is a statement about which edges a *stroker*
puts in an outline, read against `tiny-skia`'s converter, which clips a path at the strip's edge and
so re-states every curve and oblique edge it cuts. On this page a round cap is a curve at both ends of
every stroke, so every geometric rule leaves the page where it is: four million cap rows cover all
3509 rows many times over.

What makes a cut exact is not the outline's geometry but **the converter that draws it**. Since ADR
1082 almost every stroke's outline is drawn by `render_cpu::area`, which evaluates each row's two
crossings from the edge's own device coordinates (`Accumulator::accumulate`): a row of the page and
the same row of a strip are the same two evaluations wherever the strip begins, curves' chords
included, and nothing is clipped against the strip's edge. So a stroke that reaches that converter in
every division forbids no row at all, whatever its caps, joins or slope. What remains is ADR 0219's
shifted origin, which every mark of every strip but the first already carries whether a cut crosses
it or not — measured below to be the same one level.

## 2. Decision

**The backend says which strokes it draws a row at a time.** `pdf_render::unsplittable_rows` keeps
ADR 0139's rule — every stroke's whole extent — and `unsplittable_rows_where(list, target, row_local)`
asks a backend of each stroke, as a `PlannedStroke` stated in the page's own pixels: its path, its
`Stroke`, its path-to-device transform, its device extent, and whether it may be composited by
anything but painting over (a knockout element, a half of a shaped pair, a mark in a soft mask's
group). Fills, clips and soft masks keep their rows exactly as before. The fact is a backend's
because it is about its drawing, and `pdf-render` is where the walk and the other rules already are.

**`render-cpu` answers `true` only where no road of `draw_stroke` leaves `area`**, each asked of the
page so that one answer stands for every division (`drawn_row_by_row`): anti-aliasing on and not a
replacing composition (`carries_coverage_as_alpha`); no curve in the path (a curve bending tighter
than its half-width is stroked as pieces, ADR 1348, which `scan::mask_fill` leaves to `tiny-skia`);
a width above one level of a pixel (`unmeasurable_width`, ADR 1102 — under it the hairline and the
widened band draw); and a device extent inside `SUPERSAMPLED_LIMIT` both on the page and after any
strip's shift. Everything else forbids its extent as before.

**A region's height no longer declines the converter.** `area::region` declined a mark whose
accumulator would pass `CELL_BUDGET` (2²² cells), and the region is clamped to the surface — a strip
— so the sheet's frame (4967 × 3509 = 17.4 M cells) was `tiny-skia`'s on the whole page and `area`'s
in every strip of sixteen: one mark, two converters, either side of every cut. `Region::runs` cuts a
tall region into runs of whole rows the budget admits and `fill` measures them one after another;
only a single row past the budget declines. A run is exactly a strip, so it changes nothing a region
measured whole would have drawn: the run's suspicion of overlap and its `SET_WORK` are its own, as a
strip's always were. A later run that spends `SET_WORK` clears what the earlier ones wrote before the
fallback draws.

The budget was "a cost guard and not a condition" in `area.rs`'s own words, and it is still one; what
changes is that a mark past it on the whole page — at 1× only on a page over four million pixels — is
now measured exactly, as ADR 1082 intended for every mark, rather than supersampled.

## 3. Measured

On the witness at 150 dpi, every figure under the heavy-walk lock at four rayon threads:

| | before (HEAD) | after |
|---|---|---|
| rows a cut may fall on | 0 of 3509 | 1983 of 3509 with every stroke freed; the backend grants nine of sixteen strips |
| rasterising, `stroke_cut_census --draw` | 7.98 s, one strip | **2.94 s** at sixteen asked |
| `quorra-transform render`, wall, three interleaved | 11.06, 11.06, 12.15 s | **6.64, 7.74, 7.74 s** |
| the same, user time | 10.8–11.0 s | 13.0–16.8 s |
| peak resident | 1.32–1.33 GiB | 1.33–1.34 GiB |

The user time is parallelism's price, paid where it buys latency (ADR 0139's last habit). Four
threads are what the lock grants; the planner asks the machine for up to sixteen.

**What moved, looked at (trap 1).** The page drawn in one strip moves against HEAD by 74 406 pixels,
up to 39 levels, every one on a mark past the cell budget — the sheet's nested frames and the title
block's borders — because `area` measures their edges now where `tiny-skia` supersampled them: the
shapes are the same, and the 39 is an edge of 0.08 coverage the supersampler drew as nothing, which
§10.7.4's "at least as large as the area of the original shape" is the reason not to. Sixteen strips
against one move 13 892 pixels, 13 890 by one level and two by two, scattered over the battery room's
hatching; 128 of them lie within two rows of a cut. That is ADR 0219's shifted origin over a page
with millions of non-dyadic edges, not a cut re-stating anything — which moves sixteen levels and
more (section 4).

**Over the corpus** (`raster_golden --ignored`, 967 first pages at 1×): the golden **held 974, moved
0** — no first page has a mark past the cell budget at 1×. The division test moved from 153 pages to
173 that move at some division, at most 946 pixels and 3 levels, and three new pages passed ADR
1758's bound: `bug1743245.pdf` (417 pixels, one level: one grid rule's edge row), `issue14117.pdf`
(608, two: row 87, where a grey box's fill, its 0.05-pixel stroke and a one-pixel rule all end, on
the first row of the strip a cut now begins at), `prefilled_f1040.pdf` (946, one: the answer boxes'
horizontal edges and the flat tops of glyphs). Each was drawn with its moved pixels painted and
looked at; all three are ADR 1758's own classes — an edge on a rounding step at a shifted origin,
and several marks' edges in one pixel — on pages no division had reached before, and are named in
`PAST_THE_BOUND` at those ceilings.

**The six arms** (an export of HEAD with this patch and nothing else, its own target, against
`/home/AI/arms-1490/`): every arm exit 0, **0 frame digests moved** in any — raster's lanes are not
touched — and the same refused and not-comparable pages. The fifth column, the mean against the
CPU oracle, moved where the oracle did: on 5 to 7 pages at 1× by at most 0.0013, the pages the
division test names, and on 12 to 16 at 4×, where a page's large marks pass the cell budget and are
now measured exactly; there the compute lane came closer on 10 of its 16 and the CPU arm on 12 of
its 15, and the gpu lane moved away on 9 of its 12 (`issue8702.pdf` 0.0357 → 0.0584).

## 4. Calibrated (trap 13)

`stroke_cuts.rs` holds four scenes the old rule left as one strip and the new one cuts (asserted, so
equality cannot pass by never cutting): a 2400² frame `re S` round a sheet of ticks and a butt-capped
grid **byte for byte** at 2, 3, 5, 8 and 16 strips; a round-capped grid and hatching with round joins
inside ADR 1758's bound for one coverage (one level, under one pixel in a thousand), because their
outlines are not dyadic and the shifted origin reaches them. Planted both ways: `area::region`'s
height decline put back fails the frame at 2 strips (2 357 pixels); `area` withdrawn from strokes
(`tiny-skia` draws every outline) fails hatching at 2 strips by **16 levels** on 58 pixels — ADR
0138's mechanism, which the bound exists to see.

## 5. Left

- Fills keep ADR 0139's geometric rule although most of them reach `area` too; the same question
  asked of a fill would free the rows a glyph's curves forbid, and it needs the fill's own roads
  (`Exact`, the rectangle constructions) answered first.
- A run's overlap suspicion and `SET_WORK` are per run, as a strip's were; a mark whose overlap
  shows only in rows outside its run is settled from its own rows, bounded by the overlap's area.

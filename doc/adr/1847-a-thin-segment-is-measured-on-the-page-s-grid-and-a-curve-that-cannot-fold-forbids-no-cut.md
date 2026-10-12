# 1847 — A thin segment is measured on the page's grid, and a curve that cannot fold forbids no cut

Session 1505. Status: **accepted** and **built**. The project owner's target for batch eighty is
performance — "the first frame, zooming in and out, the CPU renderer and the GPU path" — and ADR
1837 §6 named this lever: "2169 of the witness's 3509 rows at 150 dpi stay forbidden, by the strokes
the backend refuses and by clips", strokes under one level of a pixel first. Amends ADR 1819's
stroke rule and ADR 1837's fill rule; ADR 0219's property and ADR 1758's bound stand. Context: ADRs
0139, 0219, 1082, 1348, 1758, 1819, 1829, 1837; traps 1, 13, 134, 146. Code:
`crates/render-cpu/src/lib.rs` (`drawn_row_by_row`, `converted_row_by_row`, `outlined_whole`),
`crates/render-cpu/src/thin_segment.rs` (`draw`), `crates/pdf-render/src/strips.rs`
(`PlannedStroke::paint`, `::blend`). Tests: `crates/render-cpu/tests/stroke_cuts.rs`. Instrument:
`crates/pdf-model/examples/stroke_cut_census.rs` (the refusals by reason). No ledger row moves.

## 1. The premise did not hold, and the census said which mark it was

The census now names every mark the backend refuses by its first reason, and the rows that reason
holds by itself (`refusals`). On the witness at 150 dpi (4967 × 3509) **the backend refuses 61
strokes, and they are all curved**: round-capped, wider than a pixel, the drawing's arcs. With those
61 freed and every other answer kept, **0 of the 2169 forbidden rows remain**. No stroke under a
level is refused that would otherwise reach the page (a one-segment one is a thin segment, and the
witness's longer ones are wide enough), and **clips forbid no row** (every mark freed: 0 of 3509).
The same holds at the fit (1417 × 1001: 61 curved, 0 of 618 left) and at 1× (0 of 1037 left).

So the brief's hypothesis — "freeing thin strokes takes the witness from nine strips to the
planner's maximum" — was right about the result and wrong about the mark: it is the curve rule.

## 2. Decision

**A curved stroke is asked the draw's own question.** ADR 1819 refused every curve because one
bending tighter than its half-width is stroked as ADR 1348's convex pieces, which `scan::mask_fill`
leaves to `tiny-skia`, whose converter re-states a cut edge. But `draw_stroked_outline` takes that
road only where `curves_may_fold` says a curve may fold, and that test reads the path in its own
space and the width `convert::stroke` resolves — neither of which a strip changes. So
`outlined_whole` asks it of the page: a curve that cannot fold is the stroker's outline, which
reaches `area` as every outline does (ADR 1819 §1). A dashed curve keeps its rows, since its dashes
cut the curves the test would read.

**A thin segment is admitted and measured on the page's grid, and only its rows move to the
surface.** Since ADR 1829, `draw_stroke` asked `pdf_render::thin_segment` under the strip's matrix,
so a strip's shifted origin (ADR 0219) re-rounded each segment's endpoints, and the planner already
freed such a segment above a level: on the witness, **64 221 pixels moved between the page drawn
whole and in two strips**. `thin_segment::draw` now asks under `transform.then(page)` and subtracts
the surface's first row from the block's row, which is exact — so a cut through one draws the page's
bytes, and the planner frees one under a level as well, asked of the same transform (trap 146).
`PlannedStroke` carries the paint and blend mode for that question. It is asked only of the strokes
the converter's question refuses: asked first, it cost a third more planning on the witness.

**A fill's span is asked at its own columns.** ADR 1837 asked whether a fill's whole span fits one
accumulator at the page's width, the widest a region can be; the region is as wide as the mark's
own columns, which a strip of whole rows never changes, so `drawn_row_by_row` asks at those, with
two columns of margin. It frees no row on the witness; a tall rule of a drawing's border is its case.

## 3. Measured

On the witness at 150 dpi, HEAD's export against HEAD's plus this, each in its own target, the
census's draws interleaved three times in one `--clock` hold (a draw's figure includes its plan):

| | HEAD | this ADR |
|---|---|---|
| rows a cut may fall on | 1340 of 3509 | **3509 of 3509** |
| strips granted of sixteen asked | 9 | **16** |
| planning | 252, 254, 257 ms | 255, 247, 245 ms |
| pixels moved, whole against 2 / 4 / 16 strips | 64 221 / 68 530 / 65 783 | **6 467 / 7 763 / 7 582**, at most 2 levels |
| sixteen asked, four threads | 1.42, 1.77, 1.42 s | 1.41, 1.42, 1.42 s |
| four asked, four threads | 1.26, 1.66, 1.27 s | 1.31, 1.09, 1.09 s |
| sixteen asked, sixteen threads | 1.10, 1.19, 1.93 s | **0.95, 1.53, 0.98 s** |

**The hypothesis's figure did not hold**: "from 2.0–2.3 s to under 1 s on four threads". Nine strips
already keep four threads busy, so on four the sixteen strips buy nothing and cost their replay;
the gain is where there are threads to draw them on, a median of 1.19 → 0.98 s at sixteen, on a
machine whose neighbours moved single draws by half a second either way.

The page drawn whole is unchanged — a one-strip surface's first row is 0, and the curved and
narrow-fill answers move only the plan: `raster_golden` held 974 first pages, moved 0. The division
test named two pages past ADR 1758's bound, each painted and looked at: `issue9418.pdf` (467
pixels, 2 levels — single pixels on the edges of four large glyphs whose contours overlap, below the
cuts its title block's curved frame now grants) and `prefilled_f1040.pdf` (1 308 pixels against its
946, one level — the answer boxes' edges at the more cuts its curves grant); both are named at those
ceilings. The six arms against `/home/AI/arms-1502/`: every arm exit 0, **0 frame digests moved**,
the oracle's mean moved on 1–3 pages an arm by at most 0.0004.

## 4. Calibrated (trap 13)

`stroke_cuts.rs` gains three scenes, each asserted one strip under ADR 0139's rule and cut under
this one: a sheet of 6 000 thin dots at non-dyadic coordinates, half of them under one level, held
**byte for byte** at 2, 3, 5, 8 and 16 strips; staggered rings, held inside ADR 1758's bound; ten
fills 7 575 rows tall and 31 columns wide among squares, held to the same bound. Planted: the thin
segment measured on the strip's grid again fails the dots at 2 strips (16 pixels, one level); every
curve refused again fails `every_scene_was_one_strip_before`; every curve drawn as pieces while the
planner frees it fails the rings at 2 strips by **16 levels** on 47 pixels, ADR 0138's mechanism;
the fill asked at the page's width again leaves the fills one strip. A ring whose pen is wider than
its radius keeps every row (`a_curve_that_may_fold_keeps_its_rows`).

## 5. Left

- Sixteen strips buy latency only where there are threads to draw them on: on four, a strip's
  replay of every command's extent test (ADR 1829 §6, 9.9 %) is what more strips cost, and the
  planner asks the machine's processor count, not the pool's. The extent test computed once per
  magnification is ADR 1841's.
- A zero-length round dot (28 684 on the witness) is still `split_degenerate`'s disc; ADR 1829's
  construction measures a disc, and admitting it is a change to `pdf_render::thin_segment`, which
  the device's line primitive also asks (ADR 1835) — so it is a decision for both backends at once.
- A clip's segments keep ADR 0139's rows; on the witness they hold none.

# 1837 — A fill drawn a row at a time forbids no cut, and a mark is suspected of overlapping as a whole

Session 1500. Status: **accepted** and **built**. The project owner's target for batch seventy-nine
is performance — "loading and display time … the CPU renderer AND the GPU path" — and ADR 1819 §5
named this lever: fills still forbade cuts by ADR 0139's geometric rule although most of them reach
`render_cpu::area`. Amends ADR 0139's fill rule as ADR 1819 amended its stroke rule; ADR 0219's
property and ADR 1758's bound stand. Context: ADRs 0137, 0139, 0219, 1082, 1341, 1347, 1758, 1819;
traps 1, 13, 66, 134, 146. Code: `crates/pdf-render/src/strips.rs` (`PlannedFill`, `PlannedMark`,
`unsplittable_rows_and_freed_fills`),
`crates/render-cpu/src/lib.rs` (`drawn_row_by_row`, `plan_strips`, `draws_row_by_row`),
`crates/render-cpu/src/area.rs` (`suspected_beyond`, `whole_span_fits`). Tests:
`crates/render-cpu/tests/stroke_cuts.rs`, `strips.rs`'s three new unit tests. Instrument:
`crates/pdf-model/examples/stroke_cut_census.rs --corpus`.

## 1. The premise held, the hypothesis did not

**Premise.** `unsplittable_rows_where` asked the backend of strokes only; a fill's curves and oblique
edges forbade their rows whatever converter drew them. Held, in the code and in the census.

**Hypothesis: "most forbidden rows after ADR 1819 are fills with curves or soft masks, which stay
forbidden, so the gain is on plans and forms, not text pages."** The first half held and the rest
did not. Over the 957 first pages the tracked corpus plans (`stroke_cut_census --corpus`), with
fills not asked, **72 % of the forbidden rows are forbidden by fills alone** — 92 985 of 128 609 at
150 dpi, 44 698 of 61 511 at 1× — and three quarters of those by curved fills alone (69 349 and
33 379), on 614 pages. Soft masks are not where they are. But a curved fill does not stay
forbidden: it reaches `area` as a glyph does, flattened there, and that converter evaluates each
row from the edges' device coordinates exactly as it does a stroke's round cap (ADR 1819 §1). So
the pages that gain are text pages — `tracemonkey.pdf` and its eight variants, `issue840.pdf`,
`ZapfDingbats.pdf`, `complex_ttf_font.pdf` — and the rows left forbidden fall to 35 720 and 16 859.

## 2. Decision

**The planner asks the backend of fills too.** `pdf_render::PlannedMark` is a fill or a stroke;
`PlannedFill` states the path, its path-to-page-pixels transform, its device extent and whether it
replaces what is below it. A fill already inside forbidden rows is not asked. A fill answered `false`
keeps ADR 0139's rows exactly.

**`render-cpu` answers `true` for a fill where no road of `draw_fill` leaves `area`** — anti-aliasing
on, not replacing, its extent inside `SUPERSAMPLED_LIMIT` before and after any strip's shift, which
are the stroke's own conditions — and its sub-pixel bands and collapsed subpaths are shapes the same
`scan::fill` draws. A rectangle is not affected: ADR 0139 never forbade one.

**A mark's suspicion of overlap is its own, not its surface's.** The division test found it, at 36
levels on five pixels of `bug1820909.1.pdf` beside a cut, 16 on `bug2013793.pdf` and 28 on
`issue19802.pdf`: `area` suspects a mark of overlapping itself *as a whole* (ADR 1347 — an overlap
confined to pixels a path partly covers shows in no sign of its own row) and settles every row as the
filled set where any row shows it. A strip holding only a glyph's rows that show nothing measured
them as the winding integral where the page, seeing the overlap in another row, measured the set.
A stroke's outline is suspected always (ADR 1341), which is why ADR 1819 never met this.
`area::fill` now also accumulates the rows its surface clamped away (`suspected_beyond`), so the
answer is the mark's in every division and on the whole page, whose edge clamps a mark as a strip's
does. It is bounded by the cell budget: a mark whose whole span passes it keeps the suspicion of its
own runs (ADR 1819 §5), and the planner keeps such a fill's rows forbidden (`whole_span_fits`).

**A freed row is a permission, not a place a cut must go.** The split balances cost and is blind to
what a cut crosses, and `replay_ratio` counts commands: freed, the cuts of a page of a few large
curved fills run through them, and the bound refused every count down to one — `shading_extend.pdf`
went from sixteen strips to one at 150 dpi, and 137 pages lost strips. And the estimate's balance
can make fewer strips through the freed rows than between the fills: the witness drew in 2.25 s on
eight strips against HEAD's 2.11 s on nine (sixteen threads, medians of three). So every count is
asked of both — the freed rows, and their union with the rows the freed fills would have forbidden,
which is the division before this ADR, both from one walk (`unsplittable_rows_and_freed_fills`) —
and the plan kept is the one of most strips either affords within the bound, the freed one on a
tie; the search stops once a division has as many strips as the next count would ask for. So no
page is drawn in fewer strips than HEAD drew it.

## 3. Measured

Every figure under the heavy-walk lock; the planner's own figures from `stroke_cut_census --corpus`
over the 957 first pages the tracked corpus plans, HEAD's rule against this one:

| | HEAD | this ADR |
|---|---|---|
| rows forbidden, summed, at 150 dpi | 128 609 | **35 720** |
| the same at 1× | 61 511 | **16 859** |
| pages gaining strips / losing any, at 150 dpi | | 42 / 0 |
| the same at 1× | | 46 / 0 |
| pages drawn as one strip, 150 dpi / 1× | 612 / 647 | 612 / 646 |

Before the plan asked both rows at every count, four pages a scale lost a strip each
(`issue19971.pdf` 16 → 15, `bug1771477.pdf` 14 → 11 at 150 dpi) and the witness nine → eight;
none does now. The witness grants 1340 of 3509 rows against 797, and keeps HEAD's nine strips at
the same cuts: its rows are forbidden by strokes and clips, not fills (section 6).

**The clock**, `--clock`, sixteen rayon threads, five interleaved draws of each page in the plan's
strips, medians (the page drawn whole beside it):

| page | HEAD | this ADR | whole |
|---|---|---|---|
| `tracemonkey.pdf` at 4× | 4 strips, 33.7 ms | **16 strips, 18.3 ms** | 62.5 ms |
| `tracemonkey.pdf` at 150 dpi | 4 strips, 17.8 ms | **13 strips, 11.7 ms** | 36.5 ms |
| `bug946506.pdf` at 4× | 2 strips, 71.6 ms | **16 strips, 34.2 ms** | 72.5 ms |
| `issue6387.pdf` at 4× | 2 strips, 34.1 ms | **4 strips, 20.9 ms** | 33.3 ms |
| `issue840.pdf` at 4× | 11 strips, 33.6 ms | **16 strips, 29.6 ms** | 106.3 ms |
| `ZapfDingbats.pdf` at 4× | 12 strips, 16.2 ms | 16 strips, 14.5 ms | 37.4 ms |

The witness at 150 dpi, three interleaved: the same nine strips at the same cuts, 1.98 and 2.14 s
against 2.04, 2.32 and 2.77 s (HEAD's second draw, 9.42 s, met a neighbour); planning 266 and 269
ms against 234 and 241 ms — the freed fills' rows cost the one walk nothing measurable.

**Pixels.** The golden, one strip: **held 974, moved 0** — the suspicion change moves no first page
drawn whole. The division test: 175 pages move at some division (173 in ADR 1819), at most 940 pixels
and 3 levels; `comments.pdf`, `highlights.pdf` and `issue7014.pdf` pass their ceilings by two or
three pixels, every one a level beside a cut a glyph now grants — painted and looked at: scattered
single pixels on glyph edges, ADR 1758's shifted origin — and their ceilings are raised to 115, 115
and 329. **The six arms** (an export of HEAD with this patch alone, its own target, against
`/home/AI/arms-1496/`): every arm exit 0, **0 frame digests moved**, the same pages in each; the
oracle's mean moved on 3 to 12 pages an arm by at most 0.0008 (`issue14117.pdf`).

## 4. Calibrated (trap 13)

`stroke_cuts.rs` gains a sheet of discs (cubics) and one of diamonds (oblique edges at dyadic
coordinates), each stacked so that every row crosses a curve or an oblique edge — asserted one strip
under ADR 0139's rule and cut under this one — held inside ADR 1758's bound for one coverage at 2,
3, 5, 8 and 16 strips; and a page of six large discs whose freed cuts replay past the bound, which
must be cut in the five gaps between them. Planted: `draw_fill`'s last draw handed to `tiny-skia`
fails the discs at 2 strips by **16 levels** on 66 pixels, the row of the cut. `suspected_beyond`
removed reproduces the 36 levels on `bug1820909.1.pdf`.

## 5. The oracle movement round 1491 left, read

Round 1491's arms saw the gpu lane's mean error against the CPU oracle move away on 9 of its 12
moved pages at 4×, worst `issue8702.pdf` 0.0357 → 0.0584. **It is the lane's, and the oracle moved
to the clause.** At 4× (2880 × 2160) that page states a white `re S` frame round its black page,
1.92 wide, whose outline's region on the page is 2881 × 2160 cells, past `area`'s budget of 2²²:
before ADR 1819 the page was one strip and `tiny-skia` supersampled the frame; since, `Region::runs`
measures it. Its
inner edge lies at device y = 3.824, so row 3 holds 0.176 of the white: the oracle now writes
level 45, as the cpu and compute lanes do, and the gpu lane writes 63 — a quarter of a pixel, four
sub-rows sampling one edge, exactly what `tiny-skia`'s supersampler wrote. Every one of the gpu
lane's extra differing pixels (10 052, the frame's inner edge on all four sides) is that, and they
are the whole of the move: 0.0584 − 0.0357 = 0.0227, the cpu lane's figure and the gpu lane's having
exchanged. §10.7.4: the coordinates are "mapped into device space but not rounded to device pixel
boundaries", and a quarter-pixel lattice is a rounding; the lane's figure measures its own sampling.
The same reading holds wherever a mark past the budget at 4× has an axis-aligned edge.

## 6. Left

- With every fill freed, 2169 of the witness's 3509 rows at 150 dpi stay forbidden, by the
  strokes the backend refuses and by clips. A stroke under one level of a pixel is the first of
  those: `tiny-skia`'s hairline draws it. ADR 1829's closed form for a
  sub-pixel stroke computes each pixel's area from device coordinates, so a stroke it draws forbids
  no row either; asking `pdf_render::thin_segment` in `drawn_row_by_row` is the next lever, once
  the classification is decided from the mark and the page's transform alone (trap 146).
- A fill too tall for one accumulator keeps its rows, and a clip chain its segments' rows.

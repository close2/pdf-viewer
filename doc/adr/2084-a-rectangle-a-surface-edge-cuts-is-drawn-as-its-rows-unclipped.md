# 2084 — A rectangle a surface's edge cuts is drawn as its rows unclipped

Session 1624, slot 2 of batch ninety-eight, a pixels round under the project owner's switch to code
improvements and bug fixes of 2026-10-14. Status: **accepted** and **built** in `render-cpu`.
Clause: none moves — §10.7.4's row stays `departed` (ADRs 1060, 1360, 2058); what is built is that a
strip, a window of the page under §10.7.4's half-open pixel square, states a rectangle's rows as the
page does. Context: ADRs 0139, 0219, 0476, 1758, 1837, 1896, 2058, 2068 §5; traps 1, 13, 134, 214,
216. Code: `render_cpu::scan::surface_edge` (new), `scan::fill_rectangles`. Tests:
`render-cpu/tests/rectangle_across_a_cut.rs` (six, new), `render-cpu/tests/page_past_the_tile.rs`
(its residue taken out).

## 1. The premise, and the hypothesis that did not hold

**Premise (ADR 2068 §5, trap 216): held.** A rectangle across a cut is a level from the page drawn
in one strip, in the row beside the cut, on any page; ADR 0139 measured an axis-aligned cut exact
on `tiny-skia`'s *path* converter and ADR 0476 sent rectangles to its *rectangle* converter.

**Hypothesis — that the converter rounds the cut row at 8.8 where the closed form gives the exact
fraction, so the closed form at a cut removes the level — did not hold, in both halves.** One
rectangle, `5.375, 19.25 – 40.5, 30.625` on a 64 × 64 surface, cut at row 20, printed at row 19:

| pixel | one strip | cut at 20 | §10.7.4's area × 255 |
|---|---|---|---|
| (5, 19), the corner | 120 | 119 | 119.531 |
| (6, 19), interior | 192 | 191 | 191.250 |
| (40, 19), the corner | 96 | 95 | 95.625 |

The cut at 30 moves row 30 the same way (160 → 159 against 159.375). The mechanism is not a
rounding of the clipped edge: the converter clips the rectangle to the surface first, and a
rectangle with one row left inside goes down its one-row branch, which states that row as
`bottom − top − 1` and its corners as `(alpha × overlap) >> 8`, where the unclipped rectangle's same
row is `256 − top` and each corner its overlap. A cut leaving two rows or more on each side draws
the page's bytes (rows 21 to 29 measured). And the closed form is neither drawing's value: 191.25
rounds to 191, the page states 192 — written at a cut it would put the strip a level from the page
the other way.

## 2. Three constructions, two measured

1. **The closed form at a cut** (`scan::mask_rectangle`, as ADR 2058 uses past 8191). Refused by
   the table above: equal to the page only if every rectangle of the page were the closed form,
   which moves every rectangle's edge on every page and gives up ADR 0476's measured saving.
2. **The cut kept off a rectangle's edge rows** — `pdf_render`'s planner forbidding the row inside
   each horizontal edge of every fill that `device_rectangles` names and of every stroke whose
   outline is one, with a margin for the 8.8 rounding. Built and measured: every division of a
   page of rectangles and of rules was then its own strips to the byte. But the rows are forbidden
   for every caller of `strip_inputs`, and one of them cannot move its cuts: `Underway::in_strips`
   cuts evenly before the page is read (ADR 1896). Over the tracked corpus at four strips, the
   first pages it gives up for "a mark crosses a row the page was cut at" went from 19 to 54 —
   **35 more of 974 first pages lose their drawing ahead**, 610 → 575 finished in strips — for a
   level. Taken back.
3. **The rows the edge leaves few of, drawn beyond the edge** — chosen. Where a surface's edge
   leaves a rectangle two rows or fewer inside the surface and the rectangle reaches past it, those
   rows are copied into a surface with one more row past each edge the rectangle crosses, the
   rectangle is drawn there by the same converter with the same paint, and the rows are copied
   back. The rectangle is unclipped at the rows the strip holds, so the converter states them by its
   own arithmetic exactly as it does on the page; nothing of its arithmetic is restated here, which
   is why ADR 0476's refusal to match its 8.8 rounding ("What was declined") is not reached. It costs no row of any
   division and no drawing ahead, and covers every rectangle the converter draws — a fill, a
   stroke's rectangular outline, any number of dashes — because it acts where the converter is
   called rather than where the planner guesses it will be.

Two rows rather than one because the branch is taken on the rectangle's extent stated to 1/256, and
the second row is the margin over that rounding; a remnant of two rows drawn either way is the same
bytes, so the margin costs a copy and never a pixel.

## 3. What it changes that is not a cut

**A page's own edge is a surface's edge**, so a rectangle with two rows or fewer inside the page and
the rest outside it is now stated at those rows as the same rectangle moved inside the page states
them — §10.7.4's pixel is the same square wherever the page's edge falls —
`a_rectangle_one_row_inside_the_page_is_its_row_unclipped` holds it. Every division states those
rows alike, so the property is the page's, not a strip's; and a window of the page (ADR 1930) and a
group's own surface take the same road at their edges.

**A shader is moved by the whole rows the scratch surface starts at**, as a strip's origin moves it
(ADR 0219): a solid paint is untouched, and a gradient or an image on a rectangle at a cut carries
ADR 0219's residue, which every mark in a strip below the first already carries.

## 4. Measured

On the round's export of HEAD plus this change alone (ADR 1890):

- **The six arms** against `/home/AI/arms-1623/`: 0 frame digests differ in any arm (968, 968, 968,
  965, 965, 965 pages) — the device and compute lanes draw no rectangle through this converter.
  The mean error against the oracle, which is this backend at sixteen strips, moves by at most
  0.0012 on 4 to 6 pages an arm, the oracle's own pixels beside its cuts: at 1× `160F-2019`,
  `annotation-underline`, `bug1743245`, `bug1019475_1`, `canvas`, `issue6106`, `issue840`,
  `prefilled_f1040`; at 4× `bug1799927`, `issue13242`, `issue20324`, `issue5501`, `issue840`,
  `prefilled_f1040`, `160F-2019`.
- **Those pages drawn whole and in 2, 4 and 16 strips** (`stroke_cut_census --draw`), pixels that
  differ from the page in one strip at sixteen, HEAD → this, every one a level: `annotation-underline`
  246 → 0, `issue6106` 7 → 0, `160F-2019` 84 → 13, `bug1019475_1` 163 → 19, `canvas` 283 → 43,
  `issue840` 333 → 44 (4×: 1 736 → 769), `prefilled_f1040` 1 308 → 943 (4×: 3 599 → 2 074),
  `issue13242` 98 → 65 (4×: 3 206 → 1 424), `issue20324` at 4× 2 100 → 9, `issue5501` at 4× 14 → 1;
  of those within two rows of a cut, `prefilled_f1040` at 4× 1 562 → 37 and `issue20324` 2 091 → 0.
  What is left is ADR 0219's shifted origin, away from the cuts.
- **`raster_golden`**: held 973, moved 1 — `bug1743245.pdf` p1 at one strip, 424 pixels of row 0 a
  level darker, a grid rule with one row inside the page's top edge now stated as the rule unclipped
  states it (§3); looked at, the page is the grid it was. Its division census: 176 pages moving at
  some division, at most 1 308 pixels and 3 levels → **173 pages, at most 943 pixels, 3 levels**;
  `issue14117.pdf` left `PAST_THE_BOUND` (32 pixels, one level, inside both bounds).
- **`Underway::in_strips`** over the corpus at four strips: 610 first pages finished in strips, 19
  given up for a mark across a cut — ADR 1896's figures, unchanged.
- **`window_against_whole`** on the owner's witness, 1200 × 900, at 1.25⁸ and 1.25¹⁰, HEAD and this
  interleaved in one `--clock` hold: every whole page's digest HEAD's; the window's HEAD's but at
  1.25⁸ on 24 threads, where the window's 1 726 pixels differing from the page became 1 724. The
  whole page, HEAD → this, two runs each: 1.25⁸ at 4 threads 1 780–2 279 → 1 698–1 762 ms, at 24
  809–847 → 819–952 ms; 1.25¹⁰ at 4 threads 4 490–4 651 → 4 472–4 751 ms, at 24 2 003–2 069 →
  1 979–2 048 ms — no difference the pairs can tell apart.

## 5. What it leaves

- **The rectangle converter's one-row branch on a rectangle the page itself holds in one row.** A
  rule thinner than a pixel inside one row is stated `bottom − top − 1` on the page as well, under
  §10.7.4's area — measured on the library alone, 75 for an area of 76.5 at `19.6–19.9`, 127 for
  127.5 at `19.25–19.75`, 191 for 191.25 at `19.125–19.875`. Coming out under the area is what
  §10.7.4's third sentence forbids; every division states it alike, so it is no division's defect,
  and it is left to a round that owns sub-pixel rectangles (`doc/todo/11`).
- `the_library_states_one_row_left_inside_a_surface_a_level_from_the_row_unclipped` holds the
  library's half on the library alone: a release whose clipped row agrees fails it, and the
  construction may then go.

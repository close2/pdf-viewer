# 1373 — The walk's scaffolding is measured step by step, and no cheaper detector is exact

Status: accepted. Session 1268. Extends ADR 1359 section 2 (the floor), which stands.
Context: ISO 32000-2 §10.7.4, §8.5.3.3, §11.6.2; CLAUDE.md principle 2 and its rule on
optimisation; ADRs 1341, 1347, 1359; habits 45, 47, 52; traps 50, 54.
Code: `crates/render-cpu/src/area.rs` (`Accumulator::measure_overlapping_rows` — its comment cites
this record; nothing else changed).

## 1. The ladder

ADR 1359 closed on "going lower needs a construction that knows where a mark overlaps without
clustering its rows". Before building one, the walk was switched off one step at a time (habit 52):
each arm HEAD with `measure_overlapping_rows` returning after the named step, one export and one
target directory, binaries `md5sum`-distinct (trap 50); `callgrind_rasterise`,
`RAYON_NUM_THREADS=1`, five draws, millions of instructions.

| step | `issue14415` | `issue19802` | `issue20232` |
|---|---|---|---|
| walk off (the row test only) | 1 534.1 | 898.2 | 281.0 |
| + `trace` into lines | +9.6 | +37.4 | +3.4 |
| + `Edge::sort_into` | +17.4 | +37.6 | +5.1 |
| + `Edge::by_row` | +15.7 | +49.7 | +5.4 |
| + `RowScratch::gather`, every row | +25.1 | +61.2 | +19.6 |
| + `RowScratch::cluster` | +25.8 | +58.4 | +17.2 |
| + `needs_walk` and `walk` | +119.5 | +123.0 | +62.5 |
| + `rewrite` (HEAD) | +7.3 | +5.4 | +26.9 |
| HEAD | 1 754.6 | 1 271.0 | 421.0 |

No step dominates. On `issue19802.pdf` — 689 marks a draw, every one a glyph about ten pixels tall
whose contours overlap, none a stroker's outline — the scaffolding before the walk is 244 M and the
walk 128 M; on the two stroke pages the walk is most of it, and on `issue20232.pdf` 11 529 of the
11 599 clusters walked are rewritten.

## 2. The detectors, priced

**A second accumulator marking pixels two chains touch** (the contract's (b)): exact — a pixel one
strand touches takes two adjacent windings, which both rules read exactly from the integral — and
built as a chain number per cell written in `Accumulator::span`, the result not yet used. Its cost
alone: page 101 +7.2%, `issue12295.pdf` +4.0%, `issue19802.pdf` +3.0%, `issue20232.pdf` +3.6%,
`issue14415.pdf` +1.3%. It runs on every mark, because a mark is known to be suspect only after it
is accumulated; the most it could save is `gather` and `cluster` on unmarked rows, at most 119.6 M
(9.4%) on the page that pays most, and nothing on page 101. Rejected.

**Row sums alone** cannot be exact: two edges of one sign each crossing a sliver of a pixel sum
below one whole winding, which is trap 54 again. **A sweep over each mark's edges** (the contract's
(a)) needs each edge's column span in each row it crosses, which is what `gather` computes; it moves
the work rather than removing it.

**Fewer entries in the cluster sort**: consecutive pieces continuing one chain entered as one run.
Exact — the shared vertex puts both in one cluster already — and measured: `issue14415` −0.02%,
`issue19802` −0.41%, `issue20232` +1.49%, page 101 0.00%, `issue12295` +0.34% (a page that walks
nothing, so layout: habit 45). Its bookkeeping costs a page of short straight strokes more than the
sort saves. Rejected.

## 3. What stands

ADR 1359's floor, now with its parts stated. No exact detector found here costs less than the
clustering it would skip. What could lower the floor is a change to what is measured, not to how it
is detected: `trace`, `sort_into` and `by_row` together are 125 M on `issue19802.pdf` because a
suspect mark is flattened a second time. ADR 1359 tried tracing once for both constructions and
measured page 101 +2.1%.

# 2253 — No strip finds a folding stroke's boundary twice, and a pool-width count carries rayon's idle loop

Status: accepted for integration. Session 1709, 2026-10-23, the pixels slot of batch one hundred and
nine, beside ADR 2252. Clause: ISO 32000-2 §10.7.4. Context: ADRs 0139, 1819, 1847, 1859, 2122,
2164; `doc/questions/A559`; habit 164.

## 1. The lever as it was named

`doc/questions/A559` ordered, with the chords at a twentieth, "one boundary per mark shared between
the strips that cross it", because ADR 2164 §4 measured the walk at the pool's width, twenty-four
threads, at +21.8 %, +63.6 % and +6.9 % on `inks.pdf`, `rotated_ink.pdf` and `issue14415.pdf`, and
read the difference from the one-thread figures as each strip finding a mark's boundary again.

## 2. Priced at its ceiling, it is zero

Habit 164 asks what the lever can remove before it is built. A boundary found again is
`area::union`'s pair comparisons run again, so the inclusive count of `Pairs::compare` under
`callgrind_rasterise` (page 1, three draws, scale 1) says how often a mark's boundary is found:

| page | `Pairs::compare`, one thread | the same, twenty-four threads, four runs |
|---|---|---|
| `inks.pdf` | 80 878 660 | 81 154 243, 81 579 928, 81 154 259, 81 078 644 |
| `rotated_ink.pdf` | 2 742 600 | 2 726 695, 2 760 432, 2 760 432, 2 726 695 |
| `issue14415.pdf` | 88 838 024 | 88 895 192, 88 890 942, 88 891 593, 88 894 314 |

The same work at both widths: every mark's boundary is found once a draw. The reason is the
planner's, and it holds by construction rather than by these pages: a stroke a curve of which may
fold is not drawn a row at a time (`strips::drawn_row_by_row`, `stroke::outlined_whole`), so its
whole extent forbids a cut (ADR 1819), and a strip draws a mark only where that same extent reaches
its rows — the one strip that holds them. `scan::mask_fill`'s routing (ADR 2252) does not change
that answer: the walk declines a mark it cannot settle to `tiny-skia`'s converter, which a cut
would change, and which marks it declines is not known before they are drawn.

## 3. What the pool-width figures measured instead

At twenty-four threads the total carries rayon's idle loop — `WorkerThread::wait_until_cold`,
36 to 41 M of `rotated_ink.pdf`'s 43 to 49 M — which is timing, not work: HEAD alone read
44 071 654, 48 694 982, 44 636 216 and 42 742 356 over four runs, 13 % apart, where one thread
repeats to the instruction. The walk adds the same work at both widths (`scan::mask_fill` +1.82 M
at one thread on `rotated_ink.pdf`, +1.74 M at twenty-four, measured before ADR 2252 §1's joins).
Over four runs a page the means are ADR 2252 §3's: every one under the fifth.

## 4. Decision

The lever is not built: there is nothing for it to share. It becomes real only when a folding
stroke's rows are freed for a cut — which needs the walk's declines settled before the draw, or a
decline drawn row by row — and the round that frees them builds one boundary per mark, shared by
the strips that cross it, and prices it then. A figure quoted at the pool's width is the mean of
several runs with their spread, or the render work (`CpuRasterizer::encode_page` inclusive) beside
the total; a single pool-width total is a sentence about the scheduler.

# 2164 — A folding stroke's exact measure costs its overlapping pairs, and is held past the fifth

Session 1664, the pixels slot of batch one hundred and three, under the project owner's answer
`doc/questions/A538` ("agreed with wait"): the exact measure of ADR 2122 is routed once it costs under
a fifth more on `inks.pdf`, `rotated_ink.pdf` and `issue14415.pdf`, by the levers ADR 2122 §5 named.
Status: **proposed** — both constructions built and measured exact, priced past the fifth, and held
out of the tree; the lever that reaches the fifth is the owner's to take (`doc/questions/Q559`).
Clause: ISO 32000-2 §10.7.4, with §8.5.3.3.2's non-zero rule and §10.7.2's flatness; the row stays
`departed` (ADRs 1060, 1360). Context: ADRs 1348, 1374, 2106, 2122, 2133.

## 1. The patch at HEAD

ADR 2122's patch is rebased onto `66c6ef23`: ADRs 2132, 2133 and 2148 moved the routing into
`scan::mask_fill`'s `Place`, so the boundary is found and measured on `place.grid` with the region
written at `place.corner`, and nothing else in it changed. Clippy and rustfmt clean, `render-cpu`
nextest 236 of 236, the ladder test green (the paperclip's pixel 243 against 242.3).

`callgrind_rasterise`, page 1, three draws, scale 1, `RAYON_NUM_THREADS=1` — the setting that
reproduces ADR 2122 §5's figures to a tenth of a percent:

| page | HEAD | the boundary walk (ADR 2122) | the clipped pieces (§3) |
|---|---|---|---|
| `inks.pdf` | 283 228 522 | 694 658 593, +145.3% | 933 072 625, +229.4% |
| `rotated_ink.pdf` | 16 958 381 | 24 692 616, +45.6% | 29 958 783, +76.7% |
| `issue14415.pdf` | 985 610 733 | 1 151 226 506, +16.8% | 1 275 196 858, +29.4% |

On the pool's own width, twenty-four threads, the walk is +144.9%, +95.5% and +12.9% against HEAD's
302 015 255, 28 869 491 and 1 383 250 540: a strip that a mark crosses finds the mark's whole
boundary again, which on `rotated_ink.pdf` is the walk's cost three and a half times.

## 2. The lever named does not reach the fifth

Under the fifth the measure may cost `inks.pdf` 67.9 million instructions a draw: the 49 million the
library's sixteenths cost now (ADR 2122 §5) and a fifth of the page's 94.4 million. The walk costs
186 million. The spatial bucket the brief named is ADR 2122 §3's grid of eight-edge chunks, already
in the patch; and the crossings that bound nothing are discarded only after their pair has been
compared, where `Pairs::compare` alone is 80.3 million instructions a draw — more than the whole
measure may cost. No discard after the comparison can bring it under.

## 3. The lever taken to its end: the pieces clipped, no crossing between two of them recorded

Nearly every contour of the three pages is a convex piece — 2 219 of 2 219 on one `inks.pdf` mark,
558 of 561 on its heaviest — and a piece's inside is the intersection of the half-planes its edges
bound. So the second construction clips every edge against the pieces whose boxes meet it, one
interval a piece, finishes an edge one piece holds, and keeps crossings and a carried winding only
for the rest, the stroker's outlines of the stretches between folds. Exact on the grid of `2^-21`,
ties by simulation of simplicity; two pieces' edges along one line are settled by direction and index.
Probed against the set walk (`crate::area::fill`, every row settled) on every mark of the three pages
at 1x and 4x that both measure, it is within one level at every pixel (habit 148). Two defects the
probe found are fixed in it: a stroker's outline whose lobe winds against the pieces cancels them
under the non-zero rule, so a piece holding a point does not by itself put it in the set (255 levels
on `issue14415.pdf`), and a piece's edge along an outline's edge was settled by two tie rules (19
levels). What it still declines is that last case — 44 of 175 marks on `issue14415.pdf` at 1x, 4 of 15
on `inks.pdf` — because the simulation states no position for a crossing along a common line.

It costs more (§1's table): about 300 000 clips an `inks.pdf` draw, ten an edge, 116 million
instructions, against the walk's 247 000 crossing records. **The cost of the exact measure is the
pairs of portions that overlap, not the records of their crossings**: removing every record between
two pieces removed nothing that mattered.

## 4. What reaches it: fewer pairs, from coarser chords

Fewer overlapping pairs is the pieces' construction (`stroke/folding.rs`, ADRs 1348 and 1374) rather
than the measure: a fold's pieces are the bands of chords flattened to a sixty-fourth of a pixel
(`FOLD_FLATNESS`), chosen against the sixteenths' step, so a band overlaps the bands of every chord
within its half-width either side. Measured in a scratch tree, the walk of §1 with only that constant
changed, one thread (`doc/questions/Q559` has the table): at 1/32 +55.2%, +22.5%, +10.4%; at 1/24
+29.9%, +14.9%, +8.9%; at 1/20 +14.5%, +7.7%, +7.8%; at 1/16 +2.7%, +4.1%, +6.4%. The arcs of the
joins (`ARC_FLATNESS`) move the price by about a percent at most. The ladder's pixel, which is held to
the curve and not to the flattened set, reads 245 at 1/20 against the curve's 242.4: the chords' own
error, which §10.7.2 leaves to the processor and which today's sixteenths exceed (255).

At the pool's width the strips each find the boundary again — at 1/20 +21.8%, +63.6% and +6.9% — so
one boundary per mark, shared by the strips that cross it, is the second lever.

The other construction tried does not reach it: halving every curve that may fold until the halves
that do not are the stroker's (ADR 1359) gives those halves to the stroker's outlines, flattened at
sixteen times the device's resolution, and `inks.pdf` cost 87 755 715 294 instructions, the outlines'
pairs.

## 5. Decision

Not routed: the sixteenths stay the converter for ADR 1348's pieces, and the row's note says the
levers were measured. Both patches are outside the tree at `/home/AI/r1664-folding-union/`, each
applying to `66c6ef23` with its ladder test: `folding-union-boundary.patch` (ADR 2122's, rebased) and
`folding-union-clipped-pieces.patch` (§3). Coarser chords change what the ladder gate measures and
move every folding stroke's rim, so they wait on the owner's word (`doc/questions/Q559`), with the
recommendation that the walk at 1/20 and a boundary shared between strips be routed together.

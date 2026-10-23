# 2122 — A folding stroke measured from its union's boundary is exact, and is priced past the fifth

Session 1643, the clause slot of batch one hundred, under the project owner's switch to code
improvements and bug fixes of 2026-10-14. Status: **proposed** — built, measured exact, priced, and
held out of the tree: the price is past the fifth the brief set, and the owner chose to wait for a
cheaper measure (`doc/questions/A538`). Rebased onto `66c6ef23` it prices +145.3%, +45.6% and +16.8%,
and the levers §5 names do not bring it under the fifth; the patch is now at
`/home/AI/r1664-folding-union/` (ADR 2164). Routed with chords at a twentieth by ADR 2252. Clause: ISO 32000-2
§10.7.4, with §8.5.3.3.2's non-zero rule; the row stays `departed` (ADRs 1060, 1360) and its note
states the bound the sixteenths give. Context: ADRs 0219, 1082, 1341, 1347, 1348, 2058, 2106.

## 1. What is owed, and what the tree pays now

ADR 1348 states a stroke that bends more tightly than its half-width as a union of convex pieces
wound one way — every chord's band, every vertex's join, the stroker's own outline between the
folds — and §10.7.4 owes each pixel the integral of the set the non-zero rule declares inside.
`crate::area` reads a path as the integral of its winding number and settles the rows where that
is not the set's area by a second measurement, pair by pair; the pieces overlap almost everywhere,
so that measurement costs `inks.pdf` 110 times its instructions (ADR 2106 §3), and the tree hands
the pieces to `tiny-skia`'s supersampled converter instead. That converter places the boundary at
four sample lines a pixel, each standing for a quarter of the pixel's height, and to a quarter of a
pixel along each line, so the set it measures lies within a quarter of a pixel of the true one and
a pixel is out by at most the area of that band inside it: at `issue14415.pdf` 4x `(1628, 407)` it
reads 0.88 where the integral is 0.683, 50 levels (ADR 2106 §4), and the paperclip of veraPDF's
`6-3-1-t01-fail-f` at 10x `(798, 567)` reads 255 where it is 242.2.

## 2. The hypothesis did not hold

The brief's hypothesis was that accumulating every piece's edges under the non-zero rule into one
coverage buffer is the union's integral. It is the integral of the **winding number** — which is
what `crate::area` already accumulates — and it is the set's area only where a pixel's winding
takes two adjacent values; at a fold's rim a pixel holds a point outside the stroke and a point two
pieces cover, and there the sum overcounts. The cheaper exact measure is the brief's other one:
the union's outline, found once.

## 3. The construction built

`render_cpu::area::union`, about 1 300 lines with its comments, the patch named in the record:

- **The boundary of the filled set is stretches of the pieces' edges.** The windings on the two
  sides of an edge differ by one and change along it only where another edge crosses it, so every
  contour is walked once, the winding beside it carried through its vertices and changed by one at
  each crossing, and a stretch is a boundary edge where one side winds zero and the other does not,
  turned so that its filled side is the side it encloses. The accumulator then measures that
  boundary exactly as it measures any simple path: it winds once round every point of the set.
- **A walk's start** is a winding fixed by a crossing with a contour already walked — the four
  windings round a crossing point follow from any one — or, for the first contour of a cluster, a
  ray from outside the mark to a point on one of its edges, tried at the midpoint and then at
  dyadic points to `2^-8` of the edge where another edge passes through the one tried.
- **Every predicate is exact**: the device coordinates on a grid of `2^-21` pixel, determinants in
  `i128`, and a determinant of zero — a piece's corner on another's edge, which the construction
  makes common — answered by Edelsbrunner and Mücke's simulation of simplicity, so that all answers
  are about one configuration. A crossing's position along an edge is a ratio rounded once in
  `f64`; two crossings closer than that rounding may be passed in either order, which costs the
  stretch between them and nothing else, and a walk never starts from a crossing within `10^-9` of
  another. Two edges crossing along a common line, a walk that does not close, and a mark past its
  work bound are declined to the library's converter, which is a cost guard of the mark alone.
- **Found from the whole mark, never from the region**, so a strip and the page find one boundary.

Two defects were found and fixed on the way, each by a probe that held the boundary to the set walk
on a dumped mark: a ray retried at a second point reused the first's marks and skipped what the
first had counted, and a ray's point on an edge collinear with the reference edge counted the
sliver between them; the dyadic points and a ray of its own per attempt are the two fixes.

## 4. Measured exact

- **Against the set walk** (`crate::area::fill` with every row settled, ADR 1341's construction):
  every mark of `issue14415.pdf`, `inks.pdf` and `rotated_ink.pdf` at 1x and 4x that the walk does
  not decline, 378 marks, within one level of it at every pixel; at `inks.pdf` 4x the walk
  declines five marks past `SET_WORK`, which this measures.
- **Against a ladder**: the paperclip's pixel `(798, 567)` reads 243 where the stroke drawn sixteen
  times finer averages 242.2 over its block, and every pixel of the fold's 8 by 8 window is within
  the flattening's own bound of its ladder. That is the test the patch adds to
  `render-cpu/tests/folding_stroke_under_a_clip.rs`, red on the library's converter (255 against
  242.3) and green with the construction.
- `render-cpu` nextest 224 of 224 with the patch, clippy and rustfmt clean.

## 5. Priced

`callgrind_rasterise`, page 1, three draws, scale 1, HEAD against HEAD with the patch:

| page | HEAD | with the union | |
|---|---|---|---|
| `inks.pdf` | 281 870 234 | 694 269 784 | +146.3% |
| `rotated_ink.pdf` | 16 958 633 | 24 674 242 | +45.5% |
| `issue14415.pdf` | 984 881 667 | 1 150 169 962 | +16.8% |

Under a fifth on one page of three. On `inks.pdf` the union costs 186 million instructions a draw
where the library's sixteenths cost 49 million: the pair comparisons 45%, ordering the crossings
along each edge 23%, the walks 14%. Two of its fifteen marks are folds of 1 612 and 2 719 vertices
whose pieces cross 40 and 27 times an edge, and they hold 137 000 of the 247 000 crossing records a
draw. Three levers were taken and measured — the exact determinant before the simulation, the
`i128` ratio converted from its halves, and an insertion sort for an edge's handful of crossings —
and a fourth, chunks of four edges for eight, changed nothing.

## 6. Decision

The construction is not routed: the sixteenths stay the converter for ADR 1348's pieces, as ADR
1348 decided for cost, and the row's note says what bound they give. The owner answers Q538 —
whether §10.7.4's integral on a folding stroke is worth this price now, or waits for a cheaper
measure. A later round that takes the construction up starts from the patch and from §5's split:
what it has to remove is the crossings that bound nothing: at least 43% of `inks.pdf`'s, whose
124 000 crossings a draw can end at most the 70 000 ends of the boundary's 35 000 stretches.

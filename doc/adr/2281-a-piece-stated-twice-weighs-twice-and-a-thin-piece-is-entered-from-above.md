# 2281 — A piece stated twice weighs twice, and a thin piece is entered from above

Status: accepted for integration. Session 1723, 2026-10-23, the pixels slot of batch one hundred and
eleven. Clause: ISO 32000-2 §10.7.4, with §8.5.3.3.2's non-zero rule; the row stays `departed` for
its other departures. Context: ADRs 2122, 2164 §3, 2268 §2, 2280.

## 1. The three declines, drawn

ADR 2268 left three marks of the folding pages declined by the walk's entry, at 1x and 4x. Each was
dumped as it reached `Union::settle` and drawn in a window of its page beside the page drawn sixteen
times finer by the library and averaged back (trap 1).

- `annotation-fileattachment.pdf`'s push pin, at 1x and 4x. The premise held in part: the path
  retraces itself, but of the mark's 38 contours at 1x, 11 restate an earlier one vertex for vertex
  and in the same order round, and 25 of 70 at 4x — not every piece. The twins are what no ray can
  start beside: every point of one's edges lies on the other's.
- `inks.pdf`'s at 1x, a mark of 322 pieces. The contour with no start is four vertices, a rectangle
  4.5 pixels long and a hundredth of a pixel tall; its two upright sides lie along the edges of the
  contours before and after it, and its two horizontal edges along nothing.

## 2. A twin is one contour of twice the weight

§8.5.3.3.2 counts a contour's crossings once for every time the path states it, so a contour stated
`k` times changes the winding by `k` wherever it changes it at all. The walk carries that as an
edge's weight: a crossing adds the crossed edge's weight, a ray counts it, the negative side of an
edge winds its weight more than the positive side, and a stretch bounds the set where one of its
two sides winds zero. The winding is then the stated path's at every point off the edges, so the
set, and the boundary the accumulator integrates, are the same.

Leaving a twin out would not be: an outline's lobe wound against the pieces (ADR 2164 §3) winds
`2 − 1` inside a twin and `1 − 1` without it. `a_piece_stated_twice_is_one_piece_of_twice_the_weight`
holds a square stated twice over a square wound the other way to the square alone, to the bit, and
is red with the weight left at one.

Twins are merged only where the walk declined the mark for a contour with no start or a crossing
along a line, and the mark is then walked again, so a mark the first walk measures is measured as it
was. A contour is compared with those before it of the same vertex count and coordinate sums, and
then vertex by vertex from each rotation; past `i8::MAX` statements a twin stays a contour of its own.

## 3. A thin piece is entered from a vertical ray

ADR 2164 §3's clipped pieces measured a piece along another's edge because that construction never
reads a winding from a ray — a convex piece's inside is the intersection of its half-planes, and two
edges along one line are settled by direction and index — at a price of +229% on `inks.pdf`, and it
declined a case of its own. The walk needs only one point per contour that no other edge passes
through, and the `inks.pdf` piece has one on each horizontal edge. So where every point tried on a
contour's upright edges lies on another edge, its horizontal edges are tried from a vertical ray, the
same predicates on the other axis; a mark the horizontal rays enter is entered as before.
`a_thin_piece_between_two_others_is_entered_from_above` holds three rectangles meeting along their
sides, the thin one walked first, to the sum of their areas in every pixel; red without the
vertical ray, `NoStartingPoint`.

## 4. What it measures

Against the page drawn sixteen times finer by the library, on the channel values where the union
and the library's sixteenths differ: the push pin at 1x, 348 values, mean 10.8 levels against 15.4;
at 4x, 1 731 values, 1.04 against 7.46; the worst value of each, 104 and 49 levels, is the pin's
needle under §10.7.4's thinnest-line rule, which no resolution averages back and which both
converters draw alike. The `inks.pdf` mark at 1x, 811 values, mean 0.59 against 4.05, worst 21.9
against 29.9.

## 5. What moves, and what still declines

`raster_golden`: 2 of 974 first pages move, raster only — `annotation-fileattachment.pdf` and
`inks.pdf` — and are re-taken. Against HEAD's six arms, 0 of 5 799 frame digests move (the frames
are `render-raster`'s); the oracle's mean error falls on the nine page-lanes it moves in, the two
pages at 1x and the push pin at 4x on each of the three lanes. A census of what the union declines,
page one of every document of the corpus at 1x and 4x through `CpuRasterizer::declined`, names none;
with both changes taken back it names exactly ADR 2268's three, so the instrument sees them. What a
mark can still be declined for is ADR 2280's `UnionCondition`, said by name — `inks.pdf` drawn at
16x declines one mark of 19 743 pieces past `UNION_WORK`, a cost guard.

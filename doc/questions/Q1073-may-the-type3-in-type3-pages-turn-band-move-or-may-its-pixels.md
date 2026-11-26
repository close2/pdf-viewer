# Q1073 — The Type 3-in-Type 3 page now costs seconds to measure exactly: may its band move, or may its pixels?

Source: round 2251 (ADR 3705), after round 2238 (ADR 3653).

## What the tree does now

ADR 3653 placed a pattern a Type 3 glyph paints with in the glyph's space (§8.7.2). On
`ContentStreamNoCycleType3insideType3.pdf` p1 the inner glyph's tiling cell became about 2 pt by
1.15 pt, and the page went from 152 commands to 12 224: 3 054 cells, each a Helvetica "ba" filled
and stroked (text rendering mode 2). Each stroke is a glyph about two device pixels high whose
outline expands into some 1 200 overlapping pieces, met with the glyph clip that bounds the tiling.

Both exact rasterisers measure that union exactly, as §10.7.4 and ADRs 1467 and 2122 ask: raster
by `area_in_pixel` where the mark and the clip both cut a pixel, the CPU oracle by `area::union`'s
boundary of the pieces. Round 2251 made raster's measurement compare only the edges that overlap
in height (ADR 3705), with every digest unchanged: its frame at zoom 1 fell from about 6 s to about
2.5 s. The CPU oracle draws the page in 5 to 10 s; `render-gpu` (vello, which estimates coverage)
in about 130 ms. The turn band is 20–33 ms.

## The ask

The rest of the cost is intrinsic to measuring a thousand overlapping pieces exactly in a handful
of pixels. Three ways on, and each is the owner's to choose:

1. **Move this page's band** to what the exact construction costs, with the figures as proof that
   the page's work grew (eighty times the commands, each a mark of a thousand pieces).
2. **Let a mark this small be measured less exactly**: below some device size, a stroke's union
   is drawn by the accumulating converter (the CPU's earlier path) and raster keeps `min` at the
   meet, which §10.7.4 permits ("at least as large as the area of the original shape") but moves
   this page's pixels and two `raster_golden` entries.
3. **Flatten by device pixels alone below a size**: `RELATIVE_FLATTEN_TOLERANCE` (ADR 0044) keeps
   sixteen pieces a turn however small the curve; at two pixels that is what makes a thousand
   pieces. Moves pixels on every page with sub-pixel curves.

The round's suggestion is 1 for now, with 2 as the construction to price next.

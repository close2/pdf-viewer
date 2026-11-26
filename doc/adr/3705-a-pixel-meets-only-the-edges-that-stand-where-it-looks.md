# 3705 — A pixel's exact meet compares only the edges that stand at the height it looks at

Session 2251. Status: accepted. Context: the turn gate's `ContentStreamNoCycleType3insideType3.pdf`
p1, outside its band since ADR 3653 placed a Type 3 glyph's tiling pattern in the glyph's space;
ISO 32000-2 §10.7.4; ADR 1467, ADR 1491.

## What the page costs, and where

ADR 3653 made the inner glyph's cell about 2 pt by 1.15 pt, so the page now states 3 054 cells,
each a Helvetica "ba" filled and stroked (text rendering mode 2), 12 224 commands where the band
was taken on 152. Each stroke is a glyph about two device pixels high, expanded into some 1 200
overlapping pieces, met with the glyph clip that bounds the tiling. Where both cut a pixel,
`raster::area_in_pixel` measures the intersection exactly (ADR 1467): it cuts the pixel into bands
at every vertex and every crossing, and it compared **every pair** of the edges through the pixel
for crossings and scanned **every** edge for every band. With a thousand edges in one pixel, each a
few hundredths of a pixel tall, that is quadratic and worse: `exact_areas` was 48 % of the frame,
and the frame 4.9 s at window size.

## Decision

Both loops sweep the edges by their tops. `crossings` compares an edge with the edges still
standing at its top, and `area_in_pixel` keeps, band by band, the edges whose span holds the band's
middle, in `through`'s order. Each pair is computed by the same `crossing` with its operands in the
same order, the cuts are sorted and deduplicated as before, and the band's boundaries reach the
stable sort in the order a full scan met them — so every area is the same bits.
`the_sweeps_measure_the_same_bits_as_every_pair` holds the old construction beside the new one
over dense overlapping pieces under both rules, with and without a clip, and compares bits.

## Cost and benefit

Some forty lines and two buffers in `Work`; the reader of `band_area` reads an `active` list
instead of a filter. On the page, raster's frame at zoom 1 fell from 5.8–7.3 s to 2.5–2.8 s, its
digest unchanged at three zooms, and the turn from 4 910 to 2 282 ms. It does **not** reach the
band: the remainder is each meet's edge buckets and their sorts, intrinsic to measuring a
thousand-piece mark exactly, and the CPU oracle pays the same shape in its union of pieces
(`render-cpu`'s `area::union`, 5–10 s on the page). What the page's work now is, and whether its
band moves, is left to the owner (Q1073).

# 1303 — A substitute is ranked by its descriptor, and one covering search serves page and chrome

Batch forty-seven. ADR 1441.

**Asked**: choose a non-embedded font's face by its font descriptor (§9.8.1, Table 120/121), one
search shared with ADR 1430's chrome, a census of the corpus's substituted fonts, the raster
golden looked at and regenerated, and `chrome_coverage` opening encrypted documents.

**Premises corrected**: ISO 32000-2 has no §9.6.6.4; the common descriptor entries are Table 120,
not 122; Table 111 is the Type 3 operators and the standard states no alias table.

**Built**: `substitute::Style` (weight from a name word, `/FontWeight` on its nine classes, PANOSE,
then `/StemV` by a documented rule — 120 thousandths or more is bold; ForceBold at least bold; width
from `/FontStretch` then a width word); a family's members ranked by their own `OS/2` weight, slope
and width; the covering search ranks repertoire first, then style and generic family, and is the
one both callers ask; the broker's description carries the style (version 2);
`LoadedFont::substitute_program`; `examples/substitution_census`; ten unit tests, the four
fixtures (bold by stem, italic angle, fixed pitch, serif) plus a condensed one each skipping with
ADR 1154's sentence where the machine lacks such a face.

**Measured**: census, 1459 documents, 3216 substituted fonts, 867 with a descriptor —
weight 11, slope 3, spacing 30, serif 44, any 68, each the machine lacking the face or the
descriptor contradicting its own name; unchanged after. Six fonts change face (every Arial Narrow,
to Nimbus Sans Narrow). No corpus font is decided by the `/StemV` rule. First lookup 2.2 ms with
the catalogue walk, a second family 88 µs.

**Gates**: `raster_golden` moved one row, `bug1671312_ArialNarrow.pdf` p1 (raster and list), looked
at against `poppler` and `mupdf` — ink and stem now theirs — and regenerated; oracle passes, the page
stays `ambiguous`; `pdf-model --test corpus`, `text_extraction`, `render-raster --test corpus` at 1×
(961 agree, 0 differ) pass. `substituted_shapes` asks ADR 0358's control only of a normal-width face.

**Left**: no serif CJK face on this machine for a Mincho's Serif flag to choose; `/AvgWidth`,
`/CapHeight`, `/XHeight` still weigh nothing.

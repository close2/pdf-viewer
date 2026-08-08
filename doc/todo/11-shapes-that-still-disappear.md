# Shapes that still disappear

Status: **items 1 and 3 closed in the three-hundred-and-eighty-ninth session (ADR 0226); item 2 is
fixed as far as any corpus document exercises it (ADR 0213) and its general case is unwitnessed.**
What is left of items 1 and 3 is one named limit — a sub-pixel rule that is *diagonal* — and it is
below.
Priority: 11
Corpus: 3 known witnesses; the general shape of the residual is stated
Clauses: §10.7.4 — see `_scan-conversion.md`
Code: `crates/pdf-render/src/sub_pixel.rs`, `crates/render-cpu/src/lib.rs`,
`crates/pdf-model/src/content.rs`'s `tile`, `crates/pdf-render/src/repeat.rs`,
`crates/render-quorra/examples/sub_pixel_marks.rs` (the instrument),
`crates/render-quorra/tests/sub_pixel_coverage.rs` (the gate, on **both** backends since 389)

Leftovers from the hundred-and-eighty-sixth to -eighth sessions, which closed §10.7.4's
"no shape ever disappears" for a fill with *no* area (ADR 0154) and for a redundant pattern-cell
clip (ADR 0155). All three are the same sentence one step along, and none of them is the
anti-aliasing departure.

## 1 and 3. A fill, and a stroke, thinner than the rasteriser's coverage quantum — **closed**

Both were `render-cpu`'s alone, which the three-hundred-and-forty-fourth session measured, and both
were paid in the three-hundred-and-eighty-ninth. `tiny-skia`'s scan converter supersamples four
times per pixel row and takes each sub-row's sample at its centre, so a fill under an eighth of a
pixel crossed no sample line and vanished; and its painter drew a stroke under a pixel wide as a
hairline smeared symmetrically about the path, so one within half a pixel of the raster's edge lost
the half of its smear that fell outside.

`pdf_render::sub_pixel_bands` draws an axis-aligned rectangle thinner than a device pixel as the
whole pixel line it lies in, at the coverage its own area there implies, and a sub-pixel stroke on a
straight axis-aligned rule is converted to the fill of its own outline first. Both backends now
answer within one level of 255 of the shape's own area at every thickness measured, and
`tests/sub_pixel_coverage.rs` gates **both** against the area rather than against each other. The
before/after ladders, the cost, and every declined case are in ADR 0226.

Two consequences worth keeping here:

- **The oracle was the backend being accused.** `render-quorra/tests/corpus.rs` calls a difference
  between the two backends quorra's by construction, so on a page of sub-pixel line work the render
  carrying the right ink was the one on trial. That gate went 914 agree / 42 differ to **920 / 36**,
  and `issue16038.pdf` — a page whose whole subject is a 0.53-pixel rule — moved **6.5359 → 1.8563**.
- **The rule takes a promotion away as often as it gives a mark back.** At 0.9 of a pixel
  `tiny-skia` rounded a rectangle up to a whole row, 11% heavy; `issue8125.pdf` page 1 left the
  oracle's contradicted list because of that half rather than the disappearing half.

### What is left: a sub-pixel rule that is not axis-aligned

`22060_A1_01_Plans.pdf` page 1 is the corpus's largest page of sub-pixel line work — an A1 drawing
that is *all* strokes under a pixel wide — and it is **unmoved to four decimals** by the above, on
both the oracle gate (worst mean 6.09) and the quorra gate (mean 0.7356). Its rules are diagonals
and polylines, and `pdf_render::sub_pixel` declines them by name: the substitution needs a device
pixel *line* to stretch a band into, and the run of pixels a slanted band passes through is a
staircase rather than a rectangle. `pdf_render::collapsed` declines the same case for the same
reason and has since ADR 0154.

**What would answer it is a different construction, not an extension of this one**: a coverage span
per scanline rather than one rectangle per pixel line, which is a scan converter of our own for the
sub-pixel case. Whether that is worth having is an open question and the honest input to it is that
`AMBIGUOUS_SUB_PIXEL_LINE_WORK` places us *between* the reference ladders on every page of the
group where the geometry can be bounded — the departure there is measured and is not a loss.

A thin shape that is not a rectangle at all — a sliver of a triangle, a glyph stem — is declined
deliberately and permanently: its cross-section is not constant along its length, so a single
coverage across a pixel line would be worse than what the rasteriser already does. ADR 0226 argues
it, and small text is the case that makes it a rule rather than a caution.

## 2. Two marks that abut across a cell's box edge without repeating

**The witness closed in the three-hundred-and-seventy-fourth session and this is what is left of
the item.** `issue16038.pdf`'s second square drew a rule its cell states on *both* box edges, so
Table 74's clip halved it and the two halves composited as `1 − (1−a)(1−b)` rather than adding —
0.1159 against the geometry's 0.1333. The two statements are one mark of the tiling, a whole
`/YStep` apart, and §11.6.2 forbids compositing portions of one object; folding them to the one
mark they describe puts the right square within 1% of the left at every scale (ADR 0213).

Three things that were written here and are worth keeping:

- **Removing the clip alone is not the answer.** It draws the rule twice at full width, which is
  what `mupdf` does and what makes its two squares differ by a factor of 1.63.
- **The NOTE that looks like the answer is not one.** §11.6.7's NOTE 2 recommends treating all
  tiles as a single transparency group against "artifacts due to multiple marking of pixels along
  the boundaries between adjacent tiles", and `tile` has built that group since the
  hundred-and-seventeenth session. Compositing inside a group is still compositing; the loss was
  *inside* it. (This file said §8.7.3.1's NOTE 2 for four sessions. The note is §11.6.7's.)
- **The general case is still open, and no page in the corpus names it.** Two *different* marks
  hanging out of opposite edges of the box and meeting at the boundary: the clipped pair is then
  the right set of points, there is no repeat to fold, and joining them would mean either a
  coverage buffer per tiling or a boolean intersection of path against box — the second of which
  would bake a flattening resolution into a display list that has none. `repeated_subpaths`
  refuses the case by name: the fold's condition is that the mark's lattice copy reaching into the
  box is itself stated. What is left over is a seam one boundary pixel wide, which is what this
  item was about all along; `issue16038.pdf` was the family's only witness and its figure repeats,
  so the residual is *unwitnessed* rather than measured-and-left.

There is also a residual on the *witness*, which is not this and is smaller: both squares sit 1.5%
to 3% under the geometry at 2× and 4×, and that is the rules' **ends** abutting column by column —
the same seam one axis over, over one pixel column per three rather than the whole length of every
rule. It is `AMBIGUOUS_TILING_CELL_CLIP`'s own last paragraph.

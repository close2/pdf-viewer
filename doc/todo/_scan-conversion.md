# Shared background: §10.7.4, and what this tree departs from

Not a todo. Referred to by `11-shapes-that-still-disappear.md`,
by `AMBIGUOUS_ZERO_AREA_FILL`, `AMBIGUOUS_TILING_CELL_CLIP`, `AMBIGUOUS_SUB_PIXEL_LINE_WORK` and
`CONTRADICTED_ANTIALIASED_EDGES` in `oracle.rs`, and by the ledger's §10.7.4 row, which is the
authoritative version.

## What the clause says

> A shape shall be scan-converted by painting any pixel whose half-open square region intersects
> the shape, no matter how small the intersection is. This ensures that no shape ever disappears
> as a result of unfavourable placement relative to the device pixel grid, as might happen with
> other possible scan conversion rules. The area covered by painted pixels shall always be at
> least as large as the area of the original shape. This rule applies both to fill operations
> and to strokes with non-zero width.

Read literally, that is **aliased** rendering: a stroke 0.4 of a pixel wide is a solid line, and
Figure 70 draws exactly that.

## What this tree does instead, and why it is allowed

Three departures, all in one direction, all licensed by §10.7.1's NOTE that the algorithm "is
not defined by PDF":

1. Both backends **anti-alias**, so a partly covered pixel is partly painted.
2. Therefore the painted area is *not* always at least the shape's.
3. `Image::area_averaged` averages over the pixel area where the clause says "there shall not be
   averaging over the pixel area" (ADR 0025 — it is what made `bug1001080.pdf` legible).

## What is honoured

Pixel boundaries on integers, half-open regions, a zero-width stroke drawn as the thinnest line
the device can produce (the clause's own permission), glyphs scan-converted by the font
rasteriser's own algorithm (the clause's last sentence allows it), and — since the
hundred-and-eighty-sixth session — **"no shape ever disappears"** for a fill whose subpath has no
extent along one axis (`pdf_render::collapsed`, ADR 0154).

**And since the three-hundred-and-sixty-eighth, *where* that mark goes.** NOTE 1 of the same
subclause says a filling region "is considered to intersect every pixel through which its boundary
passes, even if the interior of the filling region is empty", and its EXAMPLE says "A zero-width or
zero-height rectangle paints a line 1 pixel wide" — so the mark is the run of whole device pixels
the collapsed axis passes through, not a band at the shape's own fractional position. Both
statements were in `doc/md/` for the whole of the rule's life and neither had been read. Under a
rotation or a shear the band remains, because a slanted line's pixel run is a staircase; no corpus
document writes one. ADR 0208.

## Where the departure is *visible*, and how to tell it from a defect

Three oracle groups turn on this, and the distinction that matters is between a difference the
departure explains and one it does not:

- `CONTRADICTED_ANTIALIASED_EDGES` — `colors.pdf` pages 1 and 2: every renderer agrees about the
  swatch interiors to the byte and sits on a spectrum of edge softness. The pair the gate votes
  with is the pair nearest the clause, and we are furthest. The departure explains it whole.
- `AMBIGUOUS_SUB_PIXEL_LINE_WORK` — `22060_A1_01_Plans.pdf`: an A1 drawing that is *all* strokes
  under a pixel wide, so the departure moves the whole picture. Ink 10.00 ours, 10.27 `hayro`,
  10.59 `ghostscript` against 13.49 `poppler` and 13.75 `mupdf`; mean absolute difference from
  our render 571 for `hayro` against 1478, 1749 and 2091 — closer to the other renderer that
  anti-aliases at true coverage than the closest pair of references are to each other (1081).
  `/SA` occurs **zero** times in the document, so §10.7.5's promotion is not asked for.
- `AMBIGUOUS_TILING_CELL_CLIP` — `issue16038.pdf`: the departure did **not** explain it. Two
  anti-aliasing renderers (`hayro`, and `mupdf`'s left square) land on the geometry's own
  0.1333 while we were at 0.1114, and the 16% was a redundant clip's anti-aliased edge (ADR
  0155). **Anti-aliasing gives the shape's area; coming out *under* it is a defect.** The same
  page's *other* square was 13% under for the same reason at one remove — a rule the cell states
  on both box edges, halved by the clip and reassembled from two cells, the halves compositing
  rather than adding — and §11.6.2 is what settles that one: the tiles are portions of one object,
  which "shall not be composited with one another", so the two statements are the one mark they
  describe and it is drawn once (ADR 0213). Both squares are now within 1% to 3% of the geometry
  and within 1% of each other at every scale.

## The one rule this tree does not apply, deliberately

§10.7.5's stroke adjustment has two halves. The half a display can state exactly — a stroke
under half a pixel becomes one pixel — is implemented and conditioned on `/SA`, which is what
makes `AMBIGUOUS_STROKE_ADJUSTMENT`'s reading of `bug1743245.pdf` a derivation rather than a
preference. The other half asks that "the line width and the coordinates of a stroke shall
automatically be adjusted": that is grid-fitting, the non-uniformity it removes is an artefact of
the aliased scan conversion this tree already departs from, and nothing reports it because there
is no page on which this device could do better. **Any proposal to snap something to the pixel
grid has to say why it is not this.**

The one that has been taken says it in two sentences, and both are the standard's. A stroke has a
width the document stated, so adjusting its coordinates is *this* clause's requirement and is
conditional; a degenerate fill has no width, so its mark is §10.7.4's construction, stated with no
condition — and §10.7.4 exempts the zero-width **stroke** from that same rule in the next
sentence, "Zero-width strokes may be done in an implementation-defined manner that may include
fewer pixels than the rule implies". The fill's mark snaps and the `0 w` stroke does not. ADR 0208.

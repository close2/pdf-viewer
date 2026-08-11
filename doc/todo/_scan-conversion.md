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

Four departures, all licensed by §10.7.1's NOTE that the algorithm "is not defined by PDF", and
the first three all in one direction:

1. Both backends **anti-alias**, so a partly covered pixel is partly painted.
2. Therefore the painted area is *not* always at least the shape's.
3. `Image::area_averaged` averages over the pixel area where the clause says "there shall not be
   averaging over the pixel area" (ADR 0025 — it is what made `bug1001080.pdf` legible).
4. A clip's effect on a mark is a **product** where the clause states an intersection of sets —
   narrowed to the mark's own coverage in the four-hundred-and-forty-fourth session (ADR 0280), and
   the paragraph below has the reading and what is left.

## What is honoured

Pixel boundaries on integers, half-open regions, a zero-width stroke drawn as the thinnest line
the device can produce (the clause's own permission), glyphs scan-converted by the font
rasteriser's own algorithm (the clause's last sentence allows it), and — since the
hundred-and-eighty-sixth session — **"no shape ever disappears"** for a fill whose subpath has no
extent along one axis (`pdf_render::collapsed`, ADR 0154).

**And since the three-hundred-and-eighty-ninth, the shape that has an area and loses it anyway.**
Anti-aliasing replaces "paint the pixel" with coverage proportional to area, and a coverage that
rounds to *nothing* is not that replacement — it is the same disappearance reached by another road.
`tiny-skia` supersamples four times per pixel row at each sub-row's centre, so its smallest
non-zero coverage is a sixteenth of a pixel and a fill under an eighth of one vanished; and it
draws a stroke under a pixel wide as a hairline smeared symmetrically about the path, so one within
half a pixel of the raster's edge lost half its ink. `pdf_render::sub_pixel_bands` draws an
axis-aligned rectangle thinner than a device pixel as the whole pixel line it lies in, at the
coverage its own area there implies, and a sub-pixel stroke on a straight axis-aligned rule is
outlined into one first. **The graphics device never had either fault**, so this is the *oracle*
being brought up to it rather than a rule about scan conversion. ADR 0226.

**And since the four-hundred-and-thirty-second, the rule that is not axis-aligned** — which that
session found was a *different sentence* failing, not the same one. The clause makes two promises
and a diagonal separates them. "No shape ever disappears" is not at risk: a band between two of
`tiny-skia`'s sample lines vanishes, and a band that is not parallel to them crosses one every
`1/(4 tan θ)` pixels of its length, so a filled diagonal sliver 0.05 of a pixel thick reads 9.47 to
10.23 of its own 10 at every angle. What failed is "[t]he area covered by painted pixels shall
always be at least as large as the area of the original shape", and only for a **stroke**:
`tiny-skia`'s hairline lays one pixel down per step along the line's *longer device axis*, so it
carried `cos θ` of the rule's area — 29.3% short at 45°, at every thickness. The substitute is the
same rule stroked one device pixel wide with the width it gave up carried in the paint's alpha
(`pdf_render::substitute_width`), which is §10.7.4's own run of whole pixels and needs no scan
converter of ours. `issue11473.pdf`'s three diagonal hatch swatches went 0.6768 → 0.7566 against a
two-ladder limit of 0.752 to 0.760. ADR 0268; the boundary case it leaves is in `doc/todo/11`.

**And since the three-hundred-and-sixty-eighth, *where* that mark goes.** NOTE 1 of the same
subclause says a filling region "is considered to intersect every pixel through which its boundary
passes, even if the interior of the filling region is empty", and its EXAMPLE says "A zero-width or
zero-height rectangle paints a line 1 pixel wide" — so the mark is the run of whole device pixels
the collapsed axis passes through, not a band at the shape's own fractional position. Both
statements were in `doc/md/` for the whole of the rule's life and neither had been read. Under a
rotation or a shear the band remains, because a slanted line's pixel run is a staircase; no corpus
document writes one. ADR 0208.

**And since the four-hundred-and-forty-fourth, how a clip chain composes** — which is the same
subclause's *other* paragraph, the one about clipping, unread here until the
four-hundred-and-forty-third named it as a fourth departure (ADR 0279). §10.7.4 states a clip as a
**set of pixels** intersected with a set of pixels, and §8.5.4 says the same thing about a *value*:
"[t]he effective shape is the intersection of the object's intrinsic shape with the clipping path;
the source shape value shall be 0.0 outside this intersection." A clip zeroes what is outside it and
is silent about what is inside it. This tree composed a chain by *multiplying* anti-aliased
coverages, so a rectangle stated six times drew its edge at a twentieth of the mark, and the ladder
of *n* coincident `W n` clips read 0.5020, 0.2510, 0.1255, 0.0627, 0.0314, 0.0157 — each rung the
one above it halved.

The clause does not choose between `min` and a product on its own, and that is worth saying: on a
set-valued clip the two are the same function, and the question only exists because departure (1)
gives a boundary pixel a fraction. What decides is that a product moves further from the clause with
every restatement while `min` is exact for coincident and nested boundaries and never *below* the
product elsewhere. `scan::mask_intersect` takes the smaller of the two coverages; the ladder is flat
and `issue21346.pdf`'s edge went 0.041 → 0.163 of the mark against a departure-(1) 0.827 and a
clause 1.000. **The departure is narrowed rather than closed**: a mark's own coverage still meets
the clip mask inside `tiny-skia`'s `fill_path`, which multiplies, and that is the same sentence one
step along. ADR 0280; `doc/todo/11` item 4 carries what is left, and `doc/QUORRA_FEEDBACK.md` §18 is
the ask, because the graphics device composes its chain inside the library.

## Where the departure is *visible*, and how to tell it from a defect

Three oracle groups turn on this, and the distinction that matters is between a difference the
departure explains and one it does not:

- `CONTRADICTED_ANTIALIASED_EDGES` — `colors.pdf` pages 1 and 2: every renderer agrees about the
  swatch interiors to the byte and sits on a spectrum of edge softness. The pair the gate votes
  with is the pair nearest the clause, and we are furthest. The departure explains it whole.
- `AMBIGUOUS_SUB_PIXEL_LINE_WORK` — `22060_A1_01_Plans.pdf`: an A1 drawing whose four floor plans
  are **72 sampled images**, not strokes, which the four-hundred-and-thirty-second session counted
  after four sessions of this file calling it "*all* strokes under a pixel wide". Its 26 sub-pixel
  strokes carry 98% of their length within 5° of a device axis. So the departure that moves this
  picture is the *image* paragraph's — ADR 0025's area averaging — and not the shape rule's, which
  is why ADR 0226 could not move it and ADR 0268 moves it by 0.06%. Ink 10.00 ours, 10.27 `hayro`,
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

**ADR 0226 answers it by not snapping anything**, which is the cheapest form of that answer: a
0.1-unit rule draws 0.1 of a row at the fractional position the document put it, and what is
substituted is which *shape* carries the coverage rather than where the coverage goes.

The one that has been taken says it in two sentences, and both are the standard's. A stroke has a
width the document stated, so adjusting its coordinates is *this* clause's requirement and is
conditional; a degenerate fill has no width, so its mark is §10.7.4's construction, stated with no
condition — and §10.7.4 exempts the zero-width **stroke** from that same rule in the next
sentence, "Zero-width strokes may be done in an implementation-defined manner that may include
fewer pixels than the rule implies". The fill's mark snaps and the `0 w` stroke does not. ADR 0208.

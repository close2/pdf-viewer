# Shared background: §10.7.4, and what this tree departs from

Not a todo. Referred to by `11-shapes-that-still-disappear.md`,
by `AMBIGUOUS_ZERO_AREA_FILL`, `AMBIGUOUS_TILING_CELL_CLIP`, `AMBIGUOUS_SUB_PIXEL_LINE_WORK`,
`CONTRADICTED_ANTIALIASED_EDGES` and `CONTRADICTED_TIGHT_CONSENSUS` in `oracle.rs`, and by the
ledger's §10.7.4 row, which is the authoritative version.

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

1. Both backends **anti-alias**, so a partly covered pixel is partly painted. **How finely
   "partly" is measured is the departure's own second half, and it went unstated for six hundred
   sessions**: `render-cpu` rounded an edge's coverage to a *quarter* — `tiny-skia` samples four
   times per axis, at 0.125, 0.375, 0.625 and 0.875 — while the graphics device tracks the
   fraction to a level of 255. `render-quorra/examples/edge_coverage_ladder` prints both ladders
   against the geometry, and ADR 0474 has what it cost: on `colors.pdf` our whole raster was the
   page's closed form with every coverage so rounded, 33 levels of 255 from the exact one at the
   worst pixel. It is the quantum this file already records for a shape *thinner* than a pixel,
   met at the edge of a thick one.

   **The commonest shape in every PDF stopped paying it in the six-hundred-and-forty-sixth
   session** (ADR 0476). §10.7.4 defines a pixel as a product of two half-open intervals and gives
   a filled shape the same form, so an axis-aligned rectangle's coverage of a pixel is the product
   of its two one-dimensional overlaps — exact, at every placement, and derived rather than
   fitted. `pdf_render::edge` states the geometry for both backends, `render-cpu` hands such a
   fill to `tiny-skia`'s **rectangle** scan converter rather than its path one, and writes the same
   closed form into a rectangular **clip region's** mask, because this subclause says the region
   "consists of the set of pixels that would be included by a fill operation" and the two must
   therefore be one rule. Both backends now answer the ladder to a level of 255 at all twenty-one
   rungs. **What still carries the quantum is every shape that is not a single axis-aligned
   rectangle**, where it is a sixteenth rather than a quarter and averages along the edge; a glyph,
   a curve and a diagonal are in it. **A stroke's outline joined the exception in the
   six-hundred-and-ninetieth** (ADR 0535), where the outline is one rectangle: a butt-capped
   straight rule along a device axis is drawn as the fill of that rectangle and measured by the
   same closed form. **And a path stating *several* rectangles joined it in the
   seven-hundred-and-eleventh, on a clause that is not this one** (ADR 0583): this file and
   `doc/todo/11` item 7 both said such a path was deliberate and waited on item 5's seam, and
   §11.3.7.3's union is what the standard says to do with two *objects*. A path's subpaths are
   portions of one, and §11.6.2 forbids compositing portions of one object with one another — so
   the construction was forbidden rather than traded. `pdf_render::device_rectangles` decomposes
   such a path and `DeviceRectangles::share_a_device_pixel` asks §10.7.4's own question, whether
   two portions fall in one pixel; where none do, each is drawn as its own mark at the exact area,
   for a fill and for a clipping region alike. 3419 fills over 151 first pages of the pdf.js
   corpus, +0.074% of the rasteriser on a page of text against −0.53% on a page that states them.
   **And the 505 that *do* share a pixel left it in the seven-hundred-and-fifteenth** (ADR 0590):
   their portions' exact areas are **summed** into one coverage buffer and the paint blitted through
   it once, which is one composition with the backdrop for the whole object and each portion's own
   area within it. The two requirements are independent — §11.6.2 by the single blit, this
   subclause's third sentence by the closed form — and the buffer is `scan::intersected`'s, which
   ADR 0355 built for a clip and which now answers to either clause on its own. Interiors are
   pairwise disjoint by construction, so the area of a shared pixel's union is a plain sum; it is
   written in one rounding rather than accumulated, because a sum of roundings is a level or two out
   and a coverage rounded away is what this departure is a departure *from*. 22 of 958 first pages
   move, every one of them the census's own, at +0.063% of the rasteriser on a page of text and
   −32.3% on a barcode. **So departure (1)'s multi-rectangle population is closed**; what still
   carries the quantum is every shape that is not axis-aligned rectangles at all.
2. Therefore the painted area is *not* always at least the shape's. **This one had no witness for
   four hundred and seventy-two sessions and now has a large one** (ADR 0308): where a document
   states one region as *many* opaque fills, every internal boundary falls inside some device
   pixel, and §11.3.7.3 composites each mark's coverage into the last by the *union* function —
   "an 'inverted multiplication'". Two halves unite to three quarters, four quarters to `0.75⁴`,
   and *n* equal shares rise towards `1/e`, so what lies under the region shines through it. On a
   cross-section of 58 003 filled polygons the layer beneath keeps 0.1937 of itself at page scale,
   0.0673 at 4× and 0.0156 at 8×. `doc/todo/11` item 5 has the whole measurement, the three
   backends, the four references and what a cure would cost; `crates/pdf-model/examples/
   uncovered_share.rs` is the instrument for any page.

   **And the seam belongs to *this* departure rather than to §11.3.7.3, which is the correction
   the seven-hundred-and-eleventh session made** (ADR 0582). ADR 0308 recorded it as "the model,
   applied to the fractional shape §11.3.7.2's NOTE 1 says anti-aliasing produces". The model's
   values are at *points*: §11.2 is a `shall` — shape and opacity "shall be defined at every point
   in the plane" — and §11.6.4.2 makes a path's shape "1.0 inside and 0.0 outside", so the union of
   two abutting marks is 1.0 at every point and **the clause states no seam**. The fraction enters
   through a `can` in a NOTE about rasterising to device pixels, and averaging does not commute
   with a non-linear function. §11.2's own NOTE 1 names the cause: the model "does not require a
   PDF processor to rasterize objects immediately or to commit to a raster representation at any
   time before rendering the entire stack onto the page … since rasterization often causes
   significant loss of information and precision". So the seam is departure (1) meeting §11.3.7.3,
   the departure is still licensed, and what a cure costs is unchanged — but the value it would
   reach is the clause's own rather than an improvement on it.
3. `Image::area_averaged` averages over the pixel area where the clause says "there shall not be
   averaging over the pixel area" (ADR 0025 — it is what made `bug1001080.pdf` legible).
4. A clip's effect on a mark is a **product** where the clause states an intersection of sets —
   narrowed to the mark's own coverage in the four-hundred-and-forty-fourth session (ADR 0280),
   narrowed again to a filled mark in the five-hundred-and-twentieth (ADR 0355) and to a clip
   standing beside a soft mask in the five-hundred-and-twenty-eighth (ADR 0363), and the
   paragraphs below have the reading and what is left.

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
clause 1.000. **The departure is narrowed rather than closed**: a mark's own coverage still met
the clip mask inside `tiny-skia`'s `fill_path`, which multiplies, and that was the same sentence one
step along. ADR 0280; `doc/todo/11` item 4 carries what is left. **§18 was the ask, because the
graphics device composes its chain inside the library, and it is answered**: quorra takes `min`
across a chain too (its ADR 0030), read off §8.5.4's "the graphics state holds one clipping path"
rather than off this tree's argument — so the two backends compose a chain by one rule again, and
both still multiply where the clip meets the *mark*.

**And since the five-hundred-and-twentieth, where a clip meets a *filled* mark** — the step ADR
0280 priced as "this backend's own blitter" and which needed none (ADR 0355). The closed form is the
clause's own set identity rather than anybody's arithmetic: `S ∩ C = S` where `S ⊆ C`, so **a clip
that contains a mark takes nothing from it**, at the mark's anti-aliased boundary included.
`scan::intersected` rasterises the mark's coverage into a buffer with the same scan converter, takes
the smaller of it and the region per pixel, and blits the paint through the result over the whole
device pixels the mark reaches — `Mask::fill_path` and `PixmapMut::fill_rect`, both the library's
own. It declines where the substitution would say something else: a clip already 0 or 255 under the
mark, a mark that is not anti-aliased, and `BlendMode::Source`. `issue21346.pdf`'s edge went
0.163 → **0.306** of the mark. **What still multiplies** is an image's edge, part of a group's
raster, and both other backends — which is why the departure is narrowed rather than closed
a second time.

**And since the six-hundred-and-ninetieth, where a clip meets a *stroked* mark** — the same
sentence one operator over, since neither §10.7.4's clipping paragraph nor §8.5.4 names an operator
and §8.4.3 gives a stroke a shape (ADR 0535). Every stroke went to
`tiny_skia::PixmapMut::stroke_path`, which hands the finished mask to its own `fill_path`. **The
price this file and `doc/todo/11` both quoted was wrong in ADR 0476's direction**: it read
"duplicating the library's stroker and contradicting its hairline", and `tiny_skia::Path::stroke`
and `tiny_skia::Path::dash` are public and are the same stroker and dasher that method calls — its
non-hairline branch is exactly those two followed by a non-zero `fill_path`.
`render_cpu::draw_stroked_outline` performs them one call earlier so that `scan::fill` composes the
result, which is what `draw_long_mitres` has done for §8.4.3.5's paths since ADR 0398 and what
`sub_pixel_bands` and `substitute_width` have done for §10.7.4's sub-pixel rules since ADR 0226 —
so three of this backend's four stroke constructions were fills already and the fourth is now one.
The hairline half is answered by **moving the boundary rather than crossing it**: `tiny-skia`
chooses its hairline from an approximate length of the width along the transform's two basis
vectors, `pdf_render::thinnest_line` is the larger singular value, and the two agree for every
similarity transform and part by up to √2 under a shear — so the boundary is the shared crate's
now, which is trap 2 rather than a preference. Because a stroke's mark is a fill's mark from here
on, ADR 0476's exact rectangle coverage reaches a butt-capped straight rule along a device axis,
which it did not before: `coincident_edge_probe`'s stroke table read 0.4980 alone and 0.2510 under
each of three restatements where the fill table read 0.5059 four times, and all eight rungs read
0.5059 now.

**And since the five-hundred-and-twenty-eighth, a clip standing *beside* a soft mask** — the case
ADR 0355 declined because the two were already one buffer (ADR 0363). The standard states them in
an order, and the fold destroyed it: §8.5.4 intersects the clipping path with the object's *own*
shape and §11.3.7.2 multiplies the mask shape into what comes out, so `fₛ = (fⱼ ∩ C) · fₘ` rather
than `fⱼ · (C · fₘ)`. `MaskCache` keeps the soft mask's rows beside the product and `scan::fill`
composes `min(M·S, C·S)`, which is the whole of what is needed because multiplication by a
non-negative value distributes over a minimum — and which is *exact* in eight bits, since a minimum
commutes with a monotone rounding. **No corpus page can tell the two apart**: 13 compositions over
five documents, every raster byte-identical, because the two part only where the mark's boundary
and the clip's are both fractional in one pixel. It is the coverage question answered without the
robustness one, which is `CLAUDE.md`'s own case for taking it.

**The same round found the departure's account of its witness wrong.** `issue21346.pdf`'s remaining
factor is not a clip folded into a soft mask on a fill — nothing on that page reaches `scan::fill`
that way — but a **transparency group's raster** meeting its clip through `draw_pixmap`'s product.
§8.5.4 says that one is owed too: a group's shape is "defined as the union of the shapes of its
constituent objects" and "shall be influenced … by the one in effect at the time the group's results
are painted onto its backdrop". What ADR 0355 was right about is narrower — this backend's group
buffer carries *alpha*, which is shape times opacity, so the construction needs a shape channel
beside the raster. Measured with the set kept apart at that blit, the witness's edge goes
**0.306 → 0.571** of the mark against departure (1)'s 0.827. `doc/todo/11` item 4 carries it.

**And since the five-hundred-and-eighty-fourth, the floor under all of it** — which is departure (1)
meeting the eight bits its coverage is carried in (ADR 0419). Every substitution above states a mark
wider than the document's own and puts the area it gave up in the paint's **alpha**, so each has a
second floor below the rasteriser's coverage quantum: a coverage under `1/255` rounds to nothing and
the mark is gone by a third road. Measured, a 200-unit rule at 0.002 and 0.001 of a device pixel drew
**no ink at all** on both backends, and a ladder of one rule at seventeen widths against `pdftoppm`,
`mutool`, `gs` and `hayro` puts ours as the only column that reaches zero.
`pdf_render::expressible_coverage` states a positive coverage under one level *at* one level — the
one place this crate names the raster's depth — and the mark is then drawn heavier than its geometry,
which is the side of "[t]he area covered by painted pixels shall always be at least as large as the
area of the original shape" that the `shall` is on. It is not §10.7.5's promotion: the mark keeps its
place and the substitute's width, and only the last level of its alpha moves.

**And since the five-hundred-and-eighty-fifth, the mark an alpha the raster *can* hold still loses**
— which is the same floor one step along and is where departure (1) meets §10.7.4's *second*
sentence rather than its third (ADR 0420). An alpha of one level is not a level of ink: what a pixel
receives is the alpha times the shape's coverage there, and the widened mark covers no pixel fully.
§8.5.3.2's dot at 0.1 of a device pixel arrived with 2.55 levels of alpha and deposited half a level
in each of four pixels, because its substitute is a circle one pixel across whose centre sat on a
pixel *corner*; the same dot half a pixel over landed with the geometry's own ink. That is
"unfavourable placement relative to the device pixel grid" in the clause's own words, and it is the
measurement that separates it from an arithmetic floor. The answer is the one shape whose coverage
is not a fraction and the clause identifies it — "let i = floor( x ) and j = floor( y ). The pixel
that contains this point is the one identified as ( i, j )" — so `pdf_render::point_mark` states a
mark centred at a single point as *that* pixel, at the coverage its own area implies, as soon as the
widened form can reach no level in any of the at most nine pixels it can be divided between. **It is
not the snapping ADR 0208 declines**: the ink is the mark's own area rather than a whole pixel's and
the pixel is the one the mark's own centre is in, so nothing moves to a grid line; what is given up
is the mark's shape *within* one pixel. All three rasterisers take it, because all three lost the
mark — the processor and quorra drew nothing at 0.1 and below, vello half the area at 0.2 and
nothing under it.

**The same round settled what this file's departure (1) is a departure *from*, at the bottom of the
range.** §10.7.4's floor is one whole device pixel and it belongs to the aliased algorithm — asked
for it, `pdftoppm -aa no`, `gs -dGraphicsAlphaBits=1` and `mutool draw -A 0` each answer exactly one
whole pixel at every sub-pixel width, agreeing about the clause. With anti-aliasing on the four
references floor at four *different* device-pixel widths (`poppler` and `hayro` 1.0, `mupdf` 0.2,
`ghostscript` 0.27), which is where §10.7.1's NOTE hands the question to the implementation. §10.7.5
states that floor as a `shall` and conditions it on stroke adjustment, whose Table 52 initial value
is `false`. So for an anti-aliasing device the clause's only unconditional requirement here is that
no shape ever disappears, and `issue12295.pdf`'s pale ECG traces are this departure working rather
than a defect. `doc/todo/11` carries the two marks that are still lost.

## Where the departure is *visible*, and how to tell it from a defect

Three oracle groups turn on this, and the distinction that matters is between a difference the
departure explains and one it does not:

- `CONTRADICTED_TIGHT_CONSENSUS` — `colors.pdf` pages 1 and 2, and **this bullet said the wrong
  thing about them until the six-hundred-and-forty-third session**: "every renderer agrees about
  the swatch interiors to the byte and sits on a spectrum of edge softness … we are furthest. The
  departure explains it whole." The interiors half is right and the rest is not. Sixteen
  axis-aligned rectangles at known sub-pixel boundaries make each page a closed form; ours was that
  form with every coverage rounded to `tiny-skia`'s quarter (max 1 level of 255 over the whole
  raster) and `hayro`'s is the exact one (max 2). From the geometry at the worst pixel: `hayro` 2,
  `mupdf` 13, ours 33, `ghostscript` 54, `poppler` 124 — two paint the shape's area, `poppler`
  paints whole pixels, and we were third of five rather than the soft end of a spectrum. And the
  departure does not explain the *verdict* at all: the exact
  form is contradicted here too, at ssim 0.98772 against a bound of 0.98862, because the pair that
  votes is the pair furthest from the geometry. ADR 0474.

  **Since the six-hundred-and-forty-sixth our raster *is* the exact form (ADR 0476), and the two
  pages confirm 643's prediction to the fourth decimal.** That session computed, from the file's
  own arithmetic and with no code written, that a rasteriser painting precisely the covered area
  would read ssim **0.98772** on page 1 and **0.98001** on page 2. The gate now measures
  **0.9879** and **0.9802**, against bounds of 0.9886 and 0.9840. So both pages are still
  contradicted, exactly as predicted and for the predicted reason — and the two numbers arriving
  where a closed form said they would, by a route with no renderer in it, is the strongest evidence
  this file has that the construction is the clause's and not a fit.
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

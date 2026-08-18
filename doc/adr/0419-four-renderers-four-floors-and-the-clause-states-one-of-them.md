# ADR 0419 — Four renderers, four floors, and the clause states one of them

Status: accepted, 2026-08-18. Session 584.

`issue12295.pdf` draws its ECG traces as a ghost where all four reference renderers draw them
dark. Session 583 left the page named and undiagnosed, and the question it left is not "how do we
match them" but which reading of ISO 32000-2 is right, because the answer decides the code:

- ***Area is ink.*** A rule 0.1366 of a device pixel wide deposits 0.1366 of a pixel of ink and a
  correct rasteriser draws it pale. Then the four references share a *convention* the clause does
  not state, and the page stays contradicted with the citation beside it.
- ***A stroke is visible.*** §8.4.3.2 makes a zero-width line one device pixel wide, and something
  in §10.7.4 or §8.4.3 extends that to a line thinner than a device pixel. Then the construction
  ADRs 0226, 0268, 0285 and 0290 built is wrong at the bottom of its range.

Both are defensible on their face. What follows is the clauses, then the measurement, then which
won and what it cost.

## 1. What the clauses say

`spec-errata emit` over `doc/ISO_32000-2_sponsored_EC3.pdf` first, as `doc/todo/02` §4 requires:
**Errata Collection 3 touches nothing in §8.4.3.2, §8.4.3.3–6, §10.7.3, §10.7.4 or §10.7.5.** It
touches §10.7.2 — "It shall be a positive number" is struck out for "It shall be a number in the
range 0 to 100 inclusive, where a value of 0 shall specify the output device's default flatness
tolerance. The value indicates the maximum error tolerance measured in output device pixels" — and
the `doc/md/` conversion carries the superseded sentence, which is a thing to know before quoting
that one clause. Nothing in the erratum bears on line width.

### §8.4.3.2, and the two sentences that are not about zero

> A line width of 0 shall denote the thinnest line that can be rendered at device resolution: 1
> device pixel wide.

A `shall`, and it is about **zero**. `Stroke::device_width` states it in `pdf-render` and both
backends read it (ADR 0285). The clause says nothing of the kind about a width that is positive and
under a pixel; what it says instead is two sentences of description:

> The actual line width achieved can differ from the requested width by as much as 2 device pixels,
> depending on the positions of lines with respect to the pixel grid. Automatic stroke adjustment
> may be used to ensure uniform line width; see 10.7.5, "Automatic stroke adjustment".

*Can differ*, and *may be used*. So this subclause **permits** a processor to be two device pixels
out and points at §10.7.5 for the way to stop being; it requires nothing of a sub-pixel width.

### §10.7.4, whose construction is aliased and whose purpose is not

> A shape shall be scan-converted by painting any pixel whose half-open square region intersects
> the shape, no matter how small the intersection is. This ensures that no shape ever disappears as
> a result of unfavourable placement relative to the device pixel grid, as might happen with other
> possible scan conversion rules. The area covered by painted pixels shall always be at least as
> large as the area of the original shape. This rule applies both to fill operations and to strokes
> with non-zero width.

Read literally that is aliased rendering, and under it a 0.1366-pixel rule is a **solid** run of
whole pixels. This tree does not do that: `doc/todo/_scan-conversion.md`'s departure (1) is that
both backends anti-alias, licensed by §10.7.1's NOTE that "[t]he specifics of the scan conversion
algorithm are not defined as part of PDF". The departure replaces "paint the pixel" with coverage
proportional to area, and what it may not do is reach zero — that is the second sentence's stated
purpose failed by another road, which is `pdf_render::sub_pixel`'s opening argument and has been
since ADR 0226.

### §10.7.5, which states the references' answer and conditions it

> If stroke adjustment is enabled and the requested line width, transformed into device space, is
> less than half a pixel, the stroke shall be rendered as a single-pixel line.

This is exactly the floor the references draw, stated as a `shall` — **and conditioned**. Table 52
gives the graphics state's stroke adjustment parameter its initial value:

> Initial value: false .

`issue12295.pdf` contains **no `/SA`, no `/ExtGState` and no `/GS`** — zero occurrences of each in
375 776 bytes. The document does not ask for stroke adjustment, so the clause that would make its
traces a single-pixel line does not apply to it.

### §10.7.2 and §10.7.3, read because a clause that permits has been read

Neither reaches line width, and both are permissions rather than silences, which is worth the
sentence `CLAUDE.md` spends on the difference: §10.7.2's "PDF processors may choose to ignore any
flatness tolerance specified within a PDF file" and §10.7.3's "[e]ach output device may have
internal limits on the maximum and minimum tolerances attainable". A curve's flattening and a
shading's sampling are the subjects; a stroke's width is not.

## 2. What the references actually do

Evidence about our reading, never the definition of it — and here it is unusually sharp, because
the references do not agree with each other.

**They do not share the code that draws a hairline.** `objdump -p | grep NEEDED` on `pdftoppm`,
`mutool` and `gs` and on the three libraries under them: all three link `libjpeg.so.8` and
`libopenjp2.so.7`, `poppler` and `mupdf` share `libfreetype.so.6`, `poppler` and `gs` share
`liblcms2.so.2`, `mupdf` and `gs` share `libjbig2dec.so.0` — and not one library that rasterises a
path is shared by any two of them. `hayro` is `vello_cpu` and shares nothing with any of them or
with `tiny-skia`. Four rasterisers, four voices.

**The ladder.** One rule 160 units long on a 200 × 200 page, one page per width, ink over the whole
raster in levels of 255, so the geometry's own answer is `1.02 × w` at every resolution. Taken at
72 dpi and again at 576 dpi, because `doc/todo/02` §7's first habit says one ladder cannot tell
convergence from drift. Horizontal, at 72 dpi:

```text
   width   geometry      ours   poppler     mupdf        gs     hayro
     1.0     1.0200      1.02      1.02     1.024       1.3     1.024
     0.8     0.8160     0.816      1.02     0.816     1.096     1.024
     0.5     0.5100     0.512      1.02     0.476      0.82     1.024
     0.3     0.3060     0.304      1.02      0.34     0.544     1.024
     0.2     0.2040       0.2      1.02     0.204     0.272     1.024
  0.1366     0.1393     0.136      1.02     0.204     0.272     1.024
    0.05     0.0510     0.048      1.02     0.204     0.272     1.024
    0.01     0.0102     0.008     0.128     0.204     0.272     1.024
   0.001     0.0010         0     0.128     0.204     0.272     1.024
     0.0     0.0000      1.02   1.02638     0.204     0.272     1.024
```

and at 576 dpi the same widths, where one device pixel is `0.1275` of a level:

```text
   width   geometry      ours   poppler     mupdf        gs     hayro
  0.1366     0.1393    0.1275     0.255    0.1455     0.188     0.139
     0.1     0.1020     0.102    0.1275     0.102     0.137     0.128
    0.05     0.0510     0.051    0.1275     0.051     0.068     0.128
    0.01     0.0102      0.01    0.1275    0.0255     0.068     0.128
   0.001     0.0010     0.001     0.016    0.0255     0.068     0.128
     0.0     0.0000    0.1275    0.1276    0.0255     0.068     0.128
```

Read the two together and every renderer's behaviour is a **device-pixel** floor rather than a
user-space one, and **no two of the four floors are the same number**:

| | floor, in device pixels | the same at 8× |
|---|---|---|
| `poppler` | 1.0 | 1.0 |
| `hayro` | 1.0 | 1.0 |
| `mupdf` | 0.2 | 0.2 |
| `ghostscript` | 0.27 | 0.53 |
| ours | none | none |

A convention shows up as a floor and area-is-ink shows up as a straight line: ours is the straight
line, at both resolutions, to four figures — and at 0.001 of a pixel it reaches **zero**, which is
the one number in the whole table that no reading of the clause permits. The 30° ladder says the
same thing with our column exactly on the geometry at every rung and zero at 0.001.

**And the decisive rung is the one taken with anti-aliasing off.** Ask each C reference for the
clause's own algorithm and the disagreement vanishes:

```text
   width   geometry  pdftoppm -aa no  gs -dGraphicsAlphaBits=1  mutool -A 0
     0.3     0.3060             1.02                   1.02638         1.02
  0.1366     0.1393             1.02                   1.02638         1.02
    0.05     0.0510             1.02                   1.02638         1.02
```

**One whole device pixel, all three, at every sub-pixel width.** That is §10.7.4 as written, and it
is what the four voices agree about. Where they part is the moment anti-aliasing is switched on —
1.02 against 0.204 against 0.272 — which is precisely the region the clause hands to the
implementation.

**The source says it out loud in the one place this disk has it.** `tmp/hayro/hayro/src/path.rs`:

```rust
// Best-effort attempt to ensure a line width of at least 1.0, as required by the PDF
// specification. If we are stroking text, we reduce the threshold as it will otherwise
// lead to very bold-looking text at low resolutions.
```

The comment claims the specification requires it. No clause does — §8.4.3.2 requires it of a width
of *zero* and §10.7.5 requires it under a parameter this document does not set. And the code around
the comment is a heuristic by its own admission: `0.25` instead of `1.0` for text, and the whole
rule disabled "if not inside of pattern or type 3 glyph", three exceptions no clause states.
`poppler`'s, `mupdf`'s and `ghostscript`'s sources are not on this machine and their thresholds are
reported from the ladder alone.

## 3. Which reading won

**Area is ink, with one floor, and the floor is the raster's rather than the pixel's.**

The clause states a floor for the algorithm it describes — one whole pixel, for an aliased device,
and all three C references produce exactly that when asked to be aliased. It states no floor at all
for the anti-aliased algorithm §10.7.1's NOTE permits instead; the rule that looks like one is
§10.7.5's, and it comes with a condition the witness does not meet. What §10.7.4 does state
unconditionally, and what therefore binds an anti-aliasing device, is the sentence this tree has
been citing since ADR 0226: *no shape ever disappears*.

So the references' darkness is their own choice, taken four different ways, and this is
`CLAUDE.md`'s "the standard defines nothing here" in its honest form rather than as a shrug: the
titles around the subject were read, the errata over both clauses were run, and the silence is
bounded by two `shall`s on either side of it.

**And the reading convicts us of something.** Our column reaches zero, and no reading permits that.

## 4. What was implemented

`pdf_render::expressible_coverage`, and it is one line of arithmetic under a paragraph of clause:

> a coverage that is positive and under one level of 255 is stated **at** one level.

Every substitution in `pdf_render::sub_pixel` carries a mark's given-up area in the paint's
**alpha**, and an alpha is eight bits — so each of them has a second floor, further down than the
rasteriser's coverage quantum ADR 0226 answered and reached by a different road. It is applied to
all three coverages the constructions produce: `SubPixelBand::coverage`, `EnlargedMark::coverage`
and the turned body's `style.width / width`, which `render-cpu` had been computing itself and now
asks the shared crate for (trap 2: a device decision either backend can make alone is a decision
neither has made). The raster's depth is named in exactly one place.

**It costs ink and the clause asks for that direction**: "[t]he area covered by painted pixels
shall always be at least as large as the area of the original shape". A mark this floor raises is
drawn heavier than its geometry, which is the side of the sentence the `shall` is on. What it does
*not* do is §10.7.5's promotion — the mark keeps its place and the width the substitution gave it,
and only the last level of its alpha moves — so `AMBIGUOUS_STROKE_ADJUSTMENT`'s reading of
`bug1743245.pdf` stays a derivation and ADR 0208's rule about snapping is untouched.

Measured, on `sub_pixel_marks`' new seventh section, a 200-unit rule against its own area:

```text
                       before            after
  0°   0.002   cpu    0.0000  −100%     1.5686
  0°   0.001   cpu    0.0000  −100%     1.5686
  30°  0.002   cpu    0.0000  −100%     0.7804
  30°  0.001   cpu    0.0000  −100%     0.7804
```

`a_rule_under_one_level_of_alpha_still_marks_the_processors_raster` gates it, at the four rungs
above, on ink existing rather than on what the ink comes to — because where the floor bites the ink
is deliberately not the geometry.

**No document on this disk is a witness.** `sub_pixel_width_census` over 1005 first pages of every
corpus here — the 974 pdf.js documents, `pdf20examples`, `pdf-differences`, `pdfbox` and the three
`format-corpus` directories — puts the thinnest stated sub-pixel stroke at **0.05 of a device
pixel**, twelve levels of alpha and nowhere near the floor. This is an unwitnessed `shall`, taken
because it is a `shall`, and it is the same shape as `pdf_render::collapsed`'s (ADR 0154).

## 5. What is left, measured rather than assumed

- **`render-quorra` still loses the mark**, at 0.002 and 0.001 of a device pixel on an axis and at
  0.001 turned. It takes no substitution here — it hands quorra the document's own width and that
  rasteriser's coverage is what runs out — so the floor has nowhere to be applied. The gate holds
  the processor only and says so.
- **§8.5.3.2's dot is worse than ADR 0290 recorded, and the floor does not recover it.** Both
  backends draw **nothing** for a degenerate subpath under round caps at 0.1, 0.05, 0.02 and 0.01
  of a device pixel; ADR 0290's own ladder has been printing the 0.1 row as `-100.0%` since it
  landed and the gate's rungs start at 0.2. Raising the alpha does not help, and the reason is
  worth keeping: the substitute for that mark is a circle **inscribed in** one device pixel, which
  covers no pixel fully, so the raster's rounding takes the mark a second time after the alpha
  survived it. What would recover it is stating the mark as the whole pixel it lies in — which is
  snapping, which §10.7.5 conditions on `/SA` and ADR 0208 declines for a mark that has a width.
  So it is left, with its threshold measured.
- **`issue12295.pdf` does not move**, and that is the conclusion rather than a shortfall. Its
  strokes are 0.1366 of a pixel and its caps `0.1366²`, both far above one level. The page's group
  is corrected to say what the references are doing and on what evidence.

## 5a. What the choice costs a reader, looked at rather than argued

Trap 1, and it moved the price rather than the verdict. The oracle's side-by-side renders every page
at **72 dpi**, and at 72 dpi this page's traces are 0.1366 of a pixel and ours is the ghost session
583 named. That is not what a person sees. Opened in `target/pdf-viewer` under `Xvfb` at 900 × 1100,
the page is fitted at about 1.36× and the same strokes are 0.19 of a device pixel — 48 levels of 255
— and **both ECG panels are legible**, beat by beat, with the P waves and the QRS complexes
distinguishable. Lighter than the references would draw them, and a reader's page rather than a
blank one.

So the departure's cost is real and is smaller than the artefact suggests, and it shrinks with every
step of magnification while the references' floor does not move at all. That is the same shape as
`doc/todo/11` item 5's seam, which is worst at a page fit and gone by 8×, and it is worth saying in
the same breath as the verdict: **the picture that decided nothing here is the one taken at the
resolution the gate happens to use.**

## 6. What was not built, and where it would go if it ever is

A reader who wants the references' picture is asking for stroke adjustment to be on when the
document has not asked for it. That is a **preference**, not a reading, and `CLAUDE.md`'s rule
about a document's restrictions says where a preference of that kind belongs: asked once, in a
place a host can supply, rather than decided in `pdf-model` where no host can reach it. Nothing is
built for it now and no message is added — `doc/ui-boundary.md`'s test is that a clause asks for a
message, and this one asks for the opposite.

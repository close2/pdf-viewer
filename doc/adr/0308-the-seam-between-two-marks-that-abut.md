# 0308 — The seam between two marks that abut, and whose defect it is

**Status.** Accepted.
**Context.** The project owner opened a 50 MB Inkscape geological cross-section, saw a dark frame
rule shining through the polygons drawn over it, watched it disappear on magnification, and asked
a direct question: is this a bug of pdf-viewer, and if it is quorra's, write the bug report.

The answer is neither, and this ADR is the evidence rather than the assertion. It also closes the
form of `doc/todo/11`'s item 2 that had been carried as *unwitnessed* for a hundred sessions, and
gives `doc/todo/_scan-conversion.md`'s departure (2) — "the painted area is *not* always at least
the shape's" — the first witness it has ever had.

## What the page states

One page, 1666.39 × 473.539 pt, one content stream of **147 972 263 bytes**:

```text
  c   2 868 970      f   58 003      h   69 295      re      9
  m     127 295      rg  57 413      l   4 274       cm/q/Q  7
```

58 003 filled paths, each with its own colour, **no clipping path at all** and one `gs`. The
display list is 58 009 commands, so nothing is being split: `interpret` produces one `Command::Fill`
per `f` and the six strokes are the drawing's frames. Those frames are drawn *first* — the rule the
owner saw is two stroked rectangles sharing a coincident edge, at user x 1329.8279 and 1329.914,
display-list commands 1 and 2 — and the polygons are drawn over them, three to seven device pixels
across at page scale.

The residue was measured by rendering the page twice, once with those two commands in the list and
once without, and solving for the fraction of the rule's own colour still visible. At scale 1 it is
**0.19 to 0.35 at the rule's two device columns and zero at every other column**, which is exactly
the report: a rule that is covered, and is not.

## What the standard says about it

Two clauses, and they disagree only because this tree departs from the first.

§10.7.4's scan conversion is **aliased**:

> A shape shall be scan-converted by painting any pixel whose half-open square region intersects
> the shape, no matter how small the intersection is.

Under that rule both abutting fills paint the whole boundary pixel and the pair leaves nothing.
`doc/todo/_scan-conversion.md`'s departure (1) is that both backends anti-alias instead, licensed by
§10.7.1's NOTE that the algorithm "is not defined by PDF", and departure (2) is the consequence the
same subclause names: "[t]he area covered by painted pixels shall always be at least as large as the
area of the original shape", which anti-aliasing does not honour.

§11.3.7.3 then says what happens when the second mark meets the first. Result shape is the *union*
of the backdrop's and the source's, and the standard's own NOTE 1 says what union means:

> This is a generalization of the conventional concept of union for opaque shapes, and it can be
> thought of as an "inverted multiplication" -a multiplication with the inputs and outputs
> complemented. The result tends toward 1.0: if either input is 1.0, the result is 1.0.

The union of two halves is three quarters. **So the seam is not a deviation from the model — it is
the model, applied to the fractional shape §11.3.7.2's NOTE 1 says anti-aliasing produces**, which
is the same NOTE this tree already cites for putting coverage into alpha. It follows that *n* marks
sharing a boundary pixel equally leave `(1 − 1/n)ⁿ` of what is beneath, which **rises** with *n*
towards `1/e = 0.3679`: a drawing gets worse the more pieces it states its region in, and 58 003
polygons is a great many pieces.

## The three rasterisers

`crates/render-quorra/tests/abutting_marks.rs`, one fixture, at one device pixel per user unit:

```text
  share of the backdrop still showing in the boundary pixel
                               two halves    four quarters
    the arithmetic                 0.2500           0.3164
    render-cpu (tiny-skia)         0.2510           0.3137
    render-quorra                  0.2471           0.3137
    render-gpu (vello)             0.2510           0.3176
```

All three inside one level of 255 of each other and of the formula. **Nothing here is quorra's**,
so no section of `doc/QUORRA_FEEDBACK.md` is owed; `render-gpu` is not on the viewer's path at all
(`viewer-ui` depends on `render-cpu` and `render-quorra` and on neither vello nor `render-gpu`) and
was measured anyway, because "three independent rasterisers agree" is worth more than two.

## The four references, and the one that looks like it has the answer

```text
    mutool draw                    0.2510
    gs -dGraphicsAlphaBits=4       0.2200
    gs, anti-aliasing off          0.0000
    pdftoppm                       0.0000
```

`gs` with anti-aliasing off is §10.7.4's own rule and has no seam by construction — that is the
clause working, not a renderer being clever, and it is the reason the clause is written the way it
is. With anti-aliasing on it leaks 0.22, so its supersample-and-filter is not conflation handling
either; it is a quantised approximation of one.

**`pdftoppm`'s zero is trap 9 and the control proves it.** Draw a *single* rectangle whose right
edge lands half way across a device pixel over black:

```text
    render-cpu                     0.498
    mutool draw                    0.533
    gs -dGraphicsAlphaBits=4       0.467
    pdftoppm                       0.000
```

Poppler does not anti-alias an axis-aligned rectangle's edge at all — it snaps it to whole pixels —
so on the fixture it is doing §10.7.4 rather than solving conflation. Restate the same two marks
with a seam that is **not** axis-aligned and poppler leaks with everyone else: 1.765 device pixels
over the fixture's square against this tree's 2.698, `mupdf`'s 2.871 and `gs`'s 1.004. The three
renderers with true analytic coverage — this tree's two and `mupdf` — agree to a level of 255; the
two that appear not to have the artefact are the two not anti-aliasing the shape.

The same ranking holds on the owner's page itself, at 72 dpi, measuring the rule's two columns
against their neighbourhood over the region the polygons cover:

```text
    darkening along the rule, of 255      ours 36.75   mupdf 38.67   poppler 13.91
                                         gs (aa) 5.90   gs (aliased) 0.07
```

## Why it goes away when the reader zooms

`crates/pdf-model/examples/uncovered_share.rs` is the instrument this round adds: it splices an
opaque page-covering fill into the display list at a chosen depth, draws the page twice with that
fill black and white, and reports the share of it the marks above failed to cover — counting only
pixels all eight of whose neighbours are also touched, because a pixel on the *outer* edge of what
the marks cover is supposed to be partly painted and a page-wide mean would be nothing but those.

```text
  the owner's page, witness spliced under the polygons
    scale        interior pixels        lost        mean     worst
     1.00                 261 908    50 720.8      0.1937    0.4235
     2.00               1 053 910   135 113.5      0.1282    0.6980
     4.00               4 233 411   284 957.9      0.0673    0.8706
     8.00              16 970 766   265 057.3      0.0156    0.6039
```

Roughly half per doubling, which is the boundary-over-area argument and is exactly what the owner
observed. Page 100 of ISO 32000-2, whose regions are each stated once, reads 0.0105 at 2× and
0.0017 at 4× by the same instrument.

## The decision

**Record it as a known artefact with its cost, and gate the arithmetic rather than the artefact.**

`abutting_marks.rs` asserts three things and each is the standard's:

- §10.7.4's control — one statement of a region leaves none of the backdrop. Without it a backend
  that painted nothing would pass every bound below.
- §11.3.7.3 as an **upper** bound — two marks that abut leave no *more* than the union does. A
  backend that composed coverage some other way (a second multiplication, say, which is the shape
  of `doc/todo/11` item 4's remaining defect) leaves 0.4375 and fails.
- Both backends answer alike.

The bound is upper on purpose: **a rasteriser that resolved the marks against one another and left
nothing would pass this file unchanged**, so building one costs no test rewrite. That is the
difference between recording an artefact and ratcheting it, and `sub_pixel_coverage.rs`'s own
header states the same rule for the same reason.

What such a rasteriser costs is written down in `doc/todo/11` item 5 rather than left as a silence:
either draw at *N*× and box-filter down — `N²` of the rasteriser and of the raster, on the path
where this project has said latency is the feature, and still not exact, as `gs`'s 0.2200 shows —
or keep each boundary pixel's marks as sub-pixel geometry until the pixel is finished, which is
this backend's own blitter and has to answer what a blend mode and a transparency group do to a
fragment list. Neither is started. The artefact is worst at a page fit and gone by 8×, and both
cures cost their most on the frame time-to-first-page is measured from; that trade is a decision
for a round that measures both ends, not for this one.

## What was not the answer, checked

- **Not a clip.** The page states none: `0 distinct clips referenced`.
- **Not the sub-pixel machinery.** The polygons are three to seven pixels across, an order above
  `sub_pixel_bands`' quantum, and the frame rule is 1.75 device pixels wide.
- **Not a folding this tree owes.** §11.6.2 forbids compositing portions of *one* object, and ADR
  0213 folds a tiling's restatements of one mark on exactly that sentence. These are 58 003
  separate `f` operators with 57 413 colour changes between them; they are 58 003 objects, and
  §11.3.7.3 is what the standard says to do with two objects.
- **Not quorra's, and not vello's.** Measured, above.

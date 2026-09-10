# ADR 0945 — "One device pixel wide" is a family of widths, and a band is thin along one of them

Status: accepted. Session 944.
Clauses: ISO 32000-2 §10.7.4 (both of its rules — which pixels a shape reaches, and that the
painted area is at least the shape's), §8.4.3.2 (a device width varies with orientation under an
uneven CTM), §8.4.3.3 (a cap is a shape of the line's width), §10.7.5 (the rule that is *not*
being performed here, because Table 51 initialises `/SA` to `false`).
Code: `crates/pdf-render/src/sub_pixel.rs` (`substitute_width_across`, `band_substitute_width`,
`enlarged_mark_at`), `crates/render-cpu/src/lib.rs` (`draw_rule_at_one_pixel`).
Tests: `crates/render-cpu/tests/anisotropic_sub_pixel_stroke.rs`'s two, and
`crates/pdf-render/src/sub_pixel.rs`'s
`a_bands_substitute_width_is_one_device_pixel_across_the_band`,
`under_a_similarity_every_direction_gives_the_same_substitute`,
`a_paths_substitute_is_the_narrowest_its_own_directions_ask_for` and
`a_cap_is_enlarged_to_the_width_of_the_body_it_caps`.
Measurement: `crates/pdf-model/examples/anisotropic_band_census.rs`.
Documents: §10.7.4's ledger row, `doc/todo/00` step 7's record of the page this came off.

## Context

`doc/todo/00` step 7's ink sweep names three pages at or past −1 of 255 on documents the gate calls
**complete**, and each carries a diagnosis: `issue16038.pdf` (`AMBIGUOUS_TILING_CELL_CLIP`),
`issue12295.pdf` (`AMBIGUOUS_EVERYONE_OVER_THE_GEOMETRY`) and `issue14297.pdf`
(`AMBIGUOUS_DENSE_TEXT_AT_PAPER_SIZE`). All three diagnoses are ink ladders: our ink against the
references', taken at rising resolution, converging on the geometry the file states.

Two of the three survive being opened, and the measurements are in `doc/todo/00`. The third does
not, and the reason it does not is trap 1 stated as a mechanism rather than as a caution: **an ink
ladder is a measurement of *how much*, and this page's defect conserved the ink exactly.**

`issue12295.pdf` page 1 is a 24-hour Holter report — a heart-rate chart and eight ECG strips. Every
reference draws a legible ECG; this tree drew a featureless grey smear with no QRS complexes in it
at all. The page's note has recorded that as "our ECG traces are a ghost either way while all four
references draw them dark" since the five-hundred-and-eighty-third session and attributed it to
`doc/todo/11`'s sub-pixel item, on the strength of a ladder that says we are on the geometry and
the references are over it. The ladder was right and the attribution was wrong.

## The mechanism

Expanded, the document states 65 868 single-segment strokes, each `1 w`, under

```text
  0.24 0 0 -0.24 0 792 cm            the page
  0.569305 0 0 0.022590 … cm         each ECG strip
```

so the placement that reaches a stroke is `diag(0.1366332, −0.0054216)`: **two stretches a factor
of 25 apart.** §8.4.3.2 says what that does — "[i]f the CTM specifies scaling by different factors
in the horizontal and vertical dimensions, the thickness of stroked lines in device space shall
vary according to their orientation" — and the arithmetic is that a band of path width `w` swept
along `u` has device thickness

```text
  t = w · |u| · |det T| / |T u|
```

which on this page runs from 0.0054 of a device pixel for a horizontal segment to 0.1366 for a
near-vertical one. The QRS spike is the second: 0.1366 of a pixel wide and near-black, which is a
faint but perfectly visible column.

`render-cpu`'s substitution for a mark the raster cannot measure (ADR 0268) restates the rule at
one device pixel and divides the paint's alpha by the same factor. **The width it used is
`substitute_width`, `1 / min_stretch`** — one device pixel across *whichever way the mark runs* —
and that is 184.4 path units here, which is 1.0 device pixel along `y` and **25.2 along `x`**. So
the spike was drawn as a 25-pixel-wide band at an alpha of 1/184.4, and two things followed:

- **Its ink went to pixels the shape does not reach.** §10.7.4's opening sentence — "[a] shape
  shall be scan-converted by painting any pixel whose half-open square region intersects the shape,
  no matter how small the intersection is" — is about *which* pixels, and the widening this module
  performs is bounded by the pixel grid precisely because a band of the true width lies inside the
  pixel line the substitute stretches it into. Twenty-five pixels is not that bound; it is ink a
  dozen pixels from the mark.
- **A fifth of the ink was lost as well**, which is why the conservation argument does not save it.
  The alpha rides in eight bits: 1/184.4 is 1.383 levels and an eight-bit raster holds one, so the
  identity that makes the widening exact fails by 28% before any pixel is touched.

The same width was reaching §8.4.3.3's cap through `enlarged_mark`, so on this page each of the
65 859 round caps was a 25.2 × 1.0 device-pixel ellipse whose `(w/W)²` coverage — 2.9 × 10⁻⁵ — was
floored to one level by `expressible_coverage`. That floor exists so that no mark disappears; over
a shape 133 times too large it deposits 133 times the ink instead.

## Decision

**Ask the band's own question.** `substitute_width_across(T, u)` returns `|T u| / (|u| · |det T|)`,
the path-space width at which the band swept along `u` is one device pixel thick;
`band_substitute_width(path, T)` takes it over the directions the path itself states and keeps the
**narrowest**, so no part of the substitute is wider than the pixel line the true band lies in.
`render-cpu`'s general construction uses that, and states the cap at the same width through
`enlarged_mark_at`, because §8.4.3.3's cap is a shape of the width of the body it caps.

`substitute_width` stays exactly what it was and keeps its callers: §8.5.3.2's dot and Table 53's
cap **are** shapes of the line's width in every direction, and nothing narrower than
`1 / min_stretch` makes one of those a pixel across. The correction is that a band is not one of
them, and the two questions had one answer.

Under a similarity — every page transform — `|T u|` is `s · |u|` and `|det T|` is `s²`, so every
direction returns `1 / s` and this is `substitute_width` to the bit. That is why the change reaches
almost nothing.

### What is deliberately not done, with its cost

Where a path states several directions that disagree, the narrowest is a *bound* and not the exact
answer: the directions that would have wanted more are left thinner than the raster can measure.
Exactness there needs a width per segment and therefore a draw per segment, which loses the joins
between them and pays a seam — `1 − (1 − a)(1 − b)` where one scan conversion would have added.
That is not taken on this round, and the population it would buy is measured rather than guessed.
`anisotropic_band_census` over `doc/pdf.js`'s and `doc/corpora/pdfbox`'s 975 first pages prints two
distributions — what the old width made of a band, and what the new one leaves the *thinnest*
direction of a path:

```text
  strokes reaching the widening                                4 010 408
    old width: substitute band one device pixel wide           3 958 994
    old width: two to twenty-five device pixels wide              51 414
    new width: every direction's band at one device pixel      4 010 406
    new width: thinnest direction left 1/2.6 of a pixel                2
```

**The residual is two strokes**, both on `issue6127.pdf`, whose page is the only one the narrowest
width leaves a direction under a pixel on. `issue12295.pdf`'s 53 417 are all single-segment, so the
new width is exact on every one of them. A path holding a curve has a direction that varies along
it and falls back to `thinnest_line`, the width no direction whatever exceeds a pixel at.

## Consequences

Measured on a probe stating the mechanism with no page around it — one turned rule under
`diag(1/8, 1/200)`, whose geometry is 6.2532 device pixels of ink:

```text
                        ink        columns marked in one raster row
  before               4.9412                 26
  after                6.2745                  3
  the geometry         6.2532            it crosses at most 2
```

and on the document:

```text
  issue12295.pdf page 1      1×       2×       4×       8×
  before                   8.0147   7.2058   6.8566   6.8047
  after                    7.1795   6.8781   6.8262   6.8047
```

The two columns meet at eight times, and that is the control rather than a coincidence: at 8× a
`1 w` stroke is over one device pixel wide, so `draw_sub_pixel_rule` declines and neither the old
width nor the new one is asked for — the page's own geometry, 6.8047, is what both draw there. The
ladder now descends onto it from 5.5% above where it stood 17.8% above; the QRS complexes are
drawn; and the smear is gone. The verdict does not move — the page is
`ambiguous` before and after, for the reason its group's note gives, since all four references
floor a sub-pixel stroke at a device-pixel width and no two of their floors agree.

Nothing else in the corpus moves against the *references*. `issue6127.pdf` page 1 moves by 0.0002
of a level; `issue16038.pdf`, `issue14297.pdf` and `22060_A1_01_Plans.pdf` are byte-identical; the
oracle's three counts are unchanged.

**One page moves against the other backend, and it moves off a list.** `bug1844576.pdf` leaves
`render-raster/tests/corpus.rs`'s `DIFFERS_IN_SHAPE`, where its line was mean 2.1265, worst tile
5.02 at (0, 0), differing 0.0721, ssim 0.98080. The census says the page states one sub-pixel
stroke whose placement is not a similarity, and raster has never had this defect — its anisotropic
route outlines a stroke in path space at the width the document stated, which is the geometry — so
the agreement is the processor arriving where raster already was rather than either backend moving
toward the other. That is also the independent check on the fix that this round's own probe is
not: a second implementation, written to a different construction, was already drawing the answer
the new width produces.

**And the sweep's own number goes the "wrong" way, which is the finding rather than a regression.**
Step 7 measures our ink minus the lightest reference's, so a page whose over-ink was removed reads
further below the references than it did. `doc/todo/00` carries that beside the reading, because a
sweep row is a difference between two programs and this round moved one of them onto the geometry.

## What the round is really about

The three pages were taken because they are the only ones on the sweep's negative tail that sit on
documents the gate calls complete. Two are diagnosed correctly and are recorded as such. The third
had a diagnosis that was *true* — our ink is nearer the geometry than any reference's — and it was
true of a page drawn wrong, because both of its errors were errors of **placement** and the
instrument that held the page measures **quantity**. `doc/todo/00` step 5's own line says the
closed form answers "how much" and is silent on "where"; this is that line arriving on a page
nobody had opened, and the general shape is trap 1's: a note made entirely of numbers has not
looked at the page.

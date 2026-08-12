# 0285 — The quantum is inclusive, and the hairline was short at exactly one pixel

**Status.** Accepted.
**Context.** `doc/todo/11`'s one named open item, priced there as "a round of its own … because the
change is one comparison, and it moves what **every** page with an ordinary hairline draws".

## What was wrong

`tiny-skia` chooses its hairline for every stroke width **up to and including** one device pixel.
A hairline lays one pixel down per step along the line's *longer* device axis, so it carries
`cos θ` of the rule's area — and ADR 0268 stopped strictly *under* the quantum, so a rule exactly
one device pixel wide kept it:

```text
  a 200-unit rule, one device pixel wide, total ink against its own 200
                    hairline (before)   the fill of the same outline
    30 degrees            173.20                  199.73
    45 degrees            141.42                  177.44
```

**−29.3% at 45°, on every `1 w` stroke at the page's own scale**, which is most of the line work
in every technical drawing. ISO 32000-2 §10.7.4 makes it a `shall` and names the case:

> The area covered by painted pixels shall always be at least as large as the area of the original
> shape. This rule applies both to fill operations and to strokes with non-zero width.

The shortfall is not an artefact of this tree's anti-aliasing departure. Under the clause's own
binary model a band of width 1 at 45° and length `L` has area `L`, and the hairline paints one
pixel per column — `L/√2` of them. The `shall` is broken either way it is read.

The boundary that produced it was `tiny-skia`'s `<=` rather than anything derived, and it left a
one-point discontinuity: 0.999 of a pixel was filled, 1.000 was a hairline, 1.001 was filled again.

## What was done

`at_or_under_the_quantum` — one comparison, `<` to `<=`, with the two constructions split by which
one may snap:

- **The general construction is inclusive.** A rule at or under one device pixel is stroked at the
  substitute width with the width it gave up in the paint's alpha; at the quantum itself that
  factor is 1, so it is exactly "the fill of the same outline".
- **The exact construction stays strictly under it.** `pdf_render::sub_pixel_bands` draws a rule
  *thinner* than a pixel as the pixel line it lies in; a rule that is exactly one pixel wide spans
  two lines at a fractional offset, and snapping it onto one would be §10.7.5's automatic stroke
  adjustment performed without `/SA`. ADR 0208 forbids that and
  `render-cpu/tests/zero_area_fill.rs` pins it.
- **The cap comes back at the quantum.** ADR 0268 drops the cap because widening overstates it by
  `width / style.width`; at the quantum that factor is exactly 1, so dropping it would be a pure
  loss of half a pixel at each end rather than a trade. This is that ADR's own arithmetic read at
  the boundary rather than a change to it.

## The `0 w` stroke follows it, and that is a choice

§10.7.4 exempts one mark by name — "Zero-width strokes may be done in an implementation-defined
manner that may include fewer pixels than the rule implies" — and the two arrive at the rasteriser
indistinguishable, because `pdf_render::Stroke::device_width` promotes a zero width to exactly one
device pixel. So the exemption was implementable, and was implemented first, and then removed.

Three reasons, in order of weight:

1. **The permission is a `may`.** Declining it conforms.
2. **§8.4.3.2 states the width as a `shall`**: "A line width of 0 shall denote the thinnest line
   that can be rendered at device resolution: 1 device pixel wide." A staircase one pixel wide
   *along an axis* is thinner than that measured across the line. Neither reading is forced — the
   hairline is the conventional one — so this is genuinely underdetermined.
3. **Which is why the project's own rule decides it**: `device_width` resolves a zero width in the
   *shared* crate so that both backends draw one mark, and quorra strokes exactly that. Keeping
   the hairline in `render-cpu` alone would be one backend privately re-deciding what `pdf-render`
   had decided, and the two would differ by 29% on every turned `0 w` line with no clause to
   arbitrate. That is trap 2's sentence: where two backends are the oracle, a decision either can
   make alone is a decision neither has made.

**The corpus cannot rank the choice, and that was measured rather than assumed**: the whole gate
is byte-identical with the exemption in and with it out — 915 agree, 37 differ, 5 refused, the same
pages. It is written down as a choice for that reason.

## What it cost, measured on three instruments

- **The reference oracle does not move at all.** 905 agree, 68 contradicted, 786 ambiguous, 1 our
  geometry, 2 reference geometry, 14 not comparable, 18 no render — identical before and after,
  taken by stashing the change and re-running.
- **`doc/todo/00`'s step-7 ink sweep holds.** Over all 854 artefact pages, 33 rows moved and 31 of
  them **up**, by 0.001 to 0.567 of 255 — which is the `shall` being satisfied, made visible. The
  negative tail is unchanged: `issue16038` −5.734, `issue12295` −2.956, `issue14297` −1.150,
  `issue7821` −1.000, and nothing new at or past −1. **No page lost content.**
- **The cross-backend gate costs two pages**: 917/35/5/17 → 915/37/5/17. `bug1743245.pdf` and
  `issue2177.pdf` cross a bound in its third decimal place, `bug1844583.pdf` and `issue21068.pdf`
  join on similarity, and `issue14415.pdf` and `knockout_groups_test.pdf` leave. That is not one
  backend drawing the wrong thing: `render-cpu` has moved *onto* quorra's construction, so what
  shows on those pages is the two rasterisers' coverage quanta on many more edges than before.
  Against the references two of the four improved — `bug1844583.pdf` ssim 0.6152 → **0.8738**,
  `issue21068.pdf` mean 10.06 → 9.42 — and two worsened in the second decimal place.

**A null on the arbiter, a `shall` satisfied, and two pages of cross-backend churn.** The trade is
taken because the clause is not ambiguous about a stroke with non-zero width, and because the
churn is at bounds this project chose rather than at anything the standard states.

## The test that had to move, and why it is the finding under the finding

`zero_area_fill.rs::a_flat_fill_carries_a_hairline_strokes_ink_at_its_own_placement` pins ADR
0208: a zero-height *fill* is snapped to the whole device pixel row §10.7.4 names, and a `0 w`
*stroke* keeps the coordinates the document gave it. It failed at scale 2 with the two rasters
**byte-identical**, and neither construction was wrong.

`tiny-skia` supersamples four times per pixel row and takes each sub-row's sample at its centre. At
the test's placement the stroke's band lands at 298.9 to 299.9 at scale 2 — and row 299's four
sample lines are the only whole set inside it, so the band renders as one full row, which is
exactly what the snapped fill renders as. **A displacement of a tenth of a pixel is below the
instrument's own quantum.**

So the constant moved from `50.3` to `50.125`, which splits the ink across two rows at both scales,
and the lesson is one this project already holds one level up: a test must be placed off the
*rasteriser's sample grid*, not merely off the pixel boundary, or it asserts nothing while looking
as though it asserts something. The test's own comment now says so.

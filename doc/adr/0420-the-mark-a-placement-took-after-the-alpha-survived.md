# ADR 0420 — The mark a placement took after the alpha survived

Status: accepted, 2026-08-18. Session 585.

ADR 0290 gave §8.5.3.2's dot a substitution and ADR 0419 gave the substitution's alpha a floor, and
the dot was still drawn as **nothing** at 0.1, 0.05, 0.02 and 0.01 of a device pixel — on the
processor, on quorra, and (unmeasured until this round) on vello. ADR 0290's own ladder printed the
0.1 row as `-100.0%` from the day it landed and the gate's rungs started at 0.2, which is
`doc/HANDOVER.md`'s trap 1 exactly: a number on screen that nobody read.

Session 584 priced the fix and declined it, in one sentence that this round had to check rather than
inherit:

> The alpha floor cannot recover it: the substitute is a circle *inscribed in* one device pixel and
> covers none fully, and stating it as a whole pixel is snapping a mark that has a width, which
> §10.7.5 conditions on `/SA` and ADR 0208 declines.

The first half is half right and the second half is wrong about what the construction would be.

## 1. Why nothing was drawn — attributed, not guessed

Four suspects, and `doc/habits.md`'s *Measuring* says to remove one rather than reason about all
four. The one that separates them is the clause's own word:

> This ensures that no shape ever disappears as a result of **unfavourable placement relative to the
> device pixel grid**, as might happen with other possible scan conversion rules.

So `render-quorra/examples/sub_pixel_marks`' dot ladder grew a **placement** column — the same dot,
the same width, the same alpha, drawn once with its centre on a device pixel's corner and once on a
device pixel's centre. Before this round, at scale 1:

```text
  width   placed at         backend   total ink   its own area   error
   0.15   a pixel's corner  cpu          0.0157         0.0177     -11.2%
   0.15   a pixel's centre  cpu          0.0157         0.0177     -11.2%
   0.10   a pixel's corner  cpu          0.0000         0.0079    -100.0%
   0.10   a pixel's corner  quorra       0.0000         0.0079    -100.0%
   0.10   a pixel's centre  cpu          0.0078         0.0079      -0.1%
   0.10   a pixel's centre  quorra       0.0078         0.0079      -0.1%
   0.05   a pixel's centre  cpu          0.0039         0.0020      99.7%
   0.05   a pixel's centre  quorra       0.0000         0.0020    -100.0%
```

That answers it outright and rules out three of the four suspects. The substitution **is** reached
(the centre row draws the geometry's own ink); the contour is **not** dropped by either library (the
same contour draws half a pixel over); the path does **not** degenerate before the rasteriser sees
it. The coverage is computed and then lost, and the arithmetic says exactly where:

- `enlarged_mark` states the dot at one device pixel with an alpha of `(w/W)²`, which at 0.1 is
  `0.01` — **2.55 levels of 255**, comfortably above the floor ADR 0419 built. The alpha survived.
- A circle one device pixel across covers `π/4 = 0.785` of a pixel when its centre sits at one, and
  `π/16 = 0.196` of each of four when its centre sits on a corner. What a pixel receives is the
  alpha *times* that coverage: 2.0 levels at a centre, **0.5 levels at a corner**, and half a level
  is what an eight-bit raster rounds away.

So an alpha of one level is not a level of ink, and ADR 0419's floor was applied to the wrong
quantity — it floors the alpha where what has to clear a level is the alpha times the shape's own
coverage in some pixel. The 0.05 row shows the other half of the same fact: the processor's floored
alpha lands at a *centre* and not at a corner, while quorra, which had no floor, lands at neither.

**And it is not any rasteriser's defect.** Given the true circle, 0.5 of a level in four pixels is
the exact analytic answer met by the raster's depth; quorra returns exactly that at both placements.
`doc/QUORRA_FEEDBACK.md` §32 records the finding with no ask attached.

## 2. What was implemented

`pdf_render::point_mark`, and it turns on one shape whose coverage is not a fraction. §10.7.4 names
it:

> let i = floor( x ) and j = floor( y ). The pixel that contains this point is the one identified as
> ( i, j )

and §8.5.3.2 gives the mark a single point to floor — "producing a filled circle centred at the
single point". So a mark centred at a single point is stated as **the device pixel its own centre
lies in**, painted at the coverage its own area implies, as soon as the widened form can no longer
put one level into any pixel it covers. Above that it keeps ADR 0290's widened circle, where the
mark still has the shape the clause names.

The threshold is derived rather than tuned. Every mark this reaches lies inside a disc of `√2`
device pixels about its centre — §8.5.3.2's circle is one pixel across, Table 53's square is one
across each side and `√2` across its diagonal — and a span of `√2` pixels meets at most three pixel
columns and three rows. So some pixel holds at least a **ninth** of the mark's area, and where a
ninth of the area is under one level no pixel can hold a level, whatever the placement. For the
circle that is `w ≥ sqrt(9 / (255 · π/4)) = 0.212` of a device pixel.

Total ink is the mark's own area on both sides of the boundary, so the two constructions differ
about *where* the ink is and not about how much of it there is. Below the width at which the area
itself is under one level (`0.0707` for the circle), `expressible_coverage` states the least the
raster can hold, which is ADR 0419's rule reaching the mark that needed it.

**The decision is in `pdf-render` and all three backends take it**, which is trap 2's rule rather
than a convenience: the mark was lost on the processor *and* on quorra *and* on vello, by three
routes with one cause, and a device decision either backend can make alone is a decision neither has
made. `split_degenerate` and `split_dash_marks` now take the transform and return the coverage
beside the shapes; `render-cpu`, `render-gpu` and `render-quorra` each multiply their paint's alpha
by it.

### Why this is not §10.7.5's stroke adjustment, which ADR 0208 declines

ADR 0208's rule is that a mark with a stated width may not have its coordinates snapped, because
that is §10.7.5's requirement and §10.7.5 is conditioned on `/SA`, whose Table 52 initial value is
`false`. Three things separate this construction from that one:

- **Nothing is promoted.** §10.7.5 makes a sub-half-pixel stroke "a single-pixel line" — a whole
  pixel of ink. Here the pixel is painted at the mark's own area: a fiftieth of a pixel of ink where
  that is what the mark covers, two levels of 255 at 0.1.
- **No coordinate moves to a grid line.** The pixel a mark lands in is the one its own centre is
  already in, which is the clause's own flooring rather than a rounding of the geometry. The mark
  cannot move into a pixel it does not touch.
- **What is given up is sub-pixel *shape*, not position** — and an eight-bit raster cannot hold that
  anyway, which is the whole finding above. It is the same trade `sub_pixel_bands` has made since
  ADR 0226, one dimension further.

The construction is also, at these widths, *closer* to the geometry than the one it replaces: the
widened circle spreads a mark 0.1 of a pixel across over four pixels the true dot does not touch,
where this one puts it in the pixel the true dot is in.

### What was considered and not done

- **Flooring the alpha by the worst-case coverage** — `alpha ≥ (1/255)/(π/16)` — which keeps the
  widened circle and needs no new shape. Measured against the alternative: it deposits four levels
  where this deposits one, at every width below 0.14, because the alpha it needs is set by the worst
  placement and then applied at every placement. It is a larger departure from the geometry for the
  same guarantee.
- **Using the pixel construction at every sub-pixel width.** Simpler by one branch, and refused on a
  measurement: at 0.9 of a device pixel the widened circle is a circle and the rasteriser draws it
  correctly, and replacing it with a hard-edged pixel would un-anti-alias a mark the device can
  express. The substitution is a rescue, not a policy.
- **Concentrating §8.4.3.3's caps the same way.** Declined for this round with a reason rather than
  by omission: a cap sits at the end of a body that lands — `issue12295.pdf`'s 0.1366-pixel rules
  carry 35 levels in their bodies — so the shape §10.7.4 forbids from disappearing does not, and
  what a cap loses is a share of one mark's ink. It also abuts the body's own substitute, so
  concentrating it raises the "one draw rather than two" question `doc/todo/11` prices. Left there,
  with the distinction written down.

## 3. What it costs, measured

The dot ladder at scale 1, before and after, both backends identical at every rung after:

```text
  width    before (corner / centre)      after (both placements)   its own area
   0.50      0.1882 / 0.1882              0.1882                       0.1963
   0.25      0.0471 / 0.0471              0.0471                       0.0491
   0.20      0.0314 / 0.0314              0.0314                       0.0314
   0.15      0.0157 / 0.0157              0.0196                       0.0177
   0.10      0.0000 / 0.0078              0.0078                       0.0079
   0.05      0.0000 / 0.0039 (cpu)        0.0039                       0.0020
   0.02      0.0000 / 0.0039 (cpu)        0.0039                       0.0003
   0.01      0.0000 / 0.0039 (cpu)        0.0039                       0.0001
```

The 0.15 row is `+11.0%` and is the raster rounding 4.5 levels to 5; the rows at 0.05 and under are
the one-level floor, heavier than the geometry, which is the side of "[t]he area covered by painted
pixels shall always be at least as large as the area of the original shape" that the `shall` is on.
Above 0.212 nothing moved, which is the check that the widened construction was left alone.

**The device backends changed too**, and neither had been measured here before: quorra drew nothing
at 0.1 and below at a corner and at 0.05 and below at either placement, and vello — `headless_gpu`'s
new `a_sub_pixel_dot_marks_the_devices_raster` — drew `0.01569` at 0.2 against an area of `0.03142`
and nothing at 0.1, 0.05 and 0.01. Both are on the geometry now.

**The population, measured before the price** (trap 11). `pdf-model/examples/sub_pixel_width_census`
grew a line for §8.5.3.2's two marks and was run over the first page of every PDF on this disk —
1242 of 1263 read:

```text
  §8.5.3.2 dots               97 marks over 7 documents, 46 under a device pixel, 0 under 0.212
                              thinnest stated: 0.36 of a device pixel
  zero-length dash patterns   30 commands over 1 document (issue14297.pdf), all at 0.5030
```

So **no document on this disk reaches the new construction** — it is an unwitnessed `shall` taken
because it is one, the same shape as ADR 0419's floor and ADR 0154's collapsed fill. What the corpus
*does* witness is the other half of the change: the 46 sub-pixel dots between 0.36 and 1.0 now take
ADR 0290's widened form on **all three** backends, where before only the processor did.

Gates: the cross-backend gate's whole output is byte-identical before and after, the oracle's
verdict counts are unmoved, and the corpus gate is unchanged — which the census predicts, since the
processor's answer is unchanged for every width at or above 0.212 and the corpus states none below
0.36.

## 4. The gate, and that each new rung fails without the fix

`doc/HANDOVER.md` trap 2's fifth instance: a scene must fail at the defect's magnitude. The rungs
were 0.2, 0.5, 1.0 and 2.0 and the defect lived under them.

- `a_dot_lands_in_one_pixel_at_every_width_and_every_placement` (both backends) walks 0.2, 0.15,
  0.1, 0.05, 0.02 and 0.01 at **both placements**, and holds each to one level of 255 of
  `max(area, one level)` — an absolute bound, because down there what a raster can be held to is a
  level and not a percentage.
- `a_sub_pixel_dot_marks_the_devices_raster` does the same reading of vello's raster, absolutely
  rather than by cross-backend comparison: one lost pixel in forty thousand moves no
  differing-channel fraction, so the comparison gate could never have seen this.

Both were run with the fix removed by `git apply -R`: the first fails at
`a 0.1-unit dot at a pixel's corner did on the processor`, the second at
`a 0.2-unit dot drew 0.01569 of ink where its own area is 0.03142`.

## 5. The specification half, and one erratum in this round's own clause

`spec-errata emit doc/*.pdf` over clauses 8 and 10 before writing, which is `doc/todo/02` §4's rule
and which paid twice:

- **§8.5.3.2, Issue #103 (`Review`/`Completed`)** strikes "This" for "In the opaque imaging model,
  this", so the dash sentence reads *In the opaque imaging model, this rule shall apply only to
  zero-length subpaths of the path being stroked*. `doc/md/` still carries the unqualified form —
  the same shape session 584 found for §10.7.2 — and this tree quotes it verbatim in three files.
  The erratum adds a scope rather than an answer, and §11.6.2 already makes a stroked path one
  object in the transparent model, so nothing changes but the annotation. Issue #434 adds a NOTE
  that states no requirement.
- **§8.5.3.1, Issue #549 (`Review`/`Accepted`, 2026)** strikes "generate an error" for "be ignored",
  so a painting operator invoked with no current path is now a no-op by the clause. That row's one
  recorded departure — "here it paints nothing, which is the recovery a viewer owes a malformed
  file" — has become agreement. Corrected.

§10.7.4 and §8.4.3.2 are untouched by the collection, which is what ADR 0419's reading rested on and
still does.

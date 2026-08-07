# ADR 0217 — A mask is one number, and a group is three

Status: accepted, 2026-08-07 (session 380).

## Context

`doc/todo/23` names four transparency departures, each refused *by name* and together the
largest corpus-demand group left. Two of them are the same sentence at two levels:

| | corpus | what it is |
|---|---|---|
| a blending space that is not the device's three components | 4 | §11.4.7, all `/DeviceCMYK` |
| a soft-mask group with such a space | 7 | §11.6.6 and §11.5.3, the same question inside a mask |

The todo file states the shape of the work: taking either means compositing in a space the
backends do not have, so **the first question is not the clause but the display list — what
would a backend have to be handed**. ADR 0210 is the precedent: a display list can name a thing
a backend resolves, and the interpreter's ignorance of the device is not the blocker.

**The mask population was chosen, and the reason is that §11.5.3 answers the question and
§11.6.6 does not.** A painted group's result is a colour — three components at every pixel, in
a space a rasteriser would have to be taught. A *mask* group's result is one number:

> The second method of deriving a soft mask from a transparency group shall begin by
> compositing the group with a fully opaque backdrop of a specified colour. The mask value at
> any given point shall then be defined to be the luminosity of the resulting colour.

So the mask population is the one where the display-list question has a smaller answer than the
one it was asked, and the other three now owe what is written at the end of this file.

### What the report was firing on, and what was actually wrong

The condition was the group's `/CS` alone: any space that was not `DeviceRGB`, a `CalRGB`, or a
one- or three-component ICC profile got the sentence *"a soft mask's group is composited in
device RGB and its luminosity taken there, rather than in the blending colour space its /CS
names"*. Seven corpus documents matched.

`examples/luminosity_mask_census` was written to find out what those groups actually paint, and
it is the round's first finding. Over 964 documents, 21 carry a `/Luminosity` soft mask and the
census reaches 90 mask groups between them:

| group `/CS`, as it parses | groups |
|---|---|
| `/DeviceCMYK` | 39 |
| `/DeviceGray`, named or in an array | 37 |
| an ICC profile of one or three components, or a `CalRGB` | 12 |
| `/DeviceRGB` | 2 |

**Not one of the 90 sets a `k` colour.** Every one of them paints a `DeviceGray` image, a
`DeviceGray` shading or a `g` fill — with two exceptions found only by this round's new report,
below. So the departure the sentence named lived almost entirely in the *backdrop*, and the
condition it fired on was three steps away from it.

**"Reaches" is the honest word and the number is a floor**: the census walks a page's resources
and every form `XObject`'s, which is not every path a `gs` can be named down. `bug1721218_reduced.pdf`
is the proof — the census finds its eight one-component ICC groups and none of the six
`DeviceCMYK` ones the interpreter sees. So the corroboration for the finding comes from the
interpreter rather than from the census, and it is the strongest measurement in this round:
**redirecting every colour in every luminosity mask group on all 974 documents moves one page.**

## Decision

### §11.5.3's device branch is carried out in the space the group names

The clause gives two branches, "depending on the group's colour space". The device one says to
"convert the colour to `DeviceGray` by implementation-defined means", and **§10.4.2 states those
means for every device space**, so nothing here is implementation-defined after all:

- §10.4.2.2 — `gray = 0.3 × red + 0.59 × green + 0.11 × blue`, which is §11.5.3's EXAMPLE 2's
  first formula and is what a rendered pixel already gives back.
- §10.4.2.3 — `gray = 1 − min(1, 0.3 × cyan + 0.59 × magenta + 0.11 × yellow + black)`, which is
  EXAMPLE 2's second formula and does not go near RGB.

`ColourSpace::ink` is the sum inside that `min` and `ColourSpace::luminosity` applies it.

### The display list is handed a grey, and that is the whole vocabulary change

**§10.4.2.3's conversion is affine in the components except for one `min`.** Source-over
compositing is affine in each component too, so a linear functional of a subtractive colour's
components composites exactly the way the components do. A mask group can therefore be
*painted* in that one number — as a grey, on the channel a rasteriser already has — and
`SoftMask::value` reads it back unchanged, because §10.4.2.2's three weights sum to 1.0 and the
grey of a grey is that grey.

`Compositing::Luminosity` is the interpreter's flag for it, set while a mask group whose
blending space is subtractive is run, and `Interpreter::colour` is the one place a colour
becomes a paint. `SoftMaskKind::Luminosity`'s `backdrop` keeps its type and changes its meaning:
it is `/BC` resolved into whatever the group's elements are painted in, because a backdrop and
the elements composited onto it have to be the same quantity.

**No conversion into the group's space is needed, and that is a result rather than a shortcut.**
§11.6.6 would have a `DeviceRGB` colour painted into a `DeviceCMYK` group converted by §10.4.2.4
first — black generated, undercolour removed — and then to grey by §10.4.2.3. Every term
§10.4.2.4 generates cancels there, because §10.4.2.3's three weights sum to 1.0:

```text
0.3(c − k) + 0.59(m − k) + 0.11(y − k) + k  =  0.3c + 0.59m + 0.11y
```

whatever `BG` and `UCR` returned. So an RGB colour taken through `DeviceCMYK` and back to grey
is §10.4.2.2's grey of the original, and a grey taken through `DeviceCMYK` is itself. Only a
`DeviceCMYK` colour needs an arm of its own, which is why `ColourSpace::ink` has exactly one.

### `DeviceGray` counts as subtractive, and that closed a silence

§10.4.2.3 calls a grey level "the complement of the black component of `CMYK`", and the same
conversion answers a `DeviceGray` group. A `k` colour inside one was the same departure one
component narrower and had **never been reported at all** — the old condition treated
`DeviceGray` as exact, which it is for grey artwork and is not for CMYK artwork. 36 of the
corpus's 90 reachable mask groups blend in `DeviceGray`, 37 of them once the array forms are parsed.

### Three residues, each reported by name

- **The colorimetric branch.** "For CIE-based spaces, convert to the CIE 1931 XYZ space and use
  the Y component" — this tree answers with the grey of the sRGB it converts every CIE-based
  space to. That is the same page-wide documented choice §11.6.6 already records, not a second
  view of it, and it stays silent for `CalGray`, `CalRGB` and ICC. `Lab` is reported: its three
  components are not a linear map of the device's, so *neither* route is the clause's. **No
  corpus document states one**, which is said out loud rather than around.
- **More than one unit of ink.** §11.5.3 puts §10.4.2.3's `min` *after* the compositing, and a
  rendered channel holds `0..=1` where an ink reaches 2.0. `/BC [1 1 1 1]` — registration black,
  the commonest backdrop a `DeviceCMYK` mask has here — is the case, and the excess is clamped
  before the compositing rather than after. The cost is a closed form: for artwork of ink `s` at
  coverage `α` over a backdrop of ink `1 + e`, the clause gives `max(0, 1 − α·s − (1 − α)(1 + e))`
  against this tree's `α·(1 − s)`, equal at `α = 0` and `α = 1` and apart by at most `(1 − α)·e`
  between them. So it lives at the partly covered pixels of the group's own marks and nowhere
  else — the one-pixel rim of a photograph, on all five of this corpus's witnesses.
- **Colour that arrives already rasterised.** An image's samples and a shading's ramp are RGB
  before a display list can carry them, and where their own space rests on `DeviceCMYK` the grey
  of that RGB is not §10.4.2.3's grey. Reported with the space named.

### The alternative that was built and taken out again

Carrying the ink itself — `backdrop: f32`, unclamped, with the `min` applied once in
`SoftMask::value` — is arithmetically exact and was written before it was withdrawn. Two things
killed it, and both are worth the paragraph:

- **A backdrop grey below zero is what it comes to on the graphics device**, and
  `quorra-scene::Scene::mask` refuses that at the boundary with `SceneError::InvalidColor`. Its
  reduction shader clamps only at the end, so a negative backdrop would in fact produce the
  clause's answer — but relying on a library accepting a value its own validity test rejects is
  not a construction, it is a coincidence with a version number.
- **Scaling the group instead** — painting `1 − ink ÷ S` and folding the inverse into the
  transfer table, which both backends already apply to the byte they derive — needs *every*
  colour in the group scaled, and an image's samples are not colours this interpreter sees. A
  scale that a raster inside the group does not know about reads that raster's grey as `S` times
  the ink it means, which is worse than the departure it fixes.

What both alternatives were for is now `doc/todo/23`'s, with the closed form above as the size
of the prize.

## Consequences

### What moved on the gates

- **corpus 72 → 73 incomplete**, and the rise is a new report (trap 5): `bug1703683_page2_reduced.pdf`
  draws a `/DeviceN` shading, whose alternate rests on `DeviceCMYK`, inside a `/DeviceGray`
  luminosity mask group. Nothing in this tree had ever said so. Two documents lost the old
  report because the departure it named is gone — `issue14200.pdf`, whose `/DeviceCMYK` group's
  content stream is `q Q` and whose absent `/BC` is now the clause's 0 rather than 32, and
  `bug1721218_reduced.pdf`, whose six `DeviceCMYK` mask groups paint grey artwork — and five
  kept a narrower one, `/BC [1 1 1 1]`'s excess ink.
- **oracle 1686 → 1685 complete, ambiguous 749** on complete documents, with 857 agreeing, 68
  contradicted, 0/2 geometry and 9 not comparable all identical. Exactly **two pages of 1794**
  changed anything: `bug1703683_page2_reduced.pdf` page 1, whose numbers are identical to four
  decimal places and which is simply now labelled incomplete, and `issue13520.pdf` page 1, worst
  mean 6.55 → 6.54, differing 23.05% → 23.02%, structural similarity 0.8576 → 0.8580.
- quorra 914 / 42 / 1 / 17, text 99.2% (23641/23841) with 25 below the floor, dates, XMP and
  JPEG 2000 all unmoved.
- `doc/todo/00`'s step 7 over all 786 ambiguous pages: head unchanged, `issue16038.pdf` at
  −5.398 and every entry below it to a thousandth.

### The measurement

**On the corpus it is 62 pixels.** `issue13520.pdf` page 1 is the only page of 974 whose raster
moves: 62 of 18 810 pixels, by at most 28 of 255, in three vertical strips at the edges of a
soft-masked gradient; the page's ink goes 20.0823 → 20.0560 against `poppler`'s 17.6287 and
`mupdf`'s 16.5747, so it moves toward both. That is small because of the census's finding: the
corpus's mask artwork is grey, for which the two routes agree exactly, and its `DeviceCMYK`
backdrops are registration black, for which they agree as well.

**Where the clause's own arithmetic is visible, it is 32 of 255**, and that is what the fixtures
measure. A `/DeviceCMYK` mask group whose `/BC` is process black masks everything away —
`1 − min(1, 1) = 0` — and this tree drew it at 12.5% for as long as it drew luminosity masks at
all, because `CMYK_CORNERS` puts process black at `(35, 31, 32)` and its grey level is 32. Each
of the three new fixtures was checked by putting the old route back: all three fail, and the
number they fail with is 223, which is 255 − 32.

### What the other three populations now owe

- **§11.6.6's blending space for a painted group** (4 documents) is not this. Its result is
  three components, so the linearity that makes a mask one channel does not apply, and the
  display-list question stands exactly where `doc/todo/23` left it.
- **§11.4.6's knockout whose shape is not its coverage** (5) and **§11.4.4's NOTE 5 non-isolated
  group** (6) are untouched. Both are about a group's shape and its backdrop rather than about
  its colour space, and nothing here bears on either.
- **Inside the mask population**, two things remain and are now named rather than lumped: a
  colour of more than one unit of ink, whose size is the closed form above, and a raster in a
  subtractive space, which needs an image decoded into the group's own components.

### And a ledger row that had been wrong since it was written

§10.4.2.3 was `inapplicable`, on the reasoning that "[b]oth directions are for a device whose
native space is CMYK or grey. This one's is RGB (§10.2), so neither conversion is on any route
to a pixel". §11.5.3's device branch is that conversion, on a route to a pixel, on 76 of the
90 mask groups the census reaches. §10.4.2.2's note carried the same shape — "the NTSC weights that make
an RGB colour grey … is for a monochrome device and is asked of nothing here" — and every
luminosity mask on every page is that formula. Both corrected, which is `CLAUDE.md`'s own
sentence about a claim of inapplicability decaying, for the third time in twenty-four sessions.

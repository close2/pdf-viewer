# ADR 0024 — Soft-mask images at any resolution, and what §11.6 says about one object

Status: accepted, 2026-07-29.

## Context

ADR 0023 combined an explicit `/Mask` with its image on the finer of the two grids and left
the same question open one key away. §11.6.5.2 Table 143, of a soft mask's `/Width`:

> If a Matte entry (see "Table 144 - Additional entry in a soft-mask image dictionary") is
> present, shall be the same as the Width value of the parent image; otherwise independent of
> it. Both images shall be mapped to the unit square in user space (as are all images),
> regardless of whether the samples coincide individually.

Until this session a mask of any other size was refused and named. That cost three corpus
documents and fourteen images: `smaskdim.pdf` drew two bullets as squares, `issue16263.pdf`
drew black bars where a mask should have cut them to overline strokes, and
`chrome-text-selection-markedContent.pdf` carried twelve of them on one page.

The session's spec-track item was the rest of §11.6 — §11.6.4 having been read in the
fourteenth — plus §11.3.7 and §11.5, which §11.6 defers to. Seventeen ledger rows, and they
produced three defects the demand item would never have reached.

## Decision 1 — one function combines an image with a mask, whichever mask it is

`image::combine_on_the_finer_grid` is ADR 0023's grid choice with the mask's contribution
passed in as a closure. A stencil answers all or nothing (§8.9.6.3); a soft-mask image answers
its grey level (§11.6.5.2). The merit of one function is not that it saves twenty lines — it
is that the two masks now demonstrably agree about geometry, and a page cannot be masked
differently depending on which key the producer used.

Two things changed in the shared version:

- **The mask scales the base pixel's alpha rather than replacing it.** §11.3.7.1 makes alpha
  the product of shape and opacity, and §11.6.4.1 names the two as separate sources; an
  image's own shape is its stencil, an `/SMask`'s samples are opacity. Multiplying is what the
  clause describes, equals the old behaviour wherever the base image is opaque, and makes the
  order the two are applied in irrelevant.
- **The bound is on the growth, not on the total.** `MAX_MASK_GRID` refuses a combined grid
  above 2^24 samples, which is what stops a 2×2 image with a 34862×4332 mask from asking for
  604 MB. Applied flatly it also refuses a mask that is the *same size* as a large image —
  `issue19517.pdf` is a 12608×16806 scan — where combining costs exactly what the image
  already costs and `MAX_SAMPLES` has already admitted it. `combined_grid` allows the larger
  of the image's own size and the flat bound, which refuses the pathological pair and nothing
  else.

## Decision 2 — Table 143's restrictions are checked, because two of them decide behaviour

A soft-mask image dictionary is restricted, and a reader that trusts the restrictions instead
of checking them gets two of them badly wrong:

- **`/ImageMask` "[s]hall be false or absent".** A stencil decodes to the current colour where
  its bits mark and to nothing where they do not — it carries no grey level at all — so
  reading its first component as opacity makes the parent image *fully transparent*. A page
  silently missing its picture is the worst outcome available; the mask is refused and named,
  and the image draws opaque.
- **`/ColorSpace` is "Required; shall be `DeviceGray`".** A mask in some other space arrives
  from the ordinary image route as a colour, and no clause says which of its components is the
  opacity. §11.5.3's luminosity is a rule about a transparency group's colour, not about this
  key — the same confusion ADR 0023 refused for `issue6621.pdf`, and refused the same way.

No corpus document trips either. They are checks whose value is that the failure they prevent
is invisible: an image that vanishes reports nothing, and one masked by its red channel looks
plausible.

## Decision 3 — `/Matte` is undone where the arithmetic is exact, and reported where it is not

Table 144's `/Matte` says the parent image's samples have been pre-blended with a colour:
`c′ = m + α × (c - m)`. Drawing them as though they had not leaves the matte in every
partly-transparent edge — with a matte of black, which is what `issue13931.pdf` writes, a red
seal comes out with a dark fringe.

The clause is specific about where the inversion belongs:

> If a colour conversion is required, inversion of the pre-blending shall precede the colour
> conversion.

This crate holds one RGBA raster per image, so the conversion has already happened. The
inversion is still exact where that conversion was the identity on components, which is
`DeviceGray` and `DeviceRGB` — and those are inverted, in integer arithmetic, with the
clause's own clamp and its NOTE's answer for α = 0. Any other space is *reported*, because
dividing a converted byte by α computes something the standard does not describe.

The report accompanies the drawing rather than replacing it, which is the second place in this
tree where that happens (`/NeedAppearances` is the first) and it needs the same argument:
the mask itself is fully specified and applying it is right, while the pre-blending is a
defect in the colours. Refusing the mask because of the matte would draw an opaque rectangle
whose edges are *entirely* the matte colour — where α is 0, `c′ = m` — which is worse on the
page and no more honest.

## Decision 4 — a shading carries the alpha constant

§11.6.4.4 makes `ca` and `CA` properties of the graphics state applied to painting operations,
not properties of a colour. A shading *replaces* the colour rather than tinting it, so the
natural implementation returns the shading and drops the alpha along with the colour it did
not use. That is what this tree did.

`alphatrans.pdf` states `Gradient: .5` on its own face and draws a red-to-blue gradient across
three objects. All three references show them through it; we painted it opaque. The page had
been contradicted since the oracle existed and was filed under `CONTRADICTED_SUBSTITUTED_FONT`
because its labels are set in a font nobody embedded — the third time a group's name has
turned out not to be a diagnosis of its members.

`Shading::with_alpha` scales every colour a shading carries — a ramp's samples, a function
shading's grid, a mesh's corner colours — and `fill_paint`, `stroke_paint` and `sh` use it.
The clone is paid only where the constant is below 1, because a pattern set once paints every
path filled until the colour changes again.

## Decision 5 — §11.6.2's one-object rule is reported, with a condition that costs one page

> Portions of an object shall not be composited with one another, even if they are described
> in a way that would seem to cause overlaps (such as a self-intersecting path, combined fill
> and stroke of a path, or a shading pattern containing an overlap or fold-over).

`B` and its three relatives paint one object; this renderer emits a `Fill` and a `Stroke`, so
the band a centred stroke shares with the fill composites twice. Implementing it means
compositing the two parts as one element, which is §11.4.6's transparency groups again. What
was available now is honesty about it, and trap 11 says the whole value of a report is the
precision of its condition. Two conditions:

- **The paint has to composite.** Opaque painting under the Normal blend mode puts the stroke
  over the fill either way.
- **Both parts have to mark the page.** A `B` whose fill or stroke alpha is zero is one object
  painted once. Three of the six corpus documents that reach this line are exactly that —
  `issue11045.pdf` fills at alpha 0 and strokes opaque, `issue3458.pdf` the other way about —
  and they were found by instrumenting the report before believing it.

The report names 4 documents and costs the oracle one page, `alphatrans.pdf`, which the same
session had just made *agree*. That trade is stated rather than avoided: a page that reports
is a page the oracle stops judging, and the alternative was to leave a page drawn under a
model the standard does not describe with nothing said.

The clause's other two examples cost nothing here. A self-intersecting path is filled once
under one winding rule, and a mesh shading is one paint.

## Decision 6 — a `/BM` array takes the first mode this reader *knows*

§11.6.3, of the deprecated array form: a processor "shall use the first blend mode in the
array that it recognizes (or Normal if it recognizes none of them)". This tree took the first
*name* and mapped it, which maps an unrecognised leading name to Normal and never looks at the
rest. `[/FooBar /Multiply]` is multiply, and was normal.

Indistinguishable from a correct reader on every array whose first entry is a real mode, which
is every array anybody writes — the same shape as §9.3.3's word spacing in the thirteenth
session, and found the same way: by reading the clause rather than the code.

## Consequences

- Two corpus documents draw completely that did not, and one begins reporting: the incomplete
  count goes 232 → 231, and the `Image` row of the corpus gate 13 → 11, with **nothing left on
  it that is a feature** — four malformed streams, three refused bit depths, one `/Mask` that
  is not an image mask, two files the codecs refuse, and one mask the grid bound refuses.
- The oracle's gated set grows by one on balance (1512 → 1513): `smaskdim.pdf` and
  `chrome-text-selection-markedContent.pdf` enter, `alphatrans.pdf` leaves by reporting.
  Agreement rises 672 → 673 and the contradicted count falls 104 → 103.
- Seventeen ledger rows move off `unreviewed` — §11.3.7, §11.5 and §11.6 entire — taking the
  unreviewed count 668 → 652. Two of them are `silent`: §11.6.6, transparency group XObjects,
  where a `/Group` is read nowhere and a group is drawn as an ordinary form XObject, and
  §11.3.7.3, the result-shape formula that needs one. Both are the gap §11.4.6 already owns,
  recorded where a reader of those clauses would look for it.
- The only soft-mask geometry still refused is the 151-million-sample one, and the only
  `/Matte` still refused is one in a colour space no corpus document uses.

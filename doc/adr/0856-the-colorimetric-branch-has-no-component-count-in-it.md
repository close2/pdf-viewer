# ADR 0856 — §11.5.3's colorimetric branch has no component count in it, and four components make a mask group a pair

Status: accepted. Session 907.
Clauses: ISO 32000-2 §11.5.3, §11.3.4, §11.3.3, §11.4.7, §11.6.5.1, §11.6.6, §11.7.2, §8.6.5.1,
§8.6.5.5, §10.4.2.3.
Code: none of its own — this decision is the reading ADR 0857 builds. What it changes is
`doc/todo/23-transparency-departures.md` and §11.5.3's ledger row, which had recorded the shape
as a debt without saying which of the three constructions it needed.
Continues ADRs 0796, 0797, 0851. Beside ADR 0857.

## The question

ADR 0851 left §11.5.3 `partial` on one shape and named it rather than drawing it:

> A four-component profile mask group's `Y` is a function of four composited components,
> §11.4.7's construction for four components is a **pair** of rasters, and
> `pdf_render::SoftMask` carries one group and derives one value per pixel.

and measured its population, which two earlier decisions had recorded as empty: **3417 groups in
181 documents** of `CC-MAIN-2021-31`, none in `doc/pdf.js`. Round 907 was asked to read the two
clauses together, state exactly what they require, and either build the construction or sharpen
the refusal with the measurement.

## What the clauses require

**§11.5.3 branches on the kind of the space and states no component count.** The whole of the
branch is two bullets under one sentence:

> The colour C shall then be converted to luminosity in one of the following ways, depending on
> the group's colour space:
>
> - For CIE-based spaces, convert to the CIE 1931 XYZ space and use the Y component as the
>   luminosity. This produces a colorimetrically correct luminosity.

EXAMPLE 1 writes the `CalRGB` formula out and then generalises it in one line — "[a]n analogous
computation applies to other CIE-based colour spaces" — and §8.6.5.1 makes every `ICCBased` space
CIE-based whatever its channel count. §11.3.4 names the admissible blending spaces in one list:

> - ICCBased bi-directional 'GRAY', 'RGB ', and 'CMYK' colour spaces

so 'CMYK' sits beside the two this tree already draws, under the same requirement, in the same
sentence. **There is no reading on which four components take the device branch**, whose own
sentence — "For device colour spaces … with no compensation for gamma or other colour
calibration" — is addressed to spaces that have no calibration to compensate for. That is the
same conclusion ADR 0851 reached for a three-component table profile, one axis over.

**What `Y` is a function of is the *composited* colour, and that is four numbers.** §11.5.3
composites first and converts afterwards —

> The second method of deriving a soft mask from a transparency group shall begin by compositing
> the group with a fully opaque backdrop of a specified colour. The mask value at any given point
> shall then be defined to be the luminosity of the resulting colour.

— and §11.3.4 says what compositing in a four-component space *is*:

> The i th component of the result colour 𝐶𝑟 shall be obtained by applying the compositing
> formula to the i th components of the constituent colours

per component, over the space's own four, with §11.6.6 converting every mark into that space on
the way in and §11.6.5.1's `/BC` stated as "n numbers, where n is the number of components in the
colour space specified by the CS entry". So the quantity §11.5.3 takes the `Y` of is a
four-vector, and the order of the two operations is not free: **the luminosity of a composite is
not the composite of luminosities**, because a press profile is not affine in its inks. The
fixture in `a_luminance_over_four_axes_reads_the_pairs_own_components` states the gap on a
multiplicative absorption model — half a cyan-and-magenta mark over paper composites to
`0.95 × 0.70` where averaging the two ends gives `(0.36 + 1.0) ÷ 2`.

**A rasteriser has three channels, so four components are two rasters.** That is not this
clause's sentence but §11.4.7's construction, which ADR 0262 built for the page and ADR 0327
took one scope down to a group: the same content interpreted twice, once carrying the additive
complements of cyan, magenta and yellow and once the complement of black, resolved together
before the result is used. §11.3.4 is what forces the complement —

> When performing blending operations in subtractive colour spaces ( DeviceCMYK , ICCBased
> 'CMYK', Separation , and DeviceN ), the colour component values shall be complemented
> (subtracted from 1.0) before the blend function is applied

— and it is why both halves are stored that way rather than complemented around every blend.

## The three constructions, priced

**One: grow `SoftMask` to carry the pair.** A second command list beside `SoftMask::commands`
with its own half of `/BC`, a four-axis `Y`, a second buffer on each backend, and the pair read
together per pixel. Cost: one more interpretation of the mask group's content stream, one more
target-sized buffer while the mask is built, and one more command list on the wire. **Chosen**,
and ADR 0857 is what it cost.

**Two: evaluate the luminosity during compositing rather than after.** Rejected by the clause
rather than by the price. §11.3.3's formula composites colours, and a mask's own arithmetic is
downstream of it: reducing each mark to its `Y` as it is drawn would composite luminosities,
which is the operation the paragraph above shows is a different number. It would also lose
§11.3.5.2's per-component blend functions, which are defined on the components and not on a
scalar. There is no place in the model where the reduction can happen earlier.

**Three: take the `Y` of the resolved device colour.** The mask group is already drawn in *some*
raster; reading its sRGB and converting that to XYZ would need no second list at all. Rejected in
ADR 0851 and re-derived here, because it is the one that looks affordable: the resolved colour is
eight bits and clamped to sRGB's gamut, so a press colour outside that gamut — which is most of
what a four-ink space can state — comes back as whatever the clamp left, and its `Y` is wrong by
whatever the clamp removed. §11.5.3 admits no such approximation, and a mask is exactly where it
would be least visible and least recoverable: the error lands in an alpha, not in a colour, so
nothing downstream can show that it happened.

## One thing the reading added that the todo file's list did not have

`doc/todo/23` listed five pieces and was right about all five. What it did not say is that **the
second run is unconditional where the space has four components**, and the difference matters
because the group construction next door is *not*: `group_commands` skips its black half when
nothing inside the group composites, on the argument that a group's four components are converted
to the device at the end and an opaque `Normal` mark carries its colour through whatever space it
was carried in. That argument does not survive the move into a mask. A mask group's four
components are converted to **one number**, by a function that reads all four, and the black
component of a wholly opaque mark is as load-bearing as any other: a `0 0 0 1 k` fill paints
*nothing at all* into the chromatic raster, so a reader with one raster sees paper and masks
nothing away where the clause asks for `Y` of full black.

## Consequences

- §11.5.3's colorimetric branch is taken for one, three and four components, and the row's
  remaining reports are conditions the **document** fails plus one budget.
- The pair is now a carrier this tree has for a *mask*, which is what §11.3.5.2's residue in a
  `DeviceCMYK` mask group would need — that residue has no corpus member and is not built here,
  but it stopped being a construction nobody has.
- ADR 0851's rejection of the resolved-device-colour route stands, and is now recorded with the
  two alternatives it was chosen against rather than on its own.

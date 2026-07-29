# ADR 0027 — Soft masks in the graphics state, and where a mask is evaluated

Status: accepted, 2026-07-29.

## Context

A soft mask (ISO 32000-2 §11.5) is the third source of shape and opacity §11.6.4.1 names,
beside an object's own and the graphics state's constants, and the only one that varies from
point to point:

> Such an independent source, called a soft mask , defines values that may vary across
> different points on the page.

It was this tree's largest reported rendering gap: 28 corpus documents got
`Shading { name: "SMask in /GSn" }` and drew the object opaque. The seventeenth session is
why it could be taken — a mask's values come from a transparency group (§11.5.2, §11.5.3),
and until `Command::Group` existed nothing here could evaluate one.

The whole of what a mask needs is Table 142: a subtype selecting one of two derivations, a
group XObject, a backdrop colour for the second of them, and a transfer function. Reading
that table is a morning's work. **The design question is not what a mask says; it is where a
mask is evaluated**, and that is what this record is about.

## Decision

### A display list carries a mask's *commands*, not its pixels

§11.6.5.1 fixes the mask's coordinate system at the `gs`:

> The mask's coordinate system shall be defined by concatenating the transformation matrix
> specified by the Matrix entry in the transparency group's form dictionary ( see 8.10.2,
> "Form dictionaries") with the current transformation matrix at the moment the soft mask is
> established in the graphics state with the gs operator.

so the mask is fixed in *page* space when the `gs` runs, and everything after that is
resolution. A mask is a coverage per device pixel; evaluating it in `pdf-model` would mean
choosing a resolution there, and `pdf-model` does not know one — the same display list is
drawn at every zoom level and on both backends.

So `DisplayList` gains a table of `SoftMask`s beside its clips, each holding the commands its
group draws, the derivation, the backdrop and the sampled transfer function; and every
command gains `mask: Option<SoftMaskId>` beside its `clip: Option<ClipId>`. A backend
rasterises the group at the target it is drawing to, exactly as it rasterises the page.

**Per command rather than per run of commands**, because §11.6.4.3's NOTE 2 says what a mask
does to overlapping objects — "the effect of the mask multiplies with itself in the area of
overlap" — and warns that this is usually not what a producer wants. Applying one mask to a
run of objects would be the *other* picture, the one the NOTE recommends a producer get by
grouping. So the mask multiplies per object, and a producer who wanted it once wrote a group.

### What the pixels mean is decided once, for both backends

`pdf_render::SoftMask::value` takes one straight-alpha RGBA pixel of a rendered group and
returns one mask value: the alpha for §11.5.2, or the luminosity over the backdrop for
§11.5.3, then the transfer function. Both backends call it. That is the rule
`Image::area_averaged` and `Image::is_smoothed` already follow (ADR 0025): a decision the CPU
oracle and the GPU backend could make differently is a decision that does not belong in a
backend.

It is load-bearing here rather than tidy. §11.5.3's device branch is EXAMPLE 2's
`Y = 0.30 R + 0.59 G + 0.11 B`, and **both rasterisers offer a luminance mask of their own
that is not that formula**: `tiny_skia::MaskType::Luminance` is Rec. 709
(`0.2126/0.7152/0.0722`) and Vello's `push_luminance_mask_layer` is the SVG one
(`0.2125/0.7154/0.0721`). On grey artwork every formula agrees, which is why this could have
shipped unnoticed — 64 of the corpus's 134 mask dictionaries name a `/DeviceGray` group. On a
green mask they are a fifth of the mask's range apart.

### The GPU renders each mask to a texture and reads it back

Vello can express part of this natively and neither part is enough. A layer composited with
`Compose::DestIn` takes the alpha of what is drawn inside it, which is §11.5.2 exactly; a
luminance-mask layer is nearly §11.5.3, but for the coefficients above; and no blend mode is
§11.6.5.1's `/TR`, which is an arbitrary function and appears in 11 of the corpus's 134 mask
dictionaries.

So `render-gpu` renders each mask's group into a texture of its own, reads it back, converts
it with the shared function, and draws the result as the alpha of a `DestIn` layer around
each masked object. The cost is a GPU round trip per mask on a page, and the corpus says how
many that is: the heaviest first page in the 974 documents registers **27**, and the median
page with a mask at all registers one. What it buys is that the two backends compute *the same mask*, which
is the premise the whole cross-backend comparison rests on.

The alternative that would remove the round trip is a post-process shader of our own, which
means owning a wgpu pipeline beside Vello's. That is a larger change than this feature
justified, and it is written down here so that whoever needs the milliseconds knows what to
build.

### `/AIS` is deliberately not read

Table 57's alpha-is-shape flag decides whether the mask supplies shape or opacity. This
renderer keeps one alpha per pixel, and §11.3.7.1 makes alpha the product `α = f × q` of the
two — so a mask value multiplies the same number whichever it is called. The distinction is
visible only to an implementation that tracks shape and opacity separately, and the two
places that would need it, knockout and non-isolated groups, are already reported as
departures (ADR 0026). Reading `/AIS` into a field nothing consults would be a placeholder
pretending to be an implementation.

## What reading the family found

The demand item was §11.5 and §11.6.5.1. Reading §11.7 beside them — because `/BC` is stated
in a group's *blending colour space*, and §11.7.2 is the clause that says what such a space is
— produced four things the item alone would not have.

- **§11.7.3 is satisfied by a decision made for another reason.** "Spot colours shall not be
  available in a transparency group XObject that is used to define a soft mask; the alternate
  colour space shall always be substituted in that case." This tree converts a `Separation` or
  `DeviceN` colour through its tint transform at the moment it is read, everywhere, so the
  sentence is true here by construction — the clause offers two treatments and we take the
  same one always.
- **§11.7.4, overprinting, is a silence.** `/OP`, `/op` and `/OPM` are read nowhere, and 63 of
  the corpus's first-page `/ExtGState` dictionaries set one of the two booleans. Under
  §11.7.4.2 an object painted with overprinting enabled composites through a special blend
  mode that leaves the backdrop's value in every component the source does not paint. Six
  ledger rows now say so; sizing a *report* is the work they are owed, and trap 11 is why it
  was not written here — the key's presence is not the condition.
- **A mask group's `/CS` is a departure this tree can state precisely.** Compositing happens
  on three device components, so a mask whose group blends in `/DeviceCMYK` has neither our
  compositing nor our luminosity. It is reported, on 7 documents. `/DeviceGray` is *exact* —
  a grey converts to `R = G = B` and the three coefficients sum to 1 — which is worth
  proving rather than assuming, because it is 64 of the 134 and reporting them would have
  been noise.
- **§11.5.3's other branch is not taken.** "For CIE-based spaces, convert to the CIE 1931 XYZ
  space and use the Y component as the luminosity." Every colour here is device RGB by the
  time a mask composites, so the device branch is what runs even for a `CalRGB` or ICC group.
  That is the same choice §11.6.6 already records page-wide, and it is recorded rather than
  reported for consistency with it.

## Consequences

- 17 corpus documents draw completely that did not, and the `Shading` row of the corpus gate's
  breakdown — 28 documents, every one of them a soft mask — is gone.
- 20 more pages reach the oracle's gated set, 12 of them agreeing with the reference
  consensus.
- **A report that had been hidden behind another report appeared.** `knockout_smask.pdf` paints
  an opaque blue over an opaque red inside a knockout group, *under a mask*. §11.4.6's report
  fires when an element that composites overlaps one painted before it, and an opaque fill
  under a soft mask composites — so `command_composites` now says so, and the page reports
  what it always should have. It had been quiet because the mask report reached it first.
- Three pages joined the contradicted list and none is a masking defect: two are pages that
  became comparable and carry a one-pixel raster-size difference or small-glyph coverage, and
  one, `smask_luminosity_oob_transfer.pdf`, is contradicted by a single level of eight-bit
  mask quantisation on a page whose whole content is one flat composite. `oracle.rs` names
  each with the arithmetic.
- A mask's group is evaluated once per `gs`, not once per object: the CPU backend caches the
  raster beside its clip masks and under the same budget, and the GPU evaluates every mask in
  the list before it builds the scene. A page that applies the *same* `/ExtGState` a hundred
  times still registers a hundred masks, which is the one cost here nobody has bounded — the
  corpus's worst page registers 27, so it was measured rather than fixed.

## What this does not do

- **A mask is eight bits.** `tiny_skia::Mask` holds one byte per pixel and a texture holds no
  more, so a mask value is quantised. That is one level of difference on a page-wide flat
  composite, and `CONTRADICTED_MASK_QUANTISATION` in `oracle.rs` is the one page where it
  shows.
- **A mask raster is the size of the target**, wherever its group's `/BBox` is. Bounding it to
  the box would need the same coordinate-system change the CPU backend's group buffers want,
  and no corpus page pays enough for it to be measured yet.
- **Overprinting stays silent**, as above.

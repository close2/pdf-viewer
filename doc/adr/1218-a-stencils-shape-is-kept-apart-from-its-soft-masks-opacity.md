# ADR 1218 — A stencil's shape is kept apart from its soft mask's opacity

## Status

Accepted. Builds the residue ADR 1205 §3 named — "`SampleAlpha::Both` is the one true shape gap
… the blocker is `Picture::source` multiplying the stencil into its `/SMask` before a command
exists" — and takes its §5 instruction to open `pdf_model::image` rather than `pdf-render`.

## Context

ISO 32000-2 §11.3.7.2 keeps a source's shape and its opacity as two quantities, and §11.4.6 is
the clause that reads the first apart from the second:

> The existence of the knockout feature is the main reason for maintaining a separate shape
> value rather than only a single alpha that combines shape and opacity.

§11.6.4.2 says what gives a stencil its shape —

> For image masks (8.9.6.2, "Stencil masking"), the shape shall be 1.0 for painted areas and 0.0
> for masked areas.

— and §11.6.4.3 makes an `/SMask` the other quantity. A file may state both: Table 87 lists what
a stencil may *not* carry (a `/BitsPerComponent` other than 1, a `/ColorSpace`, a `/Mask`) and
`/SMask` is not among them, while §8.9.6.1 adds "a fourth type of masking effect, soft masking, is
available through the SMask entry" without excluding one.

`pdf_render::SampleAlpha::Both` is that image, and its one raster's alpha was the product. A
knockout group containing it was reported rather than drawn, because `stated_shape` could ask the
raster for neither half.

## Decision

**Route a stencil under its own `/SMask` to the device-scale producer, and give that producer a
`shape()`.**

1. `image::soft_mask_entry` answers `SoftMaskEntry::AtDeviceScale` for a parent whose
   `/ImageMask` is true, on a reason from the clause rather than from the two grids — the routing
   that already exists for a refinement too large to build. The pair therefore arrives as
   `Picture::Masked { base, opacity }`, `base` being the stencil's own samples.
2. `ImageAtDeviceScale::shape` is a new method with a `None` default, so a producer whose alpha is
   one quantity is unaffected. `MaskedAtDeviceScale::shape` hands back the base alone, restated as
   `SampleAlpha::Shape` because that is what *that* raster's alpha is; the product stays in
   `samples`. `ImageSource::shape` forwards, and answers `None` for a decoded source, whose one
   raster has already multiplied the two.
3. `transparency::shape_without_the_mask_and_the_constants` draws that shape at alpha 1.0 for the
   `Both` arm, so the element reaches the display list as a `Command::Shaped` like any other.

**The refusal is not removed, it is narrowed, and it is still stated by name.** A mask
`eligible_for_the_device_scale` declines — one behind §7.4's image codecs — and a mask carrying
Table 144's `/Matte`, whose pre-blending "shall precede the colour conversion" and therefore has
to be undone in one raster, are combined as they are read, and `unstatable_shape` reports them as
before.

## Consequences

- `raster_golden` holds 974 of 974: the routing changes what a stencil-plus-`/SMask` image is
  *combined on* — §10.7.4's device grid rather than the file's — and no corpus page has one.
  Measured rather than assumed: over the 974 tracked documents there are **1099 image `/SMask`s**
  and **none at all on a stencil** (`group_shape_census` prints 0 refusals over the same
  population). The fixture is the witness, which is trap 8's shape.
- `image_masks.rs::a_stencil_under_its_own_soft_mask_states_its_shape_to_a_knockout` is that
  fixture: a knockout group drawing one stencil twice, overlapping, whose elements arrive as
  `Shaped` commands with a `Both` object beside a `Shape` shape. Its control is the same fixture
  with `/Matte` on the mask, where the report is still what is owed.
- The confined wire cannot carry a deferred producer (`Uncodable::DeferredImage`), so a page with
  such an image now crosses as a raster rather than as a list. That is the existing fallback and
  it costs the zoom-side reuse of the list, on a page class no corpus document holds.
- A `/ImageMask` whose opacity arrived inside a JPEG 2000 codestream (`/SMaskInData`) is still
  `Both` with nothing to separate; it is reported, and §11.6.5.2's row says so.

## Alternatives rejected

- **A `shape` field on `pdf_render::Image`.** ADR 1022 §5 priced this and ADR 1205 corrected the
  price: a second raster carried by the vocabulary type is paid by every such image whether or not
  a knockout group asks. It would also have to be added at every construction site of `Image`,
  most of them in crates with nothing to do with masks.
- **Emit `Command::Shaped` from the image call site.** `Command::Shaped`'s own contract is that it
  "appears only as a direct element of a `Group` whose `knockout` is set", and an image is drawn
  wherever a content stream puts it.
- **Keep the pre-multiplication base beside `Picture::Complete`.** Two mechanisms for one fact —
  a field on the eager route and a method on the deferred one — where the routing makes one do.

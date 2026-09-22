# 1279 — Every stencil states its shape, overprinting is read, and the transfer report is deleted

Status: accepted. Session 1221.
Amends: ADR 1218, whose refusal for a stencil combined as it is read (a mask the device-scale route
declines, a `/Matte`, an `/SMaskInData` opacity) this closes; and ADR 1266 section 5, which named
that refusal as `Unsupported::TransferFunction`'s last reach.
Depends on: ADR 0570, ADR 1125, ADR 1255, ADR 1266.
Context: `crates/pdf-model/src/image.rs`, `crates/pdf-model/src/content/marked.rs`,
`crates/pdf-model/src/content/report.rs`, `crates/pdf-model/src/content/image.rs`,
`crates/pdf-model/src/content/transparency.rs`, `crates/pdf-colour/src/shading.rs`,
`crates/pdf-colour/src/mesh.rs`, `crates/pdf-render/src/transfer_channel.rs`,
`crates/pdf-model/tests/transfer_functions.rs`, `crates/pdf-model/tests/image_masks.rs`.
Clauses: ISO 32000-2 §11.7.5.2, §11.6.4.2, §11.6.4.3, §11.6.5.2 (Tables 143 and 144), §8.9.5.1
(Table 87), §7.4.9, §11.7.4.2, §11.7.4.3, §8.6.7.

`§N` is ISO 32000-2 and nothing else.

## 0. What the brief said was owed, and what the tree said

The round was briefed to build a shading pattern's ramp and a tiling cell into §11.7.5.2's channel.
Both were already built (ADR 1266): `a_ramps_pixel_is_the_composition_and_not_the_chord` and
`a_tilings_pixels_take_the_function_the_painting_mark_states` assert the clause's value, and the one
reach of `Unsupported::TransferFunction` left in the tree was `Interpreter::note_unstatable_shape`,
a `SampleAlpha::Both` image whose source could not hand back its shape. That is what was built.

## 1. A stencil's pair is a pair on every route

§11.6.4.2: "For image masks (8.9.6.2, "Stencil masking"), the shape shall be 1.0 for painted areas
and 0.0 for masked areas." ADR 1218 kept that shape apart from an `/SMask`'s opacity by carrying the
pair to the device, and left three routes that multiplied them as they were read:

- **A mask the device-scale route declines** (a codec past the plane bound, a one-component space
  other than `DeviceGray`). The eager route already decodes it; `apply_soft_mask` now hands a
  stencil's decoded mask back as a plane (`SoftMaskAtDeviceScale::of_decoded`) instead of
  multiplying it in, and `decode_parts` returns `Picture::Masked`. One mechanism, not two: the pair
  is the `Picture::Masked` ADR 1218 built, only the plane is decoded earlier.
- **A `/Matte`.** ADR 1218's fixture held that the pre-blending forces one raster. For a stencil
  that does not hold: Table 144 counts the matte's numbers in "the colour space specified by the
  ColorSpace entry (or the base entry of the colour space, if the colour space is Indexed ) in the
  parent image's image dictionary", and Table 87 does not permit an image mask a `/ColorSpace` —
  so a stencil's samples carry no pre-blending, and `unpack`'s stencil arm never read one.
- **An `/SMaskInData` opacity.** `jpx_stencil` thresholded the colour channel and never read the
  opacity channel at all, then labelled the raster `Both` — so the opacity Table 87's code 1 says "A
  PDF processor shall create a soft-mask image from" was dropped in silence. It is read into a plane
  now (`SamplesOnGrid::stencil_opacity`), and code 2's premultiplied sample is compared with the
  midpoint scaled by its own opacity, which is the undone comparison in integers.

So `pdf_model` hands no command a raster that multiplied the two, the channel reads the stencil's
painted areas as its shape, and §11.4.6's knockout states the element too — its image refusal arm
in `unstatable_shape` could no longer be reached and is gone.

## 2. The report is deleted, not left unreachable

With every shape stated, `note_unstatable_shape` could fire on no page. `Unsupported::TransferFunction`
is removed from the vocabulary, with its sentence in `viewer_core::report` and its row in
`tests/corpus.rs`'s classification. A refusal nothing reaches reads as a live one. The fixtures in
`tests/transfer_functions.rs` that asserted *this* report was absent now assert that the page raises
nothing at all, which excludes every report rather than one sentence (trap 27).

## 3. The overprinting paragraph

§11.7.5.2 closes with a paragraph no row had read: "An object is opaque for a given component only if
overprinting yields the source colour (not the backdrop colour) for that component." In this tree
overprinting yields a backdrop component only under §11.7.4.3's special mode inside a `DeviceCMYK`
compositing space (§8.6.7 NOTE 1 ignores the parameter everywhere else on this device). §10.5 maps
the device's three components, and §11.7.4.2 says the group's four "are not the device's actual
process colourants": each device component is converted from all four, and every ink moves all three
in the ink cube (`colour::CMYK_CORNERS`) as in any profile's table. Asked per component, the answer
is the same for all three, opaque for none, so `Interpreter::draw_mark` records such a mark with no
function and the page's default maps it. It still occludes. A per-component map on the channel was
rejected: it would record three identical answers.

## 4. The plumbing the channel made dead

ADR 1266 passed `None` for `pdf_colour::shading::Colouring`'s transfer at both call sites and left
the field, `transferred`, `transferred_corners`, the cache filter and the withdrawn device program
behind it, with comments saying the function was applied inside the sampling. All are removed.
`Colouring::new` takes two arguments, and a type 1 shading always offers its device program.

## Consequences

- §11.7.5.2 `partial` → `implemented`; §11.7.5 `partial` → `implemented`; §11.7 stays `partial`
  for §11.7.4. `render-gpu` still refuses a list carrying the channel by name; it is not a shipping
  backend, and the confined wire still carries such a page as pixels.
- `raster_golden` holds 974 of 974. No corpus page has a stencil under a soft mask
  (ADR 1218's census) or a `/TR` under the special overprinting mode.
- Three fixtures in `tests/transfer_functions.rs`, each derived from the clause and each failing
  with its rule planted away — the stencil page gives 255 255 255 at a painted cell of opacity 0
  where the product gives 0 0 0 — and ADR 1218's `/Matte` control in `tests/image_masks.rs` turned
  from a report into a stated shape.
- `doc/todo/13` is kept rather than deleted: `CLAUDE.md` cites it for §10.5's reading.
- The rows beside this one that name the same residue — §11.3.7, §11.3.7.2, §11.3.7.3, §11.4.4,
  §11.4.6, §11.7.4.4 — were not this round's to edit, and are handed over for re-reading.

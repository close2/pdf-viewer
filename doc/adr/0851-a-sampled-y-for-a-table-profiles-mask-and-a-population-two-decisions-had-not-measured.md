# ADR 0851 — A sampled `Y` for a table profile's mask group, and the four-component mask population two decisions had not measured

Status: accepted. Session 904.
Clauses: ISO 32000-2 §11.5.3, §11.3.4, §11.6.5.1 (Table 142), §11.6.6, §11.7.2, §11.4.7,
§8.6.5.1, §8.6.5.5, §8.6.5.9.
Code: `crates/pdf-render/src/soft_mask.rs` (`Luminance` as two shapes, `trilinear`),
`crates/pdf-model/src/colour.rs` (`RgbRoute::luminance` over a grid,
`RgbRoute::luminance_is_separable` removed), `crates/pdf-model/src/soft_mask.rs` (`entry` takes
the interpretation's `Presses`, `luminosity_departure` on the clause's own branch),
`crates/pdf-model/src/content/ext_gstate.rs`,
`crates/viewer-confined/src/protocol/display_list.rs` (a tag for the mask's `Y`),
`crates/render-quorra/src/scene.rs` (the refusal's words),
`crates/pdf-model/examples/luminosity_mask_census.rs` (the four-component shapes named).
Tests: `crates/pdf-render/src/soft_mask.rs::a_luminance_over_a_grid_interpolates_the_clauses_y`,
`crates/pdf-model/tests/transparency_groups.rs::a_table_profile_mask_group_takes_the_luminance_of_its_composited_components`,
`::a_mask_group_in_a_cie_space_with_no_route_is_named_rather_than_drawn_in_silence`,
`crates/viewer-confined/src/protocol/display_list.rs::a_luminosity_masks_sampled_y_round_trips_to_an_equal_list`.
Continues ADRs 0790, 0792, 0796, 0797. Beside ADR 0850.

## The question

ADR 0797 left §11.5.3 `partial` on two shapes of mask group `/CS`: a three-component **table**
profile, whose `Y` "is not separable" and which "keeps the grey of the sRGB its colours become — a
choice", and a **four**-component profile, "which is §11.4.7's pair inside a mask, does not fall
out of a one-raster construction, and has no corpus member". Round 904 was told to measure the
population before deciding either. The measurement inverted them.

## The measurement

`examples/luminosity_mask_census`, extended to name the four-component shapes the way ADR 0797
named the three-component ones, over `doc/pdf.js/test/pdfs` (964 documents open) and over all 145
archives of `CC-MAIN-2021-31` one at a time under `tools/bounded.sh --tree 12 --data 12` (65 720
of 65 944 open, 3343 with a `/Luminosity` mask, 77 s warm):

| mask group `/CS` | `doc/pdf.js` | the crawl |
|---|---|---|
| `/DeviceGray` | 37 | 35 141 |
| three-component `ICCBased`, matrix profile | 3 | 28 972 |
| `/DeviceCMYK` | 39 | 21 834 |
| `/DeviceRGB` | 2 | 6 597 |
| **four-component `ICCBased`, bi-directional** | **0** | **3 417**, in 181 documents |
| one-component `ICCBased` | 8 | 228 |
| `CalRGB` | 1 | 0 |
| **three-component `ICCBased`, table profile** | **0** | **0** |

So the shape ADR 0797 called a choice has no member anywhere, and the shape it recorded as having
"no corpus member" has 3417 in 181 documents. **That claim was true of `doc/pdf.js` and was written
without saying so** — the `undenominated` sweep's defect exactly, and this file records it as such
rather than as an oversight, because two decisions and a todo file carried it.

## The reading, and what was built

**§11.5.3 branches on the kind of the space, not on the shape of its arithmetic.**

> The colour C shall then be converted to luminosity in one of the following ways, depending on
> the group's colour space:
>
> - For CIE-based spaces, convert to the CIE 1931 XYZ space and use the Y component as the
>   luminosity. This produces a colorimetrically correct luminosity.

and EXAMPLE 1, having written the `CalRGB` formula out, adds "[a]n analogous computation applies to
other CIE-based colour spaces". An `ICCBased` space is CIE-based (§8.6.5.1) whether its profile
carries a matrix and three curves or a lookup table. There is no permission anywhere in the clause
to take the *device* branch for a space that is not a device space, and the device branch's own
sentence — "with no compensation for gamma or other colour calibration" — is written for spaces
that have none. So the previous behaviour was not a choice between two readings; it was the wrong
branch, and it was **silent**: `luminosity_departure` fired on `Lab` alone.

**A table profile's `Y` is sampled, and that is the same fidelity the same profile's conversion
out already has.** `RgbRoute::luminance` samples `profile.to_xyz_with(components, false)[1]` at
`RGB_TABLE_SIDE` (33) points an axis and hands `pdf_render::Luminance` a grid the backend
interpolates trilinearly — the identical grid, from the identical samples, that `profile_stages`
already builds for `ColourCube`'s conversion out of that profile. What it departs from evaluating
the profile is a property of the profile's own smoothness between its grid points, which is the
sentence ADR 0797 wrote for the conversion out and is no weaker here. **Without §8.6.5.9's black
point compensation**, as the separable shapes are: the clause asks for the colour's XYZ and the
compensation is a step toward a destination.

`Luminance` is therefore two shapes behind one type — three 256-sample curves summed, or a grid
interpolated — with the invariants held by its constructors, and `Luminance::of` is the one
function both backends that draw a mask call. `viewer-confined`'s protocol tags which shape it
wrote, so a list stating a grid cannot come back as curves.

**The route is asked of the interpretation's cache, and that is not a micro-optimisation.**
`soft_mask::entry` built an `RgbRoute` per soft-mask dictionary. For a matrix profile that is 256
samples; for a table profile it is 36 000 profile evaluations for the cube and as many again for
this `Y`, and `6081357.pdf` states 912 masks on one page. `entry` now takes the `Presses` the
interpreter already owns, so a profile is sampled once per interpretation per space
(`Presses::rgb_route`, ADR 0417's budget).

**A four-component profile mask group is named instead of drawn.** Its `Y` is a function of four
composited components; §11.4.7's construction for four components is a **pair** of rasters, because
§11.3.4 composites per component and a rasteriser has three channels; and `pdf_render::SoftMask`
carries one command list and derives one value per pixel from one raster. There is no exact
one-raster construction: the luminosity of a composite is not the composite of luminosities unless
`Y` is affine in the components, and a press profile is not. Taking the Y of the *resolved device
colour* instead was considered and rejected — the resolved colour is 8-bit and clamped to sRGB's
gamut, so a saturated press colour's `Y` would be wrong by whatever the clamp removed, which is an
approximation the clause does not admit. So it is reported, by name, with the clause's own words,
and `doc/todo/23` carries the construction it needs so that the next round does not re-derive it.

**The report's condition is the clause's branch and not a list of names**, which is trap 11's rule:
`luminosity_departure` fires on a CIE-based `/CS` for which the colorimetric route was *not* taken,
whatever the reason. That is what turned two further silences into reports at no extra cost — a
profile with no "from CIE" half, which §11.3.4 requires of a blending space, and a one-component
curve with no inverse — and it is why a space that stops having a route stops being silent by
construction rather than by somebody remembering.

## The backends

`render-cpu` draws both shapes: `SoftMask::values` is unchanged in shape and `Luminance::of` reads
whichever it holds. `render-gpu` draws them too — it reads its mask group back and calls the same
function. `render-quorra` refuses every mask carrying a `Luminance`, as it has since ADR 0797,
because `quorra_scene::MaskKind::Luminosity` weighs the channels in its own shader; the refusal's
words now name the space's own `Y` rather than three curves, and `doc/QUORRA_FEEDBACK.md` §43 asks
for the grid beside the curves in one vocabulary.

No page of `doc/pdf.js` changes: the corpus states no table-profile mask group and no
four-component one, which the census is what says. `REFUSED_BEFORE_THE_SCENE` is unmoved.

## Consequences

- §11.5.3 stays `partial`, on the four-component mask alone, and its row carries the measured
  population instead of "no corpus member".
- `pdf_render::Luminance` is a type with two shapes and a checked constructor for each; the
  confined protocol tags them.
- `RgbRoute::luminance_is_separable` is gone: every three-component route now states a `Y`, so
  there is nothing left to ask the question of.
- A `/Luminosity` mask group in a CIE-based space this reader cannot take the clause's branch for
  is loud. Three conditions that drew a wrong picture in silence now say so.

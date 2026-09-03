# ADR 0797 — Three components composited, and a cube out: `CalRGB` and `ICCBased` 'RGB ' blending spaces, and §11.5.3's `Y` of a three-component mask group parts from every reference

Status: accepted. Session 879.
Clauses: ISO 32000-2 §11.3.4, §11.4.7, §11.6.6, §11.7.2, §11.5.3, §11.6.5.1 (Table 142),
§8.6.5.3, §8.6.5.5, §10.3.1, §10.3.2, §10.4.2.1.
Code: `crates/pdf-render/src/blending.rs` (`ColourCube`, `resolve_cube`, `sample_curve`),
`crates/pdf-render/src/display_list.rs` (`GroupBlending::ThreeComponents`,
`DisplayList::set_colour_cube`), `crates/pdf-render/src/soft_mask.rs` (`Luminance`,
`SoftMask::luminance`), `crates/pdf-model/src/colour.rs` (`Compositing::Additive`, `RgbRoute`,
`RgbIdentity`, `Presses::rgb_route`, `ColourSpace::rgb_identity`, `rgb_components_at`,
`xyz_d50_to_linear_srgb`), `crates/pdf-model/src/icc.rs` (`Profile::matrix_stages`,
`MatrixStages`, `Curve::invert`, `Profile::to_device` for a matrix profile,
`Profile::is_bidirectional`), `crates/pdf-model/src/soft_mask.rs`,
`crates/pdf-model/src/content/transparency.rs` (`space_departure`, `space_identity`,
`device_components`, `group_blending`, `own_space_compositing`, `page_own_space`,
`Interpreter::group_own_space`), `crates/pdf-model/src/content.rs`,
`crates/pdf-model/src/image.rs`, `crates/render-cpu/src/lib.rs`, `crates/render-quorra/src/lib.rs`,
`crates/render-quorra/src/scene.rs`, `crates/render-gpu/src/lib.rs`, `crates/render-gpu/src/scene.rs`,
`crates/viewer-confined/src/protocol/display_list.rs`, `crates/test-scenes/src/lib.rs`,
`crates/pdf-model/examples/luminosity_mask_census.rs`.
Tests: `crates/pdf-model/tests/transparency_groups.rs::a_cal_rgb_page_group_composites_its_components_and_leaves_by_the_matrix`,
`::an_isolated_cal_rgb_group_composites_in_its_components_and_leaves_by_the_cube`,
`::a_device_rgb_group_inside_a_cal_rgb_page_is_that_cal_rgb`,
`::a_matrix_profile_page_group_composites_in_its_components`,
`::a_lab_page_group_is_reported_as_the_space_the_clause_forbids`,
`::a_cal_rgb_mask_group_takes_the_luminance_of_its_composited_components`,
`crates/pdf-render/src/blending.rs::a_linear_map_is_exact_from_its_corners`,
`::the_stages_run_in_order_and_cancel_when_they_are_inverses`,
`::a_three_component_raster_resolves_through_the_cube_under_its_own_alpha`,
`crates/pdf-render/src/soft_mask.rs::a_luminance_of_three_curves_sums_the_clauses_own_formula`,
`crates/render-cpu/tests/group_constructions.rs::a_group_in_a_three_component_blending_space_composites_its_components`
over `test_scenes::group_in_a_three_component_blending_space`, the two refusal tests
`headless_gpu.rs::the_gpu_refuses_a_group_in_a_three_component_blending_space` and
`headless_quorra.rs::quorra_refuses_a_group_in_a_three_component_blending_space`, and
`crates/viewer-confined/src/protocol/display_list.rs::a_page_in_three_components_round_trips_to_an_equal_list`.
Continues ADRs 0790, 0792, 0796.

## The question

ADR 0796 left §11.3.4's and §11.5.3's rows `partial` on one shape in both places: a `CalRGB` or
`ICCBased` 'RGB ' blending space — a page's, an isolated group's, or a `/Luminosity` mask
group's — composited in the device's three channels, "a choice now named in §11.3.4's and
§11.5.3's rows rather than a silence". Round 877 sketched the mechanism as a sampled 3 → 3 grid
on the display list and a three-curve `Y` for masks. This decision reads the clauses for the
three-component spaces, builds the mechanism — not quite the sketch, for a reason the device's
own transfer function forces — and records where the result parts from every reference this
project holds.

## The reading

**Which spaces.** §11.3.4 lists `CalRGB` and "ICCBased bi-directional 'GRAY', 'RGB ', and
'CMYK'" among the spaces that "shall be supported as blending colour spaces", and rules the
third three-component family out in so many words:

> The Lab space and ICCBased spaces that represent lightness and chromaticity separately (such
> as L*a*b*, L*u*v*, and HSV ) shall not be used as blending colour spaces because the
> compositing computations in such spaces do not give meaningful results when applied
> separately to each component.

So `Lab` stays reported, and the report now names the clause's list.

**What is composited.** The same sentence ADRs 0262, 0790 and 0792 built on: the compositing
formula is applied per component, so a space of three components composites its three, and a
raster's three channels are those components with nothing converted around a blend function —
exactly the construction the four-component pair and the one-component curve already use.

**The conversion out** is §8.6.5.3's, stated in full for `CalRGB` — "[t]he A, B , and C
components shall first be decoded individually by the gamma functions. The results shall be
treated as a three-element vector and multiplied by Matrix (a 3-by-3 matrix) to obtain the L,
M , and N components" — and a profile's `A2B` for `ICCBased`, applied where §11.4.7 puts it,
"before being composited with the context-dependent backdrop".

**The conversion in** has three parts, and §11.7.2 states the one that decides most marks:

> When converting colours, if the colour space of any graphics object is a device colour
> space, and the current group or an ancestor of the current group is defined with a CIE-based
> colour space with the same number of colourants, then, for compositing purposes only, the
> colour space of the graphics object shall be the CIE-based space of the nearest such
> ancestor.

So a `DeviceRGB` mark inside a `CalRGB` or RGB-profile group is *reinterpreted* as that space's
components — `1 0 0 rg` is the components (1, 0, 0), unconverted — which is also what §11.7.2's
NOTE 2 is about: "only CIE-based spaces can be used to predictably specify the colours of objects
within the group". A colour in the group's own space keeps its components (§11.6.6's "not
equivalent"). A CIE-based colour of another space goes in from its XYZ, §10.3.1's conversion
between CIE-based spaces: through a profile's `B2A` where it has one, through a matrix profile's
inverted stages, or through §8.6.5.3 run backwards for `CalRGB` — adapted onto the space's white,
the `Matrix` inverted, each component clamped as the clause clamps a component "falling outside
that range" and raised to the reciprocal of its gamma. And a device colour of *another* count —
a grey, a `DeviceCMYK` — is the case the clause hands over: "there is no generally defined method
for converting them", so it "shall be converted or mapped to a CIE-based colour space in an
implementationdependent fashion". This tree's way is ADR 0796's for a press: the colour's sRGB,
which §10.3.2 has this processor establish for its device spaces, taken to XYZ and in from there.
An opaque device grey painted into the group comes back out as itself, which the tests hold.

**The same §11.7.2 sentence, one paragraph up, decides a nested group.** "If the colour space of
the transparency group is a device colour space, and some ancestor of the group has a CIE-based
colour space with the same number of colourants, then the colour space of this group shall be
the CIE-based space of the nearest such ancestor." An isolated `/DeviceRGB` group inside an
sRGB-profile page — the commonest group there is — is that sRGB, not a change of space; and the
rule is general, so a `/DeviceGray` group inside a `CalGray` page and a `/DeviceCMYK` group
inside a profile's press inherit too, where until this round each compared unequal by name and
sent its page back to the device with something compositing in it. `group_blending` reads the
count. And two array-formed spaces of the *same* name are now told apart — a `CalRGB` of gamma
2.2 inside one of gamma 1 is a change of space with a conversion per pixel at its `Do`, which the
page reports rather than composites in the wrong one — by an identity `Departure` carries
(`space_identity`), which two different presses had lacked as well.

**Which profiles are bi-directional.** §11.3.4 says what the word means: "the ICC profile shall
be capable of both device to PCS and PCS to device transformations". A matrix profile — sRGB's
own, and every display profile — states no `B2A` table because it needs none: its matrix inverts
and its tone curves are monotone, and ISO 15076-1 defines the PCS-to-device direction of a
three-component matrix profile as exactly that inverse. `Profile::is_bidirectional` answers so,
and `Profile::to_device` runs the two stages backwards, a `Curve` inverted by bisection where it
is not a power law. A table profile with no `B2A` this crate reads has no way in and is not a
route: the page reports it with the clause's list.

## What was built, and why it is not the sketch

The conversion out rides on the display list as `pdf_render::ColourCube`: **one curve per
component, then a grid of `side³` device colours in linear light, then the device's transfer
function** — `resolve_cube` applies it where `resolve` and `resolve_grey` already resolve, before
the medium, and `GroupBlending::ThreeComponents` carries it one scope down.

Round 877's sketch was one sampled 3 → 3 grid, and it was priced before it was built. The
device's transfer function is steep at black — sRGB's slope is 12.92 there — so a grid of any
practical side sampled over the whole conversion in the *encoded* domain interpolates across that
toe: a linear `CalRGB` at 33 samples an axis is **fifteen levels of 255 out** just above black,
and a linear space is the one §11.7.2's NOTE 3 recommends a producer choose. Sampling in linear
light and encoding afterwards is what the two curves are for. And once the curves are outside
the grid, the grid has nothing nonlinear left to sample for the two spaces that matter: §8.6.5.3
is gamma decoding, a matrix, an adaptation and the device's matrix — everything between the
curves is linear — and a matrix profile is the same shape, so **two samples an axis reproduce
the map exactly** (a linear map is its own trilinear interpolant, which
`a_linear_map_is_exact_from_its_corners` holds) and the construction is the conversion rather
than a sampling of it. A table profile has no linear stage to separate and takes a grid of 33 an
axis with identity curves, sampled through the profile to linear light; what that departs from
evaluating the profile is a property of the profile's own smoothness between its grid points.
ADR 0272 measured a median 5.99 and at most 14.52 of 255 for the press's whole-conversion grid
and wrote that "per-axis input curves beside the grid" is what closes it; this is that shape,
one component down, and it is why the four-component press could take it too.

The mask side is §11.5.3's `Y`, which for these two spaces is a **sum of one function of each
component** — the clause's EXAMPLE 1 writes it out for `CalRGB` as the `Y` entries of the
`Matrix` weighting each gamma-decoded component, "using components of the Gamma and Matrix
entries of the colour space dictionary", and a matrix profile's is its tone curves weighted by
the middle row of its matrix. `RgbRoute::luminance` samples the three curves at 256 points and
`pdf_render::Luminance` sums them per pixel *before* Table 142's `/TR`; `/BC` is three numbers in
the space, as Table 142 says. The curves are the space's own, unadapted — the clause asks for the
colour's XYZ, and EXAMPLE 1 reads the `Matrix` directly — and without §8.6.5.9's compensation,
which is a step toward a destination and not part of a colour's XYZ. A table profile's `Y` is not
separable, and a mask group in one keeps the grey of the sRGB its colours become: a choice, priced
in `doc/todo/23`, for a population `examples/luminosity_mask_census` now prints by shape and,
on `doc/pdf.js`, finds no member of.

`render-cpu` draws all of it. `render-quorra` applies the page cube on its readback, as it does
the curve, and refuses a group carrying one and a mask carrying three curves — its luminosity
mask weighs the channels in its own shader, a different formula — so those frames go to the CPU
backend by name. `render-gpu` refuses the page cube and the group cube. `viewer-confined` writes
a third tag for the page's space and the group's and a presence byte for the mask's curves.
Images keep the `DeviceRGB` fast arm under `Compositing::Additive` and no other, because §11.7.2
makes a `DeviceRGB` sample its own components there; shadings take the producer's route, as
under every non-device compositing. `Presses::rgb_route` keeps a route per space per
interpretation, capped at the press's eight, because a table profile's route is 36 000 profile
evaluations and a group names its space at every `Do`.

## The pages, looked at

`examples/raster_digest` over `doc/pdf.js`'s 974 first pages, before and after: **nine move**.
Rendered at two pixels per unit beside `pdftoppm`, `mutool` and `gs` at the same scale, mean
absolute difference over the page in levels of 255 (`tmp/witness-879.sh`):

| page | before → after | vs `poppler` | vs `mupdf` | vs `gs` |
|---|---|---|---|---|
| `transparency_group.pdf` | 1.06 | 0.13 → 1.10 | 1.10 → **0.18** | 1.40 → **0.45** |
| `issue16742.pdf` | 0.11 | 20.35 → 20.46 | 0.02 → 0.12 | 20.35 → 20.46 |
| `issue21346.pdf` | 18.78 | **0.00 → 18.78** | 0.71 → 18.08 | 0.23 → 19.01 |
| six others | ≤ 0.004 | unchanged | unchanged | unchanged |

Six of the nine are sRGB-profile page groups whose content is `DeviceRGB`: the cube is the
profile's own identity to within a rounding, and they move by a level on a few pixels.

**`transparency_group.pdf`** is an ICC RGB page group with a `Difference` ellipse, and it moved
*toward* the two references that composite in the group's space and away from the one that does
not — which is the direction principle 5 admits as evidence: `mupdf` and `ghostscript` read the
clause the way this decision does, `poppler` ignores the space. **`issue16742.pdf`** is the one
`CalRGB` page group, already within 0.02 of `mupdf`; what moves is 450 edge pixels, where a
partly covered pixel now composites its coverage in the space's components and leaves by the
gamma, which is the different picture the clause states. Looked at (trap 1): the same green
shape.

**`issue21346.pdf` is the finding of this round, and it is written here at full length because
it is the one place this tree now parts from every reference it has, Acrobat included.** The
document is Typst's: a blue rectangle drawn through a `/Luminosity` mask whose group `/CS` is an
sRGB matrix profile and whose content is white at `/ca 0.25` over the default black backdrop.
The group composites in the profile's components — §11.6.5.1 requires the `/CS` for exactly that
— to (0.25, 0.25, 0.25), *encoded* sRGB. The clause then says which luminosity that is:

> For CIE-based spaces, convert to the CIE 1931 XYZ space and use the Y component as the
> luminosity. This produces a colorimetrically correct luminosity.

and, of the other branch, "[f]or device colour spaces, convert the colour to DeviceGray by
implementation-defined means and use the resulting gray value as the luminosity, with no
compensation for gamma or other colour calibration". An `ICCBased` space is CIE-based (§8.6.5.1),
its `Y` is the profile's, and the profile's tone curve takes 0.25 to 0.0508. So the mask is 0.05
and the rectangle is a twentieth blue: the page above is nearly white. Every reference draws it
a quarter blue, and pdf.js issue #21346's reporter tried "eight different renderers (+ Acrobat),
and they all render it as a light blue rectangle" — the device branch's 0.25, the encoded value
weighed with no gamma, taken for a profile that is not a device space.

The reading was checked three ways before it was kept. The clause's EXAMPLE 1 carries the gamma
in its formula, so a `Y` of the encoded components is not a possible reading of it; §11.7.2's
NOTE 3 and NOTE 4 say that compositing in a nonlinear space such as sRGB gives results that
"might not match the user's expectations" and are "still well-defined", so the standard
anticipated exactly this producer's choice and did not except it; and the only reading under
which the references are right is that an sRGB profile *is* the device — which is a statement
about a particular device, and §11.5.3 branches on the space's kind. Nine renderers agreeing is
strong evidence, and it is evidence of a convention: the crawl's majority mask population —
28 972 groups declaring a three-component `ICCBased` space, most of them this profile — has been
written against Acrobat, and a producer setting `/ca 0.25` to mean a quarter got a quarter
there. Principle 5 is the project owner's and absolute: agreement is evidence that the clause was
read right, disagreement is a question for the clause, never a target. The clause has been read
and it says `Y`. **So this tree draws the clause, and this paragraph is the record that it does so
knowing what it costs**: on every sRGB-profile luminosity mask with mid-tones, this viewer is
now darker in the mask than Acrobat and every other reader, by the profile's own gamma. It is the
first decision in this tree to part from all of them on a population that size, and it is the
owner's to keep or to rank — a policy the host supplies, in the shape principle 3's restriction
levels already have, would be the honest way to offer the convention beside the clause. Nothing
here curve-fits toward it.

`bug1721218_reduced.pdf`, ADR 0796's witness, does not move: its masks are one-component and
its groups four.

## What it cannot do, each reported or recorded

- A three-component table profile as a *mask* group's `/CS`: composited on the device and the
  grey of the sRGB taken — `luminosity_mask_census` prints the population by shape, and there is
  none in `doc/pdf.js`. Recorded in §11.5.3's row.
- A four-component profile as a mask group's `/CS`: §11.4.7's pair inside a mask, which does
  not fall out of this construction — a mask's group is one raster and the pair is two — and
  has no corpus member. Named in §11.5.3's row, unchanged.
- A `Lab` blending space: reported, with the clause's list.
- A three-component group inside a press or a grey page, or a press or grey group inside a
  three-component page, or two three-component spaces nested: a conversion per pixel between
  two spaces at the `Do`, recorded and the page drawn on the device with the report — the row
  `doc/todo/23` keeps.
- A table profile's route is a grid of 33 an axis; what it departs from the profile between grid
  points is not measured here, because no such page group is in either corpus.
- A three-component *group*, and a mask carrying three curves, on `render-quorra` and
  `render-gpu`: refused by name, to the CPU backend.

## Consequences

- §11.3.4 stays `partial` on ADR 0790's route-into-grey choice alone; its three-component debt
  is paid. §11.5.3 stays `partial` on the table-profile mask and the four-component mask.
  §11.4.7, §11.6.6, §11.7.2, §11.6.5.1, §8.6.5.3 and §8.6.5.5 record the reading.
- `pdf_render::GroupBlending` has a third shape and `DisplayList` a third page-level statement;
  every reader of either has an arm for it.
- `Departure` carries an identity, so two spaces of one name are two spaces.
- The oracle's and quorra's gates will name `issue21346.pdf`; the history file says how each was
  answered.

# 1229 — The separation simulation is built, and the per-component reading has a route

Status: accepted. Session 1196.
Context: ISO 32000-2 §10.8 (§10.8.1, §10.8.2, §10.8.3), §8.6.6.4, §8.6.6.5 (Tables 70, 71, 72),
§8.6.4.4, §11.3.5.2 Table 136, §11.6.6, §14.11.4 Table 400, §14.11.5; ADRs 0009, 0263, 1103,
1193 (the departure this one answers), 1228 (the control).
Code: `crates/pdf-colour/src/colour.rs`, `crates/pdf-colour/src/shading.rs`,
`crates/pdf-model/src/content/colour.rs`, `crates/pdf-model/src/image.rs`.
Tests: `crates/pdf-model/tests/colour_paths.rs::a_spot_colourants_own_separation_is_what_the_simulation_renders`,
`::two_separations_multiply_in_flat_xyz`,
`::a_separation_space_is_the_same_colour_under_the_simulation`,
`::a_process_only_nchannel_is_unmoved_by_the_simulation`,
`::a_none_component_adds_no_separation_to_the_simulation`,
`::a_spot_component_without_its_separation_keeps_the_tint_transform`,
`::a_process_component_and_a_spot_colourant_multiply`,
`crates/pdf-colour/src/colour.rs::the_white_matte_is_the_identity_of_the_multiply`.

## 1. What ADR 1193 left, and what has changed

ADR 1193 declined §8.6.6.5's per-component evaluation for an `NChannel` space with a spot
colourant, on one argument: evaluating the components individually produces *n* colours where a
fill needs one, and the only combination the standard writes down is §10.8.3's, "a permission
conditioned on a separation-simulation request this viewer has no control for". It named the
remedy in the same sentence: **"the day a separation-simulation control exists, this decision is
to be re-read against it."**

ADR 1228 built that control. This is the re-reading, and the algorithm.

## 2. §10.8.3's four steps, and the two things they do not state

> The results of the simulation should match those produced by the following steps:
>
> - a) Process the PDF as if separations were to be created for a simulated device that supports
>   subtractive process colourants and possibly spot colours. …
> - b) Convert each separation into "flat XYZ" (no gamma) and using a background matte of all
>   white.
> - c) Blend the resulting separations into a single result using a multiply blend (see "Table
>   133 -Variables used in the basic compositing formula").
> - d) Convert the result to the actual device colour space and output it.

Two things the steps leave to a reader, both decided here and both written beside the code:

**Which separations a `DeviceN` colour has.** §8.6.6.5's sentence is the answer and it is more
discriminating than it looks: a component "not present on the output device shall use the
alternate colour space of **that component**". Table 70 gives a *spot* colourant an alternate of
its own — each `/Colorants` value "shall be an array defining a Separation colour space for that
colourant" — while Table 71 gives every *process* component the **same** one, the process
dictionary's `/ColorSpace`. So the process components share an alternate and combine inside it,
which is ADR 1103's route unchanged; each spot colourant has an alternate of its own, and those
are what §10.8.3 combines. A space of process components alone therefore has exactly one
separation and the preference cannot move it, which
`a_process_only_nchannel_is_unmoved_by_the_simulation` holds. Running the multiply over the four
process components instead would model cyan, magenta and yellow as three filters in series and
replace `DeviceCMYK`'s own account of its inks with one nobody wrote.

**What unit the multiply is taken in.** Table 136 gives the blend function as `B(cb, cs) = cb ×
cs` over components, and its NOTE 3 fixes the unit: "multiplying any colour with black produces
black while multiplying with white leaves the original colour unchanged". Step b)'s matte is all
white, so white is the 1.0 of that formula and the product is of each separation's *ratio* to the
matte:

```text
result = W × Π (Xᵢ / W)        componentwise over X, Y and Z
```

with `W` the D50 white every XYZ in this crate is relative to. Multiplying the `Xᵢ` themselves
would darken the result by a factor of `W` per separation past the first and leave blank paper at
`W²`, which is NOTE 3's own sentence failing. `a_spot_colourants_own_separation_is_what_the_simulation_renders`
is the calibration: a second colourant at a tint of zero must leave the first's colour exactly
alone, and without the division it does not.

"Flat XYZ (no gamma)" is what `ColourSpace::cie_xyz_at` already answers — linear, D50-relative —
and step d)'s "actual device colour space" is sRGB (ADR 0009), which is `xyz_d50_to_srgb`. Step
a)'s "A default DestOutputProfile … should be consulted to determine the process colours to use"
needs no code of its own: `ColourSpace::device_family` already substitutes §14.11.5's intent for a
`/DeviceCMYK` process space where the resources name no §8.6.5.6 default, so the process colours a
page's intent states are the ones the simulation uses.

## 3. Where the preference travels, and why it is parse-time

A `Reading` carries §14.11.5's intent and §10.8.3's answer together through
`ColourSpace::parse_under`, and `Conversion` carries the second of them to the three routes that
parse a colour space after the interpreter has handed the work over — an image's `/ColorSpace`, a
shading's, a mesh's. That is the pairing argument `Conversion`'s own documentation already makes
twice, for the output intent (ADR 1008) and for §10.4.2.4's functions (ADR 1207): a parameter only
some of the four sites pass is a parameter some page silently loses, and a reader who asks for the
simulation and gets it on fills but not on images has been handed two pages at once.

It is decided at **parse** rather than at conversion because the preference changes what the space
*is* — which of two readings of §8.6.6.5 an `NChannel` space gets — exactly as `/Subtype` and the
process dictionary already do. The cost of that choice is the point of section 5: with the
preference off, not one byte of the new route is reached.

## 4. What is refused, and refused whole

A space keeps its tint transform entirely — never half of it — where any component cannot be
answered: no `/Colorants` entry for a spot colourant, an inconsistent process dictionary, or a
process space that is not subtractive. §10.8.1 makes separations a subtractive device's output and
§10.8.3 step a)'s simulated device "supports subtractive process colourants", while §8.6.6.5 says
additive process components exist here — "values for additive process colours (such as RGB) shall
be specified in their natural form, where 1.0 shall represent maximum intensity of colour" — and a
component whose zero is black is not a separation against a white matte.

Refusing whole is the clause's own guideline: a processor "should apply either the specified tint
transformation function or invoke the same alternative blending algorithm for all DeviceN
instances in the document". Half a space one way and half the other is the mixture it warns
against.

## 5. What it costs when it is off

The default is `Separations::Alternate` and the off path is the path that was there before: the
same `parse_at`, the same `Tints`, the same conversions. The one addition on it is a `Copy` enum
compared once per `DeviceN` space parsed, and a two-word `Reading` travelling the parse chain
where a one-word intent did.

Measured under callgrind in one sitting, two binaries built from one tree — this round's, and the
same tree with only this round's four hunks reversed — over `examples/callgrind_interpret`'s fifty
interpretations of page 101 of the standard, two runs each:

| | instructions |
|---|---|
| before | 1 252 940 325 / 1 252 937 239 |
| after, preference off | 1 253 064 439 / 1 253 064 426 |

**+0.010%**, about 2500 instructions per interpretation of that page, none of them per pixel. The
number that matters more is beside it: `raster_golden` does not move with the preference off.

## 6. What this does not build

§10.8.3's step a) says "[p]rocess the PDF **as if separations were to be created**", and that is a
page-scale statement: it wants a *plane per colourant*. This tree has four — the process components
§11.4.7's pair composites in — and §10.8.2's worked example is drawn on them: `1 0 0 0 k` then
`0 0 1 0 k` over one area under `/OP true /OPM 1` inside a `DeviceCMYK` page group comes out
(0, 166, 80), green. Overprint is built (§8.6.7 implemented; §11.7.4.3's special blend mode, ADRs
1157, 1169, 1170, 1181), so what step a) lacks is a plane for a **spot** ink. A spot colourant
reverts to the group's four process components when it is painted (§11.7.3), so two spot colourants
over one area cannot combine, and that is the page-scale half this does not build. What this builds is the four steps over the colourants **one
painting operation** states, which is where §8.6.6.5's per-component sentence bites and where ADR
1193's cost was measured. §10.8.3's row says so and stays owed.

Two corrections to the record while reading this clause, both of which a later round would
otherwise repeat:

- **`/SeparationInfo` is not §10.8's.** It is Table 400 in §14.11.4, "Separation dictionaries",
  deprecated with PDF 2.0, and it describes a *preseparated* file whose separations "shall be
  described as separate page objects, each painting only a single colourant". Table 391 is Web
  Capture's source information dictionary. A document stating `/SeparationInfo` is therefore not a
  witness for §10.8.3 at all; it is a witness for a construct §10.8.3 does not mention.
- **The clause's `ColorantTable`** in step a) is an ICC tag, and it is not read here. It would say
  what process colours the *simulated device* has where no output intent does.
- **§10.8.2's example is green written with `k` and yellow written with the `Separation` spaces the
  example itself names**, which is a defect one clause over and is not §10.8.3's. §11.7.4.3 NOTE 2
  makes the current colour space of a `Separation` that reverts *be* its alternate, so a
  `/Separation /Cyan /DeviceCMYK` fill inside a `DeviceCMYK` group meets the first bullet's
  condition — and `content::colour::cmyk_tints` answers `Some` only for a literal
  `ColourSpace::Cmyk`, so the special blend mode never fires for it. Measured in this session with
  the two fixtures above; it belongs to §11.7.4.3's row and to whoever owns `content/overprint.rs`.

# 1193 — A spot colourant evaluated alone, and the combination nobody states

Status: accepted. Session 1178.
Context: ISO 32000-2 §8.6.6.5 (Tables 70, 71, 72), §8.6.6.4, §8.6.4.4, §10.8.1, §10.8.3, §11.7.3,
§11.4.7; ADRs 0262 (compositing in the document's four components), 1001, 1103 (the process-only
route), 1119 (the word `departed`), 1157.
Code: `crates/pdf-colour/src/colour.rs`.
Tests: `crates/pdf-model/tests/colour_paths.rs::a_spot_components_own_separation_is_stated_and_not_taken`,
`crates/pdf-colour/src/colour.rs::an_nchannel_process_space_is_the_four_components_a_cmyk_group_composites`.

## 1. What the clause asks of a display, in its own order

§8.6.6.5 states the route for a device that has none of the named colourants, and states it as a
`shall`:

> PDF processors shall be able to approximate the colourants if they are not available on the
> current output device, such as a display. To accomplish this, the colour space definition
> provides a tint transformation function that shall be used to convert all the components to an
> alternate colour space.

A display is the clause's own example of that device. Everything else in the clause is either
executed here or disposed of below, and the whole of it was read again in this session:

- the four- and five-element array, the `names` array deciding how many operands `scn` takes, the
  order "in which the colours are given in the names array", the initial value of 1.0, tints
  treated subtractively;
- `/None` passed to the tint transform when the space reverts, and the all-`None` space that
  "shall always discard its output … it shall never revert to the alternate colour space";
- Table 70's `/Subtype`, with `DeviceN` and absence both meaning "that only the previous features
  shall be supported";
- Table 71's process dictionary, where it answers for every component of the space — the
  `NChannel` per-component evaluation in the one shape where no combination arises (ADR 1103),
  including the CMYK subset §8.6.6.5 permits and §8.6.4.4 supplies the absent value for;
- Table 72's `/Solidities`, `/PrintingOrder` and `/DotGain`, disposed of by the clause itself:
  "PDF processors need not use this information." They parameterise ink on paper, and
  `examples/nchannel_census` finds no document anywhere that states one.

## 2. The one requirement decided against

> For NChannel colour spaces, the components shall be evaluated individually; that is, only the
> ones not present on the output device shall use the alternate colour space of that component.

On a display no component is present, so each takes its own alternate: a process component the
process dictionary's `/ColorSpace`, and a spot component the `Separation` colour space Table 70
requires `/Colorants` to hold for it — "the value shall be an array defining a Separation colour
space for that colourant". **Where the space has a spot colourant, this tree does not do that**: it
reverts the whole space through the `tintTransform` of section 1, and `/Colorants` is read by
nothing.

## 3. Why — the combination is stated nowhere a display can reach

Evaluating the components individually produces *n* colours where a fill needs one, and every
candidate for combining them was read:

- **§8.6.6.5 itself.** NOTE 3 gives the freedom rather than the rule — "PDF processors can use
  their own blending algorithms for on-screen viewing and composite printing, rather than being
  required to use a specified tint transformation function" — and Table 70's own sentence says
  which of the two functions describes a combination: "the alternate colour space and tint
  transformation function of a Separation colour space describe the appearance of that colourant
  alone, whereas those of a DeviceN colour space describe only the appearance of its colourants in
  combination." The route not taken is the one the clause says is about a colourant *alone*; the
  route taken is the one it says is about the combination a fill needs.
- **§11.7.3.** It offers two treatments of a spot colour inside a group and this tree takes the
  second, which that row records: "The spot colour shall be converted to its alternate colour
  space. The resulting colour shall then be subject to the usual compositing rules for process
  colours." Those rules composite an object against a backdrop; they say nothing about combining
  two colours *one object* paints at one point, which is what an `NChannel` space of a spot
  component and a process component is.
- **§10.8.3, which is the one place an algorithm is written down**, and NOTE 3 sends a reader there
  by name. Separations to flat XYZ against a white matte, multiply-blended, converted to the
  device's space. Its verb is a permission over §10.8.1's "Whether separations are produced is up to
  the processing software", and its condition is a request this viewer has no control for — "[i]f it
  is important for the colours of the display for a PDF … to more closely match those produced when
  using separations". §10.8.3's own row is `reported` for exactly that reason. It is named here so
  that the departure has a remedy rather than a shrug: **the day a separation-simulation control
  exists, this decision is to be re-read against it.**
- **The consistency guideline**, which cuts the other way and is worth stating: "The PDF processor
  should apply either the specified tint transformation function or invoke the same alternative
  blending algorithm for all DeviceN instances in the document." Applying the per-component route to
  spaces with spot colourants and the tint transform to the rest would be the mixture this `should`
  warns against — while ADR 1103's process-only route is not a second blending algorithm but the
  `shall` of section 2 in the shape where nothing is blended, and a `shall` outranks a `should`.

So the departure is not "unimplemented". Executing the sentence requires choosing a blend the
standard declines to state for this device, over a function the file supplied for this purpose and
the clause's own earlier paragraph requires of a display.

## 4. The price, measured

`a_spot_components_own_separation_is_stated_and_not_taken` builds an `NChannel` space of two spot
colourants with the `/Colorants` dictionary Table 70 requires. `/Spot1`'s own `Separation` states
blue and `/Spot2`'s states green; the space's tint transform paints red. A full tint of `/Spot1`
draws **red** — 255 levels from its own entry's blue in two of three channels, the widest eight bits
allow — and the same `Separation` array selected on its own draws that blue, so the dictionary is
readable and its colour reachable. What is not built is combining it with the other component's.

`examples/nchannel_census` is where the population is: over 90 340 documents, 3764 `NChannel`
spaces in 1720 documents, of which **184 in 116 documents have a spot component** and are the
spaces this departure is about; the other 3580 take ADR 1103's route.

## 5. What is not departed, and is not a no-op

`an_nchannel_process_space_is_the_four_components_a_cmyk_group_composites` pins the other half.
Inside a page or group §11.4.7 composites in four components (ADR 0262), the process colour space
*is* what is being composited in, and §8.6.6.5's "interpreted directly as process values" is then
literal: `ColourSpace::to_cmyk`'s `Separation` arm hands `Tints::Process`'s components to
`Compositing::Subtractive` with no function and no conversion between, and a component the `names`
array omits arrives as §8.6.4.4's "complete absence of a process colourant". The process dictionary
decides pixels rather than describing them.

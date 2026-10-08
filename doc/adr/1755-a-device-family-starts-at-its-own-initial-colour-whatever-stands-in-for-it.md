# 1755 — A device family starts at its own initial colour, whatever stands in for it

Session 1459. Status: **accepted** and **built**. Context: ISO 32000-2 §8.6.8 (the `CS` operator's
initial colours), §8.6.5.6 (default colour spaces), §14.11.5 (output intents), §8.6.5.1 (a CIE-based
space's initial colour). Code: `ColourSpace::initial_colour_of`, `selected_device_family` and
`family_named` in `crates/pdf-colour/src/colour.rs`; `Interpreter::set_colour_space` in
`crates/pdf-model/src/content/colour.rs`. Found by ADR 1754's audit.

## 1. The defect

`ColourSpace::device_family` answers a selection of `DeviceGray`, `DeviceRGB` or `DeviceCMYK` with
the space that stands in for it: the resources' `/Default…` entry (§8.6.5.6), then a page's output
intent with as many components (§14.11.5, ADR 1001). `cs` then took the initial colour of *that*
space. For `DeviceCMYK` under a four-component `ICCBased` stand-in that is `[0 0 0 0]` — no ink, the
paper — where §8.6.8 says "[i]n a DeviceCMYK colour space, the initial colour shall be [0.0 0.0 0.0
1.0]". A PDF/X page that writes `/DeviceCMYK cs` and paints before `scn` drew white for black. A
`Separation` or `DeviceN` default started at full tint, whatever its family's initial colour was.

## 2. The reading, and the sentence that decides it

§8.6.5.6 says what happens to a device family's values when a default replaces the space: "Colour
values in the original device colour space shall be passed unchanged to the default colour space,
which shall have the same number of components as the original space." The initial colour is one of
those values: `cs` selects the family, §8.6.8 states the family's initial colour, and the default
receives it unchanged, exactly as it receives the operands of `k`. The clause's NOTE (2020) says that
provisions applying specifically to device spaces do not apply "to graphic objects painted" once a
default is in force — §8.6.7's overprint mode, written for colours "specified in a DeviceCMYK colour
space", is the kind of provision it names. It is informative and it is about painting, and reading
it as overriding the normative sentence would make one value of the family the only one not passed
unchanged. So the selection's family decides the initial colour, and the
stand-in decides what it looks like.

An output intent is the plainer case: §14.11.5 describes the device a document's colours were
prepared for and replaces no colour space at all — "[t]he ICC profile information in an output
intent dictionary should supplement rather than replace that in an ICCBased or default colour space"
— so the space is still `DeviceCMYK`, and so is its initial colour. This tree carries the intent as
the space for conversion's sake (ADR 1008); that is an implementation, not a reselection.

## 3. What was built

`selected_device_family` follows a selection — a bare name, a resource name, the array form ADR 1001
gives the same route, and `CalCMYK`, which §8.6.5.1 renders as `DeviceCMYK` — to the family it
selects, from the same table `device_family` reads (`family_named`), so the space and the family
cannot be read from two lists. `initial_colour_of` gives that family's initial colour, each component
held to the receiving space's range ("[i]f a colour value lies outside the range of the default
colour space, it shall be adjusted to the nearest valid value"), or the space's own where no family
was selected or the stand-in's component count is not the family's. `set_colour_space` asks it for
every `cs` and `CS` that parses; the `ICCBased` memo keeps asking `initial_colour`, since an
`ICCBased` selection is its own family.

## 4. Cost and population

One name resolution per `cs`, which `parse_under` has just made. HEAD's six corpus arms and this
tree's agree on every page, none of 5 795 moved, so the corpus does not exercise it; the fixtures are hand-built
(trap 8) and each fails on the old reading.

## 5. Left beside it

`soft_mask.rs`'s `backdrop_values` defaults Table 142's `/BC` to `space.initial_colour()` of a group
`/CS` parsed under the output intent, so a `/DeviceCMYK` luminosity group on a page with a CMYK intent
gets the profile's zeros (paper) for the backdrop Table 142 calls "the colour space's initial value,
representing black". It is the same function's question, `initial_colour_of(document, cs, &empty)`
answers it, and it is §11.6.5.1's row, so it is left to that row with `doc/todo/23` naming it.

# ADR 0790 — A one-component blending space is three equal channels

Status: accepted. Session 865.
Clauses: ISO 32000-2 §11.3.4, §11.3.5.3, §11.4.7, §11.6.6, §10.4.2.1, §10.4.2.2, §10.4.2.3, Table 145.
Code: `crates/pdf-model/src/colour.rs` (`Compositing::Grey`), `crates/pdf-model/src/content.rs`
(`interpreted`, `complete`), `crates/pdf-model/src/content/transparency.rs` (`Departure`,
`page_composites_in_grey`, `Interpreter::group_grey`, `GroupRun`), `crates/pdf-model/src/image.rs`.
Tests: `crates/pdf-model/tests/transparency_groups.rs::a_page_group_of_one_component_paints_every_mark_in_its_grey`,
`::an_isolated_group_of_one_component_composites_its_elements_in_grey`,
`::a_grey_page_whose_inner_group_blends_in_colour_falls_back_and_reports`,
`::a_calibrated_one_component_page_group_is_reported_though_nothing_composites`,
`::a_second_space_inside_the_pair_falls_back_to_the_device_and_reports` (rewritten).

## The question

`doc/todo/01`'s blame list has been led by §11.3.4 since ADR 0788 took §7.6.4.4 off it, and
its row named what it owed: of the spaces the clause says "shall be supported as blending colour
spaces", the one-component three — `DeviceGray`, `CalGray`, `ICCBased` 'GRAY' — and `ICCBased`
'CMYK' were reported by name. The four-component construction (ADR 0262) had settled the
question for `DeviceCMYK` by reading one sentence of the clause: the compositing formula is
applied per component, so four components are two three-channel rasters. This decision reads the
same sentence for one component.

## The reading

§11.3.4:

> The i th component of the result colour 𝐶𝑟 shall be obtained by applying the compositing
> formula to the i th components of the constituent colours

A space of one component therefore composites one number per pixel. A raster that holds that
number in each of its three channels runs the same arithmetic three times over and cannot
disagree with itself: §11.3.5.2's separable blend functions are per component, and §11.3.5.3
says of the four non-separable ones that

> Blending in gray colour spaces ( DeviceGray , CalGray and ICCBased gray) shall be done by
> conversion to RGB, blending in RGB, and then converting back to gray.

— which is exactly what three equal channels do, and each of the four functions returns a grey
for two greys (`SetSat` of a grey is `(0, 0, 0)`; `SetLum` of that is a grey), so the conversion
back is the identity on what the channels hold.

What is left is the conversion *in*, which §11.6.6 states for every painting operator:

> For isolated groups, if a group colour space ( CS ) is specified in the group attributes
> dictionary, all painting operators shall convert source colours in a colour space (that are
> not equivalent to the group colour space) to the group colour space before compositing
> objects into the group.

and the conversion *out*, which for `DeviceGray` is §10.4.2.2's "[a] gray level shall be
equivalent to an RGB value with all three components the same" — the identity on the raster.

So `DeviceGray` needs no second raster and no pair: one interpretation in which every colour
becomes its grey. **That is not true of the clause's other two one-component spaces.** A
`CalGray` component reaches the device through §8.6.5.2's gamma and a one-component profile's
through its curve; neither is affine, so the space's own component is not the channel's, and a
group composited in device grey is a different picture from one composited in `CalGray`. Those
two keep their report, and now keep it honestly (below).

## What was built

`Compositing::Grey`, a fourth answer to *what is a colour resolved for*, beside the device, a
mask's luminosity and a press's half. Its `paint` is `InkScale::Unit::grey_of` — §10.4.2.2's
weights for an RGB colour, §10.4.2.3's `1 − min(1, 0.3c + 0.59m + 0.11y + k)` for a CMYK one,
and the grey of the sRGB this tree converts everything else to — shared with the mask route so
that §11.5.3 and §11.3.4 cannot take two conversions (trap 6). Images take the converting arm
under it, as they do under a mask or a press; shadings take the producer's route rather than
the device program's, by the condition `device_program` already had.

`interpreted` chooses it for a page whose group states `/DeviceGray` — after Table 145's
remapping, so a `/DefaultGray` in the page's resources takes the page out of it — and keeps the
result where the run is *drawable*: a grey run is undrawable on one condition, a group inside it
that changed the space with something compositing in it, because that group's `Do` owes a
conversion per pixel between two spaces. `Interpreter::group_grey` does the same one scope down
for an isolated `/DeviceGray` group on a page compositing on the device, in one run of its
content, with the `GroupRun` it returns saying whether the group was drawn in its own space so
that the caller reports the space only where it was not. A knockout group is drawn too — the
four-component pair refuses knockout because §11.4.6's rewrites edit one list of two, and here
there is one — and §11.7.5.3's black generation does not enter, being inside §10.4.2.4's
conversion into `DeviceCMYK`, which is on no route into grey.

## What the reading found beside the code

**The report's condition was inherited from the wrong space.** `note_page_blending_space` and
`note_group_departures` fired only where something composites, on the argument that an opaque
`Normal` mark carries its colour through whatever space it is carried through. That is true of a
conversion with an inverse over a device's colours — three or four components — and false of one
component: a red mark painted into a `CalGray` group is a grey with nothing compositing at all.
So a `CalGray` page group with only opaque marks was drawn in colour and said nothing, which is
trap 11's fifth shape, an exemption derived for one question inherited by another. `Departure`
now carries the space's component count beside its name, and `loses_chroma` is the condition:
a one-component space reports whether or not anything composites. The same count reaches
`nested_space_departed`, so a grey group inside a press is recorded whatever it holds and the
device rerun draws it grey and reports the press, rather than drawing it in ink in silence.

**Reading the crawl by grep found four documents naming a `/DeviceGray` group and none of them
a page one paints**: three are `/Luminosity` masks' groups, which §11.5.3 already reduces to one
grey, and one is a non-isolated group, whose `/CS` §11.6.6 gives no effect. A page group of one
component — ADR 0272's census counted six in 65 703 — lives in an object stream where a grep
cannot see it, and the census that could is a corpus walk this round was not to run. So the page
looked at (trap 1) is the fixture's, rendered by four programs, and it is the finding below.

## The choice, written as one

Rendered at two pixels per unit, the fixture's opaque red, opaque green, half-alpha blue over
red and opaque cyan come out:

| | red | green | blue at ½ over red | cyan |
|---|---|---|---|---|
| this tree, §10.4.2.2 and §10.4.2.3 | 77 | 150 | 53 | 179 |
| `mupdf` | 129 | 220 | 99 | 150 |
| `ghostscript` | 130 | 220 | 100 | 151 |
| `poppler` | red | green | purple | cyan |

`poppler` ignores the space. `mupdf` and `ghostscript` agree to within one level and are 50 of
255 from this tree on a red, and what they compute is sRGB's linear-light luminance re-encoded
(`0.2126` of red in linear light is 0.51 encoded, 129). That is §10.4.2.1's *other* route:

> Although ICC enabled PDF processors should always follow the provisions and recommendations
> provided in 10.3, "CIE-Based colour to device colour", a less-capable PDF processor may
> choose to use the algorithms specified in the following subclauses 10.4.2.2 through 10.4.2.5.

This tree is ICC-enabled (ADR 0009) and the sentence is a *should*. It takes the classic route
here all the same, and the reason is not the two references: every `/Luminosity` mask on every
page has taken §10.4.2.2 and §10.4.2.3 since ADR 0217, `InkScale::grey_of` is one function, and
a blending space converting by one rule while a mask converts by another would be two
conversions for one sentence of §11.6.6. Moving *both* to §10.3's route is one decision, to be
priced against the mask population the oracle already judges, and `doc/todo/23` names it as
the row's remainder rather than taking it here with a witness of one fixture.

## What it cannot do, each reported

- `CalGray` and `ICCBased` 'GRAY' as a blending space — reported, now on every mark.
- A `/DeviceGray` group inside a press, or a press or RGB group with something compositing
  inside a grey page — a conversion per pixel at the `Do`; reported where it was introduced.
- `ICCBased` 'CMYK' — unchanged, `doc/todo/23`'s ICC `B2A` row.

## Consequences

- §11.3.4 stays `partial` with its debt narrowed to the two curved greys and the ICC CMYK row;
  §11.4.7, §11.6.6, §10.4.2.2 and §10.4.2.3 record the route.
- `doc/todo/01`'s blame list loses nothing by rank — the row is still its oldest `partial` —
  but what it names is now a choice between two routes the standard ranks and a nonlinear
  conversion out, rather than a space nobody had read for.

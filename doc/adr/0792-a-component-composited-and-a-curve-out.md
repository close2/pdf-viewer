# ADR 0792 — A component composited, and a curve out: `CalGray` and `ICCBased` 'GRAY' blending spaces

Status: accepted. Session 871.
Clauses: ISO 32000-2 §11.3.4, §11.4.7, §11.6.6, §11.7.2, §8.6.5.2, §8.6.5.5, §10.4.2.1,
§10.4.2.2, §10.4.2.3, Table 145.
Code: `crates/pdf-render/src/blending.rs` (`GreyCurve`, `resolve_grey`),
`crates/pdf-render/src/display_list.rs` (`GroupBlending` as an enum, `DisplayList::set_grey_curve`),
`crates/pdf-model/src/colour.rs` (`Compositing::Calibrated`, `GreyRoute`, `GreyIdentity`,
`ColourSpace::grey_identity`), `crates/pdf-model/src/content/transparency.rs`
(`one_component_compositing`, `page_one_component`, `Interpreter::group_one_component`),
`crates/pdf-model/src/content.rs` (`interpreted`, `complete`, `finished`),
`crates/pdf-model/src/image.rs`, `crates/render-cpu/src/lib.rs`, `crates/render-quorra/src/lib.rs`,
`crates/render-gpu/src/lib.rs`, `crates/viewer-confined/src/protocol/display_list.rs`.
Tests: `crates/pdf-model/tests/transparency_groups.rs::a_calibrated_page_group_composites_its_component_and_leaves_by_the_curve`,
`::an_isolated_calibrated_group_composites_in_its_component_and_leaves_by_the_curve`,
`::a_default_gray_takes_the_page_group_into_the_calibrated_space_it_names`,
`::a_one_component_profile_page_group_composites_in_its_component`,
`::a_one_component_space_the_clause_does_not_list_is_reported_though_nothing_composites`
(replacing `::a_calibrated_one_component_page_group_is_reported_though_nothing_composites`),
`crates/pdf-model/src/colour.rs::a_calibrated_greys_route_in_is_the_inverse_of_its_curve_out`,
`crates/pdf-render/src/blending.rs::a_curve_interpolates_between_its_samples`,
`::a_grey_raster_resolves_through_the_curve_under_its_own_alpha`,
`crates/viewer-confined/src/protocol/display_list.rs::a_page_in_one_component_round_trips_to_an_equal_list`,
`crates/render-cpu/tests/group_constructions.rs::a_group_in_a_one_component_blending_space_composites_its_component`
over `test_scenes::group_in_a_one_component_blending_space`, and the two refusal tests
`headless_gpu.rs::the_gpu_refuses_a_group_in_a_one_component_blending_space` and
`headless_quorra.rs::quorra_refuses_a_group_in_a_one_component_blending_space`.
Continues ADR 0790.

## The question

ADR 0790 drew `DeviceGray` blending spaces and left §11.3.4's row `partial` with the debt named:
`CalGray` and `ICCBased` 'GRAY', "whose component reaches the device through a curve and would
want a one-dimensional conversion out per pixel". This decision reads the clauses for those two
and builds that conversion.

## The reading

Three things are decided by the clauses and one is not, and the four are kept apart.

**What the group composites in.** §11.3.4 lists `CalGray` and "ICCBased bi-directional 'GRAY'"
among the spaces that "shall be supported as blending colour spaces", and applies the compositing
formula per component. So, exactly as for `DeviceGray` (ADR 0790), one number is composited per
pixel and three equal channels are that number three times. What that number *is* differs:
for `DeviceGray` it is the channel itself (§10.4.2.2), and for these two it is the space's own
component — §8.6.5.2's `A`, or the profile's device value.

**What the conversion out is.** §11.4.7 says where it happens — "the entire result shall then, if
the colour spaces are not equivalent, be converted to the native colour space of the output
device before being composited with the context-dependent backdrop" — and §8.6.5.2 says what it
is for `CalGray`: the `A` component "shall be first decoded by the gamma function, and the result
shall be multiplied by the components of the white point to obtain the L, M , and N components",
which are also `X`, `Y` and `Z` because there is no second stage. For a profile it is §8.6.5.5's
`AToB`, and the clause says the group's result is a *source*: the space "shall be used as both
the destination for objects being painted within the group and the source for the group's
results". This tree's ordinary `ColourSpace::to_rgb` for either space is that conversion; it is
sampled at 256 components into `pdf_render::GreyCurve` and applied per pixel by `resolve_grey`,
before the medium, which is where the four-component pair already resolves.

**What §8.6.5.5 says when the profile cannot be used.** A profile the tree cannot read falls to
`/Alternate` or, absent that, to `DeviceGray` — the clause's own substitution — *before* any
blending question is asked, so such a group composites as whatever it substituted, which for
`DeviceGray` is ADR 0790's construction. The three `kTRC` profiles the crawl holds all parse.

**What the conversion in is, which the standard does not state.** §11.6.6 has every painting
operator "convert source colours in a colour space (that are not equivalent to the group colour
space) to the group colour space before compositing objects into the group", and defines the
conversion for no colour: §8.6.5.2 is stated from `A` outward and never inward, and §8.6.5.5 uses
a profile's to-CIE half only. §10.3.1 hands a CIE-to-CIE conversion to ISO 15076-1, and
§10.4.2.1 ranks that route above the classic algorithms for an ICC-enabled processor. This tree
takes the conversion in to be **the inverse of the conversion out on the greys**: a source
colour's grey by §10.4.2.2 or §10.4.2.3 — `InkScale::grey_of`, the one function every
`/Luminosity` mask and every `DeviceGray` group already takes, for trap 6's reason and for the
reason ADR 0790 gave — and then the component whose device colour has that grey, found by search
over the same 256 samples (`GreyRoute::component_with_grey`). That is ADR 0263's construction for
a press one dimension down: the space's own `A2B`, sampled, and its right inverse. A colour
already in the group's space keeps its component, which is the clause's "not equivalent" read
literally. Two properties follow and are tested: an opaque device grey painted into the group
comes back out as itself, and a component sent out and brought back is itself to a level of 255.

A curve with no inverse is not a route — the sampled greys must be monotone, which every gamma
and every tone curve the standard admits is — and `GreyRoute::of` answers `None` for one that is
not, which keeps the report the space had.

## What was built

`Compositing::Calibrated(Arc<GreyRoute>)`, a fifth answer to *what is a colour resolved for*,
whose `paint` is `GreyRoute::component_of`. `page_one_component` chooses it, or
`Compositing::Grey`, for a page whose group states a one-component space this tree draws —
after Table 145's remapping, so a `/DefaultGray` of `CalGray` takes a `/DeviceGray` page group
into the calibrated route rather than out of grey — and `finished` states the curve on the
display list (`DisplayList::set_grey_curve`) so that `replace` restates it. One scope down,
`Interpreter::group_one_component` does the same for an isolated group on a device page, and the
curve rides on the group as `GroupBlending::OneComponent`, the enum's second shape beside the
four-component pair. Images take the converting arm under it before the `DeviceGray` fast arm,
because a `DeviceGray` sample is not its own component there; shadings take the producer's route
by the condition `device_program` already had.

`render-cpu` applies the page curve after the pair and before the medium, and the group curve in
`composite_in_own_space` after one `encode`; `render-quorra` applies the page curve on its
readback and refuses a group carrying one, as it refuses the pair; `render-gpu` refuses both by
name. `viewer-confined`'s protocol writes a third tag for the page's space and the group's, and
refuses a list stating a pair and a curve at once.

## The choice, written as one

Rendered at two pixels per unit, the fixture's half-alpha black over white, in a page group of:

| | `CalGray` `/Gamma 1` | `CalGray` `/Gamma 2.2` | `DeviceGray` |
|---|---|---|---|
| this tree | 188 | 129 | 128 |
| `mupdf` | 188 | 129 | 128 |
| `ghostscript` | 187 | 128 | 127 |
| `poppler` | 127 | 127 | 127 |

The two references that honour the space composite in its component and leave by its curve
exactly as this decision does; `poppler` ignores the space. What they compute differently is the
conversion *in* of a chromatic colour — an opaque red is 77 here and 129 there, §10.4.2.2 against
sRGB's linear-light luminance — which is ADR 0790's standing choice and is not moved here: it is
one decision for the masks and the blending spaces together, and `doc/todo/23` prices it. On the
three `ICCBased` 'GRAY' page groups the crawl holds (`1407449.pdf`, `2760152.pdf`,
`6942624.pdf`, found by `press_census` over all 145 archives in 62 seconds under
`tools/bounded.sh`), the pages draw and agree with all three references by eye; the crawl holds
no `CalGray` page group.

## What it cannot do, each reported

- A one-component space §11.3.4 does not list as a group's `/CS` — a `Separation` or `Indexed` —
  and a profile whose curve has no inverse: reported on every mark, with a message that now
  names the clause's list.
- A calibrated group inside a press, or a press or RGB group with something compositing inside a
  calibrated page: a conversion per pixel at the `Do`; reported where introduced, as for grey.
- `ICCBased` 'CMYK': unchanged, `doc/todo/23`'s ICC `B2A` row.
- A calibrated *group* on `render-quorra` and `render-gpu`: refused by name, to the CPU backend.

## Consequences

- §11.3.4 stays `partial`, with its debt narrowed to `ICCBased` 'CMYK' and the route into a
  one-component space; §11.4.7, §11.6.6, §8.6.5.2 and §8.6.5.5 record the construction.
- `pdf_render::GroupBlending` is an enum. Every site that read its `black` field now asks for it
  by `GroupBlending::black`, and a new shape of group-scoped conversion has a place to go.
- The survey line over `batch2/GHOSTSCRIPT` read one document fewer incomplete than round 864's
  (471 against 472), taken in the same round as `doc/todo/03` section 39's experiment; which
  document is not this decision's finding and was not chased.

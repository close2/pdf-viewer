# 2012 — DeviceCMYK inside a group of an ICCBased CMYK space is that space

Session 1588, slot 4 of batch ninety-three, a clause round. Status: **accepted** and **built** in
`crates/pdf-colour/src/colour.rs` (`Reading::within_cmyk_group`, `ColourSpace::device_family`,
`Conversion::parse_space`) and `crates/pdf-model/src/content/transparency.rs` (`stated_cmyk_group`,
`page_cmyk_group`, `Interpreter::enter_cmyk_group`). Context: ADRs 0797, 1001, 1008, 1996; the
§11.7.2, §11.6.6 and §14.11.5 ledger rows.

## 1. The question, and what the clauses say

ADR 1996 §4 found that on a page with an output intent, a `k` inside a group whose `/CS` is an
`ICCBased` CMYK profile other than the intent's was drawn through the intent's profile and then
converted into the group's. Read anew, §11.7.2 has three sentences about this, and all three are
`shall`s:

- "If an isolated transparency group or page has an ICCBased 'CMYK' colour space , DeviceCMYK shall
  be redefined within the transparency group to be the same as the blending colour space".
- A device space of the same count inside "the current group or an ancestor of the current group"
  with a CIE-based space is "for compositing purposes only" the space of "the nearest such
  ancestor" — so a `k` inside an RGB group inside a CMYK one is still the CMYK group's.
- "Nevertheless for graphics objects that are specified in DeviceCMYK , the full semantics of
  DeviceCMYK shall still apply, including overprinting."

§14.11.5 makes the intent "for informational purposes only", so the ranking is the standard's: the
redefinition outranks the intent. §8.6.5.6's default outranks the redefinition, because §11.7.2
itself names a remapped device colour as the one its device rules do not reach — "unless remapped
to default CIE-based colour spaces" — and Annex P applies the default before any search of the
ancestors. §11.6.6 confines all of it to isolated groups: "[f]or non-isolated groups, or if no group
colour space is specified, the group colour space shall be inherited from the parent group or page".
Which entries *state* such a space is Table 145's: a four-component `ICCBased` one outright, and a
`/DeviceCMYK` one where its remapping through the group's own `/DefaultCMYK` makes it one.

## 2. Decision

`Reading` and `Conversion` carry the redefining space beside the intent, and
`ColourSpace::device_family` ranks four sources for `DeviceCMYK`: the default, the redefinition,
the intent, the device. `pdf-model`'s `Interpreter::cmyk_group` holds the nearest redefinition as
a scope: from the page group at the start, replaced by an isolated group in `group_commands` across
every run of its content — the press's, the black half's, and a rerun in the parent's space, since
the redefinition is the colour's meaning and not the raster's — and by a soft mask's group's own
`/CS` alone, Table 145's group "that has no parent group or page from which to inherit". Images,
shadings and meshes parse through `Conversion::parse_space`, one reading for every route that
parses after the interpreter. `icc_spaces` is keyed by the redefinition beside the object, because
a profile that does not parse is its `/Alternate`, which the redefinition can change. ADR 1996's
`ProcessTints::on` then needs nothing new: a redefined `k` is the press's own profile, so the mode
applies.

## 3. Proof, and what moved

Six tests in `tests/overprint.rs`, each an equality with a page painting the named colour: `k` on a
page group of `/Press` under `/Other`'s intent is `/Press`'s colour, and keeps the mode; inside an
isolated `/Press` form on a page of no group it is `/Press`'s, though the group is drawn in the
parent's space; an RGB group inside it keeps `/Press`'s; a non-isolated group redefines nothing; a
`/DefaultCMYK` of `/Other` outranks it. Planted away in `device_family`, the first four fail;
ignoring isolation fails the fifth; the redefinition before the default fails the sixth.

**No corpus page moves.** HEAD's six arms against this change's, by page digest: 968 pages at 1×
and 965 at 4× on each of the processor, device and compute lanes, none different, with the change
in the corpus binary (its SHA-256 `b8ae9c79…` against HEAD's `2b692928…`); `raster_golden` holds
974, moves 0. A page moves only where an isolated group or page states an `ICCBased` CMYK space and
paints a device CMYK colour inside it; that no page of the arms did is a fact about the corpus and
not the standard (trap 8), so the fixtures are the evidence — as for ADR 1996, whose case this is.

## 4. What it costs

A profile read for this question only where the entry can state four components
(`may_state_four_components` reads the family and `/N` off the entry), so a group or page of an RGB
or grey profile — the commonest blending spaces — pays nothing; a four-component one is read once
a page and remembered in `icc_spaces`. Per colour operator, one `Option` beside the intent in
`Reading` and `Conversion` and one more `u128` in their cache keys.

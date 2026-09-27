# 1338 — A `/Colorants` entry speaks only for the colourant its key names

Session 1250. Status: accepted and **built**.
Context: `crates/pdf-model/src/colourants.rs` (`names_its_key`, `colour_space`, the tests),
`crates/pdf-colour/src/colour.rs` (`ColourSpace::nchannel_separations`),
`crates/pdf-model/tests/spot_colourants.rs`, the §8.6.6.5 ledger row.
Amends: ADR 1317 section 2's rank of sources ("a `Separation` naming it, or the `Separation` an
`NChannel`'s `/Colorants` holds for it"), which this makes precise. ADR 1317 is not edited.
Clauses: ISO 32000-2 §8.6.6.5 (Table 70), §10.8.3, §14.11.5 (Table 402).

## 1. What "ColorantTable" names, and what it does not

The round was briefed that ADR 1317 left `ColorantTable` unread and that it was Table 70's
`/Colorants`. Neither held on reading. ADR 1317 does not use the word; ADR 1229 does, and the tree's
one mention is `simulated_press`'s quotation of §10.8.3 step a): "A default DestOutputProfile , if
available for a subtractive device, or ColorantTable values, if available for a subtractive device,
should be consulted to determine the process colours to use". The `ColorantTable` there is Table
402's, an entry of a `/DestOutputProfileRef` dictionary: "An array of colourant names, each of which
shall be encoded as a name object. The order and names of the colourants in ColorantTable shall be
identical to those in the ICC colorantTableTag in the ICC profile." It names a *referenced*
profile's colourants and states none of their appearance, which lives in a profile this renderer
cannot fetch (no filesystem, no network: principle 3, and the §14.11.5 row's reading). So it can
tell a simulated device *which* process colourants it has, never what they look like, and nothing
here is built from it. It is §10.8.3's `should`, and that row's to carry.

## 2. What Table 70's `/Colorants` says, and what was missing

Table 70: "For each entry in this dictionary, the key shall be a colourant name and the value shall
be an array defining a Separation colour space for that colourant (see 8.6.6.4, "Separation colour
spaces"). The key shall match the colourant name given in that colour space." And: "the alternate
colour space and tint transformation function of a Separation colour space describe the appearance
of that colourant alone, whereas those of a DeviceN colour space describe only the appearance of its
colourants in combination."

Both readers already read the dictionary: `colourants::colour_space` ranks the entry first for a
spot plane's curve, for any `DeviceN` (Table 70 makes it optional outside `NChannel`, and
§8.6.6.5 EXAMPLE 2 is a plain `DeviceN` that states one), and `nchannel_separations` separates an
`NChannel` through it. Neither asked the second sentence. The first took any `Separation` array,
the second any colour space at all, so an entry under `/Orange` naming `Green` was read as
`Orange`'s appearance alone.

## 3. The build

An entry counts only where it is a `Separation` array whose colourant name equals its key; any
other entry is read as absent, which each reader already handles: the spot plane's curve falls to
the `DeviceN` tint transform with that component alone, and the `NChannel` reverts through its tint
transform, as where the entry is missing. A malformed entry is a writer's `shall` broken, and
reading it as absent is the one reading that does not make the file's statement about another
colourant into one about this one.

Tests: `a_colorants_entry_states_its_colourants_separation` (§8.6.6.5 EXAMPLE 2 with its functions
written out: `Orange`'s curve equals a lone `Separation`'s and differs from the tint transform's),
`a_colorants_entry_of_another_colourant_is_not_read_as_this_ones`,
`an_nchannel_whose_colorants_entry_names_another_colourant_is_not_separated`, and EXAMPLE 5 (the
mixing hints) as `the_mixing_hints_example_names_its_two_spots_and_separates_them`. The two
key-rule tests were calibrated by weakening the check to "a `Separation`, or the right name", under
which each fails.

## 4. What it moves

Nothing under the default control: both readers run only under §10.8.3's simulation, which is a
reader's request and `off` unless asked (ADRs 1228, 1329). `raster_golden` is not moved by it.

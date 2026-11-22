# 3374 — An empty `/CharSet` and a button state naming no stream are faults this validator reports

Session 2168. Status: accepted. Context: ISO 32000-2 §9.8.1 (Table 122), §12.5.5, §7.3.9; ISO
19005-2 sections 6.2.11.4.2 and 6.3.3; round 2164's veraPDF triage (`issue2462`, `issue15557`).

## The questions

veraPDF failed two outputs this tree wrote as conforming:

1. `issue2462` keeps `/CharSet ()`. `charset_names` reads a string it finds no name in as "not
   read", so the row skipped it — right for a string written in some form this crate does not
   read, wrong for an empty one, which every reading agrees lists no name.
2. `issue15557`'s check box maps `/Off` and `/Yes` to the *name* `null`. The row asked only that a
   button's `/N` be a dictionary.

## Decision

1. `charset_lists_every_glyph_in_the_program` reads a string of nothing but white space as the empty
   set (`charset_names` and its audit-cited test are unchanged), so it omits every glyph of the
   program and the existing remedy removes it (`DescriptorSetRemoved`).
2. `normal_appearance_shape` holds a button's subdictionary to §12.5.5's "the subdictionary shall
   define multiple appearance streams": one that maps no state to a stream and names something
   other than one is not that subdictionary. One that defines a stream for some state and names
   something else for another (`checkbox-bad-appearance`'s `/Off /Off`) is the subdictionary with
   an entry that defines nothing, read as §7.3.9 reads a null value; one whose every value is null
   states nothing either way and is left to the rows about an appearance being present. The census
   held the first, stricter reading against `checkbox-bad-appearance`, whose output veraPDF had
   passed, and this is the reading that replaced it. `issue15557` is now refused at
   `annotations/normal-appearance-shape` (the collapse finds no stream to select) rather than
   written as conforming.

## Consequences

`issue2462` passes veraPDF at 2b. `issue15557` moves from a false "conforms" to a refusal; what this
reader draws for a state naming no stream, and whether ADR 3359's preserved-nothing answers it, is
the open part.

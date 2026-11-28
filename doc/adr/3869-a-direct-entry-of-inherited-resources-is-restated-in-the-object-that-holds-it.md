# 3869 — A direct entry of inherited resources is restated in the object that holds it

Session 2292. Status: **accepted**. Amends ADR 3861's one remaining shape. Context:
`doc/questions/A834` (remove, never cover), ADRs 1363, 3861; ISO 32000-2 §12.5.6.23, §7.7.3.4,
§7.8.3, §8.7.4.5. Code: `crates/pdf-transform/src/redact/undrawn.rs` (`DirectEntry`,
`direct_entry`), `redact.rs` (`reserve_slots`, `place_holders`, `restated_holder`).

## Context

ADR 3861 lets an original nothing else draws take a page's edit in its own place. A shading or
pattern written **directly** in resources a page inherits from `/Pages` has no place of its own:
the page's restated `/Resources` named the replacement, and the node's entry kept the producer's
dictionary, its `/Function` stream — the samples the region covered — reached from the node for
every page that inherits it. The probe (`redact::a_direct_shading_pattern_only_the_redacted_page_paints_keeps_no_covered_samples`
and `…a_direct_shading_only_the_redacted_page_paints_keeps_no_covered_samples`) found the
producer's sample table in the output before this change.

## Decision

- An entry is *direct and inherited* when the page's located destruction replaced it at the page's
  level and the `/Resources` holding it is stated by an ancestor node. Its **holder** is the
  nearest indirect object on the way: the node itself, or the resource dictionary or the category
  table it names by reference; its path is the keys from the holder down to the entry. The entry
  found is compared with the page's effective resources, so a `/Parent` chain that disagrees with
  the page tree finds nothing.
- The search for what the output draws counts a direct entry drawn wherever an equal dictionary is
  reached, except by the page that replaced it, whose draws reach its replacement. Equality stands
  in for *resolved to this entry*: an equal dictionary elsewhere counting too errs toward *drawn*,
  the direction ADR 3861 allows.
- An entry nothing else draws: its holder's slot is reserved before the closure walk, and the
  holder is written with the page's placed replacement at the path and every other entry carried
  as the producer wrote it. Every page inheriting it resolves the name to the replacement, none of
  them draws it, so no drawing changes; the first page takes it, as in ADR 3861.
- A holder something else already replaced keeps its entries and the leak, and no redaction step
  replaces a `/Pages` node or a resource dictionary today.

## Consequences

Held by the two probes, `…a_direct_shading_in_a_table_named_by_reference_is_restated_in_the_table`
and the keeping case `…a_direct_shading_the_other_page_paints_is_kept_for_it`, each comparing the
other page's pixels before and after. Left: a direct entry of a `/Resources` object the page itself
names by reference and shares with another page (§7.7.3.3) is not restated, since restating it
would write the object even where nothing else reaches it; that is ADR 3861's shared-resources
shape for direct entries, and a later round's.

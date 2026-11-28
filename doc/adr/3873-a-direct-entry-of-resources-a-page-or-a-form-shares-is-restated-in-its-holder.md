# 3873 — A direct entry of resources a page or a form shares is restated in its holder

Session 2293. Status: **accepted**. Amends ADR 3869's last shape. Context: `doc/questions/A834`
(remove, never cover), ADRs 1363, 3861, 3869; ISO 32000-2 §12.5.6.23, §7.7.3.3, §7.8.3.
Code: `crates/pdf-transform/src/redact/undrawn.rs` (`direct_entry`, `stating_node`),
`redact/located.rs` (`override_fallen_back`), `redact.rs` (`reference_counts` passed to `adopt`).

## Context

ADR 3869 restates a shading or pattern written directly in resources a page *inherits*. Three
shapes kept the producer's function stream, the samples the region covered, for pages that never
paint the entry. The probe (five tests of `crates/pdf-transform/tests/redact.rs` named
`a_direct_shading_…`, before this change) found the producer's sample table in each output:

1. the page names its `/Resources` (or a category table in it) by reference, and another page
   names the same object (§7.7.3.3);
2. a form the page draws names its `/Resources` by reference, and another page names that object;
3. a form with no `/Resources` resolves in the page's own (§7.8.3): the form's edit named the
   replacement, and the page's dictionary, written with the page, still held the producer's entry.

## Decision

- `direct_entry` asks where the stream that resolved the name — the page, or the form whose key it
  is — finds its resources: the form's own `/Resources`, else the page's chain (`stating_node`).
  The holder is the nearest indirect object on the way, as in ADR 3869. A holder the stream itself
  is, or one that only the stream names (its reference count, `reference_counts`, is one), is
  rebuilt with that stream and needs nothing more; one something else names is restated with the
  replacement at the entry's path, where nothing else draws the entry.
- A form that states no `/Resources` gives its replacement to the page's own resources too
  (`override_fallen_back`), unless the page's content replaced the same name, whose replacement is
  the one the page's own draws need.

## Consequences

Held by `…in_resources_both_pages_name_keeps_no_covered_samples`, its pattern and table forms,
`…in_a_form_s_resources_another_page_names…`, `…a_form_without_resources_paints_from_the_page…`,
and the keeping case `…both_pages_name_and_paint_is_kept_for_the_other`; each compares the other
page's pixels before and after. An object only the edited stream names is not restated, so no
unreferenced copy is written. ADR 3874 closes the sweep.

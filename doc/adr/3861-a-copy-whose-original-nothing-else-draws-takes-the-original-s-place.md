# 3861 — A copy whose original nothing else draws takes the original's place

Session 2290. Status: **accepted**. Amends ADR 1196 §2 (copy or replace) and ADR 1363's located
shading data. Context: `doc/questions/A834`, ADR 3849; ISO 32000-2 §12.5.6.23, §7.7.3.3,
§7.7.3.4, §7.8.3, §8.10.1. Code: `crates/pdf-transform/src/redact/undrawn.rs`,
`redact.rs` (`reserve_slots`, `place_located`).

## Context

ADR 1196 decides *replace or copy* by references: an object referenced once through resources
the page does not share is the page's, and anything else is copied so the producer's object stays
for the placements that draw it. A reference is not a placement. Resources inherited from
`/Pages` (§7.7.3.4) or named by two pages (§7.7.3.3) reach every object in them from every page,
so a form, an image or a located shading one page draws and another only inherits was copied, and
the producer's object — the covered text, the destroyed samples — stayed in the file for a page
that never draws it. The probe (`redact::a_form_inherited_by_a_page_that_never_draws_it_keeps_no_covered_marks`
and three like it) found `SECRET` in the output before this change.

## Decision

- After every page is walked, each original a page copied (a form edit that is not a placement's
  copy, a cleared image, a shading or pattern whose located data were destroyed and which the
  entry names by reference) is looked for in what the output still draws: every page's content and
  every annotation's `/AP` and `/RO`, entering each form a `Do` names in its own resources or, where
  it states none, the enclosing ones (ADR 0255). A redacted page's draw of an object it copied is
  its copy's and does not count; a page's `/Thumb` and the catalog's `/AP` name tree count as
  drawing all they reach.
- An original nothing draws takes the **first** page's edit in its own place, as an owned object
  does; a second page that copied it keeps its copy, since what it removed from its own is drawn
  on the first page outside that page's region. Every page that draws the original is unchanged,
  which is what lets the edit replace an object every page's resources name: §7.8.3's names resolve
  as before, and no page draws the object whose bytes change.
- The search errs toward *drawn*: a stream that does not decode, a pattern, a font, a graphics
  state or a shading named by an operator is counted as drawing everything it reaches by
  reference; past `MAX_TOKENS` (2^24 tokens) every original counts as drawn and keeps its copy.
  Wrongly *drawn* is the behaviour before this ADR; wrongly *undrawn* would change another page,
  and no branch answers it.

## Consequences

Held by `redact::a_form_inherited_by_a_page_that_never_draws_it_keeps_no_covered_marks`,
`…a_form_in_resources_another_page_names_and_never_draws_keeps_no_covered_marks`,
`…an_image_inherited_by_a_page_that_never_draws_it_keeps_no_covered_samples`,
`…an_inherited_shading_pattern_only_the_redacted_page_paints_keeps_no_covered_samples`,
`…an_inherited_soft_mask_group_only_the_redacted_page_sets_keeps_no_covered_text`,
`…an_inherited_form_holding_a_form_the_other_page_draws_is_edited_in_place`, each comparing the
other page's pixels before and after; and by the keeping cases
`…an_inherited_form_the_other_page_draws_is_kept_for_it` and
`…an_inherited_form_an_appearance_on_the_other_page_draws_is_kept_for_it`.

Left: a shading or pattern written **directly** in inherited resources has no object to replace,
so its replacement is still a new object and the producer's function stream stays reachable from
the inherited entry (`located::Key::Direct`). A shared page content stream is drawn by the page
sharing it and is not a copy.

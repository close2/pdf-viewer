# 3809 — A structure element written directly is restated inside the object that holds it

Session 2277. Status: **accepted**. Context: ADR 3725, `doc/questions/A834`; ISO 32000-2
§12.5.6.23, §14.7.2 Table 355, §14.9.3–§14.9.5.

## Context

ADR 3725 restates every structure element whose content a redaction reached, and its ancestors,
without `/ActualText`, `/Alt`, `/E` (and `/T` on the element itself). It walked `/K` by reference
only: Table 355 lets `/K` hold a kid as a dictionary written in place, and such an element was
neither read for its content items nor restated, so its descriptions of the removed content were
carried into the output.

## Decision

- An element is addressed by its **place**: the indirect object that holds it and the `/K`
  positions from that object's dictionary down to it (`redact/traces.rs`'s `Place`). An element
  that is an object is its own holder with an empty path.
- The walk descends into kids written directly as it does into referenced ones, inheriting `/Pg`
  the same way. Every element to be restated is grouped by holder, and each holder is written
  again once, with each of its elements' entries removed at its place. The report names a direct
  element as "the structure element written directly in object N's /K (at i, j)".
- A structure tree root written directly in the catalog has no holder for its direct kids; those
  are walked for their referenced kids and not restated. No file this verb has met does this;
  Table 29 does not require the root to be indirect, so the case is named here rather than
  pretended absent.
- An element reached both as marking removed content and as an ancestor of another is restated
  once, with the union of the keys and "every content item it marks was removed" where its own
  items say so.

## Consequences

`redaction_leaves_no_trace::a_structure_element_written_directly_in_its_parent_is_restated_there`
fails without the walk into direct kids, finding the four strings ADR 3725 removes.

# 3845 — A structure tree root the catalog states directly is restated in the catalog

Session 2286. Status: **accepted**. Context: ADR 3725, ADR 3809, `doc/questions/A834`; ISO 32000-2
§7.7.2 Table 29, §12.5.6.23, §14.7.2 Table 355.

## Context

ADR 3809 addresses a structure element by its holder object and its `/K` path, and named one case it
left: Table 29 does not require the catalog's `/StructTreeRoot` to be indirect, and a root written
directly there has no object to hold its direct kids. Those kids were walked for their referenced
kids only, so an element written directly in the root's `/K` that marked removed content, or an
ancestor of one, kept its `/ActualText`, `/Alt` and `/E` in the output.

## Decision

- A place carries `in_catalog`: its holder is the catalog, and its path starts at the catalog's
  `/StructTreeRoot` instead of the holder's own dictionary. The root's direct kids are walked and
  restated like any other element written directly.
- `traces::elements` answers the restated root under the catalog's number; `cut_traces` keeps it
  apart from the slots it reserves, `write_document` replaces the catalog where nothing else did,
  and `place_catalog` writes the restated root in place of the producer's, its placement-copy
  references added the way `place_restated` adds them (`restated_dict`).
- The report names such an element "the structure element written directly in the /K of the
  structure tree root the catalog (object N) states directly (at i, j)".

## Consequences

`redaction_leaves_no_trace::a_structure_tree_root_the_catalog_states_directly_is_restated_there`
holds a root, a document element and the marking span all written directly, and finds none of the
removed text in the output.

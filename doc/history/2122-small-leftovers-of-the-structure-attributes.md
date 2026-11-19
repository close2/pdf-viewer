# Round 2122 — a `PrintField` entry from its own owners, `/TextAlign` on the runs, a policy sentence

A host round in the `lane4` worktree on four leftovers of rounds 2117 and 2119.

## Built

- **§14.8.5.6's owner sentence** (ADR 3189). `Tree::print_field` said its owner restriction was
  applied by `Tree::attribute`, which admits every owner §14.8.5.3 ranks. The clause narrows it
  ("excluding Layout, List, Table and Artifact"), so the entries are read through a new
  `Tree::print_field_attribute`, as §14.8.5.4.1's and §14.8.5.8's are. Test:
  `a_print_field_attribute_is_read_only_from_the_owners_its_clause_admits`, with a control in which
  §14.8.5.3 alone takes the `List` object's `/Role`.
- **§14.8.5.4.3's `/TextAlign`** (ADR 3191). A634 decides it without a question: the export takes
  what a platform counterpart carries and a sighted reader sees drawn, and keeps the reflowing view
  out. `pdf_model::structure::TextAlign` and `Tree::text_align`; `viewer_core`'s `Inline` carry
  inherits it and passes over an inline element's statement; `LineAlignment::of` turns start and
  end into left and right by `/WritingMode`; the node's `alignment` crosses the confined protocol
  and `viewer_accessibility` sets it on the runs. The cell's `Start` default is not exported:
  that is a choice, written down. Tests in pdf-model, viewer-core, viewer-accessibility and the
  protocol round trip.
- **The policy sentence** (ADR 3192). A handed copy that is not a PDF no longer says "its media type
  is above", because nothing printed one; a fetched copy's still does, and that is now asserted.

**Ledger.** §14.8.5.4's aggregate note restated from its rows (ADR 3190, after ADR 1599), with
§14.8.5.4.2's stale "exports to no format"; §14.8.5.4.3's and §14.8.5.6's notes and tests extended.
Every status is unchanged (`implemented`).

**Gates.** Over pdf-model, viewer-core, viewer-accessibility, viewer-confined, viewer-host: fmt 0;
clippy `-D warnings --all-targets` 0 (after a doc quote became a blockquote and a rectangle encoder
was drawn out of `encode_accessibility`, 101 lines); `pdf-sandbox --bins` 0; nextest `--lib --bins
--tests` 4075 run, 4075 passed, no difference from `expected-failures-current.json` (empty); rustdoc
`-D warnings --document-private-items` 0 after a link to a private item became code; fuzz `cargo check
--bins` 0; `cargo test -p conformance` 0 after two quotations gained their clause.

**Left.** `viewer-ffi` does not carry the alignment to C callers; Table 378's `/BackgroundColor`
(§14.8.5.4.2's, not §14.8.5.4.3's) is exportable under A634's reading and unbuilt; §14.8.5.5's and
§14.8.5.7's owner sentences are not checked on the reading side.

**Trap.** `pkill -f` on a pattern naming the target directory killed the shell that ran it, and
the background test with it: the `pgrep` waiters habit, in its `pkill` form.

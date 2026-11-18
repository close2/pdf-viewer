# 2081 — The viewer audit groups G111 to G122

Session 2081, an audit-fix round in `pdf-viewer-gwg`. Groups G111 to G122 of
`doc/reviews/2038-specification-audit/fixes.md`: 14 findings (3 B, 8 T, 3 D).

- **Fixed 14, argued 0, owner 0, open 0.** Each line of `fixes.md` names its ADR and tests.
- ADR 3012: a collection's details view shows every Table 155 subtype. `Collection::items`
  carries §7.11.6's `/CI` by `/EmbeddedFiles` key (a field added to `pdf-model`'s public
  `Collection`, the one `pdf-model` change, which the brief had kept to T134; nothing else could
  reach `collection_rows`, whose signature the audit test fixes). `CompressedSize` is the stream's
  `/Length`. The confined wire carries the items.
- ADR 3013: an element's own lines and its children stand in `/K` order in the accessibility tree
  (§14.8.2.5.1). `AccessibilityNode::interleaving` is new, on the confined wire too.
- ADR 3014 supersedes ADR 2384's sentence on the ruby roles (`RB` is `Label`, not `Ruby`).
- D26: `tree::artifact`'s doc and its test now say the description reaches the raw update, which
  AT-SPI's `common_filter` strips of a `GenericContainer`. D53: `doc/todo/30` records that
  `/HideWindowUI` is checked only as a state mapping and owes a probe line; no Windows run.
- The eight T findings are tests the audit added. Each passes, and each failed on a plant of its
  defect (all ten tests in one planted run, then reverted): T16 (Info title from XMP), T121
  (Warn level refused), T134 (style flags dropped), T71 (red dropped), T114 (saved over the
  source), T89 (duplex edges swapped), T106 (no descriptor sent), T49 (`RT` as `Label`).
- Ledger: 12.3.5.1 and 14.8.2.5.1 gain the audit tests and a sentence each; no status changed.
- Five manifest failures (B05 three, B06, B43) pass and left `expected-failures-current.json`.
- New code avoids `map_or` with a default and `#[expect]`, for the 1.100 pin the merge brings.

**Gates.** Over `-p pdf-model -p viewer-core -p viewer-host -p viewer-accessibility -p
viewer-confined -p viewer-ui -p viewer-gtk -p viewer-qt`: fmt exit 0; clippy `-D warnings
--all-targets` exit 0; nextest `--lib --bins --tests` under `xvfb-run -a` exit 100, 4284 run, 6
failed, exactly the manifest's remaining `pdf-model` entries, none new; `cd fuzz && cargo check
--bins` exit 0; rustdoc `-D warnings --document-private-items` exit 0 for each touched crate;
conformance failed only on this record's first draft, and passes with this paragraph written.

Habit: a planted run of several T findings' tests at once costs one build, not one per finding.
Plant only where the plants touch disjoint code paths, so each failure is its own plant's.

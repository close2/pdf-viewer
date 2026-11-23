# Round 2178 — one interaction gate, and the active area where it is drawn

A fix round in the `gwg` worktree for audit 2175 (§12.5): its six B findings, nine failing tests.

## What changed

- **One gate, two questions** (B02, B03; ADR 3413). `view::annotations::passes` is asked by every
  route: `annotation_at`, the new `view::link_at` (a click's activation, the hover's cursor,
  `Query::LinkAt`), `field_at` and the new `field_at_in_view` (`Query::FieldAt`), the widget and
  free-text carets, the unreachable-press report, the Tab focus step, the accessibility tree.
  `Gate::Responds` adds a widget's field `ReadOnly` (inherited, an import's flags, a script's
  `readonly`); `Gate::Present` does not, because Table 227 defines the field's flag as the pointer's
  response only — the focus, the tree and a host's refusal by name still reach a read-only field.
- **The active area follows the drawing** (B01; ADR 3414): `annotation::unaltered_point` maps the
  point through the inverse of the drawing's own §12.5.3 adjustment before the region is tested.
- **LockedContents fills** (B04; ADR 3415): `restriction::asserted` asserts `AnnotationLocked`
  for `FillInForm` where a widget of the field sets bit 10, at the policy's four levels.
- **Readers** (B05, B06; ADR 3416): an `/N` subdictionary with no `/AS` draws nothing and is said;
  a rectangle is exactly four numbers and a matrix six, and a malformed `/Rect` (placed by
  `/BBox`) or `/Matrix` (identity) is said beside the drawing.

## Ledger

12.5.3, 12.5.1, 12.5.5, 12.5.2, 12.5.6.4, 12.5.6.5, 12.7.4.1 and 7.9.5 re-read, notes corrected; no
status moves. 2175's findings `fixed`; their nine manifest entries removed.

**Gates.** fmt exit 0. clippy `-D warnings --all-targets -p pdf-model -p viewer-core -p
viewer-host -p viewer-qt` exit 0 (101 first: `doc_markdown`, then `too_many_lines`). `pdf-sandbox`
0. nextest `--lib --bins --tests` of the four (`xvfb-run -a`): 4069 run, 4069 passed, 27 skipped;
`audit-diff.py` "failed 0, expected 0". rustdoc `-D warnings --document-private-items -p pdf-model
-p viewer-core` exit 0. `fuzz` `cargo check --bins` exit 0. `cargo test -p conformance
--no-fail-fast` exit 0. `raster_golden`: held 973, moved 1 — `issue15557.pdf` p1, reports only: a
check box whose `/AP` keys by state with no `/AS` (B05's report); no pixel moved; regenerated.

## Trap

The first cut put a read-only field's widget behind the whole gate, and four tests caught it: the
accessibility tree lost a calculated total, Tab could not reach a locked push button, and a host's
click on a read-only field went silent. Table 227's "interact" has its own "that is"; read the
definition before applying the word to every route.

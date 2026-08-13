# 465 — A header cell's axis, and the grid that decides it

**Finding.** Every table header cell in every document reached a screen reader as a **column**'s
header, because `role.rs` mapped `TH` to `accesskit::Role::ColumnHeader` and nothing read Table
384's `/Scope`. The standard settles all three cases: `/Scope` where a document states one, and
§14.8.5.7's four-bullet assumption — first row *and* column → `Both`, first row → `Column`, first
column → `Row`, otherwise `Both` — where it does not. The assumption is about the cell's place in
the **grid**, so a `/RowSpan` above a row moves that row's first child off column zero, and
§14.8.4.8.3's own NOTE is what says the row and column are the *logical* ones rather than
`WritingMode`'s: "the structure always reflects the logical content order of the table". Over 978
documents the axes come to **Row 3114, Column 1670, Both 1181** — more than half of every header
cell in the corpus was being announced as the wrong kind. Read back off a real AT-SPI bus:
`pdfjs_wikipedia.pdf`'s ten `[ColumnHeader]` nodes are ten `[RowHeader]` nodes now, and
`bug2014080.pdf` — which states no `/Scope` at all — comes back as a corner cell described as
scoped to both, a header row, and five rows each beginning with a `[RowHeader]`, which is the
assumption working end to end.

**Date.** 2026-08-13.
**ADR.** [0300](../adr/0300-a-header-cells-axis-and-the-grid-that-decides-it.md).
**Touched.** `crates/pdf-model/src/structure.rs` (`HeaderScope`, `CellPlacement`, `TableGrid`,
`TableStack`, `Tree::cell_span`, `Tree::header_scope`, four tests),
`crates/pdf-model/examples/table_header_census.rs` (new), `crates/viewer-core/src/accessibility.rs`
(`AccessibilityNode::header_scope` and the grid kept down the walk),
`crates/viewer-core/tests/headless.rs` (a tagged-table fixture and its test),
`crates/viewer-confined/src/protocol{.rs,/panels.rs}` (the axis on the pipe, and a `TH` in the
round trip), `crates/viewer-accessibility/src/{role.rs,tree.rs}` and `tests/tree.rs`,
`doc/conformance/ledger.toml` (§14.8, §14.8.4.8.3, §14.8.5, §14.8.5.4.3, §14.8.5.7),
`doc/todo/31-accessibility-host.md`, `doc/todo/README.md`, `doc/verify.md`,
`doc/adr/0300-*`, this file.

## The ledger, and the sweep that came with it

§14.8.5.7 was **`inapplicable`** on the reason "[n]othing here is drawn". That is a *rendering*
argument refusing a requirement addressed to a reader — and §14.8.5.6 next door had already stopped
being inapplicable for exactly that, because a `PrintField` attribute is not drawn either and it is
what a screen reader says. Two rows about one family, disagreeing, the older one's reason naming a
capability: `doc/todo/01`'s seventh sweep, run over §14.8.5 because that is where the round landed.

It paid twice. **§14.8.5.4.3 is `silent` now**, and it is this tree's only `silent` row: ten of
Table 379's thirteen attributes describe a layout process, and `/BBox`, `/Width` and `/Height` do
not — `AccessibilityNode::quads` is built from the text layer, so a `Figure` or a cell holding an
image crosses to an assistive technology with no place at all and nothing says so. The population is
unmeasured; `doc/todo/31` carries the measurement and the work. **A rise in `silent` from zero to
one is a new report rather than a regression** (trap 5).

## What the measurement found that it was not taken for

The A/B was to price the change and it is inside the instrument's spread — 67–91 ms with, 77–89 ms
without, best of five, three runs each, `Query::AccessibilityTree` in release on ISO 32000-2. **The
number itself is the finding**: ADR 0228 recorded this query at 0.13–0.25 ms on a five-page
document, and on a thousand-page one it is eighty milliseconds, because
`viewer_core::accessibility::nodes` walks the whole document's structure tree and prunes afterwards,
resolving §14.7.3's role map per element. A screen reader asks it on every page turn. Written down
in `doc/todo/31`, not taken.

## The gates

The whole of `doc/todo/02` §2 ran after the last edit. Nothing moved: 1659 tests pass, the corpus
gate's incomplete list is unchanged, the oracle's verdicts are unchanged (68 contradicted, 786
ambiguous, 18 no render), both text gates are unchanged, quorra is unchanged, and the conformance
checker passes with the new quotations verified against `doc/md/`. Two of them had to be trimmed to
fit the *conversion* rather than the standard — Table 384's "Row, Column , or B oth" and
"WritingMode :" are artefacts of the Markdown, checked against `pdftotext` on the PDF, which is
`doc/HANDOVER.md`'s standing caveat about `doc/md/` paying for itself again.

Both new tests were checked by deleting the code they guard: with the grid's spill loop removed,
`a_spanning_cell_moves_the_next_rows_first_child_off_the_first_column` and
`a_header_cell_crosses_with_the_axis_it_describes` both fail, the second with `Some(Row)` where the
clause says `Some(Both)`.

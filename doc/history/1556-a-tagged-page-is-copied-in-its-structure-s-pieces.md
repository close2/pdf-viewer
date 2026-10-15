# 1556 — A tagged page is copied in its structure's pieces, its artifacts stay out, and a replacement keeps the break before it

Slot 5 of batch eighty-eight, 2026-10-14/15, a clause round. ADR 1948; no question.

**The premise did not hold in its first half.** The copy was already in logical order (ADR 0444),
but only where the tree reached every byte, so one `/Artifact` running head sent a select-all back
to page order, and the ordered pieces were joined with nothing between them. **Nor did the
hypothesis**: the copy walks `Tree::walk` itself, not `viewer-accessibility`'s reading.

**(a)–(c) Built.** `logical_range` now answers `LogicalPiece`s, each with the `Boundary` the tree
puts before it, decided by §14.8.4.1's categories. §14.8.3.3 says why a block ends a line: "BLSEs
shall be stacked within a reference area in block-progression order". `TH` and `TD` end with a cell,
`Link`, `Annot`, `Form` and unknown types end with nothing, and `Sub` ends a line. These are
choices, written down in ADR 1948. Declared artifacts and `Artifact` elements are accounted for and
give no piece. Artifacts by absence are not, because they are this reader's inference.
`select::logical_copy` writes a line break and a tab. Over the 78 tagged first pages, a select-all
now copies whole in logical order on **58**, against **49** before.

**(d) Built, and it was a defect rather than a guard.** `/ActualText` cut away the separator inferred
before its first glyph, so `the` ran into a replaced `first`. §14.9.4's EXAMPLE, run with its `'`
for the first time, read back as `Druc\nker`. `cut_replaced` keeps the separator, and a replacement
that removes a line-end hyphen withholds the next inferred line break, so it now reads `Drucker`.

**Rows read anew, statuses unchanged** (`implemented`): §14.8.2.2.1, §14.8.2.5, §14.8.2.5.1,
§14.8.4.5, §14.8.4.6, §14.8.4.7, §14.8.4.8.3 and §14.9.4. The brief's §14.8.4.7 for tables is
§14.8.4.8.3 in this edition. **Unfinished**: two pages copy in page order because an annotation the
tree claims by §14.7.5.3 reaches the readback without its object. Lines inside a block stay the
readback's (ADR 1948 §4).

**Files.** `pdf-model/src/{structure,content/text,content/run}.rs`, `pdf-model/tests/{accessibility,
logical_order,marked_content_scope}.rs`, `viewer-core/src/{select,viewer,open,query}.rs`,
`viewer-core/tests/{headless,tagged_copy}.rs`, the ledger's eight rows, ADR 1948, this record.

**Gates.** rustfmt: 0 diffs. clippy `-D warnings`: `viewer-core --all-targets` exit 0, `pdf-model
--lib --tests --bins --benches` exit 0 (a sibling's untracked `r1552_probe.rs` example fails).
nextest, both crates: 2446 passed, 23 skipped. `cargo test -p conformance`: exit 0. `t3-text_extract`: exit 0 (4 passed).
`t3-selection`: exit 0. `t3-accessibility`: exit 101 on one ratchet, "untagged pages answering
honestly, whole population", 893 against 892: the untagged `doc/pima7667-2001.pdf` joined the
owner's `doc/` at 22:12 (trap 43), and every other ratchet has 0 slack. `state.sh docs`: exit 0. The drive: `a` then `c` in `quorra`, `quorra-gtk` and
`quorra-qt` copies 46 characters of the fixture and 1980 of `issue9972-1.pdf` in logical order.

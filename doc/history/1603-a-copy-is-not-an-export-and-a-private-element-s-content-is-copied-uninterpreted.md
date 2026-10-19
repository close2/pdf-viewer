# 1603 — A copy is not an export, and a `Private` element's content is copied uninterpreted

Slot 5 of batch ninety-five, 2026-10-19, a clause round. ADR 2042; 2043 is unused; no question.

**Premise and hypothesis.** The premise held: `Tree::logical_range` read every type below a
`Private` element and copied its text (on the fixture, `C1\tD1\nKept.`). Its part about ISO 32000-2
did not: that edition has no `Private` row, and its Table 364 is the document-level types. The
hypothesis — a copy reproduces content in another application, so it is the export ISO 32000-1
Table 333 forbids — did not hold: §14.8.1's NOTE, in both editions, lists "[e]xtraction of text and
graphics for pasting into other applications" and "[c]onversion to other common file formats" as
two operations, Table 333's phrase is the second, and ISO TS 32005 Table 2 keeps consuming and
exporting apart and makes ignoring the subtree a permission. The sentence's other half, that the
element and its descendants shall not be interpreted, binds the copy, which reads types to place its
breaks; *interpreted* is the type's meaning and not the element's place (the same table's
`NonStruct`, not interpreted, still has its descendants processed).

**Built.** In `logical_range` an element that is `Private` or below one is read as an element of no
type: no block or cell ends inside the subtree and no element there is an `Artifact`; the readback's
separators divide it, and its content is copied where the traversal reaches it, as the page order's
copy already had it. The fixture now copies `C1 D1\nKept.`. `select::logical_copy` says what it is
given. The accessibility tree still withholds the subtree (ADR 2028), for the reason ADR 2042 §3 states.

**Rows.** §14.8.4 and §14.8.2.5 `implemented` → `implemented`, notes and tests extended; ISO 32000-1
Table 333's "Neither the Private element nor any of its descendants shall be interpreted or exported
to other document formats."

**Gates.** rustfmt clean on every file of mine (`structure.rs` checked as HEAD plus my hunk);
`RUSTFLAGS=-D warnings cargo clippy -p pdf-model -p viewer-core --all-targets`: exit 0; their nextest:
2 519 passed, 23 skipped; `cargo test -p conformance --no-fail-fast`: exit 0. Planted: HEAD's walk
fails the new test (`C1\tD1`). In the export (HEAD, `select.rs`, `tagged_copy.rs`, the 28-line
`structure.rs` patch): six arms 968, 968, 968, 965, 965, 965 pages, 0 digests differing from HEAD's
HEAD's arms, 0 pages in one only (the comparison names a planted digest);
`t3-selection` exit 0; `t3-accessibility` exit 0, `Private` elements 0; `t3-text_extract` under one `--clock` hold: exit 0, 4 passed. The
drive has no copy step to run (`grep` finds no clipboard or `Copied` in `tools/drive-windows.sh`);
the copy is driven through `Command::Copy` headless in `tagged_copy.rs`.

**Unfinished.** None of the slot. `structure.rs` is slot 7's: my hunks are the `Open` field, the
element arm and one doc section of `logical_range`. Duration: 5 100 s.

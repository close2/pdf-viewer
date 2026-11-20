# Round 2137 — an offset has an origin on the way out: audit 2136's groups A, B, D and E

A fix round in the `audit` worktree for review 2136 (§7.5): B01, B02, B04, B05, T01, D01, D02.
Group C (B03, the free-entry memory budget) needs a design and is untouched; its test stays in
the manifest.

## What changed

- Group B (ADR 3249): `XrefTable` records the header its chain was read from and whether the
  newest section is a table; `Document::offset_origin` exposes the first. The update writer states
  every new entry and its `startxref` from that origin, so a file with bytes before its header
  reads back through each update's own section and can be saved again (B02). An offset past ten
  digits (table) or four bytes (stream) is refused as `UpdateError::OffsetTooLarge` (B05). The
  form follows the newest section rather than the merged trailer's `/Type`, and the doc says so
  and names the caller's identifier for the generation (D02).
- Group A (ADR 3250): `expand_object_stream` parses each member over the bytes up to the next
  member's offset (B01); `load`'s `nth(index)` fallback is gone, so an entry whose stream holds no
  such member reads as null (D01).
- Group D (ADR 3251): `serialize`'s cross-reference stream sizes `/W`'s third field to the largest
  member index, two bytes for every stream of 65 536 members or fewer (B04).
- Group E: a test pins a hybrid section's own free entry before its own `/XRefStm` (T01), and
  `incremental_update.rs` saves a prefixed file twice in both forms.

## Ledger and manifest

Rows 7.5.2, 7.5.6, 7.5.7, 7.5.8.3 and 7.5.8.4 stay `implemented`, list the new tests and state
the construction. 7.5.4 and 7.5.8.2 stay `implemented`: B03 is a memory budget (principle 3)
against conforming files, not a misreading of either clause, and is left open by name. The
manifest loses B01, B02 and B04, 4 entries to 1. 2136's `findings.json` marks seven fixed.

**Gates.** Over `-p pdf-syntax -p pdf-transform -p pdf-model`: fmt exit 0 after one `cargo fmt`;
clippy `-D warnings --all-targets` exit 0; `pdf-sandbox --bins` exit 0; nextest `--lib --bins
--tests` 4567 run, 4566 passed, 1 failed (B03, the manifest's one entry: difference 0 and 0);
`cargo test -p pdf-syntax` 411 passed, 1 failed (B03); rustdoc exit 0; fuzz `check --bins` exit
0; `cargo test -p conformance --no-fail-fast` 478 passed after this paragraph was added.

Left: B03 (group C); whether `linearize.rs`'s own `/W [1 4 2]` streams can pass 65 536 members.

**Habit.** A writer that appends counts from the origin the reader counted from, carried from
the reader rather than found a second time.

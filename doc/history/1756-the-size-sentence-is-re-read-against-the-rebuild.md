# 1756 — The `/Size` sentence is re-read against the rebuild

Session 1756, slot 5 of batch 115, a clause round; 2026-10-26. ADR 2346; ADR 2347 unused; no `Q`.
Every `partial` row is blocked outside this machine, so this round worked a `departed` row.

**Premise.** Not held as stated. §7.5.8 is cross-reference streams and reconstructs nothing. The
rebuild is `xref::rebuild`, licensed by §C.4, an informative annex, and reached when a table is
empty or names no catalog. `doc/todo/00` has no entry for the row.

**Hypothesis.** Not held. Tested on an export of HEAD with Table 15's sentence obeyed on each section
as read, keeping free entries, and with every repair HEAD has. 65 of the 66 documents are rebuilt,
and their first-page display lists and reports are identical, because the rebuild restores every
object the sentence ignored. `issue15716.pdf` loses page one: its catalog and page tree root are
below `/Size 8` and its page is object 10. So the sentence would take effect only on the one
document whose table is otherwise sound. Finding the ignored objects by their own headers (ADR 0789)
would keep that page, but it would also cancel the sentence on every document (ADR 2346 §4).
Instructions to page one over the 66 went from 5 238 767 to 100 975 486, because each rebuild scans
the whole file. None of the six `t2-launch_path` documents states an entry above `/Size`. The
change was taken back, and `pdf-syntax` is unchanged.

**Found.** Five writers number a new object past `object_numbers().max()`, so any table that drops
entries owes them the highest number its sections stated (ADR 2346 §5).

**Rows.** §7.5.5 stays `departed`. Its note's opening, its measurement paragraph and its "departure
stands" sentence are rewritten to the re-reading, and the 66 are now counted by a command rather
than quoted from ADR 0048. §7.5.8 is untouched and `doc/todo/65` is unchanged, since no status moved.

**Files.** New: ADR 2346, this record, `crates/pdf-model/examples/size_census.rs` (the census, kept
as the command that counts the price). Edited: `doc/conformance/ledger.toml` (the §7.5.5 note). I
worked in `scratchpad/r1756/`.

**Gates.** `rustfmt --check` on the new file: 0. `cargo fmt --check -p pdf-model`: 1, on a
sibling's `image/unpack.rs`, not mine. `-D warnings clippy -p pdf-model --all-targets`: 0. `nextest
-p pdf-model`: 2 202 of 2 202. `cargo test -p conformance --no-fail-fast`: 0. An earlier run had two
reds: my census header quoted before citing a clause, now fixed, and a sibling's `round_numbers`.
`tools/state.sh --round 1756 docs` was stopped before its grant, exit 143, since `cargo doc`
documents no example. Not owed, because no reader code changed: the six arms, `t2-launch_path`
under `--clock`, and the fuzz `cargo check`. Census and digest compare my two exports with each
other, not with HEAD's six arms. Duration 3415 s, about 75 tool uses.

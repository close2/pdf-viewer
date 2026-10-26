# 1764 — A new object is numbered past what the sections stated

Session 1764, slot 5 of batch 116, a writer round; 2026-10-26. ADR 2362; ADR 2363 unused; no `Q`.

**Premise.** Held. HEAD's four incremental writers took the larger of `object_numbers().max() + 1`
and `/Size`. `archive/prepare.rs`'s `Spare` took `object_numbers().max() + 1` and skipped numbers
that resolve. `object_numbers` leaves out free and declared-and-unread entries, and a rebuild
drops the table it replaces.

**Hypothesis.** Held. `XrefTable::next_object_number` in `pdf-syntax` counts every filed number
(free ones included), the end of every unread range and what a replaced table stated
(`keep_stated`, at both rebuild sites), and takes `/Size` where that is larger. All five sites call
it. Two planted files in `tests/update.rs` failed on HEAD and pass now: an unread object 3 under
`/Size 3` was overwritten, and a freed 3 was reused at generation 0. `next_number_census` over
3 910 documents moves the incremental writers' number for 3 (`outline_goto_action.pdf`, veraPDF
`6-1-4-t02-pass-a` and `-b`), and none of the three numbers resolves. The conversion's number
moves for 346 names, because `Spare` never asked `/Size`. HEAD's and this tree's `archive` ran on
those 361 files, to 2b and to 4, with every loss authorised: 722 of 722 pairs were byte-identical,
and 259 of them wrote an added output intent, so a spare number was taken.

**Not covered.** A section this reader cannot read at all states nothing to it (ADR 2362 §2).
**Mistake.** `tools/batch.sh gates` is the merge's whole run, not a list: it emptied the shared
gates log and ran four gates before I stopped it at 101 s (exit 143).
**Rows.** §7.5.4 and §7.5.6 stay `implemented`, each note gaining a sentence and its `test` list the
new tests; `doc/todo/65` is unchanged.
**Files.** New: ADR 2362, this record, `pdf-syntax`'s `examples/next_number_census.rs`. Edited:
`pdf-syntax` `src/xref.rs`, `src/document.rs`, `tests/cross_references.rs`; `pdf-model` `src/view.rs`,
`src/view/writing.rs`; `pdf-transform` `src/attachments.rs`, `src/update.rs`, `src/archive/prepare.rs`,
`tests/update.rs`; `doc/conformance/ledger.toml` (`-U0` hunks). I worked in `scratchpad/r1764/`.

**Gates.** `cargo fmt --check` and `-D warnings clippy --all-targets` on pdf-syntax, pdf-model and
pdf-transform: 0 each. `nextest -p pdf-syntax`: 296 of 296. `nextest -p pdf-model -p
pdf-transform`: 2 893 of 2 894; the red, `a_list_boxs_selected_options_are_highlighted_behind_their_text`,
passes in my export and is a sibling's. My export (HEAD plus this change): `writer_corpus`,
`split_corpus`, `merge_corpus`, `pages_corpus`, `optimize_corpus`, `archive_corpus`,
`save_round_trip` and `pdf-vfs` `write_corpus` 0 each; `pdf-transform --test gate` under `--clock`
0 (load 4.45, no holders of the render node); fuzz `cargo check` 0; rustdoc `-D warnings` on the
three crates 0. `tools/state.sh docs` 101, from a sibling's `render-cpu` link. Against HEAD's six
arms, the cpu-1x arm matches 968 of 968 page digests; the read path gains one field no drawing
code reads. `cargo test -p conformance --no-fail-fast`: 101, its one red 1765's record. Duration 7300 s, about 112 tool uses.

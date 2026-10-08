# 1432 — `getAnnots` answers the markup, and a script writes a reader's edits

Slot 1 of batch sixty-eight, 2026-10-08, a script round. ADR 1700; no row moved, no question.

**Premise.** Half held. `surface.rs` listed `getAnnot` and `getAnnots` unbridged and ADR 1689 ranks
them first. The brief's "writes the edit log already holds" held for `contents` (a free text
annotation's retyping) and for `hidden` as a display (§12.6.4.11's override set), which no save
wrote. `popupOpen` is not in the view state at all: a window's state is `viewer-core`'s
`Open::popups`, a person's clicks for the session. Both are now kept beside the edit log and saved.

**Built.** Every page's markup annotations (Adobe's seventeen types) cross to the realm at its first
telling, re-told when an override changes; `getAnnots` (page order, the reference's sorts, only
`ANFB_ShouldNone`), `getAnnot` by `/NM`, a no-op `syncAnnotScan`, and an `Annotation` of ten
properties. `hidden` writes Table 167's bit 2, `popupOpen` Table 186's `/Open` (Table 175's on a text
note with no popup), free-text `contents` the retyping; every other write is refused by name. Wire
version 8 is shared with slot 2's ADR 1702 (edit tag 10 is mine).

**Found on the corpus.** 27 files name `getAnnot`: 17 carry Adobe's attachments shim, which skips
it at viewer version 7 or later; 3 are LaTeX `cooltooltips` documents. Their links state no `/P`, so
`this.pageNum` at their events was page one and `getAnnot` answered `null`. `page_of` now takes the
page whose `/Annots` lists the annotation; all seven events then ran with no report.

**Cost.** One reading per realm, with a timing patch applied and reversed over the Tier 1 column:
149 realms, median 62 µs, mean 182 µs, largest 5.86 ms (755 pages).

**Unfinished.** No window is drawn for a popup-less text note, so its `popupOpen` is saved but not
seen. A person's later click wins over a script's later write (`viewer-core`'s map). `contents` on
the other sixteen types, and the six filters, are refused by name.

**Gates.** rustfmt `--check` on my files: clean. `RUSTFLAGS="-D warnings" cargo clippy -p pdf-model
-p pdf-script -p pdf-script-worker --all-targets` (engine): exit 0. `cargo nextest run` on the three:
2 133 run, 2 132 passed, 1 failed (`end_to_end::the_engines_library_runs_inside_the_filter`, at load
13.5), then 25/25 three times alone. `cargo check --manifest-path fuzz/Cargo.toml`: exit 0. `cargo
test -p conformance`: exit 0, 412 passed. Behind the lock, `--tree 6`: the Tier 1 column exit 0
(21 101 runs, 11 778 finished, 9 315 threw, 8 unparsed, 0 over; HEAD's 11 776 and 9 317; by
member the two are ADR 1689's `app.setInterval` runs, slot 2's, not checked run by run); the worker
column exit 0 (149 workers, 0 lost, 0 `SIGSYS`); `pdf-model --test script_corpus` exit 0 (356 held,
0 moved); `pdf-model --test corpus` exit 0; `save_round_trip` exit 0. A planted missing save call
failed the save test.

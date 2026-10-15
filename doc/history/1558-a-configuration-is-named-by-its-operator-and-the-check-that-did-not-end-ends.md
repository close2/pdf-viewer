# 1558 — A configuration is named by its operator, and the check that did not end ends

Slot 1 of batch eighty-nine, 2026-10-15, an archive round. ADRs 1952, 1953; no Q.

**The premise held.** The census re-ranked `doc/todo/72` as the brief did: at 2b,
`optional-content/configuration-names` 45 documents, `no-transfer-function` 27, `stream-filters`
13. All 45 fail by stating no `/Name`; 26 of the 27 state only `/TR /Identity`; 14 of the 26
failing streams name a Table 92 abbreviation (`A85`, `AHx`, `Fl`). **The check's hypothesis held**:
three `gdb` samples on Isartor's `6-1-12-t01-fail-a.pdf` (10 000 pages in one `/Kids`) were in
`Pages::get` inside `0..len()` loops — *n*²/2 node reads per pass, 269 s per target in `release`
at HEAD. Not batch 88's: its one `pdf-archive` change touches no page loop.

**Built.** `pdf-model`: `Pages::every()`, `(0..len()).map(get)` from one walk, taken only where
every trusted `/Count` matches the pages beneath it and the visit bound holds, else the lookups
(ADR 1953); every page loop in `pdf-archive` and the archive verb uses it — 269 s to 1.5 s per
target. `pdf-transform`: the configuration name is a `supply` (`name-template`, `{n}` numbered past
the file's own names; the shipped `as-if-printed`, `as-shown`, `keep-everything` say
`"Configuration {n}"`), not a table default, by `A48`'s line; a shared name stays refused. A Table
92 abbreviation is written in full, its bytes kept (`Stated`); any other name refused. `/TR
/Identity` is removed where no state in the file names another function (`Stated`); a real one is
refused, and ADR 1952 section 3 decides it per profile (as-if-printed `discard` with the function
in `xmpMM:History`, as-shown stop) without building it.

**Pixels.** Page one of `issue2761.pdf` (TR), `S2.pdf` and `Test-plusminus.pdf` (names), converted
to 2b under `as-shown` and drawn at 72 dpi before and after: identical, and S2 looked at. No filter
witness converts (each is refused elsewhere); the fixture's decoded bytes are compared instead.

**Gates.** rustfmt clean on every file touched. `clippy --all-targets` on pdf-model, pdf-archive,
pdf-transform: no warning in any file of this round; `-D warnings` itself stops at siblings'
uncommitted `pdf-model` lints (`annotation.rs`, `media.rs`, `geospatial/`). `nextest` on the three:
2 884 run, 2 883 passed, 1 failed — `a_requirement_this_converter_cannot_meet…`, whose `/TR
/Identity` premise this round answered; moved to a real function, `--test archive` 192 of 192.
`cargo test -p conformance`, with this record in place: 436 passed, 0 failed.
`tools/state.sh docs`: exit 0. One `--clock` hold, 247 s: `t3-archive_corpus`
exit 0 (2b: 479 converted, 131 refused, as at HEAD), `t3-foreign_corpus` exit 0 (0 panicked),
`t3-archive-val` exit 0, the census over `doc/veraPDF-corpus` exit 0 — 2 908 of 2 908 documents,
six targets, 17 s — and over pdf.js and corpora exit 0, 107 s: 36 sites unbuilt at 2b (39 before).
`--remedy-sites --to 2b`: 36 of 74 sites not built.

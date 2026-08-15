# 545 — The ninth sweep becomes a program, and the table it had been reading short

**Finding.** A sweep round under `doc/todo/01`'s binding rule. The table-number sweep — "does the
table a sentence cites state the key it attributes to it?", a description since the
four-hundred-and-eighty-ninth and rebuilt by hand on every run — is `conformance --bin tables` now,
the tenth of the fifteen to be a program. **Its first run found eleven defects and nine of them are
in this project's documents.** Six of those nine are a round's own correction that stopped at the
code: the five-hundred-and-thirty-seventh corrected `/Encoding` and `/ShadingType` in
`damaged_stream_census.rs` and left both numbers in ADR 0366; the four-hundred-and-eighty-ninth
corrected a widget's `/H` in §12.5.6.19's row and left "Table 192's `/H`" in ADR 0123 twice; ADR
0216 corrected `/Ascent` and `/Descent` in nine comments and rows and left Table 122 standing in five
documents. **The sharpest is ADR 0245**, which found "Table 189's `/R`" for a widget's rotation,
wrote down that `viewer-gtk`, ADR 0244 and `doc/todo/37` all carried it, corrected two of the three
— and left the ADR it had just named. A number a round retires in the code goes on living in the
document the code came from.

**Second finding, and it is about the instrument every sweep since the four-hundred-and-sixtieth has
shared.** `entries::tables_in` filed the **first block** of a table the conversion splits across a
page break, so Table 31 stated six of its twenty-eight entries and Table 125 none of its three
lengths; and `key_of` stripped a trailing `1`, `2` or `3` as a footnote marker, so `/BG2`,
`/Length1`, `/FontFile2`, `/DW2` and `/UR3` were keys of nothing — and no table in the standard
carries a numeric footnote in its first column, checked over all 305 keyed tables. The first run
printed 382 suspects; the two fixes took it to 84. **The fifteenth sweep shares the parser and its
population moves with it**: 640 stated entries to 756, a reading list of 112 to 174. It had been
reading 116 of the standard's entries as though they did not exist.

**Third, from the blame band.** §10.4.2.3 was `partial` for grey-to-CMYK "which has no caller:
nothing here converts *to* CMYK" — and `ColourSpace::to_cmyk` has converted a grey to ink since the
four-hundred-and-twenty-sixth, which **§10.4.2.4's own row says in as many words**. Two rows about
one mechanism, one corrected and the other left standing. The conclusion holds and the reason had
expired: what has no caller is this clause's *arithmetic*.

**Date.** 2026-08-15.
**ADR.** [0380](../adr/0380-the-ninth-sweep-becomes-a-program-and-the-table-it-had-been-reading-short.md).

**Sweep results, verbatim from the runs.**

- `tables` (new): 409 tables captioned, 305 stating entries; 4680 sentences name a table; 1830
  attributed key citations — 1706 the table agrees with, 75 absent, 3 a denial the table
  contradicts, 46 under a table that states no entries, 0 under no such table. **11 defects.** The
  three denial hits are all one shape the program cannot see past — a negation attached to another
  noun ("Table 31 makes a page stating no `/Contents` an empty page").
- `entries`: **756** table entries stated over 244 rows, 174 reported over 46 rows — 41 named
  nowhere, 133 only elsewhere, 39 not named by the row's own note. Up from 640/112/29/83/34, and the
  whole delta is the parser fix. Known populations; nothing worked.
- `pointers`: 4799 path pointers — 2653 live, 92 absent, 14 in another crate, 1668 unrooted, 118 a
  form, 254 not carried; 52 symbol pointers, 12 undefined. 0 defects.
- `blockers`: ledger 21 — 6 expired, 10 holding, 5 naming no clause; source 27 — 10, 10, 7. **The
  same ten numbers as the five-hundred-and-thirty-seventh.** 0 defects.
- `capabilities`: ledger 51 — 36 witnessed, 44 program, 7 crate; source 150 — 123, 83, 67. 0 defects.
- `unread`: 62 rows claim, 173 keys; 49 confirmed, 124 quoted over 52 rows, 54 by the row's own code.
  0 defects.
- `quotations`: 3445 quotations in 553 documents, 1611 verbatim, 24 diverging; 1393 in 794 ledger
  notes, 1089 verbatim, 1 diverging. 0 defects.
- `callers`: 122 names no crate under `crates/` asks, 177 named by a dependent crate, 80 only inside
  `pdf-model`, 21 only by a test or an example. 0 defects.
- `retired`, over the wave's twelve nouns (`RasterCache`, `RASTER_BUDGET`, `device_program`,
  `ShadingProgram`, `FunctionPaints`, `encode_threads`, `Stale`, `MustFollow`, `capture_presented`,
  `approximated`, `find_startxref`, `quoted_spans`): 299 mentions, 2 carrying both shapes, 0
  defects — 196 of the 299 are `Stale`.
- Arithmetic (6): §7.9.2 and §O, read and kept before. Clean. Inapplicable (7): 47 of 80 name source
  vocabulary on a session-local stop-list; none wrong, and exactly one row changed status since the
  five-hundred-and-thirty-seventh (§8.9.5.4, `partial` → `implemented`), checked with `git diff` over
  the status lines. Parent counts (10): 25 counted claims against a family, 10 matching neither the
  children nor the descendants, 0 defects — all of them counts about the rows *beside* the clause.
  Sweep 14: 9 hits under a session-local vocabulary, every one naming its debt in other words.
  Errata (12): "151 struck passage(s) of 4 words or more that doc/md/ still carries as current text"
  over all fourteen PDFs, unchanged, and 71 quotations quoting struck text.

**Rows corrected in this commit.** §10.4.2.3 (the expired reason, and what has no caller instead).
Kept with evidence recorded: §9.9.1, §10.4, §11.3.6, §12.5.6.17, §12.5.6.18, §12.5.6.20, §12.5.6.22,
§7.6.5, §12.8.1, §12.8.2.2.2 — ten of the eleven the band from commit 518 held.

**Source corrected.** `crates/pdf-model/examples/damaged_stream_census.rs` (Table 78 → **77** for
`/ShadingType`, the fifth in a file where the five-hundred-and-thirty-seventh corrected four),
`crates/pdf-model/src/collection.rs` (Table 159 → **153** for `/Folders`, twice).

**Documents corrected.** ADR 0366 (Tables 122 → 119, 78 → 77), ADR 0123 (192 → 191, twice), ADR 0244
(189 → 192), ADRs 0032, 0118, 0211, 0323 and `doc/ui-boundary.md` (122 → 120 for `/Ascent`,
`/Descent`, `/FontName`), ADR 0045 (122 → **115** for `/DW2`), ADR 0200 (158 → 162 for a thread's
`/I`). ADRs 0216 and 0245 carry a note saying which documents their own corrections missed.

**Code.** `tools/conformance/src/tables.rs` and `src/bin/tables.rs` (new, twelve unit tests);
`tools/conformance/src/entries.rs` (a caption's blocks are one table; a digit is not a footnote
marker; one new test); `tools/conformance/src/lib.rs` (the module).

**Touched.** `doc/conformance/ledger.toml`, `doc/todo/01-ledger-partial-rows.md` (the run's record,
the band pointer advanced to §12.8.3.4.3 inside commit 534, ten commands now),
`doc/todo/02-every-round.md` §4 (the new command's line), `doc/ledger-and-claims.md` (nine programs
→ ten), `doc/adr/0380-*` (new), this file.

**Gates.** `cargo fmt --all --check` clean; `cargo clippy --workspace --all-targets` silent of
lints; `cargo nextest run --workspace` **2025 tests run: 2025 passed, 15 skipped**;
`cargo test --workspace --doc` green; `cargo test -p conformance -- --nocapture` 5 passed. No corpus
or oracle run: every correction this round made is a comment, a ledger note or a document, and
`git diff` over `crates/` shows no line outside a comment.

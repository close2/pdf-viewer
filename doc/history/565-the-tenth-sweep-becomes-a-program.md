# 565 — The tenth sweep becomes a program, and the number an erratum moved

**Finding.** A sweep round under `doc/todo/01`'s binding rule. The tenth sweep — a parent row's stated
count of its family against the rows below it — is `conformance --bin counts` now, the thirteenth of
the fifteen to be a program and **the last of the ten whose printed level moved with the session**:
ten hand-runs gave 16, 185, 124, 10, 160, 17, 70, 25, 41 and 4 counted claims over a ledger whose
families barely move, because each round wrote the pattern that morning. `doc/todo/01` predicted this
one "needs no discriminator at all — the count is in the sentence and the family is in the file", and
**the first half was wrong in the way session 562's prediction was**: the family is in the file, and
*which numbers are about it* is the whole problem. Following ADR 0397's lesson, both halves come from
sweeps that already exist rather than from a new heuristic — **the ninth's attribution** for the
population (a cardinal is a claim only where it governs one of the ledger's own words for a row, and
the family is the clause the sentence *names*) and **the sixth's arithmetic** for the answer (every
cardinality the family's own rows can produce, which makes this ledger's two counting conventions
derived rather than remembered: a count without the `General` row, and a count with the clause's own
row *in* the family).

**Second finding: the errata a hand-filter cannot see either.** Session 562 read `emit`'s 1097
annotations by hand and wrote down the three words to filter for; a filter written down is a filter
somebody re-invents, so it is `spec-errata moved` now — every annotation whose instruction uses *move*,
*renumber*, *delete* or *insert* **and names a clause number**, with the ledger rows, source citations
and document mentions standing on that number beside it. **Two structural errata were unrecorded.**
Issue #477 moves all of §12.3.6 down a level, which is #452's shape and was missed because this
collection writes an instruction in the past passive ("was moved and demoted") as well as in the
imperative. Issue #256 changes no number at all and says §12.6.4.8's `/Base` text "applies to all
relative URIs in a PDF document and is not limited to only URI actions as is currently implied" — a
statement about scope, whose reading this tree takes deliberately: `uri::resolve` is already general
and the other relative-URI site, §7.11.2.2's URL-based file specification, is validated and not
resolved because nothing here fetches a URL.

**And the question session 562 left: what does this tree do about a clause number the errata have
moved?** Recorded, not renamed, and findable by command. The published numbers stay because the gate
enforces it — §12.3.6's new note was written with a section sign in front of the amended number and
**failed `the_ledgers_own_prose_names_clauses_and_tables_that_exist`**, so it is written as the erratum
writes it, *subclause 12.3.5.3*. The row is where the amendment is recorded, because a row is what a
clause-reading round opens; the command is what makes it findable, because a note in one row is not read
by the round that writes the twentieth citation in `crates/`.

**Third, from the blame band, and it is one lesson twice.** §7.8.3 was `partial` because `/Properties`
"is read only for `/OC`" — `content/marked.rs`'s `property_list` resolves a `BDC` operand's name
through that subdictionary for §14.9's four entries, `/MCID` and §14.13.5's associated files, and
§14.6.2's own row has said so all along; with `/ProcSet` answered by §14.2's `inapplicable` it is
**`implemented`**. §8.4.5 listed five of Table 57's entries as unread and `content/ext_gstate.rs` reads
all five: `/SM` is the smoothness tolerance §10.7.3's *own* `implemented` row has claimed since session
74, and `/BG`, `/BG2`, `/UCR`, `/UCR2` set `black_generation_stated`, which is what makes §11.7.2's
`DeviceCMYK` group space a reported departure. **Both rows are the fifth failure shape with the right
answer one clause away, and neither is visible to any of the fifteen sweeps** — a key a row says is
unread is the second sweep's subject, and the second sweep asks whether a *source quotes* it, so a key
the tree reads under the row's own `code` array lands in the one-short-key noise it prints every run.

**Date.** 2026-08-17.
**ADR.** [0400](../adr/0400-the-tenth-sweep-becomes-a-program-and-the-number-an-erratum-moved.md).

**Sweep results, verbatim from the runs.**

- `counts` (new): **170 clause(s) have a row below them; 5184 sentence(s) govern one of the ledger's own
  words for a row; 296 attributed count(s) — 125 the family agrees with, 45 it can be counted no such
  way, 126 attributed to a clause with no rows below it; 3 place(s) count one family twice.** 0 defects
  in the ledger, and the instrument's own evidence is that its four known findings (§11.7's double
  count, §14.8.2's twelve over thirteen, §12.6.4's and §12.7.6's) all come back as `[correction]` hits
  on the numbers their rounds retired.
- `spec-errata moved` (new): **15 of 2865 annotations move, renumber, insert or delete a numbered
  clause; 2 defects** (Issue #477, Issue #256), with #452, #196 and #133 confirming the instrument
  against findings already read. The noise is a NOTE renumbered rather than a clause.
- `blockers`: ledger 22 — 6 expired, 10 holding, 6 naming no clause; source 28 — 10, 10, 8. **Identical
  to session 562's ten numbers.** 0 defects.
- `capabilities`: ledger 53 — 38 witnessed, 46 program, 7 crate; source 156 — 128, 90, 66. 0 defects;
  the three sentences more than last time are this round's own modules.
- `unread`: 64 rows claim, 175 keys; 48 confirmed, 127 quoted over 54 rows, 55 by the row's own code —
  identical. 0 defects.
- `entries`: 249 rows explain themselves by an arrival and name code, 2 name none; 788 entries stated,
  193 reported over 47 rows — 56 named nowhere, 137 only elsewhere, 55 not named by the row's own note.
  Known populations.
- `quotations`: 3799 quotations in 595 documents, 1679 verbatim, 25 diverging; 1435 in 794 ledger notes,
  1119 verbatim, 1 diverging (§8.4.4's known correction). 0 defects.
- `callers`: 122 names no crate under `crates/` asks, 177 named by a dependent crate. The delta is
  clean.
- `pointers`: 5317 path pointers — 2939 live, 103 absent, 14 in another crate, 1858 unrooted, 127 a
  form, 276 not carried; 57 symbol pointers, 12 undefined. 0 defects.
- `tables`: 409 tables captioned, 305 stating entries; 4794 sentences name a table; 1888 attributed key
  citations — 1742 the table agrees with, 93 absent, 4 a denial the table contradicts, 49 under a table
  that states no entries, 0 under no such table. **0 defects.**
- `inapplicable`: 80 rows stating 310 terms — 60 named by no source, 250 named over 72 rows, 235
  carrying a cousin. None wrong.
- `owed`: 225 `partial` rows stating 2983 terms — 159 named by no source over 105 rows, leaving 120 rows
  on the reading list. Identical to its first run.
- `retired`, over the wave's ten nouns (`recover_compressed_objects`, `CompressedRecovery`,
  `object_streams`, `signed_area`, `wound_counter_clockwise`, `Path::reversed`, `owed`,
  `Digest::Shake256`, `Role::Document`, `supports_text_ranges`): **452 mentions, 2 carrying both shapes,
  0 defects** — `owed` is the ordinary-English-word warning for the fifth wave running, a sweep named
  after the debt it measures.
- Arithmetic (6): §7.9.2 and §O, read and kept before. Clean, and still the only sweep of the fifteen
  that has never printed anything else. Errata (12), `check`: "151 struck passage(s) of 4 words or more
  that doc/md/ still carries as current text" over all fourteen PDFs, unchanged, and **71 quotations
  quoting struck text — the same 71 as sessions 545, 553 and 562**.

**Rows corrected in this commit.** §7.8.3 (**`implemented`**, `/Properties` read for §14.6.2's property
lists in general, with one named test per requirement group), §8.4.5 (five of Table 57's entries off the
not-read list, `partial` now for `/FL` alone), §12.3.6 (Issue #477's renumbering and the standing answer
to it), §12.6.4.8 (Issue #256's scope statement and the reading taken). Kept with evidence recorded:
§12.5.1, §12.5.6.19, §12.7, §12.7.4.1, §12.7.5.3, §12.7.5.4, §12.7.6, §14.11.3, §8.5.3.3.1, §11.7.4,
§12.5.6.9, §8.11.4.3, §8.10.2 — thirteen of the fifteen the band from commit 553 to 564 held.

**Code.** `tools/conformance/src/counts.rs` and `src/bin/counts.rs` (new, ten unit tests);
`tools/conformance/src/lib.rs` (the module); `tools/spec-errata/src/lib.rs` (`STRUCTURAL`, `Structural`,
`Ground`, `structural`, `clauses_named`, `standing_on`, two unit tests) and `src/main.rs` (the `moved`
subcommand).

**Touched.** `doc/conformance/ledger.toml`, `doc/errata-read.md` (the filter as a command, the two
errata, and what this tree does about a moved number), `doc/todo/01-ledger-partial-rows.md` (the run,
the band, thirteen commands now, and the prediction about this sweep's discriminator corrected),
`doc/todo/02-every-round.md` §4 (the two new commands), `doc/ledger-and-claims.md` (twelve programs →
thirteen), `doc/adr/0400-*` (new), this file.

**Gates.** `cargo fmt --all --check` clean; `cargo clippy --workspace --all-targets` silent of lints
(the `viewer-qt@0.1.0:` lines are gcc's on a cold build, which `doc/todo/02` §2 names — and three of
mine were real and are fixed: two `arithmetic_side_effects` on a `BTreeMap` counter and one
`type_complexity` on the contradiction key, now the `Counting` alias). `cargo nextest run --workspace`
**2100 tests run: 2100 passed, 15 skipped**, against 2088 before — the twelve are the new sweep's ten
unit tests and `spec-errata`'s two. `cargo test --workspace --doc` green. `cargo test -p conformance --
--nocapture` **157 passed and 5 passed** — and it **failed twice on purpose first**, once on a bare
section sign in `spec-errata`'s new doc comments and once on §12.3.6's note citing the post-erratum
number, which is the citation gate doing exactly what the decision above rests on. No corpus or oracle
run, and the reason is stronger than usual: `git diff --stat -- crates/` is **empty**, so no line this
round wrote can reach a raster.

# 562 — The fourteenth sweep becomes a program, and the errata a `check` cannot see

**Finding.** A sweep round under `doc/todo/01`'s binding rule. The fourteenth sweep — a `partial`
row whose note names nothing owed — is `conformance --bin owed` now, the twelfth of the fifteen to
be a program and the last of the four whose printed *level* moved with the session: nine hand-runs
gave 16, 24, 5, 15, 19, 10, 8, 9 and 19 hits over rounds that moved almost no rows, each under a
debt vocabulary written that morning. **`doc/todo/01` named the remedy and the obvious reading of it
is wrong.** A vocabulary *learned* from the ledger — words likelier in an owing note than in a
settled one — measures **topic** rather than debt: the top terms come out `digest`, `stored`,
`appearance`, `knockout`, `signature`, because signatures and annotations are mostly `partial` and
lexing and filters are mostly `implemented`. At every threshold tried, every one of the 228 notes
holds some word of any lexicon wide enough to be useful, so the hit count is zero and the instrument
says nothing. What the seventh sweep actually supplies is its **measurement**: a note that names a
debt names a *thing*, and a thing this tree does not have is a name no source carries. So the
discriminator is `inapplicable`'s with the sign reversed — there a term the tree *names* under a row
claiming absence, here a term the tree *lacks* under a row claiming a debt — sharing its extraction
verbatim, and the reading list is every `partial` row whose vocabulary the tree names in full,
ordered by its rarest term's reach so that a row naming nothing specific heads it. **Its first
finding is at the top of that order**: §12.7.6.1, `partial` over a clause that is three bullets and
no `shall`.

**Second finding, the round's other hazard: `spec-errata check` asks one direction of one question.**
It compares *quotations this tree has written*, so an erratum over text nobody has quoted is
invisible — which is how ISO/TS 32001 §5.1.3's deletion stood until session 555 went looking. Run
`emit` over all fourteen PDFs instead and read it against what the ledger **claims**: 1097
annotations over the three documents that carry any, twenty of them structural, and **two of the
three renumbering errata were unrecorded** — Issue #452 moves §14.7.5.1.1 up a heading level and
renumbers the rest of that family, under five ledger rows and some twenty source citations, and
Issue #196 inserts a new §7.6.5.3 and pushes the existing one down. The third is #133, read in
session 437, which is the instrument agreeing with a known finding. **And one erratum changes a
requirement rather than a number**: Issue #22 raises Table 166's `/AP` to "Required except for
conditions listed below (PDF 2.0)" — and the 2020 text already bound a writer in prose, against which
`view.rs` said in two places that "an annotation with no `/AP` is legal". The one place this program
departs from that `shall` is `write_retypings`, and it is argued in place and reported rather than
tidied away.

**Third, from the blame band, and it is one row telling the truth about itself.** §14.11 said
"§14.11.2 is `partial` for §14.11.2.2's guidelines" — false since session 442 moved §14.11.2 and
§14.11.2.1 to `implemented`, a hundred and twenty sessions — and the row's own last sentence, written
by session 402 about the half of it that was wrong *then*, is "[a] parent row is not maintained by
the sessions that correct its children". Beside it, §11.6.4.1 was `partial` for `/AIS`, which Table
57 states and this clause never mentions — a row owing a **neighbour's** debt, which is §E.1's and
§I's shape one family over — and §12.7.2 was `partial` above a note naming nothing owed.

**Date.** 2026-08-17.
**ADR.** [0397](../adr/0397-the-fourteenth-sweep-becomes-a-program-and-the-errata-a-check-cannot-see.md).

**Sweep results, verbatim from the runs.**

- `owed` (new): **225 `partial` rows stating 2983 terms — 159 named by no source, a debt named in a
  word, over 105 rows, leaving 120 rows whose every stated term this tree already names.** 1 defect
  (§12.7.6.1). Half the population lands on the list, most of them rows naming a debt in prose with
  no identifier in it, which is why it is a reading list and not a gate.
- `spec-errata emit` over all fourteen PDFs: 1097 annotations over the three documents that carry
  any; 20 structural; **2 defects** (Issue #452, Issue #196), plus Issue #22's requirement change,
  and Issues #133 and #236 confirming the instrument against findings already read.
- `blockers`: ledger 22 — 6 expired, 10 holding, 6 naming no clause; source 28 — 10, 10, 8. 0
  defects.
- `capabilities`: ledger 53 — 38 witnessed, 46 program, 7 crate; source 153 — 126, 87, 66. 0 defects.
- `unread`: 64 rows claim, 175 keys; 48 confirmed, 127 quoted over 54 rows, 55 by the row's own code.
  0 defects.
- `entries`: 249 rows explain themselves by an arrival and name code, 2 name none; 788 entries
  stated, 193 reported over 47 rows — 56 named nowhere, 137 only elsewhere, 55 not named by the row's
  own note. Known populations.
- `quotations`: 3755 quotations in 590 documents, 1662 verbatim, 25 diverging; 1428 in 794 ledger
  notes, 1112 verbatim, 1 diverging. 0 defects.
- `callers`: 122 names no crate under `crates/` asks, 177 named by a dependent crate. The delta is
  clean.
- `pointers`: 5276 path pointers — 2920 live, 100 absent, 14 in another crate, 1845 unrooted, 123 a
  form, 274 not carried; 54 symbol pointers, 12 undefined. 0 defects.
- `tables`: 409 tables captioned, 305 stating entries; 4778 sentences name a table; 1879 attributed
  key citations — 1735 the table agrees with, 91 absent, 4 a denial the table contradicts, 49 under a
  table that states no entries, 0 under no such table. **0 defects**; the absent are up from 89
  because this round wrote its own record of what it corrected.
- `inapplicable`: 80 rows stating 310 terms — 60 named by no source, 250 named over 72 rows, 235
  carrying a cousin. None wrong.
- `retired`, over the wave's fourteen nouns (`Presenter`, `detach_presenter`, `Layer`, `PresentCost`,
  `QuorraWindowRenderer`, `render_retained`, `Digest::ALL`, `TRIED_WHEN_UNSTATED`, `Shake256`,
  `substituted_media_box`, `ccitt_rows`, `Tree::child`, `supports_text_ranges`, `select::continues`):
  **913 mentions, 8 carrying both shapes, 0 defects** — 768 of the 913 are `Layer` and `Presenter`.
- Arithmetic (6): §7.9.2 and §O, read and kept before. Clean. Parent counts (10): 4 counted claims
  against a family under a session-local pattern, 0 defects — **the last sweep whose level is still
  session-local**, named in `doc/todo/01` as the next a program should take over. Errata (12),
  `check`: "151 struck passage(s) of 4 words or more that doc/md/ still carries as current text"
  over all fourteen PDFs, unchanged, and 71 quotations quoting struck text — the same 71 as sessions
  545 and 553.

**Rows corrected in this commit.** §12.7.6.1 (**`implemented`**, a clause that is three bullets and
no `shall`), §11.6.4.1 (**`implemented`**, `partial` for a neighbour's debt), §12.7.2
(**`implemented`**, a note naming nothing owed), §14.11 (its account of §14.11.2, stale for a hundred
and twenty sessions), §12.5.2 (Table 166's `/AP` writer requirement, Issue #22, and the departure),
§14.7.5.1.1 (Issue #452's renumbering), §7.6.5.2 (Issue #196's insertion), §7.2.3 (a percentage split
in half by an append — the §Q shape session 553 found). Kept with evidence recorded: §11.3.7.2,
§11.6.4, §11.6.4.4, §11.4, §11.4.3, §11.4.8, §12.3 — seven of the ten the band from commit 541 to
546 held.

**Source corrected.** `crates/pdf-model/src/view.rs` (two doc comments saying an annotation with no
`/AP` is legal, which Table 166 contradicts; the departure argued and named in the second).

**Code.** `tools/conformance/src/owed.rs` and `src/bin/owed.rs` (new, six unit tests);
`tools/conformance/src/inapplicable.rs` (`Kind: Ord`, for the reach cache the wider population
needs); `tools/conformance/src/lib.rs` (the module);
`crates/pdf-model/src/action.rs` (`each_of_the_three_form_action_types_reaches_its_own_answer`,
§12.7.6.1's evidence).

**Touched.** `doc/conformance/ledger.toml`, `doc/errata-read.md` (the structural errata `check`
cannot see, and what each turned out to be), `doc/todo/01-ledger-partial-rows.md` (the run's record,
the band pointer advanced to commit 553 with §12.5.1, twelve commands now, sweep 10 named as the next
a program should take over, and the learned-vocabulary attempt recorded so nobody builds it twice),
`doc/todo/02-every-round.md` §4 (the new command's line, and `emit` beside `check`),
`doc/ledger-and-claims.md` (eleven programs → twelve), `doc/adr/0397-*` (new), this file.

**Gates.** `cargo fmt --all --check` clean; `cargo clippy --workspace --all-targets` silent of lints
(the `viewer-qt@0.1.0:` lines are gcc's on a cold build, which `doc/todo/02` §2 names) — **which is
also this round's second hazard answered**: sessions 555 and 558 each fixed one inherited
`clippy::doc_markdown` in `page_geometry.rs` differently while both reported the lint run silent, and
this tree is rebased onto `main`, so a lint surviving there would have printed here. None did.
`cargo nextest run --workspace` **2076 tests run: 2076 passed, 15 skipped**, against 2069 before — the
seven are the new sweep's six unit tests and §12.7.6.1's evidence; `cargo test --workspace --doc`
green; `cargo test -p conformance -- --nocapture` 5 passed. No corpus or oracle run: every change
under `crates/` is a doc comment or a test, and `git diff` shows no executable line, so nothing this
round did can reach a raster.

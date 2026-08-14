# 525 — The fifth sweep becomes a program, and the mark a save never took off

**Finding.** A sweep round under `doc/todo/01`'s binding rule. The caller sweep — "the model
implements this, who calls it?", every `pub fn` in `pdf-model` against the crates that could ask
it, run by hand since the two-hundred-and-fifty-third session — is `conformance --bin callers` now.
It is the sweep whose *number* is the finding (the four-hundred-and-eighth's result was that a
whole new host program took **zero** names off the list), and every run had used a script written
that session, so the levels were never comparable: 246/85 against the round before's 246/86 at the
same commit, 101/82 against 92/77 with the population unchanged at 286. **Its first run found two
`pub fn`s nothing in the tree names, and the first of them was a defect a person could see.**
`ViewState::additions`'s own doc comment named the caller it did not have — "what a host asks to
know whether there is anything to save" — while `viewer_core::Open::dirty` answered that question
from `cursor > 0`, the length of the undo log, which a save does not shorten. So a document saved
and left open went on saying it had unsaved work: `Event::Dirty` never came back false and
`viewer-ui`'s title kept its mark for the rest of the session. What is unsaved is the cursor's
distance from the last save, and `Open::saved_at` is that distance.

**Second finding, from the blame band.** §I.2 was `partial` for a writer-side sentence above a note
of its own saying §7.5.6's incremental update meets it — the note corrected and the status not —
and three rows were `partial` because of it: §I because its child was, §E.1 because "§I's row … is
`partial` for the version half", §E as the aggregate of E.1 and E.2. Four statuses moved in one
commit. **No sweep in `doc/todo/01` looks for a row whose reason is a neighbour's *status*.**

**Date.** 2026-08-14.
**ADR.** [0360](../adr/0360-the-fifth-sweep-becomes-a-program-and-the-mark-a-save-never-took-off.md).

**Sweep results, verbatim from the runs.**

- `callers` (new): 289 distinct `pub fn` names in `pdf-model`, 15 crates naming it in a manifest —
  174 named by a dependent crate, 19 by a tool or a fuzz target, 73 only inside `pdf-model`, 21
  only by a test or an example, **2 by nothing at all**; 115 that no crate under `crates/` asks.
  **2 defects** (`ViewState::additions`, and `Namespace::is_standard` named rather than built).
- `blockers`: ledger 21 — 6 expired, 10 holding, 5 naming no clause; source 26 — 9, 10, 7. 0 defects.
- `capabilities`: ledger 47 — 33 witnessed, 40 program, 7 crate; source 144 — 118, 78, 66. 0 defects.
- `unread`: 63 rows claim, 172 keys; 50 confirmed, 122 quoted over 52 rows, 54 by the row's own
  code. 0 defects — the three keys that moved off "confirmed" since the five-hundred-and-tenth's
  and -seventeenth's identical numbers are the ledger moving (23 notes rewritten in the eight
  rounds between), not the tree; `/Interpolate`'s three rows are corrections narrating it as
  formerly unread.
- `entries`: 236 rows in the population, 113 entries over 45 rows — 31 named nowhere, 82 only
  elsewhere, 36 not named by the row's own note. Known populations; nothing worked.
- `quotations`: 3223 quotations in 502 documents, 1495 verbatim, 23 diverging, 0 defects.
- `retired`, over this wave's fourteen nouns: 164 mentions, 4 nouns carrying both shapes, 0 defects.
- Arithmetic (6): §7.9.2 and §O, clean. Inapplicable (7): 61 of 80 name source vocabulary on a
  session-local stop-list; none wrong, and no row in that population changed since 517 (checked
  with `git diff` over the status lines). Citations (8): 6 mentions of 3 dead paths, 0 defects.
  Table numbers (9): 1088 citations checked, 69 suspects, **1 defect** (`view.rs`'s "Table 177's
  `/AP`"). Parent counts (10): 160 counted claims, 35 of them checkable "aggregate of the N below"
  arithmetic, 0 defects. Ledger quotation marks (11): 1248 spans (session-local normaliser), 866
  verbatim, 47 diverging, 0 defects. Sweep 14: 10 hits (session-local vocabulary), §E corrected
  below. Errata (12): "151 struck passage(s) of 4 words or more that doc/md/ still carries as
  current text" over all fourteen PDFs, unchanged.

**Rows corrected in this commit.** §I.2 (**`implemented`**, the writer sentence it already said was
met, the two others disposed of, the clause's own `shall` on a processor named), §I, §E.1, §E (all
three **`implemented`**, each having been `partial` for a neighbour's status), §12.6.3 (the count of
performed action types, "eleven" against §12.6.4's eight and §12.6's ten). Kept with evidence
recorded: §12.6.4.4, §8.9.5.1, §7.9, §12.5.6.7, §8.11.4.4, §8.11.4.5, §12.5.6.21, §14.11.6.2,
§14.12.4, §12.4.3.

**Source corrected.** `crates/pdf-model/src/view.rs` (Table 177's `/AP` → Table 166's, the same
sentence having the right attribution eight words later; `ViewState::additions`'s claim about a
caller it does not have), `crates/pdf-model/src/structure.rs` (`Namespace::is_standard`'s absent
caller named against `doc/todo/48`), `doc/todo/48` (the predicate its second owed item needs).

**Code.** `tools/conformance/src/callers.rs` and `bin/callers.rs` (new, twelve unit tests);
`crates/viewer-core/src/open.rs` (`saved_at`, `dirty` and `saved`), `crates/viewer-core/src/viewer.rs`
(the save says the document is clean), `crates/viewer-core/tests/headless.rs`
(`a_save_takes_the_unsaved_mark_off_and_an_edit_puts_it_back`).

**Touched.** `doc/conformance/ledger.toml`, `doc/todo/01-ledger-partial-rows.md` (the run's record,
the band pointer advanced to commit 511, seven commands now), `doc/todo/02-every-round.md` §4 (the
new command's line), `doc/todo/48-the-specification-we-check-against.md`,
`doc/ledger-and-claims.md` (six programs → seven), `doc/adr/0360-*` (new), this file.

**Gates.** `cargo fmt --all --check` clean; `cargo clippy --workspace --all-targets` silent;
`cargo nextest run --workspace` all green; `cargo test --workspace --doc` green;
`cargo test -p conformance -- --nocapture` green. No corpus or oracle run: nothing this round
changed reaches a raster — the one behavioural change is an `Event::Dirty` a host draws its title
mark from, and neither gate opens a window or saves a file.

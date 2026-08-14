# 517 — The fourth sweep becomes a program, and the two ADRs a correction walked past

**Finding.** A sweep round under `doc/todo/01`'s binding rule. The retired-claim sweep — the
string a correction retired, grepped over every other row since the two-hundred-and-sixteenth
session and over `doc/adr/` since the four-hundred-and-twenty-ninth — is `conformance --bin
retired` now. It is the one sweep whose population cannot be derived, because what was retired is
what the last rounds decided, so its nouns are arguments; what a program adds over the grep is the
*order*, classifying each mention as a **correction** (a retirement narrated in place) or a
**standing claim**, because a noun carrying both is where every finding this sweep has ever had
lives. Its first run took seventeen nouns from sessions 511–516 and both of its defects were in
ADRs: ADR 0235's consequence still read "`RadiosInUnison` crosses and is not obeyed" six rounds
after session 511 read that sentence out of §12.7.5.2.3 — and **that session's own record names
ADR 0235 as the fourth place carrying the claim**, having corrected the other three — and ADR
0337 still filed `freetext_no_appearance.pdf` under `doc/todo/21`'s per-character fallback, which
ADR 0348 read out while correcting the todo files and the ledger row. The eighth sweep paid beside
it, on a `doc/todo/37` cited in `viewer-ffi/src/form.rs` a hundred sessions after the file was
deleted.

**Second finding, from the blame band.** §12.3.2.2 was `partial` for `Target::Number`, which is
read and resolved; what nothing had read is the parenthesis in the same paragraph as the bounding
box every `/FitB` uses — "[i]f any side of the bounding box lies outside the page's crop box, the
corresponding side of the crop box shall be used instead". `interpret` puts the displayed box at
the *origin* rather than clipping to it, so a `/FitB` on a page with ink off its edge magnified to
fit ink this viewer never draws. And §14.8.2.1 said the map between page content order and logical
content order "is what remains" while §14.8.2.5's row had named `Tree::logical_range` as that map
since session 413 — two rows, one mechanism, disagreeing.

**Date.** 2026-08-14.
**ADR.** [0352](../adr/0352-the-fourth-sweep-becomes-a-program-and-the-noun-two-corrections-missed.md).

**Sweep results, verbatim from the runs.**

- `retired` (new): 17 nouns, 544 mentions, 7 nouns carrying both shapes. **2 defects** (ADR 0235,
  ADR 0337). Noise shape it taught itself: a noun that is also an ordinary English word — `prefix`
  262 mentions, `joining` 36, no finding between them. Rule it taught itself: a sentence
  containing Markdown's `~~` is a retirement, because this project strikes the retired
  sentence and writes the correction in the next one. Re-run after the corrections and that
  rule, over the same nouns and this round's own record: 575 mentions, still 7 both-shapes.
- `blockers`: ledger 20 — 6 expired, 9 holding, 5 naming no clause; source 26 — 9, 10, 7. 0 defects.
- `capabilities`: ledger 47 — 33 witnessed, 40 program, 7 crate; source 141 — 115, 77, 64. 0 defects.
- `unread`: 63 rows claim, 172 keys; 53 confirmed, 119 quoted over 51 rows, 54 by the row's own
  code — the five-hundred-and-tenth's five numbers exactly. 0 defects.
- `entries`: 236 rows in the population, 113 entries over 45 rows — 32 named nowhere, 81 only
  elsewhere, 37 not named by the row's own note. One worked: §12.8.2.3's `/Msg`.
- `quotations`: 3123 quotations in 483 documents, 1462 verbatim, 23 diverging, 0 defects.
- Caller (5): 286 distinct `pub fn` names in `pdf-model`, the same population as 510; 101 unnamed
  by the eight hosts and 82 by nothing under a session-local extraction, whose level is not
  comparable across runs. Arithmetic (6): §7.9.2 and §O, clean. Inapplicable (7): 43 of 80 name
  source vocabulary on a session-local stop-list; none wrong. Citations (8): 5 hits, **1 defect**
  (`viewer-ffi/src/form.rs`'s `doc/todo/37`). Table numbers (9): 1104 citations checked, 80
  suspects, 0 defects. Parent counts (10): 17 counted claims, 0 defects. Ledger quotation marks
  (11): 1243 spans (session-local normaliser), 719 verbatim, 30 diverging, 0 defects. Sweep 14: 19
  hits (session-local vocabulary), §12.3.2 corrected below. Errata (12): "151 struck passage(s) of
  4 words or more that doc/md/ still carries as current text" over all fourteen PDFs, unchanged.

**Rows corrected in this commit.** §14.8.2.1 (the logical-order map, **`implemented`**),
§12.3.2.2 (the crop-box bound read and applied; the wait named against §12.6.4.3 rather than
against `Target::Number`), §12.3.2 (its one unsettled child named), §12.8.2.3 (`/Msg` named and
disposed of), §12.7.6.4 (a precision: what is owed is XFDF, which the clause names). Kept with
evidence recorded: §7.6.4.3.2, §8.11, §9.8.2, §12.2, §12.3.4, §12.5.6.5, §12.7.3, and §12.7.6.4
beside its precision.

**Documents corrected.** `doc/adr/0235` (the `RadiosInUnison` consequence, struck and re-argued),
`doc/adr/0337` (the per-character filing, struck and re-argued), `crates/viewer-ffi/src/form.rs`
(the dead `doc/todo/37`).

**Code.** `tools/conformance/src/retired.rs` and `bin/retired.rs` (new, nine unit tests);
`crates/viewer-core/src/open.rs` (`content_box`, with §12.3.2.2's parenthesis quoted over it and a
unit test for the three cases).

**Touched.** `doc/conformance/ledger.toml`, `doc/todo/01-ledger-partial-rows.md` (the run's record,
the band pointer advanced, six commands now), `doc/todo/02-every-round.md` §4 (the new command's
line), `doc/ledger-and-claims.md` (five programs → six), `doc/adr/0352-*` (new), this file.

**Gates.** `cargo fmt --all --check` clean; `cargo clippy --workspace --all-targets` silent;
`cargo nextest run --workspace` all green; `cargo test --workspace --doc` green;
`cargo test -p conformance -- --nocapture` green. The one change that reaches a person's screen is
a `/FitB`'s magnification, which no corpus or oracle page exercises — neither gate applies a
destination — so the raster gates were not owed by this round's edits.

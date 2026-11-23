# Round 2192 — the script walks are held to the bytes, and commit a choice by its selection

A round in the `gwg` worktree on record 2188's three leftovers. ADRs 3469–3472.

## Tier 1 independent of the load (ADR 3469)

- A budget counted in engine steps was not taken: no step count reaches `pdf-model` (`ScriptResult` has none, and a
  field added to it breaks every literal of it, the owner's audit tests among them), and `5712688.pdf`'s time is
  mostly outside the engine. Counting stopped documents apart alone was not enough: at the loads measured nothing
  was stopped, so the document would swing ~10 000 runs between the floor and the apart column with the load.
- Built: `ViewState::hold_sequences_to`, the view state's own sequence budget (the viewer keeps the second); both
  walks hold sequences to a minute; `ViewState::sequences_stopped` counts every stop, and the stop's sentence is
  kept past the 256-sentence cap (sixteen more at most); Tier 1 names a stopped document and holds the rest.
- `5712688.pdf` alone: 9 988 runs and 8 911 throws, nothing stopped, at 360 s, 400 s and 2 372 s (one processor
  shared with three busy loops). The held documents' Tier 1 walk, unpinned and pinned: 21 162 runs, 9 315 throws.

## `Entered::Chosen` (ADR 3470)
The synthetic commit is one file both walks include (`pdf-model/tests/support/synthetic_commit.rs`): a selection-only
choice field is entered as the options its value selects; one whose `/V` names no option is not entered (ADR 0248's
export-value reading stands). Of ADR 3453's ten, the five `FOP-2216` run again; `PDFBOX-923` (`NA`) and pdf.js
`issue15815`, `issue17998` (export values) do not. pdf.js-8048-0 took three more fields.

## Held-only mode (ADR 3471)
`PDFVIEWER_SCRIPT_CORPUS_HELD=1`: either walk reads only Tier 0's held documents, 43 s and about eight minutes built
(the whole walks take an hour and a half and more). In `doc/verify.md` and `doc/todo/02` section 2's map.

## The held files (ADR 3472)
Tier 0's whole walk in its update mode: six lines moved, all `untaken`, no digest, none added or removed. Tier 1:
both populations ran 21 162 times, threw 9 315, 8 unparsed, nothing stopped: each floor is 21 162. The whole walk
failed on one run stopped by `FIELD_EVENT`'s 100 ms clock alone, so the walk's runs now take six seconds (ADR 3469).

## Left, in order
1. The five fields whose `/V` names an export value or `NA` stay untaken; reading an export-value `/V` as its option
   is ADR 0248's question, not the walk's.
2. The train's whole Tier 1 walk confirms `WALK_RUN_BUDGET`; this round re-ran only the held documents under it.

**Gates.** Over `-p pdf-script -p pdf-model -p viewer-qt`: fmt 0; clippy `-D warnings --all-targets` 0, and with
`engine` 0; `pdf-sandbox --bins` 0; nextest 3036 passed, 0 failed, `audit-diff.py` "failed 0, expected 0"; rustdoc 0;
fuzz `check --bins` 0; conformance 0; walks: Tier 0 whole update 0, Tier 1 held 0 twice, Tier 1 whole 101 (one `Wall` run), Tier 1 held under the new clock 0 (21 162 runs, none over a budget).
Habit: a gate counting what a time budget let finish holds that budget out of the load's reach.

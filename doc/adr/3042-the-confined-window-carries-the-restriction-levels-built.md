# 3042 — The confined window carries the restriction levels: ADR 3021 §2 built

Session 2086. Status: **accepted, built.** Builds ADR 3021 §2, whose record stays as written ("accepted, not built").
Context: review 2038 (finding D59). Clauses: ISO 32000-2 §7.6.4.1 Table 22, §12.7.3; `CLAUDE.md` principle 3.

## Decision

`quorra-confined` takes `--restrictions=` and `--ignore-restrictions` as the other windows do (`viewer_host`'s
`Settings::take`). It sends its worker `Command::Restrict(RestrictionScope::Window(..))` before the document, with
the reader's other words. A worker started again after a crash is sent it again. Where neither word is typed, the
policy is `RestrictionPolicy::default()`: `off` for every operation, as before, which is the reader-favouring end.
Refusals and warnings are said on the terminal, as they already were. An operation at `ask` goes on the window's
question card (`Asked::Restricted`) with `viewer_host::asked`'s words. Enter proceeds, and Escape answers `no` with
`viewer_host::declined`'s sentence. Before this, the window answered every question `no` and printed
`viewer_host::unanswerable`.

## Consequences

- D59 is `fixed`. `quorra-confined`'s tests assert that the policy crosses as `Command::Restrict`, that the two words
  are carried rather than refused, and that an `Asking` event puts the card up and Enter answers it.
- No ISO clause requires the control. The obligation is principle 3's and ADR 1190's, so no ledger row moves.

# 1637 — The todo files are held by a ratchet

Session 1400. Status: **accepted**. Carries out ADR 1576 section 3, which it does not edit.
Context: ADRs 1023, 1535, 1547, 1576; `tools/conformance/tests/spelled_ordinals.rs`.

## 1. The shape

`the_todo_files_name_no_round` holds two counts over `doc/todo/*.md`. Each count has a figure
written in the test, the count may only fall to that figure, and once the figure is zero the
assertion is equality.

- `TODO_CEILING` counts lines that spell a round's ordinal in words. It is the same string the
  source gate holds at `== 0`.
- `TODO_NUMBERED_CEILING` counts the digit form: `session` or `round`, either case, singular or
  plural, at the start of a word and followed by a space and a digit. The source gate's
  `Session <digit>` shape misses this, because a todo file writes the word in lower case (ADR 1576
  section 3.3).

**The ratchet is `<=` above zero, and that departs from ADR 1535's `==` on purpose.** Six rounds
edit `doc/todo/` in one worktree at once. Under an `==` ceiling, a sibling that rewrites one line
fails tier 1 for all six until somebody writes the new figure down. `<=` cannot fail that way.
Lowering the figure is the merge's job, or the next round's: it is the one line to edit when the
printed count is below it. At zero the two assertions agree, and the figure cannot be lowered
further.

## 2. Where the figures stand, and what carries them

- **The spelled form is 2, both in `doc/todo/56`.** Round 1395 owned that file this batch, so it
  was left alone. Every other todo file is at 0: the sixteen that carried the other 63 of the 65
  lines named in the brief are rewritten under ADR 1576 section 1. Where an incident was the reason for a rule,
  the incident stays and only the ordinal goes.
- **The digit form is 119.** None of it is in the files this round rewrote, and none is in
  `doc/todo/02`. Those 119 lines sit in `56` and in sixteen files no spelled ordinal reached: 58, 57,
  38, 61, 62, 59, 21, 48, 63, 67, 03, 46, 49, 68, 23 and 60. Each is a later round's, file by file, and each
  rewrite lowers the figure.
- **`doc/todo/02`'s incidents** are the reasons for its rules, and they keep the incident without
  the number ("one merge found ten converter-fixture failures"). No exemption line was needed.

## Consequences

The figure moves only downward. A round that rewrites a todo file prints the new count and lowers
the figure in the same change.

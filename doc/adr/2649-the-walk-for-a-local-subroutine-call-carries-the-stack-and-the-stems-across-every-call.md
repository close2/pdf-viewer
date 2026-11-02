# 2649 — The walk for a local subroutine call carries the stack and the stems across every call, and stops at `endchar`

Session 1907, Slot 7 of batch 131, 2026-11-02.
Status: implemented.
Context: ADR 2628 (its Open section and decision 6's `SeacWalk`, which already carries both), the Font DICT
repair that asks which glyphs of an unreadable Font DICT call a local subroutine. Code:
`crates/pdf-font/src/cff.rs` (`calls_local_subr`, `LocalSubrWalk`, `Stopped`),
`crates/pdf-font/src/cff_stop_tests.rs`.

## Decision

1. **The operand stack and the stem count belong to the walk, not to a call.** Technical Note #5177
   section 4.7 keeps one argument stack across `callsubr` and `callgsubr` — what the caller pushed
   before the call and what the subroutine pushes "act according to the manner in which the subroutine
   is coded" — and section 4.3 sizes a `hintmask`'s data bytes by every stem hint the charstring
   declared. `calls_local_subr` started each called global subroutine at an empty stack and no stems
   and handed neither back, so a mask after stems declared in a subroutine was read as no bytes and
   its first byte as an operator: a `255` there swallowed the next four bytes as a fixed-point number,
   and a `callsubr` among them was missed. A miss is on the wrong side: the Font DICT repair counts
   such a glyph as not lost when its Private DICT is replaced by an empty one.
2. **`endchar` ends the walk wherever it is run** (section 4.2: the last operator of the outline;
   its Note 6 lets a charstring end in a call whose subroutine ends in `endchar`). The walk returned
   to the caller and went on reading bytes the interpreter never runs, which could count a `callsubr`
   nothing reaches.
3. **The bound is the one the walk had.** Every push and every call spends one of the glyph's
   `CHARSTRING_OPERATIONS`, shared across the calls, so the one stack holds at most that many
   operands, as the per-call stacks together did; `a_fan_out_of_five_at_ten_levels…` stays the
   test at the bound.

Rejected: folding this walk into `SeacWalk`. That walk answers another question with its own list of
operators it cannot follow, and the two would then change together for a reason only one of them has.

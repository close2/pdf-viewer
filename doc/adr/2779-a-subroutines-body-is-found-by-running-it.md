# 2779 — A subroutine's body is found by running it

Session 1984, a single archive round.
Status: accepted and built. In `pdf-font`: `cff.rs` (`subroutine_body`, `BodyWalk`, `BodyEnd`,
`inline_leading_call` passes the caller's operands).
Tests: `cff_stop_tests.rs`'s `a_mask_counts_the_stems_the_caller_declares` and
`a_width_behind_a_nested_call_and_a_mask_is_restated`; `a_subroutine_is_spliced_up_to_its_first_return`
updated to the new signature.
Context: ADR 2628 (a subroutine spliced up to its first `return`), ADR 2773 (the machine's face
embedded, its advances restated), `doc/pdfa-blocking-issues.md` R09 (a).
Clauses: ISO 19005-2 section 6.2.11.5, ISO 19005-4 section 6.2.10.5; Adobe Technical Note #5177
sections 3.1 (the width), 4.3 (hint masks) and 4.7 (subroutines), Appendix B (ten levels).

## Context

`pdf_font::restate` restates a CFF glyph's advance by splicing the subroutine call at the head of
its charstring until the width is in the charstring itself, then rewriting that operand (ADR 2628).
The splice needs the subroutine's body: the bytes before the `return` that ends it. Finding that
`return` means reading the subroutine as Type 2, because a `hintmask` or `cntrmask` is followed by
one data bit per stem hint declared so far and a mask byte may be 11. The reader counted only the
stems the subroutine itself declared, from zero, and declined a subroutine that called another
before a mask. URW's Nimbus faces factor a glyph's hints into a subroutine that takes its first
stems from a nested call (`-15 callgsubr hstemhm … hintmask`) or from the caller's operands, so
9 of NimbusSans-Regular's 855 glyphs and 12 of NimbusRoman-Bold's were refused, and with them the
machine's face of 25 documents at each part under `as-shown` (`MACHINE_FACE_NOT_RESTATABLE`).

## Decision

1. **The body is found by running the subroutine as the interpreter would**: operands pushed with
   their values, a call popping its number and running the subroutine it names (to its `return`,
   at its own level), stem hints and masks counting `stack / 2` stems and clearing, a mask stepping
   over `ceil(stems / 8)` data bytes. The run starts from the caller's operands before the call,
   so a first stem hint inside the subroutine counts them, as section 3.1's stack does.
2. **The first `return` at the subroutine's own level ends it**, as before (ADR 2628). An `endchar`
   there ends it just past the operator; an `endchar` inside a nested call ends the glyph, so the
   whole subroutine is the body and its later bytes are never run.
3. **What the walk cannot keep exactly is declined, by name, not guessed at**: an escaped operator
   other than the four flex operators (an arithmetic or storage operator changes the stack by a rule
   the walk does not follow), a call naming no subroutine, nesting past Appendix B's ten levels, a
   walk past `CHARSTRING_OPERATIONS`. The glyph is then refused as before.
4. Splicing in place of a second route. Restating the operand where the glyph pushed it, without
   splicing, covers a width the glyph wrote ahead of the call but not one a shared subroutine pushes
   (NimbusRoman-Bold's 12); the exact splice covers both with one rule, and every other byte of the
   glyph and every subroutine is copied.

## Consequences

Every glyph of the 35 URW faces on this VM is restatable (a probe of each face, every glyph, through
`restate::with_widths` and its read-back); before, 9 and 12 of two faces were not. The documents R09
(a) left on this refusal convert under `as-shown` and `no-data-loss` where it was their only one.
The transform's code-by-code proof (`machine_face_proved`) still holds each converted font to the
window's outline.

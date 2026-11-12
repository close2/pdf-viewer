# 2811 — `bitshift` shifts zeros in from either end of a 32-bit integer

Session 2000, the round that fixes the owner's specification audit (`doc/reviews/1983-specification-audit.md`).
Status: **accepted and built** in `pdf-colour` (`function.rs`: the `Operator::Bitshift` arm of the calculator, and
its unit test `a_bit_shift_moves_left_for_a_positive_count_and_shifts_zeros_in_going_right`).
It **supersedes ADR 0369 and ADR 0371 on one point**: the sign-preserving right shift, and ADR 0371's "the sign
repeated" past the register's width. Everything else those two decided stands.
Context: ADRs 0369, 0371; the audit's "Direct-reference and assertion follow-up".
Clauses: ISO 32000-2 §7.10.5.1, §7.10.5.2, Annex B.3 (Table 42's `bitshift` row); the PostScript Language Reference,
third edition (PLRM3), the `bitshift` operator and Appendix B's implementation limits.

## The point ADR 0369 decided

§B.3 says only "Perform bitwise shift of int 1 (positive is left)". ADR 0369 read the width of the integer as
unstated anywhere this project held, and chose the right shift that keeps the sign, because it is the one answer
that does not depend on a width: `-4 -1 bitshift` is `-2` at 32 bits and at 64.

## Why it does not stand

§7.10.5.1 gives the calculator's operators PostScript's meaning, and the audit retrieved PLRM3 itself (its provenance
and hash are in the audit's `source/PLRM3-source.json`). PLRM3 defines `bitshift` as shifting int1's binary
representation by shift bits, the bits shifted out lost and **the bits shifted in zero**, a negative shift being a
right shift; it warns that the result is arithmetically correct only for a positive int1. The warning is about what
the zero fill does to a negative value — it is the reason to expect a large positive answer — and permits no sign
extension. PLRM3's Appendix B states the integer as 32-bit two's complement, so the width ADR 0369 found unstated is
stated in the document the clause defers to.

## What is built

The right shift is the logical one on the value's 32 two's-complement bits, which is [`Value::Integer`]'s `i32`
already: `-8 -1 bitshift` is `2147483644`, `-1 -1 bitshift` is `2147483647`, and a shift of 32 bits or more in either
direction leaves `0`. The left shift is unchanged (`1 31 bitshift` is `-2147483648`, the sign bit set by the shift).
The audit's `type4_right_bitshift_inserts_zero_bits_for_negative_values` passes; the crate's own test that asserted
the sign-preserving answers now asserts these, and fails against the old arm.

## Cost

None measured: `examples/type4_operator_census` found no corpus program that reaches `bitshift` when ADR 0371 ran it.

# 3379 — A text-positioning operand, or a `"`'s spacing, that is not a number is reported

Session 2169. Status: accepted. Context: ADR 3355 (round 2163 left these); ADR 3335.

## The clauses

Table 106: `Td` and `TD` take `tx ty`, which "shall denote numbers"; `Tm`'s "operands shall all be
numbers"; `TD` "shall have the same effect as this code: - t y TL t x t y Td". Table 107: `"`'s
"a w and a c shall be numbers", and it "shall have the same effect as this code: a w Tw a c Tc
string '".

## Decision

An operand present and not a number is reported as `StreamSyntaxFault::TextOperandNotANumber`, a
sibling of ADR 3355's variant, because Tables 106 and 107 are not Table 103 and the sentence names
its table. What the operator's own stated equivalent can still do is done:

- `Td` and `Tm` move nothing — a position one coordinate short is not stated.
- `TD` sets the leading from a numeric `ty` whatever `tx` is, since its `-ty TL` needs only `ty`;
  this is a change, the old code skipped the leading too.
- `"` keeps the spacing whose operand is not a number, as `Tw` and `Tc` do under ADR 3355, sets the
  other, moves to the next line and shows its string, as before.

An operand absent is the loop's shortfall report and is not judged again.

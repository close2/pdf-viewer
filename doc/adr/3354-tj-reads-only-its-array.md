# 3354 — `TJ` and `d` read their array out of its brackets, and what stands outside is left over

Session 2163. Status: accepted. Context: audit 2161's finding B02; ADRs 0018, 3335.

## The clauses

Table 107, `TJ`: one operand, an array, and "[e]ach element of array shall be either a string or a
number". §8.4.3.6: a dash array's elements "shall be numbers". §7.8.2: "all of the operands needed by
an operator shall immediately precede that operator ... operands shall not be left over".

## What was wrong

The content lexer leaves an array flattened, its brackets as nulls. `d` split on them; `TJ` read every
pending operand as an element, so a number before the `[` moved the text, a string outside any array
was shown, and so was a string in a nested array.

## Decision

One reading, `FlatArray::last_of` in `content/run/operands.rs`, for both operators: the array is the
last bracketed run of the operands — the one nearest the operator, as §7.8.2 has it. What stands before
it, and for `TJ` after it, is left over: not read, and reported as `OperandOutsideItsArray`. `d`'s phase
is the operand nearest `d` after the array; more than one there is left over too. A `TJ` element that is
neither a string nor a number is not shown and is reported (`TextArrayElementOfTheWrongType`). A `TJ`
with operands and no array shows nothing and is reported the same way as a leftover.

A null is a bracket in this reading whatever wrote it, so a `null` element or a nested array ends the run
early and what precedes it is reported as left over rather than shown. Neither is an element either
clause admits, so the reading only decides *which* report a malformed array gets; telling a bracket from
`null` would need the lexer to rebuild arrays, which no valid stream needs.

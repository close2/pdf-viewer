# 3419 — A `Tj`, `'` or `"` whose string operand is not a string is reported

Session 2179. Status: accepted. Context: ADRs 3355, 3379; ISO 32000-2 Table 107.

## Decision

Table 107 gives `Tj`, `'` and `"` a `string` operand. One present and of another type is
reported as `StreamSyntaxFault::TextOperandNotAString`, a sibling of ADR 3379's
`TextOperandNotANumber` rather than that variant renamed, because a host that filters by name
keeps the two apart and the sentence names the type the table states. Nothing is shown for it,
as before; what the operator's stated equivalent does ahead of the showing is still done — `'`
is "T* string Tj", so the move to the next line is made, and `"` keeps ADR 3379's spacing rule.
An operand absent is the loop's shortfall report and is not judged again.

# 3641 — Operands left over are written out alone, and the operator keeps its own

Session 2235. Status: **accepted and built**; extends ADR 3558 and ADR 3631, and does what ADR
3631's "Not done" named. Context: `T02-05-01_008_Font-set-operator-missing.pdf` and `_012` (the
format corpus's hand-built set), whose `/F0 36. (Hello PDF-world!) Tj` gives `Tj` three operands
outside any text object; `issue6342.pdf` (pdf.js). Clauses: ISO 32000-2 §7.8.2; ISO 19005-2 and ISO 19005-4 section 5.1.

## Context

The rule the interpreter follows is §7.8.2's, not §8.2's: §8.2 and its Table 50 give each operator
its operands, and §7.8.2 says where they stand — "all of the operands needed by an operator shall
immediately precede that operator" — and that "operands shall not be left over when an operator
finishes execution". NOTE 2 there says PDF "has no concept of an operand stack", so what a reader
does with operands in front of an operator's own is not the standard's; this reader gives the
operator the last of them and drops the rest (`pdf_model::content::run`), and the grammar logs
`model/operands-left-over` (ADR 3629). ADR 3631 wrote such an operator out with all its operands
where the page drew the same, which takes the show off `T02-05-01_008`'s page, so the proof left
it and `as-shown` refused the file by the class's name.

## Decision

1. **Where the leftovers are an operator's only repair, only they are written out.**
   `as_drawn::Walk` keeps where each operand since the last operator began (at most 64, far below
   the interpreter's own bound of 65 536, so each one it keeps the reader keeps too); the place
   written as one white-space character runs from the first leftover to the last byte before the
   operator's own first operand. The operator keeps exactly the operands the reader ran it with.
2. **An operator that needs another repair as well is written out whole**, as ADR 3558 and 3631
   do, and left where that changes the page.
3. **The proof still decides** (ADR 3558): both files drawn, page by page, display list and text.
   By construction the page draws the same, and the proof is what says so.
4. **A salvaged run among the operands written out goes with them**; one among the operator's own
   stays a place of its own after them, so the places stay in order and apart.
5. The loss is `content-as-drawn`, as before.

## Why this is on the near side of the authoring line

Nothing is invented or moved: the bytes removed are operands no reader of this program ran.

## Consequences

`issue6342`'s leftovers are written out, and its refusal under `as-shown` is now section 6.2.2's
operator `c08.`. `T02-05-01_008` and `_012` do not convert: their `Tj` stands outside any `BT … ET`
as well (§9.4.1), a second class at the same operator, so it is written out whole, the proof finds
the text gone, and ADR 3558 leaves it — wrapping it in `BT … ET` would read the producer's intent.
They are photographed under `as-if-printed` as before.

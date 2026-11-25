# 3631 — Operands left over and an `EMC` closing nothing are written out where the page draws the same, and an unresolved property list is defined

Session 2232. Status: **accepted and built**; extends ADR 3558 and ADR 3615. Context:
`issue15629.pdf` (pdf.js), whose form closes one `BMC` with two `EMC`s, and `issue6342.pdf`,
whose form holds operands left over. Clauses: ISO 32000-2 §7.8.2, §14.6.1 Table 352, §14.6.2;
ISO 19005-2 and ISO 19005-4 section 5.1.

## Decision

1. **Two classes join `WRITTEN_OUT_WHERE_DRAWN_THE_SAME`**: `model/operands-left-over` and
   `model/marked-content-unbalanced`. An operator with leftovers is written out with all its
   operands, exactly as ADR 3558 writes the other classes; an `EMC` with no section open is
   written out alone. Each stands only where both files draw each page the same.
2. **A well-typed `BDC` naming a list nothing defines is not written out.** The checker's walk
   now logs `model/property-list-unresolved` there (ADR 3629), and ADR 2684's mechanical rewrite
   already defines such a name as a membership dictionary naming no group, which hides nothing,
   as the reader drew it, and loses nothing. So section 5.1 counts the class answered where that
   rewrite defines names, and the fence does not photograph a page for it. Writing the section
   out would be a loss where none is needed. A `BDC` with an operand of the wrong type is still
   written out with its section (ADR 3615).
3. **The output's own check holds every write**: a section left open at its stream's end, or a
   name whose entry exists but is no dictionary, is no place either rewrite takes; the output's
   check finds the class again and the conversion is refused, not wrong.
4. The loss for a written-out operator is `content-as-drawn`; the definition is lossless.

## Why this is on the near side of the authoring line

No mark is invented or moved; the proof is the window's own drawing of both files.

## Not done

Writing out only the leftover operands, keeping the operator, would answer more pages: the
interpreter drops exactly those. It needs `as_drawn` to keep each operand's position; until then a
leftover in front of an operator that draws is photographed under `as-if-printed` and refused by
the class's name under `as-shown`.

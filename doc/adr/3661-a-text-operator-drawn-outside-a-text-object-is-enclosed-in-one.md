# 3661 — A text operator this reader drew outside a text object is enclosed in one that starts where it drew

Session 2240. Status: **accepted and built**; extends ADR 3558 and ADR 3641, and replaces ADR 3558
item 2's answer for `model/text-operator-outside-a-text-object`. Context: ADR 3353 (what this reader
draws there), the owner's PDF/A stance (A694 item 3: where the input is bad, bake in the state the
viewer shows; elegant remedy first). Clauses: ISO 32000-2 §9.3.1, §9.4.1, Table 105, §9.4.2, §9.4.3;
ISO 19005-2 and ISO 19005-4 section 5.1.

## The clauses

§9.4.2 and §9.4.3: "The text-positioning operators shall only appear within text objects." and "The
text-showing operators shall only appear within text objects." Table 105: `BT` begins a text object
"initializing the text matrix, T m , and the text line matrix, T lm , to the identity matrix"; `ET`
ends one "discarding the text matrix". §9.4.1 names what a text object may hold: the text state,
text-positioning and text-showing operators, and "The other operators that may appear in a text object
are those related to the general graphics state, colour, and marked-content". §9.3.1: "The text state
operators may appear outside text objects" — so `Tf`, `Tc`, `Tz` and the rest outside are no fault,
and the grammar flags only `Td`, `TD`, `Tm`, `T*`, `Tj`, `TJ`, `'` and `"`.

## What this reader draws

ADR 3353's choice: such an operator is reported and acts on the matrices the reader holds outside a
text object — the identity at the start of a page's content and of each form, and after each `ET`,
which resets them; a stray `ET` acts on nothing; `q` and `Q` do not touch them (they are not graphics
state). The clause states no reading of the fault; this one is this reader's.

## Decision

1. **A text object is added around each run of them**, not the operators written out (ADR 3558 took
   the text off the page). A `BT` goes in front of the first operator's operands; an `ET` after the
   last such operator, before the first operator §9.4.1 keeps out of a text object (`q`, `Q`, `cm`,
   path construction and painting, `Do`, `sh`, an inline image, `d0`, `d1`, `BX`, `EX`, an operator no
   table defines), before a `BT`, and before `BMC`, `BDC` and `EMC`, so that no marked-content
   sequence crosses the added object's edge; and at the end of each stream. A producer's stray `ET`
   closes the added object itself (`T02-05-01_001`'s shape), and no second `ET` is written.
2. **The added `BT` starts where this reader drew.** The walk follows the matrices: identity;
   *stated* by a `Tm` the reader ran as written (its six numbers, respelled by `write::object`, which
   reads back the same value); *shown* past the line matrix by a glyph's width; or *moved* by a `Td`,
   `TD`, `T*`, `'` or `"` it does not compose. An operator that sets both matrices (`Tm`) needs
   nothing; one that reads the line matrix (`Td`, `TD`, `T*`, `'`, `"`) needs it identity or stated;
   `Tj` and `TJ` read the text matrix, which must equal the line matrix. Where it does not hold, no
   object is added and the class keeps its refusal, in a sentence of its own naming this ADR.
3. **ADR 3558's proof still decides**: both files drawn, page by page, display list and text. `BT`
   opens a reading scope and `ET` realises a clip and a knockout group, which an object this reader
   never ended did not; where that changes a page, the operators are left.
4. **The loss is `content-as-drawn`.** No byte of the producer's is removed, but which matrix a text
   operator outside a text object is drawn by is ADR 3353's reading and not a value the standard
   states, so the clause does not make it certain; the history names the count of objects added.

## Why this is on the near side of the authoring line

No mark is invented or moved: the producer's operators draw what the window drew, and the proof says
so. The `BT`, `ET` and `Tm` added state the reader's own state at that point, the producer's `Tm`
numbers where one is restated.

## Consequences

`T02-05-01_001`, `_003` and pdf.js's `issue10388_reduced` convert under `as-shown`, which needed ADR
3662 for the two T02 files; veraPDF 1.30.3 passes them. Not done: a `Td` from a stated matrix is not
composed, and a width is never measured, so a show after a show across a cut stays refused.

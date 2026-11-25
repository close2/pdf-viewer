# 3615 — A marked-content section whose opening operator is written out is written out whole

Session 2228. Status: **accepted and built**; extends ADR 3558. Context: `issue17679_2.pdf`
(pdf.js), refused under `as-shown` at section 5.1 though ADR 3558's evidence had it written out.
Clauses: ISO 32000-2 §7.8.2, §14.6.1 Table 352, §14.6.2; ISO 19005-2 and ISO 19005-4 section 5.1.

## Context

The document's form writes `123 /oc1 BDC … EMC` and `456 /oc7 BDC … EMC`: a tag that is not a name
("tag shall be a name object", Table 352) and property-list names its resources do not hold
(§14.6.2). This reader opens a section for each with what it was given and draws its marks; the
names govern nothing, so nothing is hidden. Two classes are logged at the form:
`model/operand-of-the-wrong-type` (the `BDC`) and `model/property-list-unresolved` (its name),
the second added to the log after ADR 3558 was built. ADR 3558's write took the `BDC` alone, so
(a) the `EMC` it left would balance nothing in the output, and (b) the property-list class was
no class a write answers, so section 5.1 kept its refusal whatever the write did.

## Decision

1. **A `BMC` or `BDC` written out takes its balancing `EMC` with it.** `as_drawn`'s walk keeps a
   stack of the open sections (at most 1024 deep, past which a section is not followed and its
   operator left) and writes out an `EMC` exactly where the operator that opened its section was.
   The proof is unchanged: the pages are drawn from both files and the write stands only where
   each page's display list and text are equal.
2. **`model/property-list-unresolved` is answered where the write took its `BDC`**: it joins
   `WRITTEN_OUT_WHERE_DRAWN_THE_SAME`. The answer is per stream, as for the other classes, and a
   `BDC` the grammar finds nothing at — a name tag over an unresolved name — is not written out;
   the output's own check then still finds the class and the write is refused rather than wrong.
3. The loss is `content-as-drawn`, as in ADR 3558: nothing a reader sees differs, and the
   producer's bytes for the two sections are gone.

## Why this is on the near side of the authoring line

No mark is invented or moved; the section governed nothing the window drew, and the proof is the
window's own drawing of both files.

## Evidence

`issue17679_2.pdf` converts under `as-shown` at 2b; veraPDF 1.30.3 passes the output.
`issue6342.pdf` stays refused by name: its form holds `model/operands-left-over`, which no write
answers and the grammar does not read (ADR 3542), beside an operator short of its operands, and
runs such as `c08.` that ISO 19005-2 section 6.2.2 refuses as operators the base
standard does not define.

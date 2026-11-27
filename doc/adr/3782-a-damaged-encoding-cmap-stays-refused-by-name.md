# 3782 — A damaged encoding `CMap` stays section 5.1's, refused by name

Session 2270. Status: **accepted**. Context: ADR 3570 point 5, ADR 3781, ADR 3428 (an encoding
`CMap`'s `bfchar` restated as `cidchar`); ISO 32000-2 §9.7.5.3, §9.7.6.2, §9.7.4.2.

## Context

ADR 3570 left two classes unbuilt: `to-unicode-damaged` (ADR 3781 builds it) and `cmap-damaged`,
an encoding `CMap` whose entries partly read. The brief allowed the second only with a draw proof.

## Decision

**Not built.** An encoding `CMap` is not a table of text beside the drawing; it is the drawing's
first step. Its codespace ranges decide how §9.7.6.2 cuts every shown string into codes, and its
`cidchar`, `cidrange` and `notdef` entries decide the CID that selects each glyph (§9.7.4.2). A
rewrite proved by lookup alone, as ADR 3781's is, would prove the code-to-CID table and not the
cutting: a damaged codespace entry changes which bytes form a code, and so which code is looked up
at all. The proof this needs is a draw proof — every shown string, cut and drawn by the font as
restated, selecting the outline and advance it selected — which ADR 3570's simple-font proof does
not reach (it asks single-byte codes of a simple font) and no composite-font draw proof exists in
the converter yet. So the class stays mapped to section 5.1 alone, and the output's own check
refuses it by name.

## Consequences

- A document whose only obstacle is a damaged encoding `CMap` stays refused, said by the class.
- Building it is a draw proof over the shown strings of every composite font naming the `CMap`,
  over the survey's strings, read whole (`pdf_archive::survey`, `shown_complete`); a route for a
  later round, which would also need the strings no content stream shows to be left as no code.

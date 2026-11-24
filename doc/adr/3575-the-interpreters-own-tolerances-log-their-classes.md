# 3575 — The interpreter's own tolerances log their classes

Session 2218, fix round 4 of the silent-repair campaign. Status: **accepted**. Context: ADR 3477,
ADR 3541 (the interpreter's first classes), ADR 3542 (the validator reads the grammar), review
2194's T05; ISO 32000-2 §7.8.2, §8.4.5, §8.10.2, §10.5, §14.6.1, §14.6.2.

## Context

Review 2194's T05 named the interpreter's tolerances that its own report and the log both missed:
operands left in front of an operator's own, `EMC`, `BX` and `EX` unbalanced, a `BDC` naming a
property list the resources do not hold, an `/ExtGState` entry of the wrong type, an unreadable
transfer function, and a form's malformed `/Matrix` and missing `/BBox`.

## Decision

- Eight classes in `pdf_model::repair`, each recorded at the content site of the stream the
  interpreter is reading, where it makes the repair: `operands-left-over` (§7.8.2's "operands shall
  not be left over"), `compatibility-sections-unbalanced`, `marked-content-unbalanced` (an `EX` or
  `EMC` that ends nothing, and a section the stream leaves open), `property-list-unresolved`,
  `graphics-state-entry-of-the-wrong-type` (one helper, `typed_entry`, for Table 57's scalar
  entries: an absent entry is §8.4.5's own "need not be present"), `transfer-function-unreadable`,
  `form-matrix-malformed` (an absent `/Matrix` is Table 93's identity, no repair) and
  `form-bbox-unusable`.
- What is drawn does not change: each site logs the reading it already made.
- `pdf-archive` maps all eight to section 5.1 alone. The survey does not run the interpreter
  (ADR 3542), so a check finds them only where the page was drawn from the same document first;
  teaching the grammar the token-level ones (left-over operands, the two balances) is left.
- The cost on a well-formed stream is one comparison per operator (the left-over count); the rest
  are on branches a well-formed file does not take.

## Consequences

- The viewer says each of them once (ADR 3544) with the class's own sentence.
- Left: a halftone's unreadable `/TransferFunction` (`component_transfer`'s `.ok()`), an
  out-of-range `/CA` and `/ca` clamped, a form's `/Resources` of the wrong type, and the blend mode
  a name Table 134 and 135 do not list reads as `Normal`, for which the round found no clause.

# 1114 — A text matrix that scales is a pair of lengths, and nothing else is

## Status

Accepted, 2026-09-15. Session 1100. Narrows what ADR 0211's layout reports under §12.7.4.3.

## Context

ISO 32000-2 §12.7.4.3 admits one `Tm` in a `/DA` and states what a processor does with it:

> The default appearance string shall contain at most one Tm (text matrix) operator. If this
> operator is present, the interactive PDF processor shall replace the horizontal and vertical
> translation components with positioning values it determines to be appropriate, based on the
> field value, the quadding ( Q ) attribute, and any layout rules it employs.

The *translation* is replaced, so the rest of the matrix stands over everything the appearance
draws. `variable_text` obeyed the sentence and measured the box in unscaled text space anyway, so
an advance of `w` was laid out as `w` and drawn as `a·w`: a right-quadded line under `2 0 0 2 0 0`
started at the box's right edge instead of ending there, and the clip cut off what ran past.
`Owed::TransformedTextMatrix` said so out loud, which is trap 5's rule and was the honest answer
for as long as nothing carried the matrix out.

## Decision

**A diagonal matrix with positive elements is carried out by changing the space the layout is
measured in; every other linear part keeps the report.**

`Scale` divides the box by the pair before anything is measured, so `wrap`, `auto_size`, the
quadding arithmetic and the caret, selection and offset a question asks for are all in the space
the matrix maps *from*; the positions the `Tm` carries and the answers handed back are multiplied
by it again on the way out. A `/DA` stating no `Tm` runs under the identity pair, where every
multiplication is by one — which is why the change moves no corpus page and `raster_golden` holds.

**Three linear parts are refused and the refusal is the clause rather than caution.** A rotation
puts a line off the single axis this module lays text along; a skew shears the glyphs relative to
the box the layout measures; a negative element mirrors. None of the three is a pair of lengths a
box can be divided by, so none can produce "positioning values it determines to be appropriate"
under this module's arithmetic, and a layout that pretended otherwise would be trap 5's silence
inside a feature otherwise built. A comb is the fourth case and is the clause's own: Table 231
bit 25 writes one `Tm` per cell, so the `/DA`'s matrix is dropped there and the report says so.

**What this does not decide.** Carrying out a rotation would mean laying a line along a direction
rather than an axis, and every question in `Asked` — a caret, a point, a selection — would become
a shape rather than a rectangle. That is a different piece of work and the row names it.

## Alternatives

**Scale the horizontal axis only.** Rejected on the picture: `2 0 0 2 0 0` doubles the glyph
height too, and a box divided on one axis puts the baseline where a half-height line would sit.
The planted defect that only multiplies `x` back is in `tests/variable_text.rs` for that reason.

**Keep the report and change nothing.** Honest, and it was the answer for eleven hundred sessions
— but the row had measured the population and found no witness, which is a reason to defend the
rule with fixtures rather than a reason to leave the clause unexecuted.

## Consequences

`pdf_model::variable_text` gains `Scale` and two parameters; `Owed::TransformedTextMatrix` keeps
its name and means three matrices and a comb. No public type changes and nothing crosses the ABI.
§12.7.4.3's ledger row stays `partial`, and the edge it names is narrower by the case that had a
closed form.

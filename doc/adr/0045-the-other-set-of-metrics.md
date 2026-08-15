# ADR 0045 — The other set of metrics

Status: accepted, 2026-07-30.

## Context

Vertical writing had been refused by name since the twentieth session, with the refusal itself
naming the fix:

> a `CMap` in vertical writing mode, which needs `/W2` metrics

and, for the predefined name, "`Identity-V`, whose vertical writing mode needs `/W2` metrics".
Four corpus documents wanted it; `vertical.pdf` should set two columns down the right edge of a
page and was drawing nothing at all.

The spec item beside it was §9.2, "Organisation and use of fonts", which was four `unreviewed`
rows and is where the second set of metrics is defined.

## What the two clauses say

§9.2.4 gives a glyph in writing mode 1 two vectors rather than one:

- **`w1`**, the displacement, "whose horizontal component shall be 0";
- **`v`**, "a position vector from the origin used for horizontal writing (origin 0) to the
  origin used for vertical writing (origin 1)".

§9.7.4.3 supplies them. `/DW2` is "an array of two values: the vertical component of the position
vector `v` and the vertical component of the displacement vector `w1`", with Table 115's default
`[880 -1000]`, and the two components it does not state are fixed by the clause: "the horizontal
component of the position vector shall be half the glyph width, and that of the displacement
vector shall be 0". `/W2` states them per CID, in the same two formats `/W` uses, three numbers
wide instead of one.

The clause's own NOTE explains the sign, and is the sort of sentence worth quoting rather than
deriving: "a negative value for the vertical component places the origin of the next glyph below
the current glyph because vertical coordinates in a standard coordinate system increase from
bottom to top".

## Three things move with the writing mode, and one does not

§9.4.4 computes "`tx` in horizontal writing mode or `ty` in vertical writing mode (the variable
corresponding to the other writing mode shall be set to 0)", and the two formulas differ in one
term: the horizontal scaling `Th` multiplies `tx` alone. That is the rule that is easiest to get
wrong by symmetry — `Th` scales the *width* of a line, not the advance along it, so a vertical
column set with `Tz 50` is half as wide and exactly as tall.

Character and word spacing are added to whichever component applies, and §9.4.3's `TJ`
adjustment "shall be subtracted from the current horizontal or vertical coordinate, depending on
the writing mode". All three were implemented as the horizontal case with the mode never asked
about, which is the shape of this finding: **a feature nobody uses in one axis is a feature
implemented in the other**.

The glyph itself moves too: the outline is stated relative to origin 0 and the text position
*is* origin 1, so it is drawn at `-v`.

The text readback swaps axes with them — a glyph placed against the writing direction begins a
new word and one placed off the line begins a new column, which is a newline either way.

## Decision

- **`CMap::identity_vertical`** is `Identity-H` with `/WMode 1`, which is what Table 116 says
  `Identity-V` is: "vertical version of Identity-H . The mapping is the same as for
  Identity-H ."
- **A `CMap` whose stream dictionary's `/WMode` disagrees with the file's own is still
  refused.** Table 118 requires them to agree, and a font where they do not has said two things.
- **`LoadedFont::vertical_metrics` takes the horizontal width in** rather than computing it,
  because `v`'s horizontal component is defined as half of it and duplicating that lookup would
  be a second place for `/W` to be read.

## Consequences

| | before | after |
|---|---|---|
| corpus documents drawing with nothing reported | 856 | **859** |
| pages we claim to draw completely | 1653 | **1656** |
| agreeing with the reference consensus | 808 | **811** |
| contradicted by it | 88 | **88** |
| ledger subclauses nobody has read | 348 | **344** |

Three pages joined the judged set and all three agree. §9.2 and §9.7.4 are complete as reviews.

The lesson worth keeping is about this project's own refusals. "A `CMap` in vertical writing
mode, which needs `/W2` metrics" named the two entries, the clause that defines them, and — via
the corpus gate — the four documents that wanted them. It is a to-do item with the answer
written on it, and it sat for sixteen sessions. **The refusals in this tree are its own
best-specified backlog**, and reading them for what they name is cheaper than reading a clause
for what is missing.

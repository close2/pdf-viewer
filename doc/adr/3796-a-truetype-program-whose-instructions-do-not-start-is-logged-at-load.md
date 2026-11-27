# 3796 — A TrueType program whose instructions do not start is logged at load

Session 2273. Status: **accepted** and built. Context: ADR 0727 (the hint-reliant family); ISO
32000-2 §9.9; ISO 19005-2 and ISO 19005-4 section 5.1.

## Context

For the family of TrueType programs whose instructions construct their glyphs (`skrifa`'s
`require_interpreter`), `pdf_font` builds a `HintingInstance` on the first outline, which runs the
program's `fpgm` and `prep`. Where they fail it draws the outline unhinted, and the outline path
holds no document, so nothing was logged (records 2262, 2264).

## Decision

The loader can tell, and it is asked there: `LoadedFont::load` calls `start_interpreter`, which for
a program in the family fills the same cell the outline path reads and, where the instance does
not start, logs the new class `font/truetype-instructions-unstarted` at the program. It maps to
section 5.1 alone: the program crosses as embedded, and no restatement of it is proved to draw what
this reader drew.

## Cost

Every embedded TrueType program now has its `sfnt` header, outline table headers and name records
read at load rather than at its first outline, which is work its first outline does anyway; the
instance itself is still built only for the family. A per-glyph instruction failure, met inside
the outline path, still falls back unhinted without a note: no document is in hand there.

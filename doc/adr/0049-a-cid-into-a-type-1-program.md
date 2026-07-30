# ADR 0049 — A CID into a Type 1 program, and the ToUnicode that lied

Status: accepted, 2026-07-31.

## Context

`issue11740_reduced.pdf` sat in `CONTRADICTED_UNEXPLAINED` and its side-by-side answered in one
look: `mupdf` and `hayro` draw **Оглавление**, `poppler` draws one blob, `ghostscript` draws
nothing, and we drew **Î ãëàâëåíèå** — the Windows-1251 bytes of the same word, shown in a
Latin-1 face. With `unsupported: []`. Trap 1's archetype, twelve sessions after it was written
down.

## Why we drew that

The document's `/F1` is a `Type0` font over a `CIDFontType0` whose descriptor embeds
**`/FontFile`** — a bare Type 1 program. §9.9's Table 124 gives a CIDFont `/FontFile2` and
`/FontFile3` and never `/FontFile`, so the file has written something the clause does not
describe, and this tree did what it does for a CIDFont with no usable program: substituted.

A substitute for a composite font can only be addressed through what the codes *mean*, because
a CID indexes the glyphs of the font that defined it and says nothing about any other — which
§9.7.4.2 states from the other side, "CIDs shall not participate in glyph selection". So the
substitute was addressed through `/ToUnicode`.

**And this file's `/ToUnicode` is a faithful record of the wrong thing.** It maps CID 1 to
U+00CE, CID 2 to U+00E3, CID 3 to U+00EB — the byte values `CE E3 EB`, which are `Огл` in
Windows-1251, recorded as though they were Latin-1 code points. Every step after that was
correct and the picture was wrong.

## Decision

**A CID indexes the charstrings of a bare Type 1 program, exactly as it indexes a non-CID-keyed
CFF's.** The clause does not say this about `/FontFile`, and it does not have to: it says it
about the case one analogy away, and §9.6 says the two cases are the same format.

§9.7.4.2, on a CFF whose Top DICT does not use CIDFont operators:

> The CIDs shall be used directly as GID values, and the glyph procedure shall be retrieved
> using the CharStrings INDEX

§9.6.2.1's NOTE 1, on what a CFF is:

> an alternative, more compact but functionally equivalent representation of a Type 1 font
> program

A bare Type 1 program is a name-keyed program whose charstrings are in an order, exactly as a
non-CID-keyed CFF's are. So the sentence transfers without inventing anything, and it lands on
machinery that already existed: `cid_to_glyph` already falls through to `/CIDToGIDMap`, which
for a `CIDFontType0` is the identity, and `type1::Program::draw` already takes a glyph *index*.
The change is to stop diverting `Program::Type1` to the substitute, and to read the program's
scale from its `/FontMatrix` rather than asking `FontRef` for an `sfnt` header it does not have.

**This is the second time §9.6.2.1's NOTE 1 has decided a design question**, after ADR 0040
made `cff.rs` and `type1.rs` share a type rather than a copy of §9.6.5.2's rules. A sentence
that says two things are one thing is worth more than most algorithms.

## Result

- `issue11740_reduced.pdf` draws **Оглавление**, and its raster now matches `mupdf`'s and
  `hayro`'s. It stays contradicted, because the two references that agree are `poppler` drawing
  one glyph and `ghostscript` drawing none — so it moves to
  `CONTRADICTED_REFERENCES_DREW_NOTHING` rather than out of the list.
- `issue5751.pdf` **left** the contradicted list by starting to report: its `/FontFile` is a
  Type 1 program this tree's parser refuses, and a malformed program is reported rather than
  substituted, which is what a *simple* font with the same defect already got. One document
  joined the incomplete row, and that is trap 5's rule rather than a regression.
- The test does not assert the glyph *count*, because a substitute drew ten glyphs too. It
  asserts the first glyph's width over its height: **0.94** for the capital О the program
  contains, **0.34** for the capital Î with a circumflex that the substitute drew. Confirmed by
  reverting the change.

## The spec track: §9.10, and clause 9 is complete

The family the demand item lands in, which is the best shape the two-track rule has.

§9.10.2 lists three methods for mapping a code to Unicode "in the priority given", and
`LoadedFont::text` is that list in that order: `/ToUnicode`, then a simple font's glyph name
through the Adobe Glyph List, then — not implemented — a predefined `CMap` plus its
`registry-ordering-UCS2` mapping, which is Table 116's licensing question seen from the other
end.

**The priority is load-bearing, and this page is the witness that it is right for one question
and wrong for another.** For *extraction*, `/ToUnicode` first is correct even here: it is the
producer's own statement, and a reader that second-guessed it would be inventing. For
*drawing*, it was never the right source at all — it became one only because the program had
been declared unusable. §9.10.1 draws exactly that line, between "the information content of
text" and "its rendered appearance", and the fix is that line applied.

## Consequences

- Clause 9 has no `unreviewed` row left: 65 of 65. Six clauses are now complete as reviews —
  7, 8, 9, 10, 11 and 13's exclusions.
- `CONTRADICTED_UNEXPLAINED` is 43, from 50 two sessions ago.
- The handover's feature table said a CIDFont writing `/FontFile` "is substituted for". It is
  read.

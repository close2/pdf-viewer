# ADR 0067 — A permission, not a fourth method

Status: accepted, 2026-07-31.

## Context

The text gate landed in the sixty-third session and named 46 documents whose page one we draw
completely and whose words `pdftotext` finds and we do not. Thirty-one of them were classified
as one thing: fonts where all three of §9.10.2's methods fail, which is the limit
`Interpretation::glyphs` was created for in the eighth session.

`issue15910.pdf` was in the seven left undiagnosed, and it is a small file worth reading. Its
content stream draws five lines. Three of them come back; two do not:

```
/F10 9.96 Tf BT 1 0 0 1 85 716.4 Tm (Käferhofen 10) Tj ET
/F10 9.96 Tf BT 1 0 0 1 52.1 715.95 Tm (Allgäu) Tj ET
```

`/F10` is `/Subtype /TrueType`, `/BaseFont /XTDZQO+Arial`, an embedded `/FontFile2`, `/Flags 4`
— the Symbolic bit — **no `/Encoding` and no `/ToUnicode`**. §9.6.5.4's symbolic route takes the
code straight to a glyph through the program's `cmap`, so the drawing is right and no glyph
*name* was involved anywhere. §9.10.2's second method asks for the name the glyph selection
algorithm used, and there is none. The page read back as though those two lines were not there.

## The sentence that had been read as an ending

The clause's closing sentence has two halves, and this project had been acting on the first:

> If these methods fail to produce a Unicode value, there is no way to determine what the
> character code represents in which case a PDF processor may choose a character code of their
> choosing.

The second half is a **permission**, and it is the only place in §9.10 that grants one.

## Decision

**Where all three methods fail for a simple font, name the glyph the way the font program names
it.** Two statements, in this order, both read from data the file itself carries:

1. **The `post` table's glyph name, through the Adobe Glyph List.** This is §9.10.2's own second
   step with the name coming from the program rather than from the encoding, which is the only
   difference between them.
2. **The program's Unicode `cmap` subtable, inverted.** An entry mapping U+00E4 to glyph 74 is
   the font saying that glyph 74 is `ä`, and it says so whichever direction it is read in.

`issue15910.pdf` needs the second, and needing it is the argument for having both: its `post` is
version 2.0 — the version that *does* carry names — with all seventy-nine of them the empty
string. A table that satisfies the format and states nothing.

This is a **choice under the clause's permission**, not a fourth method, and the doc comment
says so. The distinction matters because a method is owed and a choice is argued.

### Why this is not the fallback that fills the page

This file's own habits forbid exactly this shape: *"A fallback that fills the page is worse than
one that leaves it blank"* — "if nothing else matched, the code is the glyph index" drew
`v 0' ' W` for `What's an interval?`. The difference is where the answer comes from. Taking the
code as a character invents a mapping; asking the program what it drew reads one. And the
mechanism cannot answer where the font does not: a `post` of version 3.0 holds no names at all,
a name outside the Adobe Glyph List answers nothing, and a font with no Unicode subtable has
nothing to invert.

**It was measured rather than argued.** Over the pdf.js corpus the readback went from **96.5% to
97.8%** of the words `pdftotext` finds, and **no document moved the other way** — which is the
observation that separates this from a fallback, because a fallback that invents text lowers a
score somewhere. Three documents left the text gate's list: `bug894572.pdf` and `issue1350.pdf`
on the `post` route, `issue15910.pdf` on the inverted `cmap`. The 14 specification PDFs still
score 100%.

The inversion is built lazily and only where the `post` table left something unanswered: it
walks every mapping the font states, which is a few hundred for a subset and a few thousand for
a full CJK face, and most fonts never reach it because they carry a `/ToUnicode`.

## Consequences

**The corpus text score is 97.8%** and the failing list is 43. Neither pixel gate moves — this is
extraction, and the oracle's tolerance class asks `glyphs` rather than `text`, so not even a
bound changes.

**`glyph_for_selector` stops being `#[cfg(test)]`.** It had one caller, the
widths-against-charstrings cross-check; it now has two, because naming a glyph requires having
the glyph.

**Six documents are still undiagnosed** — `issue13211.pdf`, `issue16538.pdf`, `issue16553.pdf`,
`issue19182.pdf`, `issue19971.pdf`, `bug1392647.pdf` — and they are what the gate is for. Two
sessions of it have produced two defects and one clause reading, which is the rate the oracle
took several sessions to reach.

# ADR 0022 — Text rendering modes: three operations, not eight cases

Status: accepted, 2026-07-29.

## Context

ISO 32000-2 §9.3.6 Table 104 gives eight text rendering modes. This tree implemented one of
them properly and approximated three:

- **Modes 1 and 2** stroke, or fill and stroke, the glyph outline. Both were drawn as a plain
  fill in the *non-stroking* colour and reported as `text render mode 1` / `2`, on 14 corpus
  first pages. A page that outlines its display type came out solid, usually in the wrong
  colour, because a file using mode 1 normally sets only the stroking colour.
- **Modes 4 to 7** add the glyphs to the clipping path. No clip was built, so anything painted
  after the text object covered its whole area. `text_clip_cff_cid.pdf` shows "ABC123" in mode
  7 and then paints a rectangle expecting to see it through the letters; we drew a solid blue
  bar. 5 corpus first pages, and the oracle found this one — no metric this tree owns could
  have, because a solid bar is a plausible page.

Both were *reported*, which is the only reason either was schedulable rather than shipped.

Nothing new was needed in `pdf-render`: `Command::Stroke` and `Clip` have carried everything
required since the first display list. What was missing was the middle — the same shape as the
`d` operator in ADR 0018.

## Decision 1 — read Table 104 as three independent operations

The obvious implementation is a `match` on eight modes. The clause is not written that way:
it describes filling, stroking and clipping as three things that "shall have the same effects
for a text object as they do for a path object", and the table is their eight combinations.
So the code computes three booleans and each drives one emission. Adding a rule then touches
one branch rather than four arms, and the reader can check the code against the table by
reading down a column.

## Decision 2 — move the glyph outline into user space to stroke it

§9.3.6: "The graphics state parameters affecting those operations, such as line width, shall
be interpreted in user space rather than in text space."

A `Command::Stroke` carries its width and dash lengths in its *path's* space, and a glyph
outline is in em units — so handing the outline over unchanged would have divided the width by
the font size and stretched it by the horizontal scaling. An 11-point glyph would have been
outlined about eleven times too thickly, and a horizontally scaled one anisotropically.

Two ways out: scale the width by the inverse of the glyph-to-user transform, or move the
geometry. The first is only correct for a uniform transform — the text matrix may scale the
axes differently or shear, and there is no scalar width that expresses that. So
`Path::extend_transformed` bakes the glyph-to-user transform into the points and the command
carries the CTM and the state's stroke parameters unchanged. It is exact for any text matrix.

The cost is a copy of the outline per stroked glyph, where the fill path shares one `Arc`
across every occurrence of a letter. It is paid only by the four modes that stroke, and those
are 14 documents of display type rather than pages of body text.

## Decision 3 — an empty accumulator sets no clip, because the clause says so

§9.3.6: "If no glyphs are shown or if the only glyphs shown have no outlines (for example, if
they are ASCII SPACE characters (20h)), no clipping shall occur."

This is the sentence most likely to be skipped, and skipping it fails in the worst direction:
the natural implementation clips to whatever accumulated, and clipping to an empty path hides
everything drawn after the text object. A mode-7 text object showing one space is what a
producer emits for a blank line of OCR text, so the page after it would have gone blank. There
is a test, and it fails when the guard is removed.

The corpus supplied a second, unforeseen instance the same day. `recursiveCompositGlyf.pdf`
shows "hello world" in mode 7 over a red page and its font is a deliberately malformed
TrueType whose composite glyph refers to itself; `skrifa` produces no outline, so no clipping
occurs and the page comes out red. So do poppler's and `hayro`'s. `mupdf` refuses the font and
draws nothing; only `ghostscript`, with its own TrueType interpreter, recovers the glyphs. The
clause decides our answer and the agreement is evidence we read it the same way — which is the
only thing agreement is ever evidence of.

## Decision 4 — a hidden layer suppresses the marks and not the clip

§8.11.3.1, on content an optional content group turns off:

> Graphics state operations, such as setting the colour, transformation matrix, and clipping,
> shall still be applied.

and

> graphics state parameters that persist past the end of a marked-content section shall be the
> same whether the optional content is visible or not.

The clip a text object leaves behind outlives the `ET` that built it and lasts until `Q`, so it
is one of those parameters. Hidden text therefore still accumulates its outlines while painting
nothing. `end_path` already applied the same reading to a path's `W`; this makes the two agree
rather than leaving one clause read twice, differently.

## Decision 5 — an undefined `Tr` operand is reported, not obeyed

Table 104 defines 0 to 7 and says nothing about anything else. With three booleans selecting
operations by matching against the defined values, an operand of 9 matches none of them and
would silently draw nothing — a whole text object missing with `unsupported: []`, which is
principle 3's failure mode exactly. The mode is left as it was and the operand is named. No
corpus document does this; the guard exists because the cost of it being wrong is a blank page
and the cost of having it is one comparison.

## What the family review then found

The demand item was §9.3.6; the ledger review that followed covered §9.3 and §9.4 entire,
which is the pairing `doc/HANDOVER.md` recommends. It produced two things §9.3.6 would not
have:

- **§9.3.3 was implemented as a rule about a code's *value* rather than its encoded length.**
  Word spacing applies to "every occurrence of the single-byte character code 32 … when using
  a simple font (including Type 3) or a composite font that defines code 32 as a single-byte
  code", and "shall not apply to occurrences of the byte value 32 in multiple-byte codes". We
  applied it to any code numerically equal to 32, so an `Identity-H` string containing the two
  bytes `00 20` was pushed right by `Tw` for every one of them. No page of Latin text can show
  this, because a composite font's space is usually some other CID entirely.
  `LoadedFont::has_single_byte_codes` answers it per font, which is exact for every mapping
  this crate builds; a general `CMap` may mix code lengths, and when embedded `CMap` streams
  land it has to become a property of the code.
- **§9.3.8, text knockout, is a `silent` row** — the third in the ledger, after §11.4.6 and
  §8.11.4.4, and a corner of the first. `/TK` arrives only through an `/ExtGState` and nothing
  looks for the key. Its initial value is `true`, which means the whole text object behaves as
  a non-isolated knockout group so that a later glyph overwrites an earlier one where they
  overlap; we composite normally, which is indistinguishable while glyphs are opaque and wrong
  under a constant alpha or a non-Normal blend mode.

## Consequences

- 18 corpus documents stop reporting a rendering mode and 16 of them become complete; the
  corpus's incomplete count falls 251 → 235 and the `Operator` row 33 → 15, leaving nothing on
  that row but malformed streams.
- 16 pages enter the oracle's judged set. 11 agree with the reference consensus, 3 are
  ambiguous, 2 are not comparable, and **none is contradicted** — the contradicted count stays
  at 103 with the same names.
- Ledger: thirteen rows of §9.3 and §9.4 move off `unreviewed`, `UNREVIEWED_CEILING` falls
  686 → 673, and `REVIEW_OWED` loses §9.3.6 and §9.4.1.
- The `d1` uncoloured-glyph rule interacts with this and was already right: Table 111 gives a
  glyph description one colour, so `d1` makes the stroking colour equal the fill colour, and a
  mode that now genuinely strokes therefore cannot change an uncoloured glyph's colour.

## Alternative rejected — keep approximating mode 1 as a fill

It renders something for 14 documents rather than nothing, which is why it was there. What
decided against it is that the approximation is not close: an outlined heading and a solid one
differ everywhere, and the fill used the wrong one of the two colour parameters. The report
made it honest, not correct, and the implementation cost one function.

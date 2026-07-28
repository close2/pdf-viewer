# ADR 0018 — A glyph that is a content stream

Status: accepted, 2026-07-29.

## Context

Every font this tree could read named a *program* — `TrueType`, CFF, CFF-in-`OpenType` — and
`pdf-font` turned a character code into an outline by parsing it. ISO 32000-2 §9.6.4 defines a
fourth kind that has no program at all:

> In Type 3 fonts, glyphs shall be defined by streams of PDF graphics operators.

Each stream sits in the font's `/CharProcs` dictionary under a glyph name, and drawing one
means running the content interpreter. That is the whole architectural question this ADR
answers, and it has a corollary the eighth session paid for: **a Type 3 font cannot be
substituted**, because the names in its `/Differences` array are procedure names.
`french_diacritics.pdf` names its procedures `/a192`, `/a199`, `/a224`, which are also
`ZapfDingbats` glyph names, so the substitution path drew dingbats and reported nothing. The
eighth session stopped that by refusing the font, which turned 23 documents of silence into 24
documents that said what they needed — and made this the largest named gap in the corpus.

## The decision

Read the font in `pdf-model`, not in `pdf-font`. `pdf_font::LoadedFont::load` refuses a Type 3
font with `FontError::Type3`, and that refusal is the hand-off: `crate::type3::Type3Font`
picks it up, and the interpreter runs each glyph description the same way it already runs a
form `XObject`, an annotation appearance and a tiling pattern cell.

The alternative — teaching `pdf-font` to hand back "a content stream" — would have inverted
the crate dependency, since interpreting one needs the interpreter. The layering says a font
crate produces outlines; a font whose glyphs are *drawings* is not a font crate's business.

Four of §9.6.4's rules are where the work is, and each is a defect if it is missed:

- **Glyph space is whatever the font says it is.** §9.2.4 gives every other font a glyph space
  of one thousandth of a text-space unit; a Type 3 font states its own in `/FontMatrix`. A
  font drawing on a 1-unit grid — real documents do — is a thousand times the usual
  convention. `/FontMatrix` is therefore required rather than assumed: a file without one is
  reported, because guessing the common `[0.001 0 0 0.001 0 0]` would silently draw such a
  font a thousandth of its intended size.
- **The widths are in that space too**, which Table 110 states by contrast: they are "in glyph
  space as specified by `FontMatrix` (unlike the widths of a Type 1 font, which are in
  thousandths of a unit of text space)". Only the horizontal component of the transformed
  width is used, which under a matrix without rotation is the `a` coefficient alone.
- **The encoding is the only mapping there is.** §9.6.5.3, and its NOTE that "Type 3 fonts do
  not support the concept of a default glyph name". A code the encoding does not name reaches
  nothing; a name absent from `/CharProcs` paints nothing. Neither is an error, and both still
  advance the text position.
- **The recursion is real.** A glyph description may show text in another Type 3 font, and
  `ContentStreamCycleType3insideType3.pdf` in the corpus makes that a cycle. It shares the
  bound with form `XObject`s, because it is the same danger and the same cost.

## `d1` says the glyph has one colour, and the references disagree about what that means

Table 111 gives a glyph description two ways to start. `d0` states a width and declares that
the description specifies its own colour. `d1` states a width and a bounding box and declares
that it specifies "only shape, not colour" — its colour "shall be determined by the graphics
state in effect each time this glyph is painted by a text-showing operator".

`Type3WordSpacing.pdf` in the corpus is a fixture written to test exactly this, and it split
the reference renderers two against two. Its page sets a **blue** fill and a **red** stroke,
shows text in render mode 0, and its square glyph is a `d1` description that *strokes*:

| | square |
|---|---|
| `poppler`, `ghostscript` | blue, dashed |
| `mupdf` | red, dashed |
| us, before | red, solid |

Two readings. Either a `d1` description's stroke is a stroke, and takes the inherited
*stroking* colour; or the description's marks are a shape, and the shape is painted in the one
colour the text-showing operator was using — the non-stroking colour, for render mode 0.

The clause settles it, and not by counting renderers. "Its colour" is singular. The
description "is executed solely to determine the glyph's shape". And the clause's own reason
for admitting an image mask inside such a description is that a mask "merely defines a region
of the page to be painted with the current colour" — the mask model, in which every mark a
`d1` description makes is part of one region wearing one colour. Under the other reading a
glyph would have two colours, and the sentence naming one would mean nothing.

So an uncoloured description's marks are painted in the colour the glyph is painted with. That
`poppler` and `ghostscript` agree is evidence we read the clause the way they did, which is
all agreement is ever evidence of.

## Reading §8.6.8 rather than Table 111 alone

Table 111 says a `d1` description should not set "the colour (or other colour-related
parameters including transparency)" and points at §8.6.8. §8.6.8 is the precise statement, and
reading it produced three things Table 111's sentence alone would not have:

1. **The same restriction governs an uncoloured tiling pattern** (`/PaintType 2`) "and to all
   other content streams invoked from within" either kind of figure. One flag now serves both,
   which is what the clause describes rather than two features that happen to rhyme. A cell
   that set its own colour used to be obeyed.
2. **The list is not what Table 111's parenthesis suggests.** It is the twelve colour
   operators plus `ri` and `sh`, and within an `/ExtGState`, `/TR`, `/TR2`, `/BG`, `/BG2`,
   `/UCR`, `/UCR2`, `/HT` and `/UseBlackPtComp`. The constant alphas and the blend mode are
   *not* on it, so an earlier reading of ours that suppressed `/ca`, `/CA` and `/BM` was
   wrong and was removed.
3. **"Unless painting an image mask, all image painting operators shall be ignored"**, with a
   NOTE saying why: a stencil designates places to paint the current colour, and any other
   image carries colours of its own.

The same clause carries an unrelated sentence that turned out to be a live defect: `cs` and
`CS` set the colour to its initial value, "which depends on the colour space", and this tree
set black for every space. Six cases, and three of them are not black — a `Separation` starts
at *full* ink, an `Indexed` space at whatever its table's entry 0 holds, and a `Pattern` space
at a pattern "that causes nothing to be painted".

## Consequences

**24 documents stopped reporting a Type 3 font, and 10 of them became complete.** The other 14
immediately began reporting something the refusal had been standing in front of — ten draw
their glyphs as *inline images*, which this interpreter does not decode (§8.9.7), one carries
a soft mask, two use a stroking text render mode and one has a malformed number in a glyph
description. That is the project's own habit in miniature: fixing the mask shows what the mask
was hiding.

Ten pages joined the oracle's comparison and none of them is contradicted.

**A glyph description is re-run per occurrence, and nothing caches it.** `LoadedFont` caches
outlines because a page is the same few dozen glyphs over and over, and the same argument
would apply here — but the project's rule is that an optimisation is justified by a benchmark,
and nothing has measured this yet. The corpus gate's total did not move. When it does, the
cache belongs in `Type3Font` beside the `/CharProcs` dictionary it would key.

**What this does not do** is make a Type 3 font's *text* extractable beyond `/ToUnicode`. A
glyph name in such a font names a procedure and says nothing about the character, so a font
without that entry contributes nothing to the page's text — which is honest rather than
silent, and is the same position `pdftotext` takes.

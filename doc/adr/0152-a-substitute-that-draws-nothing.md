# ADR 0152 — A substitute that draws nothing

Status: accepted, 2026-08-02. Session 182. The fifth item the ambiguous ranking named, and it
disproved a sentence this project had written about itself.

## The page

`issue8372.pdf` is 200×50 points and draws two characters: `目录`, in `AdobeHeitiStd-Regular`,
not embedded, `Adobe-GB1` reached through `UniGB-UTF16-H`. Three references draw them. We drew
**nothing**, with `0 commands` and `unsupported: []`.

## Why nothing was said

§9.10.2's third method works exactly as ADR 0140 built it: the predefined `CMap` turns the codes
into CIDs, `Adobe-GB1-UCS2` turns those into `目` and `录`, and `LoadedFont` looks each character
up in the substitute face's `cmap`. The face is chosen by `substitute::installed`, which matches on
the generic *family* the descriptor implies — and the family a Chinese font resolves to is a Latin
face, which has no glyph for either character. `glyph_for` answers `None`, `outline` answers
`None`, the interpreter draws nothing, and nothing is wrong enough anywhere to report.

The handover has named this gap for dozens of sessions and asked for the measurement first:

> A font is reported as a whole, and that is not fine-grained enough. `FontError` is the only
> channel a font has, so a font that maps *some* of its document's codes draws those and says
> nothing about the rest. … Not hard; not done; measure the volume first.

## The measurement, which chose the condition

`LoadedFont::uncovered_character` answers `Some(c)` in one situation only: the font was
substituted, §9.10.2 gave the code a character, and the face has no glyph for it. Asking it in the
direction "what did this code mean, and can the face draw it" rather than "is there an outline"
keeps a **space** out of the count — U+0020 is in every face — which is the distinction the
handover said was needed.

Reporting *every* such code named **13** corpus documents, most of which draw nearly all of their
text. That is trap 11's failure: a report costs a judged page, and thirteen documents leaving the
gated set to say "one character of this page is missing" is a bad trade. So the condition is the
one the *simple*-font path already applies at load time — "the face draws none of the codes the
document declares" — counted per font per page and reported only where the count drawn is zero.

That names **10** documents, and eight of them are pages we draw blank: `issue8372.pdf`,
`issue3521.pdf`, `issue13343.pdf`, `issue19182.pdf`, `90ms_rksj_h_sample.pdf`,
`noembed-eucjp.pdf`, `noembed-sjis.pdf`, `noembed-identity.pdf`, `noembed-identity-2.pdf`,
`noembed-jis7.pdf`.

## What it disproved

`CONTRADICTED_SUBSTITUTED_FONT`'s comment said of the last two, added in the hundred-and-fifty-
sixth session:

> the side-by-side has the same five kana in the same places in all four panels … which is what
> two different Japanese faces look like

**Our panel is blank.** `noembed-eucjp.pdf` produces no commands and its raster's mean is exactly
1.0; `poppler`, `mupdf` and `ghostscript` draw あいうえお and `hayro` — the other Rust renderer —
is blank beside us. The pages were contradicted because we drew nothing, and the sentence written
about them described the *references'* panels. The same claim is in the handover, which said those
documents "now draw あいうえお in a face the references do not have".

Nothing could have caught it: no gate looks at a page's ink, the corpus gate had nothing to
report, and a comment is not executable. What caught it was making the silence loud and then
reading what fell out.

## The cost, stated

- Corpus: **76 → 86 incomplete**. Every one is a new report, which trap 5 says is not a regression
  — and this time eight of them are blank pages.
- Oracle: complete pages 1684 → 1672. `agrees` 846 → **840**, `ambiguous` 755 → **751**,
  `contradicted` 72 → **70**. The two that left the contradicted list are the `noembed` pair, and
  they left because they are now *reported*, not because they are fixed;
  `CONTRADICTED_SUBSTITUTED_FONT` records that in place of the sentence it used to hold.
- The six pages that used to *agree* were agreeing on whitespace: five kana on an A4 page move a
  mean by less than the bound. Trap 1, from the other side.
- Text gate: unchanged at 98.2%; its denominator moves with the incomplete set, as it is meant to.

## What is owed, and it is now measured

**A substitute chosen by coverage rather than by family name.** `substitute::installed` ranks
candidates by the generic family a descriptor implies, which cannot express "this face must be
able to draw Chinese". This machine can: `fc-match :charset=76EE` answers `DroidSansFallback.ttf`,
which is why three references draw the page. Ten documents and eight blank pages are what it is
worth, and the shape is a coverage predicate over `substitute::catalogue`'s faces, sampled from
the collection's own `-UCS2` table. It is a *choice* about which face, which §9.10.2 leaves open —
so it is a documented choice, and it is not this session's, because a report that is true is worth
more than a substitute that is guessed.

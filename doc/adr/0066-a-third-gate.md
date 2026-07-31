# ADR 0066 — A third gate: the text, over the whole corpus

Status: accepted, 2026-07-31.

## Context

This project has two gates. `corpus.rs` asks whether 974 real documents open and draw without
reporting anything; `oracle.rs` asks whether what we drew agrees with two independent
renderers. Neither can police text. The oracle's own tolerance comment says why: the reference
renderers disagree with each other at worst-tile 26–28 on text pages, which is above the signal,
so `Tolerance` measures structural similarity instead and asks whether the same shapes are in
the same places — not whether they are the right letters.

The instrument that *can* answer that has existed since the fifth session and has been pointed
at fourteen files. `Interpretation::text` is a readback of what the drawing pass placed, from
the same code-to-glyph decisions, and `text_extraction.rs` compares it against `pdftotext` over
the 14 specification PDFs in `doc/`. The handover has named the extension as an opportunity
since the thirty-first session: "974 documents against 14, needing only a tolerance, since
`pdftotext` supplies the reference for each."

## Decision

**Run the same comparison over the pdf.js corpus, as an `#[ignore]`d gate with a named ratchet.**

It runs in **30 seconds** over 974 documents — `pdftotext` per document, in parallel with rayon,
each given the same 30-second budget `pdfref` gives a renderer, because this corpus holds files
written to make a reader loop.

Three decisions inside it:

**Only pages we claim to draw completely are gated.** A page whose font this tree refuses draws
no glyphs and reads back nothing; failing it here would score the report rather than the
extraction. That is the oracle's denominator rule, and it carries the oracle's warning with it:
the gated set grows when reports stop firing and shrinks when a silence ends. 95 pages are
incomplete and not gated.

**The floor is 0.90, not the 0.99 the specification PDFs are held to.** Those are consistently
typeset files from three producers. The pdf.js corpus is 974 files chosen for having broken a
reader, and what survives at this level is disagreement about what a *word* is rather than about
which glyph was drawn.

**Two more compatibility foldings, and both are arguments rather than conveniences.** `fold`
already normalises the dashes, quotes and spaces producers spell inconsistently. It now also
decomposes the Alphabetic Presentation Forms — a `/ToUnicode` mapping a ligature glyph to U+FB01
is *correct*, and one mapping it to `f` then `i` is correct too, and Unicode records the pair as
a compatibility equivalence — and removes hyphens from both sides, because a word broken across
a line is written with a hyphen the word does not have and **nothing in the content stream says
which hyphens those are**. That is the module's own argument about spaces, one level up. The two
foldings took the corpus from 94.4% to **96.5%** and the failing list from 56 to 47.

## What it found immediately

**`operator-in-TJ-array.pdf` drew one word of five.** The file writes

```
[(Grandes) 0.0 Tc -250.0 (Clientèles,) 0.0 Tc … (Marchés) ] TJ
```

— an operator between two elements of an array. §7.3.6 makes an array "a one-dimensional
collection of objects arranged sequentially", so a keyword is not an element of one; §7.8.2 puts
an operator after its operands, and an array *is* one operand, so it is not an operator either.
The content interpreter flattens arrays deliberately, so each `Tc` reached the operator dispatch
and consumed the runs accumulated before it.

The recovery is to skip the keyword and keep the array, **and report it**: the file is malformed,
no clause states a reading for it, and drawing the text in silence would be the fallback
principle 3 forbids. This is a fifth instance of the pattern trap 5 lists — a report that
accompanies drawing rather than replacing it — and it costs one page out of the oracle's judged
set, which is what a report always costs. The page went from 7 glyphs to 39 and from 25% of the
reference's words to 100%.

An unbalanced `[` would otherwise suppress every operator for the rest of a stream, which on a
fuzzed file means a blank page; array depth is abandoned after one operand cap's worth of tokens.

## What the remaining 46 are

Classified with one number per document — how many of the glyphs that marked the page produced a
character in the readback — which separates two failures that both look like a low score.

- **31 draw glyphs and name none of them.** This is the limit `Interpretation::glyphs` was
  created for in the eighth session, not a defect: §9.10.2's three methods all fail and the
  clause says plainly that then "there is no way to determine what the character code
  represents". `issue918.pdf` is the archetype and the largest entry — 1327 glyphs, 193
  reference words, a readback of nothing but inferred spaces — because its Type 3 fonts name
  their glyphs `/a45`, `/a66`, `/a97`, which is the character code in decimal, and it states no
  `/ToUnicode` at all.
- **7 are right-to-left scripts read back in painting order.** `issue10301.pdf` draws Hebrew
  that reads `אבג` and we return `גבא`. This is the module's own rule again: a content stream
  records positions, not words, and painting order, not reading order. Turning one into the
  other is the Unicode bidirectional algorithm over a layout this crate does not analyse.
- **`issue8697.pdf` is a question about a clause.** A Symbol font whose glyphs are Greek and
  whose codes are Latin: §9.10.2's second method names the *glyph*, so we read
  `Ωηατ Οπερατινγ Σψστεµσ ∆ο` where `pdftotext` reads `What Operating Systems Do`. Both are
  defensible and the clause is about naming what was drawn.
- **7 nobody has diagnosed**, and they are the list worth working.

## Consequences

**A gate that can fail on the right letters, over 974 documents, in 30 seconds.** The existing
14-file test is unchanged and still held to 0.99; the two share their scoring code, so the
foldings apply to both.

**`MAX_INCOMPLETE` rises 96 → 97**, and it is a new report on a page that now draws *more* —
the rise this ratchet exists to permit.

**The oracle's judged set falls by one** for the same reason, 832 → 831 agreeing, with
contradicted unchanged at 65.

**The cost of the instrument is one external program per document.** `pdftotext` is already
required by the existing test and ships with poppler, which the oracle needs anyway. No caching:
30 seconds is the whole run, and ADR 0020's argument for remembering reference *renders* — 1000
seconds of subprocess time — does not apply at this scale.

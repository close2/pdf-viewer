# What is left of font substitution

Status: reported at runtime; three distinct gaps, the third now measured.
Priority: 21
Corpus: 2 + 40 documents
Clauses: §9.10.2, §9.7.4.2, §9.8.3
Code: `crates/pdf-font/src/substitute.rs`, `crates/pdf-model/src/content.rs`

## 1. A per-character fallback — 2 documents

§9.10.2 gives a code a character and the face a family match found has no glyph for it. Since the
hundred-and-eighty-third session a substitute is chosen by **coverage** — the widest-repertoire
face on the machine that can draw a character of the collection's own script (ADR 0153) — and
eight of the ten blank pages that named this now draw. The two left are `issue11555.pdf` and
`issue2128r.pdf`, whose characters no single face on this machine has.

What is owed is a **per-character** fallback, which every real text stack has and which means
`LoadedFont` carrying more than one face. Note that these two draw *most* of their text, so they
no longer report — see gap 3.

## 2. A substitute that cannot be addressed — 40 fonts

Composite fonts naming an `Identity` ordering, where the codes are indices into a font nobody
supplied and §9.10.2's third method has nothing to read. §9.7.4.2 leaves such a font reachable
only through `/ToUnicode`, which addresses by character; without one there is no question to ask.
Honest refusals. The `-UCS2` `CMap`s closed the rest of this population in session 156 (ADR 0140).

## 3. A font is reported as a whole, and that is not fine-grained enough

`FontError` and the "drew nothing" tally are the only channels a font has, so a font that maps
*some* of its document's codes draws those and says nothing about the rest. ADR 0152 measured the
alternative — reporting every uncovered code named 13 documents that mostly draw fine, and each
report costs the oracle a judged page (trap 11) — and chose "drew none" deliberately.

The general case needs a report where a glyph is *shown*. **The distinction it was said to need
already exists** and this file was wrong about that: `LoadedFont::glyph_index` is public and
answers "which glyph does this code reach, if any", so "no glyph" is `None` from it while "blank
glyph" is `Some(g)` with an empty outline — `LoadedFont::outline` collapses the two and
`glyph_index` does not. The whitespace-readback test added in ADR 0157 is the other half.

So what was owed was **the measurement, not the mechanism**, and the two-hundred-and-forty-fourth
and -fifth sessions took it. `Interpretation::codes_without_a_glyph` counts the codes a page
showed that reached no glyph — excluding the two cases that are not marks missed, a code that
reads back as whitespace and a code §9.10.2 gave a character the substitute lacks — and
`tests/corpus.rs` prints the sum over page one of all 974 documents, **counting only pages that
report nothing**, because a document whose font already says "no outline for any of the codes
this page shows" is not silent about it:

```text
codes reaching no glyph *in silence*: 50 over 9 documents
    26 pr12564.pdf        3 bug1392647.pdf         1 issue2017r.pdf
     8 issue14821.pdf     2 issue2884_reduced.pdf  1 issue4398.pdf
     6 bug1151216.pdf     2 issue4650.pdf          1 issue7020.pdf
```

**And then the largest contributor turned out not to be a missing mark at all.**
`PDFVIEWER_TRACE_MISSING_GLYPH=1` names each code's readback, and all 26 of `pr12564.pdf`'s are
one code that reads back as `#` — `pdftotext` renders that page as `1101#Strayer#Drive#*#San#Jose`,
so the code **is** the document's space and having no outline is correct. The whitespace exemption
is right in principle and blind to a font that reads a space back as something else.

So the real silent population is **24 codes over 8 documents**, and ADR 0152's trade holds
comfortably: turning them into reports would cost the oracle eight judged pages to name
twenty-four codes.

**`issue14821.pdf` is the one worth opening**, and it is the whole of what is left worth opening:
eight codes reading back as `1`, `2`, `3`, `7`, `e` and three `x`s, drawn as nothing. That is
eight characters of a page's text absent in silence. The rest are ones and twos, two of them
reading back as a replacement character or a CJK ideograph, which is a `/ToUnicode` question
rather than a glyph one.

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
session took it. `Interpretation::codes_without_a_glyph` counts the codes a page showed that
reached no glyph — excluding the two cases that are not marks missed, a code that reads back as
whitespace and a code §9.10.2 gave a character the substitute lacks — and `tests/corpus.rs`
prints the sum over page one of all 974 documents:

```text
codes reaching no glyph: 109 over 14 documents
    39 issue12963.pdf          8 issue14821.pdf         4 issue6127.pdf
    26 pr12564.pdf             6 bug1151216.pdf         3 bug1392647.pdf
    10 recursiveCompositGlyf.pdf   5 issue13316_reduced.pdf   2 issue2884_reduced.pdf …
```

**ADR 0152's trade still holds**: the population is 14 documents, where that ADR measured 13, so
turning every one of these into a report would still cost the oracle fourteen judged pages to
name 109 codes. The measurement is not a gate and nothing fails on it.

**What is new is the shape of the population, and it is two documents.** `issue12963.pdf` (39)
and `pr12564.pdf` (26) are two thirds of the whole count, and the second is a page this project
has already looked at twice — `AMBIGUOUS_GLYPH_SCAN_CONVERSION` diagnosed it as glyph coverage
without noticing that 26 of its codes draw nothing at all. Those two are worth opening before any
decision about reporting: a per-code report is a poor trade against 109 codes spread thinly and a
good one against 65 concentrated in two files.

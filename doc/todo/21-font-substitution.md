# What is left of font substitution

Status: reported at runtime; four distinct gaps, the first now **empty of witnesses**, the third measured and the fourth measured and declined.
Priority: 21
Corpus: 40 documents
Clauses: §9.10.2, §9.7.4.2, §9.8.1, §9.8.3
Code: `crates/pdf-font/src/substitute.rs`, `crates/pdf-model/src/content.rs`

## 1. A per-character fallback — **0 documents, and this section was wrong about its own two**

§9.10.2 gives a code a character and the face a family match found has no glyph for it. Since the
hundred-and-eighty-third session a substitute is chosen by **coverage** — the widest-repertoire
face on the machine that can draw a character of the collection's own script (ADR 0153) — and
eight of the ten blank pages that named this now draw.

**All ten do.** This file said the two left were `issue11555.pdf` and `issue2128r.pdf`, "whose
characters no single face on this machine has", and the two-hundred-and-fifty-sixth session opened
the pictures: `issue2128r.pdf` draws every one of its Chinese characters and `issue11555.pdf`
draws its whole vertical mixture of Latin and kana. Both report nothing, both are above the text
gate's floor, and neither has a code reaching no glyph. The claim was a *prediction* about ADR
0153's rule that nobody re-checked after the rule landed — the same shape as a ledger row whose
"what IS done" half is wrong, one directory over, and `doc/todo/01`'s sweeps do not look here.

**So the mechanism is owed with no witness at all.** It was built in that session and reverted in
the same one, and what the attempt is worth is the two reasons it was not kept:

- **A sample is a sample, so a chain chosen from one is the wrong shape.** The first design
  extended `installed_covering` to return several faces covering `script_sample`'s characters
  between them. It changed nothing on either document, because both were *already* covered by one
  face — the characters that could go missing are the ones the sample does not contain, which is
  the whole point. What the mechanism actually needs is a lookup at *draw* time, per character.
- **The draw-time version has no machine-independent gate.** It works: the machine's widest faces,
  ranked by `cmap` size, bounded at eight, asked in order when the primary face lacks a character.
  But every assertion about it is an assertion about which faces this machine has, and ADR 0133
  exists precisely because `substitute.rs` was the last machine-dependent code in the tree. A
  feature whose only test says "on this machine, in August 2026" is not one this project ships.

What would make it shippable is a witness — a document whose substituted composite font shows a
character its chosen face lacks — or a face this binary carries that is addressable by character.
§9.6.2.2's fourteen are not: they are name-keyed CFF, which §9.7.4.2 leaves unreachable for a
composite font.

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

**`issue14821.pdf` was the one worth opening, and it is answered**: the document asks for glyphs
its own embedded subsets do not contain. Five of its eight are `Identity-H` CIDs — 21, 22, 26,
91 — whose `loca` entries are empty by the glyph table's own statement and which the descendant's
`/W` does not list either; the other three are ASCII codes in a nonsymbolic `TrueType` subset
whose `/Encoding` names a content stream, whose `(3,1)` `cmap` maps all three to glyph 0, and
whose `post` is version 3.0 with no glyph names at all. Every route §9.6.5.4 and §9.7.4.2 state
ends at nothing. The evidence is in those two ledger rows and the refusal is on the handover's
closed-by-decision list; `poppler` draws them from a face this machine has.

The rest of the population is ones and twos, two of them reading back as a replacement character
or a CJK ideograph, which is a `/ToUnicode` question rather than a glyph one.

## 4. A fourth gap, measured and deliberately not closed — the substitute's cap height

Not a code reaching no glyph but a glyph of the wrong size, and it is the only part of substitution
this tree has a *number* for. The compiled-in Helvetica is Liberation Sans; the reference renderers
resolve `NimbusSans` through `fontconfig`. Drawn straight from the two files, the capital `I` is
**0.687500 em** against **0.729167 em**, in the regular and the bold alike, and the corpus rasters
reproduce both exactly — `issue6108.pdf` at 12 pt draws 66 device rows against 70, `issue7580.pdf`
at 18 pt draws 99 against 105. That is 5.7% shorter capitals and 1.0% to 7.7% of the page's ink on
the six `CONTRADICTED_SUBSTITUTED_FONT` pages naming a Helvetica or Arial face; the serif faces have
no such gap, and the advances have none in either family.

**It is left open on purpose** (ADR 0267): §9.5 NOTE 5 puts substitution beyond the standard,
§9.8.1 says a descriptor's metrics exist so that a processor may synthesise or select a substitute
and states no `shall` about it, and closing the gap by scaling to 0.729167 would be scaling to where
another program's font sits. **What would open it is a document** — a `/FontDescriptor` stating a
usable `/CapHeight` for a non-embedded face, which no corpus page has yet been shown to do.
`/CapHeight` is on §9.8.1's ledger row's list of Table 120 entries this tree does not read.

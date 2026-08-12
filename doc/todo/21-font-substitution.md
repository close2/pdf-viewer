# What is left of font substitution

Status: reported at runtime; four distinct gaps — the first still **empty of witnesses**, the second unchanged, the third **characterised, fixed and re-measured** (ADR 0270), the fourth measured and declined.
Priority: 21
Corpus: 40 documents. **The corpus gate's own two silence lines, read in the
four-hundred-and-forty-fifth**: `codes reaching no glyph *in silence*: 5 over 2 documents`
(`issue14821.pdf` 3, `issue4650.pdf` 2) and `codes reaching a glyph the font draws blank: 57 over
9 documents` — the split ADR 0270 drew, and the first of the two is what §3 below is about.
`doc/HANDOVER.md`'s not-implemented table said **24 over 8** for that first line until that round
read the gate.
Clauses: §9.10.2, §9.7.4.2, §9.6.2.2, §9.6.5.4, §9.8.1, §9.8.3
Code: `crates/pdf-font/src/substitute.rs`, `crates/pdf-font/src/lib.rs`, `crates/pdf-model/src/content.rs`

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

## 3. A font is reported as a whole — measured, and the silence was half spaces

`FontError` and the "drew nothing" tally are the only channels a font has, so a font that maps
*some* of its document's codes draws those and says nothing about the rest. ADR 0152 measured the
alternative — reporting every uncovered code named 13 documents that mostly draw fine, and each
report costs the oracle a judged page (trap 11) — and chose "drew none" deliberately.

`Interpretation::codes_without_a_glyph` is the measurement that question needs, and the
four-hundred-and-thirty-fourth session found it was counting two different things (ADR 0270).
`LoadedFont::outline` answers `None` both where the routes of §9.6.5.4 and §9.7.4.2 **reached a
glyph the program contains** and describes with no contours — which is how every sfnt stores a
space — and where they reached **nothing, or `.notdef`**. Only the second is a mark the reader
loses, and `codes_reaching_a_blank_glyph` now holds the first.

**The split, over ADR 0269's 65 944 web documents and over the 974:**

| | web codes | web documents | corpus |
|---|---|---|---|
| a glyph the font describes as empty — not a mark missed | **28 837** | 359 | 57 over 9 |
| no glyph, or `.notdef` — a mark lost, before the fix | **22 435** | 277 | 5 over 2 |
| ADR 0269's total | 51 272 | 635 | 62 over 10 |

`pr12564.pdf`'s 26 — the corpus's largest contributor, diagnosed by hand in the
two-hundred-and-forty-fifth session — are the first row, and so is the web's largest:
`0300276.pdf` shows one `Identity-H` code 118 times whose `/ToUnicode` maps it to U+0007 and whose
glyph the font contains and draws blank. The whitespace exemption in front of the count cannot see
a font that reads its own space back as something else; the glyph index can.

**And the second row had one mechanism in it.** Five documents were 4912 of the 22 435 and all
five are Cyrillic through a standard-14 name: ten of §9.6.2.2's fourteen compiled-in faces are
Foxit's bare CFF, whose charsets carry the standard Latin character set and nothing else, so a
`/Differences` naming `afii10017` reached no glyph while the Latin codes of the same font drew.
`substitute_face` now replaces a substituted face with one of the same family whose code table
over Table 109's declared codes is a strict superset, and the population goes **22 435 codes over
277 documents to 780 over 236** — with the web's reports 1144 → 1138, six of them leaving because
the font they named now draws — so ADR 0269's second-largest reported population, a font with no
outline for any code the page shows, is **261 → 255 of 65 944**.

**So ADR 0152's trade holds, and this is the number rather than the opinion.** Reporting every
uncovered code would name 236 documents of 65 944 and 2 of the 974 to account for 780 codes, 231
of those documents losing four codes or fewer. What is left is characterised rather than counted:
a content stream showing **character code 0**, which reaches `.notdef` and reads back as U+FFFD or
U+FFFF — the mode of the distribution, 109 documents losing exactly four codes and 98 losing two;
an **embedded** subset missing a code it is asked for (`3867739.pdf`'s `$` and `>`, 43 of them);
and `4728077.pdf`'s 54, one `Identity-H` code 0 reaching CID 0. Nothing left has more than two
documents behind it.

**`issue14821.pdf` is the corpus witness and it splits the way its ledger rows say.** Five of its
eight are `Identity-H` CIDs whose `loca` entries are empty by the glyph table's own statement —
the first row above — and three are ASCII codes in a nonsymbolic `TrueType` subset whose `(3,1)`
`cmap` maps all three to glyph 0 and whose `post` is version 3.0 with no glyph names at all. Every
route §9.6.5.4 and §9.7.4.2 state ends at nothing. The refusal is on the handover's
closed-by-decision list; `poppler` draws them from a face this machine has.

**What this section still owes** is §1's chain: the superset rule takes one face or none, so a
document whose encoding no single face of its family covers still loses whatever the face in hand
lacks.

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

# 2147 — A font whose shipped face is the other kind is restated as the face's kind

Session 1655, the archive slot of batch one hundred and two. Status: **accepted**, **built** in
`pdf-transform`. It **extends** ADR 2146's construction to "other font embedding", the brief's
second group. Context: ADRs 1209, 2146; `doc/questions/A47`. Code: `described::restated_kind`,
`described::restatement_proved`, the `restated` arm of `fonts::embed_faces`.

## 1. The sub-reasons, and the largest

The group is the refusals of `fonts/font-programs-embedded` other than the missing descriptor and
the composite font. The census before (`doc/history/1655`) named five sentences. Two were the same
fact: §9.9's Table 124 admits the shipped face under no key of the dictionary's own `/Subtype`
(pdf.js 7 + 2, `doc/corpora` 59 + 6 documents at 2b). The other three reach one document each.
Round 1655's census of fonts with a descriptor and no program shows the shape. Most are `TrueType`
dictionaries naming Times New Roman or Courier New, whose shipped face is one of Foxit's CFF
programs (`TimesNewRomanPSMT` alone is 43 documents). A few are `Type1` dictionaries naming
Helvetica, whose face is a Liberation `glyf` program.

## 2. Decision

**The dictionary is restated as the face's kind, and the file's own descriptor receives the face.**
`TrueType` with a CFF face becomes `Type1` with `/FontFile3 /Type1C`. `Type1` with a `glyf` face
becomes `TrueType` with `/FontFile2`, and its encoding is restated on `WinAnsiEncoding` where ISO
19005-2 section 6.2.11.6 does not already admit it, as ADR 2146 does. §9.6.3 lets the two
dictionaries hold the same entries, so nothing else in the dictionary moves. The producer's
`/Widths` and descriptor stay theirs, but for the key the program goes under.

The same proof as ADR 2146's holds it to the page. The restated dictionary, the file's descriptor
without `/CharSet` and `/CIDSet`, and the program about to be written (route 1's own, or route 2's
restated one) are read back through `LoadedFont`. They are compared with the source font code by
code: outline up to winding, advance, and scale. The source must be drawn from the very face being
embedded. A face of the operator's (`--font`) is not restated, and keeps Table 124's refusal with
its advice to name a face of the admitted kind.

## 3. Result

At 2b under `as-if-printed`, both Table 124 sentences go to zero on both corpora. Together with ADR
2146: pdf.js 618 → 750 converted, `doc/corpora` 179 → 260. **What is left of the group** is the
font no shipped face covers (pdf.js 5, `doc/corpora` 9), the restatement a program cannot carry
(1), and two widths for one glyph (1, the fence). The next build is ADR 2146's machine-face
refusal (`doc/corpora` 36). Its answer is to decide whether a page this machine draws from its own
face may be archived with the shipped one. That is the as-shown profile's question rather than a
font question.

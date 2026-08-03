# What is left of font substitution

Status: reported at runtime; three distinct gaps.
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

The general case needs a report where a glyph is *shown*, which needs `LoadedFont` to distinguish
"this code has no glyph" from "this code's glyph is blank", which a space legitimately is. The
whitespace-readback test added in ADR 0157 is half of that distinction already.

Not hard; not done; measure the volume first.

# 1192 — The rest of Table 124, and the descendant a composite font's program belongs in

## What was built

- **§9.9's Table 124 as one function, read in the writing direction** (ADR 1221). `key_for` stated
  two of the table's rows and refused everything else; it now states all of them, and
  `pdf_font::embedding` reads an sfnt's own table directory and asks its `CFF ` table's Top DICT
  whether it uses CIDFont operators, because that is what the `OpenType` row turns on and a
  producer's `/Subtype` cannot settle it. Two of the table's distinctions are load-bearing:
  `MMType1` is in the `Type1C` row and **not** in the `OpenType` one, and a `glyf` face has no row
  under a `Type1` or `MMType1` dictionary at all — every key the table opens to those two carries
  Compact Font Format. That last is a refusal with a sentence of its own, because the advice the
  general one gives is wrong: another file of the same face will not do, a CFF-flavoured one will.
- **A composite font's supplied program, into the descendant's descriptor** (ADR 1222).
  §9.7.4.2 puts it there and §9.7.4.3's `/W` and `/DW` are what its advances are restated against;
  Table 115's `/CIDToGIDMap` `Identity` is written beside it, because the entry becomes required the
  moment a Type 2 CIDFont carries a program. The programs written are the ones whose CIDs *are*
  glyph indices — a `CFF ` Top DICT without CIDFont operators, or the identity map — and a CID-keyed
  one is refused on §9.7.4.2's own `CIDSystemInfo` sentence and ADR 1188's fence. §9.9.1 forbids a
  `cmap` under a CIDFont dictionary, so the directory is rebuilt without it.
- **The archive sweep breaks the font site's refusals down by reason and names the documents.**

## Measured

`archive_corpus` under the lock: **PDF/A-2b 474 converted / 136 refused, PDF/A-4 207 / 152** — the
default sweep is unchanged, and has to be, since `--font` is never a default.

The site's refusals, per the census: four documents at 2b and three at 4. **Six of the seven convert
when `--font` names a program Table 124 admits**, each output held to its target again with every
requirement met: the two composite pairs are this round's build, and the two `MMType1` dictionaries
take a *bare CFF* under the row that was already written — which corrects ADR 1209 section 3, whose
"closes none of them" was measured with `glyf` faces only. The seventh states no `/FontDescriptor`,
which `--font` names none for. Table 124's `OpenType` row is exercised on
`6-2-11-4-1-t01-fail-a.pdf`, which converts either way and now takes a CFF-flavoured OpenType face
as `/FontFile3` `/Subtype /OpenType`.

## Left undone

Nothing this contract named. The composite route writes `Identity-H` and `Identity-V` only; every
other case is refused with the clause that refuses it rather than passed over.

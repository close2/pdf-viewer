# 3137 — A romanised Hei, Gothic, Dotum or Gulim name is a sans request

Session 2109. Status: **accepted and built.** Context: ADR 3102 (the serifed CJK names), ADR 3117 (the same styles
read in their own scripts), `crates/pdf-font/src/substitute.rs` (`names_a_cjk_sans`, `family_of`). Clauses: ISO
32000-2 §9.5 NOTE 5, §9.8.2 (Table 121's `Serif`).

## Context

ADR 3117 made 黑体, ゴシック, 고딕, 돋움 and 굴림 sans requests, read before the flags, and said that the romanised
names were not: `SimHei`, `MS Gothic` or `Dotum` over a descriptor with Table 121's `Serif` bit set was a serif
request, drawn by a Song or Mincho face under a name that says the opposite. The standard decides nothing here (§9.5
NOTE 5 leaves the substitute to the processor); the name is what the producer said.

## Decision

`names_a_cjk_sans` is the counterpart of `names_a_cjk_serif`, read in `family_of`'s sans arm with the native words
of ADR 3117: Hei in `simhei`, `yahei`, `jhenghei`, `heiti`, `mhei`; Gothic in the CJK families' names (`msgothic`,
`mspgothic`, `msuigothic`, `yugothic`, `hirakaku`, `kakugo`, `kozgo`, `malgungothic`, `nanumgothic`, `hygothic`,
`adobegothic`); `dotum`, `gulim`; and `meiryo`, every member of which is a gothic face.

Not the bare words. `gothic` names a sans face in `Franklin Gothic` but a monospaced one in `Letter Gothic`, and this
list decides nothing about Latin type; `hei` is a syllable of `Fahrenheit` and `Heidelberg`. The serif arm is still
read first, as before, and no serif name contains a sans word (the test asserts both ways).

## Consequences

A romanised CJK sans name over a `Serif` flag or a serif PANOSE number now draws in the sans face of its region.
Pages that move are those whose page-one font is unembedded (or its program unreadable), named so, and classified
serif by PANOSE or `/Flags`; the round's census of the golden list names them in its record.

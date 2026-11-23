# 3409 — A font stating no `/Subtype` is a finding, and its program decides the kind written

Session 2177. Status: **accepted** and built. Context: ADR 2608 (a font by where it stands), ADR
3376 item 4, ADR 2652's `T02-04-01_005` row; ISO 32000-2 §9.6.2.1 Table 109, §9.6.4 Table 110,
§9.7.4.1 Table 115, §9.7.6.1 Table 119, §9.9 Table 124; ISO 19005-2 section 6.2.11.2, ISO 19005-4
section 6.2.10.2.

## Why this validator passed it

`T02-04-01_005` holds one font, `<< /BaseFont /Times-Italic /Type /Font >>`, shown on its page. Every
font row asked a question only a kind answers, and `font-programs-embedded` skipped a font with no
`/Subtype` outright, so nothing judged it; the part 4 output kept it unembedded. veraPDF counts the
dictionary as no font at all and fails 6.2.2-3 (an undefined font resource) at part 4.

## Decision

1. **The row** `fonts/font-dictionary-states-its-subtype`: a font by position (ADR 2608) whose
   `/Subtype` is absent, not a name, or a name none of the four tables gives, fails both parts'
   font-types sections. A dictionary that is a font by its `/Type` alone is not judged: the first
   census found a catalog and a page stating `/Type /Font` (`T02-01_006`, `T02-03_002`), whose
   fault is their own row's. A font with no kind is no Type 3 font, so `font-programs-embedded` now
   binds it like any other.
2. **The kind written is the program's.** `Rewrite::FontSubtypeStated` (`archive::font_subtypes`)
   writes the kind Table 124's filing of the embedded program decides: `/FontFile` `Type1`,
   `/FontFile2` `TrueType` (`CIDFontType2` as a descendant), `/FontFile3 /Type1C` `Type1`,
   `/CIDFontType0C` `CIDFontType0`, `/OpenType` by its outlines.
3. **A font embedding no program takes the kind of the face embedded for it.** The substitution's
   `described` route already writes the face's `/Subtype`; it now accepts a font stating none,
   with this viewer's layout and the loss `layout-from-this-viewer`, because §9.6.2.2's fourteen
   are Type 1 fonts and a dictionary that names no kind is not one. `font_subtypes` checks that
   the substitution reached every such font and refuses by name otherwise.
4. **A font no face was embedded for, with no program and no descriptor, naming one of §9.6.2.2's
   fourteen** is given `Type1`: the substitution embeds a face for every such font a page renders,
   so this one draws nothing (`poppler-742-0-fuzzed`'s `/Subype` typo), the fourteen are Type 1
   fonts, and this viewer lays a font of no kind out by the same published metrics.
5. Where the substitution photographed a page, an undecided font is left to the output's
   re-check, since the page may no longer name it (`T02-04-01_006`'s `/Type7`, photographed in
   session 2173 and passing veraPDF).
6. Refused by name: a font nothing decides a kind for (no program, no face embedded, no standard
   name), a multiple master program (Table 124 files it with the Type 1 ones), a
   program a descendant cannot take, a direct font the substitution did not write out.

## Measured

`T02-04-01_005` converts at 2b and 4 under `as-if-printed`, the font written `/Subtype /Type1`
with the shipped face, and veraPDF 1.30.3 passes both outputs; `as-shown` refuses on the layout
loss. Census effect in the round's record.

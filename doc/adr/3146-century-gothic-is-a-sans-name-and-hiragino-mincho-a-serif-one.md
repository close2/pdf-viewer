# 3146 — `Century Gothic` is a sans-serif name, and Hiragino Mincho (`HiraMin`) a serif one

Session 2111. Status: **accepted and built.** Context: ADR 3102 (the serifed CJK styles named), ADR 3117 (a name read
as characters), `crates/pdf-font/src/substitute.rs`'s `family_of` and `names_a_cjk_serif`. Clauses: ISO 32000-2
§9.5 NOTE 5 (font substitution is implementation-dependent), §9.8.2 (Table 121's Serif flag, which the name outranks).

## Context

`family_of` reads an unembedded font's folded name against Latin style words before Table 121's flags. Two names were
read wrong, both left by round 2109:

- `century` is among the serif words, for Century Schoolbook, Century Expanded and Century Old Style; but Century
  Gothic is a geometric sans-serif face, and its name matched `century` first, so a `/CenturyGothic` request was a
  serif one.
- ADR 3102's list of the families every one of whose members is a Mincho names `KozMin`, `Ryumin` and `HeiseiMin`,
  but not Hiragino Mincho, whose PostScript names are `HiraMinPro-W3`, `HiraMinProN-W6` and their kin. The full name
  `HiraginoMincho…` was read through `mincho`; the PostScript name, which is what a `/BaseFont` carries, fell through
  to the flags.

## Decision

`century` is a serif word unless the name also says `gothic`, the sans style's own word, so `CenturyGothic` reaches
the sans words and `CenturySchoolbook` stays serif. This qualifies the one word rather than reordering the branches:
`gothic` alone is not a Latin sans word here, because `Letter Gothic` and `Franklin Gothic` are left to the rest of
the list and the flags as before.

`hiramin` joins `names_a_cjk_serif`'s words. `HiraKaku` (Kaku Gothic) and `HiraMaru` (Maru Gothic) do not contain it,
and the test says so.

Both are choices about what a name already says, as ADR 3102's are; §9.5 NOTE 5 says font substitution is
implementation-dependent, so the words are a documented choice and not a clause's.

## Consequence

No golden page moves (ADR 3145): the one tracked file naming Century Gothic, `issue4722`, states a `CIDFontType2` under
`Identity-H` with no program and no `/ToUnicode`, refused before a family is asked, and no tracked file names `HiraMin` in bytes a search reads uncompressed. Tests:
`century_gothic_is_a_sans_request_and_century_schoolbook_a_serif_one` (its font dictionary is `issue4722`'s name and
flags), and two `HiraMin` names and two Hiragino gothic names in `a_mincho_song_or_myeongjo_name_is_a_serif`.

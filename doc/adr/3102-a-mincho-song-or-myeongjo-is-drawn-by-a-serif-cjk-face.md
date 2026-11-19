# 3102 — A Mincho, Song or Myeongjo font is drawn by a serif CJK face where one is installed

Session 2100. Status: **accepted.** Context: ADR 1441 §5 (no serif CJK face then), ADR 3068 ("Sans for Mincho"),
ADR 3101 (the collection's breadth), `crates/pdf-font/src/substitute.rs` (`names_a_cjk_serif`, `family_of`).
Clauses: ISO 32000-2 §9.8.1, §9.8.2 (Table 121's `Serif`), §9.5 NOTE 5.

## Context

Noto Serif CJK is installed here (`fonts-noto-cjk` ships both families; `fc-list | grep -i "serif cjk"` lists ten
faces). Two things kept every Mincho in Noto Sans CJK. The whole-`cmap` repertoire ranked first (ADR 1441), and the
serif family maps 44 777 characters to the sans family's 44 810, so the serif face could never win. And
`family_of` knew Latin serif names only: `HeiseiMin-W3`, `KozMinPr6N-Regular` and `Ryumin-Light` with no Serif
bit in `/Flags` were sans requests.

## Decision

- The repertoire is counted against the font's collection (ADR 3101). Over Adobe-Japan1's 15 465 characters both
  families map 15 109, and over GB1, CNS1 and Korea1 they tie too; the 59 the sans face adds are supplementary-plane
  ideographs those tables do not name. So the generic family decides, as ADR 1441 meant it to among equals.
- A folded name naming a serifed CJK style is a serif request: `mincho`, `song`, `ming…`/`mingliu`/`adobeming`,
  `batang`, `myeongjo`, and the all-Mincho families `kozmin`, `ryumin`, `heiseimin`, `simsun`. The name decides
  before PANOSE and `/Flags`, as it already did for Latin names; this is a documented choice, §9.5 NOTE 5 leaving
  substitution to the processor.

## Consequences

- Mincho, Song and Ming fonts draw in Noto Serif CJK of their region; gothic, Hei and Dotum fonts are unchanged.
  Pages that move: `90ms_rksj_h_sample`, `issue11555`, `issue13343`, `issue15053`, `issue6286`,
  `noembed-identity`, `noembed-jis7`, and `issue19182` (`PMingLiU`).
- Adobe-KR's table names 47 of the sans family's extra ideographs, so an Adobe-KR Myeongjo still draws in the sans
  face: repertoire before family, unchanged.

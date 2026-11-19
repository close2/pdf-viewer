# 3118 — A collection's breadth is counted in the Basic Multilingual Plane, so an Adobe-KR Myeongjo is drawn serif

Session 2104. Status: **accepted and built.** Context: ADR 1441 (repertoire before style), ADR 3067 (per-character
borrowing), ADR 3101 (breadth counted against the collection), ADR 3102 (Mincho, Song and Myeongjo are serif
requests), `crates/pdf-font/src/substitute.rs` (`Repertoire::breadth`, `ranked_covering`). Clauses: ISO 32000-2 §9.5
NOTE 5, §9.7.5.2, §9.8.1, §9.10.2.

## Context

Over Adobe-KR's `-UCS2` table Noto Sans CJK maps 21 061 characters and Noto Serif CJK 21 014. The 47 are all
supplementary-plane ideographs; over the Basic Multilingual Plane both map 21 001. Breadth ranks before family, so
every Adobe-KR font, a Myeongjo included, was drawn by the gothic face.

## Decision

**The breadth is the count of the collection's BMP characters a face maps** (with no collection, the whole `cmap`, as
before). The ranking is this tree's (§9.5 NOTE 5 leaves substitution to the processor), and the reason is the
characters actually shown: since ADR 3067 a character the chosen face lacks is borrowed from a face that draws it, one
character at a time, so the shortfall costs no blank. What breadth still decides is which face draws most of a line,
and a collection's supplementary-plane characters are its rarest, extensions outside the national character sets the
collections were first built on (GB 2312, Big Five, JIS X 0208, KS X 1001), which lie in the BMP. A face thin in the
BMP still loses: Droid Sans Fallback maps 8 767 of Adobe-KR's BMP characters and stays behind both Noto faces.

Counted on this machine with every catalogue face: for Adobe-Japan1, GB1, CNS1 and Korea1 the order is unchanged
(the Noto families tie in the BMP as they tied overall, and Droid Sans Fallback stays behind); only Adobe-KR's changes.

## Consequences

An Adobe-KR Myeongjo or Batang is drawn by Noto Serif CJK KR, and a page showing one of the 47 borrows it from the sans
face. Adobe-KR's gothic fonts are unchanged.

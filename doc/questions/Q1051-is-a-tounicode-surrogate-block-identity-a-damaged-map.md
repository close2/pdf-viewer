# Q1051 — Is a `/ToUnicode` range that maps the surrogate block to itself a damaged map?

Source: round 2230, the silent-repair census (ADRs 3574, 3621).

## What the tree does now

`pdf_font::tounicode`'s `text_of` reads a destination as UTF-16BE and reads a lone surrogate as
U+FFFD, logging nothing. ISO 32000-2 §9.10.3 states every destination in UTF-16BE, where a
surrogate stands only in a pair, so a lone one is a destination that does not read as written.

Round 2230 marked such a map damaged (`font/to-unicode-damaged`, section 5.1's) and the census
moved nine documents: `basicapi`, `bug1186827`, `bug1473809`, `issue12963`, `issue2074`,
`issue2956`, `issue5677`, `mixedfonts` (pdf.js) and govdocs1 `499039` went from written at
PDF/A-2b to refused there and written at PDF/A-4. Every one carries the same producer pattern, an
identity map over the whole two-byte range in `bfrange`s of 256 (`<d800> <d8ff> <d800>` through
`<df00> <dfff> <df00>`), which states a lone surrogate for every code from D800 to DFFF. veraPDF
reads none of the nine as failing on it. The round reverted the marking and left the site held.

## The ask

Is a range whose destinations fall in the surrogate block a departure the reader logs (and section
5.1 then refuses until a route restates the map — dropping the surrogate ranges would be the
elegant remedy, since no code in them names a character), or is it read past without a class?

*Recommendation: log it, and in the same round have the converter's rewrite of a `/ToUnicode`
drop the ranges whose destinations are surrogates, stated as a loss of no character*, so that
the nine stay written at PDF/A-2b. Until the answer, the site stays in `HELD`.

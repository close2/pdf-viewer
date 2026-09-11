# 957 — Eleven refusals that were only unwritten code

Date: 2026-09-11. ADR: 0957.
Files: `crates/pdf-transform/src/archive/{decision,rewrite,sites,prepare}.rs`,
`crates/pdf-transform/tests/archive_unconsidered.txt`, `doc/pdf-a-mitigations.md` §13.3, §13.3.1.

The first of the three parallel streams. `doc/pdf-a-mitigations.md` §13.3 had just named
twenty-two requirements the converter refused although the right answer loses nothing; this round
built eleven of them — seven rewrites, one routing rule, and `Answer::AsUnderlying` as a fifth kind
of answer for a compound row that takes its decision from the rows it names.

**What is worth carrying is not the eleven but the three catalogue entries they corrected**, since
the catalogue was written a round earlier and was already evidence rather than instruction:

- **The site of a failure is the object a dictionary is *written in*, not the dictionary.** Both
  blend-mode entries and the rendering-intent entry read as though the failing entry sat on the
  object the validator names. It usually does not: a graphics state written directly inside a
  page's resource dictionary is reported at the *page's* object number — so a rewrite that edited
  the named dictionary alone would have written nothing and **reported success**. Both rewrites
  descend the object they are given.
- **`/Intent` is two keys with one spelling.** §8.9.5.1's Table 87 gives an image `XObject` a
  rendering intent; §8.11.2.3 gives an optional content group an `/Intent` of `View` or `Design`.
  A rewrite taking the catalogue at its word would have turned a layer's intent into
  `RelativeColorimetric`.
- **`/CharSet` is deprecated too**, not only `/CIDSet`. §9.8.1's Table 122 deprecates both in PDF
  2.0, which makes *remove* the better of the two lossless routes at every target rather than at
  one — the base standard deciding between two answers ISO 19005 leaves open.

The census ratchet ADR 0955 built ran in the direction it was built for: eleven rows out of
`REFUSED_BY_NAME`, and `archive_unconsidered.txt` held to equality in both directions so that a row
cannot leave the table by being forgotten.

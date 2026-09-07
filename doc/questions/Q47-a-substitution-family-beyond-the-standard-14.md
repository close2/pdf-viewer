# Q47 — Which font family may a substitute come from, beyond the standard 14?

Source: `doc/pdf-a-conversion-limits.md` §4.9 and §10, after the owner overturned the
"refuse a non-embedded font" position on 2026-09-07.
Status: **open** — answered when `A47-a-substitution-family-beyond-the-standard-14.md` exists beside this file.

## Why it needs the owner

Both parts require that only font programs legally embeddable for unlimited universal rendering be
used (ISO 19005-2 §6.2.11.4.1, ISO 19005-4 §6.2.10.4.1). Most fonts installed on a machine are
not: a system Arial's OS/2 `fsType` bits usually permit preview and print rather than the
unlimited embedding the clause requires, and no program can verify a licence from the bytes.

**The common case is already solved and needs nothing.** `data/standard-fonts/` ships Foxit's ten
faces under BSD-3-Clause and four Liberation Sans faces under the SIL OFL, both of which permit
embedding, with `/NOTICE` carrying the obligations and `viewer-ui/tests/notices.rs` checking them.
What is undecided is everything else — a corporate face, CJK, a symbol font.

## What the tree does meanwhile

Substitution at *render* time already happens and is reported per font by `is_substituted()`.
Nothing is written into a file.

## Recommendation

Add one OFL family with wide coverage — the OFL exists to permit exactly this — through the three
steps Liberation already went through: read the licence off a copy, add a row to
`doc/third-party-data.md`, extend `/NOTICE`. Ship nothing whose licence has not been read. Where
no shipped face covers a document's characters, that font falls back to
`doc/pdf-a-conversion-limits.md` §2.1's refusal rather than to a guess.

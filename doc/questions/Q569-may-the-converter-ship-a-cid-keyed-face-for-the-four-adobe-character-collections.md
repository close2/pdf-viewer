# Q569 — May the converter ship a CID-keyed face for the four Adobe character collections?

Source: session 1706 (`doc/history/1706`). The census named there is the evidence.

## The fact

The largest refusal the real corpora meet in pdf.js under both shipped profiles, at both parts'
lowest level, is a composite font the file does not embed and nobody named a program for. About
half of those fonts are `Adobe-Identity`: a CID there is an index into the producer's own program
(ISO 32000-2 §9.7.4.2), so no other face can be given the right glyphs, and the refusal is the
standard's, not ours. The rest name a predefined collection — `Adobe-Japan1`, `Adobe-GB1`,
`Adobe-CNS1` — and §9.7.3 makes the character collection the interpretation of the CID numbers,
so a CID of a collection names the same character in every font of it.
This reader draws those pages today from a face of this machine's, as it draws an unembedded simple
font (ADR 0270). Every face this program ships is a simple font's, so the converter has nothing of
that collection to embed, and refuses.

## Why it needs the owner

A remedy is a shipped CID-keyed program per collection, ordered by that collection's CIDs: for
the four collections ISO 32000-2 §9.7.5.2 names, an open-licensed family such as Source Han (SIL
OFL 1.1) is tens of megabytes per weight. That is a data dependency of a size `doc/stack.md` has
never been asked about, and a licence notice like A550's beside it. Embedding it over the machine's
face would be the same named loss `machine-face` is (ADR 2162): `as-if-printed` discards, `as-shown`
stops.

## Recommendation

Not now. The `Adobe-Identity` half stays refused whatever is shipped, the other half is a dozen
pdf.js test documents and one of `doc/corpora`'s, and `--font <base-font>=<path>` already answers
any of them for an operator who holds the producer's program. Revisit when a corpus of real CJK
documents weighs it.

## What the tree does meanwhile

Refuses each by name, with the sentence that says `--font` resolves it (`fonts.rs`'s
`COMPOSITE_NOT_SUBSTITUTED`). Nothing is built.

# Q737 — Does A569 replace A47's download shape, and is a wide-coverage family beyond CJK still owed?

Source: round 1865, the audit of every answered `A` file against the tree (ADR 2564).

## The question

`A47` (2026-09-09) asks for "the licence-clean OFL family with wide coverage, including a
Noto-CJK-class family", and for "Two downloads. The default download includes the universal font
family", with a smaller one named so that it is unmistakably incomplete. `A569` (2026-10-23) ships the
four Adobe collections' faces "as a second download beside the base program (a CJK pack)", and `A596`
publishes that pack as an asset beside each binary. What is in the tree: the base program carries the
Foxit faces and Liberation Sans (`data/standard-fonts/`), the CJK pack is the second download, and no
wide-coverage family for the scripts outside CJK (Arabic, Hebrew, Devanagari, Thai and the rest) is
shipped. Two things are unclear:

1. **The shape.** Is the base program without the pack now the default download (A569's shape), or is
   the default still the full one and the smaller one named incomplete (A47's shape)?
2. **The scope.** Is the wide-coverage family for the scripts beyond CJK still owed, or did A569's
   answer narrow "universal" to the four collections?

## Why it cannot be settled without the owner

Both are the owner's words and they conflict on the default; neither later answer names A47. A round
choosing one would be deciding what a person downloads.

## What the tree does meanwhile

The release publishes the base binary and the CJK pack beside it (A596). A document showing a script
no shipped face covers is refused by the converter at that site (`crates/pdf-transform/src/archive/fonts.rs`),
as A47 itself asks "where no shipped face covers a document's characters".

## Recommendation

**(1) Keep A569's shape and name the base download as the one without the pack**, which keeps A47's
point — a person who does not choose gets everything — by making the release page offer the full set
(binary and pack) first. **(2) The scope still stands**: a wide-coverage OFL family for the scripts
beyond CJK is owed under the same discipline (licence read off a copy, a `doc/third-party-data.md` row,
`/NOTICE`), as a third download or inside the pack; one round of about 100 tool calls takes the census
of the refusals it would free and the family's licence first.

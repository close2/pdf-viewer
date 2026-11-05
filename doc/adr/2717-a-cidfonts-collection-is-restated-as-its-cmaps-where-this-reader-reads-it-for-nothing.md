# 2717 — A `CIDFont`'s collection is restated as its `CMap`'s where this reader reads it for nothing

Session 1951, a single archive round.
Status: accepted and built. In `pdf-transform`: `archive/collections.rs` (`restated`, `unread`, the
refusals `NOT_AN_OBJECT` and `READ_BY_THIS_READER`), `Rewrite::CollectionRestated`
(`collection-restated`, a stated interpretation, `COLLECTION_RESTATED` in `archive/decision/fonts.rs`),
`archive/rewrite/values.rs` (`restate_collection`). Tests: `collections::tests`, three. Context:
`doc/pdf-a-mitigations.md`'s entry for `fonts/cid-system-info-agrees-with-the-cmap` (the refusal this
replaces was `the-fence`), `doc/pdfa-blocking-issues.md` R10, ADR 2693 (a restatement that is not
lossless is named in the history).

## The case

ISO 19005-2 section 6.2.11.3.1 and ISO 19005-4 section 6.2.10.3.1 hold a composite font over a
`CMap` other than the identity ones to one registry and ordering in both `/CIDSystemInfo`
dictionaries, and the `CIDFont`'s supplement at least the `CMap`'s. `pdf.js:issue18117` writes
`Adobe-Identity-0` in its `CIDFontType2` over an embedded `CMap` stating `Adobe-CNS-7`;
`pdf.js:issue9534_reduced` a `CIDFontType0` stating `Atex-Identity` over one stating ` Atex - Identity `
(the strings padded with spaces); `pdf.js:issue11768_reduced` `Adobe-Japan1-3` against `Adobe-Japan1-6`.

The mitigation entry held this a fence: the clause is symmetric, and the file states two collections
without saying which its producer meant.

## The reading

Which one the producer meant is not what decides a restatement here; what this reader reads is. §9.7.3
names the collection, and §9.7.4.2 selects an embedded program's glyph without it — through the
`/CIDToGIDMap` of a Type 2 `CIDFont`, the charset or the glyph index of a Type 0 one. This reader
consults the `CIDFont`'s registry and ordering for three things only (`pdf_font::composite::collection_names`):
§9.10.2's step (b) table from CIDs to Unicode, the vertical forms of a collection set vertically, and
§9.8.3.3's `/FD` glyph classes; it never reads the supplement, which §9.7.3 says "shall not be used in
determining compatibility between character collections". With the program embedded, no `/FD`, and
neither collection one it holds a table (or, set vertically, vertical forms) for — or both naming the
same collection — the entry is read by nothing that draws a glyph or extracts a character.

## Decision

There, the `CIDFont` is given the `CMap`'s registry, ordering and supplement, as a direct dictionary
(an indirect one may be shared). The `CMap`'s side is not the one moved, because the reader reads the
`CMap`'s codes; the `CIDFont`'s is the statement nothing consults. It is a `Stated` answer, not a
mechanical one: the `CIDFont` no longer says the collection it said, and the conversion's sentence says
so. Everywhere else the refusal names which of the conditions failed: a font that is no object, or a
collection this reader reads.

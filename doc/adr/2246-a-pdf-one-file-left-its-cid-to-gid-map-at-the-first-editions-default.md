# 2246 — A file labelled PDF 1.x left its `/CIDToGIDMap` at the first edition's default

Session 1706, the archive slot of batch one hundred and nine. Status: **accepted**, **built** in
`pdf-transform`. It **amends ADR 0955**'s part 4 half of `fonts/cid-to-gid-map-present`, which read
the answer off the *target's* base document and so refused every absent entry at part 4. Code:
`sites::cid_to_gid_maps`, `decision::CID_TO_GID_REINTERPRETS`. Context: `doc/todo/72` item 6,
`doc/pdf-a-mitigations.md`'s entry for the row, `doc/history/1706`.

## The reading

ISO 19005-2 section 6.2.11.3.2 and ISO 19005-4 section 6.2.10.3.2 both require an embedded Type 2
CIDFont to state the entry, as a stream or as `Identity`. ISO 32000-1's table gives the entry
`Identity` as its default; ISO 32000-2's Table 115 makes it "(Required for Type 2 CIDFonts with
embedded font programs)" and gives no default. ADR 0955 drew the line at the target: part 2 is
built on ISO 32000-1, so writing `Identity` there restates the base document, and part 4 is built on
ISO 32000-2, so the same write would invent a mapping.

That asks the wrong document. What an absent entry *meant* is a question about the source, and
ISO 32000-2 §I.2 answers which edition describes the source:

> A PDF file is labelled with the version number from the PDF standard that describes the set of functionality used by the PDF file.

and the same subclause says ISO 32000-1 contains PDF 1.0 through 1.7. A file labelled 1.7 or
earlier is therefore one whose absent `/CIDToGIDMap` says `Identity`, by the standard that describes
it. A part 4 conversion relabels it PDF 2.0, and a relabelling has to carry across every default
the new label withdraws, or it changes what the file says. Writing `Identity` is that carrying —
the producer's own statement, spelled in the edition that has no default — so it is `Stated` under
both shipped profiles, with no loss.

## Decision

- **Part 2**: unchanged — the entry is written wherever it is absent.
- **Part 4**: the entry is written where it is absent **and** `Document::version` — the header
  raised by any revision's catalog `/Version` — is below 2.0.
- **Still refused**, by the one sentence `CID_TO_GID_MAP_NOT_DERIVED`, which now names the label:
  a file labelled 2.0 or later, or labelled with no version at all (it never said which edition
  describes it); and an entry that is present and is neither shape, which is the producer's own.
  Deriving a map from the embedded program stays a reading of the font this converter does not
  make.

This reader already draws an absent entry as the identity (`pdf_font::composite`), so no glyph
moves: the converted pages render byte for byte as the sources did (`doc/history/1706`).

## What it costs, and what it does not decide

Nothing at part 2. At part 4 it is one comparison of a version the document computes on demand.
It does not extend to any other default ISO 32000-2 withdrew; each such row is read on its own
clause, since some 1.x defaults were withdrawn because PDF 2.0 changed the behaviour rather than
the spelling.

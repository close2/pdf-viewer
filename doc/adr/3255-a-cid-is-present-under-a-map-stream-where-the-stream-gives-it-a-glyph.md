# 3255 — A CID is present under a map stream where the stream gives it a glyph the program holds

Session 2138, an archive round in the `lane3` worktree. Status: **accepted** and built. Settles
the reading ADR 3239 left open. Context: ADR 3239; `crates/pdf-archive/src/table/fonts.rs`
(`cidset_lists_every_cid_in_the_program`, `present_under_a_map_stream`). Clauses: ISO 32000-2
§9.7.4.2 and Table 115, §9.7.6.3, §9.8.3.1 Table 124; ISO 19005-2 section 6.2.11.4.2.

## Context

Section 6.2.11.4.2 asks a `/CIDSet` to identify every CID present in the font program, and Table
124 calls it a stream identifying which CIDs are present in the CIDFont file. A `CIDFontType2`
program has glyph indices only; `/CIDToGIDMap` relates them. The check passed over a map stream.

## The reading

Table 115 gives every CID whose two bytes the stream holds a glyph index — "the glyph index for a
particular CID value c shall be a 2-byte value stored in bytes 2 × 𝑐 and 2 × 𝑐 + 1" — and sets no
value apart. §9.7.6.3 names the one absence a map stream has: "if a (character) code does not
have a corresponding GID in the CIDtoGIDMap stream, the glyph for CID 0 shall be substituted". So
a CID is present where the stream holds a whole index for it and the program holds a glyph at that
index, glyph 0 included; a CID past the stream, or sent past the program's last glyph, is not.
This is read from the two clauses, not chosen.

## Decision

The check judges a `CIDFontType2`'s `/CIDSet` under a map stream by that reading, with CID 0
judged as under the identity map. A `CIDFontType0` stating a map stream is still passed over:
Table 115 states the entry for Type 2 alone.

## What veraPDF does differently

Read in veraPDF 1.30.3's source (`CIDFontType2Program.containsCID`,
`GFPDCIDFont.getcidSetListsAllGlyphs`): it agrees on CIDs other than 0 — every CID below the
stream's length whose index is below `numGlyphs` — and skips CID 0 in both directions. It also
fails a set marking a CID the program lacks, which this check does not ask. ADR 3239's route,
which leaves the `/CIDSet` out where the readings parted, stays: removal is lossless, and the
restated set it would otherwise write is now judged by this reading.

# 3239 — A CIDSet is left out where a map stream sends a CID other than 0 to glyph 0

Session 2134, an archive round in the `lane3` worktree. Status: **accepted** and built. Context:
ADR 3237 (where it was found), ADR 2556 (the `CIDToGIDMap` restated as a stream), the mitigation
entry `fonts/cidset-lists-every-cid-in-the-program` (removal is one of its two lossless routes);
`crates/pdf-transform/src/archive/fonts/cid_notdef_glyph.rs` (`set_removed`). Clauses: ISO 32000-2
§9.7.4.2 and Table 115, §9.8.3.1 Table 124; ISO 19005-2 section 6.2.11.4.2.

## Context

Section 6.2.11.4.2 asks a `/CIDSet` to identify every CID "present in the font program". A
`CIDFontType2` program has glyph indices, and a map stream relates the two; Table 115 states the
stream's layout and nothing about an entry of 0. Two readings follow. This tree's: a CID is
present where the map reaches a glyph other than 0, which is how the restated set was written.
veraPDF 1.30.3's (`CIDFontType2Program.containsCID`, read in its source): every CID below the
stream's length whose glyph index is below `numGlyphs`, glyph 0 included. On the source of ADR
3237 the two disagree, and veraPDF fails the output at 6.2.11.4.2 while this tree's checker, which
passes over a stream map, says nothing.

## Decision

Where the map the route writes sends a CID other than 0 to glyph 0 — the shape where the readings
part — the descriptor it writes states no `/CIDSet`. Table 124 makes the entry optional, ISO
32000-2 deprecates it, and part 4 states no rule for it; removal loses nothing the program does
not state. Which reading is the standard's is not decided here: §9.7.4.2 does not settle it, and
this decision does not need it to.

## Not changed

`pdf-archive`'s 6.2.11.4.2 rule still passes over a stream map; judging one needs the reading
above settled first.

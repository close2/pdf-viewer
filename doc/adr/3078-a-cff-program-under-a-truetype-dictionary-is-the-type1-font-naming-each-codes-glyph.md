# 3078 — A CFF program under a TrueType dictionary is the Type1 font naming each code's glyph

Session 2094, an archive round in the `lane4` worktree, on the remedy the re-taken census of
`doc/pdfa-blocking-issues.md` ranks first among those still to build. Status: accepted and built
for a font no TrueType row also binds. In `pdf-font`: `embedding::wrapped_cff_glyph_names`. In
`pdf-transform`: `archive/font_files/type1_names.rs` (`encoding`, `as_type1`), `Retyping::ToType1`
and `Refiled::to_type1` (`archive/font_files.rs`), the dictionary's rewrite in
`archive/rewrite/values.rs` (`retype_cid_font`). Tests: `crates/pdf-transform/tests/archive/
truetype_over_cff.rs` (four), `font_files::tests::a_program_no_row_admits_keeps_its_refusal`.
Context: ADR 2307 (the refiling, and this case's refusal), ADR 2326 and ADR 2342 (the two
retypings that came before, and their shape), ADR 2490 (the blocks reopened). Clauses: ISO
19005-2 section 6.2.11.4.1, ISO 19005-4 section 6.2.10.4.1; ISO 32000-2 §9.9 (Table 124), §9.9.1,
§9.6.5.1 (Table 112), §9.6.5.2, §9.6.5.4.

## Context

Table 124's `FontFile3` `OpenType` row admits a program whose `CFF ` table uses no `CIDFont`
operators under a `Type1` or a `CIDFontType0` dictionary, and under no `TrueType` one. ADR 2307
refused such a program under a simple `TrueType` dictionary because the only true filing changes
the subtype, and §9.9.1 changes the glyph-selection route with it: §9.6.5.4's `cmap` route for a
TrueType font, §9.6.5.2's glyph names for "Type 1 fonts using "CFF " tables". The census of
session 2094 ranks the row this answers (`fonts/font-programs-filed-by-their-format`, two
documents, each alone) first among the remedies left to build at both parts.

## Decision

1. **The retyping is made where it selects the same glyphs.** Every code this reader drew a glyph
   for is given, in a new `/Differences`, the CFF charset name of that glyph (`.notdef` for glyph
   0); a code that drew none is left to `/BaseEncoding /StandardEncoding`, stated so that the
   program's built-in encoding is not what decides it. The dictionary becomes `Type1`; the
   program moves to `/FontFile3` `/OpenType` by ADR 2307's existing move; `/Widths`, `/FirstChar`,
   `/LastChar` and `/ToUnicode` stay the producer's.
2. **The proof is this reader's.** The retyped dictionary is loaded over the same program and
   every one of the 256 codes compared for glyph and advance; any difference refuses. A glyph a
   charset names twice is reached by the first name only, so a second glyph of the same name has
   no name and refuses. Nothing is removed, so the rewrite is `Mechanical` under every profile.
3. **A font a TrueType row also binds keeps a refusal of its own.** ISO 19005-2 section 6.2.11.6's
   rows (the `cmap` rows and the three encoding rows) bind TrueType fonts alone and end with the
   retyping, but their rewrites run on the same font and write it as a TrueType font again. Until
   the retyping is taught to end them, such a font is refused by that sentence. `issue215`, the
   corpus's witness, is one: its `/Differences` names small-capital glyphs outside the Adobe Glyph
   List, so it moves from the row's old sentence to this one.
4. **The old sentence stays for what no row admits**: a Type 1 program under `CIDFontType0`
   (`issue11740_reduced`), whose true filing would change the program's format.

## Consequences

The fixture's font converts at 2b and 4 under `as-shown`, `as-if-printed` and `refuse-any-loss`,
our validator passing the output. No census document is freed yet; what frees `issue215` is the
TrueType rows answered by the retyping (their findings at a retyped font dropped from the input
their preparations read, `prepare.rs`'s TrueType routes), which is the next step of this row.

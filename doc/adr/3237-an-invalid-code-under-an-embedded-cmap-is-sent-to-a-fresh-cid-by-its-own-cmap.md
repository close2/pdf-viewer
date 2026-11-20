# 3237 — An invalid code under an embedded CMap is sent to a fresh CID by its own CMap

Session 2134, an archive round in the `lane3` worktree. Status: **accepted** and built. Answers
`doc/pdfa-blocking-issues.md`'s R03a entry for `issue11768_reduced` with the construction session
2127 wrote into it. Context: ADR 2557 (a shown CID 0 references `.notdef` whatever glyph it
reaches), ADR 2622 (`Identity-H` restated to send code `<0000>` to a fresh CID), ADR 2556 (the
copy of glyph 0 per width); `crates/pdf-transform/src/archive/fonts/fresh_cid.rs`
(`planned_for_invalid`, `invalid_codes_sent`, `renamed`), `cid_notdef_glyph.rs` (`own_cid`,
`proved`), `notdef_glyph.rs` (`metric`); `pdf_font::Code::in_codespace`. Clauses: ISO 32000-2
§9.7.6.3, §9.7.5.3, §9.7.4.2, §9.10.2; ISO 19005-2 section 6.2.11.8, ISO 19005-4 section 6.2.10.9.

## Context

`issue11768_reduced` shows `<e0e0e0636d>` in an `Adobe-Japan1` `CIDFontType2` encoded by an
embedded `UniJIS-UTF8-H` and stating no `ToUnicode`. `e0` begins the range `<e08080>`–`<efbfbf>`
and the second `e0` leaves it, so §9.7.6.3 calls the code invalid, consumes three bytes (the best
partial match's length) and draws CID 0: the `.notdef` the two parts forbid. ADR 2622's route
restates `Identity-H` only, and a map keyed by CID cannot move a code that has no CID.

## Decision

1. **The font's own CMap is restated**: a codespace range holding exactly that code and a
   `cidchar` sending it to a fresh CID, written after the file's first `endcodespacerange`. Every
   code beginning with the same bytes but another took three bytes before and still does, since a
   longer partial match of the new range has the same length; the proof splits every shown
   string again with both fonts and requires the same codes.
2. **The fresh CID** lies past every CID shown, every CID the map reaches, every CID `/W` states,
   every CID the CMap selects and, for a collection §9.10.2's third method reads by CID, every CID
   the collection's `-UCS2` CMap maps — so text extraction reads nothing for it, as it read nothing
   for the invalid code. The proof compares the extracted text of every code.
3. **The CMap's name is restated** (`FreshCid-<n>-<g>-<name>`, in the dictionary and in the file's
   `/CMapName` definition): its content is no longer that of the CMap its old name registers. A
   file naming another CMap to build on, a vertical one, or one not defining its stated name
   keeps the refusal.
4. **A bearing a metrics array stops short of is read as 0**: the program's `hmtx` holds 398
   long metrics and no bearing for its other 9 562 glyphs, which the copy of glyph 0 must restate.
   The Manual sizes the array by the glyph count and says nothing of a shorter one; `read-fonts`
   answers no bearing, and `skrifa`, which draws the window's outlines, reads it as 0, so 0 is
   what the window drew each glyph by and what the restated table states.

ADR 3239 is the `/CIDSet` this construction leaves out.

## Result

`issue11768_reduced` converts at 2b and 4 under both shipped profiles; veraPDF 1.30.3 passes all
four; page 1 at 150 dpi is pixel-identical to the source's.

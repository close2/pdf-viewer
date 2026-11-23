# 2186 — A name-keyed CFF written CID-keyed, text with no font paints, and the census triaged

Session 2186, an archive round in the `lane3` worktree: the 63 outputs of 19 documents veraPDF
failed in round 2181's census. ADRs 3445 to 3447; no question. Ledger rows 9.3.1, 9.3.6 and
9.7.4.2 (`implemented` before and after).

**Triage (ADR 3447).** Every document is in `doc/pdfa-blocking-issues.md` section 1's table:
three ours (two built below, ADR 3446's one), twelve veraPDF's, two the document's
(`issue14462_reduced`'s program, `PDFBOX-3148-2-fuzzed`'s ASCII85 stream with no EOD), and
`219789`, `issue19326` known. Four departures plainly veraPDF's are drafted, not filed, in
`doc/upstream/verapdf-1.30.3-departures.md`: the 6.2.11.3.1 profile compares the supplements
`<=` where the clause asks `>=`; `re%` read as one operator (§7.2.3); a rectangle of indirect
numbers read as zero (§7.3.10); a CFF encoding supplement read other than as TN 5176's SID.
`lenIV -1`, a format 12 `cmap`, `facct2025`'s exact `/CharSet` and `029016`'s inline image are
recorded as veraPDF's and not drafted. No valid program is restated to suit veraPDF.

**`T02-05-01_004`, `_008` (ADR 3445), ours.** Text shown before any `Tf` paints nothing here, and
the survey returned before recording the paint; section 6.2.4.3 asks which space the operator
uses, so the use is recorded and the existing output intent remedy reaches both. All four
outputs pass veraPDF.

**`issue7901` (ADR 3446), ours.** New row `fonts/cidfonttype0-glyphs-selected-by-cid` (6.2.11.2 /
6.2.10.2, §9.7.4.2); `pdf_font::cff::cid_keyed_by_map` writes the name-keyed CFF as a CID-keyed one
whose format 0 charset gives each CID the glyph the `/CIDToGIDMap` gave it, charstrings and
subroutines byte for byte, each CID proved to draw the same outline at the same width;
`Rewrite::CidKeyedProgramWritten` files it as `CIDFontType0C` and drops the map. Both outputs
pass veraPDF; page 1 renders identical at 144 dpi.

**Census.** With and without the pack, every line equals 2181's, and the kept outputs are 2181's
path for path. Fifteen change size: the twelve outputs of the three documents above,
`T02-05-01_006` at 2b and 4 (text before any `Tf`, now with an output intent), and `507676` at 4,
whose photographed page's text changed with 2ae60ea3e (ADR 3429), which 2181's census predates.
veraPDF: 5516 written as conforming, 51 failing outputs of 16 documents over 16 rules (was 63, 19,
16), 20 unfinished; the three documents above left the list and none joined it.

**Gates.** Over `-p pdf-archive -p pdf-transform -p pdf-font -p viewer-qt`: fmt 0; clippy 0;
`pdf-sandbox` build 0; nextest 2186 passed, 0 failed (manifest difference none); rustdoc 0; `fuzz`
check 0; conformance 478 passed, 0 failed after two §9.7.4.2 quotes were made verbatim.

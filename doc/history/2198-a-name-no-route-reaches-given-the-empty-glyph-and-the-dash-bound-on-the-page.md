# 2198 — A name no route reaches given the empty glyph, and the dash bound on the page

Session 2198, an archive round in the `lane3` worktree: the three builds ADRs 3473 to 3475 left
named. ADRs 3493 to 3496. Ledger rows 8.4.3.6, 9.6.5.2, 9.6.5.4 and 7.4.3 (`implemented`), before
and after.

**issue13316_reduced (ADR 3493).** `face_widths` gives the empty glyph to a code the file's route
takes to no glyph (ADR 2880's population) as to a code on `.notdef`. The window draws the font from
this machine's `NimbusSans-Regular`, an OpenType `CFF ` face, which the empty glyph did not reach:
`pdf_font::embedding::with_empty_cff_glyphs` now appends the glyphs to the `CFF ` table and carries
`maxp`, `hhea`/`hmtx` and `vhea`/`vmtx` with them. The report and history name such codes in a
clause of their own; the producer's `/ToUnicode` is kept. All four outputs pass veraPDF and page 1
renders as the source, PNG for PNG. `issue20489` (a code on `.notdef` under the same kind of face)
converts under `as-shown` with it; `issue13316_reduced` under `as-if-printed` keeps its text
instead of a photograph.

**History (ADR 3494).** The ASCII restatement is prepared before the packet, named in
`xmpMM:History` beside `undecodable-stream`'s entry, and the packet is written whenever it is.

**Dash bound (ADRs 3495, 3496).** `pdf_render::dashing` counts a pattern past `MAX_DASHES` again
over the part of the path the raster can show — the page's grid in `render-cpu`, the target in
`render-raster` and `render-gpu`, the crop box in `pdf-model`'s report — cutting straight segments
to the region grown by a mark's reach and leading each piece in from off the region at the phase
the subpath had there. On `020747`'s page 7 the four strokes of 0.94 to 1.76 × 10⁸ dashes are 1 393
to 7 064 on the page, so the bound no longer stops it: it converts at part 4 under
`as-if-printed`, veraPDF passing. `raster_golden` held 974, moved 0; `drawn_ahead_in_strips` passes.

**Census.** With and without the pack alike: `as-if-printed` corpora part 4 483 to 484 (`020747`);
`as-shown` pdf.js 2b 909 to 911 and part 4 922 to 924 (`issue13316_reduced`, `issue20489`). Kept
outputs are 2193's path for path plus those five; sizes change only for the ASCII restatements'
history entries and `issue13316_reduced` under `as-if-printed`, whose font is now embedded rather
than photographed. veraPDF: 5525 written as conforming, 47 failing outputs of 15 documents over 14
rules, 20 unfinished, the list 2193's. Sections 2 and 9 regenerated: section 9's R08 21 to 18, R09 13 to 12,
R11 4 to 2, R03 47 to 46.

**Gates.** Over `-p pdf-archive -p pdf-transform -p pdf-render -p render-cpu -p render-raster
-p render-gpu -p viewer-qt -p pdf-model -p pdf-font`: fmt 0; clippy 0; `pdf-sandbox` build 0;
nextest 5967 passed, 0 failed (manifest difference none); rustdoc 0; `fuzz` check 0;
`raster_golden` 0; `drawn_ahead_in_strips` 0; conformance 478 passed, 0 failed.

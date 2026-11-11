# 2003 — A CID drawn as nothing keeps its nothing, and a descendant gains a descriptor

Session 2003, a single round, PDF/A lane's worktree. `doc/pdfa-blocking-issues.md` section 3, R09 (a); ADRs
2824 and 2825.

**The premise.** Held for all eight at HEAD (round 1998's census): each refused under `as-shown` and not `as-if-printed`, at both
parts on `fonts/font-programs-embedded`, `issue2128r` also on 6.1.8's names under every profile. A probe of ADR
2807's route read why: seven stopped on a shown CID other than 0 for which the window drew no glyph of
`DroidSansFallbackFull` (no Latin letter in it), no outline and no `.notdef` in place, so nothing at its `/W`
advance; `90ms_rksj_h_sample`'s descendant states no `/FontDescriptor`; `issue13343`'s "descriptor already written"
was the pack's fallback after the first stop.

**Built.** ADR 2824: `pdf_font::embed::for_embedding_with_empty_glyphs` appends empty `glyf` entries after the
kept glyphs; the route sends each such CID by the `/CIDToGIDMap` to one, at its width, never to `.notdef`
(6.2.11.8), reports the CIDs and records them in `xmpMM:History` (A48's condition); no character is stated, the
CID's meaning staying its collection's. ADR 2825: ADR 2461's rule for where the descriptor stands, and a
descendant stating none gets Table 120's entries measured on the subset and the face.

**Freed** under every profile withholding `machine-face` at both parts: `90ms_rksj_h_sample`, `issue11555`,
`issue13343`, `issue19182`, `issue6286`. **Left**: `issue15053` and `mixedfonts`, whose empty CIDs are
proportional Latin (0.604, 0.278 em) that the face's 256-unit em cannot state within 6.2.11.5's thousandth;
`issue2128r`, whose font now converts, on 6.1.8 alone. R09: 21 to 15 at each part.
A832 landed during the round
(`as-shown` admits `glyphs-from-to-unicode` where the proof holds) and was handed to this round; the wall budget
was spent, so the four `Adobe-Identity` documents are left to the next PDF/A round.

**Pixels** (CPU, 72 dpi, `/OutputIntents` renamed, `as-shown`, `refuse-any-loss`, `no-data-loss` at both parts):
`90ms_rksj_h_sample`, `issue13343` (two pages) and `issue6286` 0 px; `issue11555` one pixel by 2 levels and
`issue19182` one by 10, the outlines proved equal once wound (ADR 0396's winding).
**veraPDF 1.30.3**: 74 of 80 outputs PASS, and 18 of the census's 20 new ones; every failure is `issue11555` at 2b
on 6.2.11.3.1-1 alone, veraPDF wanting the CIDFont's supplement (6) at most the `CMap`'s (2) where section
6.2.11.3.1 wants it at least (round 1734, ADR 2302).

**Gates.** fmt 0 and clippy `-D warnings --all-targets` 0 on `pdf-font` and `pdf-transform`; nextest of
`pdf-transform`, `pdf-archive` and `pdf-font` 0 (1863 run); three tests, each failing under its plant;
`cd fuzz && cargo check --bins` 0; archive tier 0 and 0; censuses against 1998 (`cmp.py`): shipped 10 NEW, 0
CHANGED, 0 GONE; stating 10 NEW, 0 CHANGED, 0 GONE; `delta.py` 0 newly refused in either; `rank.py --table`: section
2's table and section 9 replaced whole; `cargo test -p conformance` 0; rustdoc `-D warnings` of `pdf-font` and `pdf-transform` 0; tier 2
(after the report): `corpus` 101, 974 documents, HEAD's 62 incomplete; `raster_golden` 101, HEAD's 16 moved first
pages exactly; `tools/state.sh --round 2003 docs` 0.

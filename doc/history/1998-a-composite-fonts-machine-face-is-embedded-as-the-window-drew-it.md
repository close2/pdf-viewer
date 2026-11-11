# 1998 — A composite font's machine face is embedded as the window drew it

Session 1998, a single round, PDF/A lane's worktree. `doc/pdfa-blocking-issues.md` section 3, R09 (a); ADR 2807;
Q832.

**The premise.** Held for all 26 documents at HEAD: every one refused under `as-shown` at both parts, on
`fonts/font-programs-embedded`. By the losses named: `machine-face` alone on 11 collection fonts (the CJK pack's face,
where the window drew `DroidSansFallbackFull`: `90ms_rksj_h_sample`, `issue11555`, `issue13343`, `issue15053`,
`issue19182`, `issue6286`, `issue8372`, `mixedfonts`, `noembed-eucjp`, `noembed-jis7`, `noembed-sjis`) and two more
also refused on `bound-names-are-valid-utf8` (`issue2128r`, `issue3521`); with `glyphs-from-to-unicode` on four
`Adobe-Identity` fonts (`issue11651`, `issue20065`, `issue2840`, `issue9084`); with `layout-from-this-viewer` on four
fonts with no descriptor (`T02-04-01_001`, `T02-04-01_002`, `issue18099_reduced`, `issue2391-2`);
`layout-from-this-viewer` alone on four (`All Quiet`, `issue6069`, `issue6106`, `issue6108`); and `issue13316_reduced`
on a machine face lacking a shown code. `as-if-printed` converted all but the three with other refusals.

**The face read first.** `DroidSansFallbackFull`: `glyf`, 49 382 glyphs, 4.0 MB, `fsType` 0x0008 (editable).

**Built** (ADR 2807). Where the profile withholds `machine-face`, ahead of the pack and re-keying: the face the
window drew, as a §9.9.2 `glyf` subset under `/FontFile2` and a `CIDFontType2`, a `/CIDToGIDMap` stream sending each
shown CID to the glyph the window drew, advances restated to `/W` (and `/W2` where the face states `vmtx`), held to
the window code by code; any condition it does not meet takes the old route and its named loss.

**Freed**: `issue8372`, `noembed-eucjp`, `noembed-jis7`, `noembed-sjis`, under `as-shown`, `refuse-any-loss`,
`no-data-loss`, `keep-everything`, `only-metadata-loss`, `derive-attachments`, at both parts. **Left**, traced by a
probe of the route's refusals: seven documents show a CID the window drew no glyph for (an empty `glyf` glyph is a
`pdf-font` writer the round did not owe tier 2 for); `90ms_rksj_h_sample` has no descriptor object; the four
`Adobe-Identity` documents are on Q832; the `described` fonts also lose `layout-from-this-viewer`, so section 2 of the
brief frees nothing under `as-shown` and was not taken.

**Pixels** (CPU, 72 dpi, `/OutputIntents` renamed): `issue8372` and `noembed-jis7` 0 px against the source;
`noembed-eucjp` and `noembed-sjis` one pixel by 2 levels, the outlines proved equal once wound one way.
**veraPDF 1.30.3**: 48 of 64 outputs of the four documents PASS; the 16 at 2b of `noembed-eucjp` and `noembed-jis7`,
the pack's at HEAD included, FAIL 6.2.11.3.1-1 alone, where veraPDF wants the CIDFont's supplement (2) at most the
`CMap`'s (1) and section 6.2.11.3.1 wants it at least (round 1734, ADR 2302).

**Gates.** fmt 0 and clippy `-D warnings --all-targets` 0 on `pdf-transform`; nextest of `pdf-transform` and
`pdf-archive` 0 (1473 run); two tests, each failing under its plant; `cargo test -p conformance` 0 once this
paragraph was written; archive tier 0 and 0; censuses against 1996: shipped 8 NEW, 0 CHANGED, 0 GONE, stating 8 NEW,
0 CHANGED, 0 GONE, 0 newly refused in either; R09 26 to 21; rustdoc `-D warnings` of `pdf-transform` 0, the
workspace walk of `tools/state.sh --round 1998 docs` 0 (finished after the report).

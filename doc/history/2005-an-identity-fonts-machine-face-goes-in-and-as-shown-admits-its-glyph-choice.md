# 2005 — An `Identity` font's machine face goes in, and `as-shown` admits its glyph choice

Session 2005, a single round, PDF/A lane's worktree. `doc/pdfa-blocking-issues.md` section 3, R09 (a); owner
answer A832; ADRs 2828 and 2829.

**The premise.** Half held. At HEAD each of `issue11651`, `issue20065`, `issue2840`, `issue9084` refused on
`fonts/font-programs-embedded` alone under the six profiles withholding `machine-face`, at both parts, naming
`machine-face` and `glyphs-from-to-unicode`; `as-if-printed` and `state-every-loss` converted. Admitting the loss
would have freed none: a probe of ADR 2807's route found it never reached an `Identity` font, since `shown` asked
the predefined files for `Identity-H`, which Table 116 states by a rule; and the window drew three of the four from
URW's `CFF ` OpenType faces (`NimbusSans-Regular`, `NimbusSans-Bold`, `NimbusRoman-Regular`, `fsType` 0x0004), which
the route did not take. `issue11651`'s `/FontFile2` is damaged, so the window draws it from metrics alone (ADR 1094).

**Built.** ADR 2828: `Identity-H` and `Identity-V` by Table 116's rule; a `CFF ` face the window drew written by
`pdf_font::embed::renumbered` as a bare CID-keyed subset under `/FontFile3` `/CIDFontType0C`, the descendant a
`CIDFontType0` with no `/CIDToGIDMap` (Table 117), advances restated and the whole held to the window code by code.
ADR 2829: a per-loss key `glyphs-from-to-unicode` at the site, set in `as-shown` alone; only the proved route
names that loss without `machine-face`, so the stop stays wherever the proof did not hold. The loss sentences now
name the machine's face beside the shipped one (`SUBSTITUTION_REINTERPRETS` among them).

**Freed** under `as-shown` at 2b and 4: `issue20065`, `issue2840`, `issue9084`, and `issue5244` (R08's `Adobe-Identity`
font whose character no shipped face draws). **Left**: `issue11651` (no face drawn). Every other withholding
profile still stops, now on `glyphs-from-to-unicode` alone for those three. R09 15 to 12 at each part, R08 35 to 34.

**`factur-x` exit 1** was round 2003's script: `doc/profiles/factur-x.toml` is a departure (RFC 0007 section 4.7),
not one of the eight shipped profiles, and runs as `--config factur-x.toml --depart-from-the-standard`; so run, the
four documents exit 4 on the same site.

**Pixels** (CPU, 72 dpi, `/OutputIntents` renamed): the eight new outputs 0 px from their sources.
**veraPDF 1.30.3**: the eight new outputs PASS, and the two changed `state-every-loss` outputs of `issue2840`.

**Gates.** fmt 0 on `pdf-transform`; clippy `-D warnings --all-targets` 0 (101 first, `row` at 102 lines, then shortened); nextest of `pdf-transform` and
`pdf-archive` 0 (1478 run, 25 skipped); three new tests, each failing under its plant (a charset glyph one index on;
`Identity-H` read as a file; the profile's line deleted); archive tier 0 and 0; censuses against round 2003
(`cmp.py`): shipped 8 NEW (the four documents under `as-shown` at both parts), 0 CHANGED, 0 GONE; stating 0 NEW,
8 CHANGED (`state-every-loss`'s appended page of the four documents: the glyph-choice sentence now names the
machine's face too), 0 GONE; `rank.py --table`: section 2's table and section 9 replaced whole; `cargo test -p
conformance` 0 after `git add -N`; `tools/state.sh --round 2005 docs` 0. No tier 2 is owed: `pdf-font` is untouched.

# 1992 — What the first pass wrote unjudged is converted again

Session 1992, a single round, PDF/A lane's worktree. `doc/pdfa-blocking-issues.md` section 8 (`D-defect`); ADR 2795.

**The premise.** Half true. At HEAD `AIAA-2005-6173-712` converts under all four main profiles at both parts, so
its line was stale. The other five lines held, and the census's `D-defect` row named six documents section 8 did not
(`JA1-1996-PDF`, `JA3-1998-PDF`, `T02-02_002_page-tree-kids-links-recursive`, `hesr13613-sup-0001-Authormatrix`,
`poppler-90-0-fuzzed`, `027613`) and `issue14297` beside `issue14200`.

**Traced.** `bug864847`: the face substituted for a programless symbolic TrueType carries (0,3), (1,0), (3,1), which
ISO 19005-2 section 6.2.11.6 refuses at part 2; the source met the row, so its rewrite was never asked. `JA1`/`JA3`:
`/Prev 576066 0 R` is a reference where ISO 32000-2 Table 15 asks for an offset, the check enumerates the first-page
section only, and the thumbnails' `LZWDecode` reach the output's table. `auth-event-ef-open`: an `/EFOpen` crypt
filter this reader cannot open, copied with `/Filter [/Crypt]`. `issue14200`/`issue14297`: a group's `/CS /DeviceCMYK`
beside sRGB took the output intent, and part 2 prepared no ICC default because §11.6.6's Table 145 forbids its
`DeviceN` one for a group.

**Built.** ADR 2795: ADR 2534's second pass also runs where the output fails a requirement the source met; a
family-less colour row takes the part 4 `DeviceCMYK` row's stand-in press (the inks' profile as a statement, the
shipped press as `press-assumed` under `as-if-printed`); at part 2 that stand-in is built only for a group blending in
`DeviceCMYK`. The report's `settled` sentence names both triggers.

**Freed**, every main profile unless named: `JA1`, `JA3` (2b, 4); `bug864847` (2b); `issue14297` (4; 2b under
`as-shown`, `as-if-printed`, `only-metadata-loss`); `issue14200` (4); `auth-event-ef-open` (4, not `no-data-loss`);
`issue15910` (`state-every-loss` 4). **Left**: `issue14200` at 2b (its soft mask's `/G` has no `/Resources`),
`digitally_signed_3D_Portfolio`, the `uniXXXX` rows, the `T02-02`, operator and transfer-function lines.

**Pixels** (CPU, 72 dpi, pages 1–4, `/OutputIntents` renamed): all 72 pages of the freed outputs under `as-shown`,
`as-if-printed` and `state-every-loss` at both parts are pixel-identical to their sources.
**Tests.** Four in `pdf-transform/tests/archive.rs`, each failing under its plant (the old condition fails two, no
part 2 stand-in one, a stand-in built for marks one); the old `..._stays_refused` test became `..._takes_the_assumed_inks`.

**Gates.** See the hand-off message: fmt, clippy, nextest, conformance, archive tier, both censuses against 1989,
veraPDF 1.30.3 (82 of 82 outputs of the seven documents pass), `state.sh docs`.

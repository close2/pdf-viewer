# 1967 — `DeviceCMYK` beside an RGB intent takes the assumed inks as its `/DefaultCMYK`

Session 1967, a single round, PDF/A lane's worktree. R05 (b), ADR 2747.

**The premise.** Half true. R05's "frees 9 at 4" counts `as-if-printed`, where all nine (`363_Risk`,
`E3_Mayhew`, `LLMImplications`, `NeilJairath_3`, `carp2010-2`, `digitally_signed_3D_Portfolio`,
`me450f12`, `response-to-fiber`, `freeculture`) stop at ADR 2518's overprint sentence (6.2.4.2). (b) is `as-shown`'s: 29 documents
refused at 4 by 6.2.4.3, 19 by it alone, 6 of those also overprint. 2b takes each through part 2's
`DeviceN` default; `digitally_signed_3D_Portfolio` fails 2b's output check.

**Built (ADR 2747).** At part 4 with nothing supplied and `press-assumed` not authorised (every
profile but `as-if-printed`), `supplied_cmyk::prepare` takes ADR 2700's profile as the `ICCBased`
`/DefaultCMYK` (`Press::Inks`), `Decision::Stated` with `INKS_REINTERPRETS`, the history recording
it. §8.6.5.6 passes the values unchanged, so the page is drawn through the `A2B` that samples the
inks the window used: a restatement, no loss. `as-if-printed` keeps GRACoL; part 2 keeps `DeviceN`;
a configured press does not replace the inks (A784); the overprint guard binds them.

**Freed** at 4: under `as-shown` 13 (`0004-637X_751`, `0004-637X_752`, `1742-6596`, `382252…`,
`4387ba…`, `Orellana2022`, `hae14831`, `jne5_4_l03`, `issue10640`, `issue14627`, `issue269_2`,
`issue6413`, `stamps`); `no-data-loss` 11, `state-every-loss` 12 (stating census). The other 16
`as-shown` refusals: the 9 overprint files, and 7 kept by the operator fence, font or annotation rows.
**Pixels** (CPU, 72 dpi, every page of the 13): against the source at most 1 level of 255, with
`/OutputIntents` renamed at most 1, with `/DefaultCMYK` renamed at most 1: ADR 2700's fill bound.

**Tests** (`archive/assumed_inks_default.rs`, five; the mixed-page test of `archive.rs` rewritten,
two refusal tests removed): the inks written, stated and conforming; 324 patches drawn within one
level; a configured press not used; part 2's `DeviceN`; overprint mode 1 refused. Plants: GRACoL in
place of the inks fails three; no part-4 gate fails part 2's; no guard fails the overprint test; no
inks route fails five.

**Census** (against round 1966): shipped 13 NEW (`as-shown/4`), 0 GONE, 0 CHANGED; stating 23 NEW
(`no-data-loss/4` 11, `state-every-loss/4` 12, `mattgib_1` only there), 0 GONE, 1 CHANGED:
`bug1365930` under `state-every-loss/4`, whose `DeviceCMYK` group row now carries the inks
default too, every pixel equal to the old output's. Rank: R05 `as-shown` 4 stops 29 to 9.
veraPDF 1.30.3 passes the 61 CLI outputs on the route, the 36 NEW census outputs and the CHANGED one.

**Gates.** fmt 0 and clippy `-D warnings --all-targets` 0 on pdf-transform; nextest pdf-transform
and pdf-archive 0 (1439 passed); archive tier 0; both censuses 0; `cargo test -p conformance` 0
(471 passed) after `git add -N`, the §14.11.5 row naming the renamed test; `tools/state.sh --round 1967 docs` 0.

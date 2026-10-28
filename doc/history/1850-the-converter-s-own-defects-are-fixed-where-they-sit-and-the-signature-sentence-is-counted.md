# 1850 — The converter's own defects are fixed where they sit, and the signature sentence is counted

Slot 11 of batch 125, 2026-10-28, an "archive" round (Sonnet 5.5, high); ADRs 2534 and 2535 used, Q716 unused. I worked
in `scratchpad/r1850/` and in an export built there, its target deleted; no ledger row moved. Two sessions (the first handed over): 170 and 95 tool uses.

**Premise.** "The sixteen requirements gather into a few defects, each at one site": five causes did (ADRs 2534 and
2535), and five documents remain untraced. The four part-4 refusals "with no
sentence" had one all along: `signature_decision` refuses a signed source with `--authorise signature-assertion allows
it`, a field of the conversion and not a row of `decided`, which the census harness did not read. The count was a
harness blind spot, and the harness reads it now (five documents, `signatures/signature-assertion`, `R16`).

**Built.** A conversion that constructed appearances and would be refused is converted once more over its own output
(`archive/settle.rs`, one extra pass, `Conversion::settled`), whether or not the source met the failing requirement, and
the face it embeds is an `xmpMM:History` entry even where the first output's packet conformed (`wants_metadata`). The
rendering-intent rewrite finds its sites structurally where the validator counted more places than it kept, and
restates `/RI` only in a graphics state (a push button's `/MK /RI` is Table 192's rollover icon). Annotation
dictionaries written directly in `/Annots` get `/F`; a font with a descriptor and no `/Widths` is given the widths the
viewer draws it with. Seven tests, each failing with its fix removed (the intents plants: 32 of 40 restated; 41 places
changed with a button beside 40 states).

**Census** (with the pack, `as-if-printed`, against the same tree before): `output-fails` 23 documents to 5, and the
`D-defect` row from 17 to 20 per profile and target to 4. Written at 2b: pdf.js 838 to 847, corpora 386 to 390; at 4:
841 to 850 and 385 to 387; without the pack 827 to 836 at 2b. No census line wrote fewer. Freed and checked with veraPDF
at 2b and 4 (`failedRules` 0, page counts equal to the sources'): 13 documents, 25 outputs; `NeilJairath_3` at 4 stops at
R05, with its sentence.

**Found, not fixed.** `0004-637X_751_2_90` conforms by our checker and fails veraPDF 6.2.11.5 because
`loading/advances.rs:55` reads no `hsbw` width of a bare Type 1 program (a checker blind spot); `veraPDFHiResWrongObjectID`
(a stream `/Length` of 8700 over 5958 bytes, from headers that disagree);
`AIAA-2005-6173-712` (a page whose `/Font` lacks the `/F3` its content names); `bug864847` (a symbolic TrueType program,
2b only); `issue14200` (a CMYK group with no CMYK intent); `auth-event-ef-open` at 4; the latent 32-place prefix in the
other users of `sites::objects_named`; and the part-4 signature authorisation (ADR 2535 section 4).

**Gates.** `cargo test -p pdf-transform --test archive` exit 0 in the worktree (448 passed, 6 ignored) and in the export
(414 passed, workers rebuilt; the two JPX reds of the first reading were stale workers and pass); clippy `-D warnings`
for `-p pdf-transform --all-targets` exit 0 in both; rustfmt clean on my files. The pack census exit 0 again, lines
identical to the table above and the same five `output-fails`; 25 freed outputs veraPDF `failedRules` 0 (the face
embedded is in `xmpMM:History`). `archive_corpus --ignored` exit 0 (7 passed), `cross_check --ignored` exit 0 (1
passed). `--bin cited` and `--bin quotations` exit 0 at the first reading; `cargo test -p conformance --no-fail-fast`
exit 0. **Validation.** An Opus validator failed the first reading on five findings — the every-object intent remedy rewrote a push button's rollover icon, settled outputs wrote no history entry though the report claimed one, the settling condition missed its own stated cause, a record line misread stale workers as a HEAD failure, and two comments carried history — and passed the round after it fixed all five, its plants re-run with the icon untouched and the entry in the packet.

# 1904 — An OpenType-CFF program's widths are restated in both statements

Batch 131, slot 4 (archive), two sessions on Opus 5.5. ADR 2642.

**What changed.** An OpenType program with PostScript outlines states each advance in `hmtx` and in
its `CFF ` table's charstrings. `restate::with_widths` now restates both — the table through
`cff::with_advances` in its own `FontMatrix` scale, the file laid out again by `sfnt::with_tables` —
and reads every sfnt restatement back (`sfnt::checksums_hold`, each advance through
`restate::advance`), refusing with `RestateError::NotReadBack`. `restate::advance` answers `None`
where the two disagree. The checker's width row judges the charstring width too. At the
orchestrator's ask, `rewrite_dictionary` (past clippy's 100 lines by a sibling's hunk) gives two arms
to `name_the_copied_descriptor` and `substitute_font_program`.

**The probe.** ADR 2630's Greek document with `--font Times-Roman=` an OpenType-CFF Noto Serif,
`as-shown`: at HEAD it converts, the checker passes it and veraPDF 1.30.3 fails 6.2.11.5 with 375
checks at 2b and 4; with the change it converts and passes veraPDF at both, 23 pages drawn as
HEAD's output draws them. The planted failure, HEAD's output, still fails veraPDF (375 checks).
**The plant.** The `CFF ` record's checksum left stale after the restatement: the read-back refuses
and two tests fail with `NotReadBack`; the checker without the charstring reading fails the fixture.

**The read-back's first form.** Summing each table over aligned words refused `issue3323.pdf` and
`363_Risk-Based Management of Groundwater Contamination _Paper.pdf`, whose producers put a `glyf`
program's `head` off the word boundary; the read-back is now `checksummed`'s fixed point. Both
convert again as at HEAD: `issue3323` at 2b and 4 `as-shown`, veraPDF pass; `363` at 2b
`as-if-printed`, veraPDF pass, and refused at 4 and under `as-shown` as at HEAD.


**Gates.** On the export: `cargo nextest run -p pdf-font` exit 0 (358 passed); `cargo test -p
pdf-archive` exit 0 (254 unit tests and the integration binaries passed); `--test archive --test
profiles` exit 0 (524 and 13 passed, 6 ignored); clippy `--no-deps -D warnings -p pdf-font -p
pdf-archive -p pdf-transform --all-targets` exit 0; on the worktree, `pdf-transform`'s clippy exit
101 (101 of 100 lines), then exit 0 after the second lift. `rustfmt --check` on the seven files exit
0. Census against HEAD's, no pack (409 s) and with it (240 s), exit 0 each: every line identical.
`rank.py` exit 0. `--bin cited` exit 0 (11 501 pairs); `--bin quotations` exit 0; `cargo test -p
conformance --no-fail-fast` exit 101 (469 passed, 1 failed: `records`, on round 1905's record).

**Found, not fixed.** `crates/pdf-font/src/cff.rs` `with_charstrings` (~line 699) appends the
restated `CharStrings` INDEX and leaves the old one unreferenced: the probe's `CFF ` table went from
811 902 to 1 589 054 bytes, its output from 3.08 MB to 3.35 MB.

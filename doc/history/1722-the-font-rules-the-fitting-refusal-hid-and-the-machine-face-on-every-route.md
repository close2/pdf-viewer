# 1722 — The three rules the fitting refusal hid, and `machine-face` on every route

Slot 3 of batch one hundred and eleven, 2026-10-23, an archive round. ADRs 2278, 2279; Q585.
**Built.** (1) ADR 2278, read against ISO 19005-2 section 6.2.11.6 / 19005-4 section 6.2.10.6 and
sections 6.8 / 6.9: a `Type1` encoding on an admitted base whose `Differences` names `.notdef` is
restated once the font is `TrueType` (the converter's; `E3_Mayhew_et_al_2009`, and the census's
`3253-Article Text-6302-1-10-20190531`); a `Type1` descriptor flagged Symbolic restated as
`TrueType` with a Latin encoding says Nonsymbolic in its copy (the converter's; the two PDF 2.0
samples); an embedded PDF stating no part of ISO 19005 fails the embedded-file rule (the
validator's; the portfolio's five attachments, which `as-if-printed` now discards). (2) ADR 2279:
the unrestated route into a file's descriptor names `machine-face` over a machine's face. (3) Q585
asks whether `substitute-stretch` wants a bound, with the widest fit drawn; it recommends none.
**Drawn** (page 1, 100 and 288 dpi, source against output): the three font documents ≤ 1 level;
`3253` ≤ 1 level on every one of its twelve pages; the portfolio's output pixel for pixel HEAD's.
veraPDF 1.30.3 passes all six remedied outputs. Census against HEAD in ADR 2279's table.
**Hypotheses.** "Two of the three are the converter's": held, and the third (6.8) is the
validator's. "`machine-face` refuses twenty to forty `as-shown` documents and no printing-profile
one": the second half held, the first did not — ten per part (eight pdf.js, two veraPDF), none in
`doc/corpora`, where seven already-refused documents now name it beside another loss. "The owner
wants no bound": open in Q585. **Premise that did not hold**: the validator passed an undeclared
embedded PDF as possibly part 1, which no part this tree holds supports; and five tests passed only
because the unrestated route dropped a machine-face loss (ADR 2279).
**Gates.** Worktree: `rustfmt --check` on my six Rust files exit 0. Export of this slot alone:
`cargo fmt --check` exit 0; `-D warnings` clippy `--all-targets` on pdf-archive and pdf-transform
exit 0; nextest both 952 passed, 0 failed, 16 skipped (the dev sandbox worker built first, trap
10); rustdoc `-D warnings` on both exit 0. Plant (HEAD's sources, these tests): exit 100, the three
new tests and the machine-face assertion failing (trap 13). `tools/state.sh --round 1722 archive`
in both exports exit 0: validator corpus identical, over 0; converter over veraPDF identical;
`as-shown` two more refused per part; cross-check 13 560 compared, 0 disagreements. Real-corpora
census alone exit 0 in both. veraPDF 1.30.3 over the six outputs: PASS. `cargo test -p
conformance --no-fail-fast` in the worktree: 462 passed, 2 failed, both siblings' records
(1721, 1726); `evidence`, `adr_status`, `questions` exit 0. `t3-foreign_corpus` not run: its header reads back five
writers and not `archive`. Arms not compared: `cargo tree -p render-raster` reaches neither crate.
5784 s.

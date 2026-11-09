# 1979 — A shipped face is embedded with the fitting the window drew it at

Session 1979, a single round, PDF/A lane's worktree. R09 (b); ADRs 2769 and 2770.

**The premise.** Half true. Round 1974's shipped census refuses 65 documents under `as-shown` on
`substitute-stretch`. In 32 it is the only refusal at both parts, in 6 at one part (four PDF 2.0
files at 4, `363_Risk…` and `smaskdim` at 2b), and in 24 it comes with `machine-face`, which is R09
(a). Every face in the 38 is Liberation Sans (`glyf`), at scales from 0.9928 to 0.99999. Most
scales are integer `/Widths` against a 2048-unit em. The licences allow the change: Liberation is
OFL 1.1, renamed `QuorraText` (A550), and Foxit is BSD-3.

**Built.** ADR 2769: `pdf_font::stretch::sfnt_outlines` scales every x coordinate, component offset
and two-by-two cross term, every glyph and font box and every side bearing. It writes `loca` long.
Advances and instructions are untouched. It reads every glyph back within one unit of the em. The
conversion holds each shown glyph to the window's outline (`same_once_written`). The TrueType
`cmap` restatement's proof accepts the face scaled by the same writer; without that,
`issue17492` and `issue7580` stopped on 6.2.11.6. ADR 2770: the fitting is written where the
profile withholds the loss, and `as-if-printed` and `state-every-loss` keep A566's unmodified face.

**Freed**: under `as-shown`, 33 documents at 2b and 36 at 4, and under `no-data-loss` 31 and 35.
`preEnc_20141025_105451` changes under `as-shown` at both parts, because its attachment `48.rcpt.pdf`
now converts and is laid out as pages. R09's rank row goes from 121 to 80 at each part, and no
document is refused on `substitute-stretch` any more. **Kept by name**: a fitted CFF face, a
variable font, a number past its field. `machine-face` is R09 (a).

**Pixels** (CPU, 72 dpi, pages 1–4, `/OutputIntents` renamed): each of the 36 `as-shown` outputs at 4
is within one level of its source, and `freetexts` within two, as its unmodified face is. The
unmodified face (the `as-if-printed` output) differs by 7 levels over 35 000 to 54 000 pixels on
`E3_Mayhew` pages 2–4, and by 12 over 218 on `bug1671312_reduced`. Not this round's:
`issue13447`'s constructed appearances (226 levels, as at HEAD); `PDF 2.0 with page level output
intent`, whose page-level intent the rename removes too.

**Tests.** `stretch::tests` (five; an unscaled offset fails three); `archive.rs`'s two fitting tests
(`write_fitting` off fails the first and the decision test); `truetype_cmap.rs`'s restatement over
the fitted face (fails without the proof's change).

**Gates.** fmt, clippy `-D warnings --all-targets` and rustdoc 0 on pdf-font, pdf-transform; nextest
of the two and pdf-archive 0 (1830); `cargo test -p conformance` 0 (471); censuses against 1974:
shipped 69 NEW, 2 CHANGED, stating 66 NEW, 0 GONE; veraPDF 1.30.3 passes all 137 (and 301 candidates);
archive tier 0; tier 2 as at 1974 (16 moved, 62 incomplete); `state.sh docs` queued at hand-off.

# 1571 — Overprint mode 1 is a statement or a press's loss, and a TR2 is written Default

Slot 1 of batch ninety-one, 2026-10-19, an archive round. ADRs 1978, 1979; no Q.

**The premise held; the hypotheses half held.** The census named two witnesses per site, not one,
and neither overprint witness had OPM as its only refusal. Rendered first (trap 182), `102952.pdf`
with `/OPM 1` written `/OPM 0` by hand draws 77 of 77 pages byte-identically at 72 dpi and pages
1–8 at 720 dpi: this reader's window composites by the mode only in a four-component blending
space. It is still a loss — its states govern `0 0 0 1 k` text, which a press overprints under mode
1 (§8.6.7). The ICC premise named ISO 15076-1; the check reads ICC.1:1998-09 and :2001-12.
`issue14814.pdf` lacks only `cprt`, which no colour reads; `435321.pdf`'s tag table is unreadable.

**Built.** `pdf-archive`'s survey records which graphics state set the mode at each `ICCBased` CMYK
overprint, the `DeviceCMYK` marks painted under a non-zero mode, and whether a bound stopped the
walk (`stopped_short`). `pdf-transform` sets those states to `/OPM 0`: a statement where they
govern no `DeviceCMYK` mark, `Loss::OverprintMode` where they do (`as-shown` discards it,
`as-if-printed` stops), refused by name in a four-component blending space (ADR 1978). `/TR2`
other than `Default` is written `Default` under ADR 1964's two answers (ADR 1978 section 6). The
CharSet removal read only the validator's 32 kept places of 190; a shortened list now takes every
font's descriptor (ADR 1978 section 5). The ICC site is decided, not built (ADR 1979).

**Pixels.** `102952.pdf` → 2b under `as-shown`: 2 of 77 pages differ at 72 dpi, at most 8 of 255
(the GRACoL output intent), pages 1–4 identical at 720 dpi. `2018-Lee` OPM 0 by hand: 20 of 20
identical. veraPDF `6-2-4-2-t02-fail-a/b/c`: statements, identical, veraPDF pass. `6-2-5-t02-fail-a`
(TR2, authorised): the invader cyan to red, 0.95 % of channels, max 174 — the loss, stated.
veraPDF 1.30.3 calls `102952.pdf`'s output non-compliant on 6.6.2.3.1 alone: four XMP properties
(`xmpTPg:HasVisibleTransparency`, `HasVisibleOverprint`, `SwatchGroups`,
`xmpMM:OriginalDocumentID`) this validator holds predefined — a question for the XMP edition part 2
names, not taken up here.

**Gates.** rustfmt clean on every file touched. `RUSTFLAGS=-D warnings clippy -p pdf-archive` and
`-p pdf-transform --all-targets`: exit 0 (after moving a `match` arm into `set_device_colour` and
two per-document routes into `stated_where_free`). `nextest -p pdf-archive -p pdf-transform`: 799
passed, 0 failed. `cargo test -p conformance`: 411 passed, 1 failed — `records`, on sibling 1575's
record. `tools/state.sh docs`: exit 0. One `--tree 12` hold, 599 s: `t3-archive_corpus` exit 0 (2b
485 converted, 125 refused; 4: 226, 133 — 481/129 and 222/137 before), `t3-foreign_corpus` exit 0,
`t3-archive-val` exit 0, `cross_check` exit 0; veraPDF corpus census exit 0, 2 908 of 2 908
opened; pdf.js and corpora census exit 0, 1 450 of 1 477: 33 unbuilt at 2b (35 before).
`--remedy-sites --to 2b`: 33 of 75 not built (35). Duration: 8 500 s.

# 2240 — A text operator outside a text object enclosed in one, and a recovered file's proof

Lane 3, the PDF/A converter's last content class from the silent-repair campaign. ADRs 3661, 3662.

- **What this reader draws** (ADR 3353, `run/text_ops.rs`): a positioning or showing operator
  outside `BT … ET` acts on the identity matrices `ET` leaves (or the start of the content set); a
  stray `ET` acts on nothing; text state operators are no fault outside (§9.3.1).
- **ADR 3661**: `as_drawn::Walk` adds a `BT` in front of each run of them and an `ET` after the
  last, before the first operator §9.4.1 keeps out of a text object (and before `BMC`/`BDC`/`EMC`),
  at a stream's end, or not at all where the producer's stray `ET` follows. The walk follows the
  matrices (identity, stated by a `Tm`, shown past the line matrix, moved); the `BT` restates a
  `Tm`'s numbers where the operator reads a stated matrix, and none is added where it cannot state
  it. ADR 3558's proof decides; loss `content-as-drawn` (the reading is ADR 3353's, not the
  clause's). The class has its own refusal sentence.
- **ADR 3662**: the proof could never run on a recovered file — `incremental_update` refuses one
  (`UpdateError::Recovered`) — so `T02-05-01_008` and `_012` were refused by ADR 3641's class, not
  §9.4.1 (their survey names no outside-text operator). The proof's second file for a recovered
  source is now written whole from the objects this reader read (`whole_file`).
- **Documents** (`as-shown`, both parts): `T02-05-01_001`, `_003`, `_004`, `_008`, `_012` and
  pdf.js's `issue10388_reduced` convert; veraPDF passes them. The class's other carriers —
  govdocs `020806`, `027613`, `040669`, `150277`, `427669`, `455746`, `504076` — stay refused by
  another content class first. `T02-05-01_006`, `_009`, `_011`, `issue9252` and five govdocs files
  now pass section 5.1's proof and are refused by section 6.2.2's operator row or another class.
- **Census** against round 2235's, with and without the pack (the same movement): `as-if-printed`
  does not move; `as-shown` either part, pdf.js 905 → 906 written and 42 → 41 refused at both,
  corpora 413 → 418 and 54 → 49. veraPDF 1.30.3 over 5505 outputs (2235: 5493):
  5453 compliant, 40 not and 12 unfinished as before; the twelve new outputs pass.
- Left: a `Td` from a stated matrix is not composed and a width never measured, so a show after a
  show across a cut stays refused; the govdocs carriers' other classes.

**Gates.** Over `-p pdf-model -p pdf-archive -p pdf-transform -p viewer-qt -p conformance`: fmt 0;
clippy `-D warnings --all-targets` 101 (two functions past 100 lines, a `write!` ending in a
newline, an `assert!` of emptiness), fixed, then 0; sandbox `--bins` 0; nextest `--lib --bins
--tests` (`xvfb-run -a`) 100, 5321 of 5324, `audit-diff.py` "failed 3, expected 3" (round 2237's
coordinates audit tests, round 2238's); rustdoc `pdf-transform` 0; fuzz 0; `cargo test -p conformance` 0.

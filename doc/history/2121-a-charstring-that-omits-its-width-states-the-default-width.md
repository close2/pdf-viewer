# Round 2121 — a Type 2 charstring that omits its width states `defaultWidthX`

A font round in `gwg` on ADR 3173 section 5: an omitted charstring width was read as none.

## Read

- Adobe Technical Notes #5177 section 3.1 and #5176 section 15 and Table 23 (fetched; not held in
  `doc/`): an omitted width is the Private DICT's `defaultWidthX`, a stated one is `nominalWidthX`
  plus the operand, and both operators default to 0.
- `read-fonts` reads both per subfont (`FDSelect` included); its `None` for an unset
  `defaultWidthX` was the whole defect.

## Built

- `cff::evaluated_widths` reads that `None` as 0 (ADR 3185 quotes the notes). `None` stays for a
  charstring that cannot be evaluated or is not there.
- Tests: `cff_width_tests.rs` (no default under a nominal, a non-zero default, two Font DICTs of
  defaults 250 and 600, a glyph past the INDEX); `metrics.rs`'s window advance of a font with no
  `/Widths`; `pdf-archive`'s 6.2.11.5 rule. The old reading planted back failed four of them.
- The ten compiled-in bare CFF faces have no such glyph (probed): substitutes do not move.

## Measured

- veraPDF 1.30.3, PDF/A-2b, on a one-glyph fixture whose `space` omits its width under a DICT
  stating only `nominalWidthX` 100, `/Widths [278]`: the installed main binary's output failed
  6.2.11.5; this tree restates the program's width to 278 and its output is compliant, as is
  its output of the same file with `/Widths [0]`.
- `pdf-archive --test corpus` (the validator over the veraPDF corpus): 0 missed, 0 over, every
  target's `all` line equal to the 2026-11-19 post-merge train's.

**Gates.** pdf-font, pdf-archive, pdf-transform, pdf-model: fmt 0; clippy `-D warnings
--all-targets` 0 (after one `doc_markdown` backtick); `pdf-sandbox --bins` 0; nextest `--lib --bins
--tests` 4937 run, 4937 passed, no difference from `expected-failures-current.json` (empty);
`cargo test -p pdf-font --lib` 0; rustdoc `-D warnings --document-private-items` 0 after a link to
a private item became prose; `cargo test -p conformance` 0 after the notes' blockquotes moved from
`cff.rs` to ADR 3185 (nothing in `doc/` checks them). Fuzz not owed (no signature changed).

## Left

- `read-fonts` floors `defaultWidthX` and `nominalWidthX`; Table 23 makes each a number (ADR 3185).

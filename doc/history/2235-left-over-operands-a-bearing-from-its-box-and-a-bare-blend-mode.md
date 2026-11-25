# 2235 — Operands left over written out alone, a bearing from its box, and a bare blend mode as drawn

Lane 3, the PDF/A converter's leftovers from the silent-repair campaign. ADRs 3641, 3642, 3643.

- **Operands left over** (ADR 3641): where `model/operands-left-over` is an operator's only repair,
  `as_drawn` writes out only the leftovers and keeps the operator with its own (§7.8.2's
  "immediately precede"; §8.2 gives the counts, not the rule). The walk keeps each operand's start
  (at most 64); a salvaged run among the leftovers goes with them, one among the operator's own
  stays after. The proof by drawing both files still decides. The refusal sentence says so.
- **`bug911034`** (ADR 3642): `horizontal_metric` answers `None` only for a missing advance; a
  missing bearing is the glyph's `glyf` `xMin` (zero with no outline) in `with_advances` and
  `stretch`, and the restated program is proved glyph for glyph (`as_read::sfnt_draws_alike`).
  `stretch_cff` and `embed` still decline. An sfnt's refusal has its own sentence.
- **`/BM`** (ADR 3643): §11.6.3 gives `Normal` only to the array form, so a bare unknown name
  restated as `/Normal` is the loss `content-as-drawn`, not a statement; section 5.1's
  `model/blend-mode-unknown` is answered where the rewrite restates it. The configuration test of a
  shape qualifier now uses `annotations/normal-appearance-shape`.
- **Census**, with and without the pack (the same movement), against round 2232's:
  `as-if-printed` does not move. Under `as-shown` `issue6342`'s leftovers are written out and its
  refusal is now 6.2.2's `c08.`; `455746` stays refused, naming `syntax/number-salvaged` before the
  wrong-type operator it named. `T02-05-01_008` and `_012` do not move: their `Tj` also stands
  outside any `BT`, a second class at the operator, which ADR 3558 leaves. `bug911034` converts as
  in round 2232's (round 2230 refused it). No corpus document states a bare `/BM`. veraPDF 1.30.3
  over 5493 outputs: 40 not compliant, 12 unfinished, round 2232's documents; `bug911034`'s pass.
- Left: the T02 shows outside a text object; `embed`'s and `stretch_cff`'s missing bearings.
- Launch counts not owed: `pdf-font`'s loading path reads the advance alone, as before.

**Gates.** Over `-p pdf-font -p pdf-model -p pdf-archive -p pdf-transform -p viewer-qt -p conformance`:
fmt 0; clippy `-D warnings --all-targets` 101 (three functions past 100 lines, an unchecked
addition, a by-value enum), fixed, then 0; `pdf-sandbox --bins` 0; nextest `--lib --bins --tests`
(`xvfb-run -a`) 100, 5769 of 5771, `audit-diff.py` "failed 2, expected 0": this record's gates and
`repair_census` (four new defaults in `sfnt.rs` unmarked, a marker pushed out of reach; `stretch.rs`
7 of 8 held), both fixed; rustdoc of `pdf-font` 101 (`[t]`), escaped, then 0, `pdf-transform` 0;
fuzz 0; `cargo test -p conformance` 0.

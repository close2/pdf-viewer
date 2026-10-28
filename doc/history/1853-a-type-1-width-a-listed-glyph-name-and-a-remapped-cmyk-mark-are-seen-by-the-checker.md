# 1853 — A Type 1 width, a listed glyph name and a remapped CMYK mark are seen by the checker

Slot 3 of batch 126, 2026-10-28, an "archive" round (Opus 5.5), three sessions; ADRs 2540, 2541, Q719 unused. I worked
in `scratchpad/r1853/` and two exports built there, of this tree and of HEAD. No ledger row moved.

**Premise.** "Three readers too lenient at one site each" held for two of three as stated. The Type 1 reader was blind ,
but the brief's witness `0004-637X_751_2_90` is a CFF: its CFF width reader rounded 776.89 to 777, and once that was
fixed the witness found a second defect, a real `nominalWidthX` read as 0, so a restated 778 read back 1 273. The other
two held; the table compiled in is the 2002 list exactly (ADR 2540 section 2).

**Built.** `type1::Program::advance` reads `hsbw`/`sbw`; a CFF width is read unrounded and a real nominal width floored;
a one-unit difference is consistent in the validator and the converter; a disagreeing Type 1 program is refused as
`PROGRAM_NOT_RESTATABLE`; `encoding::is_listed` is the row test of 6.2.11.6 and 6.2.11.7.2; a `DeviceCMYK` mark under an
`ICCBased` `/DefaultCMYK` is an `ICCBased` paint, and the supplied-press guard refuses only marks whose mode the press
rewrite does not set (ADR 2541); `sites::objects_named` refuses a list cut short past 4 096 objects (trap 289).

**Moved.** Against HEAD's census without the CJK pack: pdf.js 875 to 865, 881 to 871, 816 to 806, 818 to 808 converted
documents; corpora 398 and 401 unchanged, 336 to 335, 333 to 332; with the pack the same documents move (876 to 866, 882
to 872). Twelve leave, each under the widths or the `Differences` row, and veraPDF fails HEAD's output of each at both
targets (24 of 24, no disagreement); `javauninstall-7r` converts and veraPDF passes it. Before the one-unit fix six more
had left that veraPDF passes (ADR 2540 sections 4 and 5).

**Plants.** Each blind spot passed HEAD's checker and fails this one and veraPDF, and one byte reverses it: the
witness's width 778 to 777 (both pass), `/section` to `/uni00A7` (both fail 6.2.11.6), `OPM` 0 to 1 (both fail 6.2.4.2).
At the one-unit bound, `issue11045`'s 272 against its program's 271 passes both, and 273 fails both at 6.2.11.5.

**Found, not fixed.** 6.2.11.4.1 and 6.2.11.5 reach a `uniXXXX` glyph by the lenient name mapping (veraPDF fails a
planted one at all three clauses, this tree at 6.2.11.6 alone); `read-fonts`' charstring evaluator has no operation
budget (1.3 ms at k = 3, depth 10); `fonts/truetype_cmap.rs` refuses past 32 printed places; `rank.py` maps the
`Differences` row to no remedy (10 documents).

**Gates.** In the export: `cargo test -p pdf-archive -p pdf-font` 612 passed, exit 0; `--test archive` 449 passed and
`--test profiles` 7 passed, exit 0; `cargo clippy -p pdf-archive -p pdf-font -p pdf-transform --all-targets -D warnings`
exit 0; `cross_check` 12 555 places, 0 disagreements, exit 0; every census exit 0, the last pair 278 s and 213 s, peak
2.41 GiB; veraPDF on 42 outputs and 4 plants. In this tree: `cargo check --tests -p pdf-font -p pdf-archive -p
pdf-transform` exit 0; `rustfmt --check --edition 2024` on my files exit 0; `--bin cited` exit 0; `--bin quotations`
exit 0; `tools/state.sh --round 1853 archive` exit 0 (the validator 0 missed and 0 over at all six targets); `cargo test
-p conformance --no-fail-fast` 470 passed, 0 failed, exit 0.

Duration: 5484 s, about 5640 s and 8575 s, three sessions.

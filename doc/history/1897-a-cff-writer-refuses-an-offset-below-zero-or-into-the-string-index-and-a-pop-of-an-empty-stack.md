# 1897 — A CFF writer refuses an offset below zero or into the String INDEX, and an operator that pops an empty stack

Slot 7 of batch 130, 2026-11-02, Opus 5.5, three sessions, the first two handed over; a parsers round
(robustness); ADR 2628 used, ADR 2629 and Q773 unused. Ledger: no row moved — the program's inside is
Technical Notes #5176 and #5177, not ISO 32000-2.

**Premise.** No artifact of rounds 1882 or 1888 was kept, so a ten-minute fork-mode run of `cff_write` at
HEAD, under the bound with its own target and `-s none`, from 170 fresh seeds, gave two. The brief's
hypothesis held in part: (a) and (b) are offset arithmetic, but the HEAD stop was a `Private` at offset
-48, which `offset_outside` passed because it asked `at > -1`; (a)'s negative size and (b)'s String INDEX
offset were fixed as worded (ADR 2628, decisions 1–3). (d) is a pre-scan of the charstring written
(decision 4), and the other HEAD stop is not (d) but the reserved 12 6, which `read-fonts` runs as Type 1's
`seac` and so gives a glyph another glyph's width (decision 6). Each later run's stops were decoded to a
root of their own: a subroutine spliced past its first `return` (5), a `Private` ending past the program
(7, the brief's (c)), a width that would be a forty-ninth operand (8), and a Local Subr INDEX ending past
the program, so that the walk read no subroutines on the input and some on the longer output (9).

**Plant.** Session 1: the String INDEX check removed, its test failed, 4 of 5 passed. Session 3: decision
8's check disabled (`false &&`), its test failed, 8 of 9 passed; restored, byte-identical.

**Gates.** Shared tree, decisions 1–9: `cargo nextest run -p pdf-font` exit 0, 354 of 354 tests;
`cargo clippy --no-deps -p pdf-font --all-targets -- -D warnings` exit 0; `rustfmt --edition 2024 --check`
on `cff.rs` and `cff_stop_tests.rs` exit 0; `cargo fmt --check` and `RUSTFLAGS="-D warnings" cargo clippy
--all-targets` under `fuzz/Cargo.toml` exit 0 (decisions 1–8); `--bin cited` exit 0, `--bin quotations`
exit 0, `cargo test -p conformance --no-fail-fast` exit 0, 470 passed 0 failed. Export: nextest exit 0,
346 of 346. Census, no pack, in the export: decisions 1–7, 1–8 and 1–9 each exit 0, 0 non-noise lines
moved against HEAD's, each in about 250 s at 2.5 GiB peak. Fuzz: decisions 1–7, stopped at 89 s on one
artifact (decision 8); decisions 1–8, ten minutes, fork mode, `-ignore_crashes=1`, 1 737 seeds, 4 306 093
executions, 0.11 GiB peak, two stops (decision 9, and Open); eleven of the twelve artifacts replay clean on
decisions 1–9. Not run: a ten-minute run on decisions 1–9; `raster_golden` and the six arms (no render
path calls these writers: their callers are `pdf-transform/src/archive/fonts/` alone).

**Found, not fixed.** `crates/pdf-font/src/cff.rs:1544` `calls_local_subr` counts no stem a called global
subroutine declares before a `hintmask`, so it can misread the mask's length. `cff.rs:891` `font_dict_shifted`
copies a Font DICT's `charset`, `Encoding`, `CharStrings`, `FDSelect` and `FDArray` unshifted while the
String INDEX grows under them (ADR 2628, Open; artifact `64656d0d`, `cff_write.rs:185`).

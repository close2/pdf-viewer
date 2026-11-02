# 1907 — A Font DICT's offsets are shifted with the bytes they name, and the walk for a local subroutine call carries the stack and the stems

Slot 7 of batch 131, 2026-11-02, Opus 5.5, two sessions, the first handed over; a parsers round
(robustness); ADRs 2648 and 2649 used, Q783 unused. Ledger: no row moved — the program's inside is
Technical Notes #5176 and #5177, not ISO 32000-2.

**Premise.** The brief sent (a) to TN 5176 section 19; that section is `FDSelect`. The Font DICT is
section 18's, which asks of it only that it specify its Private DICT, and the notes give it no operator
table, so it is read with Table 9's, whose `charset`, `Encoding`, `CharStrings`, `FDSelect` and `FDArray`
are offsets from the start of the data: each is shifted, none refused (ADR 2648). A name-keyed program's
`FDArray` was copied whole with its `Private` offsets unshifted, and is now rewritten where it reads. The
artifact `64656d0d` was not kept on disk; its shape is minimised into a test. (b) held as worded, and
`endchar` was found to end the walk inside a subroutine as well (ADR 2649).

**Plant.** (a): the shift taken out (`font_dict_shifted` back to `Private` alone), the new test failed
(`79 == 79`), restored. (b): a stack and a stem count per call restored, the mask test failed, 12 of 13
passed; restored, the planted line absent.

**Gates.** Shared tree: `cargo nextest run -p pdf-font` exit 0, 362 of 362 tests; `cargo clippy --no-deps
-p pdf-font --all-targets -- -D warnings` exit 0; `rustfmt --edition 2024 --check` on `cff.rs` and
`cff_stop_tests.rs` exit 0; `cargo fmt --check` under `fuzz/Cargo.toml` exit 0; `RUSTFLAGS="-D warnings"
cargo clippy --all-targets` under `fuzz/Cargo.toml` exit 101, on round 1905's `policy_document.rs:136` and
round 1906's `formcalc_cascade.rs:174`, and exit 0 in the export; `--bin cited` exit 0, `--bin quotations`
exit 0, no line naming these files; `cargo test -p conformance --no-fail-fast` exit 101, 469 passed and 1
failed, `records` on a sibling's draft record (round 1905's, then round 1903's), this one passing. Export (HEAD and these two files): nextest exit 0, 357 of 357;
clippy exit 0. Fuzz, `cff_write`, ten minutes, fork mode, `-ignore_crashes=1`: 634 s, 1 305 105
executions, 0 crashes, 0 timeouts, 0 out-of-memory stops, no artifact, 0.11 GiB peak. Memory, `getrusage`
under the fan-out fixture: HEAD 12 032 KiB, this code 12 576 KiB. Census, no pack, in the export: exit 0,
377 s, 2.49 GiB peak, its 779 lines from `running 1 test` identical to HEAD's but for two timing lines.
`raster_golden` in the export exit 0, 2 passed. The six arms in the export, by page against HEAD's (name,
frame digest, mean error): all six exit 0, 968 pages at 1x and 965 at 4x each, 0 lines moved, 2 599 s under the lock at 3.80 GiB peak.

**Found, not fixed.** `crates/pdf-font/src/cff.rs:1679`, `LocalSubrWalk::run`'s escape-byte arm clears the
stack on every `12 x` operator, Type 2's arithmetic among them (TN 5177 section 4.4), so a stem hint whose
operands an `add` or a `dup` left counts short and a later `hintmask` can be misread by a byte.

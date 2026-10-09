# 1482 — A test waits on the announcement its code already makes, and a poll on a child's exit stays

Slot 3 of batch seventy-six, 2026-10-09, a script round. ADR 1800; ADR 1801 and Q378 not used. No
ledger row moved, because the contract is the tests and no clause is reached.

**Premise.** Held. `grep -rn thread::sleep crates/*/tests/*.rs | wc -l` printed 13, at the lines quoted.

**Hypothesis.** Partly held. The four `viewer-host` polls became waits on the `wake` each `Submitter`
method already takes. The two `signature_policy` loops were also a five-second race. `turn_path`'s
cadence and back-off stay timed. Not foreseen: the 2 s at `turn_path.rs:908` was covering two waits,
the plant's bring-up and the driver's lagging counter. Once the test was held at the plant's line,
the first reading said 23.9% against a 20% threshold. `reaping.rs`'s 300 ms settle decided nothing,
and is gone. Four polls on a child's exit, and `in_process_deadline`'s poll on a count, have no event
to wait on, and they stay with their reason beside them. `save_round_trip`'s `ask` piped a stdout
that it never drained while it polled. Its streams now go to files. No corpus document reaches that
hazard: the mupdf witness prints at most 10 863 B, from `prefilled_f1040.pdf`.

**Built.** In `viewer-host`, `ANSWERED_WITHIN` and the wake channel (`fetched`, `fetch_policy_woken`,
`collected`). In `turn_path`, `BUSY_FROM_HERE`, `PLANT_SECONDS` and a read loop that runs until the
counter crosses the threshold. In `save_round_trip`, `wait_within` and a test that a witness printing
256 KiB is read whole. Kept sleeps: 7, each commented, citing ADR 1800. No crate under test
changed. Files: `crates/pdf-model/tests/save_round_trip.rs`, `text_extraction.rs`; `crates/pdf-sandbox/tests/in_process_deadline.rs`, `reaping.rs`;
`crates/pdf-transform/tests/foreign_corpus.rs`; `crates/render-raster/tests/turn_path.rs`;
`crates/viewer-core/tests/headless.rs` (near line 8776; round 1480's hunk kept); `crates/viewer-host/
tests/fetch_import.rs`, `signature_policy.rs`, `submit.rs`; ADR 1800; this record.

**Gates.** `rustfmt --check --edition 2024` on the ten files: exit 0. `cargo clippy -D warnings
--all-targets` for `pdf-model`, `pdf-sandbox`, `pdf-transform`, `render-raster`, `viewer-core` and
`viewer-host`: exit 0, after a neighbour's lint in `pdf-model/src/submission.rs` cleared. `cargo
nextest run` per crate, exit 0 each: 2 027, 48, 489, 100, 374 and 239 passed. Ten runs in a row, exit
0 each: the three `viewer-host` files, `reaping`, `in_process_deadline` (2.7 to 3.7 s), `save_round_trip`,
`headless a_drag_across`, `text_extraction` and `foreign_corpus`. Plants (trap 13): with no `wake()`,
7 of 7 tests failed at once; with the answer unsent, 7 of 7. A `Drop` without its wait failed the
stranger test, and with no `Drop` it failed on `left: "1"`. A piped witness stdout was killed at
30 s. A 1000% threshold read on to the plant's end (7.1 s, at most 99.0%) and failed. The `turn_path`
gate under `--clock`: exit 0, 33 figures banded, 0 outside, waited 1 066.4 s and held 172.3 s. The
plant ten times under `--clock`: exit 0 each, first reading 25.9 to 47.1%, waited 364.8 s. Witness
census as a `--tree 6` walk: exit 0, waited 3 118.1 s, held 92.7 s. `cargo test -p conformance
--no-fail-fast`: one failure, `no_comment_names_a_round_by_number`, on round 1484's uncommitted
knob in `pdf-vfs/src/serve.rs:185`. Duration 4 926 s.

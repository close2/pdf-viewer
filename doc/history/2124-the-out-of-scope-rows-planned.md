# Round 2124 — the out-of-scope rows, planned

A docs round in the `audit` worktree: RFC 0015 plans what implementing the 51 `out-of-scope` rows
would mean, done safely, and asks Q946. It builds nothing, lifts no exclusion and moves no row.

## What it found

- The 51 rows are 27 of §13.6, 18 of §13.7, §13.1, §13.2.1, §13.2.7.5, §13.5, §12.6.4.16 and
  §12.6.4.18. Audio and video playing is three rows; 3D is over half the rows and two thirds of the
  151 reader obligations extracted from clause 13.
- The hand-off the exclusion admits is unfinished: every window says "this window starts no player
  yet, so nothing plays", and A254's `Media` level is unbuilt. In scope now; Q946 question 1.
- ISO 32000-2 cites ECMA-363's third edition and says it does not cite the fourth; the U3D trigger
  names the fourth. PRC has no trigger, and ISO 14739-1 is not held.
- Measured (`cargo deny` 0.20.2): `cpal` adds 105 KiB to a stripped release binary and links
  `libasound`; `rusty_h264-decoder` 0.17.3 and `rusty_aac` 2.1.1 were released after RFC 0012's audit;
  `u3d` is unchanged; two PRC crates (`prc` 0.3.0, `pdfcer-3d` 0.163.0) are new and unjudgeable
  without the standard.

## Ledger

None touched: 788 `implemented`, 51 `out-of-scope`, 31 `inapplicable`, 11 `departed`, 2 `writer-side`
before and after.

**Gates.** No crate is touched, so fmt, clippy, the `pdf-sandbox` build, nextest and rustdoc are owed
by none and were not run; `expected-failures-current.json` is unchanged. `cargo test -p conformance
--no-fail-fast` exit 0 once this paragraph was written (`records` had failed for its absence, every
other target passing).

## Habit

A brief's "what already ships" is a claim about the tree: grep the windows for the event's handler
before writing that a hand-off happens. Here the model emitted the event and every window declined it.

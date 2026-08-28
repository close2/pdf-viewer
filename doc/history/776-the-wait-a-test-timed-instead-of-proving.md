# 776 — The wait a test timed instead of proving

The launch test that five rounds' workspace runs watched fail under sibling load asserted the
machine along with the requirement: it demanded a real page inside `Drawing::SETTLE`'s 16.7 ms
of wall clock, so a loaded scheduler failed a correct mechanism. The requirement — a launch
*waits* for page one instead of polling for it — is structural, and the test now asserts it
structurally: one `settle` call, never a poll, must answer a page made too expensive to have
been finished before the wait began, and the wait's own ledger (`spent`) must have moved.
No assertion states that any duration was enough.

Date: 2026-08-28.
ADR: [0714](../adr/0714-the-wait-a-test-timed-instead-of-proving.md) — reserved by the
briefing; `main` carried through 0711 at this round's base.

Touched: `crates/viewer-host/src/drawing.rs` (one test rewritten, nothing outside `mod tests`),
the ADR and this file. **No pixel can move and no behaviour moves**: `Drawing::settle`,
`Drawing::SETTLE` and both hosts are exactly as they were.

## The A/B, all four arms in one sitting

Load supplied by this round's own CPU burners — plain `while :` loops started by a script that
records PIDs and killed by those PIDs, never by name (`doc/environment.md`'s `pkill` rule).
The machine was otherwise quiet (load average 3.5 at the start, 24 cores).

- **Old shape, unmodified tree, 48 nice-19 burners (load ~19): 4 of 8 runs FAIL** — the five
  rounds' observation reproduced on demand before anything changed, in the same tree that
  passes it 12/12 quiet. That is 772's diff-reverted A/B arriving from the other side: the
  failure is the machine's, produced at will by load alone.
- **New shape, planted pre-settle poll** (trap 13: `settle` degenerated to `collect`, the
  exact shape ADR 0678 replaced): **fails 4 of 4 quiet and 10 of 10 under load ~33.** The
  calibration is deterministic, not statistical — a poll cannot put the page in the one
  call's answer (the 5 000-fill page cannot finish in the microseconds before it) and cannot
  move `spent` off zero however lucky its timing.
- **New shape, real tree: 3 of 3 quiet; 20 of 20 under 48 nice-19 burners (load to ~46);
  10 of 10 under 48 nice-0 burners (load to ~54).** The whole `drawing::` module passed twice
  more at load ~54–57.

What is honestly still wall clock: the generous budget handed to `settle` and the test's
`GIVE_UP` bound are liveness bounds — load can only slow a pass, never fail one — and the
`spent > 0` assertion reads a clock but asserts only that it moved during a block, skipped
where no drawing thread exists because the fallback draws synchronously inside `ask`. The ADR
carries the argument; the cost of the new shape is about 0.4 s of drawing per run on a quiet
machine where the old shape took 0.02 s.

## Gates

The briefing asked for the full §2 sequence and it was run whole, on a machine gone quiet
after the burners were killed (checked with `vmstat`: 93% idle before the oracle line, the
lagging load *average* notwithstanding).

- `cargo fmt --all --check` clean; `RUSTFLAGS="-D warnings" cargo clippy --workspace
  --all-targets` silent (the `proc-macro-error2` future-incompat note is Cargo's, about a
  dependency, and not a lint).
- `cargo nextest run --workspace`: **2706 run, 2706 passed, 18 skipped** — including the
  rewritten test, in the full parallel suite that used to be where the old shape failed.
- `cargo test --workspace --doc` passed; `RUSTFLAGS="-D warnings" cargo check` over
  `fuzz/Cargo.toml --bins` clean.
- Oracle, with `PDFREF_CACHE` pointed at the shared reference cache: **3 passed in 61.6 s**,
  61 contradicted pages, none new, artefacts under the worktree's own build directory.
- The corpus gate passed in 5.9 s. The remaining lines ran sequentially after the oracle,
  every summary `ok`: text extraction **4 passed, 33.4 s**; selection census **1 passed,
  6.6 s**; accessibility census **1 passed, 20.2 s**; dates 1; xmp 2; jpeg2000 **1 passed,
  14.9 s**; quorra corpus **1 passed, 38.2 s**; fixed documents **1 passed, 12.4 s**;
  conformance **207 passed, 0 failed** across its binaries.

## The ledger

No clause is touched: the change is a test of host machinery, cites no normative requirement
it did not already cite, and `doc/conformance/ledger.toml` is unchanged. `cargo test -p
conformance` ran as part of §2 and is green over the new doc comments.

## Sweeps

Run after the sequence, never beside it: `ledger`, `tables`, `counts`, `entries`, `unread`,
`blockers`, `capabilities`, `inapplicable`, `owed`, `overstated`, `overtaken`, `pointers`,
`quotations`, `parts`, and `spec-errata check` — all exit 0, all at their documented noise.
`ledger`: 875 rows, 0 new. `spec-errata check`: 0 struck passages current, 0 quotations of
struck text. The only lines anywhere in the fifteen reports that name a file this round
touched are two **standing** entries in `drawing.rs` prose the diff does not reach (a
blockers line on a doc comment naming no clause at line 116, and `parts`' decay-detector
walking "the two hosts" at line 326 — correct in context, `viewer-host` serving exactly the
two toolkit hosts); both predate the round, checked against the diff. Nothing names the new
test, the ADR or this file. `quoted` and `unpriced` were not run: this round touches no
page-list note, and both take the oracle's log as their right-hand side (765's precedent).

## What is deliberately not done

- `Drawing::SETTLE`'s value (one 60 Hz refresh) is asserted nowhere. It is a product choice
  documented on the constant; a round that wants to retune it measures the real hosts on a
  quiet machine.
- §5's binaries were not rebuilt: this round measures nothing of the program a person runs,
  and `tools/round.sh` says the rebuild is owed on fifth rounds or before such a measurement.

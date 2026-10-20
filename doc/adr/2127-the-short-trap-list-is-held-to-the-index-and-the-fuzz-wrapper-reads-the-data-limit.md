# 2127 — The short trap list is held to the index, and the fuzz wrapper reads the data limit

Session 1645, batch one hundred's slot 7, a docs round. Status: **accepted** and **applied**.
Context: ADR 2110 (the reading set compacted, whose §2 found `doc/traps/every-round.md`'s row 25
departed from the index it claims to copy), ADR 1639 (the every-round lists), ADR 1036 (a row is a
lookup key), trap 13 (an instrument is calibrated by planting its defect), trap 221 and ADR 2102 (a
sanitised fuzz build under `tools/bounded.sh`), ADR 1423 (a campaign's scratch corpus), ADR 2126
(the second compaction, the round that made this).

## 1. The ten rows every round reads are the index's, and now a gate says so

`doc/traps/every-round.md` says its rows are "the full index's rows, verbatim: a change to one is made
in `README.md` and copied here". Nothing held the copy, and ADR 2110 found one row reworded in the index
and not in the copy — the row every round reads before it starts, stating a rule the index no longer
stated. `tools/conformance/tests/every_round_rows.rs` holds it: every table line of the short list
whose first cell is a number must be the index's line of that number, identical after trailing
whitespace; a number the index lacks and a number the list carries twice are named too. A failure
names the trap, both line numbers and the character where the copy departs.

**Calibrated twice** (trap 13). In the file, a second test runs the comparison over a hand-built
index and names a drifted cell, a missing row and a duplicate, and passes an exact copy. On the tree,
a `-U0` patch changing row 2's "a paint" to "the paint" failed the gate with "trap 2 (line 30)
differs from doc/traps/README.md line 40 at character 35", and was reversed. Which ten rows the list
holds is not gated: they are chosen by the count `tools/state.sh traps` prints, which moves as
`doc/history/` grows, and a gate on membership would fail a batch for a citation it did not make. Both
files now name the gate where a row is changed.

## 2. `tools/fuzz.sh` builds without the sanitiser and starts in scratch under a data limit

`tools/bounded.sh` always sets `RLIMIT_DATA`; AddressSanitizer reserves its shadow memory as one
mapping at start, which the limit refuses, so the wrapper's default `cargo fuzz run` never started
under the lock (trap 221: 730 s lost to it). And libFuzzer's `-jobs` writes `fuzz-<n>.log` into the
directory the run starts in, which was `fuzz/`. The wrapper now reads `ulimit -d` itself rather than
being told: under a limit it passes `-s none` — `doc/verify.md`'s campaign paragraph already
prescribes it, and it keeps libFuzzer's coverage and overflow checks — and starts the run in `--scratch
DIR`, or in `scratchpad/r$ROUND_SESSION/fuzz-<target>/` where `ROUND_SESSION` is set, and refuses with
neither, exit 2, naming trap 221. Without a limit it runs as `doc/verify.md`'s lines state, sanitised,
from `fuzz/`. `--fuzz-dir` names the crate wherever the run starts, so cargo-fuzz resolves
`fuzz/corpus/<target>` and `fuzz/artifacts/<target>` as before.

The condition is the limit and not the wrapper's marker (`HEAVY_WALK_HELD_BY`), because the limit is
what breaks the sanitiser: a run under `bounded.sh` without `--lock` has the limit and no marker. The
extra arguments after `--` now reach `eval` as `"$@"` rather than spliced into its string. `cargo-fuzz`
is put on `PATH` only where none is, so a caller's own is the one that runs.

`tools/conformance/tests/fuzz_wrapper.rs` runs the wrapper for real against a stand-in `cargo` and
`cargo-fuzz` first on `PATH`, which record their starting directory and arguments: under a 4 GiB limit
the run carries `--fuzz-dir <root>/fuzz -s none object` and starts in the scratch directory named;
under a limit with neither scratch nor session it is refused and cargo never starts; with no limit it
starts in `fuzz/` without `-s none` (where the test's own hard limit allows a child none, it says so
and the two limited cases still ran). Calibrated: with `sanitiser="-s none"` planted to empty, the
first case fails.

## 3. The small finds, each a correction applied

- `tools/state.sh traps` printed "cited with no row in the index: 0 632", which reads as one figure.
  It is two numbers the pattern took for citations — "traps 0" in record 1320 is a test named
  `traps` exiting 0, "trap 632" in record 637 is "the trap 632 named", a round's number — and records
  are not rewritten. The count is unchanged; the print is a count and the numbers as citations: "2
  (trap 0, trap 632)".
- `Cargo.toml`'s comment above `raster-gpu` called it "a git dependency rather than a path" with a
  revision pinned in `Cargo.lock` and a source in `deny.toml`. It is a path dependency: `raster/` is
  this tree's sub-project (merge `d9bc96b2`), its crates are workspace members, `Cargo.lock` carries
  no source for them and `deny.toml` says the rasteriser "reaches the graph by path". Rewritten so.
- `tools/drive-windows.sh` pointed at `doc/verify.md`'s "Driving the three windows", a section that
  does not exist. The list is the bold lead **The drive list** under that file's
  `tools/drive-windows.sh` line, which the pointer now names.
- Trap 221's row said "the wrapper does not yet", and its entry "Two rules until the wrapper is
  fixed". Both pointers now say `tools/fuzz.sh` applies the two rules under a data limit; the
  incident is unchanged (ADR 1525: a trap's story is not rewritten, its pointers are kept current).

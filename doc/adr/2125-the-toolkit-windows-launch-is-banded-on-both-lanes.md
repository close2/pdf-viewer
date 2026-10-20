# 2125 — The toolkit windows' launch is banded on both lanes

Session 1644, slot 6 of batch one hundred. Status: **accepted**. Context: `CLAUDE.md` principle 2
("Perf gates run in CI: cold open, time-to-first-page … A regression fails the build", and cold
bring-up "its own gate"); ADR 2071 section 3 ("Cold bring-up is its own gate for `quorra` and the
toolkits have none … The table above is a round's measurement, not a band"); ADR 1969, whose
method the bands take; ADR 2124, the profile read from the same hold. It cites no clause and moves
no ledger row.

## 1. What is built

- **One harness, `crates/viewer-host/tests/toolkit_launch/mod.rs`, included by path into
  `crates/viewer-gtk/tests/launch_path.rs` and `crates/viewer-qt/tests/launch_path.rs`.** The
  binary a gate launches has to be the one Cargo built for it, and `CARGO_BIN_EXE_<name>` exists
  only in the package that owns the binary, so each toolkit crate has the test; the two would
  otherwise be the same file twice, and a library module would ship the harness in
  every window. The directory keeps Cargo from taking the file for a target of `viewer-host`.
- **Seven figures a window**, each the minimum of nine fresh launches a lane with the lanes
  interleaved, read off the window's own `--trace=launch,frames`: `window_ms` on both lanes (the
  toolkit's first size for the page area), `page_one_ms` and `first_frame_ms` on both, and
  `device_up_ms` — principle 2's bring-up gate, held separately from the page.
- **The conditions are `launch_path`'s, by derivation rather than by copy of its numbers**: the
  fastest class of core, found from `cpuinfo_max_freq`, with rayon given that many threads; one
  runnable task per physical core as the ceiling above which a run prints and does not judge; and a
  display of the harness's own, taken only once its lock names its own server (trap 181). No `Xvfb`
  is a failure with a sentence, as an absent figure or an unbanded one is: nothing is skipped.
- **`TOOLKIT_LAUNCH_BINARY` measures another build of the window against the same bands**, which
  is how HEAD's export was measured beside the change in one hold (habit 101), and the run prints
  the binary it launched.
- **`doc/checks/toolkit-launch.toml` holds the bands**, one `[[window]]` a window.

## 2. The bands

Three runs of each window's gate on the tree's export, interleaved with three on HEAD's, in one
`--clock` hold on 2026-10-20 at a one-minute load of 9.1 to 10.4; each band is `launch-path.toml`'s
clock rule over the tree's three minima — the smallest × 0.85 to the largest × 1.20, rounded
outward to a tenth — and the three are written above it in the file. HEAD's three fell inside
every band (Qt's first frame 90.1 to 93.1 ms on the device and 74.7 to 76.8 on the processor, GTK's
89.8 to 98.6 and 89.1 to 92.1), and each window's gate then ran once more on each export against
the written bands and exited 0.

## 3. What is not gated, said rather than implied

- **A cold page cache.** Every launch here reads the document and the shared objects warm, which is
  right for an A/B and is not principle 2's cold-start measurement; `doc/todo/42` section 6 keeps
  the cold native launch as unmeasured.
- **The absolute figure depends on how the harness runs.** The same HEAD binary, in the same hold,
  put Qt's window at a median of 55 ms on `--cpu` under the profile's sampler (ADR 2124) and at a
  minimum of 67 to 68 ms under this harness, while GTK's agreed; the two launch through the same
  `taskset`, and differ in the display (a long-running server against the harness's own fresh one,
  started inside the lock's `nice 19`) and in rayon's thread count. Which of those is Qt's is not
  measured. The bands are of this harness's method, which is the one the gate repeats.
- **Who runs it.** It is a `--clock` gate a round runs by hand on the line the band file spells;
  putting it in the merge's tier 2 beside `t2-launch_path` is the batch tooling's change to make.

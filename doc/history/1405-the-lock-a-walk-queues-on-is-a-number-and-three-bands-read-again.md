# 1405 — The lock a walk queues on is a number, and three bands the clock crossed are read again

Measure slot of batch sixty-three. ADRs 1646 and 1647; no question written.

**The lock** (ADR 1646). `tools/bounded.sh --lock [--round N]` takes `/home/AI/heavy-walk.lock`
itself, uses an ancestor's descriptor where `flock` already holds it (a second description would
queue behind its own caller), and appends `<asked> batch= round= wait= hold= exit= cmd=` to
`/home/AI/heavy-walk.log` when the wrapper ends; `tools/state.sh gates-cost` prints the last
batch's lines and each round's sums. Self-test case 7 calibrated by planting both defects. This
round's eight runs queued 160, 1 762, 1 679, 448 and 460 s, and nothing three times.
**A launch child's pool** (ADR 1646). `launch_path`'s children inherited the caller's
`RAYON_NUM_THREADS` (four under the lock's prefix, 24 under `bounded.sh`); they now get the pinned
count, as `turn_path`'s do (trap 122). On `bug1815476.pdf` the width moved nothing measurable.
**The bands** (ADR 1647), HEAD in a private worktree, own target directory, load 0.4 to 3.3:
`issue14415.pdf`'s step 8.37 to 8.66 ms in ten `turn_path` runs and 8.31 to 8.69 in fifteen child
sets, so it keeps its band (the sentence below goes to slot 3's `turn-path.toml`); `bring_up_ms`
15.51 to 19.76 in fifteen of sixteen `launch_path` runs and 21.26 in one, idle-born children 4.4 ms
higher, the adapter check most of it, a 30 ms spin no help (built, measured, removed), so it keeps
13.6 .. 20.6 with the reason; `bug1815476.pdf`'s allocations are bimodal (45.9/46.3 against 52.8 to
56.1, the buffers' overlap is the scheduler's), so the floor moves by the file's rule to 41.0.
For slot 3: "Ten runs on 2026-10-07 at a load of 1.4 to 3.3 read the step at 8.37 to 8.66 ms and
fifteen sets of three children, ten born after 1.5 s of idle, 8.31 to 8.69; round 1398's 10.73 did
not recur, the busy end of trap 110, and the band keeps its reason (ADR 1647)."

**Premise.** Held: all three readings sit past their edges in the files named. Not quite: the
bring-up and memory readings are in no record of 1398's (only the step is); and the bring-up
excursion is not the processor's clock the brief's "gate's own warm-up" assumed.

**Gates.** `bash -n` on `tools/bounded.sh` and `tools/state.sh`: 0. `tools/bounded.sh --self-test`:
0, seven cases. `rustfmt --check --edition 2024` on `launch_path.rs`: 0. `RUSTFLAGS="-D warnings"
cargo clippy -p viewer-ui --all-targets`: 0. `cargo nextest run -p viewer-ui`: 0, 149 passed.
`cargo test -p conformance`: 0, 397 passed. Behind the lock: `launch_path --release` with clocks
sixteen times on HEAD with the change, 15 of 16 with 0 outside; on the new bands twice with clocks
(0 outside, the clocks declined by a calibration of 0.906 and 0.782) and once counted (31 judged, 0
outside); `turn_path --release` ten times, 33 of 33 judged, 0 outside, each.

**Unfinished.** `doc/environment.md`'s rule line and `tools/batch.sh run` still take the lock with a
bare `flock` (not this slot's files); `tests/bounded.rs`'s module comment says six cases.
